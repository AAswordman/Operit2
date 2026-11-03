#include "base/host_io.h"
#include "base/random.h"
#include "linux/abi.h"
#include "linux/error_mapping.h"
#include "linux/events/event_counter.h"
#include "linux/events/inotify.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/metadata.h"
#include "linux/files/memory_file.h"
#include "linux/files/open_how.h"
#include "linux/files/record_lock.h"
#include "linux/syscalls.h"
#include <atomic>
#include <climits>
#include <dirent.h>
#include <fcntl.h>
#include <sched.h>
#include <sys/file.h>
#include <sys/time.h>
#include <time.h>

namespace operit::arm64 {
namespace {
void create_guest_directory(const Path &path, mode_t mode) {
    path.require_writable();
    check_host(::mkdirat(path.parent.get(), path.name.c_str(), 0700));
    const auto identity = path.stat();
    // Restore private owner access before opening: the host umask may remove
    // every permission, and the final guest mode may itself be 0000.
    check_host(::fchmodat(path.parent.get(), path.name.c_str(), 0700, AT_SYMLINK_NOFOLLOW));
    FD directory(
        ::openat(path.parent.get(), path.name.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
    check_host(directory.get());
    struct stat opened{};
    check_host(::fstat(directory.get(), &opened));
    if (!S_ISDIR(identity.st_mode) || identity.st_dev != opened.st_dev || identity.st_ino != opened.st_ino)
        guest_error(116);
    path.require_writable();
    check_host(::fchmod(directory.get(), mode));
}
bool same_directory(const Path &first, const Path &second) {
    if (first.mount_id != second.mount_id || first.virtual_path != second.virtual_path)
        return false;
    if (first.virtual_path)
        return first.node && second.node && first.node->identity == second.node->identity;
    const auto a = first.stat(), b = second.stat();
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino;
}
std::string native_directory_name(const Path &parent, const Path &child) {
    const auto identity = child.stat();
    FD scan(::openat(parent.parent.get(), ".", O_RDONLY | O_DIRECTORY | O_CLOEXEC));
    check_host(scan.get());
    const int raw = ::fcntl(scan.get(), F_DUPFD_CLOEXEC, 0);
    check_host(raw);
    std::unique_ptr<DIR, decltype(&::closedir)> stream(::fdopendir(raw), ::closedir);
    if (!stream) {
        const int error = errno;
        ::close(raw);
        guest_error(linux_errno(error));
    }
    for (;;) {
        errno = 0;
        const auto *item = ::readdir(stream.get());
        if (!item) {
            if (errno)
                guest_error(linux_errno(errno));
            guest_error(identity.st_nlink ? 11 : 2);
        }
        if (!std::strcmp(item->d_name, ".") || !std::strcmp(item->d_name, ".."))
            continue;
        struct stat info{};
        if (::fstatat(parent.parent.get(), item->d_name, &info, AT_SYMLINK_NOFOLLOW) == 0 &&
            info.st_dev == identity.st_dev && info.st_ino == identity.st_ino)
            return item->d_name;
    }
}
// Recover display coordinates from pinned directories, including bind-mount
// attachments. The spelling is verified against the starting inode at the end.
std::string directory_name(const Filesystem &filesystem, const Path &directory) {
    auto root = filesystem.resolve("/");
    auto cursor = filesystem.directory_anchor(directory.parent.get(), directory.guest, directory.mount,
                                               directory.node, directory.virtual_components);
    std::vector<std::string> names;
    for (size_t depth = 0; depth < max_path; ++depth) {
        if (same_directory(cursor, root)) {
            std::string result = "/";
            for (auto it = names.rbegin(); it != names.rend(); ++it) {
                if (result.size() != 1)
                    result += '/';
                result += *it;
            }
            if (result.size() >= max_path)
                guest_error(36);
            if (!same_directory(filesystem.resolve(result), directory))
                guest_error(11);
            return result;
        }
        auto parent = filesystem.resolve_at("..", cursor);
        if (same_directory(parent, cursor))
            guest_error(2); // Detached or unlinked cwd is not reachable from '/'.
        std::string name;
        if (cursor.mount_id != parent.mount_id) {
            if (!cursor.mount)
                guest_error(11);
            const auto &target = cursor.mount->target;
            name = target.substr(target.rfind('/') + 1);
        } else if (cursor.virtual_path) {
            if (cursor.virtual_components.empty())
                guest_error(11);
            name = cursor.virtual_components.back();
        } else
            name = native_directory_name(parent, cursor);
        if (name.empty() || !same_directory(filesystem.resolve_at(name, parent), cursor))
            guest_error(11);
        names.push_back(std::move(name));
        cursor = std::move(parent);
    }
    guest_error(36);
}
} // namespace
std::string files::directory_guest_name(const Filesystem &filesystem, const Path &directory) {
    return directory_name(filesystem, directory);
}
Path Syscalls::directory_path(uint64_t fd) {
    if (static_cast<int32_t>(fd) == -100) {
        auto directory = std::atomic_load(&fs_context_->directory);
        if (!directory) {
            auto initial = filesystem_.resolve(fs_context_->cwd);
            auto candidate = std::make_shared<Path>(filesystem_.resolve_at(".", initial));
            if (std::atomic_compare_exchange_strong(&fs_context_->directory, &directory, candidate))
                directory = std::move(candidate);
        }
        return filesystem_.directory_anchor(directory->parent.get(), directory->guest, directory->mount,
                                             directory->node, directory->virtual_components);
    }
    auto &entry = descriptors_.get(static_cast<uint32_t>(fd));
    if (!S_ISDIR(descriptor_stat(entry).st_mode))
        guest_error(20);
    auto node = entry.host.get() < 0 && entry.description->virtual_file
                    ? entry.description->virtual_file->node : VirtualNode::Ptr{};
    return filesystem_.directory_anchor(entry.host.get(), entry.path.empty() ? "/" : entry.path,
                                         entry.description->mount, std::move(node));
}
std::string Syscalls::directory_base(uint64_t fd) {
    auto directory = directory_path(fd);
    return directory_name(filesystem_, directory);
}
Path Syscalls::resolve_path(uint64_t fd, const std::string &name, bool follow,
                            Filesystem::ResolveIntent intent) {
    if (name.empty())
        guest_error(2);
    Path directory;
    if (name.front() != '/')
        directory = directory_path(fd);
    return filesystem_.resolve_at(name, directory, 0, follow, intent);
}
Path Syscalls::at_path(uint64_t fd, uint64_t address, bool follow) {
    return resolve_path(fd, memory_.string(address), follow);
}
void Syscalls::guest_stat(uint64_t destination, const struct stat &info) {
    check_buffer(destination, sizeof(linux_abi::Stat), write_perm);
    linux_abi::Stat value{};
    value.device = info.st_dev;
    value.inode = info.st_ino;
    value.mode = info.st_mode;
    value.links = info.st_nlink;
    value.uid = info.st_uid;
    value.gid = info.st_gid;
    value.rdevice = info.st_rdev;
    value.size = info.st_size;
    value.block_size = info.st_blksize;
    value.blocks = info.st_blocks;
#ifdef __APPLE__
    value.atime = info.st_atimespec.tv_sec;
    value.atime_ns = info.st_atimespec.tv_nsec;
    value.mtime = info.st_mtimespec.tv_sec;
    value.mtime_ns = info.st_mtimespec.tv_nsec;
    value.ctime = info.st_ctimespec.tv_sec;
    value.ctime_ns = info.st_ctimespec.tv_nsec;
#else
    value.atime = info.st_atim.tv_sec;
    value.atime_ns = info.st_atim.tv_nsec;
    value.mtime = info.st_mtim.tv_sec;
    value.mtime_ns = info.st_mtim.tv_nsec;
    value.ctime = info.st_ctim.tv_sec;
    value.ctime_ns = info.st_ctim.tv_nsec;
#endif
    memory_.copy_in(destination, &value, sizeof(value));
}
int64_t Syscalls::dispatch_file(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (a[8]) {
    case 5:    // setxattr
    case 6:    // lsetxattr
    case 8:    // getxattr
    case 9:    // lgetxattr
    case 11:   // listxattr
    case 12:   // llistxattr
    case 14:   // removexattr
    case 15: { // lremovexattr
        const bool follow = a[8] != 6 && a[8] != 9 && a[8] != 12 && a[8] != 15;
        auto path = resolve_path(uint32_t(-100), memory_.string(a[0]), follow);
        return files::xattr(memory_, a[8], a, &path, nullptr);
    }
    case 7:  // fsetxattr
    case 10: // fgetxattr
    case 13: // flistxattr
    case 16: {
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        if (entry.description->flags & files::arm64_open::path)
            guest_error(9);
        return files::xattr(memory_, a[8], a, nullptr, &entry);
    }
    case 43: { // statfs
        auto path = resolve_path(uint32_t(-100), memory_.string(a[0]));
        files::statfs(memory_, a[1], path);
        return 0;
    }
    case 44: { // fstatfs
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        files::statfs(memory_, a[1], entry);
        return 0;
    }
    case 291: { // statx
        const uint32_t flags = a[2];
        if ((flags & ~(256u | 2048u | 4096u | 0x6000u)) || (flags & 0x6000u) == 0x6000u)
            guest_error(22);
        const std::string name = !a[1] && (flags & 4096) ? std::string{} : memory_.string(a[1]);
        if (name.empty() && (flags & 4096) && int32_t(a[0]) != -100) {
            auto &entry = descriptors_.get(uint32_t(a[0]));
            files::statx(memory_, a[4], uint32_t(a[3]), descriptor_stat(entry), nullptr, &entry);
        } else {
            if (name.empty() && !(flags & 4096))
                guest_error(2);
            auto path = name.empty() ? directory_path(a[0])
                                     : resolve_path(a[0], name, !(flags & 256));
            auto info = path.node && path.node->open_stream && !(flags & 256) ? path.node->stream_stat()
                                                                              : path.stat();
            files::statx(memory_, a[4], uint32_t(a[3]), info, &path);
        }
        return 0;
    }
    case 17: // getcwd
        fs_context_->cwd = directory_base(uint32_t(-100));
        if (a[1] < fs_context_->cwd.size() + 1)
            guest_error(34);
        check_buffer(a[0], fs_context_->cwd.size() + 1, write_perm);
        memory_.copy_in(a[0], fs_context_->cwd.c_str(), fs_context_->cwd.size() + 1);
        return fs_context_->cwd.size() + 1;
    case 23:
        return descriptors_.duplicate(uint32_t(a[0]));
    case 24:
        if (a[2] & ~uint64_t{524288})
            guest_error(22);
        return descriptors_.duplicate_to(
            uint32_t(a[0]), uint32_t(a[1]), a[2] & 524288, [this](const Descriptor &entry) {
                files::record_lock_close(scheduler_, scheduler_.current().pid, entry);
                ipc::posix_mq_close(entry.description, scheduler_.current().pid);
            });
    case 25:
        return descriptor_control(cpu);
    case 29:
        return descriptor_ioctl(cpu);
    case 32: { // flock
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (entry.description->flags & files::arm64_open::path)
            guest_error(9);
        const uint32_t command = uint32_t(a[1]);
        const bool nonblocking = command & 4;
        int operation;
        switch (command & ~uint32_t{4}) {
        case 1:
            operation = LOCK_SH;
            break;
        case 2:
            operation = LOCK_EX;
            break;
        case 8:
            operation = LOCK_UN;
            break;
        default:
            guest_error(22);
        }
        if (entry.host.get() < 0 || entry.description->virtual_file ||
            entry.description->is_anon_inode())
            guest_error(95);
        auto attempt = [operation, nonblocking](int fd) -> std::optional<int64_t> {
            if (::flock(fd, operation | LOCK_NB) == 0)
                return 0;
            const int error = linux_errno(errno);
            if (error == 11 && !nonblocking && operation != LOCK_UN)
                return {};
            return -error;
        };
        if (auto result = attempt(entry.host.get()))
            return *result;
        if (interrupt_for_pending_signal())
            return -4;
        FD host(::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
        check_host(host.get());
        auto held = std::make_shared<Descriptor>(std::move(host), entry, true);
        scheduler_.block({}, -4, [attempt, held]() { return attempt(held->host.get()); });
        return 0;
    }
    case 34: { // mkdirat
        const std::string name = memory_.string(a[1]);
        if (name.empty())
            guest_error(2);
        auto path = resolve_path(a[0], name, false, Filesystem::ResolveIntent::CreateDirectory);
        const mode_t mode = (a[2] & 0777) & ~fs_context_->mask;
        create_guest_directory(path, mode);
        return 0;
    }
    case 35: { // unlinkat
        if (a[2] & ~uint64_t{512})
            guest_error(22);
        auto path = at_path(a[0], a[1], false);
        path.require_removable();
        if (::unlinkat(path.parent.get(), path.name.c_str(), (a[2] & 512) ? AT_REMOVEDIR : 0) < 0) {
            const int error = errno;
            // Darwin reports EPERM for unlinking a directory. Linux callers
            // need EISDIR to select recursive directory removal.
            struct stat info{};
            if (error == EPERM && !(a[2] & 512) &&
                ::fstatat(path.parent.get(), path.name.c_str(), &info, AT_SYMLINK_NOFOLLOW) == 0 &&
                S_ISDIR(info.st_mode))
                guest_error(21);
            guest_error(linux_errno(error));
        }
        InotifyOpenFile::unlinked(path);
        return 0;
    }
    case 38: { // renameat
        auto source = at_path(a[0], a[1], false);
        auto target = at_path(a[2], a[3], false);
        source.require_removable();
        target.require_removable();
        if (source.mount_id != target.mount_id)
            guest_error(18);
        check_host(
            ::renameat(source.parent.get(), source.name.c_str(), target.parent.get(), target.name.c_str()));
        InotifyOpenFile::renamed(source, target);
        return 0;
    }
    case 46: {
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        if (entry.description->flags & files::arm64_open::path)
            guest_error(9);
        if (int64_t(a[1]) < 0 || entry.description->is_anon_inode())
            guest_error(22);
        entry.description->require_writable();
        if (!files::descriptor_writable(entry))
            guest_error(22);
        auto seal_guard = files::memory_file_check_truncate(entry.host.get(), a[1]);
        check_host(::ftruncate(entry.host.get(), int64_t(a[1])));
        return 0;
    }
    case 49: { // chdir
        auto resolved = resolve_path(uint32_t(-100), memory_.string(a[0]));
        auto directory = filesystem_.resolve_at(".", resolved);
        fs_context_->cwd = directory.guest;
        std::atomic_store(&fs_context_->directory, std::make_shared<Path>(std::move(directory)));
        return 0;
    }
    case 50: { // fchdir also accepts an O_PATH directory descriptor.
        auto directory = directory_path(a[0]);
        fs_context_->cwd = directory.guest;
        std::atomic_store(&fs_context_->directory, std::make_shared<Path>(std::move(directory)));
        return 0;
    }
    case 56: // openat
    case 437: // openat2; central dispatch may also route directly to dispatch_open.
        return dispatch_open(cpu);
    case 57: {
        auto &entry = descriptors_.get(uint32_t(a[0]));
        files::record_lock_close(scheduler_, scheduler_.current().pid, entry);
        ipc::posix_mq_close(entry.description, scheduler_.current().pid);
        descriptors_.close(uint32_t(a[0]));
        return 0;
    }
    case 61: { // getdents64
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (entry.description->flags & files::arm64_open::path)
            guest_error(9);
        if (entry.description->is_anon_inode())
            guest_error(20);
        size_t capacity = static_cast<size_t>(std::min<uint64_t>(a[2], max_io));
        check_buffer(a[1], capacity, write_perm);
        if (entry.description->virtual_file)
            return virtual_getdents(entry, a[1], capacity);
        if (!entry.description->directory) {
            int fd = ::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0);
            check_host(fd);
            entry.description->directory = ::fdopendir(fd);
            if (!entry.description->directory) {
                int error = errno;
                ::close(fd);
                guest_error(linux_errno(error));
            }
        }
        if (entry.description->mount_children.empty())
            entry.description->mount_children = filesystem_.mounted_children(directory_base(a[0]));
        std::vector<uint8_t> data(capacity);
        size_t used = 0;
        bool native_end = false;
        for (;;) {
            const long offset = ::telldir(entry.description->directory);
            errno = 0;
            auto *item = ::readdir(entry.description->directory);
            if (!item) {
                if (errno && !used)
                    guest_error(linux_errno(errno));
                native_end = !errno;
                break;
            }
            bool mounted = false;
            for (const auto &name : entry.description->mount_children)
                mounted |= name == item->d_name;
            if (mounted)
                continue;
            size_t name_size = std::strlen(item->d_name) + 1;
            size_t record_size = (19 + name_size + 7) & ~size_t{7};
            if (record_size > capacity - used) {
                ::seekdir(entry.description->directory, offset);
                if (!used)
                    guest_error(22);
                break;
            }
            uint64_t inode = item->d_ino, next = ::telldir(entry.description->directory);
            uint16_t length = record_size;
            std::memcpy(data.data() + used, &inode, 8);
            std::memcpy(data.data() + used + 8, &next, 8);
            std::memcpy(data.data() + used + 16, &length, 2);
            data[used + 18] = item->d_type;
            std::memcpy(data.data() + used + 19, item->d_name, name_size);
            used += record_size;
        }
        while (native_end && entry.description->mount_cursor < entry.description->mount_children.size()) {
            const auto &name = entry.description->mount_children[entry.description->mount_cursor];
            const size_t record_size = (19 + name.size() + 1 + 7) & ~size_t{7};
            if (record_size > capacity - used) {
                if (!used)
                    guest_error(22);
                break;
            }
            auto path = filesystem_.resolve_at(name, directory_path(a[0]));
            struct stat info = path.stat();
            const uint64_t inode = info.st_ino;
            const uint64_t offset = (uint64_t{1} << 62) | (entry.description->mount_cursor + 1);
            const uint16_t length = record_size;
            std::memcpy(data.data() + used, &inode, 8);
            std::memcpy(data.data() + used + 8, &offset, 8);
            std::memcpy(data.data() + used + 16, &length, 2);
            data[used + 18] = S_ISDIR(info.st_mode)   ? DT_DIR
                              : S_ISLNK(info.st_mode) ? DT_LNK
                              : S_ISCHR(info.st_mode) ? DT_CHR
                                                      : DT_REG;
            std::memcpy(data.data() + used + 19, name.c_str(), name.size() + 1);
            used += record_size;
            ++entry.description->mount_cursor;
        }
        if (entry.description->inotify_open)
            entry.description->inotify_open->accessed();
        memory_.copy_in(a[1], data.data(), used);
        return used;
    }
    case 62: {
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (entry.description->flags & files::arm64_open::path)
            guest_error(9);
        if (entry.description->inotify) {
            if (uint32_t(a[2]) > 4)
                guest_error(22);
            // Linux inotify uses noop_llseek: no event position changes and
            // the readiness marker must never receive a native seek.
            return 0;
        }
        if (a[2] > 2)
            guest_error(22);
        if (entry.description->is_anon_inode())
            guest_error(29);
        auto seal_guard = files::memory_file_lock(entry.host.get());
        if (entry.description->virtual_file)
            return entry.description->virtual_file->seek(int64_t(a[1]), a[2]);
        if (entry.description->directory && a[2] == 0) {
            if (int64_t(a[1]) < 0)
                guest_error(22);
            if (a[1] & (uint64_t{1} << 62)) {
                size_t cursor = a[1] & ~(uint64_t{1} << 62);
                if (cursor > entry.description->mount_children.size())
                    guest_error(22);
                entry.description->mount_cursor = cursor;
            } else {
                ::seekdir(entry.description->directory, a[1]);
                entry.description->mount_cursor = 0;
            }
            return a[1];
        }
        off_t result = ::lseek(entry.host.get(), int64_t(a[1]), int(a[2]));
        if (result < 0)
            guest_error(linux_errno(errno));
        return result;
    }
    case 63:
    case 64:
        return io(a[8] == 64, a[0], a[1], a[2]);
    case 65:  // readv
    case 66:  // writev
    case 69:  // preadv
    case 70:  // pwritev
    case 286: // preadv2
    case 287: // pwritev2
        return vector_io(cpu);
    case 67:
    case 68:
        return io(a[8] == 68, a[0], a[1], a[2], a[3], true);
    case 78: { // readlinkat; empty names address an O_PATH symlink itself.
        if (!a[3])
            guest_error(22);
        const auto name = memory_.string(a[1]);
        std::string target;
        if (name.empty()) {
            if (int32_t(a[0]) == -100)
                guest_error(2);
            auto &entry = descriptors_.get(uint32_t(a[0]));
            if (!S_ISLNK(descriptor_stat(entry).st_mode))
                guest_error(22);
            auto file = entry.description->virtual_file;
            if (!file || !file->node->target)
                guest_error(22);
            target = file->node->target();
        } else
            target = resolve_path(a[0], name, false).readlink();
        size_t size = std::min<uint64_t>(a[3], max_path);
        check_buffer(a[2], size, write_perm);
        size_t length = std::min(size, target.size());
        memory_.copy_in(a[2], target.data(), length);
        return length;
    }
    case 79: { // newfstatat
        if (a[3] & ~uint64_t{256 | 4096})
            guest_error(22);
        std::string name = !a[1] && (a[3] & 4096) ? std::string{} : memory_.string(a[1]);
        struct stat info{};
        if (name.empty() && (a[3] & 4096)) {
            info = int32_t(a[0]) == -100 ? directory_path(a[0]).stat()
                                        : descriptor_stat(descriptors_.get(uint32_t(a[0])));
        } else {
            if (name.empty())
                guest_error(2);
            auto path = resolve_path(a[0], name, !(a[3] & 256));
            info =
                path.node && path.node->open_stream && !(a[3] & 256) ? path.node->stream_stat() : path.stat();
        }
        guest_stat(a[2], info);
        return 0;
    }
    case 80: {
        struct stat info{};
        info = descriptor_stat(descriptors_.get(uint32_t(a[0])));
        guest_stat(a[1], info);
        return 0;
    }
    default:
        break;
    }
    return unsupported(cpu);
}
} // namespace operit::arm64
