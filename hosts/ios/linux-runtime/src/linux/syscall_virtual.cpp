#include "linux/error_mapping.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/memory_file.h"
#include "linux/syscalls.h"
#include <dirent.h>
#include <fcntl.h>

namespace operit::arm64 {
struct stat Syscalls::descriptor_stat(const Descriptor &entry) {
    if (ipc::is_posix_mq_file(entry.description))
        guest_error(95); // Guest mqueue metadata is not the readiness pipe's metadata.
    if (entry.description->terminal)
        return terminals_->stat(*entry.description->terminal);
    if (entry.host.get() < 0 && entry.description->virtual_file)
        return entry.description->virtual_file->node->stat();
    struct stat info{};
    if (entry.description->is_anon_inode()) {
        info.st_dev = 0x7003;
        info.st_ino = entry.description->inode;
        info.st_mode = 0600; // Linux anon_inode objects have permissions without an S_IF* type.
        info.st_nlink = 1;
        info.st_uid = ::getuid();
        info.st_gid = ::getgid();
        info.st_blksize = 4096;
        return info;
    }
    check_host(::fstat(entry.host.get(), &info));
    return info;
}
int Syscalls::open_virtual(const Path &path, int host_flags, int guest_flags) {
    auto node = path.node;
    if (!node)
        guest_error((host_flags & O_CREAT) ? 30 : 2);
    if (guest_flags & files::arm64_open::temporary)
        guest_error(95);
    if (guest_flags & files::arm64_open::path) {
        if (node->open_stream && !(guest_flags & files::arm64_open::no_follow)) {
            // A followed proc magic link pins its target, not the proc symlink.
            // The callback receives a native metadata-only open request.
            FD host = node->open_stream(host_flags);
            struct stat info{};
            check_host(::fstat(host.get(), &info));
            if ((guest_flags & files::arm64_open::directory) && !S_ISDIR(info.st_mode))
                guest_error(20);
            int fd = descriptors_.insert(std::move(host), "", guest_flags);
            try {
                auto &entry = descriptors_.get(fd);
                entry.description->mount = path.mount;
                files::memory_file_attach(entry);
            } catch (...) {
                descriptors_.close(fd);
                throw;
            }
            return fd;
        }
        if ((guest_flags & files::arm64_open::directory) && node->kind != VirtualNode::Kind::Directory)
            guest_error(20);
        // Retain the node itself, including a final nofollow symlink. Do not
        // activate devices, terminals, or a proc magic link's stream callback.
        int fd = descriptors_.insert(FD{}, path.guest, guest_flags);
        try {
            auto &description = *descriptors_.get(fd).description;
            description.mount = path.mount;
            description.virtual_file = std::make_shared<VirtualOpen>(node);
            description.read_only = true;
        } catch (...) {
            descriptors_.close(fd);
            throw;
        }
        return fd;
    }
    if ((host_flags & O_CREAT) && (host_flags & O_EXCL))
        guest_error(17);
    if (host_flags & O_NOFOLLOW && node->kind == VirtualNode::Kind::Symlink)
        guest_error(40);
    if ((host_flags & O_DIRECTORY) && node->kind != VirtualNode::Kind::Directory)
        guest_error(20);
    if (auto terminal = terminals_->open_node(*node, guest_flags, scheduler_.current().pid)) {
        int fd = descriptors_.insert(std::move(terminal->host), terminal->path, guest_flags, 0);
        auto &description = *descriptors_.get(fd).description;
        description.mount = path.mount;
        description.terminal = std::move(terminal->endpoint);
        description.scheduler_io = true;
        terminals_->opened(*description.terminal, scheduler_.current().pid, guest_flags & 256);
        return fd;
    }
    if (node->open_stream) {
        int stream_flags = host_flags & ~O_TRUNC;
        if ((host_flags & O_TRUNC) && (host_flags & O_ACCMODE) == O_RDONLY)
            stream_flags = (stream_flags & ~O_ACCMODE) | O_RDWR;
        FD host = node->open_stream(stream_flags);
        if (host_flags & O_TRUNC) {
            if (path.is_read_only())
                guest_error(30);
            struct stat info{};
            check_host(::fstat(host.get(), &info));
            if (S_ISDIR(info.st_mode))
                guest_error(21);
            if (S_ISREG(info.st_mode)) {
                auto seal_guard = files::memory_file_check_truncate(host.get(), 0);
                check_host(::ftruncate(host.get(), 0));
            }
        }
        int fd = descriptors_.insert(std::move(host), "", guest_flags, 0);
        try {
            auto &entry = descriptors_.get(fd);
            entry.description->mount = path.mount;
            entry.description->standard_stream = node->standard_stream;
            entry.description->scheduler_io = true;
            files::memory_file_attach(entry);
        } catch (...) {
            descriptors_.close(fd);
            throw;
        }
        return fd;
    }
    if (node->kind == VirtualNode::Kind::Symlink)
        guest_error(40);
    if (node->kind == VirtualNode::Kind::Directory && (host_flags & O_ACCMODE) != O_RDONLY)
        guest_error(21);
    if (!node->device() && !node->write_contents &&
        ((host_flags & O_ACCMODE) != O_RDONLY || (host_flags & O_TRUNC)))
        guest_error(13);
    int fd = descriptors_.insert(FD{}, path.guest, guest_flags, 0);
    auto &description = *descriptors_.get(fd).description;
    description.mount = path.mount;
    description.virtual_file = std::make_shared<VirtualOpen>(node);
    description.read_only = true; // Metadata belongs to the mounted virtual filesystem.
    return fd;
}
int64_t Syscalls::virtual_getdents(Descriptor &entry, uint64_t address, size_t capacity) {
    files::require_data_descriptor(entry);
    auto &file = *entry.description->virtual_file;
    auto directory = filesystem_.directory_anchor(-1, entry.path, entry.description->mount, file.node);
    if (file.node->kind != VirtualNode::Kind::Directory)
        guest_error(20);
    if (file.directory_names.empty()) {
        file.directory_names = {".", ".."};
        if (file.node->names) {
            auto names = file.node->names();
            file.directory_names.insert(file.directory_names.end(), names.begin(), names.end());
        }
        auto mounted = filesystem_.mounted_children(files::directory_guest_name(filesystem_, directory));
        for (const auto &name : mounted)
            if (std::find(file.directory_names.begin(), file.directory_names.end(), name) ==
                file.directory_names.end())
                file.directory_names.push_back(name);
    }
    std::vector<uint8_t> bytes(capacity);
    size_t used = 0;
    while (file.offset < file.directory_names.size()) {
        const auto &name = file.directory_names[file.offset];
        size_t size = (19 + name.size() + 1 + 7) & ~size_t{7};
        if (size > capacity - used) {
            if (!used)
                guest_error(22);
            break;
        }
        struct stat info{};
        try {
            info = filesystem_.resolve_at(name, directory, 0, false).stat();
        } catch (const GuestError &error) {
            if (error.number == 2) {
                ++file.offset;
                continue;
            } // A task/fd exited since enumeration began.
            throw;
        }
        uint64_t inode = info.st_ino, next = file.offset + 1;
        uint16_t length = size;
        std::memcpy(bytes.data() + used, &inode, 8);
        std::memcpy(bytes.data() + used + 8, &next, 8);
        std::memcpy(bytes.data() + used + 16, &length, 2);
        bytes[used + 18] = S_ISDIR(info.st_mode)   ? DT_DIR
                           : S_ISLNK(info.st_mode) ? DT_LNK
                           : S_ISCHR(info.st_mode) ? DT_CHR
                                                   : DT_REG;
        std::memcpy(bytes.data() + used + 19, name.c_str(), name.size() + 1);
        used += size;
        ++file.offset;
    }
    memory_.copy_in(address, bytes.data(), used);
    return used;
}
} // namespace operit::arm64
