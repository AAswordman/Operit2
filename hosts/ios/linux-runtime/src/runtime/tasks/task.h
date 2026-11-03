#pragma once
#include "cpu/aarch64/state.h"
#include "linux/process/limit_enforcement.h"
#include "linux/ipc/sysv_sem.h"
#include "linux/security/seccomp.h"
#include "linux/signals/signals.h"
#include "linux/threads/rseq.h"
#include <chrono>
#include <functional>
#include <optional>
#include <vector>

namespace operit::arm64 {
using TaskClock = std::chrono::steady_clock;
using TaskDeadline = std::optional<TaskClock::time_point>;
constexpr uint64_t round_robin_interval_nanoseconds = 100000000;
struct Task {
    enum class SchedulingPolicy : uint32_t { Other = 0, Fifo = 1, RoundRobin = 2, Batch = 3, Idle = 5 };
    uint32_t tid = 1, pid = 1;
    // Owned by this task; a new child receives its own admission charge.
    process::UidLimitCharge task_limit_charge;
    uint64_t start_boottime_nanoseconds = 0;
    uint32_t vfork_child = 0, last_cpu = 0;
    uint64_t cpu_affinity = UINT64_MAX;
    SchedulingPolicy sched_policy = SchedulingPolicy::Other;
    uint32_t rt_priority = 0;
    int32_t nice = 0;
    bool sched_reset_on_fork = false;
    uint64_t rr_remaining_nanoseconds = round_robin_interval_nanoseconds;
    uint64_t realtime_runtime_nanoseconds = 0;
    bool realtime_watchdog_pi_boosted = false;
    // 100 means absent from the realtime queue; effective priorities <100
    // include ordinary owners temporarily boosted by a PI waiter.
    int realtime_queue_priority = 100;
    int base_priority() const noexcept {
        if (sched_policy == SchedulingPolicy::Fifo || sched_policy == SchedulingPolicy::RoundRobin)
            return 99 - int(rt_priority);
        // Keep idle-policy tasks behind every ordinary nice level. A PI donor
        // can still raise them through the same effective-priority query.
        return sched_policy == SchedulingPolicy::Idle ? 140 : 120 + nice;
    }
    uint64_t user_nanoseconds = 0, system_nanoseconds = 0, runtime_nanoseconds = 0;
    uint64_t minor_faults = 0, major_faults = 0, input_blocks = 0, output_blocks = 0;
    uint64_t voluntary_switches = 0, involuntary_switches = 0;
    CpuState cpu;
    threads::RseqState rseq;
    security::SeccompState seccomp;
    std::unique_ptr<ipc::SemaphoreTask> semaphore_task;
    signals::ThreadState signals;
    uint64_t signal_mask = 0, clear_tid = 0, robust_list = 0;
    std::string name = "operit";
    enum class State { Runnable, Waiting, Exited } state = State::Runnable;
    bool stopped = false;
    bool ptrace_stopped = false, ptrace_exit_pending = false;
    std::optional<uint64_t> ptrace_syscall;
    std::optional<uint64_t> ptrace_user_x8;
    std::optional<int> ptrace_deferred_exit;
    std::optional<signals::Queued> ptrace_signal;
    uint64_t futex = 0;
    uint64_t futex_space = 0;
    uint32_t futex_mask = UINT32_MAX;
    bool futex_requeue_pi = false;
    int futex_priority = 100;
    uint64_t futex_sequence = 0;
    struct FutexVectorEntry {
        uint64_t address, space, sequence = 0;
        uint32_t mask = UINT32_MAX;
        bool queued = true;
    };
    std::vector<FutexVectorEntry> futex_vector;
    int futex_vector_result = -1;
    TaskDeadline deadline;
    int64_t timeout_result = 0;
    std::function<std::optional<int64_t>()> poll;
    std::function<void()> wait_cleanup;
};
} // namespace operit::arm64
