#include "linux/files/allocation.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/memory_file.h"
#include "linux/process/resource_limits.h"
#include "memory/address_space.h"
#include <algorithm>
#include <array>
#include <cerrno>
#include <climits>
#include <fcntl.h>
#include <limits>
#include <sys/stat.h>
#include <unistd.h>
#ifdef __APPLE__
#include <sys/mount.h>
#endif
#ifdef __linux__
#include <sys/syscall.h>
#endif

namespace operit::arm64::files {
namespace {
int allocation_errno(int error, bool file_control = false) {
#ifdef EDQUOT
    if (error == EDQUOT)
        return 122;
#endif
#ifdef ETXTBSY
    if (error == ETXTBSY)
        return 26;
#endif
    if (file_control && (error == ENOTTY || error == ENOSYS))
        return 95;
    return linux_errno(error);
}
void native_result(int result, bool file_control = false) {
    if (result < 0)
        guest_error(allocation_errno(errno, file_control));
}
void check_mode(uint32_t mode) {
    constexpr uint32_t known = allocation_keep_size | allocation_punch_hole |
                               allocation_collapse_range | allocation_zero_range |
                               allocation_insert_range | allocation_unshare_range;
    if (mode & ~known)
        guest_error(95);
    switch (mode & ~allocation_keep_size) {
    case 0:
    case allocation_zero_range:
    case allocation_unshare_range:
        return;
    case allocation_punch_hole:
        if (mode & allocation_keep_size)
            return;
        break;
    case allocation_collapse_range:
    case allocation_insert_range:
        if (!(mode & allocation_keep_size))
            return;
        break;
    }
    guest_error(95);
}
void check_description(const Descriptor &entry) {
    const auto &description = *entry.description;
    require_descriptor_io(entry, true);
    description.require_writable();
    if (description.pipe)
        guest_error(29);
    if (description.socket || description.terminal || description.counter || description.poll ||
        description.timer)
        guest_error(19);
    if (description.virtual_file) {
        switch (description.virtual_file->node->kind) {
        case VirtualNode::Kind::Directory:
            guest_error(21);
        case VirtualNode::Kind::File:
            guest_error(95);
        default:
            guest_error(19);
        }
    }
    if (entry.host.get() < 0)
        guest_error(9);
    const int flags = ::fcntl(entry.host.get(), F_GETFL);
    native_result(flags);
    if ((flags & O_ACCMODE) != O_WRONLY && (flags & O_ACCMODE) != O_RDWR)
        guest_error(9);
    // O_APPEND on an open description does not make the inode append-only.
    // Do not change it: native fallocate/fcntl and Darwin pwrite preserve it.
}
struct FileChange {
    Memory &memory;
    bool attempted = false;
    ~FileChange() {
        if (attempted) {
            memory.code_epoch.fetch_add(1, std::memory_order_relaxed);
        }
    }
};

#ifdef __APPLE__
void write_zeroes(int fd, uint64_t offset, uint64_t end, FileChange &change) {
    static const std::array<uint8_t, 64 * 1024> zeroes{};
    while (offset < end) {
        const size_t count = size_t(std::min<uint64_t>(end - offset, zeroes.size()));
        change.attempted = true;
        const ssize_t written = ::pwrite(fd, zeroes.data(), count, off_t(offset));
        if (written < 0)
            guest_error(allocation_errno(errno));
        if (!written)
            guest_error(5);
        offset += uint64_t(written);
    }
}
void preallocate(int fd, uint64_t end, const struct stat &info, FileChange &change) {
#ifdef F_PREALLOCATE
    if (info.st_blocks < 0 || uint64_t(info.st_blocks) > uint64_t(INT64_MAX) / 512)
        guest_error(27);
    const uint64_t allocated = uint64_t(info.st_blocks) * 512;
    fstore_t request{};
    request.fst_flags = F_ALLOCATEALL;
#ifdef F_ALLOCATEPERSIST
    request.fst_flags |= F_ALLOCATEPERSIST;
#else
    // Allocation beyond EOF must survive close for KEEP_SIZE and sparse files.
    guest_error(95);
#endif
    request.fst_posmode = F_PEOFPOSMODE;
    // F_PEOFPOSMODE requests ADDITIONAL bytes after the physical allocation,
    // rather than an absolute logical EOF. Asking for end would overallocate.
    request.fst_length = off_t(end > allocated ? end - allocated : 0);
    change.attempted = true;
    native_result(::fcntl(fd, F_PREALLOCATE, &request), true);
    if (request.fst_bytesalloc < request.fst_length)
        guest_error(28);
#else
    (void)fd;
    (void)end;
    (void)info;
    (void)change;
    guest_error(95);
#endif
}
void punch_hole(int fd, uint64_t offset, uint64_t end, uint64_t file_size, FileChange &change) {
#ifdef F_PUNCHHOLE
    if (offset >= file_size)
        return;
    end = std::min(end, file_size);
    fpunchhole_t request{};
    request.fp_offset = off_t(offset);
    request.fp_length = off_t(end - offset);
    change.attempted = true;
    if (::fcntl(fd, F_PUNCHHOLE, &request) == 0)
        return;
    const int error = errno;
    if (error != EINVAL)
        guest_error(allocation_errno(error, true));

    // Darwin filesystems can require aligned holes. Linux zeroes the boundary
    // bytes and deallocates only whole blocks, preserving both adjacent data
    // and EOF. st_blksize is an I/O hint, not the filesystem allocation unit.
    struct statfs filesystem{};
    native_result(::fstatfs(fd, &filesystem));
    if (filesystem.f_bsize <= 0)
        guest_error(95);
    const uint64_t block = uint64_t(filesystem.f_bsize);
    const uint64_t remainder = offset % block;
    const uint64_t prefix = remainder ? std::min(end - offset, block - remainder) : 0;
    const uint64_t first = offset + prefix;
    const uint64_t last = end - end % block;
    if (first < last) {
        request = {};
        request.fp_offset = off_t(first);
        request.fp_length = off_t(last - first);
        native_result(::fcntl(fd, F_PUNCHHOLE, &request), true);
        write_zeroes(fd, offset, first, change);
        write_zeroes(fd, last, end, change);
    } else
        write_zeroes(fd, offset, end, change);
#else
    (void)fd;
    (void)offset;
    (void)end;
    (void)file_size;
    (void)change;
    guest_error(95);
#endif
}
void darwin_allocate(int fd, uint32_t mode, uint64_t offset, uint64_t end,
                     const struct stat &info, FileChange &change) {
    const uint32_t operation = mode & ~allocation_keep_size;
    const uint64_t file_size = uint64_t(info.st_size);
    if (operation == allocation_punch_hole) {
        punch_hole(fd, offset, end, file_size, change);
        return;
    }
    if (operation != 0 && operation != allocation_zero_range)
        guest_error(95);
    if (operation == allocation_zero_range) {
        try {
            preallocate(fd, end, info, change);
        } catch (const GuestError &error) {
            // Real zero writes allocate storage themselves. They cannot reserve
            // the invisible tail of a KEEP_SIZE request past EOF, however.
            if (error.number != 95 || ((mode & allocation_keep_size) && end > file_size))
                throw;
        }
        write_zeroes(fd, offset, mode & allocation_keep_size ? std::min(end, file_size) : end, change);
        return;
    }
    preallocate(fd, end, info, change);
    if (!(mode & allocation_keep_size) && end > file_size) {
        change.attempted = true;
        native_result(::ftruncate(fd, off_t(end)));
    }
}
#endif

int64_t allocate(Memory &memory, const Descriptor &entry, const process::ResourceLimits &limits,
                 uint32_t mode, int64_t offset, int64_t length, bool &size_signal) {
    const Memory::AccessGuard access(memory);
    // Linux fdget() rejects O_PATH before fallocate validates its arguments.
    require_data_descriptor(entry);
    if (offset < 0 || length <= 0)
        guest_error(22);
    check_mode(mode);
    check_description(entry);
    const int fd = entry.host.get();
    struct stat info{};
    native_result(::fstat(fd, &info));
    if (S_ISFIFO(info.st_mode))
        guest_error(29);
    if (S_ISDIR(info.st_mode))
        guest_error(21);
    if (!S_ISREG(info.st_mode) && !S_ISBLK(info.st_mode))
        guest_error(19);
    if (uint64_t(length) > uint64_t(INT64_MAX) - uint64_t(offset))
        guest_error(27);
    const uint64_t end = uint64_t(offset) + uint64_t(length);
    if (end > uint64_t(std::numeric_limits<off_t>::max()))
        guest_error(27);
#ifdef __APPLE__
    if (!S_ISREG(info.st_mode))
        guest_error(95);
    if (info.st_flags & (UF_IMMUTABLE | SF_IMMUTABLE))
        guest_error(1);
    if ((mode & ~allocation_keep_size) && (info.st_flags & (UF_APPEND | SF_APPEND)))
        guest_error(1);
#endif
    const uint32_t operation = mode & ~allocation_keep_size;
#ifdef __APPLE__
    if (operation != 0 && operation != allocation_punch_hole && operation != allocation_zero_range)
        guest_error(95);
#endif
    // Lock identity/size before checking limits, then check seals under that
    // same recursive lock. shmem checks the size limit BEFORE F_SEAL_GROW.
    // Memory precedes the memfd lock, matching mmap's lock order.
    auto identity = memory_file_lock(fd);
    if (identity && operation != 0 && operation != allocation_punch_hole)
        guest_error(95); // Linux shmem has no zero/insert/collapse/unshare mode.
    native_result(::fstat(fd, &info));
    if (info.st_size < 0)
        guest_error(27);
    const uint64_t file_size = uint64_t(info.st_size);
    uint64_t requested_size = file_size;
    if (operation == allocation_insert_range) {
        if (uint64_t(length) > uint64_t(INT64_MAX) - file_size)
            guest_error(27);
        requested_size = file_size + uint64_t(length);
    } else if ((operation == 0 || operation == allocation_zero_range ||
                operation == allocation_unshare_range) &&
               (!(mode & allocation_keep_size) || bool(identity)))
        requested_size = end;
    if (S_ISREG(info.st_mode)) {
        try {
            limits.check_file_resize(file_size, requested_size);
        } catch (const GuestError &error) {
            if (error.number == 27)
                size_signal = true;
            throw;
        }
    }
    // Remains held through every native allocation, truncate, zero or punch.
    auto seal = memory_file_check_fallocate(fd, mode, uint64_t(offset), uint64_t(length));
    FileChange change{memory};
#ifdef __APPLE__
    darwin_allocate(fd, mode, uint64_t(offset), end, info, change);
#elif defined(__linux__) && defined(SYS_fallocate)
    change.attempted = true;
    const long result = ::syscall(SYS_fallocate, fd, mode, off_t(offset), off_t(length));
    if (result < 0) {
        if (errno == ENOSYS)
            guest_error(95);
        guest_error(allocation_errno(errno));
    }
#else
    guest_error(95);
#endif
    return 0;
}
} // namespace

int64_t fallocate(Memory &memory, const Descriptor &entry, const process::ResourceLimits &limits,
                  uint32_t mode, int64_t offset, int64_t length,
                  const AllocationSizeSignal &signal_file_size) {
    bool size_signal = false;
    try {
        return allocate(memory, entry, limits, mode, offset, length, size_signal);
    } catch (const GuestError &) {
        // Signal delivery may access Memory or the scheduler. No memory/seal
        // lock from allocate() remains held at this boundary.
        if (size_signal && signal_file_size)
            signal_file_size();
        throw;
    }
}
int64_t fallocate(Memory &memory, DescriptorTable &descriptors, uint64_t fd, uint32_t mode,
                  int64_t offset, int64_t length, const AllocationSizeSignal &signal_file_size) {
    auto &entry = descriptors.get(uint32_t(fd));
    return fallocate(memory, entry, *descriptors.resource_limits(), mode, offset, length, signal_file_size);
}
} // namespace operit::arm64::files
