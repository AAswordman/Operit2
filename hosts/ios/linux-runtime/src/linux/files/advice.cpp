#include "linux/files/advice.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "linux/files/descriptor_access.h"
#include <algorithm>
#include <array>
#include <cerrno>
#include <climits>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#ifdef __APPLE__
#include <sys/mman.h>
#endif
#ifdef __linux__
#include <sys/syscall.h>
#endif

namespace operit::arm64::files {
namespace {
#ifdef __APPLE__
constexpr int linux_o_direct = 0x4000;
#endif
constexpr uint32_t sync_range_flags = sync_range_wait_before | sync_range_write | sync_range_wait_after;

Descriptor &lookup(DescriptorTable &descriptors, int32_t fd) {
    auto &entry = descriptors.get(uint32_t(fd));
    require_data_descriptor(entry);
    return entry;
}

bool anonymous(const OpenDescription &description) {
    return description.counter || description.poll || description.timer;
}

struct stat file_status(const Descriptor &entry) {
    const auto &description = *entry.description;
    struct stat info{};
    // Runtime event/timer descriptors can use native pipes as wakeup handles.
    // Their backing handle's type is not the guest open description's type.
    if (anonymous(description))
        return info;
    if (description.pipe) {
        info.st_mode = S_IFIFO;
        return info;
    }
    if (description.socket) {
        info.st_mode = S_IFSOCK;
        return info;
    }
    if (description.terminal) {
        info.st_mode = S_IFCHR;
        return info;
    }
    if (description.virtual_file)
        return description.virtual_file->node->stat();
    check_host(::fstat(entry.host.get(), &info));
    return info;
}

bool cache_backing(const Descriptor &entry, const struct stat &info) {
    const auto &description = *entry.description;
    return entry.host.get() >= 0 && !description.virtual_file && !anonymous(description) &&
           (S_ISREG(info.st_mode) || S_ISBLK(info.st_mode));
}

#ifdef __APPLE__
bool unsupported_hint(int error) {
    return error == EINVAL || error == ENOTTY || error == ENOSYS || error == ENOTSUP || error == EOPNOTSUPP;
}
#endif

void wait_writeback(int fd) {
#ifdef __APPLE__
    check_host(::fsync(fd));
#else
    if (::fdatasync(fd) == 0)
        return;
    const int error = errno;
    if (error != EINVAL && error != ENOSYS)
        guest_error(linux_errno(error));
    check_host(::fsync(fd)); // Directories and filesystems without a data-only sync.
#endif
}

void start_writeback(int fd) {
#if defined(__APPLE__) && defined(F_FLUSH_DATA)
    if (::fcntl(fd, F_FLUSH_DATA) == 0)
        return;
    const int error = errno;
    if (!unsupported_hint(error))
        guest_error(linux_errno(error));
#endif
    wait_writeback(fd);
}

#ifdef __APPLE__
// Linux advice accepts a positive range whose sum overflows loff_t, treating
// its end as LLONG_MAX. Length zero also means through EOF, including readahead's
// vfs_fadvise(WILLNEED) path. Keep the endpoint unsigned and exclusive.
uint64_t advice_end(int64_t offset, int64_t length) {
    if (!length || uint64_t(length) > uint64_t(INT64_MAX) - uint64_t(offset))
        return uint64_t(INT64_MAX) + 1;
    return uint64_t(offset) + uint64_t(length);
}

void willneed(const Descriptor &entry, const struct stat &info, int64_t offset, int64_t length) {
    constexpr uint64_t readahead_window = 2 << 20;
    uint64_t end = advice_end(offset, length);
    if (S_ISREG(info.st_mode))
        end = std::min(end, info.st_size > 0 ? uint64_t(info.st_size) : 0);
    if (uint64_t(offset) >= end)
        return;
    uint64_t remaining = std::min(end - uint64_t(offset), readahead_window);
#ifdef F_RDADVISE
    struct radvisory request{};
    request.ra_offset = off_t(offset);
    request.ra_count = int(remaining);
    if (::fcntl(entry.host.get(), F_RDADVISE, &request) == 0)
        return;
    const int error = errno;
    if (!unsupported_hint(error))
        guest_error(linux_errno(error));
#endif
    // fadvise permits O_WRONLY. Do not turn a missing native hint into EBADF
    // by attempting a positioned read on a write-only open description.
    if ((entry.description->flags & 3) == 1)
        guest_error(95);
    std::array<uint8_t, 64 << 10> buffer;
    uint64_t position = uint64_t(offset);
    while (remaining) {
        const size_t count = size_t(std::min<uint64_t>(remaining, buffer.size()));
        const ssize_t done = ::pread(entry.host.get(), buffer.data(), count, off_t(position));
        if (done < 0) {
            if (errno == EINTR)
                continue;
            guest_error(linux_errno(errno));
        }
        if (!done)
            break;
        position += uint64_t(done);
        remaining -= uint64_t(done);
    }
}

bool unavailable_invalidation(int error) {
    // Eviction is advisory: locked pages, unsupported mappings and temporary
    // mapping pressure can leave pages resident, just as on Linux.
    return unsupported_hint(error) || error == EBUSY || error == EAGAIN || error == ENOMEM ||
           error == ENODEV || error == EACCES || error == EOVERFLOW;
}

void dontneed(const Descriptor &entry, const struct stat &info, int64_t offset, int64_t length) {
    start_writeback(entry.host.get());
    // A write-only description can request writeback but cannot create a readable
    // mapping. Retaining cached pages is permitted for this advisory operation.
    if (!S_ISREG(info.st_mode) || (entry.description->flags & 3) == 1 || info.st_size <= 0)
        return;
    const uint64_t bytes = uint64_t(info.st_size), alignment = uint64_t(::getpagesize());
    uint64_t end = std::min(advice_end(offset, length), bytes);
    // Preserve the leading partial page and, except at EOF, the trailing one.
    // On a 16K Darwin host, retaining extra 4K guest pages is safe for a hint.
    uint64_t position = (uint64_t(offset) + alignment - 1) / alignment * alignment;
    if (end != bytes)
        end = end / alignment * alignment;
    while (position < end) {
        const size_t count = size_t(std::min<uint64_t>(end - position, 16 << 20));
        void *mapping = ::mmap(nullptr, count, PROT_READ, MAP_SHARED, entry.host.get(), off_t(position));
        if (mapping == MAP_FAILED) {
            const int error = errno;
            if (unavailable_invalidation(error))
                return;
            guest_error(linux_errno(error));
        }
        const int result = ::msync(mapping, count, MS_SYNC | MS_INVALIDATE);
        const int error = errno;
        ::munmap(mapping, count);
        if (result < 0) {
            if (unavailable_invalidation(error))
                return;
            guest_error(linux_errno(error));
        }
        position += count;
    }
}

void darwin_advice(const Descriptor &entry, const struct stat &info, int64_t offset, int64_t length,
                   int32_t advice) {
    // Non-cacheable devices, directories and sockets have no file-data cache
    // for these hints. Linux's generic advice likewise ignores valid hints there.
    if (!cache_backing(entry, info))
        return;
    switch (advice) {
    case fadv_normal:
        // Preserve an independently requested guest O_DIRECT cache bypass.
        check_host(::fcntl(entry.host.get(), F_NOCACHE,
                           (entry.description->flags & linux_o_direct) ? 1 : 0));
        check_host(::fcntl(entry.host.get(), F_RDAHEAD, 1));
        break;
    case fadv_random:
        check_host(::fcntl(entry.host.get(), F_RDAHEAD, 0));
        break;
    case fadv_sequential:
        check_host(::fcntl(entry.host.get(), F_RDAHEAD, 1));
        break;
    case fadv_willneed:
        willneed(entry, info, offset, length);
        break;
    case fadv_dontneed:
        dontneed(entry, info, offset, length);
        break;
    case fadv_noreuse:
        check_host(::fcntl(entry.host.get(), F_NOCACHE, 1));
        break;
    }
}
#else
int native_advice(int advice) {
    switch (advice) {
    case fadv_normal:
        return POSIX_FADV_NORMAL;
    case fadv_random:
        return POSIX_FADV_RANDOM;
    case fadv_sequential:
        return POSIX_FADV_SEQUENTIAL;
    case fadv_willneed:
        return POSIX_FADV_WILLNEED;
    case fadv_dontneed:
        return POSIX_FADV_DONTNEED;
    case fadv_noreuse:
        return POSIX_FADV_NOREUSE;
    default:
        guest_error(22);
    }
}
void host_advice(int fd, int64_t offset, int64_t length, int advice) {
    const int error = ::posix_fadvise(fd, off_t(offset), off_t(length), native_advice(advice));
    if (error)
        guest_error(linux_errno(error)); // posix_fadvise returns an errno; it does not set errno.
}
#endif
} // namespace

int64_t readahead(Memory &memory, DescriptorTable &descriptors, int32_t fd, int64_t offset, uint64_t count) {
    (void)memory;
    auto &entry = lookup(descriptors, fd);
    if ((entry.description->flags & 3) == 1)
        guest_error(9);
    const auto info = file_status(entry);
    if (!cache_backing(entry, info))
        guest_error(22);
    if (offset < 0 || count > uint64_t(INT64_MAX))
        guest_error(22);
#if defined(__linux__) && defined(SYS_readahead)
    if (::syscall(SYS_readahead, entry.host.get(), off_t(offset), size_t(count)) == 0)
        return 0;
    const int error = errno;
    if (error != ENOSYS)
        guest_error(linux_errno(error));
#endif
#ifdef __APPLE__
    willneed(entry, info, offset, int64_t(count));
#else
    host_advice(entry.host.get(), offset, int64_t(count), fadv_willneed);
#endif
    return 0;
}

int64_t fadvise64(Memory &memory, DescriptorTable &descriptors, int32_t fd, int64_t offset, int64_t length,
                  int32_t advice) {
    (void)memory;
    auto &entry = lookup(descriptors, fd);
    const auto info = file_status(entry);
    // Linux diagnoses a pipe before examining its range or advice value.
    if (S_ISFIFO(info.st_mode))
        guest_error(29);
    if (offset < 0 || length < 0 || advice < fadv_normal || advice > fadv_noreuse)
        guest_error(22);
    // Anonymous handles, terminals and unbacked virtual nodes have no native
    // file-data cache; do not advise their internal pipe/event wakeup handle.
    if (anonymous(*entry.description) || entry.description->terminal || entry.description->virtual_file)
        return 0;
#ifdef __APPLE__
    darwin_advice(entry, info, offset, length, advice);
#else
    host_advice(entry.host.get(), offset, length, advice);
#endif
    return 0;
}

int64_t sync_file_range(Memory &memory, DescriptorTable &descriptors, int32_t fd, int64_t offset,
                       int64_t length, uint32_t flags) {
    (void)memory;
    auto &entry = lookup(descriptors, fd);
    // Lookup precedes argument validation, matching the Linux fdget path.
    if ((flags & ~sync_range_flags) || offset < 0 || length < 0 ||
        uint64_t(length) > uint64_t(INT64_MAX) - uint64_t(offset))
        guest_error(22);
    const auto info = file_status(entry);
    if (!S_ISREG(info.st_mode) && !S_ISBLK(info.st_mode) && !S_ISDIR(info.st_mode))
        guest_error(29);
    if (entry.host.get() < 0 || entry.description->virtual_file)
        guest_error(22);
#if defined(__linux__) && defined(SYS_sync_file_range)
    if (::syscall(SYS_sync_file_range, entry.host.get(), off_t(offset), off_t(length), flags) == 0)
        return 0;
    const int error = errno;
    if (error != ENOSYS)
        guest_error(linux_errno(error));
#elif defined(__linux__) && defined(SYS_sync_file_range2)
    if (::syscall(SYS_sync_file_range2, entry.host.get(), flags, off_t(offset), off_t(length)) == 0)
        return 0;
    const int error = errno;
    if (error != ENOSYS)
        guest_error(linux_errno(error));
#endif
    // There is no access-mode or read-only-mount check: writeback can be
    // requested through a read-only descriptor, and length == 0 means to EOF.
    // Darwin flush/wait operations cover the whole file, a stronger guarantee.
    if (flags & sync_range_wait_before)
        wait_writeback(entry.host.get());
    if (flags & sync_range_write)
        start_writeback(entry.host.get());
    if (flags & sync_range_wait_after)
        wait_writeback(entry.host.get());
    return 0;
}
} // namespace operit::arm64::files
