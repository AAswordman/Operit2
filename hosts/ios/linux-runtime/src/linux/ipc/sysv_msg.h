#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <vector>

namespace operit::arm64 {
class Memory;
class Scheduler;

namespace ipc {
class MessageQueueNamespace;

// All identities and capabilities are guest values in the user namespace
// that owns the IPC namespace, never inferred from host credentials.
struct MessageQueueCredentials {
    uint32_t euid = 0, egid = 0, fsgid = 0;
    std::vector<uint32_t> groups;
    bool cap_ipc_owner = false, cap_sys_admin = false, cap_sys_resource = false;
};

struct MessageQueueLimits {
    uint32_t max_queues = 32000;
    uint32_t max_message_size = 8192;
    uint32_t default_queue_bytes = 16384;
};

// Create one object per guest IPC namespace, and share it across fork, exec,
// CLONE_VM and threads. CLONE_NEWIPC/unshare install a new object only after
// the namespace change commits. Queues survive the creating process's exit.
// Each object belongs to one scheduler: invoke all its operations at that
// scheduler's serial syscall/idle-poll boundary, never from parallel workers.
std::shared_ptr<MessageQueueNamespace> make_message_queue_namespace(
    Scheduler &scheduler, const MessageQueueLimits &limits = {});

// AArch64 syscalls 186..189; arguments are the raw guest registers. Errors
// throw GuestError/MemoryFault, as in Syscalls::dispatch. Blocking returns 0
// with the task in Waiting; Scheduler later supplies the actual result.
// The caller must pass a pending-signal callback equivalent to
// Syscalls::interrupt_for_pending_signal(): return true and mark the current
// thread's signal state interrupted when a deliverable signal is pending.
// Task retirement/interrupt must run the scheduler's wait_cleanup before the
// task's Memory is destroyed. A blocked operation retains its IPC namespace.
// System V message syscalls have no timeout argument. Linux MSG_COPY is
// supported; other unrecognized msgctl commands return EINVAL.
int64_t message_queue_syscall(const std::shared_ptr<MessageQueueNamespace> &ipc_namespace,
                              Memory &memory, const MessageQueueCredentials &credentials,
                              uint64_t number, uint64_t arg0, uint64_t arg1, uint64_t arg2,
                              uint64_t arg3, uint64_t arg4,
                              const std::function<bool()> &has_pending_signal);
} // namespace ipc
} // namespace operit::arm64
