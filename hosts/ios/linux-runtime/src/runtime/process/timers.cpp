#include "runtime/machine.h"
#include "linux/io_uring/io_uring.h"
#include "linux/process/pidfd.h"
#include "linux/events/inotify.h"

namespace operit::arm64 {
ProcessTimerHooks Machine::timer_hooks() {
    ProcessTimerHooks hooks;
    hooks.signals = signal_hooks();
    hooks.reserve_signal = [this](uint32_t pid) {
        const auto *owner = find_process(pid);
        if (!owner || owner->state != Process::State::Running)
            guest_error(3);
        const auto credentials = process::pidfd_credentials(*this, pid);
        return std::make_shared<process::UidLimitCharge>(uid_limits_.reserve_signal(
            credentials.real_uid, *owner->image.syscalls->resource_limits()));
    };
    hooks.cpu_usage = [this](uint32_t id, bool per_thread) -> std::optional<CpuUsage> {
        if (per_thread) {
            const auto *task = scheduler_.lookup(id);
            if (!task || task->state == Task::State::Exited)
                return std::nullopt;
            const auto *owner = find_process(task->pid);
            if (!owner || owner->state != Process::State::Running)
                return std::nullopt;
            return snapshot_resource_usage(*owner, task).cpu;
        }
        const auto *process = find_process(id);
        if (!process || process->state != Process::State::Running)
            return std::nullopt;
        return snapshot_resource_usage(*process).cpu;
    };
    return hooks;
}

void Machine::bind_signal_queue(signals::Pending &pending, uint32_t pid) {
    pending.set_charge_source([this, pid](const signals::Info &info, bool forced) {
        const auto *owner = find_process(pid);
        if (!owner)
            guest_error(3);
        const auto credentials = process::pidfd_credentials(*this, pid);
        const bool override_limit = forced || info.code != -1; // SI_QUEUE obeys RLIMIT_SIGPENDING.
        return std::make_shared<process::UidLimitCharge>(uid_limits_.reserve_signal(
            credentials.real_uid, *owner->image.syscalls->resource_limits(), override_limit));
    });
    pending.set_observers(
        [this, pid](signals::Info &info) {
            auto *owner = find_process(pid);
            if (!owner)
                return false;
            // Ordinary signals do not require a ProcessTimers instance. A
            // queue is bound before its owner creates any timer as well.
            if (!owner->timers)
                return true;
            auto hooks = timer_hooks();
            return owner->timers->signal_dequeued(info, hooks);
        },
        [this, pid](const signals::Info &info) {
            auto *owner = find_process(pid);
            if (owner && owner->timers)
                owner->timers->signal_discarded(info);
        });
}

int64_t Machine::process_timer(CpuState &cpu, Memory &memory) {
    auto &process = current_process();
    if (!process.timers) {
        process.timers = std::make_unique<ProcessTimers>(process.pid);
        bind_signal_queue(process.signals->pending, process.pid);
        for (const auto tid : scheduler_.live_tids(process.pid))
            bind_signal_queue(scheduler_.find(tid).signals.pending, process.pid);
    }
    auto hooks = timer_hooks();
    return process.timers->dispatch(cpu, memory, hooks);
}

void Machine::poll_runtime_events() {
    // Both Machine's quantum boundary and Scheduler's idle loop enter here
    // with all workers stopped, before retrying a blocked guest operation.
    // Settle the preceding syscall before a timer/signal can wake its task
    // and reset the continuous realtime timeout. Enforce the charged limits
    // before servicing those wakeups as well.
    stop_accounting();
    io_uring::service_completions(io_uring_context_, scheduler_);
    flush_ptrace_events();
    service_ptrace_exits();
    bind_runtime_signal_queues();
    InotifyFile::service_all();
    enforce_cpu_limits();
    poll_process_timers();
}

signals::Deadline Machine::next_runtime_deadline() {
    signals::Deadline result;
    for (auto &[pid, process] : processes_) {
        (void)pid;
        if (process->reaped || process->state != Process::State::Running || !process->timers)
            continue;
        auto hooks = timer_hooks();
        const auto deadline = process->timers->next_deadline(hooks);
        if (deadline && (!result || *deadline < *result))
            result = deadline;
    }
    return result;
}

void Machine::poll_process_timers() {
    for (auto &[pid, process] : processes_) {
        (void)pid;
        if (process->state != Process::State::Running || !process->timers)
            continue;
        auto hooks = timer_hooks();
        process->timers->poll(hooks);
    }
}
} // namespace operit::arm64
