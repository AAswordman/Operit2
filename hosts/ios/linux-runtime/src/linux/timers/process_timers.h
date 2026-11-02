#pragma once
#include "runtime/process/controller.h"
#include <memory>

namespace operit::arm64 {
namespace process {
class UidLimitCharge;
}
struct ProcessTimerHooks {
    signals::Hooks signals;
    // One preallocated sigqueue per POSIX timer, charged to its creator's real
    // UID and RLIMIT_SIGPENDING even for SIGEV_NONE. EAGAIN rejects creation.
    std::function<std::shared_ptr<process::UidLimitCharge>(uint32_t)> reserve_signal;
    // Called only at a serial safe point after worker CPU deltas are committed.
    // IDs are explicit guest PID/TIDs; nullopt means that the target has exited
    // and been removed. Cross-process reads must not use the caller's identity.
    std::function<std::optional<CpuUsage>(uint32_t, bool per_thread)> cpu_usage;
};

class ProcessTimers {
    struct Impl;
    std::unique_ptr<Impl> impl_;

  public:
    explicit ProcessTimers(uint32_t pid);
    ~ProcessTimers();
    ProcessTimers(const ProcessTimers &) = delete;
    ProcessTimers &operator=(const ProcessTimers &) = delete;
    ProcessTimers(ProcessTimers &&) noexcept;
    ProcessTimers &operator=(ProcessTimers &&) noexcept;

    // AArch64 102/103 and 107..111. Fork creates an empty instance for the child.
    int64_t dispatch(CpuState &cpu, Memory &memory, ProcessTimerHooks &hooks);
    void poll(ProcessTimerHooks &hooks);
    // Feed this into the VM's idle wakeup deadline; CPU timers never advance idle time.
    signals::Deadline next_deadline(const ProcessTimerHooks &hooks) const;
    // Invoke on every dequeued signal before handler-frame/sigtimedwait copies.
    // False means a notification invalidated by exec; discard that signal.
    bool signal_dequeued(signals::Info &info, ProcessTimerHooks &hooks);
    // Invoke when a queued signal is discarded without being dequeued.
    void signal_discarded(const signals::Info &info);
    // POSIX timers are deleted by exec; the three interval timers survive it.
    void exec_reset();
    void clear();
};
} // namespace operit::arm64
