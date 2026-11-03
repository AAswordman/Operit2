#pragma once
#include "linux/files/memory_file.h"
#include "linux/process/resource_limits.h"
#include <cstdint>
#include <functional>
#include <memory>
#include <optional>

namespace operit::arm64 {
class Scheduler;
namespace process {
struct UidLimitState;
struct UidLimitUsage {
    uint64_t tasks = 0, pending_signals = 0;
};

// One actual guest task or allocated sigqueue entry. Move it into the owning
// task/queue only after admission succeeds. Destruction rolls back failed
// creation and releases the charge on final removal, independently of later
// credential changes. Never copy a parent's charge into a child.
class UidLimitCharge {
    friend class UidLimitAccounting;
    enum class Kind { Task, Signal };
    std::shared_ptr<UidLimitState> state_;
    uint32_t uid_ = 0;
    Kind kind_ = Kind::Task;
    UidLimitCharge(std::shared_ptr<UidLimitState> state, uint32_t uid, Kind kind);

  public:
    UidLimitCharge() = default;
    UidLimitCharge(UidLimitCharge &&other) noexcept;
    UidLimitCharge &operator=(UidLimitCharge &&other) noexcept;
    UidLimitCharge(const UidLimitCharge &) = delete;
    UidLimitCharge &operator=(const UidLimitCharge &) = delete;
    ~UidLimitCharge();
    void reset() noexcept;
    explicit operator bool() const { return bool(state_); }
    uint32_t real_uid() const { return uid_; }
};

// One accounting domain per guest runtime/user namespace. Copies share the
// domain; forked processes and all their threads must use that same domain.
class UidLimitAccounting {
    std::shared_ptr<UidLimitState> state_;
    UidLimitCharge reserve(uint32_t real_uid, UidLimitCharge::Kind kind,
                          uint64_t limit, bool override_limit) const;

  public:
    UidLimitAccounting();
    UidLimitUsage usage(uint32_t real_uid) const;
    // Bootstrap tasks must be counted even if a subsequently lowered limit
    // already lies below their count. Threads count individually, including
    // the thread-group leader until its final removal/reap.
    UidLimitCharge track_task(uint32_t real_uid) const;
    // Before clone/fork publication. privileged is guest CAP_SYS_ADMIN or
    // CAP_SYS_RESOURCE, never a host capability. Initial-namespace uid 0 is exempt.
    UidLimitCharge reserve_task(uint32_t real_uid, const ResourceLimits &limits,
                              bool privileged = false) const;
    // Move a task's existing charge after a real-uid change. Linux does not
    // reject setuid for NPROC; callers can use task_limit_exceeded at exec.
    void reassign_task(UidLimitCharge &charge, uint32_t real_uid) const;
    bool task_limit_exceeded(uint32_t real_uid, const ResourceLimits &limits,
                              bool privileged = false) const;
    // Reserve for the TARGET real uid and target limits, once per allocated
    // sigqueue object, after standard-signal coalescing. EAGAIN on exhaustion.
    // Kernel/ordinary kill allocation may override the cap but still counts.
    // POSIX timers retain one charge for their preallocated queue object.
    UidLimitCharge reserve_signal(uint32_t real_uid, const ResourceLimits &limits,
                                 bool override_limit = false) const;
};

// Returns Linux send status (0 or a negative errno), never signals the host.
// CPU expiry is process-wide; FSIZE targets the calling guest thread. A missing
// controller/retired target returns ESRCH so callers can detect absent delivery.
int send_limit_signal(Scheduler &scheduler, uint32_t tid, int signal, bool process_wide = false);
// Call at a serial safe point after settling all workers' CPU counters. Return
// value is 0/no expiry, a positive generated signal, or a negative delivery errno.
int enforce_cpu_limit(Scheduler &scheduler, uint32_t tid, ResourceLimits &limits,
                      uint64_t user_nanoseconds, uint64_t system_nanoseconds);
// Per-thread RT service, process-directed SI_KERNEL limit signals.
int enforce_realtime_limit(Scheduler &scheduler, uint32_t tid, ResourceLimits &limits,
                           uint64_t runtime_nanoseconds);

// No guest-memory access is allowed while this returned guard is held. The
// memfd guard serializes offset inspection, FSIZE prefix reduction and seals
// with native I/O. Use length(), even for ordinary files. Copy guest buffers
// before acquiring it; release before guest copy-back or scheduler waits.
// A preliminary short-lived guard may be used to bound a guest buffer copy;
// acquire again for the native operation because limits/offset/seals can change.
files::MemoryFileGuard guard_file_write(const ResourceLimits &limits, int host_fd, uint64_t count,
                                        std::optional<uint64_t> offset = std::nullopt,
                                        std::optional<bool> append = std::nullopt,
                                        const std::function<void()> &file_size_signal = {});
} // namespace process
} // namespace operit::arm64
