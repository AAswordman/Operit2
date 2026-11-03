#include "runtime/tasks/scheduler.h"
#include "linux/events/inotify.h"
#include "linux/io_uring/io_uring.h"
#include "linux/network/message_batch.h"
#include "linux/threads/futex_pi.h"
#include "runtime/process/controller.h"
#include <algorithm>
#include <array>
#include <thread>

namespace operit::arm64 {
namespace {
bool can_run(const Task &task) noexcept {
    return task.state == Task::State::Runnable && !task.stopped && !task.ptrace_stopped &&
           !task.ptrace_deferred_exit && !task.vfork_child;
}
bool before_in_rotation(uint32_t left, uint32_t right, uint32_t cursor) noexcept {
    if ((left > cursor) != (right > cursor))
        return left > cursor;
    return left < right;
}
} // namespace
thread_local Scheduler::ScopedTask *Scheduler::task_scope_ = nullptr;

Scheduler::ScopedTask::ScopedTask(Scheduler &scheduler, uint32_t tid)
    : scheduler_(&scheduler), tid_(tid ? tid : scheduler.current_tid()), previous_(task_scope_) {
    if (!scheduler.lookup(tid_))
        guest_error(3);
    task_scope_ = this;
}
Scheduler::ScopedTask::~ScopedTask() noexcept {
    task_scope_ = previous_;
}
Scheduler::ScopedTask *Scheduler::bound_task() const noexcept {
    for (auto *scope = task_scope_; scope; scope = scope->previous_)
        if (scope->scheduler_ == this)
            return scope;
    return nullptr;
}
bool Scheduler::references_task(uint32_t tid) const noexcept {
    if (current_ == tid)
        return true;
    for (auto *scope = task_scope_; scope; scope = scope->previous_)
        if (scope->scheduler_ == this && scope->tid_ == tid)
            return true;
    return false;
}
uint32_t Scheduler::current_tid() const noexcept {
    auto *scope = bound_task();
    return scope ? scope->tid_ : current_;
}
void Scheduler::set_current_tid(uint32_t tid) noexcept {
    if (auto *scope = bound_task())
        scope->tid_ = tid;
    else
        current_ = tid;
}
Scheduler::Scheduler() {
    tasks_.try_emplace(1).first->second.start_boottime_nanoseconds = process_boottime_nanoseconds();
    threads::futex_pi_set_priority_source(*this, [this](uint32_t tid) {
        const auto *task = lookup(tid);
        return task ? task->base_priority() : 120;
    });
}
Scheduler::~Scheduler() {
    // Machine's address spaces may already have gone away. These registry
    // removals must not execute callbacks capturing their old Syscalls objects.
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        socket_messages::forget_message_wait(task);
    }
    threads::futex_pi_vm_exit(*this);
}
void Scheduler::complete(Task &task, int64_t value) {
    ScopedTask context(*this, task.tid);
    if (task.wait_cleanup) {
        auto cleanup = std::move(task.wait_cleanup);
        cleanup();
    }
    task.cpu.x[0] = uint64_t(value);
    task.state = Task::State::Runnable;
    // Real wakeup starts a new continuous-RT interval; yield/preemption do not.
    task.realtime_runtime_nanoseconds = 0;
    refresh_realtime(task);
    task.futex = 0;
    task.futex_space = 0;
    task.futex_requeue_pi = false;
    task.futex_vector.clear();
    task.futex_vector_result = -1;
    task.deadline.reset();
    task.poll = {};
    if (processes_)
        processes_->wait_completed(task.tid);
}
void Scheduler::retire(Task &task) {
    ScopedTask context(*this, task.tid);
    if (task.wait_cleanup) {
        auto cleanup = std::move(task.wait_cleanup);
        cleanup();
    }
    dequeue_realtime(task);
    task.state = Task::State::Exited;
    task.poll = {};
    task.futex = 0;
    task.futex_space = 0;
    task.futex_requeue_pi = false;
    task.futex_vector.clear();
    task.futex_vector_result = -1;
    task.deadline.reset();
    task.cpu.exclusive_valid = false;
    // A thread disappears at exit; a group leader remains charged as a zombie
    // until its process is reaped, even if peer threads are still running.
    if (task.tid != task.pid)
        task.task_limit_charge.reset();
}
std::vector<uint32_t> Scheduler::task_ids(uint32_t pid) const {
    std::vector<uint32_t> result;
    for (const auto &[tid, task] : tasks_)
        if (!pid || task.pid == pid)
            result.push_back(tid);
    return result;
}
std::vector<uint32_t> Scheduler::live_tids(uint32_t pid) const {
    std::vector<uint32_t> result;
    for (const auto &[tid, task] : tasks_)
        if (task.state != Task::State::Exited && (!pid || task.pid == pid))
            result.push_back(tid);
    return result;
}
uint32_t Scheduler::priority_cursor(int priority, uint32_t fallback) const noexcept {
    const auto it = priority_cursors_.find(priority);
    return it == priority_cursors_.end() ? fallback : it->second;
}
int Scheduler::effective_priority(uint32_t tid) const {
    const auto *task = lookup(tid ? tid : current_tid());
    if (!task || task->state == Task::State::Exited)
        guest_error(3);
    return threads::futex_pi_effective_priority(*this, task->tid);
}
uint64_t Scheduler::eligible_cpus(const Task &task) const noexcept {
    const uint64_t available = execution_cpus_ >= 64 ? UINT64_MAX : (uint64_t{1} << execution_cpus_) - 1;
    return task.cpu_affinity & available;
}
void Scheduler::dequeue_realtime(Task &task) {
    if (task.realtime_queue_priority >= 100)
        return;
    auto found = realtime_queues_.find(task.realtime_queue_priority);
    if (found != realtime_queues_.end()) {
        found->second.remove(task.tid);
        if (found->second.empty())
            realtime_queues_.erase(found);
    }
    task.realtime_queue_priority = 100;
}
void Scheduler::refresh_realtime(Task &task) {
    refresh_realtime(task, can_run(task) && eligible_cpus(task) ? effective_priority(task.tid) : 100);
}
void Scheduler::refresh_realtime(Task &task, int priority) {
    // A PI owner leaving the RT scheduling class loses its watchdog timeout.
    // Requested FIFO/RR policy changes retain it until a blocking wakeup.
    if (task.realtime_watchdog_pi_boosted && priority >= 100 && can_run(task) && eligible_cpus(task))
        task.realtime_runtime_nanoseconds = 0;
    task.realtime_watchdog_pi_boosted = priority < 100 && task.base_priority() >= 100;
    if (priority >= 100) {
        dequeue_realtime(task);
        return;
    }
    const int previous = task.realtime_queue_priority;
    if (priority == previous)
        return;
    // Allocate the new entry before removing the old one. A priority decrease
    // (including PI deboost) joins the head; an increase or wake joins the tail.
    auto &queue = realtime_queues_[priority];
    if (previous < 100 && priority > previous)
        queue.push_front(task.tid);
    else
        queue.push_back(task.tid);
    dequeue_realtime(task);
    task.realtime_queue_priority = priority;
}
void Scheduler::requeue_realtime_tail(Task &task) {
    const auto found = realtime_queues_.find(task.realtime_queue_priority);
    if (found == realtime_queues_.end())
        return;
    auto &queue = found->second;
    const auto entry = std::find(queue.begin(), queue.end(), task.tid);
    if (entry != queue.end())
        queue.splice(queue.end(), queue, entry);
}
void Scheduler::scheduling_changed(uint32_t tid) {
    // Linux retains a task's unused RR slice across policy/priority changes.
    // Only a newly created task starts with a fresh slice.
    refresh_realtime(find(tid));
}
void Scheduler::account_runtime(uint32_t tid, uint64_t nanoseconds, bool realtime_limit_active) {
    auto *task = lookup(tid);
    if (!task || task->state == Task::State::Exited || !nanoseconds)
        return;
    // Like the Linux RT watchdog, start counting only while a finite limit is
    // installed. Ordinary PI owners temporarily executing in RT count too.
    if (realtime_limit_active && effective_priority(tid) < 100) {
        auto &runtime = task->realtime_runtime_nanoseconds;
        runtime = nanoseconds > UINT64_MAX - runtime ? UINT64_MAX : runtime + nanoseconds;
    }
    if (task->sched_policy != Task::SchedulingPolicy::RoundRobin)
        return;
    const auto remaining = task->rr_remaining_nanoseconds;
    if (nanoseconds < remaining) {
        task->rr_remaining_nanoseconds -= nanoseconds;
        return;
    }
    task->rr_remaining_nanoseconds = round_robin_interval_nanoseconds -
                                    (nanoseconds - remaining) % round_robin_interval_nanoseconds;
    refresh_realtime(*task);
    requeue_realtime_tail(*task);
}
void Scheduler::yield() {
    auto &task = current();
    refresh_realtime(task);
    if (task.realtime_queue_priority < 100)
        requeue_realtime_tail(task);
    else
        priority_cursors_[effective_priority(task.tid)] = task.tid;
    // RR keeps the unused CPU-time budget across a voluntary yield.
}
std::vector<std::pair<int, uint32_t>> Scheduler::ordered_candidates() {
    std::vector<std::pair<int, uint32_t>> ordinary;
    ordinary.reserve(tasks_.size());
    for (auto &[tid, task] : tasks_) {
        const bool runnable = can_run(task) && eligible_cpus(task);
        const int priority = runnable ? effective_priority(tid) : 100;
        refresh_realtime(task, priority);
        if (runnable && priority >= 100)
            ordinary.emplace_back(priority, tid);
    }
    std::sort(ordinary.begin(), ordinary.end(), [this](const auto &left, const auto &right) {
        if (left.first != right.first)
            return left.first < right.first;
        return before_in_rotation(left.second, right.second, priority_cursor(left.first, current_));
    });
    std::vector<std::pair<int, uint32_t>> result;
    result.reserve(tasks_.size());
    for (const auto &[priority, queue] : realtime_queues_)
        for (const auto tid : queue)
            result.emplace_back(priority, tid);
    result.insert(result.end(), ordinary.begin(), ordinary.end());
    return result;
}
std::vector<Scheduler::RunnableTask> Scheduler::runnable_batch(size_t maximum, size_t minimum) {
    if (!maximum)
        return {};
    maximum = std::min<size_t>(maximum, std::min<uint32_t>(execution_cpus_, 64));
    const auto candidates = ordered_candidates();
    std::array<int, 64> owners;
    owners.fill(-1);
    // Augmenting paths preserve all earlier (higher priority / FIFO) selections
    // while finding distinct CPUs. A pinned FIFO peer must not slip through as
    // a second sequential executor job on the already occupied virtual CPU.
    auto assign = [&](auto &self, size_t index, uint64_t &visited) -> bool {
        const auto &task = tasks_.at(candidates[index].second);
        uint64_t possible = eligible_cpus(task) & ~visited;
        while (possible) {
            const uint64_t preferred = task.last_cpu < 64 ? uint64_t{1} << task.last_cpu : 0;
            const auto bit = possible & preferred ? preferred : possible & (~possible + 1);
            possible &= ~bit;
            if (visited & bit)
                continue;
            visited |= bit;
            const auto cpu = uint32_t(__builtin_ctzll(bit));
            const int previous = owners[cpu];
            if (previous < 0 || self(self, size_t(previous), visited)) {
                owners[cpu] = int(index);
                return true;
            }
        }
        return false;
    };
    std::vector<size_t> chosen;
    chosen.reserve(std::min(maximum, candidates.size()));
    for (size_t index = 0; index < candidates.size() && chosen.size() < maximum; ++index) {
        uint64_t visited = 0;
        if (assign(assign, index, visited))
            chosen.push_back(index);
    }
    // Machine falls back to its already selected serial task if fewer than
    // two CPUs can run. Committing a singleton here would silently advance
    // ordinary-task rotation twice and starve one of two CPU-pinned peers.
    if (chosen.size() < minimum)
        return {};
    std::vector<RunnableTask> result;
    result.reserve(chosen.size());
    for (auto index : chosen) {
        for (uint32_t cpu = 0; cpu < 64; ++cpu)
            if (owners[cpu] == int(index)) {
                const auto [priority, tid] = candidates[index];
                result.push_back({tid, cpu});
                if (priority >= 100)
                    priority_cursors_[priority] = tid;
                break;
            }
    }
    if (!result.empty())
        current_ = result.back().tid;
    return result;
}
std::vector<uint32_t> Scheduler::runnable_tids(size_t maximum) {
    std::vector<uint32_t> result;
    for (const auto &task : runnable_batch(maximum))
        result.push_back(task.tid);
    return result;
}
bool Scheduler::process_alive(uint32_t pid) const {
    for (const auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.pid == pid && task.state != Task::State::Exited)
            return true;
    }
    return false;
}
Scheduler::FutexAddress Scheduler::futex_address(const Memory &memory, uint64_t address, bool private_futex) {
    if (!private_futex) {
        if (auto identity = memory.shared_identity(address)) {
            auto found = shared_futexes_.find(*identity);
            if (found == shared_futexes_.end())
                found = shared_futexes_.emplace(*identity, next_futex_key_++).first;
            return {found->second, UINT64_MAX};
        }
        // Linux also distinguishes a shared futex in a private anonymous mm
        // from a FUTEX_PRIVATE key at the same virtual address.
        const auto identity = std::make_pair(uint64_t(reinterpret_cast<uintptr_t>(&memory)), address);
        auto found = shared_mm_futexes_.find(identity);
        if (found == shared_mm_futexes_.end())
            found = shared_mm_futexes_.emplace(identity, next_futex_key_++).first;
        return {found->second, UINT64_MAX};
    }
    return {address, static_cast<uint64_t>(reinterpret_cast<uintptr_t>(&memory))};
}
Scheduler::ScopedFutexVectorVisibility::ScopedFutexVectorVisibility(Scheduler &scheduler, FutexAddress key)
    : scheduler_(scheduler), key_(key) {
    for (auto &[tid, task] : scheduler_.tasks_) {
        (void)tid;
        if (task.state != Task::State::Waiting)
            continue;
        for (const auto &entry : task.futex_vector) {
            if (entry.queued && entry.address == key.address && entry.space == key.space) {
                task.futex = key.address;
                task.futex_space = key.space;
                break;
            }
        }
    }
}
Scheduler::ScopedFutexVectorVisibility::~ScopedFutexVectorVisibility() noexcept {
    for (auto &[tid, task] : scheduler_.tasks_) {
        (void)tid;
        if (!task.futex_vector.empty() && task.futex == key_.address && task.futex_space == key_.space) {
            task.futex = 0;
            task.futex_space = 0;
        }
    }
}
uint32_t Scheduler::spawn_task(const CpuState &cpu, uint64_t stack, uint64_t tls, uint64_t clear_tid,
                               bool process) {
    for (auto it = tasks_.begin(); it != tasks_.end();) {
        const auto &task = it->second;
        // Keep a zombie leader's UID charge until reap. Even an uncharged
        // leader retains the group's birth time while peer threads survive.
        const bool retained_leader = task.tid == task.pid &&
            (task.task_limit_charge || process_alive(task.pid));
        if (task.state == Task::State::Exited && !retained_leader && !references_task(it->first))
            it = tasks_.erase(it);
        else
            ++it;
    }
    if (tasks_.size() >= 1024 || next_tid_ >= INT32_MAX)
        guest_error(11);
    refresh_realtime(current());
    Task task;
    task.tid = next_tid_;
    task.pid = process ? next_tid_ : current().pid;
    task.start_boottime_nanoseconds = process_boottime_nanoseconds();
    task.cpu = cpu;
    // A traced syscall can retain its original number while x8 was edited.
    if (current().ptrace_user_x8)
        task.cpu.x[8] = *current().ptrace_user_x8;
    task.cpu.x[0] = 0;
    task.cpu.sp = stack;
    task.cpu.tls = tls;
    task.cpu.exclusive_valid = false;
    task.seccomp = current().seccomp;
    task.signal_mask = current().signal_mask;
    task.signals = signals::fork_thread(current().signals, true, false);
    task.name = current().name;
    task.cpu_affinity = current().cpu_affinity;
    task.sched_policy = current().sched_policy;
    task.rt_priority = current().rt_priority;
    task.nice = current().nice;
    if (current().sched_reset_on_fork) {
        if (task.sched_policy == Task::SchedulingPolicy::Fifo ||
            task.sched_policy == Task::SchedulingPolicy::RoundRobin) {
            task.sched_policy = Task::SchedulingPolicy::Other;
            task.rt_priority = 0;
        }
        task.nice = std::max(task.nice, int32_t{0});
    }
    task.clear_tid = clear_tid;
    tasks_.emplace(next_tid_, std::move(task));
    try {
        refresh_realtime(tasks_.at(next_tid_));
    } catch (...) {
        tasks_.erase(next_tid_);
        throw;
    }
    exited_ = false;
    return next_tid_++;
}
uint32_t Scheduler::spawn(const CpuState &cpu, uint64_t stack, uint64_t tls, uint64_t clear_tid) {
    return spawn_task(cpu, stack, tls, clear_tid, false);
}
uint32_t Scheduler::spawn_process(const CpuState &cpu, uint64_t stack, uint64_t tls, uint64_t clear_tid) {
    return spawn_task(cpu, stack, tls, clear_tid, true);
}
void Scheduler::discard(uint32_t tid) {
    auto it = tasks_.find(tid);
    if (it != tasks_.end() && tid != current_tid()) {
        retire(it->second);
        // Explicit removal covers failed creation and final process reap.
        // Discard observers must run before the owning process is destroyed.
        it->second.signals.pending.discard(UINT64_MAX);
        it->second.task_limit_charge.reset();
        if (!references_task(tid))
            tasks_.erase(it);
    }
}
void Scheduler::reap_process(uint32_t pid) {
    for (auto it = tasks_.begin(); it != tasks_.end();) {
        auto &task = it->second;
        if (task.pid != pid || task.state != Task::State::Exited) {
            ++it;
            continue;
        }
        // Run discard observers while the process and its timers still exist.
        task.signals.pending.discard(UINT64_MAX);
        task.task_limit_charge.reset();
        if (!references_task(it->first))
            it = tasks_.erase(it);
        else
            ++it;
    }
}
void Scheduler::block(TaskDeadline deadline, int64_t timeout_result,
                      std::function<std::optional<int64_t>()> poll) {
    auto &task = current();
    dequeue_realtime(task);
    task.state = Task::State::Waiting;
    task.deadline = deadline;
    task.timeout_result = timeout_result;
    task.poll = std::move(poll);
    task.futex = 0;
    task.futex_space = 0;
    task.futex_requeue_pi = false;
    task.futex_vector.clear();
    task.futex_vector_result = -1;
}
void Scheduler::wait_futex(uint64_t address, uint32_t mask, TaskDeadline deadline, uint64_t space) {
    block(deadline, -110);
    current().futex = address;
    current().futex_space = space ? space : current().pid;
    current().futex_mask = mask;
    // Linux's source futex plist orders realtime tasks by normal priority;
    // ordinary tasks share priority 100 and retain insertion order.
    current().futex_priority = std::min(current().base_priority(), 100);
    current().futex_sequence = next_futex_sequence_++;
}
void Scheduler::wait_futex_vector(std::vector<Task::FutexVectorEntry> entries, TaskDeadline deadline,
                                  std::function<std::optional<int64_t>()> clock_poll) {
    for (auto &entry : entries)
        entry.sequence = next_futex_sequence_++;
    block(deadline, -110, std::move(clock_poll));
    auto &task = current();
    task.futex_priority = std::min(task.base_priority(), 100);
    task.futex_vector = std::move(entries);
}
Scheduler::FutexMatch Scheduler::find_futex_waiter(FutexAddress key, uint32_t mask,
                                                   uint64_t before_sequence) const {
    FutexMatch first;
    int first_priority = 0;
    uint64_t first_sequence = 0;
    for (const auto &item : tasks_) {
        const auto tid = item.first;
        const auto &task = item.second;
        if (task.state != Task::State::Waiting)
            continue;
        const auto consider = [&](uint64_t address, uint64_t space, uint32_t entry_mask,
                                  uint64_t sequence, size_t index) {
            if (address != key.address || space != key.space || !(entry_mask & mask) ||
                sequence >= before_sequence)
                return;
            if (!first.tid || std::make_pair(task.futex_priority, sequence) <
                                  std::make_pair(first_priority, first_sequence)) {
                first = {tid, index};
                first_priority = task.futex_priority;
                first_sequence = sequence;
            }
        };
        if (task.futex_vector.empty()) {
            consider(task.futex, task.futex_space, task.futex_mask, task.futex_sequence, SIZE_MAX);
        } else {
            for (size_t index = 0; index < task.futex_vector.size(); ++index) {
                const auto &entry = task.futex_vector[index];
                if (entry.queued)
                    consider(entry.address, entry.space, entry.mask, entry.sequence, index);
            }
        }
    }
    return first;
}
uint32_t Scheduler::first_futex_waiter(FutexAddress key, uint32_t mask,
                                        uint64_t before_sequence) const {
    return find_futex_waiter(key, mask, before_sequence).tid;
}
void Scheduler::complete_futex_vectors() {
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.futex_vector_result >= 0)
            complete(task, task.futex_vector_result);
    }
}
unsigned Scheduler::wake_futex_deferred(FutexAddress key, unsigned count, uint32_t mask) {
    unsigned woken = 0;
    while (woken < count) {
        const auto match = find_futex_waiter(key, mask, UINT64_MAX);
        if (!match.tid)
            break;
        auto &task = tasks_.at(match.tid);
        if (task.futex_requeue_pi) {
            complete_futex_vectors();
            guest_error(22);
        }
        if (match.vector_index == SIZE_MAX) {
            complete(task, 0);
        } else {
            task.futex_vector[match.vector_index].queued = false;
            task.futex_vector_result = int(match.vector_index);
        }
        ++woken;
    }
    return woken;
}
unsigned Scheduler::wake_futex(uint64_t address, unsigned count, uint32_t mask, uint64_t space) {
    const auto woken = wake_futex_deferred({address, space ? space : current().pid}, count, mask);
    complete_futex_vectors();
    return woken;
}
unsigned Scheduler::wake_futex_pair(FutexAddress first, unsigned first_count, FutexAddress second,
                                    unsigned second_count, bool wake_second) {
    unsigned woken = wake_futex_deferred(first, first_count, UINT32_MAX);
    if (wake_second)
        woken += wake_futex_deferred(second, second_count, UINT32_MAX);
    complete_futex_vectors();
    return woken;
}
unsigned Scheduler::requeue_futex(uint64_t from, uint64_t to, unsigned count, uint64_t space,
                                  uint64_t target_space) {
    unsigned moved = 0;
    space = space ? space : current().pid;
    target_space = target_space ? target_space : current().pid;
    const auto cutoff = next_futex_sequence_;
    while (moved < count) {
        const auto match = find_futex_waiter({from, space}, UINT32_MAX, cutoff);
        if (!match.tid)
            break;
        auto &task = tasks_.at(match.tid);
        if (task.futex_requeue_pi)
            guest_error(22);
        if (match.vector_index == SIZE_MAX) {
            task.futex = to;
            task.futex_space = target_space;
            task.futex_sequence = next_futex_sequence_++;
        } else {
            auto &entry = task.futex_vector[match.vector_index];
            entry.address = to;
            entry.space = target_space;
            entry.sequence = next_futex_sequence_++;
        }
        ++moved;
    }
    return moved;
}
unsigned Scheduler::wake_and_requeue_futex(FutexAddress from, FutexAddress to, unsigned wake_count,
                                           unsigned requeue_count) {
    const auto woken = wake_futex_deferred(from, wake_count, UINT32_MAX);
    try {
        const auto moved = requeue_futex(from.address, to.address, requeue_count, from.space, to.space);
        complete_futex_vectors();
        return woken + moved;
    } catch (...) {
        complete_futex_vectors();
        throw;
    }
}
void Scheduler::wake(uint32_t tid, int64_t value) {
    auto *task = lookup(tid);
    if (task && task->state == Task::State::Waiting)
        complete(*task, value);
}
void Scheduler::interrupt(uint32_t tid, int64_t value) {
    auto *task = lookup(tid);
    if (!task || task->state != Task::State::Waiting)
        return;
    ScopedTask context(*this, tid);
    if (value == -4) {
        if (auto partial = socket_messages::interrupt_message_wait(*task))
            value = *partial;
        if (value == -4)
            if (auto partial = io_uring::interrupt_wait(*task))
                value = *partial;
        if (value != -4 && task->signals.restart)
            task->signals.restart->interrupted = false;
    }
    value = threads::futex_pi_interrupt(*this, *task, value);
    if (task->state == Task::State::Waiting)
        complete(*task, value);
}
void Scheduler::stop_process(uint32_t pid) {
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.pid == pid && task.state != Task::State::Exited) {
            dequeue_realtime(task);
            task.stopped = true;
        }
    }
}
void Scheduler::continue_process(uint32_t pid) {
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.pid == pid) {
            if (task.stopped && task.state == Task::State::Runnable)
                task.realtime_runtime_nanoseconds = 0;
            task.stopped = false;
            if (task.state != Task::State::Exited)
                refresh_realtime(task);
        }
    }
}
void Scheduler::exit(int code, bool group) {
    const auto pid = current().pid;
    retire(current());
    if (group) {
        exit_process(pid, code);
        return;
    }
    exited_ = live_tids().empty();
    if (exited_)
        exit_code_ = code & 255;
}
void Scheduler::exit_process(uint32_t pid, int code) {
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.pid == pid && task.state != Task::State::Exited)
            retire(task);
    }
    exited_ = live_tids().empty();
    if (exited_)
        exit_code_ = code & 255;
}
void Scheduler::replace_process_image(uint32_t pid, const CpuState &entry) {
    const auto executing_tid = current_tid();
    const auto *leader = lookup(pid);
    const auto process_start = leader ? leader->start_boottime_nanoseconds
                                      : current().start_boottime_nanoseconds;
    for (auto &[tid, task] : tasks_)
        if (task.pid == pid && tid != executing_tid && task.state != Task::State::Exited)
            retire(task);
    // Rekey the existing node without copying its task-limit charge or
    // allocating a replacement node after the old task has been removed.
    auto survivor_node = tasks_.extract(executing_tid);
    tasks_.erase(pid);
    auto &survivor = survivor_node.mapped();
    survivor.tid = survivor.pid = pid;
    survivor.start_boottime_nanoseconds = process_start;
    survivor.cpu = entry;
    survivor.cpu.exclusive_valid = false;
    survivor.clear_tid = survivor.robust_list = 0;
    survivor.state = Task::State::Runnable;
    survivor.stopped = false;
    survivor.futex = survivor.futex_space = 0;
    survivor.futex_requeue_pi = false;
    survivor.futex_vector.clear();
    survivor.futex_vector_result = -1;
    survivor.deadline.reset();
    survivor.poll = {};
    survivor.wait_cleanup = {};
    if (survivor.realtime_queue_priority < 100) {
        auto &queue = realtime_queues_.at(survivor.realtime_queue_priority);
        for (auto &tid : queue)
            if (tid == executing_tid) {
                tid = pid;
                break;
            }
    }
    survivor_node.key() = pid;
    tasks_.insert(std::move(survivor_node));
    // exec can change a non-leader TID into the process leader. Keep outer
    // contexts valid when the syscall's inner scope unwinds.
    if (current_ == executing_tid)
        current_ = pid;
    for (auto *scope = task_scope_; scope; scope = scope->previous_)
        if (scope->scheduler_ == this && scope->tid_ == executing_tid)
            scope->tid_ = pid;
    set_current_tid(pid);
    exited_ = false;
}
TaskClock::time_point Scheduler::poll_waiting() {
    // Workers have stopped here. Pump readiness before polling guest waits,
    // including each select() iteration when every guest task is blocked.
    if (processes_)
        processes_->poll_runtime_events();
    else
        InotifyFile::service_all();
    const auto now = TaskClock::now();
    auto next_poll = now + std::chrono::milliseconds(1);
    for (auto &[tid, task] : tasks_) {
        if (task.state != Task::State::Waiting || task.ptrace_deferred_exit)
            continue;
        ScopedTask context(*this, tid);
        if (task.poll) {
            // Signal delivery may complete this wait from inside its own callback.
            auto poll = std::move(task.poll);
            std::optional<int64_t> result;
            try {
                result = poll();
            } catch (const GuestError &error) {
                result = -error.number;
            } catch (const MemoryFault &) {
                result = -14;
            }
            if (task.state == Task::State::Exited)
                continue;
            if (result) {
                complete(task, *result);
                continue;
            }
            if (task.state != Task::State::Waiting)
                continue;
            if (!task.poll)
                task.poll = std::move(poll);
        }
        if (task.deadline) {
            if (now >= *task.deadline)
                complete(task, task.timeout_result);
            else
                next_poll = std::min(next_poll, *task.deadline);
        }
    }
    if (processes_)
        if (const auto deadline = processes_->next_runtime_deadline())
            next_poll = std::min(next_poll, *deadline);
    return next_poll;
}
bool Scheduler::select(bool blocking) {
    if (exited_)
        return false;
    for (;;) {
        const auto next_poll = poll_waiting();
        const auto candidates = ordered_candidates();
        if (!candidates.empty()) {
            const auto [priority, selected] = candidates.front();
            if (current_tid() != selected)
                current().cpu.exclusive_valid = false;
            if (priority >= 100)
                priority_cursors_[priority] = selected;
            set_current_tid(selected);
            return true;
        }
        if (live_tids().empty()) {
            exited_ = true;
            return false;
        }
        if (!blocking)
            return false;
        std::this_thread::sleep_until(next_poll);
    }
}
} // namespace operit::arm64
