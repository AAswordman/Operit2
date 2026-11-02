#include "runtime/tasks/scheduler.h"
#include "linux/events/inotify.h"
#include "linux/network/message_batch.h"
#include "linux/threads/futex_pi.h"
#include "runtime/process/controller.h"
#include <algorithm>
#include <thread>

namespace operit::arm64 {
namespace {
bool can_run(const Task &task) noexcept {
    return task.state == Task::State::Runnable && !task.stopped && !task.vfork_child;
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
    task.futex = 0;
    task.futex_space = 0;
    task.deadline.reset();
    task.poll = {};
}
void Scheduler::retire(Task &task) {
    ScopedTask context(*this, task.tid);
    if (task.wait_cleanup) {
        auto cleanup = std::move(task.wait_cleanup);
        cleanup();
    }
    task.state = Task::State::Exited;
    task.poll = {};
    task.futex = 0;
    task.futex_space = 0;
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
std::vector<uint32_t> Scheduler::runnable_tids(size_t maximum) {
    if (!maximum)
        return {};
    std::vector<std::pair<int, uint32_t>> candidates;
    candidates.reserve(tasks_.size());
    for (const auto &[tid, task] : tasks_)
        if (can_run(task))
            candidates.emplace_back(effective_priority(tid), tid);
    std::sort(candidates.begin(), candidates.end(), [this](const auto &left, const auto &right) {
        if (left.first != right.first)
            return left.first < right.first;
        return before_in_rotation(left.second, right.second, priority_cursor(left.first, current_));
    });
    std::vector<uint32_t> result;
    result.reserve(std::min(maximum, candidates.size()));
    // TLS bindings identify syscall/signal callers, not the batch cursor.
    for (const auto &[priority, tid] : candidates) {
        if (result.size() == maximum)
            break;
        result.push_back(tid);
        priority_cursors_[priority] = tid;
    }
    if (!result.empty())
        current_ = result.back();
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
bool Scheduler::has_runnable() const {
    for (const auto &[tid, task] : tasks_) {
        (void)tid;
        if (can_run(task))
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
    }
    return {address, static_cast<uint64_t>(reinterpret_cast<uintptr_t>(&memory))};
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
    Task task;
    task.tid = next_tid_;
    task.pid = process ? next_tid_ : current().pid;
    task.start_boottime_nanoseconds = process_boottime_nanoseconds();
    task.cpu = cpu;
    task.cpu.x[0] = 0;
    task.cpu.sp = stack;
    task.cpu.tls = tls;
    task.cpu.exclusive_valid = false;
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
    task.state = Task::State::Waiting;
    task.deadline = deadline;
    task.timeout_result = timeout_result;
    task.poll = std::move(poll);
    task.futex = 0;
    task.futex_space = 0;
}
void Scheduler::wait_futex(uint64_t address, uint32_t mask, TaskDeadline deadline, uint64_t space) {
    block(deadline, -110);
    current().futex = address;
    current().futex_space = space ? space : current().pid;
    current().futex_mask = mask;
}
unsigned Scheduler::wake_futex(uint64_t address, unsigned count, uint32_t mask, uint64_t space) {
    unsigned woken = 0;
    space = space ? space : current().pid;
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (woken == count)
            break;
        if (task.state == Task::State::Waiting && task.futex == address && task.futex_space == space &&
            (task.futex_mask & mask)) {
            complete(task, 0);
            ++woken;
        }
    }
    return woken;
}
unsigned Scheduler::requeue_futex(uint64_t from, uint64_t to, unsigned count, uint64_t space,
                                  uint64_t target_space) {
    unsigned moved = 0;
    space = space ? space : current().pid;
    target_space = target_space ? target_space : current().pid;
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (moved == count)
            break;
        if (task.state == Task::State::Waiting && task.futex == from && task.futex_space == space) {
            task.futex = to;
            task.futex_space = target_space;
            ++moved;
        }
    }
    return moved;
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
        if (value != -4 && task->signals.restart)
            task->signals.restart->interrupted = false;
    }
    if (task->state == Task::State::Waiting)
        complete(*task, value);
}
void Scheduler::stop_process(uint32_t pid) {
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.pid == pid && task.state != Task::State::Exited)
            task.stopped = true;
    }
}
void Scheduler::continue_process(uint32_t pid) {
    for (auto &[tid, task] : tasks_) {
        (void)tid;
        if (task.pid == pid)
            task.stopped = false;
    }
}
void Scheduler::exit(int code, bool group) {
    const auto pid = current().pid;
    retire(current());
    if (group)
        for (auto &[tid, task] : tasks_) {
            (void)tid;
            if (task.pid == pid && task.state != Task::State::Exited)
                retire(task);
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
    survivor.deadline.reset();
    survivor.poll = {};
    survivor.wait_cleanup = {};
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
        if (task.state != Task::State::Waiting)
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
        uint32_t selected = 0;
        int best_priority = 0;
        for (const auto &[tid, task] : tasks_) {
            if (!can_run(task))
                continue;
            const int priority = effective_priority(tid);
            if (!selected || priority < best_priority ||
                (priority == best_priority &&
                 before_in_rotation(tid, selected, priority_cursor(priority, current_tid())))) {
                selected = tid;
                best_priority = priority;
            }
        }
        if (selected) {
            if (current_tid() != selected)
                current().cpu.exclusive_valid = false;
            priority_cursors_[best_priority] = selected;
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
