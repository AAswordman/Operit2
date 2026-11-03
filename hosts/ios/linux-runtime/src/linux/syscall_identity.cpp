#include "base/fd.h"
#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include <cerrno>
#include <cstdio>
#include <optional>
#include <string>
#include <unistd.h>
#if defined(__linux__)
#include <fcntl.h>
#include <sys/syscall.h>
#elif defined(__APPLE__)
#include <sys/sysctl.h>
#endif

namespace operit::arm64 {
namespace {
constexpr uint32_t unchanged_id = UINT32_MAX;
struct Identity {
    uint32_t real, effective;
    std::optional<uint32_t> saved;
};

// Linux reference: v6.12/kernel/sys.c (__sys_setreuid/__sys_setregid,
// __sys_setresuid/__sys_setresgid, getres*, __sys_setfsuid/__sys_setfsgid).
// https://github.com/torvalds/linux/blob/v6.12/kernel/sys.c
// No host setter is used, including the traditional setfsuid(-1) query.
Identity host_identity(bool group) {
    Identity value{group ? uint32_t(::getgid()) : uint32_t(::getuid()),
                   group ? uint32_t(::getegid()) : uint32_t(::geteuid()), std::nullopt};
#if defined(__linux__)
    // Raw getters avoid depending on libc's _GNU_SOURCE declaration of getres*.
    if (group) {
        gid_t real, effective, saved;
        if (::syscall(SYS_getresgid, &real, &effective, &saved) == 0)
            value = {uint32_t(real), uint32_t(effective), uint32_t(saved)};
    } else {
        uid_t real, effective, saved;
        if (::syscall(SYS_getresuid, &real, &effective, &saved) == 0)
            value = {uint32_t(real), uint32_t(effective), uint32_t(saved)};
    }
#elif defined(__APPLE__)
    // XNU kern_sysctl.c: fill_user{32,64}_eproc obtains p_svuid/p_svgid
    // from kauth_cred_getsvuid/getsvgid; these fields are NOT aliases of euid.
    // https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sysctl.c
    // KERN_PROC_PID describes process credentials, so reject a per-thread
    // override before trusting its saved IDs (kern_prot.c: gettid).
    uid_t thread_uid;
    gid_t thread_gid;
    errno = 0;
    const int overridden = ::pthread_getugid_np(&thread_uid, &thread_gid);
    if (!((overridden == -1 && errno == ESRCH) || overridden == ESRCH))
        return value;
    int mib[] = {CTL_KERN, KERN_PROC, KERN_PROC_PID, ::getpid()};
    struct kinfo_proc info{};
    size_t size = sizeof(info);
    if (::sysctl(mib, 4, &info, &size, nullptr, 0) != 0 || size != sizeof(info))
        return value;
    const auto &process = info.kp_eproc.e_pcred;
    const auto &credentials = info.kp_eproc.e_ucred;
    if (credentials.cr_ngroups <= 0 ||
        uint32_t(process.p_ruid) != uint32_t(::getuid()) ||
        uint32_t(credentials.cr_uid) != uint32_t(::geteuid()) ||
        uint32_t(process.p_rgid) != uint32_t(::getgid()) ||
        uint32_t(credentials.cr_groups[0]) != uint32_t(::getegid()))
        return value;
    value = group ? Identity{uint32_t(process.p_rgid), uint32_t(credentials.cr_groups[0]),
                             uint32_t(process.p_svgid)}
                  : Identity{uint32_t(process.p_ruid), uint32_t(credentials.cr_uid),
                             uint32_t(process.p_svuid)};
#endif
    return value;
}

Identity guest_identity(ProcessController *controller, uint32_t pid, bool group) {
    if (!controller)
        return host_identity(group);
    // A controller with a missing registration must report ESRCH; silently
    // falling back to the host here would disagree with other guest checks.
    const auto value = process::pidfd_credentials(*controller, pid);
    return group ? Identity{value.real_gid, value.effective_gid, value.saved_gid}
                 : Identity{value.real_uid, value.effective_uid, value.saved_uid};
}

std::optional<uint32_t> host_filesystem_identity(bool group) {
#if defined(__linux__)
    // proc.rst documents Uid/Gid as real/effective/saved/filesystem. Read the
    // calling HOST THREAD, not /proc/self/status (the host process leader).
    // https://github.com/torvalds/linux/blob/v6.12/Documentation/filesystems/proc.rst
    // https://github.com/torvalds/linux/blob/v6.12/fs/proc/array.c
    const auto actual = host_identity(group);
    FD file(::open("/proc/thread-self/status", O_RDONLY | O_CLOEXEC));
    if (file.get() < 0)
        return std::nullopt;
    std::string status;
    const char *label = group ? "\nGid:" : "\nUid:";
    char buffer[1024];
    while (status.size() < 65536) {
        const auto count = ::read(file.get(), buffer, sizeof(buffer));
        if (count < 0 && errno == EINTR)
            continue;
        if (count <= 0)
            break;
        status.append(buffer, size_t(count));
        const auto start = status.find(label);
        if (start == std::string::npos || status.find('\n', start + 1) == std::string::npos)
            continue;
        uint32_t real, effective, saved, filesystem;
        if (std::sscanf(status.c_str() + start + 5,
                        " %u %u %u %u",
                        &real, &effective, &saved, &filesystem) != 4)
            return std::nullopt;
        // A procfs mounted in another user namespace, or racing host changes,
        // must not cause a guessed filesystem ID to escape to the guest.
        if (real != actual.real || effective != actual.effective ||
            !actual.saved || saved != *actual.saved)
            return std::nullopt;
        return filesystem;
    }
    return std::nullopt;
#elif defined(__APPLE__)
    // Darwin has no independent Linux fsuid/fsgid. Native file permission
    // checks use the calling thread's effective credentials, including an
    // override; the saved process IDs are irrelevant to this query.
    return group ? uint32_t(::getegid()) : uint32_t(::geteuid());
#else
    (void)group;
    return std::nullopt;
#endif
}
uint32_t known_id(const std::optional<uint32_t> &value) {
    if (!value)
        guest_error(38); // ENOSYS: unavailable host information, never euid-as-saved.
    return *value;
}
void retain_id(uint32_t requested, uint32_t current) {
    if (requested != unchanged_id && requested != current)
        guest_error(1); // EPERM, even when the HOST has CAP_SETUID/CAP_SETGID.
}
void retain_filesystem_identity(bool group, uint32_t effective) {
    if (known_id(host_filesystem_identity(group)) != effective)
        guest_error(1);
}
void retain_re_identity(const Identity &current, bool group, uint32_t real, uint32_t effective) {
    retain_id(real, current.real);
    retain_id(effective, current.effective);
    // setre* rewrites saved even when an explicitly supplied real ID is the
    // existing one; effective != old real also triggers this hidden change.
    if (real != unchanged_id || (effective != unchanged_id && effective != current.real)) {
        if (known_id(current.saved) != current.effective)
            guest_error(1);
    }
    // Unlike setres*, setre*(-1, -1) still resets the filesystem ID.
    retain_filesystem_identity(group, current.effective);
}
void retain_res_identity(const Identity &current, bool group, uint32_t real, uint32_t effective,
                         uint32_t saved) {
    retain_id(real, current.real);
    retain_id(effective, current.effective);
    if (saved != unchanged_id)
        retain_id(saved, known_id(current.saved));
    // Linux's no-op fast path preserves fsuid/fsgid when effective == -1.
    // An explicit effective ID must ALSO retain the filesystem identity.
    if (effective != unchanged_id)
        retain_filesystem_identity(group, current.effective);
}
void retain_single_identity(const Identity &current, bool group, uint32_t requested) {
    // No -1 sentinel for setuid/setgid. With zero guest capabilities they
    // change only effective/fs IDs, and the target must equal real or saved.
    if (requested == unchanged_id)
        guest_error(22);
    if (requested != current.effective)
        guest_error(1);
    if (requested != current.real && requested != known_id(current.saved))
        guest_error(1);
    retain_filesystem_identity(group, current.effective);
}
} // namespace

int64_t Syscalls::dispatch_identity(CpuState &cpu) {
    const auto &a = cpu.x;
    const auto number = a[8];
    switch (number) {
    case 151: // setfsuid
    case 152: // setfsgid
        // Linux uid_t/gid_t arguments are the LOW 32 bits, including -1.
        // All actual changes are refused silently; every argument therefore
        // returns the old unsigned 32-bit ID, widened without sign extension.
        // If the old ID cannot be read, report ENOSYS rather than invent it.
        return known_id(host_filesystem_identity(number == 152));
    case 143: case 144: case 145: case 146: case 147: case 148:
    case 149: case 150: case 174: case 175: case 176: case 177:
        break;
    default:
        return unsupported(cpu);
    }
    const bool group = number == 143 || number == 144 || number == 149 || number == 150 ||
                       number == 176 || number == 177;
    const auto current = guest_identity(scheduler_.processes(), process_anchor_->pid, group);
    switch (number) {
    case 174: case 176:
        return current.real;
    case 175: case 177:
        return current.effective;
    case 148: case 150: { // getresuid/getresgid: ordered put_user, not all-or-nothing.
        const uint32_t values[] = {current.real, current.effective, known_id(current.saved)};
        for (unsigned index = 0; index < 3; ++index) {
            check_buffer(a[index], 4, write_perm);
            memory_.store(a[index], values[index], 4);
        }
        return 0;
    }
    case 143: case 145:
        retain_re_identity(current, group, uint32_t(a[0]), uint32_t(a[1]));
        return 0;
    case 147: case 149:
        retain_res_identity(current, group, uint32_t(a[0]), uint32_t(a[1]), uint32_t(a[2]));
        return 0;
    case 144: case 146:
        retain_single_identity(current, group, uint32_t(a[0]));
        return 0;
    }
    return unsupported(cpu);
}
} // namespace operit::arm64
