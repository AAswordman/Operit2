#include "linux/error_mapping.h"
#include "linux/events/inotify.h"
#include "linux/files/memory_file.h"
#include "linux/syscalls.h"
#include <cerrno>
#include <fcntl.h>
#ifndef __APPLE__
#include <sys/syscall.h>
#endif
#include <sys/time.h>
#ifdef __APPLE__
#include <sys/stdio.h>
#endif

namespace operit::arm64 {
namespace {
constexpr uint32_t rename_noreplace = 1, rename_exchange = 2, rename_whiteout = 4;
constexpr uint32_t at_symlink_nofollow = 256, at_empty_path = 4096;

void chmod_descriptor(int descriptor, mode_t mode) {
    struct stat info{};
    check_host(::fstat(descriptor, &info));
    // Darwin permits changing link modes; Linux does not. In particular an
    // O_PATH|O_NOFOLLOW FD with AT_EMPTY_PATH still addresses the link itself.
    if (S_ISLNK(info.st_mode))
        guest_error(95);
#if defined(__linux__) && defined(SYS_fchmodat2)
    // Unlike fchmod, fchmodat2(AT_EMPTY_PATH) accepts Linux O_PATH descriptors.
    check_host(::syscall(SYS_fchmodat2, descriptor, "", mode, at_empty_path));
#else
#if defined(__linux__)
    const int flags = ::fcntl(descriptor, F_GETFL);
    check_host(flags);
    if (flags & O_PATH)
        guest_error(95); // No unsafe pathname reopen on older host headers.
#endif
    check_host(::fchmod(descriptor, mode));
#endif
}

void chmod_path(const Path &path, mode_t mode) {
    path.require_writable();
#if defined(__linux__) && defined(SYS_fchmodat2)
    // The guest resolver already followed the final link when requested.
    // Always forbid the host from following another final link.
    check_host(::syscall(SYS_fchmodat2, path.parent.get(), path.name.c_str(), mode,
                         at_symlink_nofollow));
#else
    // O_EVTONLY still requires file read permission on Darwin; chmod must work
    // for an owned mode-0000 file. The pinned parent plus native NOFOLLOW keeps
    // replacement links inside this directory from redirecting into the host.
    if (S_ISLNK(path.stat().st_mode))
        guest_error(95);
    check_host(::fchmodat(path.parent.get(), path.name.c_str(), mode, AT_SYMLINK_NOFOLLOW));
#endif
}
} // namespace
int64_t Syscalls::dispatch_path(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (a[8]) {
    case 36: { // symlinkat: link contents stay in the guest namespace.
        auto contents = memory_.string(a[0]);
        auto path = at_path(a[1], a[2], false);
        path.require_writable();
        check_host(::symlinkat(contents.c_str(), path.parent.get(), path.name.c_str()));
        return 0;
    }
    case 37: { // linkat
        if (a[4] & ~uint64_t{1024})
            guest_error(22);
        auto source = at_path(a[0], a[1], a[4] & 1024);
        auto target = at_path(a[2], a[3], false);
        target.require_writable();
        if (source.mount_id != target.mount_id)
            guest_error(18);
        check_host(
            ::linkat(source.parent.get(), source.name.c_str(), target.parent.get(), target.name.c_str(), 0));
        return 0;
    }
    case 45: { // truncate
        auto path = filesystem_.resolve(memory_.string(a[0]), fs_context_->cwd);
        path.require_writable();
        FD fd(::openat(path.parent.get(), path.name.c_str(), O_WRONLY | O_NOFOLLOW | O_CLOEXEC));
        check_host(fd.get());
        auto seal_guard = files::memory_file_check_truncate(fd.get(), a[1]);
        check_host(::ftruncate(fd.get(), int64_t(a[1])));
        return 0;
    }
    case 48:    // faccessat
    case 439: { // faccessat2
        uint64_t flags = a[8] == 439 ? a[3] : 0;
        if ((a[2] & ~uint64_t{7}) || (flags & ~uint64_t{256 | 512}))
            guest_error(22);
        auto path = at_path(a[0], a[1], !(flags & 256));
        if (path.virtual_path) {
            auto info = path.stat();
            unsigned required = ((a[2] & 4) ? 0444 : 0) | ((a[2] & 2) ? 0222 : 0) | ((a[2] & 1) ? 0111 : 0);
            if ((info.st_mode & required) != required)
                guest_error(13);
            return 0;
        }
        if (a[2] & 2)
            path.require_writable();
        check_host(::faccessat(path.parent.get(), path.name.c_str(), int(a[2]),
                               AT_SYMLINK_NOFOLLOW | ((flags & 512) ? AT_EACCESS : 0)));
        return 0;
    }
    case 52: { // fchmod
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        entry.description->require_writable();
        check_host(::fchmod(entry.host.get(), a[1] & 07777));
        return 0;
    }
    case 53: { // fchmodat
        auto path = at_path(a[0], a[1]);
        path.require_writable();
        check_host(::fchmodat(path.parent.get(), path.name.c_str(), a[2] & 07777, AT_SYMLINK_NOFOLLOW));
        return 0;
    }
    case 452: { // fchmodat2 (fs/open.c: do_fchmodat / chmod_common)
        const uint32_t flags = uint32_t(a[3]);
        if (flags & ~(at_symlink_nofollow | at_empty_path))
            guest_error(22);
        const mode_t mode = mode_t(a[2] & 07777);
        const auto name = memory_.string(a[1]);
        if (name.empty() && (flags & at_empty_path)) {
            if (int32_t(a[0]) == -100) {
                // A pinned cwd also survives rename; never reopen its display path.
                auto directory = directory_path(a[0]);
                chmod_path(directory, mode);
            } else {
                auto &entry = descriptors_.get(uint32_t(a[0]));
                if (ipc::is_posix_mq_file(entry.description))
                    guest_error(95);
                entry.description->require_writable();
                if (entry.description->virtual_file) {
                    if (entry.description->virtual_file->node->kind == VirtualNode::Kind::Symlink)
                        guest_error(95);
                    guest_error(30);
                }
                chmod_descriptor(entry.host.get(), mode);
            }
        } else {
            auto path = resolve_path(a[0], name, !(flags & at_symlink_nofollow));
            chmod_path(path, mode);
        }
        return 0;
    }
    case 54: { // fchownat
        if (a[4] & ~uint64_t{256 | 4096})
            guest_error(22);
        auto name = memory_.string(a[1]);
        if (name.empty() && (a[4] & 4096)) {
            auto &entry = descriptors_.get(uint32_t(a[0]));
            if (ipc::is_posix_mq_file(entry.description))
                guest_error(95);
            entry.description->require_writable();
            check_host(::fchown(entry.host.get(), uid_t(a[2]), gid_t(a[3])));
        } else {
            auto path = filesystem_.resolve(
                name, !name.empty() && name.front() == '/' ? "/" : directory_base(a[0]), !(a[4] & 256));
            path.require_writable();
            check_host(::fchownat(path.parent.get(), path.name.c_str(), uid_t(a[2]), gid_t(a[3]),
                                  AT_SYMLINK_NOFOLLOW));
        }
        return 0;
    }
    case 55: { // fchown
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        entry.description->require_writable();
        check_host(::fchown(entry.host.get(), uid_t(a[1]), gid_t(a[2])));
        return 0;
    }
    case 276: { // renameat2 (fs/namei.c: filename_renameat2)
        const uint32_t flags = uint32_t(a[4]);
        if ((flags & ~(rename_noreplace | rename_exchange | rename_whiteout)) ||
            ((flags & rename_exchange) && (flags & (rename_noreplace | rename_whiteout))))
            guest_error(22);
        if (flags & rename_whiteout)
            guest_error(95); // No whiteout/device emulation or non-atomic fallback.
        auto source = at_path(a[0], a[1], false);
        auto target = at_path(a[2], a[3], false);
        source.require_removable();
        target.require_removable();
        if (source.mount_id != target.mount_id)
            guest_error(18);
        if (!flags) {
            check_host(::renameat(source.parent.get(), source.name.c_str(),
                                   target.parent.get(), target.name.c_str()));
        } else {
#ifdef __APPLE__
            if (flags & rename_noreplace) {
                // Darwin allows RENAME_EXCL on a case-insensitive same-vnode
                // rename (even identical names). Linux requires EEXIST. This
                // check only REJECTS that exception: every actual mutation
                // still uses atomic RENAME_EXCL, including a racing target.
                const auto from = source.stat();
                struct stat to{};
                if (::fstatat(target.parent.get(), target.name.c_str(), &to, AT_SYMLINK_NOFOLLOW) == 0 &&
                    from.st_dev == to.st_dev && from.st_ino == to.st_ino)
                    guest_error(17);
            }
            const unsigned host_flags = (flags & rename_noreplace) ? RENAME_EXCL : RENAME_SWAP;
            check_host(::renameatx_np(source.parent.get(), source.name.c_str(),
                                       target.parent.get(), target.name.c_str(), host_flags));
#elif defined(__linux__) && defined(SYS_renameat2)
            check_host(::syscall(SYS_renameat2, source.parent.get(), source.name.c_str(),
                                 target.parent.get(), target.name.c_str(), flags));
#else
            guest_error(95);
#endif
        }
        if (flags & rename_exchange)
            InotifyOpenFile::exchanged(source, target);
        else
            InotifyOpenFile::renamed(source, target);
        return 0;
    }
    case 82:   // fsync
    case 83: { // fdatasync (fsync is a stronger durability guarantee)
        auto &entry = descriptors_.get(uint32_t(a[0]));
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        if (entry.description->virtual_file)
            guest_error(22);
        check_host(::fsync(entry.host.get()));
        return 0;
    }
    case 88: { // utimensat
        if (a[3] & ~uint64_t{256})
            guest_error(22);
        struct timespec times[2]{};
        if (a[2]) {
            check_buffer(a[2], 32, read_perm);
            for (unsigned i = 0; i < 2; ++i) {
                times[i].tv_sec = int64_t(memory_.load(a[2] + i * 16, 8));
                int64_t ns = int64_t(memory_.load(a[2] + i * 16 + 8, 8));
                if (ns == 1073741823)
                    times[i].tv_nsec = UTIME_NOW;
                else if (ns == 1073741822)
                    times[i].tv_nsec = UTIME_OMIT;
                else if (ns < 0 || ns >= 1000000000)
                    guest_error(22);
                else
                    times[i].tv_nsec = ns;
            }
        }
        if (!a[1]) {
            if (a[3])
                guest_error(22);
            auto &entry = descriptors_.get(uint32_t(a[0]));
            entry.description->require_writable();
            check_host(::futimens(entry.host.get(), a[2] ? times : nullptr));
        } else {
            auto path = at_path(a[0], a[1], !(a[3] & 256));
            path.require_writable();
            check_host(::utimensat(path.parent.get(), path.name.c_str(), a[2] ? times : nullptr,
                                   AT_SYMLINK_NOFOLLOW));
        }
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
