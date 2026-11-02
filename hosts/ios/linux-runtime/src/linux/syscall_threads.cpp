#include "linux/syscalls.h"
#include "linux/threads/futex_pi.h"
#include "runtime/process/controller.h"
#include <algorithm>
#include <atomic>
#include <chrono>
#include <climits>
#include <time.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
using SchedulingPolicy = Task::SchedulingPolicy;
constexpr uint32_t sched_reset_on_fork = 0x40000000u;
bool realtime_policy(SchedulingPolicy policy) noexcept {
    return policy == SchedulingPolicy::Fifo || policy == SchedulingPolicy::RoundRobin;
}
bool valid_scheduling_policy(uint32_t policy) noexcept {
    return policy == 0 || policy == 1 || policy == 2 || policy == 3 || policy == 5;
}
Task &scheduling_target(Scheduler &scheduler, uint64_t id) {
    const int32_t tid = int32_t(id);
    if (tid < 0)
        guest_error(22);
    return scheduler.find(uint32_t(tid));
}
bool same_scheduling_owner(Scheduler &scheduler, const Task &target) {
    if (target.pid == scheduler.current().pid || !scheduler.processes())
        return true;
    const auto hooks = scheduler.processes()->signal_hooks();
    const auto caller = hooks.process(scheduler.current().pid);
    const auto owner = hooks.process(target.pid);
    return caller.effective_uid == owner.real_uid || caller.effective_uid == owner.effective_uid;
}
std::shared_ptr<process::ResourceLimits>
scheduling_limits(Scheduler &scheduler, const Task &target,
                  const std::shared_ptr<process::ResourceLimits> &caller_limits) {
    if (target.pid == scheduler.current().pid)
        return caller_limits;
    auto *controller = scheduler.processes();
    auto *syscalls = controller ? controller->process_syscalls(target.pid) : nullptr;
    if (!syscalls)
        guest_error(3);
    return syscalls->resource_limits();
}
void check_scheduling_change(Scheduler &scheduler, const Task &target, SchedulingPolicy policy,
                             uint32_t priority, bool reset_on_fork,
                             const std::shared_ptr<process::ResourceLimits> &caller_limits) {
    // capget/capset expose an empty effective guest capability set. In
    // particular, host uid 0 never supplies guest CAP_SYS_NICE. All exceptions
    // below therefore require the target's actual guest resource limits.
    if (!same_scheduling_owner(scheduler, target))
        guest_error(1);
    if (target.sched_reset_on_fork && !reset_on_fork)
        guest_error(1);
    if (realtime_policy(policy)) {
        const auto limits = scheduling_limits(scheduler, target, caller_limits);
        const auto maximum = limits->get(process::Resource::RealtimePriority).soft;
        if ((policy != target.sched_policy && !maximum) ||
            (priority > target.rt_priority && priority > maximum))
            guest_error(1);
    }
    if (target.sched_policy == SchedulingPolicy::Idle && policy != SchedulingPolicy::Idle) {
        const auto limits = scheduling_limits(scheduler, target, caller_limits);
        if (uint64_t(20 - target.nice) > limits->get(process::Resource::Nice).soft)
            guest_error(1);
    }
}
} // namespace

TaskDeadline Syscalls::thread_deadline(uint64_t address, bool absolute, bool realtime) {
    if (!address)
        return {};
    check_buffer(address, 16, read_perm);
    int64_t seconds = int64_t(memory_.load(address, 8));
    int64_t nanos = int64_t(memory_.load(address + 8, 8));
    if (seconds < 0 || nanos < 0 || nanos >= 1000000000)
        guest_error(22);
    const auto now = TaskClock::now();
    if (absolute) {
        timespec host{};
        ::clock_gettime(realtime ? CLOCK_REALTIME : CLOCK_MONOTONIC, &host);
        seconds -= host.tv_sec;
        nanos -= host.tv_nsec;
    }
    if (seconds < 0)
        return now;
    // Saturate huge valid Linux timespecs instead of overflowing host chrono.
    const auto maximum =
        std::chrono::duration_cast<std::chrono::seconds>(TaskClock::time_point::max() - now).count();
    if (seconds >= maximum)
        return TaskClock::time_point::max();
    return now + std::chrono::seconds(seconds) + std::chrono::nanoseconds(nanos);
}
void Syscalls::release_robust_list(Task &task) {
    // Linux walks at most ROBUST_LIST_LIMIT entries and ignores inaccessible
    // user pointers during exit. List links carry the PI marker in bit zero.
    const Memory::AccessGuard access(memory_);
    const uint64_t head = task.robust_list;
    task.robust_list = 0;
    if (!head || !memory_.accessible(head, 24, read_perm))
        return;
    const int64_t offset = int64_t(memory_.load(head + 8, 8));
    const uint64_t pending = memory_.load(head + 16, 8);
    auto release = [&](uint64_t link, bool pending_operation) {
        const bool pi = link & 1;
        const uint64_t entry = link & ~uint64_t{1};
        if (!entry)
            return true;
        const uint64_t address = entry + uint64_t(offset);
        try {
            if ((address & 3) || !memory_.accessible(address, 4, read_perm | write_perm))
                return false;
            if (pi) {
                threads::futex_pi_owner_died(scheduler_, memory_, address, task.tid);
                return true;
            }
            const uint32_t value = uint32_t(memory_.load(address, 4));
            const uint32_t owner = value & threads::futex_tid_mask;
            const bool wake_pending = pending_operation && !owner && !(value & threads::futex_owner_died);
            if (owner == task.tid) {
                memory_.store(address, (value & threads::futex_waiters) | threads::futex_owner_died, 4);
            }
            if (wake_pending || (owner == task.tid && (value & threads::futex_waiters))) {
                const auto key = scheduler_.futex_address(memory_, address);
                scheduler_.wake_futex(key.address, 1, UINT32_MAX, key.space);
            }
            return true;
        } catch (const MemoryFault &) {
        } catch (const GuestError &) {
        }
        return false;
    };
    uint64_t link = memory_.load(head, 8);
    for (unsigned count = 0; count < 2048 && (link & ~uint64_t{1}) != head; ++count) {
        const uint64_t entry = link & ~uint64_t{1};
        if (!entry || !memory_.accessible(entry, 8, read_perm))
            break;
        uint64_t next;
        try {
            next = memory_.load(entry, 8);
        } catch (const MemoryFault &) {
            break;
        } catch (const GuestError &) {
            break;
        }
        if (entry != (pending & ~uint64_t{1}) && !release(link, false))
            break;
        link = next;
    }
    release(pending, true);
}
void Syscalls::release_task_resources(Task &task) {
    // Runs for exit, fatal signals and exec while the old address space exists.
    // Cancel a queued lock first, then release every contended PI lock it owns,
    // including locks missing from a corrupt or unfinished robust list.
    threads::futex_pi_task_exit(scheduler_, memory_, task);
    try {
        release_robust_list(task);
    } catch (const MemoryFault &) {
        // Linux silently abandons an inaccessible robust list during exit.
    } catch (const GuestError &) {
    }
    task.robust_list = 0;
    if (task.clear_tid) {
        try {
            if (memory_.accessible(task.clear_tid, 4, write_perm)) {
                memory_.store(task.clear_tid, 0, 4);
                auto key = scheduler_.futex_address(memory_, task.clear_tid);
                scheduler_.wake_futex(key.address, 1, UINT32_MAX, key.space);
            }
        } catch (const MemoryFault &) {
        } catch (const GuestError &) {
        }
    }
    task.clear_tid = 0;
}
int64_t Syscalls::dispatch_threads(CpuState &cpu) {
    const auto &a = cpu.x;
    auto &task = scheduler_.current();
    switch (a[8]) {
    case 93:
    case 94: {
        const auto pid = task.pid;
        const auto code = int(a[0]) & 255;
        if (a[8] == 94) {
            for (auto tid : scheduler_.live_tids(pid))
                release_task_resources(scheduler_.find(tid));
        } else {
            release_task_resources(task);
        }
        scheduler_.exit(code, a[8] == 94);
        exited = !scheduler_.process_alive(pid);
        exit_code = code;
        return 0;
    }
    case 96:
        task.clear_tid = a[0];
        return task.tid;
    case 99:
        if (a[1] != 24)
            guest_error(22);
        task.robust_list = a[0];
        return 0;
    case 100: { // get_robust_list
        auto &target = scheduler_.find(uint32_t(a[0]));
        check_buffer(a[1], 8, write_perm);
        check_buffer(a[2], 8, write_perm);
        memory_.store(a[1], target.robust_list, 8);
        memory_.store(a[2], 24, 8);
        return 0;
    }
    case 118: // sched_setparam: retain the target's policy and fork-reset flag.
    case 119: { // sched_setscheduler: guest scheduling only, never the host thread.
        const uint64_t parameter = a[a[8] == 118 ? 1 : 2];
        if (int32_t(a[0]) < 0 || !parameter)
            guest_error(22);
        check_buffer(parameter, 4, read_perm);
        const uint32_t priority = uint32_t(memory_.load(parameter, 4));
        auto &target = scheduling_target(scheduler_, a[0]);
        const uint32_t requested = a[8] == 118 ? uint32_t(target.sched_policy) : uint32_t(a[1]);
        const uint32_t policy_number = requested & ~sched_reset_on_fork;
        if (!valid_scheduling_policy(policy_number))
            guest_error(22);
        const auto policy = SchedulingPolicy(policy_number);
        const bool reset_on_fork = a[8] == 118 ? target.sched_reset_on_fork
                                              : bool(requested & sched_reset_on_fork);
        if (priority > 99 || (realtime_policy(policy) != (priority != 0)))
            guest_error(22);
        check_scheduling_change(scheduler_, target, policy, priority, reset_on_fork, resource_limits());
        target.sched_policy = policy;
        target.rt_priority = priority;
        target.sched_reset_on_fork = reset_on_fork;
        return 0;
    }
    case 120: { // sched_getscheduler returns the requested policy, not a PI boost.
        const auto &target = scheduling_target(scheduler_, a[0]);
        return uint32_t(target.sched_policy) | (target.sched_reset_on_fork ? sched_reset_on_fork : 0);
    }
    case 121: { // sched_getparam returns the real per-task base RT priority.
        if (int32_t(a[0]) < 0 || !a[1])
            guest_error(22);
        const auto &target = scheduling_target(scheduler_, a[0]);
        check_buffer(a[1], 4, write_perm);
        memory_.store(a[1], target.rt_priority, 4);
        return 0;
    }
    case 122: { // sched_setaffinity uses actual virtual CPU execution slots.
        auto &target = scheduler_.find(uint32_t(a[0]));
        if (!a[1])
            guest_error(22);
        const uint64_t count = std::min<uint64_t>(a[1], 8);
        check_buffer(a[2], count, read_perm);
        uint64_t requested = 0;
        memory_.copy_out(a[2], &requested, count);
        const auto cpus = scheduler_.execution_cpus();
        const uint64_t available = cpus == 64 ? UINT64_MAX : (uint64_t{1} << cpus) - 1;
        requested &= available;
        if (!requested)
            guest_error(22);
        target.cpu_affinity = requested;
        return 0;
    }
    case 123: { // sched_getaffinity returns the guest kernel's 64-bit CPU mask.
        const auto &target = scheduler_.find(uint32_t(a[0]));
        if (a[1] < 8)
            guest_error(22);
        check_buffer(a[2], 8, write_perm);
        const auto cpus = scheduler_.execution_cpus();
        const uint64_t available = cpus == 64 ? UINT64_MAX : (uint64_t{1} << cpus) - 1;
        memory_.store(a[2], target.cpu_affinity & available, 8);
        return 8;
    }
    case 125: // sched_get_priority_max
    case 126: { // sched_get_priority_min
        const uint32_t policy = uint32_t(a[0]);
        if (!valid_scheduling_policy(policy))
            guest_error(22);
        return realtime_policy(SchedulingPolicy(policy)) ? (a[8] == 125 ? 99 : 1) : 0;
    }
    case 140: // setpriority
    case 141: { // getpriority: raw Linux result is 20 - nice.
        const uint32_t which = uint32_t(a[0]);
        const uint32_t who = uint32_t(a[1]);
        if (which > 2)
            guest_error(22);
        auto *controller = scheduler_.processes();
        auto group = [&](const Task &target) {
            return controller ? controller->process_group(target.pid) : target.pid;
        };
        auto uid = [&](const Task &target) {
            return controller ? controller->signal_hooks().process(target.pid).real_uid
                              : uint32_t(::getuid());
        };
        const uint32_t id = who ? who : which == 0 ? task.tid : which == 1 ? group(task) : uid(task);
        const int32_t requested_nice = std::clamp(int32_t(a[2]), int32_t{-20}, int32_t{19});
        bool matched = false;
        int best_nice = 20, error = 0;
        for (const auto tid : scheduler_.live_tids()) {
            auto &target = scheduler_.find(tid);
            const uint32_t target_id = which == 0 ? target.tid : which == 1 ? group(target) : uid(target);
            if (target_id != id)
                continue;
            matched = true;
            if (a[8] == 141) {
                best_nice = std::min(best_nice, int(target.nice));
                continue;
            }
            try {
                if (!same_scheduling_owner(scheduler_, target))
                    guest_error(1);
                if (requested_nice < target.nice) {
                    const auto limits = scheduling_limits(scheduler_, target, resource_limits());
                    if (uint64_t(20 - requested_nice) > limits->get(process::Resource::Nice).soft)
                        guest_error(13); // Raising nice priority needs RLIMIT_NICE or guest CAP_SYS_NICE.
                }
                target.nice = requested_nice;
            } catch (const GuestError &failure) {
                if (!error)
                    error = failure.number;
            }
        }
        if (!matched)
            guest_error(3);
        if (error)
            guest_error(error);
        return a[8] == 141 ? 20 - best_nice : 0;
    }
    case 178:
        return task.tid;
    case 98: { // futex: syscall dispatch runs after all guest workers stop.
        if (a[1] & ~uint64_t{0x1ff})
            guest_error(38);
        const unsigned operation = a[1] & 127;
        if ((a[1] & 256) && operation != 9 && operation != 13)
            guest_error(38);
        if (a[0] & 3)
            guest_error(22);
        if (operation == 6 || operation == 7 || operation == 8 || operation == 13) {
            if (operation == 7) // FUTEX_UNLOCK_PI
                return threads::futex_pi_unlock(scheduler_, memory_, a[0], a[1] & 128);
            const bool trylock = operation == 8; // FUTEX_TRYLOCK_PI ignores timeout.
            const auto timeout =
                trylock ? std::optional<threads::FutexPiTimeout>{}
                        : threads::futex_pi_timeout(memory_, a[3], operation == 6 || (a[1] & 256));
            return threads::futex_pi_lock(scheduler_, memory_, a[0], a[1] & 128, trylock, timeout,
                                         [this] { return interrupt_for_pending_signal(); });
        }
        const Memory::AccessGuard access(memory_);
        check_buffer(a[0], 4, read_perm);
        const uint32_t value = uint32_t(memory_.load(a[0], 4));
        const auto key = scheduler_.futex_address(memory_, a[0], a[1] & 128);
        threads::futex_pi_check_non_pi(scheduler_, key);
        if (operation == 0 || operation == 9) {
            const uint32_t mask = operation == 9 ? uint32_t(a[5]) : UINT32_MAX;
            if (!mask)
                guest_error(22);
            auto deadline = thread_deadline(a[3], operation == 9, a[1] & 256);
            if (value != uint32_t(a[2]))
                guest_error(11);
            if (deadline && *deadline <= TaskClock::now())
                guest_error(110);
            scheduler_.wait_futex(key.address, mask, deadline, key.space);
            return 0;
        }
        if (operation == 1 || operation == 10) {
            const uint32_t mask = operation == 10 ? uint32_t(a[5]) : UINT32_MAX;
            if (!mask || int32_t(a[2]) < 0)
                guest_error(22);
            return scheduler_.wake_futex(key.address, uint32_t(a[2]), mask, key.space);
        }
        if (operation == 3 || operation == 4) {
            if (a[4] & 3 || int32_t(a[2]) < 0 || int32_t(a[3]) < 0)
                guest_error(22);
            check_buffer(a[4], 4, read_perm);
            if (operation == 4 && value != uint32_t(a[5]))
                guest_error(11);
            const auto target = scheduler_.futex_address(memory_, a[4], a[1] & 128);
            threads::futex_pi_check_non_pi(scheduler_, target);
            unsigned woken = scheduler_.wake_futex(key.address, uint32_t(a[2]), UINT32_MAX, key.space);
            return woken + scheduler_.requeue_futex(key.address, target.address, uint32_t(a[3]), key.space,
                                                    target.space);
        }
        if (operation == 5) { // FUTEX_WAKE_OP
            if (a[4] & 3 || int32_t(a[2]) < 0 || int32_t(a[3]) < 0)
                guest_error(22);
            check_buffer(a[4], 4, read_perm | write_perm);
            const auto target = scheduler_.futex_address(memory_, a[4], a[1] & 128);
            threads::futex_pi_check_non_pi(scheduler_, target);
            const uint32_t encoded = uint32_t(a[5]);
            unsigned op = encoded >> 28, cmp = (encoded >> 24) & 15;
            int32_t operand = int32_t(sign_extend((encoded >> 12) & 4095, 12));
            int32_t comparison = int32_t(sign_extend(encoded & 4095, 12));
            if ((op & 7) > 4 || cmp > 5)
                guest_error(38);
            if (op & 8) {
                if (operand < 0 || operand > 31)
                    guest_error(22);
                operand = int32_t(uint32_t{1} << operand);
            }
            const uint32_t old = uint32_t(memory_.load(a[4], 4));
            uint32_t result;
            switch (op & 7) {
            case 0:
                result = uint32_t(operand);
                break;
            case 1:
                result = old + uint32_t(operand);
                break;
            case 2:
                result = old | uint32_t(operand);
                break;
            case 3:
                result = old & ~uint32_t(operand);
                break;
            default:
                result = old ^ uint32_t(operand);
                break;
            }
            memory_.store(a[4], result, 4);
            const int32_t signed_old = int32_t(old);
            bool condition = cmp == 0   ? signed_old == comparison
                             : cmp == 1 ? signed_old != comparison
                             : cmp == 2 ? signed_old < comparison
                             : cmp == 3 ? signed_old <= comparison
                             : cmp == 4 ? signed_old > comparison
                                        : signed_old >= comparison;
            unsigned woken = scheduler_.wake_futex(key.address, uint32_t(a[2]), UINT32_MAX, key.space);
            if (condition) {
                woken += scheduler_.wake_futex(target.address, uint32_t(a[3]), UINT32_MAX, target.space);
            }
            return woken;
        }
        return unsupported(cpu);
    }
    case 101:   // nanosleep
    case 115: { // clock_nanosleep
        if (a[8] == 115 && (a[0] > 1 || (a[1] & ~uint64_t{1})))
            guest_error(22);
        auto deadline = thread_deadline(a[a[8] == 101 ? 0 : 2], a[8] == 115 && (a[1] & 1), a[0] == 0);
        if (!deadline)
            guest_error(14);
        scheduler_.block(deadline, 0);
        return 0;
    }
    case 167:
        return dispatch_control(cpu);
    case 283: // private expedited barriers: all guest tasks share one host executor.
        if (a[1])
            guest_error(22);
        if (a[0] == 0)
            return 8 | 16;
        if (a[0] == 8 || a[0] == 16) {
            std::atomic_thread_fence(std::memory_order_seq_cst);
            return 0;
        }
        guest_error(22);
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
