#include "linux/threads/futex_pi.h"
#include <algorithm>
#include <map>
#include <memory>
#include <mutex>
#include <new>
#include <time.h>
#include <utility>
#include <vector>

namespace operit::arm64::threads {
namespace {
using Key = std::pair<uint64_t, uint64_t>; // Address space (or shared identity), word.
Key key_of(Scheduler::FutexAddress key) {
    return {key.space, key.address};
}
struct Endpoint {
    Memory *memory;
    uint64_t address;
    bool private_futex;
};
struct PiLock;
struct Waiter {
    uint32_t tid, pid;
    Endpoint endpoint;
    std::optional<FutexPiTimeout> timeout;
    std::weak_ptr<PiLock> lock;
    bool queued = false;
    std::optional<Key> source;
    Key target{};
    bool requeue_pi = false;
};
struct PiLock {
    Key key;
    uint32_t owner;
    std::vector<std::shared_ptr<Waiter>> waiters;
};
struct State {
    Scheduler *scheduler;
    std::map<Key, std::shared_ptr<PiLock>> locks;
    std::map<uint32_t, std::shared_ptr<Waiter>> waiting;
    std::function<int(uint32_t)> priority;
};
struct Registry {
    // Memory::AccessGuard precedes this mutex whenever guest memory is accessed.
    // Cleanup and priority queries need only this mutex. wake() calls cleanup
    // synchronously, so nested acquisition on the coordinator is intentional.
    std::recursive_mutex mutex;
    std::map<const Scheduler *, std::shared_ptr<State>> states;
};
Registry &registry() {
    static Registry value;
    return value;
}
std::shared_ptr<State> state_for(Scheduler &scheduler) {
    auto &slot = registry().states[&scheduler];
    if (!slot) {
        slot = std::make_shared<State>();
        slot->scheduler = &scheduler;
    }
    return slot;
}
void require_word(Memory &memory, uint64_t address) {
    if (address & 3)
        guest_error(22);
    if (!memory.accessible(address, 4, read_perm | write_perm))
        guest_error(14);
}
timespec sample_clock(bool realtime) {
    timespec now{};
    if (::clock_gettime(realtime ? CLOCK_REALTIME : CLOCK_MONOTONIC, &now))
        throw Failure("cannot sample PI futex clock");
    return now;
}
bool expired(const std::optional<FutexPiTimeout> &timeout) {
    if (!timeout)
        return false;
    const auto now = sample_clock(timeout->realtime);
    return timeout->seconds < now.tv_sec ||
           (timeout->seconds == now.tv_sec && timeout->nanoseconds <= now.tv_nsec);
}
TaskDeadline monotonic_deadline(const std::optional<FutexPiTimeout> &timeout) {
    if (!timeout || timeout->realtime)
        return {};
    const auto clock_now = sample_clock(false);
    const auto steady_now = TaskClock::now();
    if (timeout->seconds < clock_now.tv_sec ||
        (timeout->seconds == clock_now.tv_sec && timeout->nanoseconds <= clock_now.tv_nsec))
        return steady_now;
    int64_t seconds = timeout->seconds - clock_now.tv_sec;
    int64_t nanoseconds = timeout->nanoseconds - clock_now.tv_nsec;
    if (nanoseconds < 0) {
        --seconds;
        nanoseconds += 1000000000;
    }
    const auto capacity = TaskClock::time_point::max() - steady_now;
    const auto whole_seconds = std::chrono::duration_cast<std::chrono::seconds>(capacity);
    if (seconds > whole_seconds.count() ||
        (seconds == whole_seconds.count() &&
         std::chrono::nanoseconds(nanoseconds) > capacity - whole_seconds))
        return TaskClock::time_point::max();
    return steady_now + std::chrono::seconds(seconds) + std::chrono::nanoseconds(nanoseconds);
}
void erase_empty(State &state, const std::shared_ptr<PiLock> &lock) {
    if (!lock->waiters.empty())
        return;
    auto it = state.locks.find(lock->key);
    if (it != state.locks.end() && it->second == lock)
        state.locks.erase(it);
}
void detach(State &state, std::shared_ptr<Waiter> waiter) {
    if (!waiter->queued)
        return;
    waiter->queued = false;
    auto it = state.waiting.find(waiter->tid);
    if (it != state.waiting.end() && it->second == waiter)
        state.waiting.erase(it);
    if (auto lock = waiter->lock.lock()) {
        auto &queue = lock->waiters;
        queue.erase(std::remove(queue.begin(), queue.end(), waiter), queue.end());
        erase_empty(state, lock);
    }
    // FUTEX_WAITERS is a conservative hint, as on Linux. In particular, removing
    // the final timed-out waiter must not touch an unmapped or reused user word.
}
int priority_of(const State &state, uint32_t tid, size_t depth) {
    const int base = state.priority ? state.priority(tid) : 120;
    // Every task can wait on only one lock; cycle() rejects cycles at enqueue.
    // Bound recursion defensively without allocating during owner-exit handoff.
    if (depth > state.waiting.size())
        return base;
    int result = base;
    for (const auto &[key, lock] : state.locks) {
        (void)key;
        if (lock->owner != tid)
            continue;
        for (const auto &waiter : lock->waiters)
            if (waiter->queued)
                result = std::min(result, priority_of(state, waiter->tid, depth + 1));
    }
    return result;
}
int priority_of(const State &state, uint32_t tid) {
    return priority_of(state, tid, 0);
}
bool cycle(const State &state, uint32_t owner, uint32_t caller) {
    for (size_t remaining = state.waiting.size() + 1; owner && remaining; --remaining) {
        if (owner == caller)
            return true;
        auto it = state.waiting.find(owner);
        if (it == state.waiting.end())
            return false;
        auto lock = it->second->lock.lock();
        if (!lock || !it->second->queued)
            return false;
        owner = lock->owner;
    }
    return owner != 0;
}
bool ordinary_waiters(const Scheduler &scheduler, Key key) {
    for (auto tid : scheduler.live_tids()) {
        const auto *task = scheduler.lookup(tid);
        if (task && task->state == Task::State::Waiting &&
            task->futex_space == key.first && task->futex == key.second)
            return true;
    }
    return false;
}
std::function<void()> wait_cleanup(const std::shared_ptr<State> &state,
                                    const std::shared_ptr<Waiter> &waiter) {
    std::weak_ptr<State> weak_state = state;
    std::weak_ptr<Waiter> weak_waiter = waiter;
    return [weak_state, weak_waiter] {
        std::lock_guard<std::recursive_mutex> guard(registry().mutex);
        auto owner = weak_state.lock();
        auto pending = weak_waiter.lock();
        if (owner && pending)
            detach(*owner, pending);
    };
}
void finish_error(State &state, const std::shared_ptr<Waiter> &waiter, int error) {
    const uint32_t tid = waiter->tid;
    detach(state, waiter);
    state.scheduler->wake(tid, -int64_t(error));
}
void fail_lock(State &state, const std::shared_ptr<PiLock> &lock, int error) {
    while (!lock->waiters.empty()) {
        auto waiter = lock->waiters.front();
        finish_error(state, waiter, error);
    }
    erase_empty(state, lock);
}
std::shared_ptr<Waiter> next_waiter(State &state, const std::shared_ptr<PiLock> &lock) {
    std::shared_ptr<Waiter> best;
    int best_priority = 0;
    // Removing a waiter can erase the map entry, but the shared lock remains
    // alive throughout handoff. FIFO order is retained among equal priorities.
    for (size_t index = 0; index < lock->waiters.size();) {
        const auto waiter = lock->waiters[index];
        if (!waiter->queued) {
            ++index;
            continue;
        }
        const auto *task = state.scheduler->lookup(waiter->tid);
        if (!task || task->pid != waiter->pid || task->state != Task::State::Waiting) {
            detach(state, waiter);
            continue;
        }
        if (expired(waiter->timeout)) {
            finish_error(state, waiter, 110);
            continue;
        }
        try {
            const auto &endpoint = waiter->endpoint;
            require_word(*endpoint.memory, endpoint.address);
            if (key_of(state.scheduler->futex_address(*endpoint.memory, endpoint.address,
                                                     endpoint.private_futex)) != lock->key) {
                finish_error(state, waiter, 14);
                continue;
            }
        } catch (const MemoryFault &) {
            finish_error(state, waiter, 14);
            continue;
        } catch (const GuestError &error) {
            finish_error(state, waiter, error.number);
            continue;
        }
        const int priority = priority_of(state, waiter->tid);
        if (!best || priority < best_priority) {
            best = waiter;
            best_priority = priority;
        }
        ++index;
    }
    return best;
}
// Called with the global guest-memory guard and registry mutex held. The word
// is committed before wake(), so success always means the waiter owns the lock.
void handoff(State &state, const std::shared_ptr<PiLock> &lock, Memory *memory,
             uint64_t address, bool owner_died) {
    auto next = next_waiter(state, lock);
    if (!memory && next) {
        memory = next->endpoint.memory;
        address = next->endpoint.address;
    }
    if (!memory) {
        erase_empty(state, lock);
        return;
    }
    require_word(*memory, address);
    const uint32_t old = uint32_t(memory->load(address, 4));
    if ((old & futex_tid_mask) != lock->owner &&
        !(owner_died && !(old & futex_tid_mask) && (old & futex_owner_died)))
        guest_error(22);
    if (!next) {
        memory->store(address, owner_died ? (old & futex_waiters) | futex_owner_died : 0, 4);
        erase_empty(state, lock);
        return;
    }
    const uint32_t new_owner = next->tid;
    memory->store(address, new_owner | futex_waiters | (owner_died ? futex_owner_died : 0), 4);
    lock->owner = new_owner;
    detach(state, next);
    state.scheduler->wake(new_owner, 0);
}
} // namespace

std::optional<FutexPiTimeout> futex_pi_timeout(Memory &memory, uint64_t address, bool realtime) {
    if (!address)
        return {};
    const Memory::AccessGuard access(memory);
    if (!memory.accessible(address, 16, read_perm))
        guest_error(14);
    FutexPiTimeout result{int64_t(memory.load(address, 8)), int64_t(memory.load(address + 8, 8)), realtime};
    if (result.seconds < 0 || result.nanoseconds < 0 || result.nanoseconds >= 1000000000)
        guest_error(22);
    return result;
}
int64_t futex_pi_lock(Scheduler &scheduler, Memory &memory, uint64_t address, bool private_futex,
                      bool trylock, std::optional<FutexPiTimeout> timeout,
                      const std::function<bool()> &interrupted) try {
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    require_word(memory, address);
    const Key key = key_of(scheduler.futex_address(memory, address, private_futex));
    const auto state = state_for(scheduler);
    auto &task = scheduler.current();
    if (!task.tid || task.tid > futex_tid_mask)
        guest_error(22);
    if (ordinary_waiters(scheduler, key))
        guest_error(22);
    const uint32_t value = uint32_t(memory.load(address, 4));
    const uint32_t owner = value & futex_tid_mask;
    auto found = state->locks.find(key);
    if (found != state->locks.end() && found->second->owner != owner)
        guest_error(22);
    if (owner == task.tid)
        guest_error(35);
    if (!owner) {
        if (found != state->locks.end())
            guest_error(22);
        memory.store(address, task.tid | (value & futex_owner_died), 4);
        return 0;
    }
    // Even a failed trylock may leave this conservative hint set. It prevents
    // a userspace TID->0 unlock from bypassing an existing kernel waiter.
    memory.store(address, value | futex_waiters, 4);
    const auto *owner_task = scheduler.lookup(owner);
    if (!owner_task || owner_task->state == Task::State::Exited)
        guest_error(3);
    if (trylock)
        guest_error(11);
    if (cycle(*state, owner, task.tid))
        guest_error(35);
    if (expired(timeout))
        guest_error(110);
    if (interrupted && interrupted())
        guest_error(4);
    if (task.state != Task::State::Runnable || task.wait_cleanup || state->waiting.count(task.tid))
        guest_error(22);
    auto lock = found == state->locks.end() ? std::make_shared<PiLock>(PiLock{key, owner, {}})
                                            : found->second;
    auto waiter = std::make_shared<Waiter>(Waiter{task.tid, task.pid, {&memory, address, private_futex},
                                                timeout, lock, false, {}, {}, false});
    auto cleanup = wait_cleanup(state, waiter);
    std::function<std::optional<int64_t>()> clock_poll;
    if (timeout && timeout->realtime) {
        // Only the absolute clock is sampled. Lock acquisition is exclusively
        // driven by unlock/owner-exit handoff, never by this callback.
        clock_poll = [timeout]() -> std::optional<int64_t> {
            return expired(timeout) ? std::optional<int64_t>(-110) : std::nullopt;
        };
    }
    const auto deadline = monotonic_deadline(timeout);
    lock->waiters.push_back(waiter);
    try {
        state->locks.emplace(key, lock);
        state->waiting.emplace(task.tid, waiter);
    } catch (...) {
        waiter->queued = true;
        detach(*state, waiter);
        throw;
    }
    waiter->queued = true;
    scheduler.block(deadline, -110, std::move(clock_poll));
    task.wait_cleanup = std::move(cleanup);
    return 0;
} catch (const std::bad_alloc &) {
    guest_error(12);
}
int64_t futex_wait_requeue_pi(Scheduler &scheduler, Memory &memory, uint64_t source,
                              uint32_t expected, uint64_t target, bool private_futex,
                              std::optional<FutexPiTimeout> timeout,
                              const std::function<bool()> &interrupted) try {
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    if (source == target || (source & 3))
        guest_error(22);
    require_word(memory, target);
    if (!memory.accessible(source, 4, read_perm))
        guest_error(14);
    const Key from = key_of(scheduler.futex_address(memory, source, private_futex));
    const Key to = key_of(scheduler.futex_address(memory, target, private_futex));
    if (from == to)
        guest_error(22);
    futex_pi_check_non_pi(scheduler, {from.second, from.first});
    if (uint32_t(memory.load(source, 4)) != expected)
        guest_error(11);
    if (expired(timeout))
        guest_error(110);
    auto state = state_for(scheduler);
    auto &task = scheduler.current();
    if (!task.tid || task.tid > futex_tid_mask || task.state != Task::State::Runnable ||
        task.wait_cleanup || state->waiting.count(task.tid))
        guest_error(22);
    if (task.signals.restart) {
        task.signals.restart->restartable = true;
        task.signals.restart->always_restart = true;
    }
    if (interrupted && interrupted())
        guest_error(4);
    auto waiter = std::make_shared<Waiter>(Waiter{task.tid, task.pid,
                                                {&memory, target, private_futex}, timeout, {}, false, {}, {}, false});
    waiter->source = from;
    waiter->target = to;
    waiter->requeue_pi = true;
    auto cleanup = wait_cleanup(state, waiter);
    std::function<std::optional<int64_t>()> clock_poll;
    if (timeout && timeout->realtime)
        clock_poll = [timeout]() -> std::optional<int64_t> {
            return expired(timeout) ? std::optional<int64_t>(-110) : std::nullopt;
        };
    const auto deadline = monotonic_deadline(timeout);
    state->waiting.emplace(task.tid, waiter);
    waiter->queued = true;
    scheduler.wait_futex(from.second, UINT32_MAX, deadline, from.first);
    task.futex_requeue_pi = true;
    task.poll = std::move(clock_poll);
    task.wait_cleanup = std::move(cleanup);
    return 0;
} catch (const std::bad_alloc &) {
    guest_error(12);
}
int64_t futex_cmp_requeue_pi(Scheduler &scheduler, Memory &memory, uint64_t source,
                             uint64_t target, bool private_futex, uint32_t wake_count,
                             uint32_t requeue_count, uint32_t expected) try {
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    if (source == target || (source & 3) || wake_count != 1 || int32_t(requeue_count) < 0)
        guest_error(22);
    require_word(memory, target);
    if (!memory.accessible(source, 4, read_perm))
        guest_error(14);
    const Key from = key_of(scheduler.futex_address(memory, source, private_futex));
    const Key to = key_of(scheduler.futex_address(memory, target, private_futex));
    if (from == to)
        guest_error(22);
    if (uint32_t(memory.load(source, 4)) != expected)
        guest_error(11);
    futex_pi_check_non_pi(scheduler, {from.second, from.first});
    // Validate a target read even when the source queue is empty, as Linux does.
    (void)memory.load(target, 4);
    auto state = state_for(scheduler);
    uint64_t moved = 0;
    while (moved < uint64_t(requeue_count) + 1) {
        const auto tid = scheduler.first_futex_waiter({from.second, from.first});
        if (!tid)
            break;
        auto *task = scheduler.lookup(tid);
        auto found = state->waiting.find(tid);
        // Mixed operations and wrong target keys are errors at the offending
        // entry. Earlier successful transfers remain committed, as on Linux.
        if (!task->futex_requeue_pi || found == state->waiting.end() ||
            !found->second->source || *found->second->source != from || found->second->target != to)
            guest_error(22);
        auto waiter = found->second;
        if (expired(waiter->timeout)) {
            finish_error(*state, waiter, 110);
            continue;
        }
        const uint32_t value = uint32_t(memory.load(target, 4));
        const uint32_t owner = value & futex_tid_mask;
        auto current_lock = state->locks.find(to);
        if (owner == tid)
            guest_error(35);
        if (ordinary_waiters(scheduler, to) ||
            (current_lock != state->locks.end() && current_lock->second->owner != owner))
            guest_error(22);
        if (!owner) {
            if (current_lock != state->locks.end())
                guest_error(22);
            memory.store(target, tid | (value & futex_owner_died) |
                                     (requeue_count ? futex_waiters : 0), 4);
            // Never report a successful wait without committing its ownership.
            detach(*state, waiter);
            scheduler.wake(tid, 0);
        } else {
            memory.store(target, value | futex_waiters, 4);
            const auto *owner_task = scheduler.lookup(owner);
            if (!owner_task || owner_task->state == Task::State::Exited)
                guest_error(3);
            if (cycle(*state, owner, tid))
                guest_error(35);
            auto lock = current_lock == state->locks.end()
                            ? std::make_shared<PiLock>(PiLock{to, owner, {}}) : current_lock->second;
            lock->waiters.push_back(waiter);
            try {
                state->locks.emplace(to, lock);
            } catch (...) {
                lock->waiters.pop_back();
                throw;
            }
            waiter->lock = lock;
            waiter->source.reset();
            task->futex = task->futex_space = 0;
            task->futex_requeue_pi = false;
            if (task->signals.restart)
                task->signals.restart->always_restart = false;
        }
        ++moved;
    }
    return int64_t(moved);
} catch (const std::bad_alloc &) {
    guest_error(12);
}
int64_t futex_pi_interrupt(Scheduler &scheduler, Task &task, int64_t result) {
    if (result != -4)
        return result;
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    auto state = registry().states.find(&scheduler);
    if (state == registry().states.end())
        return result;
    auto found = state->second->waiting.find(task.tid);
    if (found == state->second->waiting.end() || !found->second->requeue_pi)
        return result;
    const bool before_requeue = bool(found->second->source);
    if (task.signals.restart) {
        task.signals.restart->always_restart = before_requeue;
        task.signals.restart->restartable = before_requeue;
        if (!before_requeue)
            task.signals.restart->interrupted = false;
    }
    return before_requeue ? -4 : -11;
}
int64_t futex_pi_unlock(Scheduler &scheduler, Memory &memory, uint64_t address, bool private_futex) try {
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    require_word(memory, address);
    const uint32_t value = uint32_t(memory.load(address, 4));
    if ((value & futex_tid_mask) != scheduler.current_tid())
        guest_error(1);
    const Key key = key_of(scheduler.futex_address(memory, address, private_futex));
    if (ordinary_waiters(scheduler, key))
        guest_error(22);
    auto state_it = registry().states.find(&scheduler);
    if (state_it != registry().states.end()) {
        const auto state = state_it->second;
        auto it = state->locks.find(key);
        if (it != state->locks.end()) {
            const auto lock = it->second;
            if (lock->owner != scheduler.current_tid())
                guest_error(22);
            handoff(*state, lock, &memory, address, false);
            return 0;
        }
    }
    memory.store(address, 0, 4);
    return 0;
} catch (const std::bad_alloc &) {
    guest_error(12);
}
void futex_pi_check_non_pi(const Scheduler &scheduler, Scheduler::FutexAddress key) {
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    auto it = registry().states.find(&scheduler);
    if (it != registry().states.end() && it->second->locks.count(key_of(key)))
        guest_error(22);
}
void futex_pi_owner_died(Scheduler &scheduler, Memory &memory, uint64_t address, uint32_t tid) {
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    require_word(memory, address);
    const uint32_t value = uint32_t(memory.load(address, 4));
    if ((value & futex_tid_mask) != tid)
        return;
    const auto shared_key = key_of(scheduler.futex_address(memory, address));
    const auto private_key = key_of(scheduler.futex_address(memory, address, true));
    auto state_it = registry().states.find(&scheduler);
    if (state_it != registry().states.end()) {
        auto state = state_it->second;
        for (auto key : {shared_key, private_key}) {
            auto it = state->locks.find(key);
            if (it != state->locks.end()) {
                auto lock = it->second;
                if (lock->owner != tid)
                    guest_error(22);
                handoff(*state, lock, &memory, address, true);
                return;
            }
        }
    }
    memory.store(address, (value & futex_waiters) | futex_owner_died, 4);
}
void futex_pi_task_exit(Scheduler &scheduler, Memory &memory, Task &task) {
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    auto it = registry().states.find(&scheduler);
    if (it == registry().states.end())
        return;
    const auto state = it->second;
    auto waiting = state->waiting.find(task.tid);
    if (waiting != state->waiting.end()) {
        auto waiter = waiting->second;
        detach(*state, waiter);
    }
    for (auto owned = state->locks.begin(); owned != state->locks.end();) {
        const auto lock = owned->second;
        // Only this lock's queue is touched by handoff/cleanup. Advance before
        // it can erase the current map entry, without allocating an exit list.
        ++owned;
        if (lock->owner != task.tid)
            continue;
        try {
            handoff(*state, lock, nullptr, 0, true);
        } catch (const MemoryFault &) {
            fail_lock(*state, lock, 14);
        } catch (const GuestError &error) {
            fail_lock(*state, lock, error.number);
        } catch (const std::bad_alloc &) {
            fail_lock(*state, lock, 12);
        }
    }
}
void futex_pi_set_priority_source(Scheduler &scheduler, std::function<int(uint32_t)> source) {
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    state_for(scheduler)->priority = std::move(source);
}
int futex_pi_effective_priority(const Scheduler &scheduler, uint32_t tid) {
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    auto it = registry().states.find(&scheduler);
    return it == registry().states.end() ? 120 : priority_of(*it->second, tid);
}
void futex_pi_vm_exit(Scheduler &scheduler) noexcept {
    std::lock_guard<std::recursive_mutex> guard(registry().mutex);
    auto it = registry().states.find(&scheduler);
    if (it == registry().states.end())
        return;
    for (auto &[tid, waiter] : it->second->waiting) {
        (void)tid;
        waiter->queued = false;
    }
    registry().states.erase(it);
}
} // namespace operit::arm64::threads
