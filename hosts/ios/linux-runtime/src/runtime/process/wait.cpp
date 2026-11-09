#include "runtime/machine.h"
#include "linux/process/pidfd.h"
#include <algorithm>

namespace operit::arm64 {
std::optional<ProcessWaitResult> Machine::wait(const ProcessWait &request) {
    auto &parent = current_process();
    // Poll callbacks may belong to a different task than Scheduler::current().
    const auto *caller = scheduler_.lookup(request.caller_tid);
    const uint32_t waiting_pid = caller ? caller->pid : parent.pid;
    auto *waiting = find_process(waiting_pid);
    if (!waiting)
        guest_error(10);
    constexpr uint32_t untraced = 2, exited = 4, continued = 8, nowait = 0x1000000;
    constexpr uint32_t nothread = 0x20000000, all = 0x40000000, clone = 0x80000000;
    bool eligible = false;
    auto fill_usage = [](ProcessWaitResult &result, Process &child) {
        child.maximum_rss = std::max(child.maximum_rss, child.image.memory->resident());
        result.user_microseconds = (child.user_nanoseconds + child.child_user_nanoseconds) / 1000;
        result.system_microseconds = (child.system_nanoseconds + child.child_system_nanoseconds) / 1000;
        result.maximum_rss = std::max(child.maximum_rss, child.child_maximum_rss) / 1024;
        result.minor_faults = child.minor_faults + child.child_minor_faults;
        result.major_faults = child.major_faults + child.child_major_faults;
        result.voluntary_switches = child.voluntary_switches + child.child_voluntary_switches;
        result.involuntary_switches = child.involuntary_switches + child.child_involuntary_switches;
    };
    debug::WaitRequest trace_request;
    trace_request.kind = request.kind == ProcessWait::Kind::Pid     ? debug::WaitRequest::Kind::Tid
                         : request.kind == ProcessWait::Kind::Group ? debug::WaitRequest::Kind::Group
                                                                    : debug::WaitRequest::Kind::Any;
    trace_request.id = request.id;
    trace_request.options = request.options;
    if (const auto event = ptrace_->wait(request.caller_tid, trace_request, eligible)) {
        ProcessWaitResult result;
        result.pid = event->tid;
        result.uid = event->uid;
        result.status = event->status;
        result.trace_stop = !event->exited;
        const auto *task = scheduler_.lookup(event->tid);
        auto *child = task ? find_process(task->pid) : nullptr;
        if (child)
            fill_usage(result, *child);
        if (event->exited && !(request.options & nowait) && child &&
            event->tid == child->pid && child->trace_exit_held) {
            if (child->parent == waiting_pid) {
                child->trace_exit_held = false;
                if (child->state == Process::State::Zombie) {
                    child->reaped = true;
                    waiting->account_waited_child(*child);
                    result.process_reaped = true;
                }
            } else {
                release_ptrace_exit(event->tid);
            }
        }
        return result;
    }
    for (auto &[pid, child] : processes_) {
        if (child->reaped || child->parent != waiting_pid)
            continue;
        if (request.kind == ProcessWait::Kind::Pid && pid != request.id)
            continue;
        if (request.kind == ProcessWait::Kind::Group && child->group != request.id)
            continue;
        if ((request.options & nothread) && child->parent_tid != request.caller_tid)
            continue;
        if (!(request.options & all) && ((child->exit_signal != 17) != bool(request.options & clone)))
            continue;
        eligible = true;
        int status = 0;
        bool available = false, zombie = child->state == Process::State::Zombie;
        if (zombie && child->trace_exit_held)
            continue;
        if (zombie && (request.options & exited)) {
            available = true;
            status = child->status;
        } else {
            auto event = std::find_if(child->wait_events.begin(), child->wait_events.end(), [&](int value) {
                return value == 0xffff ? bool(request.options & continued) : bool(request.options & untraced);
            });
            if (event != child->wait_events.end()) {
                status = *event;
                available = true;
                if (!(request.options & nowait))
                    child->wait_events.erase(event);
            }
        }
        if (!available)
            continue;
        ProcessWaitResult result;
        result.pid = pid;
        result.uid = process::pidfd_credentials(*this, pid).real_uid;
        result.status = status;
        fill_usage(result, *child);
        if (zombie && !(request.options & nowait)) {
            child->reaped = true;
            waiting->account_waited_child(*child);
            result.process_reaped = true;
        }
        return result;
    }
    if (!eligible)
        guest_error(10);
    return {};
}
void Machine::collect_reaped() {
    for (auto it = processes_.begin(); it != processes_.end();) {
        auto &process = *it->second;
        if (!process.reaped || process.pid == scheduler_.current().pid) {
            ++it;
            continue;
        }
        process::pidfd_process_reaped(*this, process.pid);
        // The scheduler owns the leader's charge until this final reap and
        // runs thread pending-queue discard observers before removing tasks.
        scheduler_.reap_process(process.pid);
        archive_image(process);
        retired_.instructions += process.instructions;
        retired_.decoded_blocks += process.decoded_blocks;
        retired_.block_cache_hits += process.cache_hits;
        retired_.syscalls += process.calls;
        retired_.unsupported_syscalls += process.unsupported;
        if (process.last_unsupported)
            retired_.last_unsupported_syscall = process.last_unsupported;
        retired_.accelerated_calls += process.acceleration_calls;
        retired_.accelerated_bytes += process.acceleration_bytes;
        it = processes_.erase(it);
    }
    collect_thread_executions();
}
uint32_t Machine::parent_pid(uint32_t pid) const {
    return require_process(pid).parent;
}
uint32_t Machine::process_group(uint32_t pid) const {
    return require_process(pid).group;
}
uint32_t Machine::session(uint32_t pid) const {
    return require_process(pid).session;
}
std::vector<uint32_t> Machine::process_ids() const {
    std::vector<uint32_t> ids;
    for (const auto &[pid, process] : processes_)
        if (!process->reaped)
            ids.push_back(pid);
    return ids;
}
void Machine::set_process_group(uint32_t pid, uint32_t group) {
    auto &caller = current_process();
    auto *target = find_process(pid ? pid : caller.pid);
    if (!target || (target->pid != caller.pid && target->parent != caller.pid))
        guest_error(3);
    group = group ? group : target->pid;
    if (target->session != caller.session || target->pid == target->session)
        guest_error(1);
    if (target->pid != caller.pid && target->execed)
        guest_error(13);
    if (group != target->pid) {
        bool found = false;
        for (const auto &[candidate, process] : processes_) {
            (void)candidate;
            found |= !process->reaped && process->group == group && process->session == caller.session;
        }
        if (!found)
            guest_error(1);
    }
    target->group = group;
}
uint32_t Machine::create_session() {
    auto &process = current_process();
    for (const auto &[pid, candidate] : processes_) {
        (void)pid;
        if (!candidate->reaped && candidate->group == process.pid)
            guest_error(1);
    }
    const auto previous = process.session;
    process.session = process.group = process.pid;
    process.image.syscalls->terminal_manager()->session_created(process.pid, previous);
    return process.pid;
}
} // namespace operit::arm64
