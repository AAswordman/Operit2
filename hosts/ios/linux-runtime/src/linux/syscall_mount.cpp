#include "fs/virtual/procfs.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"

namespace operit::arm64 {
namespace {
bool inside_mount(const std::string &path, const std::string &mount) {
    return path == mount || (mount == "/" && !path.empty() && path.front() == '/') ||
           (path.size() > mount.size() && path.compare(0, mount.size(), mount) == 0 &&
            path[mount.size()] == '/');
}
} // namespace
int64_t Syscalls::dispatch_mount(CpuState &cpu) {
    // The guest capability set is empty, independently of the host UID.
    // A future capability model must check CAP_SYS_ADMIN in the user
    // namespace that owns this mount namespace before permitting changes.
    constexpr bool guest_cap_sys_admin = false;
    if (!guest_cap_sys_admin)
        guest_error(1);
    const auto &a = cpu.x;
    if (a[8] == 39) { // umount2
        if (a[1] > UINT32_MAX)
            guest_error(22);
        const auto target = memory_.string(a[0]);
        const auto flags = uint32_t(a[1]);
        if (flags & ~(mounts::detach | mounts::no_follow))
            guest_error(95);
        if (!(flags & mounts::detach)) {
            std::string mounted;
            {
                const auto path = filesystem_.resolve(target, fs_context_->cwd, !(flags & mounts::no_follow));
                if (!path.mount_root || !path.mount || !path.mount_id)
                    guest_error(22);
                mounted = path.guest;
            }
            auto check_busy = [&](Syscalls &context) {
                if (!filesystem_.shares_mounts(context.filesystem_))
                    return;
                if (inside_mount(context.fs_context_->cwd, mounted))
                    guest_error(16);
            };
            if (auto *processes = scheduler_.processes()) {
                for (uint32_t pid : processes->process_ids())
                    if (auto *context = processes->process_syscalls(pid))
                        check_busy(*context);
            } else
                check_busy(*this);
        }
        filesystem_.unmount(target, flags, fs_context_->cwd);
        return 0;
    }
    if (a[8] != 40)
        guest_error(22);
    const uint64_t flags = a[3];
    const std::string target = memory_.string(a[1]);
    const std::string source = a[0] ? memory_.string(a[0]) : std::string{};
    const bool uses_type = !(flags & mounts::bind);
    const std::string type = uses_type && a[2] ? memory_.string(a[2]) : std::string{};
    const std::string data = uses_type && a[4] ? memory_.string(a[4]) : std::string{};
    VirtualNode::Ptr root;
    if (!(flags & (mounts::bind | mounts::remount))) {
        if (type == "proc")
            root = process_filesystem(process_source());
        else if (type == "devfs" || type == "devtmpfs")
            root = device_filesystem(terminals_);
        else if (type == "devpts")
            root = terminals_->slave_filesystem();
    }
    filesystem_.mount(source, target, type, flags, data, fs_context_->cwd, std::move(root));
    return 0;
}
} // namespace operit::arm64
