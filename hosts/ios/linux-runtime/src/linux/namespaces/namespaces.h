#pragma once
#include <cstdint>
#include <functional>
#include <memory>
#include <utility>

namespace operit::arm64 {
class DescriptorTable;
class Filesystem;
struct OpenDescription;

namespace namespaces {
struct UserNamespaceIdentity;
class MountNamespace;
// Linux UAPI values, independent of the host's sched.h.
constexpr uint64_t clone_newtime = 0x00000080;
constexpr uint64_t clone_fs = 0x00000200;
constexpr uint64_t clone_files = 0x00000400;
constexpr uint64_t clone_newns = 0x00020000;
constexpr uint64_t clone_newcgroup = 0x02000000;
constexpr uint64_t clone_newuts = 0x04000000;
constexpr uint64_t clone_newipc = 0x08000000;
constexpr uint64_t clone_newuser = 0x10000000;
constexpr uint64_t clone_newpid = 0x20000000;
constexpr uint64_t clone_newnet = 0x40000000;
constexpr uint64_t namespace_flags = clone_newtime | clone_newns | clone_newcgroup |
                                     clone_newuts | clone_newipc | clone_newuser |
                                     clone_newpid | clone_newnet;

// Preparation may fail; commit must only publish already prepared state and
// must not throw. Syscalls/lifecycle must serialize the complete transaction.
class PreparedChange {
  public:
    virtual ~PreparedChange() = default;
    virtual void commit() noexcept = 0;
};

template <class T> class SharedChange final : public PreparedChange {
    std::shared_ptr<T> &slot_;
    std::shared_ptr<T> next_;

  public:
    SharedChange(std::shared_ptr<T> &slot, std::shared_ptr<T> next)
        : slot_(slot), next_(std::move(next)) {}
    void commit() noexcept override { slot_.swap(next_); }
};

template <class T>
std::unique_ptr<PreparedChange> replace_shared(std::shared_ptr<T> &slot, std::shared_ptr<T> next) {
    return std::make_unique<SharedChange<T>>(slot, std::move(next));
}

struct UnshareContext {
    Filesystem &filesystem;
    DescriptorTable &descriptors;
    // True only if these are calling-task resources, or the thread group has
    // exactly one live task. Merely having a Process-owned Syscalls is not enough.
    bool task_scoped = false;
    bool cap_sys_admin = false; // Guest capability, never the host UID.
    // Copy the caller's fs_struct. If next_mounts is non-null, also rebase
    // pinned cwd/root mount references to that staged namespace. Return null
    // when the rebase cannot be implemented without losing the pinned paths.
    std::function<std::unique_ptr<PreparedChange>(const Filesystem *next_mounts)> prepare_fs;
    // The caller owns this identity for the lifetime of the namespace, even
    // when no namespace FDs are open. NEWNS atomically replaces this slot.
    std::shared_ptr<MountNamespace> *mount_identity = nullptr;
};

// Unshare FS/FILES and, with a verified rebase hook and CAP_SYS_ADMIN, NEWNS.
// Unknown/unsupported flags or unavailable isolation produce EINVAL; denied
// namespace creation produces EPERM. 0 is a successful no-op.
int64_t unshare(uint64_t flags, UnshareContext &context);

// Call before cloning; other clone flags remain the caller's responsibility.
// For NEWNS, clone must create the child's inherited filesystem and private
// fs_struct, then call unshare(clone_newns, child_context) before publishing
// the child. Without a safe cwd/root rebase hook, reject NEWNS.
void check_clone(uint64_t flags, bool cap_sys_admin);

// One identity object per actual guest user namespace. Do not substitute a
// host UID, PID, pathname or freshly fabricated object at setns time.
struct UserNamespaceIdentity final {};

// Initialize once for a new guest's initial mount namespace. Fork/clone without
// NEWNS and exec retain the same shared_ptr. It pins a stable identity even
// across closing and reopening every namespace FD.
std::shared_ptr<MountNamespace> acquire_mount_namespace(
    const Filesystem &filesystem, std::shared_ptr<const UserNamespaceIdentity> owner);

// Creates an actual guest namespace open description using the target task's
// existing, non-null identity; filesystem must still match that identity.
// Only call from a verified procfs namespace-open implementation after access
// checks. Dup/fork preserve the identity. SCM_RIGHTS and /proc/self/fd reopening
// require separate hooks to preserve it; reject those operations until those
// hooks exist. No host mounts are made.
int open_mount_namespace(DescriptorTable &descriptors, const Filesystem &filesystem,
                         std::shared_ptr<MountNamespace> identity, uint32_t flags = 0);
std::shared_ptr<MountNamespace> mount_namespace_file(
    const std::shared_ptr<OpenDescription> &description) noexcept;
bool same_mount_namespace(const std::shared_ptr<MountNamespace> &file,
                          const Filesystem &filesystem) noexcept;

struct SetnsContext {
    DescriptorTable &descriptors;
    bool task_scoped = false;
    bool fs_private = false; // fs_struct has exactly one user, not just one process.
    bool cap_sys_admin = false;
    bool cap_sys_chroot = false;
    std::shared_ptr<const UserNamespaceIdentity> user_namespace;
    // Prepare an install of the TARGET mount identity, preserving local
    // virtual contexts and setting cwd/root to the target namespace root.
    // Copying/replacing the entire target Filesystem is not generally safe.
    std::function<std::unique_ptr<PreparedChange>(const Filesystem &target)> prepare_mount_install;
    // Must describe the caller's current filesystem, and be held per task.
    Filesystem *filesystem = nullptr;
    std::shared_ptr<MountNamespace> *mount_identity = nullptr;
};

// Only genuine namespace FDs made above are accepted. pidfd-based setns and
// user-namespace ancestry are unsupported. FDs/type/permissions are checked
// before any change; missing safe installation hooks fail closed.
int64_t setns(uint64_t fd, uint64_t type, SetnsContext &context);
} // namespace namespaces
} // namespace operit::arm64
