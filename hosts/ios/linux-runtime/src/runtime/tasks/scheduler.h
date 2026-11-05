#pragma once
#include "runtime/tasks/task.h"
#include <cstddef>
#include <map>
#include <list>
#include <vector>

namespace operit::arm64 {
class ProcessController;
// Guest tasks are scheduled at syscall/instruction-quantum boundaries. Each task
// carries a PID; Machine chooses that process's address space before execution.
class Scheduler {
  public:
    class ScopedTask;

  private:
    std::map<uint32_t, Task> tasks_;
    uint32_t current_ = 1, next_tid_ = 2;
    uint32_t execution_cpus_ = 1;
    std::map<int, uint32_t> priority_cursors_;
    std::map<int, std::list<uint32_t>> realtime_queues_;
    void dequeue_realtime(Task &task);
    void refresh_realtime(Task &task);
    void refresh_realtime(Task &task, int priority);
    void requeue_realtime_tail(Task &task);
    std::vector<std::pair<int, uint32_t>> ordered_candidates();
    uint64_t eligible_cpus(const Task &task) const noexcept;
    uint32_t priority_cursor(int priority, uint32_t fallback) const noexcept;
    static thread_local ScopedTask *task_scope_;
    ScopedTask *bound_task() const noexcept;
    bool references_task(uint32_t tid) const noexcept;
    void set_current_tid(uint32_t tid) noexcept;
    bool exited_ = false;
    int exit_code_ = 0;
    ProcessController *processes_ = nullptr;
    std::map<Memory::SharedIdentity, uint64_t> shared_futexes_;
    std::map<std::pair<uint64_t, uint64_t>, uint64_t> shared_mm_futexes_;
    uint64_t next_futex_key_ = address_limit;
    uint64_t next_futex_sequence_ = 1;
    void complete(Task &task, int64_t value);
    void retire(Task &task);
    uint32_t spawn_task(const CpuState &cpu, uint64_t stack, uint64_t tls, uint64_t clear_tid, bool process);

  public:
    // Binding only changes this host thread's current task. Workers must finish
    // their scopes before the coordinator changes the task map or wait state.
    class ScopedTask final {
        friend class Scheduler;
        Scheduler *scheduler_;
        mutable uint32_t tid_;
        ScopedTask *previous_;

      public:
        ScopedTask(Scheduler &scheduler, uint32_t tid);
        ~ScopedTask() noexcept;
        ScopedTask(const ScopedTask &) = delete;
        ScopedTask &operator=(const ScopedTask &) = delete;
        ScopedTask(ScopedTask &&) = delete;
        ScopedTask &operator=(ScopedTask &&) = delete;
    };
    Scheduler();
    ~Scheduler();
    Scheduler(const Scheduler &) = delete;
    Scheduler &operator=(const Scheduler &) = delete;
    Scheduler(Scheduler &&) = delete;
    Scheduler &operator=(Scheduler &&) = delete;
    uint32_t current_tid() const noexcept;
    void set_execution_cpus(uint32_t cpus) noexcept {
        execution_cpus_ = cpus ? cpus : 1;
    }
    uint32_t execution_cpus() const noexcept {
        return execution_cpus_;
    }
    Task &current() {
        return tasks_.at(current_tid());
    }
    const Task &current() const {
        return tasks_.at(current_tid());
    }
    Task &find(uint32_t tid) {
        auto it = tasks_.find(tid ? tid : current_tid());
        if (it == tasks_.end() || it->second.state == Task::State::Exited)
            guest_error(3);
        return it->second;
    }
    Task *lookup(uint32_t tid) {
        auto it = tasks_.find(tid);
        return it == tasks_.end() ? nullptr : &it->second;
    }
    const Task *lookup(uint32_t tid) const {
        auto it = tasks_.find(tid);
        return it == tasks_.end() ? nullptr : &it->second;
    }
    // Includes retained Exited tasks whose pending queues still need cleanup.
    // A zero pid selects every retained task, as with live_tids().
    std::vector<uint32_t> task_ids(uint32_t pid = 0) const;
    std::vector<uint32_t> live_tids(uint32_t pid = 0) const;
    int effective_priority(uint32_t tid) const;
    struct RunnableTask { uint32_t tid, cpu; };
    // Coordinator only: FIFO order for RT priorities, CPU-time slices for RR,
    // and the existing per-priority rotation for ordinary guest tasks.
    // A rejected batch must not consume rotation or replace the serial selection.
    std::vector<RunnableTask> runnable_batch(size_t maximum = SIZE_MAX, size_t minimum = 1);
    std::vector<uint32_t> runnable_tids(size_t maximum = SIZE_MAX);
    void scheduling_changed(uint32_t tid);
    void account_runtime(uint32_t tid, uint64_t nanoseconds, bool realtime_limit_active = false);
    void yield();
    bool process_alive(uint32_t pid) const;
    struct FutexAddress {
        uint64_t address, space;
    };
    // The PI implementation examines the legacy single-address fields. Expose
    // matching vector entries while it checks a PI target, without changing
    // which queues the scheduler itself searches.
    class ScopedFutexVectorVisibility final {
        Scheduler &scheduler_;
        FutexAddress key_;

      public:
        ScopedFutexVectorVisibility(Scheduler &scheduler, FutexAddress key);
        ~ScopedFutexVectorVisibility() noexcept;
        ScopedFutexVectorVisibility(const ScopedFutexVectorVisibility &) = delete;
        ScopedFutexVectorVisibility &operator=(const ScopedFutexVectorVisibility &) = delete;
    };

  private:
    struct FutexMatch {
        uint32_t tid = 0;
        size_t vector_index = SIZE_MAX;
    };
    FutexMatch find_futex_waiter(FutexAddress key, uint32_t mask, uint64_t before_sequence) const;
    void complete_futex_vectors();
    unsigned wake_futex_deferred(FutexAddress key, unsigned count, uint32_t mask);

  public:
    FutexAddress futex_address(const Memory &memory, uint64_t address, bool private_futex = false);
    bool sole_ordinary_task() const noexcept {
        if (tasks_.size() != 1)
            return false;
        const auto &task = tasks_.begin()->second;
        return task.tid == current_ && task.state == Task::State::Runnable && !task.stopped &&
               !task.ptrace_stopped && !task.ptrace_exit_pending && !task.ptrace_syscall &&
               task.sched_policy == Task::SchedulingPolicy::Other;
    }
    bool exited() const {
        return exited_;
    }
    int exit_code() const {
        return exit_code_;
    }
    void set_process_controller(ProcessController *processes) {
        processes_ = processes;
    }
    ProcessController *processes() const {
        return processes_;
    }
    uint32_t spawn(const CpuState &cpu, uint64_t stack, uint64_t tls, uint64_t clear_tid);
    uint32_t spawn_process(const CpuState &cpu, uint64_t stack, uint64_t tls, uint64_t clear_tid);
    void discard(uint32_t tid);
    // Final process reap releases exited-task queues and UID admission charges.
    void reap_process(uint32_t pid);
    void block(TaskDeadline deadline, int64_t timeout_result,
               std::function<std::optional<int64_t>()> poll = {});
    void wait_futex(uint64_t address, uint32_t mask, TaskDeadline deadline, uint64_t space = 0);
    void wait_futex_vector(std::vector<Task::FutexVectorEntry> entries, TaskDeadline deadline,
                           std::function<std::optional<int64_t>()> clock_poll = {});
    // Serial coordinator only. Match Linux's realtime-priority / FIFO plist;
    // the cutoff keeps same-key requeue from selecting one task repeatedly.
    uint32_t first_futex_waiter(FutexAddress key, uint32_t mask = UINT32_MAX,
                                uint64_t before_sequence = UINT64_MAX) const;
    unsigned wake_futex(uint64_t address, unsigned count, uint32_t mask = UINT32_MAX, uint64_t space = 0);
    unsigned wake_futex_pair(FutexAddress first, unsigned first_count, FutexAddress second,
                             unsigned second_count, bool wake_second);
    unsigned requeue_futex(uint64_t from, uint64_t to, unsigned count, uint64_t space = 0,
                           uint64_t target_space = 0);
    unsigned wake_and_requeue_futex(FutexAddress from, FutexAddress to, unsigned wake_count,
                                    unsigned requeue_count);
    void wake(uint32_t tid, int64_t value = 0);
    void interrupt(uint32_t tid, int64_t value = -4);
    void stop_process(uint32_t pid);
    void continue_process(uint32_t pid);
    void exit(int code, bool group);
    void exit_process(uint32_t pid, int code);
    void replace_process_image(uint32_t pid, const CpuState &entry);
    // Call only after workers stop; completes waits without selecting a task.
    TaskClock::time_point poll_waiting();
    bool select(bool blocking = true);
};
} // namespace operit::arm64
