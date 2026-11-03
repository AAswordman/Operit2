#pragma once

#include <cstdint>
#include <memory>

namespace operit::arm64 {
class Memory;
class Scheduler;
struct CpuState;
namespace signals {
struct ProcessState;
struct ThreadState;
}

namespace security {

// This is task state, not process or address-space state. Copy it when a
// clone/fork task is created; retain the calling task's copy across exec.
// Only a successful TSYNC propagates later installations to existing siblings.
struct SeccompFilter;
class SeccompState {
    bool no_new_privileges_ = false;
    std::shared_ptr<const SeccompFilter> filter_;

    friend int64_t seccomp_prctl(SeccompState &, Memory &, uint64_t, uint64_t, uint64_t, uint64_t,
                                 uint64_t);
    friend int64_t seccomp_syscall(SeccompState &, Memory &, uint64_t, uint64_t, uint64_t, Scheduler *);
    friend struct SeccompAccess;

  public:
    bool no_new_privileges() const { return no_new_privileges_; }
    unsigned mode() const { return filter_ ? 2u : 0u; }
};

enum class SeccompAction { Allow, Log, Errno, Trap, KillThread, KillProcess };
struct SeccompDecision {
    SeccompAction action = SeccompAction::Allow;
    // Errno is the Linux syscall return value, already negated and clamped.
    int64_t value = 0;
    uint16_t data = 0;
};

// Evaluate before dispatching *every* guest SVC, including unknown calls,
// prctl, seccomp, clone, exec, signal returns and exits. cpu.pc must be the
// post-SVC PC; x8 is the syscall number and x0..x5 are untouched arguments.
// Allow dispatches normally; Log also dispatches after audit logging; Errno
// returns value without dispatch; Trap queues SIGSYS and preserves x0;
// KillThread/KillProcess terminate the requested scope as SIGSYS, without
// executing the original syscall. Unknown actions fail closed as KillProcess.
// A last-thread KillThread and every KillProcess produce SIGSYS core status;
// a non-last KillThread retires only the caller and must run ordinary thread
// resource/clear-tid cleanup. Do not send a catchable SIGSYS for either kill.
SeccompDecision seccomp_evaluate(const SeccompState &state, const CpuState &cpu);

// Call for Trap at the serial syscall boundary before signal delivery.
// The forced SIGSYS has SYS_SECCOMP, si_errno=data, call_addr=cpu.pc,
// si_syscall=(int32_t)x8, si_arch=AUDIT_ARCH_AARCH64. Do not dispatch or
// overwrite x0, and do not restart the filtered syscall.
void seccomp_force_trap(signals::ProcessState &process, signals::ThreadState &thread, uint64_t &signal_mask,
                        const CpuState &cpu, uint16_t data);

// Return Linux-style 0/negative errno. Guest memory faults propagate in the
// same way as other syscall handlers and are translated to -EFAULT by handle.
// Route prctl options 21, 22, 38 and 39 here, and AArch64 syscall 277 below.
// Guest capabilities are currently empty: installation requires no_new_privs.
// SET_NO_NEW_PRIVS accepts (1,0,0,0) only; GET accepts (0,0,0,0) only.
int64_t seccomp_prctl(SeccompState &state, Memory &memory, uint64_t option, uint64_t arg2, uint64_t arg3,
                      uint64_t arg4, uint64_t arg5);
// TSYNC requires the scheduler and must run on its serial syscall coordinator,
// after all parallel guest workers have stopped. Without it TSYNC fails closed.
int64_t seccomp_syscall(SeccompState &state, Memory &memory, uint64_t op, uint64_t flags, uint64_t args,
                        Scheduler *scheduler = nullptr);

} // namespace security
} // namespace operit::arm64
