#pragma once
#include "cpu/aarch64/state.h"
#include "linux/process/limit_enforcement.h"
#include "linux/signals/signals.h"
#include <chrono>
#include <functional>
#include <optional>

namespace operit::arm64 {
using TaskClock = std::chrono::steady_clock;
using TaskDeadline = std::optional<TaskClock::time_point>;
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
    signals::ThreadState signals;
    uint64_t signal_mask = 0, clear_tid = 0, robust_list = 0;
    std::string name = "operit";
    enum class State { Runnable, Waiting, Exited } state = State::Runnable;
    bool stopped = false;
    uint64_t futex = 0;
    uint64_t futex_space = 0;
    uint32_t futex_mask = UINT32_MAX;
    TaskDeadline deadline;
    int64_t timeout_result = 0;
    std::function<std::optional<int64_t>()> poll;
    std::function<void()> wait_cleanup;
};
} // namespace operit::arm64
