#include "runtime/machine.h"
#include "linux/process/pidfd.h"
#include "memory/fault.h"
#include <utility>

namespace operit::arm64 {
void Machine::initialize_ptrace() {
    debug::Hooks hooks;
    hooks.target = [this](uint32_t tid) -> std::optional<debug::Target> {
        auto *task = scheduler_.lookup(tid);
        auto *process = task ? find_process(task->pid) : nullptr;
        if (!task || task->state == Task::State::Exited || !process ||
            process->state != Process::State::Running)
            return {};
        const auto credentials = process::pidfd_credentials(*this, task->pid);
        debug::Target result;
        result.tid = tid;
        result.tgid = task->pid;
        result.parent_tid = process->parent_tid;
        result.pgrp = process->group;
        result.ruid = credentials.real_uid;
        result.euid = credentials.effective_uid;
        result.suid = credentials.saved_uid;
        result.rgid = credentials.real_gid;
        result.egid = credentials.effective_gid;
        result.sgid = credentials.saved_gid;
        result.permitted_capabilities = credentials.permitted_capabilities;
        result.dumpable = credentials.dumpable;
        result.cap_sys_ptrace = credentials.cap_sys_ptrace;
        result.alive = true;
        result.cpu = &task->cpu;
        return result;
    };
    hooks.memory = [this](uint32_t tid, uint64_t address, void *buffer, size_t size, debug::Access access) {
        auto *task = scheduler_.lookup(tid);
        auto *process = task ? find_process(task->pid) : nullptr;
        if (!process || process->state != Process::State::Running)
            return false;
        auto &memory = *process->image.memory;
        try {
            switch (access) {
            case debug::Access::Read:
                memory.copy_out(address, buffer, size);
                return true;
            case debug::Access::Write:
                memory.copy_in(address, buffer, size);
                return true;
            case debug::Access::ForceRead:
                return memory.ptrace_read(address, buffer, size);
            case debug::Access::ForceWrite:
                return memory.ptrace_write(address, buffer, size);
            }
        } catch (const GuestError &) {
        } catch (const MemoryFault &) {
        }
        return false;
    };
    hooks.set_stopped = [this](uint32_t tid, bool stopped) {
        if (auto *task = scheduler_.lookup(tid); task && task->state != Task::State::Exited) {
            task->ptrace_stopped = stopped;
            scheduler_.scheduling_changed(tid);
        }
    };
    hooks.attach_stop = [this](uint32_t tid) {
        const auto *target = scheduler_.lookup(tid);
        if (!target || target->state == Task::State::Exited)
            return false;
        const auto &caller = scheduler_.current();
        const auto credentials = process::pidfd_credentials(*this, caller.pid);
        signals::Queued queued{};
        queued.info = signals::Info::user(signals::stop_signal, caller.pid, credentials.real_uid);
        ptrace_signals_.push_back({tid, std::move(queued), true});
        return true;
    };
    hooks.resume_signal = [this](uint32_t tid, std::optional<signals::Queued> queued) {
        if (auto *task = scheduler_.lookup(tid))
            task->ptrace_signal = std::move(queued);
    };
    hooks.queue_signal = [this](uint32_t tid, signals::Queued queued) {
        ptrace_signals_.push_back({tid, std::move(queued), false});
    };
    hooks.notify = [this](uint32_t tracer, uint32_t tracee, int status) {
        ptrace_notifications_.push_back({tracer, tracee, status});
    };
    hooks.release_exit = [this](uint32_t tid) { ptrace_releases_.push_back(tid); };
    ptrace_ = std::make_unique<debug::Ptrace>(std::move(hooks));
}

void Machine::flush_ptrace_events() {
    auto queued_signals = std::move(ptrace_signals_);
    ptrace_signals_.clear();
    for (auto &item : queued_signals) {
        auto *task = scheduler_.lookup(item.tid);
        auto *process = task ? find_process(task->pid) : nullptr;
        if (!task || task->state == Task::State::Exited || !process ||
            process->state != Process::State::Running || (item.attach && !ptrace_->traced(item.tid)))
            continue;
        if (item.attach && task->stopped) {
            if (task->state == Task::State::Waiting) {
                signals::interrupted(task->signals, task->signal_mask);
                scheduler_.interrupt(task->tid);
            }
            ptrace_->signal_delivery_stop(item.tid, std::move(item.signal));
            continue;
        }
        auto hooks = signal_hooks();
        (void)signals::send_thread(hooks, signal_process(*process), signal_thread(*task),
                                   item.signal.info, item.signal.forced, std::move(item.signal.charge));
    }
    auto notifications = std::move(ptrace_notifications_);
    ptrace_notifications_.clear();
    for (const auto &item : notifications) {
        auto *tracer = scheduler_.lookup(item.tracer);
        auto *parent = tracer ? find_process(tracer->pid) : nullptr;
        if (!tracer || tracer->state == Task::State::Exited || !parent ||
            parent->state != Process::State::Running)
            continue;
        const auto *tracee = scheduler_.lookup(item.tracee);
        auto *child = tracee ? find_process(tracee->pid) : nullptr;
        const uint32_t uid = child ? process::pidfd_credentials(*this, child->pid).real_uid : 0;
        const bool stopped = (item.status & 255) == 0x7f;
        const int code = stopped ? 4 : (item.status & 127) ? ((item.status & 128) ? 3 : 2) : 1;
        const int value = stopped ? ((item.status >> 8) & 255)
                                 : (item.status & 127) ? (item.status & 127) : ((item.status >> 8) & 255);
        auto hooks = signal_hooks();
        (void)signals::send_process(hooks, signal_process(*parent),
                                    signals::Info::child(item.tracee, uid, value, code));
    }
    auto releases = std::move(ptrace_releases_);
    ptrace_releases_.clear();
    for (const auto tid : releases)
        release_ptrace_exit(tid);
}

void Machine::wait_completed(uint32_t tid) {
    auto *task = scheduler_.lookup(tid);
    if (!task || !task->ptrace_exit_pending || task->ptrace_stopped || task->ptrace_deferred_exit)
        return;
    task->ptrace_exit_pending = false;
    ptrace_->syscall_exit(tid);
}

int64_t Machine::ptrace_request(CpuState &cpu) {
    return ptrace_->request(scheduler_.current_tid(), cpu.x[0], int64_t(cpu.x[1]), cpu.x[2], cpu.x[3]);
}

bool Machine::dispatch_guest_syscall(Task &task, Process &process, bool resumed) {
    const uint64_t number = resumed ? *task.ptrace_syscall : task.cpu.x[8];
    if (!resumed && ptrace_->syscall_entry(task.tid)) {
        task.ptrace_syscall = number;
        return false;
    }
    task.ptrace_syscall.reset();
    const auto code = int(task.cpu.x[0]) & 255;
    const auto guest_x8 = task.cpu.x[8];
    if (resumed && guest_x8 != number)
        task.ptrace_user_x8 = guest_x8;
    task.cpu.x[8] = number;
    signals::remember_syscall(task.signals, task.cpu, true);
    process.image.syscalls->handle(task.cpu);
    ptrace_->preserve_event_syscall_result(task.tid);
    task.ptrace_user_x8.reset();
    // rt_sigreturn restores all registers from the guest signal frame.
    if (number != 139)
        task.cpu.x[8] = guest_x8;
    if (task.state == Task::State::Exited)
        task.signals.pending.discard(UINT64_MAX);
    if (number == 93 && task.state == Task::State::Exited) {
        if (task.tid == process.pid)
            process.leader_status = code << 8;
        const auto survivors = scheduler_.live_tids(process.pid);
        if (!survivors.empty())
            for (auto &[pid, child] : processes_) {
                (void)pid;
                if (child->parent == process.pid && child->parent_tid == task.tid)
                    child->parent_tid = survivors.front();
            }
    }
    if (!scheduler_.process_alive(process.pid) && process.state != Process::State::Zombie)
        finish_process(process, number == 94 ? code << 8 : process.leader_status.value_or(code << 8));
    if (task.state != Task::State::Exited && !task.ptrace_deferred_exit && ptrace_->traced(task.tid))
        task.ptrace_exit_pending = true;
    return true;
}

bool Machine::task_exiting(uint32_t pid, uint32_t tid, bool group, int status) {
    if (defer_ptrace_exit(pid, tid, group, status))
        return true;
    prepare_ptrace_exit(pid, tid, group, status);
    return false;
}

bool Machine::defer_ptrace_exit(uint32_t pid, uint32_t tid, bool group, int status) {
    auto *process = find_process(pid);
    if (!process)
        return false;
    if (group && process->ptrace_group_exit)
        return true;
    const auto tids = group ? scheduler_.live_tids(pid) : std::vector<uint32_t>{tid};
    bool interested = false;
    for (const auto target : tids)
        interested |= ptrace_->wants_exit_event(target);
    if (!interested)
        return false;
    ptrace_exit_deferred_ = true;
    if (group)
        process->ptrace_group_exit = status;
    for (const auto target : tids) {
        auto *task = scheduler_.lookup(target);
        if (!task || task->state == Task::State::Exited)
            continue;
        task->ptrace_deferred_exit = status;
        task->ptrace_exit_pending = false;
        if (group && task->ptrace_stopped)
            ptrace_->fatal_signal(target);
        if (ptrace_->wants_exit_event(target)) {
            if (task->ptrace_stopped)
                ptrace_->fatal_signal(target);
            if (!ptrace_->exit_event(target, status))
                throw Failure("cannot create ptrace exit event");
        }
        scheduler_.scheduling_changed(target);
    }
    return true;
}

void Machine::service_ptrace_exits() {
    if (!ptrace_exit_deferred_)
        return;
    auto exit_code = [](int status) {
        return (status & 127) ? 128 + (status & 127) : (status >> 8) & 255;
    };
    const auto retire = [&](uint32_t tid, bool group_leader_pass) {
        auto *task = scheduler_.lookup(tid);
        auto *process = task ? find_process(task->pid) : nullptr;
        if (!task || !process || !task->ptrace_deferred_exit || task->ptrace_stopped)
            return;
        const auto pid = task->pid;
        const bool group_leader = process->ptrace_group_exit && tid == pid;
        if (group_leader != group_leader_pass ||
            (group_leader && scheduler_.live_tids(pid).size() != 1))
            return;
        const int status = *task->ptrace_deferred_exit;
        Scheduler::ScopedTask context(scheduler_, tid);
        task->ptrace_deferred_exit.reset();
        prepare_ptrace_exit(pid, tid, false, status);
        process->image.syscalls->release_task_resources(*task);
        scheduler_.exit(exit_code(status), false);
        task->signals.pending.discard(UINT64_MAX);
        if (tid == pid)
            process->leader_status = status;
        const auto survivors = scheduler_.live_tids(pid);
        if (!survivors.empty()) {
            for (auto &[child_pid, child] : processes_) {
                (void)child_pid;
                if (child->parent == pid && child->parent_tid == tid)
                    child->parent_tid = survivors.front();
            }
        } else if (process->state != Process::State::Zombie) {
            const int process_status = process->ptrace_group_exit.value_or(process->leader_status.value_or(status));
            process->ptrace_group_exit.reset();
            finish_process(*process, process_status);
        }
    };
    const auto tids = scheduler_.live_tids();
    for (const auto tid : tids)
        retire(tid, false);
    // A group leader's final exit remains waitable only after its peers have
    // exited, but a stopped peer must not hold up another tracee's exit.
    for (const auto tid : tids)
        retire(tid, true);
    ptrace_exit_deferred_ = false;
    for (const auto tid : scheduler_.live_tids())
        if (scheduler_.find(tid).ptrace_deferred_exit) {
            ptrace_exit_deferred_ = true;
            break;
        }
}

void Machine::prepare_ptrace_exit(uint32_t pid, uint32_t tid, bool group, int status) {
    const auto tids = group ? scheduler_.live_tids(pid) : std::vector<uint32_t>{tid};
    for (const auto target : tids) {
        if (ptrace_->task_exit(target, status) && target == pid)
            if (auto *process = find_process(pid))
                process->trace_exit_held = true;
    }
    for (const auto target : tids)
        ptrace_->tracer_exit(target);
}

void Machine::release_ptrace_exit(uint32_t tid) {
    auto *process = find_process(tid);
    if (!process || !process->trace_exit_held)
        return;
    process->trace_exit_held = false;
    if (process->state != Process::State::Zombie)
        return;
    notify_parent(*process, process->status, process->exit_signal);
    auto *parent = find_process(process->parent);
    if ((!parent && process->pid != 1) ||
        (parent && process->exit_signal == 17 &&
         (parent->signals->actions->entries[16].handler == 1 ||
          (parent->signals->actions->entries[16].flags & signals::sa_no_cldwait)))) {
        process->reaped = true;
        process::pidfd_process_reaped(*this, process->pid);
        scheduler_.reap_process(process->pid);
    }
}
} // namespace operit::arm64
