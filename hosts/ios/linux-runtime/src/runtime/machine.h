#pragma once
#include "runtime/console/console.h"
#include "linux/debug/ptrace.h"
#include "linux/process/limit_enforcement.h"
#include "runtime/process/process.h"
#include "runtime/tasks/executor.h"
#include <map>

namespace operit::arm64 {
class Machine final : public ProcessController {
    ort_arm64_config config_;
    Scheduler scheduler_;
    process::UidLimitAccounting uid_limits_;
    std::shared_ptr<io_uring::CompletionContext> io_uring_context_;
    std::map<uint32_t, std::unique_ptr<Process>> processes_;
    std::unique_ptr<debug::Ptrace> ptrace_;
    struct PtraceNotification { uint32_t tracer, tracee; int status; };
    struct PtraceSignal { uint32_t tid; signals::Queued signal; bool attach; };
    std::vector<PtraceNotification> ptrace_notifications_;
    std::vector<PtraceSignal> ptrace_signals_;
    std::vector<uint32_t> ptrace_releases_;
    bool ptrace_exit_deferred_ = false;
    std::optional<Console> console_;
    uint32_t execution_threads_ = 1, maximum_parallel_tasks_ = 1;
    std::unique_ptr<ParallelExecutor> executor_;
    struct ThreadExecution {
        std::shared_ptr<Memory> memory;
        const FixedComponents *source = nullptr;
        uint64_t generation = 0;
        std::unique_ptr<Interpreter> interpreter;
        std::unique_ptr<FixedComponents> acceleration;
    };
    std::map<uint32_t, ThreadExecution> thread_executions_;
    bool run_parallel(uint64_t &budget);
    bool load_attempted_ = false, loaded_ = false, started_ = false, failed_ = false;
    bool schedule_pending_ = false, main_exited_ = false;
    int main_exit_code_ = 0;
    uint64_t quantum_remaining_ = 4096;
    ort_arm64_status failure_status_ = ORT_ARM64_FAULT;
    ort_arm64_result retired_{};
    uint32_t accounting_pid_ = 0, accounting_tid_ = 0;
    uint64_t accounting_user_ = 0, accounting_system_ = 0;
    void begin_accounting(uint32_t pid);
    void stop_accounting();
    CpuUsage uncommitted_cpu_usage(uint32_t pid, uint32_t tid = 0) const;
    ResourceUsage snapshot_resource_usage(const Process &process, const Task *task = nullptr) const;
    ResourceUsage snapshot_child_usage(const Process &process) const;
    Process &current_process();
    const Process &require_process(uint32_t pid) const;
    void archive_image(Process &process);
    void commit_exec(Process &process);
    void finish_process(Process &process, int status);
    void release_vfork(Process &process);
    void release_tasks(Process &process, bool preserve_current_sem_undo = false);
    void notify_parent(Process &process, int status, unsigned signal = 17);
    void collect_reaped();
    signals::ThreadRef signal_thread(Task &task);
    signals::ProcessRef signal_process(Process &process);
    signals::Delivery deliver_signals(Task &task, Process &process, uint32_t cpu);
    void configure_terminal(Process &process);
    void drain_terminal_signals(Process &process);
    ProcessTimerHooks timer_hooks();
    void bind_signal_queue(signals::Pending &pending, uint32_t pid);
    void bind_runtime_signal_queues();
    void discard_pending_signals(Process &process);
    void poll_process_timers();
    void enforce_cpu_limits();
    void initialize_ptrace();
    void flush_ptrace_events();
    void release_ptrace_exit(uint32_t tid);
    void prepare_ptrace_exit(uint32_t pid, uint32_t tid, bool group, int status);
    bool defer_ptrace_exit(uint32_t pid, uint32_t tid, bool group, int status);
    void service_ptrace_exits();
    bool dispatch_guest_syscall(Task &task, Process &process, bool resumed);

  public:
    std::string error;
    explicit Machine(const ort_arm64_config &config);
    ~Machine() override;
    int bind(const char *source, const char *target, uint32_t flags);
    int load(const char *path, const char *const *argv, const char *const *envp);
    ort_arm64_result run(uint64_t budget);
    int terminal_open(uint32_t columns, uint32_t rows);
    int64_t terminal_read(void *bytes, size_t length);
    int64_t terminal_write(const void *bytes, size_t length);
    int terminal_resize(uint32_t columns, uint32_t rows);
    int terminal_signal(int number);
    int terminal_close();
    Process *find_process(uint32_t pid);
    Scheduler &scheduler() {
        return scheduler_;
    }
    uint32_t clone(const CpuState &cpu, const ProcessClone &request) override;
    void exec(const std::string &path, const std::vector<std::string> &arguments,
              const std::vector<std::string> &environment, const std::string &working_directory) override;
    std::optional<ProcessWaitResult> wait(const ProcessWait &request) override;
    void terminate(uint32_t pid, unsigned signal, bool core = false) override;
    uint32_t parent_pid(uint32_t pid) const override;
    uint32_t process_group(uint32_t pid) const override;
    uint32_t session(uint32_t pid) const override;
    void set_process_group(uint32_t pid, uint32_t group) override;
    uint32_t create_session() override;
    std::vector<uint32_t> process_ids() const override;
    Syscalls *process_syscalls(uint32_t pid) override;
    CpuUsage cpu_usage(uint32_t id, bool per_thread, bool gettime) override;
    ResourceUsage resource_usage(ResourceScope scope) override;
    ProcessAccounting proc_stat_accounting(uint32_t pid, uint32_t tid,
                                           bool whole_group) const override;
    void poll_runtime_events() override;
    signals::Deadline next_runtime_deadline() override;
    int64_t process_timer(CpuState &cpu, Memory &memory) override;
    signals::Hooks signal_hooks() override;
    void wait_completed(uint32_t tid) override;
    int64_t ptrace_request(CpuState &cpu) override;
    bool task_exiting(uint32_t pid, uint32_t tid, bool group, int status) override;
};
} // namespace operit::arm64
