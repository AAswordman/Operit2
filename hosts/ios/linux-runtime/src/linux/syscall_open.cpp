#include "base/random.h"
#include "linux/error_mapping.h"
#include "linux/events/inotify.h"
#include "linux/files/memory_file.h"
#include "linux/files/open_how.h"
#include "linux/syscalls.h"
#include <array>
#include <cerrno>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
FD open_guest_file(const Path &path, int flags, mode_t mode) {
    // Truncation is performed only after acquiring the inode's seal guard.
    flags &= ~O_TRUNC;
    if (!(flags & O_CREAT))
        return path.open(flags);
    if (flags & O_DIRECTORY)
        guest_error(22);
    for (;;) {
        if (!(flags & O_EXCL)) {
            FD existing;
            try {
                existing = path.open(flags & ~O_CREAT);
            } catch (const GuestError &error) {
                if (error.number != 2)
                    throw;
            }
            if (existing.get() >= 0) {
                struct stat info{};
                check_host(::fstat(existing.get(), &info));
                if (S_ISDIR(info.st_mode))
                    guest_error(21);
                return existing;
            }
        }
        FD created;
        try {
            created = path.open(flags | O_EXCL, mode);
        } catch (const GuestError &error) {
            if (error.number == 17 && !(flags & O_EXCL))
                continue;
            throw;
        }
        path.require_writable();
        check_host(::fchmod(created.get(), mode));
        return created;
    }
}
FD open_metadata(const Path &path, uint64_t guest_flags, const struct stat &expected) {
    if ((guest_flags & files::arm64_open::directory) && !S_ISDIR(expected.st_mode))
        guest_error(20);
#ifdef __APPLE__
    // O_SYMLINK pins a link itself; combining Darwin O_NOFOLLOW with it would
    // reject the link. O_EVTONLY avoids a data-open description in the guest.
    int flags = O_EVTONLY | O_SYMLINK | O_NONBLOCK | O_CLOEXEC;
#else
    int flags = O_PATH | O_NOFOLLOW | O_CLOEXEC;
#endif
    if (guest_flags & files::arm64_open::directory)
        flags |= O_DIRECTORY;
    FD result(::openat(path.parent.get(), path.name.c_str(), flags));
    check_host(result.get());
    struct stat actual{};
    check_host(::fstat(result.get(), &actual));
    if (actual.st_dev != expected.st_dev || actual.st_ino != expected.st_ino)
        guest_error(11);
    return result;
}
void truncate_open_file(int fd, uint64_t guest_flags) {
    if (!(guest_flags & files::arm64_open::truncate))
        return;
    struct stat info{};
    check_host(::fstat(fd, &info));
    if (S_ISDIR(info.st_mode))
        guest_error(21);
    // O_TRUNC has no resizing effect on FIFOs, terminals or other devices.
    if (S_ISREG(info.st_mode)) {
        auto seal_guard = files::memory_file_check_truncate(fd, 0);
        check_host(::ftruncate(fd, 0));
    }
}
FD anonymous_file(const Path &directory, int host_flags, mode_t mode) {
    directory.require_writable();
#ifdef __APPLE__
    // Darwin can emulate unlinked scratch files, but cannot publish their inode
    // later through linkat(/proc/self/fd/N, ..., AT_SYMLINK_FOLLOW). Reject the
    // linkable O_TMPFILE variant up front so callers such as apk use their named
    // temporary-file fallback. O_TMPFILE|O_EXCL explicitly forbids publication.
    if (!(host_flags & O_EXCL))
        guest_error(95); // EOPNOTSUPP in the guest ABI.
#endif
    host_flags &= ~(O_DIRECTORY | O_CREAT | O_TRUNC | O_NOFOLLOW);
#ifdef __APPLE__
    // Create in the requested directory, so quota, filesystem, mount and access
    // checks apply to that directory. Unlink before publishing any guest FD.
    constexpr char hex[] = "0123456789abcdef";
    for (unsigned attempt = 0; attempt < 128; ++attempt) {
        std::array<unsigned char, 16> random{};
        random_bytes(random.data(), random.size());
        std::string name = ".operit-tmp-";
        for (const auto byte : random) {
            name += hex[byte >> 4];
            name += hex[byte & 15];
        }
        FD result(::openat(directory.parent.get(), name.c_str(),
                           host_flags | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600));
        if (result.get() < 0) {
            if (errno == EEXIST)
                continue;
            guest_error(linux_errno(errno));
        }
        check_host(::unlinkat(directory.parent.get(), name.c_str(), 0));
        check_host(::fchmod(result.get(), mode));
        return result;
    }
    guest_error(17);
#else
    FD result(::openat(directory.parent.get(), ".", host_flags | O_TMPFILE | O_CLOEXEC, mode));
    check_host(result.get());
    check_host(::fchmod(result.get(), mode));
    return result;
#endif
}
void configure_open_file(int descriptor, uint64_t guest_flags) {
#ifdef __APPLE__
    if (guest_flags & files::arm64_open::direct)
        check_host(::fcntl(descriptor, F_NOCACHE, 1));
#else
    (void)descriptor;
    (void)guest_flags;
#endif
}
} // namespace
int Syscalls::open_flags(uint64_t guest) {
    using namespace files::arm64_open;
    if (guest & ~known)
        guest_error(22);
    if (guest & files::arm64_open::path) {
#ifdef __APPLE__
        return O_EVTONLY | O_SYMLINK | O_CLOEXEC;
#else
        return O_PATH | O_NOFOLLOW | O_CLOEXEC;
#endif
    }
    int flags = (guest & access_mode) == 2 ? O_RDWR : ((guest & access_mode) == 1 ? O_WRONLY : O_RDONLY);
    if (guest & create)
        flags |= O_CREAT;
    if (guest & exclusive)
        flags |= O_EXCL;
    if (guest & no_ctty)
        flags |= O_NOCTTY;
    if (guest & truncate)
        flags |= O_TRUNC;
    if (guest & append)
        flags |= O_APPEND;
    if (guest & nonblock)
        flags |= O_NONBLOCK;
    if (guest & data_sync)
        flags |= O_DSYNC;
    if (guest & full_sync)
        flags |= O_SYNC;
    if (guest & directory)
        flags |= O_DIRECTORY;
    if (guest & no_follow)
        flags |= O_NOFOLLOW;
    if (guest & async)
        flags |= O_ASYNC;
#ifndef __APPLE__
    if (guest & direct)
        flags |= O_DIRECT;
    if (guest & no_atime)
        flags |= O_NOATIME;
#endif
    return flags;
}
int Syscalls::open_path(const Path &path, uint64_t guest_flags, mode_t mode) {
    using namespace files::arm64_open;
    int host_flags = open_flags(guest_flags);
    if (path.virtual_path)
        return open_virtual(path, host_flags, int(guest_flags));
    FD host;
    VirtualNode::Ptr link;
    const bool metadata_only = guest_flags & files::arm64_open::path;
    if (metadata_only) {
        const auto info = path.stat();
        if (S_ISLNK(info.st_mode)) {
            // Link contents belong to the pinned inode and cannot change in
            // place. Keep them for readlinkat(fd, "") after rename/unlink.
            const auto contents = path.readlink();
            link = std::make_shared<VirtualNode>(VirtualNode::Kind::Symlink, path.guest, 0777);
            link->target = [contents] { return contents; };
        }
        host = open_metadata(path, guest_flags, info);
    } else if (guest_flags & temporary) {
        auto directory = filesystem_.resolve_at(".", path);
        host = anonymous_file(directory, host_flags, mode);
    } else {
        // Linux permits O_RDONLY|O_TRUNC. The guest description retains its
        // read-only access mode; the native FD also needs write access to resize.
        if ((guest_flags & truncate) && !(guest_flags & access_mode))
            host_flags = (host_flags & ~O_ACCMODE) | O_RDWR;
        host = open_guest_file(path, host_flags, mode);
        if (guest_flags & truncate)
            path.require_writable();
        truncate_open_file(host.get(), guest_flags);
    }
    if (!metadata_only)
        configure_open_file(host.get(), guest_flags);
    const int number = descriptors_.insert(std::move(host), guest_flags & temporary ? "" : path.guest,
                                           int(guest_flags));
    try {
        auto &entry = descriptors_.get(number);
        entry.description->mount = path.mount;
        if (link)
            entry.description->virtual_file = std::make_shared<VirtualOpen>(std::move(link));
        files::memory_file_attach(entry);
        if (!metadata_only)
            entry.description->inotify_open = InotifyOpenFile::opened(
                path, entry.host.get(), (guest_flags & access_mode) != 0, guest_flags & temporary);
    } catch (...) {
        descriptors_.close(number);
        throw;
    }
    return number;
}
int64_t Syscalls::dispatch_open(CpuState &cpu) {
    const auto &a = cpu.x;
    linux_abi::OpenHow how;
    if (a[8] == files::openat2_syscall) {
        how = files::read_open_how(memory_, a[2], a[3]);
    } else if (a[8] == 56) {
        // Legacy openat masks unknown flags and O_PATH's ignored modifiers.
        how.flags = uint32_t(a[2]) & files::arm64_open::known;
        if (how.flags & files::arm64_open::path)
            how.flags &= files::arm64_open::path | files::arm64_open::directory |
                         files::arm64_open::no_follow | files::arm64_open::close_on_exec;
        if (how.flags & (files::arm64_open::create | files::arm64_open::temporary))
            how.mode = a[3] & 07777;
        files::validate_open_how(how);
    } else
        return unsupported(cpu);
    const auto name = memory_.string(a[1]);
    if (name.empty())
        guest_error(2);
    Path directory;
    if (name.front() != '/' || (how.resolve & path_resolution::in_root))
        directory = directory_path(a[0]);
    auto request = files::resolve_openat2(filesystem_, name, directory, how);
    return open_path(request.path, request.flags, request.mode & ~fs_context_->mask);
}
} // namespace operit::arm64
