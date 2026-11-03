#include "runtime/machine.h"
#include <algorithm>
#include <sys/resource.h>
#if defined(__APPLE__)
#include <mach/mach.h>
#include <pthread.h>
#endif

namespace operit::arm64 {
namespace {
uint64_t add_counter(uint64_t first, uint64_t second) {
    if (second > UINT64_MAX - first)
        throw Failure("guest CPU accounting overflow");
    return first + second;
}
uint64_t cpu_nanoseconds(int64_t seconds, int64_t microseconds) {
    if (seconds < 0 || microseconds < 0 || microseconds >= 1000000)
        throw Failure("invalid host thread CPU time");
    const auto value = static_cast<unsigned __int128>(seconds) * 1000000000 +
                       uint64_t(microseconds) * 1000;
    if (value > UINT64_MAX)
        throw Failure("host thread CPU time overflow");
    return uint64_t(value);
}
CpuUsage host_cpu_usage() {
#if defined(__APPLE__)
    thread_basic_info_data_t info{};
    mach_msg_type_number_t count = THREAD_BASIC_INFO_COUNT;
    if (::thread_info(::pthread_mach_thread_np(::pthread_self()), THREAD_BASIC_INFO,
                      reinterpret_cast<thread_info_t>(&info), &count) != KERN_SUCCESS)
        throw Failure("cannot measure guest execution CPU time");
    const auto user = cpu_nanoseconds(info.user_time.seconds, info.user_time.microseconds);
    const auto system = cpu_nanoseconds(info.system_time.seconds, info.system_time.microseconds);
#else
    rusage usage{};
    if (::getrusage(RUSAGE_THREAD, &usage))
        throw Failure("cannot measure guest execution CPU time");
    const auto user = cpu_nanoseconds(usage.ru_utime.tv_sec, usage.ru_utime.tv_usec);
    const auto system = cpu_nanoseconds(usage.ru_stime.tv_sec, usage.ru_stime.tv_usec);
#endif
    return {user, system, add_counter(user, system)};
}
template <typename State> CpuUsage cpu_snapshot(const State &state) {
    return {state.user_nanoseconds, state.system_nanoseconds, state.runtime_nanoseconds};
}
CpuUsage add_usage(const CpuUsage &first, const CpuUsage &second) {
    return {add_counter(first.user_nanoseconds, second.user_nanoseconds),
            add_counter(first.system_nanoseconds, second.system_nanoseconds),
            add_counter(first.runtime_nanoseconds, second.runtime_nanoseconds)};
}
template <typename State> void charge_usage(State &state, const CpuUsage &delta) {
    const auto total = add_usage(cpu_snapshot(state), delta);
    state.user_nanoseconds = total.user_nanoseconds;
    state.system_nanoseconds = total.system_nanoseconds;
    state.runtime_nanoseconds = total.runtime_nanoseconds;
}
template <typename State> ResourceUsage resource_snapshot(const State &state, uint64_t maximum_rss) {
    ResourceUsage usage;
    usage.cpu = cpu_snapshot(state);
    usage.maximum_rss = maximum_rss;
    usage.minor_faults = state.minor_faults;
    usage.major_faults = state.major_faults;
    usage.input_blocks = state.input_blocks;
    usage.output_blocks = state.output_blocks;
    usage.voluntary_switches = state.voluntary_switches;
    usage.involuntary_switches = state.involuntary_switches;
    return usage;
}
} // namespace

CpuUsage Machine::uncommitted_cpu_usage(uint32_t pid, uint32_t tid) const {
    if (!accounting_pid_ || accounting_pid_ != pid || (tid && accounting_tid_ != tid))
        return {};
    const auto now = host_cpu_usage();
    if (now.user_nanoseconds < accounting_user_ || now.system_nanoseconds < accounting_system_)
        throw Failure("guest execution CPU clock moved backwards");
    const auto user = now.user_nanoseconds - accounting_user_;
    const auto system = now.system_nanoseconds - accounting_system_;
    return {user, system, add_counter(user, system)};
}
void Machine::stop_accounting() {
    if (!accounting_pid_)
        return;
    const auto delta = uncommitted_cpu_usage(accounting_pid_, accounting_tid_);
    const auto it = processes_.find(accounting_pid_);
    if (it != processes_.end()) {
        charge_usage(*it->second, delta);
        if (auto *task = scheduler_.lookup(accounting_tid_); task && task->pid == accounting_pid_) {
            charge_usage(*task, delta);
            const auto limit = it->second->image.syscalls->resource_limits()->get(process::Resource::RealtimeTime);
            scheduler_.account_runtime(task->tid, delta.runtime_nanoseconds,
                                       limit.soft != process::resource_infinity);
        }
    }
    accounting_pid_ = accounting_tid_ = 0;
}
void Machine::begin_accounting(uint32_t pid) {
    const auto tid = scheduler_.current().tid;
    if (accounting_pid_ == pid && accounting_tid_ == tid)
        return;
    stop_accounting();
    const auto now = host_cpu_usage();
    accounting_user_ = now.user_nanoseconds;
    accounting_system_ = now.system_nanoseconds;
    accounting_pid_ = pid;
    accounting_tid_ = tid;
}
ResourceUsage Machine::snapshot_resource_usage(const Process &process, const Task *task) const {
    const auto rss = std::max(process.maximum_rss, process.image.memory->resident());
    auto result = task ? resource_snapshot(*task, rss) : resource_snapshot(process, rss);
    result.cpu = add_usage(result.cpu, uncommitted_cpu_usage(process.pid, task ? task->tid : 0));
    return result;
}
ResourceUsage Machine::snapshot_child_usage(const Process &process) const {
    ResourceUsage usage;
    usage.cpu = {process.child_user_nanoseconds, process.child_system_nanoseconds,
                 process.child_runtime_nanoseconds};
    usage.maximum_rss = process.child_maximum_rss;
    usage.minor_faults = process.child_minor_faults;
    usage.major_faults = process.child_major_faults;
    usage.input_blocks = process.child_input_blocks;
    usage.output_blocks = process.child_output_blocks;
    usage.voluntary_switches = process.child_voluntary_switches;
    usage.involuntary_switches = process.child_involuntary_switches;
    return usage;
}
CpuUsage Machine::cpu_usage(uint32_t id, bool per_thread, bool gettime) {
    const auto &caller = scheduler_.current();
    if (per_thread) {
        const auto *task = scheduler_.lookup(id ? id : caller.tid);
        if (!task || task->pid != caller.pid ||
            (task->state == Task::State::Exited && task->tid != task->pid) || !find_process(task->pid))
            guest_error(22);
        const auto committed = cpu_snapshot(*task);
        return gettime ? add_usage(committed, uncommitted_cpu_usage(task->pid, task->tid)) : committed;
    }
    // Linux accepts the caller's own nonleader TID for a process-clock read,
    // but clock_getres requires the process leader ID (or the zero alias).
    const auto pid = !id || (gettime && id == caller.tid) ? caller.pid : id;
    const auto *process = find_process(pid);
    if (!process)
        guest_error(22);
    const auto committed = cpu_snapshot(*process);
    return gettime ? add_usage(committed, uncommitted_cpu_usage(pid)) : committed;
}

ResourceUsage Machine::resource_usage(ResourceScope scope) {
    // Workers have joined and charged their deltas before any syscall dispatch.
    // Include this serial slice without stopping its clock: repeated readers
    // must neither lose the rest of a syscall nor charge the same slice twice.
    auto &process = current_process();
    switch (scope) {
    case ResourceScope::Process:
    case ResourceScope::Thread:
        process.maximum_rss = std::max(process.maximum_rss, process.image.memory->resident());
        return snapshot_resource_usage(process, scope == ResourceScope::Thread ? &scheduler_.current()
                                                                               : nullptr);
    case ResourceScope::Children:
        return snapshot_child_usage(process);
    }
    guest_error(22);
}
void Machine::enforce_cpu_limits() {
    bool settled = false;
    for (auto &[pid, process] : processes_) {
        if (process->state != Process::State::Running)
            continue;
        auto &limits = *process->image.syscalls->resource_limits();
        const auto cpu = limits.get(process::Resource::Cpu);
        const auto realtime = limits.get(process::Resource::RealtimeTime);
        const bool cpu_limited = cpu.soft != process::resource_infinity || cpu.hard != process::resource_infinity;
        const bool realtime_limited = realtime.soft != process::resource_infinity ||
                                      realtime.hard != process::resource_infinity;
        if (!cpu_limited && !realtime_limited)
            continue;
        if (!settled) {
            stop_accounting();
            settled = true;
        }
        const auto tids = scheduler_.live_tids(pid);
        if (tids.empty())
            continue;
        if (cpu_limited) {
            const int result = process::enforce_cpu_limit(scheduler_, tids.front(), limits,
                process->user_nanoseconds, process->system_nanoseconds);
            if (result < 0 && result != -3)
                guest_error(-result);
            if (result == 9)
                continue;
        }
        if (realtime_limited) {
            for (const auto tid : tids) {
                const auto *task = scheduler_.lookup(tid);
                if (!task || !task->realtime_runtime_nanoseconds || scheduler_.effective_priority(tid) >= 100)
                    continue;
                const int result = process::enforce_realtime_limit(
                    scheduler_, tid, limits, task->realtime_runtime_nanoseconds);
                if (result < 0 && result != -3)
                    guest_error(-result);
                if (result == 9)
                    break;
            }
        }
    }
}
} // namespace operit::arm64
