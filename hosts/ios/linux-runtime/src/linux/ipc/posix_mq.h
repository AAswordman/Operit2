#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <vector>

namespace operit::arm64 {
class DescriptorTable;
class Memory;
class Scheduler;
struct OpenDescription;
namespace signals { struct Hooks; }

namespace ipc {
class PosixMqAccounting;
class PosixMqNamespace;
class PosixMqPollSubscription;

struct PosixMqCredentials {
    uint32_t uid = 0, fsuid = 0, fsgid = 0;
    std::vector<uint32_t> groups;
    bool cap_dac_override = false, cap_fowner = false, cap_sys_resource = false;
};

struct PosixMqLimits {
    uint32_t queues = 256, messages = 10, message_size = 8192;
    uint32_t default_messages = 10, default_message_size = 8192;
};

struct PosixMqContext {
    PosixMqCredentials credentials;
    uint32_t umask = 0022;
    // Current guest soft RLIMIT_MSGQUEUE; accounting is by real UID.
    uint64_t message_queue_limit = 819200;
    // Required when an operation would block; same contract as
    // Syscalls::interrupt_for_pending_signal(). Missing hook returns EOPNOTSUPP.
    std::function<bool()> interrupt_pending;
    // Required for SIGEV_SIGNAL, including on a sending syscall. These hooks
    // must resolve guest processes and wake their signal waiters.
    signals::Hooks *signal_hooks = nullptr;
};

// Create one accounting domain per guest user namespace and share it between
// its IPC namespaces. Never use a host-global IPC namespace or host mq_*.
std::shared_ptr<PosixMqAccounting> make_posix_mq_accounting();
std::shared_ptr<PosixMqNamespace> make_posix_mq_namespace(
    const std::shared_ptr<PosixMqAccounting> &accounting, const PosixMqLimits &limits = {});

// Dispatch AArch64 180..185, in register order. Errors throw GuestError or
// MemoryFault. A blocked operation returns 0 after Scheduler::block; its result
// is later installed in the task's X0. Calls and lifecycle hooks must run at
// serial Scheduler safe points, with all sharing tasks on the same Scheduler.
// The context and hooks need only outlive this call. Memory must outlive its
// blocked task; Scheduler retirement/exec must clear its poll callback first.
// Kernel names are single components WITHOUT libc's leading '/'. mq_attr is
// the AArch64 64-byte layout and sigevent is the AArch64 64-byte layout.
int64_t posix_mq_syscall(const std::shared_ptr<PosixMqNamespace> &space, Memory &memory,
                         DescriptorTable &descriptors, Scheduler &scheduler,
                         const PosixMqContext &context, uint64_t number, uint64_t arg0,
                         uint64_t arg1 = 0, uint64_t arg2 = 0, uint64_t arg3 = 0,
                         uint64_t arg4 = 0);

// Aliased OpenDescriptions preserve queue lifetime and flags through dup,
// fork and CLONE_FILES. mq_open always sets FD_CLOEXEC. Inherit the namespace
// on fork/exec; create a new one only when CLONE_NEWIPC/unshare commits.
// Invoke before EVERY descriptor close, replacement, close_range, close-on-exec
// and process descriptor teardown, with the closing process's TGID. This is
// per close, not merely the last OpenDescription reference.
void posix_mq_close(const std::shared_ptr<OpenDescription> &description, uint32_t pid);
// At committed exec call this even if FD_CLOEXEC was cleared: pending signal
// notification must not be delivered to the new program. At process exit call
// posix_mq_process_exit, including queues unlinked while still open. After an
// IPC namespace switch call these hooks for every namespace in which this
// process could still have an mq registration, not only its current one.
void posix_mq_process_exec(const std::shared_ptr<PosixMqNamespace> &space, uint32_t pid);
void posix_mq_process_exit(const std::shared_ptr<PosixMqNamespace> &space, uint32_t pid);

// Use this to intercept all other FD operations before they reach the native
// marker. In particular fstat/statx, fchmod/fchown and ftruncate must not
// report or mutate the marker pipe as though it were the queue. These metadata
// operations are not implemented here; return EOPNOTSUPP until integrated.
bool is_posix_mq_file(const std::shared_ptr<OpenDescription> &description);

// Generic read/write uses the installed virtual file (Linux mqueue status
// text, not message payload); descriptor flags remain shared with fcntl.
// Route poll/select/epoll through this function BEFORE their generic virtual
// file fallback. requested/result use Linux POLL bits (IN=1, OUT=4,
// RDNORM=64, WRNORM=256); nullopt denotes a non-mqueue description. A native
// pipe is only a readability marker and is NOT a complete epoll backend.
// This function is a snapshot; edge-triggered users must also subscribe below.
std::optional<uint32_t> posix_mq_poll(const std::shared_ptr<OpenDescription> &description,
                                      uint32_t requested);

// The subscription follows the underlying queue, including independent opens
// and inherited descriptions. Callbacks run at the same serial scheduler safe
// points as the queue operations. The first argument is the complete current
// POLL mask; the second indicates a Linux mqueue poll-waiter wakeup. A state
// update without a wakeup can occur when a blocked sender immediately refills
// a queue. Destroying the returned handle unsubscribes it.
std::shared_ptr<PosixMqPollSubscription> posix_mq_subscribe(
    const std::shared_ptr<OpenDescription> &description,
    std::function<void(uint32_t, bool)> callback);
} // namespace ipc
} // namespace operit::arm64
