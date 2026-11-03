#include "runtime/machine.h"
#include "linux/process/pidfd.h"

namespace operit::arm64 {
signals::Delivery Machine::deliver_signals(Task &task, Process &process, uint32_t cpu) {
    auto &memory = *process.image.memory;
    if (task.last_cpu != cpu)
        threads::rseq_event(task.rseq, threads::RseqEvent::Migrate);
    task.last_cpu = cpu;
    if (!threads::rseq_resume(memory, task.cpu, task.rseq, cpu, cpu))
        signals::force_fault(*process.signals, task.signals, task.signal_mask, 11, task.rseq.address, 1);
    return signals::deliver(memory, task.cpu, *process.signals, task.signals, task.signal_mask,
                            process.image.signal_trampoline, [&](CpuState &saved) {
                                threads::rseq_event(task.rseq, threads::RseqEvent::Signal);
                                return threads::rseq_resume(memory, saved, task.rseq, cpu, cpu);
                            }, &task.ptrace_signal, [&](const signals::Queued &queued) {
                                return ptrace_->signal_delivery_stop(task.tid, queued);
                            });
}

void Machine::discard_pending_signals(Process &process) {
    // Run discard observers before releasing the owning timers' reservations.
    process.signals->pending.discard(UINT64_MAX);
    for (const auto tid : scheduler_.task_ids(process.pid))
        scheduler_.lookup(tid)->signals.pending.discard(UINT64_MAX);
}

void Machine::bind_runtime_signal_queues() {
    for (const auto &[pid, process] : processes_)
        if (!process->reaped)
            bind_signal_queue(process->signals->pending, pid);
    for (const auto tid : scheduler_.live_tids()) {
        auto &task = scheduler_.find(tid);
        bind_signal_queue(task.signals.pending, task.pid);
    }
}

signals::ThreadRef Machine::signal_thread(Task &task) {
    auto *owner = find_process(task.pid);
    if (owner)
        bind_signal_queue(task.signals.pending, task.pid);
    return {task.tid, task.pid, &task.cpu, &task.signals, &task.signal_mask,
            ptrace_ && ptrace_->traced(task.tid)};
}
signals::ProcessRef Machine::signal_process(Process &process) {
    bind_signal_queue(process.signals->pending, process.pid);
    signals::ProcessRef result;
    result.pid = process.pid;
    result.parent = process.parent;
    result.group = process.group;
    result.session = process.session;
    const auto credentials = process::pidfd_credentials(*this, process.pid);
    result.real_uid = credentials.real_uid;
    result.effective_uid = credentials.effective_uid;
    result.saved_uid = credentials.saved_uid;
    result.zombie = process.state == Process::State::Zombie;
    result.signals = process.signals;
    for (auto tid : scheduler_.live_tids(process.pid))
        result.threads.push_back(signal_thread(scheduler_.find(tid)));
    return result;
}
signals::Hooks Machine::signal_hooks() {
    signals::Hooks hooks;
    hooks.current = [this] { return signal_thread(scheduler_.current()); };
    hooks.process = [this](uint32_t pid) {
        auto *process = find_process(pid);
        if (!process)
            guest_error(3);
        return signal_process(*process);
    };
    hooks.processes = [this] {
        std::vector<signals::ProcessRef> result;
        for (auto pid : process_ids())
            result.push_back(signal_process(*find_process(pid)));
        return result;
    };
    hooks.block = [this](signals::Deadline deadline, int64_t value, signals::Poll poll) {
        scheduler_.block(deadline, value, std::move(poll));
    };
    hooks.wake = [this](uint32_t tid, bool interrupt) {
        auto *task = scheduler_.lookup(tid);
        if (task && task->ptrace_stopped) {
            auto *process = find_process(task->pid);
            if (process && (signals::pending_mask(*process->signals, task->signals) &
                            signals::bit(signals::kill_signal)))
                ptrace_->fatal_signal(tid);
        }
        if (task && interrupt && task->state == Task::State::Waiting) {
            auto *process = find_process(task->pid);
            const auto pending = process ? signals::pending_mask(*process->signals, task->signals) : 0;
            if (task->vfork_child && !(pending & signals::bit(9)))
                return;
            signals::interrupted(task->signals, task->signal_mask);
            scheduler_.interrupt(tid);
        }
    };
    hooks.effect = [this](uint32_t pid, const signals::Delivery &delivery) {
        auto *process = find_process(pid);
        if (!process)
            return;
        switch (delivery.effect) {
        case signals::Effect::Terminate:
        case signals::Effect::Core:
            terminate(pid, delivery.signal, delivery.effect == signals::Effect::Core);
            break;
        case signals::Effect::Stop:
            scheduler_.stop_process(pid);
            process->wait_events.push_back(((delivery.signal & 255) << 8) | 0x7f);
            notify_parent(*process, process->wait_events.back());
            break;
        case signals::Effect::Continue:
            scheduler_.continue_process(pid);
            if (delivery.signal != 9) {
                process->wait_events.push_back(0xffff);
                notify_parent(*process, 0xffff);
            }
            break;
        default:
            break;
        }
    };
    hooks.trampoline = current_process().image.signal_trampoline;
    return hooks;
}
void Machine::configure_terminal(Process &process) {
    TerminalHooks hooks;
    hooks.process = [this](uint32_t pid) {
        const auto &process = require_process(pid);
        if (process.state != Process::State::Running)
            guest_error(3);
        return TerminalProcess{process.pid, process.session, process.group, process.parent};
    };
    hooks.group_in_session = [this](uint32_t group, uint32_t session) {
        for (const auto &[pid, process] : processes_) {
            (void)pid;
            if (!process->reaped && process->state == Process::State::Running && process->group == group &&
                process->session == session)
                return true;
        }
        return false;
    };
    hooks.group_session = [this](uint32_t group) -> std::optional<uint32_t> {
        for (const auto &[pid, process] : processes_) {
            (void)pid;
            if (!process->reaped && process->state == Process::State::Running && process->group == group)
                return process->session;
        }
        return {};
    };
    hooks.orphaned_group = [this](uint32_t group) {
        for (const auto &[pid, process] : processes_) {
            (void)pid;
            if (process->reaped || process->state != Process::State::Running ||
                process->group != group || process->parent <= 1)
                continue;
            const auto *parent = find_process(process->parent);
            if (parent && !parent->reaped && parent->state == Process::State::Running &&
                parent->session == process->session && parent->group != group)
                return false;
        }
        return true;
    };
    hooks.can_steal = [](uint32_t) { return false; }; // Guest CAP_SYS_ADMIN is absent.
    hooks.signal_group = [this](uint32_t group, unsigned number) {
        auto hooks = signal_hooks();
        const auto info = signals::Info::user(number, 0, 0, 128);
        for (auto pid : process_ids()) {
            auto *process = find_process(pid);
            if (process && process->group == group && process->state == Process::State::Running)
                signals::send_process(hooks, signal_process(*process), info);
        }
    };
    hooks.blocked_or_ignored = [this](uint32_t pid, unsigned number) {
        const auto &process = require_process(pid);
        if (signals::ignored(*process.signals, number))
            return true;
        auto *task = scheduler_.lookup(scheduler_.current().tid);
        return task && task->pid == pid && (task->signal_mask & signals::bit(number));
    };
    process.image.syscalls->terminal_manager()->set_hooks(std::move(hooks));
}
void Machine::drain_terminal_signals(Process &process) {
    auto pending = process.image.syscalls->terminal_manager()->take_signals();
    if (pending.empty())
        return;
    auto hooks = signal_hooks();
    for (const auto &signal : pending) {
        const auto info = signals::Info::user(signal.signal, 0, 0, 128);
        for (auto pid : process_ids()) {
            auto *target = find_process(pid);
            if (target && target->group == signal.group && target->state == Process::State::Running)
                signals::send_process(hooks, signal_process(*target), info);
        }
    }
}
} // namespace operit::arm64
