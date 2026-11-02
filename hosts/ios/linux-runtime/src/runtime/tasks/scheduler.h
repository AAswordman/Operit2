#pragma once
#include "runtime/tasks/task.h"
#include <cstddef>
#include <map>
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
    uint32_t priority_cursor(int priority, uint32_t fallback) const noexcept;
    static thread_local ScopedTask *task_scope_;
    ScopedTask *bound_task() const noexcept;
    bool references_task(uint32_t tid) const noexcept;
    void set_current_tid(uint32_t tid) noexcept;
    bool exited_ = false;
    int exit_code_ = 0;
    ProcessController *processes_ = nullptr;
    std::map<Memory::SharedIdentity, uint64_t> shared_futexes_;
    uint64_t next_futex_key_ = address_limit;
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
    // The coordinator calls this between batches; choose effective priorities
    // first and advance a separate round-robin cursor for each priority level.
    std::vector<uint32_t> runnable_tids(size_t maximum = SIZE_MAX);
    bool process_alive(uint32_t pid) const;
    bool has_runnable() const;
    struct FutexAddress {
        uint64_t address, space;
    };
    FutexAddress futex_address(const Memory &memory, uint64_t address, bool private_futex = false);
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
    unsigned wake_futex(uint64_t address, unsigned count, uint32_t mask = UINT32_MAX, uint64_t space = 0);
    unsigned requeue_futex(uint64_t from, uint64_t to, unsigned count, uint64_t space = 0,
                           uint64_t target_space = 0);
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
