#include "linux/files/metadata.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <array>
#include <cerrno>
#include <climits>
#include <cstring>
#include <fcntl.h>
#include <sys/statvfs.h>
#include <sys/xattr.h>
#ifdef __APPLE__
#include <sys/mount.h>
#else
#include <sys/sysmacros.h>
#include <sys/vfs.h>
#endif

namespace operit::arm64::files {
namespace {
struct LinuxStatfs {
    int64_t type, block_size;
    uint64_t blocks, free_blocks, available_blocks, files, free_files;
    int32_t fsid[2];
    int64_t name_length, fragment_size, flags, spare[4];
};
struct Timestamp {
    int64_t seconds;
    uint32_t nanoseconds, reserved;
};
struct LinuxStatx {
    uint32_t mask, block_size;
    uint64_t attributes;
    uint32_t links, uid, gid;
    uint16_t mode, spare;
    uint64_t inode, size, blocks, attributes_mask;
    Timestamp atime, btime, ctime, mtime;
    uint32_t rdev_major, rdev_minor, dev_major, dev_minor;
    uint64_t mount_id;
    uint32_t dio_memory_alignment, dio_offset_alignment;
    uint64_t spare2[12];
};
static_assert(sizeof(LinuxStatfs) == 120);
static_assert(sizeof(LinuxStatx) == 256);
void require_memory(Memory &memory, uint64_t address, uint64_t size, unsigned permissions) {
    if (!memory.accessible(address, size, permissions))
        guest_error(14);
}
FD path_descriptor(const Path &path, bool attributes = false) {
    if (path.virtual_path)
        guest_error(path.node ? 95 : 2);
    // O_EVTONLY obtains metadata without imposing file-content read permission.
    // O_SYMLINK opens the final symlink itself, so a concurrent replacement
    // cannot make a metadata operation follow it outside the guest namespace.
#ifdef __APPLE__
    (void)attributes;
    int flags = O_EVTONLY | O_SYMLINK | O_NOFOLLOW | O_CLOEXEC;
#else
    int flags = (attributes ? O_RDONLY | O_NONBLOCK : O_PATH) | O_NOFOLLOW | O_CLOEXEC;
#endif
    FD result(::openat(path.parent.get(), path.name.c_str(), flags));
    check_host(result.get());
    return result;
}
uint64_t mount_flags(const struct statfs &info, bool read_only) {
    uint64_t value = read_only ? 1 : 0;
#ifdef __APPLE__
    if (info.f_flags & MNT_RDONLY)
        value |= 1;
    if (info.f_flags & MNT_NOSUID)
        value |= 2;
    if (info.f_flags & MNT_NODEV)
        value |= 4;
    if (info.f_flags & MNT_NOEXEC)
        value |= 8;
    if (info.f_flags & MNT_SYNCHRONOUS)
        value |= 16;
    if (info.f_flags & MNT_NOATIME)
        value |= 1024;
#else
    value |= info.f_flags;
#endif
    return value;
}
void native_statfs(Memory &memory, uint64_t address, int fd, bool read_only) {
    struct statfs info{};
    check_host(::fstatfs(fd, &info));
    LinuxStatfs result{};
    result.type = info.f_type;
#ifdef __APPLE__
    if (!std::strcmp(info.f_fstypename, "nfs"))
        result.type = 0x6969;
    else if (!std::strcmp(info.f_fstypename, "msdos"))
        result.type = 0x4d44;
    else if (!std::strcmp(info.f_fstypename, "exfat"))
        result.type = 0x2011bab0;
    result.fsid[0] = info.f_fsid.val[0];
    result.fsid[1] = info.f_fsid.val[1];
    result.name_length = 255;
#else
    result.fsid[0] = info.f_fsid.__val[0];
    result.fsid[1] = info.f_fsid.__val[1];
    result.name_length = info.f_namelen;
#endif
    result.block_size = info.f_bsize;
    result.fragment_size = info.f_bsize;
    result.blocks = info.f_blocks;
    result.free_blocks = info.f_bfree;
    result.available_blocks = info.f_bavail;
    result.files = info.f_files;
    result.free_files = info.f_ffree;
    result.flags = mount_flags(info, read_only);
    memory.copy_in(address, &result, sizeof(result));
}
void anonymous_statfs(Memory &memory, uint64_t address) {
    LinuxStatfs result{};
    result.type = 0x09041934;
    result.block_size = result.fragment_size = page_size;
    result.fsid[0] = 0x7003;
    result.name_length = 255;
    result.flags = 32;
    memory.copy_in(address, &result, sizeof(result));
}
void virtual_statfs(Memory &memory, uint64_t address, const VirtualNode::Ptr &node, bool read_only) {
    if (!node)
        guest_error(2);
    LinuxStatfs result{};
    const bool proc = node->identity.compare(0, 5, "/proc") == 0;
    // These node filesystems do not allocate blocks or persistent inodes.
    const bool devices = node->identity == "/dev" || node->identity.compare(0, 5, "/dev/") == 0;
    result.type = proc ? 0x9fa0 : (devices ? 0x1373 : 0x858458f6);
    result.block_size = result.fragment_size = page_size;
    result.name_length = 255;
    result.fsid[0] = proc ? 0x7001 : 0x7002;
    result.flags = 2 | 8 | (read_only ? 1 : 0);
    memory.copy_in(address, &result, sizeof(result));
}
uint32_t device_major(dev_t value, bool virtual_device) {
    if (virtual_device)
        return (uint64_t(value) >> 8) & 0xfff;
    return static_cast<uint32_t>(major(value));
}
uint32_t device_minor(dev_t value, bool virtual_device) {
    if (virtual_device)
        return (uint64_t(value) & 0xff) | ((uint64_t(value) >> 12) & 0xfffff00);
    return static_cast<uint32_t>(minor(value));
}
int xattr_error(int error) {
#ifdef __APPLE__
    if (error == ENOATTR)
        return 61;
#endif
    return linux_errno(error);
}
// Only Linux's user namespace has a direct, unprivileged Darwin equivalent.
// Exposing native security/trusted namespaces would imply Linux LSM/capability
// checks that the host xattr API does not implement.
void validate_name(const std::string &name) {
    if (name.empty())
        guest_error(34);
    if (name.size() > 255)
        guest_error(34);
    if (name.compare(0, 5, "user.") || name.size() == 5)
        guest_error(95);
}
std::vector<uint8_t> attribute_list(int fd) {
    for (;;) {
#ifdef __APPLE__
        const ssize_t size = ::flistxattr(fd, nullptr, 0, 0);
#else
        const ssize_t size = ::flistxattr(fd, nullptr, 0);
#endif
        if (size < 0)
            guest_error(xattr_error(errno));
        if (size > 65536)
            guest_error(7);
        std::vector<char> native(static_cast<size_t>(size));
        if (size) {
#ifdef __APPLE__
            const ssize_t read = ::flistxattr(fd, native.data(), native.size(), 0);
#else
            const ssize_t read = ::flistxattr(fd, native.data(), native.size());
#endif
            if (read < 0) {
                if (errno == ERANGE)
                    continue;
                guest_error(xattr_error(errno));
            }
            native.resize(read);
        }
        std::vector<uint8_t> result;
        for (size_t position = 0; position < native.size();) {
            const size_t left = native.size() - position;
            const size_t length = strnlen(native.data() + position, left);
            if (length == left)
                guest_error(5);
            if (length > 5 && !std::memcmp(native.data() + position, "user.", 5))
                result.insert(result.end(), native.data() + position, native.data() + position + length + 1);
            position += length + 1;
        }
        return result;
    }
}
} // namespace
void statfs(Memory &memory, uint64_t address, const Path &path) {
    require_memory(memory, address, sizeof(LinuxStatfs), write_perm);
    if (path.virtual_path) {
        virtual_statfs(memory, address, path.node, path.is_read_only());
        return;
    }
    auto fd = path_descriptor(path);
    native_statfs(memory, address, fd.get(), path.is_read_only());
}
void statfs(Memory &memory, uint64_t address, const Descriptor &entry) {
    require_memory(memory, address, sizeof(LinuxStatfs), write_perm);
    if (entry.description->is_anon_inode()) {
        anonymous_statfs(memory, address);
        return;
    }
    if (entry.description->virtual_file) {
        const bool read_only = entry.description->mount ? entry.description->mount->read_only()
                                                        : entry.description->is_read_only();
        virtual_statfs(memory, address, entry.description->virtual_file->node, read_only);
        return;
    }
    native_statfs(memory, address, entry.host.get(), entry.description->is_read_only());
}
void statx(Memory &memory, uint64_t address, uint32_t requested, const struct stat &info, const Path *path,
           const Descriptor *entry) {
    require_memory(memory, address, sizeof(LinuxStatx), write_perm);
    if (requested & 0x80000000u)
        guest_error(22);
    LinuxStatx result{};
    result.mask = 0x7ff; // STATX_BASIC_STATS: every field below comes from stat.
    result.block_size = info.st_blksize;
    result.links = info.st_nlink;
    result.uid = info.st_uid;
    result.gid = info.st_gid;
    result.mode = info.st_mode;
    result.inode = info.st_ino;
    result.size = info.st_size;
    result.blocks = info.st_blocks;
    const auto node =
        path ? path->node
             : (entry && entry->description->virtual_file ? entry->description->virtual_file->node : nullptr);
    const bool guest_device = node || (entry && entry->description->is_anon_inode());
    const bool virtual_device = node && node->device();
    result.dev_major = device_major(info.st_dev, guest_device);
    result.dev_minor = device_minor(info.st_dev, guest_device);
    result.rdev_major = device_major(info.st_rdev, virtual_device);
    result.rdev_minor = device_minor(info.st_rdev, virtual_device);
#ifdef __APPLE__
    result.atime = {info.st_atimespec.tv_sec, uint32_t(info.st_atimespec.tv_nsec), 0};
    result.ctime = {info.st_ctimespec.tv_sec, uint32_t(info.st_ctimespec.tv_nsec), 0};
    result.mtime = {info.st_mtimespec.tv_sec, uint32_t(info.st_mtimespec.tv_nsec), 0};
    if (!guest_device && (requested & 0x800)) {
        result.btime = {info.st_birthtimespec.tv_sec, uint32_t(info.st_birthtimespec.tv_nsec), 0};
        result.mask |= 0x800;
    }
    if (!guest_device) {
        result.attributes_mask = 0x4 | 0x10 | 0x20 | 0x40;
        if (info.st_flags & UF_COMPRESSED)
            result.attributes |= 0x4;
        if (info.st_flags & (UF_IMMUTABLE | SF_IMMUTABLE))
            result.attributes |= 0x10;
        if (info.st_flags & (UF_APPEND | SF_APPEND))
            result.attributes |= 0x20;
        if (info.st_flags & UF_NODUMP)
            result.attributes |= 0x40;
    }
#else
    result.atime = {info.st_atim.tv_sec, uint32_t(info.st_atim.tv_nsec), 0};
    result.ctime = {info.st_ctim.tv_sec, uint32_t(info.st_ctim.tv_nsec), 0};
    result.mtime = {info.st_mtim.tv_sec, uint32_t(info.st_mtim.tv_nsec), 0};
#endif
    if (path) {
        result.mount_id = path->mount_id + 1;
        result.mask |= 0x1000;
        result.attributes_mask |= 0x2000;
        if (path->mount_root || path->guest == "/")
            result.attributes |= 0x2000;
    }
    memory.copy_in(address, &result, sizeof(result));
}
int64_t xattr(Memory &memory, uint64_t operation, const std::array<uint64_t, 31> &a, const Path *path,
              const Descriptor *entry) {
    if (operation < 5 || operation > 16)
        guest_error(22);
    const bool set = operation <= 7;
    const bool list = operation >= 11 && operation <= 13;
    const bool remove = operation >= 14;
    const bool mutating = set || remove;
    if (entry && entry->description->is_anon_inode()) {
        if (list)
            return 0;
        guest_error(95);
    }
    if (mutating) {
        if (path)
            path->require_writable();
        if (entry)
            entry->description->require_writable();
    }
    const auto node =
        path ? path->node
             : (entry && entry->description->virtual_file ? entry->description->virtual_file->node : nullptr);
    if ((path && path->virtual_path) || (entry && entry->description->virtual_file)) {
        if (!node)
            guest_error(2);
        if (mutating)
            guest_error(95);
        if (list)
            return 0; // The virtual node contains no extended attributes.
        guest_error(61);
    }
    const std::string name = list ? std::string{} : memory.string(a[1]);
    if (!list)
        validate_name(name);
    if (path && S_ISLNK(path->stat().st_mode)) {
        if (list)
            return 0;
        guest_error(mutating ? 1 : 61);
    }
    FD owned;
    const int fd = entry ? entry->host.get() : (owned = path_descriptor(*path, true), owned.get());
    if (remove) {
#ifdef __APPLE__
        int result = ::fremovexattr(fd, name.c_str(), 0);
#else
        int result = ::fremovexattr(fd, name.c_str());
#endif
        if (result < 0)
            guest_error(xattr_error(errno));
        return 0;
    }
    const uint64_t address = a[list ? 1 : 2];
    const uint64_t requested_size = a[list ? 2 : 3];
    if (set && requested_size > 65536)
        guest_error(7);
    const uint64_t size = std::min<uint64_t>(requested_size, 65536);
    if (set && size)
        require_memory(memory, address, size, read_perm);
    if (list) {
        auto names = attribute_list(fd);
        if (!size)
            return names.size();
        if (size < names.size())
            guest_error(34);
        require_memory(memory, address, names.size(), write_perm);
        memory.copy_in(address, names.data(), names.size());
        return names.size();
    }
    std::vector<uint8_t> data(size);
    if (set) {
        if (a[4] & ~uint64_t{3} || (a[4] & 3) == 3)
            guest_error(22);
        if (size)
            memory.copy_out(address, data.data(), size);
        int flags = 0;
        if (a[4] & 1)
            flags |= XATTR_CREATE;
        if (a[4] & 2)
            flags |= XATTR_REPLACE;
#ifdef __APPLE__
        int result = ::fsetxattr(fd, name.c_str(), data.data(), data.size(), 0, flags);
#else
        int result = ::fsetxattr(fd, name.c_str(), data.data(), data.size(), flags);
#endif
        if (result < 0)
            guest_error(xattr_error(errno));
        return 0;
    }
#ifdef __APPLE__
    ssize_t result = ::fgetxattr(fd, name.c_str(), size ? data.data() : nullptr, size, 0, 0);
#else
    ssize_t result = ::fgetxattr(fd, name.c_str(), size ? data.data() : nullptr, size);
#endif
    if (result < 0)
        guest_error(xattr_error(errno));
    if (size && result) {
        require_memory(memory, address, result, write_perm);
        memory.copy_in(address, data.data(), result);
    }
    return result;
}
} // namespace operit::arm64::files
