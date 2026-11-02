#include "linux/files/memory_file.h"
#include "base/fd.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "memory/address_space.h"
#include <algorithm>
#include <cerrno>
#include <climits>
#include <cstdlib>
#include <fcntl.h>
#include <map>
#include <sys/stat.h>
#include <tuple>
#include <unistd.h>
#include <utility>
#include <vector>
#ifdef __linux__
#include <sys/syscall.h>
#endif

namespace operit::arm64::files {
struct MemoryFileState {
    FD anchor;
    uint64_t device, inode;
    std::string name;
    std::recursive_mutex mutex;
    uint32_t seals;
    uint64_t shared_writable_mappings = 0;

    MemoryFileState(int fd, const struct stat &info, std::string label, uint32_t flags)
        : anchor(::fcntl(fd, F_DUPFD_CLOEXEC, 0)), device(uint64_t(info.st_dev)),
          inode(uint64_t(info.st_ino)), name(std::move(label)),
          seals(flags & mfd_allow_sealing ? 0 : memory_file_seal_seal) {
        check_host(anchor.get());
    }
    uint64_t size() const {
        struct stat info{};
        check_host(::fstat(anchor.get(), &info));
        return info.st_size < 0 ? 0 : uint64_t(info.st_size);
    }
};

namespace {
constexpr uint32_t supported_seals = memory_file_seal_seal | memory_file_seal_shrink |
                                     memory_file_seal_grow | memory_file_seal_write |
                                     memory_file_seal_future_write;
constexpr uint32_t write_seals = memory_file_seal_write | memory_file_seal_future_write;
constexpr size_t max_name = 249; // NAME_MAX minus the six-byte "memfd:" prefix.
struct FileKey {
    uint64_t device, inode;
    bool operator<(const FileKey &other) const {
        return std::tie(device, inode) < std::tie(other.device, other.inode);
    }
};
struct Binding {
    std::weak_ptr<OpenDescription> description;
    std::shared_ptr<MemoryFileState> state;
};
struct Registry {
    std::mutex mutex;
    std::map<FileKey, std::weak_ptr<MemoryFileState>> files;
    std::map<const OpenDescription *, Binding> descriptions;
};
Registry &registry() {
    static Registry instance;
    return instance;
}
void collect(Registry &entries) {
    // The explicit description-closed hook reclaims immediately. This sweep
    // also bounds retention if a caller has not yet installed that hook.
    for (auto it = entries.descriptions.begin(); it != entries.descriptions.end();) {
        if (it->second.description.expired())
            it = entries.descriptions.erase(it);
        else
            ++it;
    }
    for (auto it = entries.files.begin(); it != entries.files.end();) {
        if (it->second.expired())
            it = entries.files.erase(it);
        else
            ++it;
    }
}
std::shared_ptr<MemoryFileState> state_for(int fd) {
    if (fd < 0)
        return {};
    auto &entries = registry();
    {
        std::lock_guard<std::mutex> lock(entries.mutex);
        if (entries.files.empty())
            return {};
    }
    struct stat info{};
    check_host(::fstat(fd, &info));
    std::lock_guard<std::mutex> lock(entries.mutex);
    auto found = entries.files.find({uint64_t(info.st_dev), uint64_t(info.st_ino)});
    if (found == entries.files.end())
        return {};
    auto state = found->second.lock();
    if (!state)
        entries.files.erase(found);
    return state;
}
void bind_description(const Descriptor &entry, const std::shared_ptr<MemoryFileState> &state) {
    if (!state)
        return;
    auto &entries = registry();
    std::lock_guard<std::mutex> lock(entries.mutex);
    entries.descriptions.insert_or_assign(entry.description.get(), Binding{entry.description, state});
}
std::shared_ptr<MemoryFileState> state_for(const Descriptor &entry) {
    auto state = state_for(entry.host.get());
    bind_description(entry, state);
    return state;
}
std::string read_name(Memory &memory, uint64_t address) {
    const Memory::AccessGuard access(memory);
    std::string name;
    name.reserve(max_name);
    for (size_t length = 0; length <= max_name; ++length) {
        if (!memory.accessible(address, 1, read_perm))
            guest_error(14);
        const uint8_t value = uint8_t(memory.load(address, 1));
        if (!value)
            return name;
        if (length == max_name)
            guest_error(22); // memfd uses EINVAL, not path ENAMETOOLONG.
        name.push_back(char(value));
        ++address;
    }
    guest_error(22);
}
FD anonymous_backing(const std::string &name, uint32_t flags) {
#if defined(__linux__) && defined(SYS_memfd_create)
    const int native = int(::syscall(SYS_memfd_create, name.c_str(), flags | mfd_cloexec));
    if (native >= 0)
        return FD(native);
    if (errno != ENOSYS)
        guest_error(linux_errno(errno));
#else
    (void)name;
    (void)flags;
#endif
    // Darwin has no memfd. An immediately unlinked regular file preserves
    // seek/read/write/truncate/MAP_SHARED semantics, including sparse holes.
    // POSIX shm FDs do not supply the regular-file I/O contract on Darwin.
    // This fallback is anonymous, but its host backing need not be RAM-only.
    const char *configured = std::getenv("TMPDIR");
    std::string directory = configured && *configured ? configured : "/tmp";
    if (directory.back() != '/')
        directory += '/';
    const std::string pattern = directory + "operit-memfd-XXXXXX";
    std::vector<char> path(pattern.begin(), pattern.end());
    path.push_back('\0');
    FD fd(::mkstemp(path.data()));
    check_host(fd.get());
    check_host(::unlink(path.data()));
    check_host(::fcntl(fd.get(), F_SETFD, FD_CLOEXEC));
    // Linux memfd mode is 0777 independently of the process umask. There is
    // already no directory entry when these permission bits are installed.
    check_host(::fchmod(fd.get(), 0777));
    return fd;
}
} // namespace

int memory_file_create(Memory &memory, DescriptorTable &descriptors, uint64_t name_pointer,
                       uint32_t flags) {
    // Huge-page, no-exec and all unknown flags are unsupported; never silently
    // create an ordinary executable file for an unimplemented flag contract.
    if (flags & ~(mfd_cloexec | mfd_allow_sealing))
        guest_error(22);
    auto name = read_name(memory, name_pointer);
    {
        auto &entries = registry();
        std::lock_guard<std::mutex> lock(entries.mutex);
        collect(entries);
    }
    FD fd = anonymous_backing(name, flags);
    struct stat info{};
    check_host(::fstat(fd.get(), &info));
    auto state = std::make_shared<MemoryFileState>(fd.get(), info, std::move(name), flags);
    // AArch64 O_LARGEFILE is 0x20000 (0x8000 is O_NOFOLLOW on this ABI).
    const int guest_flags = 2 | 0x20000 | ((flags & mfd_cloexec) ? 524288 : 0);
    const FileKey key{state->device, state->inode};
    {
        // Register before publishing the guest FD. Descriptor's constructor
        // attaches its description, including /proc-created descriptions.
        auto &entries = registry();
        std::lock_guard<std::mutex> lock(entries.mutex);
        entries.files.insert_or_assign(key, state);
    }
    try {
        return descriptors.insert(std::move(fd), "", guest_flags);
    } catch (...) {
        auto &entries = registry();
        std::lock_guard<std::mutex> lock(entries.mutex);
        entries.files.erase(key);
        throw;
    }
}

void memory_file_attach(const Descriptor &entry) {
    (void)state_for(entry);
}
void memory_file_description_closed(const OpenDescription *description) noexcept {
    auto &entries = registry();
    std::lock_guard<std::mutex> lock(entries.mutex);
    auto found = entries.descriptions.find(description);
    if (found == entries.descriptions.end())
        return;
    const FileKey key{found->second.state->device, found->second.state->inode};
    entries.descriptions.erase(found);
    auto file = entries.files.find(key);
    if (file != entries.files.end() && file->second.expired())
        entries.files.erase(file);
}
std::optional<std::string> memory_file_link(int host_fd) {
    auto state = state_for(host_fd);
    if (!state)
        return {};
    return "/memfd:" + state->name + " (deleted)";
}
int memory_file_get_seals(const Descriptor &entry) {
    if (entry.description->flags & 0x200000) // O_PATH
        guest_error(9);
    auto state = state_for(entry);
    if (!state)
        guest_error(22);
    std::lock_guard<std::recursive_mutex> lock(state->mutex);
    return int(state->seals);
}
void memory_file_add_seals(const Descriptor &entry, uint32_t seals) {
    if (entry.description->flags & 0x200000)
        guest_error(9);
    if (!(entry.description->flags & 3))
        guest_error(1); // F_ADD_SEALS requires write access, including adding 0.
    if (seals & ~supported_seals)
        guest_error(22);
    auto state = state_for(entry);
    if (!state)
        guest_error(22);
    std::lock_guard<std::recursive_mutex> lock(state->mutex);
    if (state->seals & memory_file_seal_seal)
        guest_error(1);
    if ((seals & memory_file_seal_write) && !(state->seals & memory_file_seal_write) &&
        state->shared_writable_mappings)
        guest_error(16);
    // All checks precede mutation: adding WRITE|SEAL with busy mappings must
    // not leave SEAL installed after the operation returns EBUSY.
    state->seals |= seals;
}

MemoryFileGuard::MemoryFileGuard(std::shared_ptr<MemoryFileState> state) : state_(std::move(state)) {
    if (state_)
        lock_ = std::unique_lock<std::recursive_mutex>(state_->mutex);
}
MemoryFileGuard::MemoryFileGuard(MemoryFileGuard &&) noexcept = default;
MemoryFileGuard &MemoryFileGuard::operator=(MemoryFileGuard &&other) noexcept {
    if (this != &other) {
        // Unlock before dropping the state which owns this lock's mutex.
        lock_ = std::unique_lock<std::recursive_mutex>();
        state_ = std::move(other.state_);
        lock_ = std::move(other.lock_);
        length_ = other.length_;
        offset_ = other.offset_;
    }
    return *this;
}
MemoryFileGuard::~MemoryFileGuard() = default;
MemoryFileGuard memory_file_lock(int host_fd) {
    return MemoryFileGuard(state_for(host_fd));
}
MemoryFileGuard memory_file_check_write(int host_fd, uint64_t length,
                                        std::optional<uint64_t> offset, std::optional<bool> append) {
    auto guard = memory_file_lock(host_fd);
    guard.length_ = length;
    guard.offset_ = offset.value_or(0);
    if (!guard || !length)
        return guard;
    if (guard.state_->seals & write_seals)
        guest_error(1);
    if (offset && *offset > uint64_t(INT64_MAX))
        guest_error(22);
    const int flags = ::fcntl(host_fd, F_GETFL);
    check_host(flags);
    const uint64_t bytes = guard.state_->size();
    if (append.value_or(bool(flags & O_APPEND)))
        guard.offset_ = bytes;
    else if (!offset) {
        const off_t position = ::lseek(host_fd, 0, SEEK_CUR);
        if (position < 0)
            guest_error(linux_errno(errno));
        guard.offset_ = uint64_t(position);
    }
    // Linux shmem_write_begin rejects a whole extending write chunk before
    // copying; generic_perform_write returns earlier completed chunks on error.
    // Use the guest's base-page chunks (large folios are not emulated here).
    // With an unaligned EOF, even bytes before EOF in its failing chunk must
    // remain untouched. FSIZE reduction has already happened at the caller.
    if ((guard.state_->seals & memory_file_seal_grow) &&
        (guard.offset_ > bytes || length > bytes - guard.offset_)) {
        const uint64_t completed_end = bytes - bytes % page_size;
        if (guard.offset_ >= completed_end)
            guest_error(1);
        guard.length_ = completed_end - guard.offset_;
    }
    return guard;
}
MemoryFileGuard memory_file_check_truncate(int host_fd, uint64_t length) {
    auto guard = memory_file_lock(host_fd);
    guard.length_ = length;
    if (!guard)
        return guard;
    if (length > uint64_t(INT64_MAX))
        guest_error(22);
    const uint64_t bytes = guard.state_->size();
    if ((length < bytes && (guard.state_->seals & memory_file_seal_shrink)) ||
        (length > bytes && (guard.state_->seals & memory_file_seal_grow)))
        guest_error(1);
    // WRITE and FUTURE_WRITE alone do not seal the file size.
    return guard;
}
MemoryFileGuard memory_file_check_fallocate(int host_fd, uint32_t mode, uint64_t offset,
                                            uint64_t length) {
    auto guard = memory_file_lock(host_fd);
    guard.offset_ = offset;
    guard.length_ = length;
    if (!guard)
        return guard;
    constexpr uint32_t keep_size = 1, punch_hole = 2;
    if (mode & ~(keep_size | punch_hole))
        guest_error(95); // shmem does not implement collapse/insert/zero ranges.
    if (!length || offset > uint64_t(INT64_MAX) || length > uint64_t(INT64_MAX) - offset ||
        ((mode & punch_hole) && !(mode & keep_size)))
        guest_error(22);
    if ((mode & punch_hole) && (guard.state_->seals & write_seals))
        guest_error(1);
    // shmem_fallocate checks GROW even with KEEP_SIZE, after its hole-punch
    // branch. Reserving blocks past EOF must not bypass the growth seal.
    if (!(mode & punch_hole) && offset + length > guard.state_->size() &&
        (guard.state_->seals & memory_file_seal_grow))
        guest_error(1);
    return guard;
}

MemoryFileMapping::MemoryFileMapping(std::shared_ptr<MemoryFileState> state, bool shared, bool may_write)
    : state_(std::move(state)), shared_(shared), may_write_(may_write) {
    if (shared_ && may_write_) {
        std::lock_guard<std::recursive_mutex> lock(state_->mutex);
        ++state_->shared_writable_mappings;
    }
}
MemoryFileMapping::~MemoryFileMapping() {
    if (shared_ && may_write_) {
        std::lock_guard<std::recursive_mutex> lock(state_->mutex);
        --state_->shared_writable_mappings;
    }
}
void MemoryFileMapping::check_protection(unsigned permissions) const {
    if (shared_ && (permissions & write_perm) && !may_write_)
        guest_error(13);
}
std::shared_ptr<MemoryFileMapping> memory_file_check_mmap(int host_fd, bool shared,
                                                        unsigned permissions, bool permit_write) {
    auto state = state_for(host_fd);
    if (!state)
        return {};
    std::lock_guard<std::recursive_mutex> lock(state->mutex);
    if (permissions > (read_perm | write_perm | exec_perm))
        guest_error(22);
    const int flags = ::fcntl(host_fd, F_GETFL);
    check_host(flags);
    if ((flags & O_ACCMODE) == O_WRONLY)
        guest_error(13);
    bool may_write = true;
    if (shared) {
        may_write = permit_write && (flags & O_ACCMODE) == O_RDWR;
        if ((permissions & write_perm) && !may_write)
            guest_error(13);
        if (state->seals & write_seals) {
            if (permissions & write_perm)
                guest_error(1);
            may_write = false;
        }
    }
    return std::shared_ptr<MemoryFileMapping>(new MemoryFileMapping(state, shared, may_write));
}
void memory_file_check_mprotect(const std::shared_ptr<MemoryFileMapping> &mapping,
                                unsigned permissions) {
    if (mapping)
        mapping->check_protection(permissions);
}
} // namespace operit::arm64::files
