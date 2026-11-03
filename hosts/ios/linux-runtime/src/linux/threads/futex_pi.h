#pragma once
#include "runtime/tasks/scheduler.h"
#include <functional>
#include <optional>

namespace operit::arm64::threads {
constexpr uint32_t futex_tid_mask = 0x3fffffffu;
constexpr uint32_t futex_owner_died = 0x40000000u;
constexpr uint32_t futex_waiters = 0x80000000u;

struct FutexPiTimeout {
    int64_t seconds = 0, nanoseconds = 0;
    bool realtime = false;
};
std::optional<FutexPiTimeout> futex_pi_timeout(Memory &memory, uint64_t address, bool realtime);
int64_t futex_pi_lock(Scheduler &scheduler, Memory &memory, uint64_t address, bool private_futex,
                      bool trylock, std::optional<FutexPiTimeout> timeout = {},
                      const std::function<bool()> &interrupted = {});
int64_t futex_pi_unlock(Scheduler &scheduler, Memory &memory, uint64_t address, bool private_futex);
// Condition wait and PI mutex acquisition are distinct phases. The timeout is
// absolute and spans both; success is returned only after acquiring the target.
int64_t futex_wait_requeue_pi(Scheduler &scheduler, Memory &memory, uint64_t source,
                              uint32_t expected, uint64_t target, bool private_futex,
                              std::optional<FutexPiTimeout> timeout = std::nullopt,
                              const std::function<bool()> &interrupted = nullptr);
int64_t futex_cmp_requeue_pi(Scheduler &scheduler, Memory &memory, uint64_t source,
                             uint64_t target, bool private_futex, uint32_t wake_count,
                             uint32_t requeue_count, uint32_t expected);
// Signal interruption before requeue restarts unconditionally; after requeue
// it returns EAGAIN and drops its PI donation via the normal wait cleanup.
int64_t futex_pi_interrupt(Scheduler &scheduler, Task &task, int64_t result);
void futex_pi_check_non_pi(const Scheduler &scheduler, Scheduler::FutexAddress key);
void futex_pi_owner_died(Scheduler &scheduler, Memory &memory, uint64_t address, uint32_t tid);
void futex_pi_task_exit(Scheduler &scheduler, Memory &memory, Task &task);

// Scheduler integration: priorities use Linux's internal order (smaller wins;
// SCHED_OTHER defaults to 120). The source reads base priorities, never inherited
// priorities. Query at both serial and parallel runnable-task selection points.
void futex_pi_set_priority_source(Scheduler &scheduler, std::function<int(uint32_t)> source);
int futex_pi_effective_priority(const Scheduler &scheduler, uint32_t tid);
// Call before destroying the Scheduler, after guest workers have stopped.
void futex_pi_vm_exit(Scheduler &scheduler) noexcept;
} // namespace operit::arm64::threads
