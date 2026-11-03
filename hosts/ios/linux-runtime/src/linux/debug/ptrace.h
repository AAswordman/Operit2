#pragma once

#include "cpu/aarch64/state.h"
#include "linux/signals/signals.h"
#include <cstddef>
#include <cstdint>
#include <functional>
#include <map>
#include <optional>

namespace operit::arm64::debug {

// All IDs are guest IDs. The runtime calls this service on its serialized
// coordinator and keeps a returned CPU pointer valid until the API call ends.
// Hooks must not retire tasks or re-enter this service during a callback.
struct Target {
    uint32_t tid = 0, tgid = 0, parent_tid = 0, pgrp = 0;
    uint32_t ruid = 0, euid = 0, suid = 0, rgid = 0, egid = 0, sgid = 0;
    uint64_t permitted_capabilities = 0;
    bool alive = false, dumpable = true, cap_sys_ptrace = false;
    CpuState *cpu = nullptr;
};

enum class Access { Read, Write, ForceRead, ForceWrite };

struct Hooks {
    std::function<std::optional<Target>(uint32_t)> target;
    // Ordinary transfers access the named task's memory as a user pointer.
    // Force transfers implement Linux ptrace FOLL_FORCE, including COW and
    // executable-cache invalidation. Return false if inaccessible; never
    // temporarily change the VMA's guest-visible protection.
    std::function<bool(uint32_t, uint64_t, void *, size_t, Access)> memory;
    // This is separate from group-stop and from Task::Waiting. A stopped
    // tracee's pending blocking operation must remain intact.
    std::function<void(uint32_t, bool)> set_stopped;
    // Queue guest SIGSTOP and interrupt a blocking operation if necessary;
    // the stop is reported only at signal_delivery_stop, not at ATTACH itself.
    std::function<bool(uint32_t)> attach_stop;
    // Apply the selected signal once, without re-entering a ptrace delivery
    // stop. nullopt means suppress the intercepted signal. Called only after a
    // signal-delivery stop; the original info/forced flag survive unchanged if
    // the tracer supplies the signal number in the current siginfo.
    std::function<void(uint32_t, std::optional<signals::Queued>)> resume_signal;
    // Queue a new thread-directed signal through the normal delivery path.
    // Unlike resume_signal, the queued signal may cause a new ptrace stop.
    std::function<void(uint32_t, signals::Queued)> queue_signal;
    // Make blocked tracer wait calls re-poll and issue guest SIGCHLD with
    // CLD_TRAPPED for a stop (or CLD_EXITED/KILLED/DUMPED for an exit).
    std::function<void(uint32_t, uint32_t, int)> notify;
    // A tracer died before consuming this tracee's exit. Release the exit to
    // the real parent without reaping it; the parent's waiter must be woken.
    std::function<void(uint32_t)> release_exit;
};

struct WaitRequest {
    enum class Kind { Any, Tid, Group } kind = Kind::Any;
    uint32_t id = 0, options = 0;
};
struct WaitResult {
    uint32_t tid = 0, uid = 0;
    int status = 0;
    bool exited = false;
};

// One instance per VM. Never call from an executing interpreter worker.
class Ptrace final {
  public:
    static constexpr uint32_t o_tracesysgood = 0x1;
    static constexpr uint32_t o_tracefork = 0x2;
    static constexpr uint32_t o_tracevfork = 0x4;
    static constexpr uint32_t o_traceclone = 0x8;
    static constexpr uint32_t o_traceexec = 0x10;
    static constexpr uint32_t o_tracevforkdone = 0x20;
    static constexpr uint32_t o_traceexit = 0x40;

  private:
    enum class Stop { Signal, Entry, Exit, Fork, VforkDone, Exec, EventExit };
    struct Event {
        Stop kind = Stop::Signal;
        int status = 0;
        uint64_t message = 0;
        bool reported = false;
        signals::Info info{};
    };
    struct Tracee {
        uint32_t tracer_tid = 0, tracer_tgid = 0, tgid = 0, pgrp = 0, uid = 0;
        uint32_t options = 0;
        bool syscall_trace = false, dead = false, force_capable = false;
        int exit_status = 0;
        std::optional<uint64_t> saved_syscall_x7;
        std::optional<uint64_t> event_syscall_x0, saved_syscall_result, vfork_syscall_x0;
        std::optional<uint32_t> pending_vfork_done;
        std::optional<Event> stop;
        std::optional<signals::Queued> pending_signal;
    };
    Hooks hooks_;
    std::map<uint32_t, Tracee> tracees_;

    std::optional<Target> live(uint32_t tid) const;
    Tracee *owned(uint32_t caller_tid, uint32_t tid, bool require_stop);
    void stop(uint32_t tid, Event event);
    void stop_vfork_done(uint32_t tid, uint32_t child_tid);
    void unstop(uint32_t tid, Tracee &tracee);
    int64_t regset(uint32_t caller_tid, const Target &target, bool write,
                   uint64_t type, uint64_t iovec);

  public:
    explicit Ptrace(Hooks hooks);
    Ptrace(const Ptrace &) = delete;
    Ptrace &operator=(const Ptrace &) = delete;

    bool traced(uint32_t tid) const noexcept;
    // Independent of the current stop; false after detach or task_exit.
    bool wants_exit_event(uint32_t tid) const noexcept;
    bool empty() const noexcept { return tracees_.empty(); }

    // Raw syscall 117 ABI. On PEEK* the word is copied to caller `data` and
    // the result is 0, exactly like the kernel's generic_ptrace_peekdata.
    int64_t request(uint32_t caller_tid, uint64_t request, int64_t pid,
                    uint64_t addr, uint64_t data);

    // Probe before the ordinary child wait. eligible remains true even when
    // the tracee has no event yet. WNOWAIT does not consume an event. On a
    // consumed exit event the runtime can release the real parent's wait.
    std::optional<WaitResult> wait(uint32_t caller_tid, const WaitRequest &request,
                                   bool &eligible);

    // SVC has advanced PC before entry. On true, save this pending syscall;
    // after resume execute it once without entering the interpreter first.
    // AArch64 saves the syscall number separately from x8 before the stop.
    // Changing x8 via SETREGSET must not silently change that saved number;
    // NT_ARM_SYSTEM_CALL, which can modify it in Linux, is not implemented.
    bool syscall_entry(uint32_t tid);
    // Call once x0 is final, including from Scheduler::complete for waits.
    bool syscall_exit(uint32_t tid);
    // After selecting/dequeuing a deliverable signal, before disposition or
    // handler-frame creation. On true retain the queued item here, skip normal
    // delivery, and switch tasks. SIGKILL never enters a ptrace stop.
    bool signal_delivery_stop(uint32_t tid, signals::Queued queued);
    // Invoke when a SIGKILL must wake a task already in ptrace-stop.
    void fatal_signal(uint32_t tid);
    // After successful creation and before either task runs in userspace,
    // inherit the relationship when selected and queue the child's SIGSTOP.
    // Event selection follows CLONE_VFORK, exit_signal and CLONE_UNTRACED;
    // CLONE_PTRACE independently requests inheritance without an event.
    void clone(uint32_t parent_tid, uint32_t child_tid, uint64_t flags, uint32_t exit_signal);
    // The runtime completes clone/wakeup synchronously; retain the kernel's
    // not-yet-returned x0 until the event stop has actually been resumed.
    void preserve_event_syscall_result(uint32_t tid);
    // Called before waking the vfork parent after the child releases its VM.
    // A completion behind an existing ptrace-stop is reported after resume.
    void vfork_done(uint32_t parent_tid, uint32_t child_tid);
    // A non-leader exec whose destination TID is already a tracee is not yet
    // supported, even if the execing thread is untraced. Check before commit.
    bool can_exec(uint32_t old_tid, uint32_t new_tid) const;
    // Call after exec's TID rekey and before execution of the new image. This
    // also rekeys any tracer relationships owned by the execing thread. Do not
    // call tracer_exit(old_tid) for that thread as part of the rekey. Without
    // TRACEEXEC this queues a legacy SIGTRAP through normal signal delivery.
    void exec(uint32_t old_tid, uint32_t new_tid);
    // Before exit side effects or task retirement, report the anticipated wait
    // status if TRACEEXIT is enabled. On true, suspend finalization until the
    // tracee resumes; this stop is not task_exit. Call at most once for that
    // exit after a true result, including after resume or tracer death.
    bool exit_event(uint32_t tid, int encoded_status);
    // Call before retiring each traced task; status is a wait status, not an
    // exit code. A true result means tracer exit must be waited before the
    // real parent's normal exit notification/reap is released.
    bool task_exit(uint32_t tid, int status);
    // Call for each dying tracer thread (even if peers in its TGID live).
    // The tracees are detached and resumed preserving independent group-stop.
    void tracer_exit(uint32_t tid);
};

} // namespace operit::arm64::debug
