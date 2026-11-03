#pragma once
#include "cpu/aarch64/state.h"
#include "linux/signals/signals.h"
#include <functional>
#include <optional>
#include <string>
#include <vector>

namespace operit::arm64 {
class Syscalls;
struct ProcessClone {
    uint64_t flags = 0, stack = 0, parent_tid = 0, tls = 0, child_tid = 0;
    uint32_t exit_signal = 17;
};
struct ProcessWait {
    enum class Kind { All, Pid, Group } kind = Kind::All;
    uint32_t id = 0;
    uint32_t options = 0;
    uint32_t caller_tid = 0;
};
struct ProcessWaitResult {
    uint32_t pid = 0, uid = 0;
    int status = 0;
    bool trace_stop = false, process_reaped = false;
    uint64_t user_microseconds = 0, system_microseconds = 0, maximum_rss = 0;
    uint64_t minor_faults = 0, major_faults = 0, voluntary_switches = 0, involuntary_switches = 0;
};
struct CpuUsage {
    uint64_t user_nanoseconds = 0, system_nanoseconds = 0, runtime_nanoseconds = 0;
};
enum class ResourceScope { Process, Thread, Children };
struct ResourceUsage {
    CpuUsage cpu;
    uint64_t maximum_rss = 0; // Bytes, including the process address space for RUSAGE_THREAD.
    uint64_t minor_faults = 0, major_faults = 0;
    uint64_t input_blocks = 0, output_blocks = 0;
    uint64_t voluntary_switches = 0, involuntary_switches = 0;
};
struct ProcessAccounting {
    ResourceUsage self, children;
    uint64_t start_boottime_nanoseconds = 0;
};
// Same origin and suspend semantics as the guest CLOCK_BOOTTIME (clock ID 7).
// Task creation samples this once; exec retains the process leader's birth time.
uint64_t process_boottime_nanoseconds();

// Owned by Machine. Syscall handlers use this interface without owning or replacing
// their executing Syscalls object; exec commits at the next interpreter boundary.
class ProcessController {
  public:
    virtual ~ProcessController() = default;
    virtual uint32_t clone(const CpuState &cpu, const ProcessClone &request) = 0;
    virtual void exec(const std::string &path, const std::vector<std::string> &arguments,
                      const std::vector<std::string> &environment, const std::string &working_directory) = 0;
    virtual std::optional<ProcessWaitResult> wait(const ProcessWait &request) = 0;
    virtual void terminate(uint32_t pid, unsigned signal, bool core = false) = 0;
    virtual uint32_t parent_pid(uint32_t pid) const = 0;
    virtual uint32_t process_group(uint32_t pid) const = 0;
    virtual uint32_t session(uint32_t pid) const = 0;
    virtual void set_process_group(uint32_t pid, uint32_t group) = 0;
    virtual uint32_t create_session() = 0;
    virtual std::vector<uint32_t> process_ids() const = 0;
    virtual Syscalls *process_syscalls(uint32_t pid) = 0;
    // IDs are guest IDs. Zero selects the caller. Thread clocks may only name
    // the caller's thread group; process clocks may name an unreaped process.
    // gettime additionally permits the caller TID as a process-clock target.
    // A false gettime validates the target without sampling running workers.
    virtual CpuUsage cpu_usage(uint32_t id, bool per_thread, bool gettime) = 0;
    // Return a coherent snapshot after settling active worker CPU deltas.
    // Children contains only descendants accounted by a consuming wait.
    virtual ResourceUsage resource_usage(ResourceScope scope) = 0;
    // Explicit guest identities, sampled only after parallel workers join.
    // whole_group includes exited threads retained in process accounting;
    // children contains descendants consumed by wait, shared by the group.
    // Missing/reaped processes or absent nonleader threads report ENOENT.
    virtual ProcessAccounting proc_stat_accounting(uint32_t pid, uint32_t tid,
                                                   bool whole_group) const = 0;
    // Scheduler idle polling uses the same event pump as Machine::run.
    virtual void poll_runtime_events() = 0;
    virtual signals::Deadline next_runtime_deadline() = 0;
    virtual int64_t process_timer(CpuState &cpu, Memory &memory) = 0;
    virtual signals::Hooks signal_hooks() = 0;
    virtual void wait_completed(uint32_t) {}
    virtual int64_t ptrace_request(CpuState &) { return -38; }
    // True means exit is deferred until an attached tracer resumes the task.
    virtual bool task_exiting(uint32_t, uint32_t, bool, int) { return false; }
    // Signal integration: a child event targets its parent PID, not an arbitrary
    // host process. The signal layer supplies actual pending-queue delivery.
    std::function<void(uint32_t, uint32_t, unsigned, int)> child_event;
    std::function<void(uint32_t)> exec_event;
};
} // namespace operit::arm64
