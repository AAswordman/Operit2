#pragma once

#include <cstdint>
#include <memory>
#include <vector>

namespace operit::arm64 {
class Memory;
namespace process {
class ResourceLimits;
}

namespace ipc {
class SharedMemoryNamespace;
class SharedMemoryAddressSpace;

// Values are guest credentials and guest capabilities, not host privileges.
// Supplementary groups and fsgid must be the calling task's actual guest
// credentials; ipcperms uses euid, fsgid and supplementary groups.
struct SharedMemoryCredentials {
    uint32_t euid = 0, egid = 0, fsgid = 0;
    std::vector<uint32_t> groups;
    bool cap_ipc_owner = false;
    bool cap_sys_admin = false;
};

struct SharedMemoryLimits {
    uint64_t max_segment_size = uint64_t{1} << 40;
    uint64_t max_total_pages = uint64_t{1} << 28;
    uint32_t max_segments = 4096;
};

// Make one namespace per initial guest IPC namespace, never a host-global
// singleton. Fork/exec normally retain it; CLONE_NEWIPC/unshare install a
// newly created one only after the caller's namespace transaction commits.
std::shared_ptr<SharedMemoryNamespace> make_shared_memory_namespace(
    const SharedMemoryLimits &limits = {});

// Exactly one address-space table accompanies each distinct guest Memory.
// CLONE_VM (including CLONE_THREAD) shares the same table. Other forks call
// fork_shared_memory_address_space AFTER Memory::fork_clone and after the
// child PID is known, before publishing the child. The clone is transactional:
// dropping an unpublished result reverses its attachment references.
std::shared_ptr<SharedMemoryAddressSpace> make_shared_memory_address_space();
std::shared_ptr<SharedMemoryAddressSpace> fork_shared_memory_address_space(
    const std::shared_ptr<SharedMemoryAddressSpace> &parent, Memory &child_memory,
    uint32_t child_pid);

// Call at committed exec and whole-process exit, even for a zombie retained
// for wait; do not call for an individual CLONE_THREAD exit. Exec's new image
// gets an empty table and retains the IPC namespace. Release only this image's
// ownership; other CLONE_VM users keep their mappings and attachment counts.
// This also works when an exec/clone transaction is rolled back: dropping its
// uncommitted table reverses the references it acquired.
void release_shared_memory_address_space(std::shared_ptr<SharedMemoryAddressSpace> &space,
                                         uint32_t pid) noexcept;

// Memory can split, remove or replace VMAs independently of shmdt. While
// holding Memory::AccessGuard across the entire operation, call this after
// every successful munmap, MAP_FIXED mmap, mprotect and madvise on this Memory,
// and before publishing a fork. It tracks fragments and final detach, even
// after IPC namespace changes. Call before another guest syscall can observe
// the state. An untouched address space may be notified too.
void shared_memory_mappings_changed(const std::shared_ptr<SharedMemoryAddressSpace> &space,
                                    Memory &memory, uint32_t pid);

// Before mremap, under the same Memory::AccessGuard, reject operations touching
// a SysV attachment (including a fixed destination). The current Memory API
// cannot safely preserve shmdt identity across moved/duplicated mappings.
// For an allowed mremap, call shared_memory_mappings_changed after success.
void shared_memory_check_remap(const std::shared_ptr<SharedMemoryAddressSpace> &space,
                               Memory &memory, uint64_t old_address, uint64_t old_length,
                               uint64_t new_address, uint64_t new_length, uint64_t flags);

// AArch64 syscalls 194..197. Arguments are raw guest registers; result is a
// nonnegative Linux return value, errors throw GuestError. The caller supplies
// its current namespace, address space, resource limits, credentials, TGID,
// and mmap allocation cursor explicitly. Hold no namespace lock on entry.
// Namespace and address-space references must remain alive for the call.
int64_t shared_memory_syscall(const std::shared_ptr<SharedMemoryNamespace> &ipc_namespace,
                              const std::shared_ptr<SharedMemoryAddressSpace> &space,
                              Memory &memory, const process::ResourceLimits &limits,
                              const SharedMemoryCredentials &credentials, uint32_t pid,
                              uint64_t &mmap_next, uint64_t number, uint64_t arg0,
                              uint64_t arg1, uint64_t arg2);
} // namespace ipc
} // namespace operit::arm64
