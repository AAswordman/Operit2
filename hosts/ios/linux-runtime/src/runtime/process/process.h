#pragma once
#include "cpu/aarch64/execution_engine.h"
#include "fs/filesystem.h"
#include "linux/syscalls.h"
#include "linux/timers/process_timers.h"
#include "runtime/process/controller.h"
#include <algorithm>
#include <deque>
#include <memory>

namespace operit::arm64 {
struct ProcessImage {
    std::shared_ptr<Memory> memory;
    std::unique_ptr<Filesystem> filesystem;
    std::unique_ptr<Syscalls> syscalls;
    std::unique_ptr<ExecutionEngine> executor;
    std::unique_ptr<FixedComponents> acceleration;
    CpuState entry;
    std::string executable;
    uint64_t signal_trampoline = 0, heap_end = 0;
};
struct Process {
    enum class State { Running, Zombie } state = State::Running;
    uint32_t pid = 1, parent = 0, group = 1, session = 1, parent_tid = 1;
    std::unique_ptr<ProcessTimers> timers;
    unsigned exit_signal = 17;
    bool execed = false, reaped = false, trace_exit_held = false;
    std::optional<int> ptrace_group_exit;
    int status = 0;
    std::optional<int> leader_status;
    uint32_t vfork_parent = 0;
    std::shared_ptr<signals::ProcessState> signals = std::make_shared<signals::ProcessState>();
    std::deque<int> wait_events;
    ProcessImage image;
    std::unique_ptr<ProcessImage> pending_exec;
    uint64_t instructions = 0, decoded_blocks = 0, cache_hits = 0;
    uint64_t calls = 0, unsupported = 0, last_unsupported = 0;
    uint64_t acceleration_calls = 0, acceleration_bytes = 0;
    uint64_t user_nanoseconds = 0, system_nanoseconds = 0, runtime_nanoseconds = 0;
    uint64_t maximum_rss = 0, minor_faults = 0, major_faults = 0;
    uint64_t input_blocks = 0, output_blocks = 0;
    uint64_t voluntary_switches = 0, involuntary_switches = 0;
    uint64_t child_user_nanoseconds = 0, child_system_nanoseconds = 0, child_runtime_nanoseconds = 0;
    uint64_t child_maximum_rss = 0, child_minor_faults = 0, child_major_faults = 0;
    uint64_t child_input_blocks = 0, child_output_blocks = 0;
    uint64_t child_voluntary_switches = 0, child_involuntary_switches = 0;
    // Called once when this parent consumes a child's exit status, never for
    // WNOWAIT or automatic SIGCHLD reaping. Include previously waited descendants.
    void account_waited_child(const Process &child) {
        child_user_nanoseconds += child.user_nanoseconds + child.child_user_nanoseconds;
        child_system_nanoseconds += child.system_nanoseconds + child.child_system_nanoseconds;
        child_runtime_nanoseconds += child.runtime_nanoseconds + child.child_runtime_nanoseconds;
        child_maximum_rss = std::max({child_maximum_rss, child.maximum_rss, child.child_maximum_rss});
        child_minor_faults += child.minor_faults + child.child_minor_faults;
        child_major_faults += child.major_faults + child.child_major_faults;
        child_input_blocks += child.input_blocks + child.child_input_blocks;
        child_output_blocks += child.output_blocks + child.child_output_blocks;
        child_voluntary_switches += child.voluntary_switches + child.child_voluntary_switches;
        child_involuntary_switches += child.involuntary_switches + child.child_involuntary_switches;
    }
};
} // namespace operit::arm64
