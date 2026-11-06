#include "runtime/machine.h"
#include "linux/namespaces/namespaces.h"
#include "linux/process/pidfd.h"
#include "runtime/process/image_loader.h"
#include <algorithm>

namespace operit::arm64 {
namespace {
constexpr uint64_t clone_vm = 0x100, clone_fs = 0x200, clone_files = 0x400;
constexpr uint64_t clone_sighand = 0x800, clone_ptrace = 0x2000, clone_vfork = 0x4000, clone_parent = 0x8000;
constexpr uint64_t clone_thread = 0x10000, clone_settls = 0x80000, clone_parent_settid = 0x100000;
constexpr uint64_t clone_child_cleartid = 0x200000, clone_child_settid = 0x1000000;
constexpr uint64_t clone_clear_sighand = uint64_t{1} << 32;
void require_write(Memory &memory, uint64_t address, uint64_t length) {
    if (!memory.accessible(address, length, write_perm))
        guest_error(14);
}
void check_exec_arguments(const process::ResourceLimits &limits, const std::string &path,
                          const std::vector<std::string> &arguments,
                          const std::vector<std::string> &environment) {
    const uint64_t budget = limits.exec_argument_limit();
    uint64_t used = 0;
    auto charge = [&](uint64_t count) {
        if (count > budget - used)
            guest_error(7);
        used += count;
    };
    const uint64_t arguments_count = std::max<size_t>(arguments.size(), 1);
    if (arguments_count > budget / 8 || environment.size() > budget / 8 - arguments_count)
        guest_error(7);
    charge((arguments_count + environment.size()) * 8);
    auto string = [&](const std::string &value) {
        if (value.size() >= 32 * page_size)
            guest_error(7);
        charge(value.size() + 1);
    };
    string(path);
    if (arguments.empty())
        charge(1);
    for (const auto &argument : arguments)
        string(argument);
    for (const auto &entry : environment)
        string(entry);
}
void limit_exec_image(ProcessImage &image, const process::ResourceLimits &limits) {
    auto &memory = *image.memory;
    const auto mappings = memory.mappings();
    const auto stack = std::find_if(mappings.begin(), mappings.end(), [&](const Memory::Mapping &mapping) {
        return mapping.backing.path == "[stack]" && mapping.start <= image.entry.sp &&
               image.entry.sp < mapping.end;
    });
    if (stack == mappings.end())
        guest_error(8);
    const uint64_t top = stack->end;
    uint64_t cursor = image.entry.sp;
    auto word = [&]() {
        if (cursor > top - 8)
            guest_error(8);
        const uint64_t value = memory.load(cursor, 8);
        cursor += 8;
        return value;
    };
    uint64_t strings_begin = top;
    auto string = [&](uint64_t address) {
        if (address < stack->start || address >= top)
            guest_error(8);
        strings_begin = std::min(strings_begin, address);
    };
    const uint64_t argc = word();
    if (argc > 4096)
        guest_error(7);
    for (uint64_t index = 0; index < argc; ++index)
        string(word());
    if (word())
        guest_error(8);
    uint64_t envc = 0;
    for (uint64_t address; (address = word());) {
        if (++envc > 4096)
            guest_error(7);
        string(address);
    }
    bool have_auxv_end = false;
    for (unsigned index = 0; index < 64; ++index) {
        const uint64_t type = word(), value = word();
        if (!type) {
            have_auxv_end = true;
            break;
        }
        if (type == 31) // AT_EXECFN is part of the copied string budget.
            string(value);
    }
    if (!have_auxv_end)
        guest_error(8);
    // The loader stores argv/env and AT_EXECFN consecutively up to stack top.
    // Include pointer-table cost, but do not charge auxv/random/padding to ARG_MAX.
    limits.check_exec_arguments(top - strings_begin + (std::max<uint64_t>(argc, 1) + envc) * 8);
    const uint64_t capacity = std::min(top - stack->start,
                                      limits.get(process::Resource::Stack).soft) & ~(page_size - 1);
    if (!capacity || top - image.entry.sp > capacity)
        guest_error(7);
    if (top - stack->start > capacity)
        memory.unmap(stack->start, top - stack->start - capacity);
    // Validate only the candidate image. Failure leaves the running image and
    // descriptor table intact, including when the inherited AS limit was lowered.
    limits.check_exec_image(memory);
}
} // namespace
uint32_t Machine::clone(const CpuState &cpu, const ProcessClone &request) {
    namespaces::check_clone(request.flags, false); // Guest capabilities are empty.
    constexpr uint64_t allowed = clone_vm | clone_fs | clone_files | clone_sighand | clone_ptrace | clone_vfork |
                                 clone_parent | clone_settls | clone_parent_settid | clone_child_cleartid |
                                 clone_child_settid | clone_clear_sighand | clone_thread | 0x40000 |
                                 0x400000 | 0x800000;
    if (request.flags & ~allowed)
        guest_error(22);
    if (request.exit_signal > 64 || ((request.flags & clone_sighand) && !(request.flags & clone_vm)) ||
        ((request.flags & clone_sighand) && (request.flags & clone_clear_sighand)))
        guest_error(22);
    auto &parent = current_process();
    const auto credentials = process::pidfd_credentials(*this, parent.pid);
    const auto inherit_semaphore_task = [this, &request](Task &child) {
        if (request.flags & 0x40000) { // CLONE_SYSVSEM
            auto &parent_task = scheduler_.current();
            if (!parent_task.semaphore_task)
                parent_task.semaphore_task = ipc::make_semaphore_task();
            child.semaphore_task = ipc::clone_semaphore_task(*parent_task.semaphore_task, true);
        }
    };
    if (request.flags & clone_thread) {
        constexpr uint64_t required = clone_vm | clone_fs | clone_files | clone_sighand;
        if ((request.flags & required) != required || (request.flags & (clone_vfork | clone_clear_sighand)) ||
            request.exit_signal)
            guest_error(22);
        if (!request.stack || (request.stack & 15))
            guest_error(22);
        if (request.flags & clone_parent_settid)
            require_write(*parent.image.memory, request.parent_tid, 4);
        if (request.flags & clone_child_settid)
            require_write(*parent.image.memory, request.child_tid, 4);
        auto task_limit_charge =
            uid_limits_.reserve_task(credentials.real_uid, *parent.image.syscalls->resource_limits());
        process::pidfd_clear_nproc_check(*this, parent.pid);
        const auto tid =
            scheduler_.spawn(cpu, request.stack, (request.flags & clone_settls) ? request.tls : cpu.tls,
                             (request.flags & clone_child_cleartid) ? request.child_tid : 0);
        try {
            scheduler_.find(tid).task_limit_charge = std::move(task_limit_charge);
            inherit_semaphore_task(scheduler_.find(tid));
            if (request.flags & clone_parent_settid)
                parent.image.memory->store(request.parent_tid, tid, 4);
            if (request.flags & clone_child_settid)
                parent.image.memory->store(request.child_tid, tid, 4);
        } catch (...) {
            scheduler_.discard(tid);
            throw;
        }
        ptrace_->clone(scheduler_.current_tid(), tid, request.flags, request.exit_signal);
        return tid;
    }
    if ((request.flags & clone_parent) && parent.pid == 1)
        guest_error(22);
    if (request.stack && (request.stack & 15))
        guest_error(22);
    if (request.flags & clone_parent_settid)
        require_write(*parent.image.memory, request.parent_tid, 4);
    // CLONE_PARENT changes the child's parent, not its inherited credentials.
    // CLONE_THREAD returned above and retains the existing process identity.
    auto task_limit_charge =
        uid_limits_.reserve_task(credentials.real_uid, *parent.image.syscalls->resource_limits());
    process::pidfd_clear_nproc_check(*this, parent.pid);
    auto child = std::make_unique<Process>();
    child->parent = (request.flags & clone_parent) ? parent.parent : parent.pid;
    child->parent_tid = (request.flags & clone_parent) ? parent.parent_tid : scheduler_.current().tid;
    child->group = parent.group;
    child->session = parent.session;
    child->exit_signal = request.exit_signal;
    child->signals = signals::fork_process(*parent.signals, request.flags & clone_sighand);
    if (request.flags & clone_clear_sighand)
        for (auto &action : child->signals->actions->entries)
            if (action.handler != 1)
                action = {};
    auto &image = child->image;
    parent.image.syscalls->sync_shared_memory_mappings();
    image.memory = (request.flags & clone_vm) ? parent.image.memory
                                              : std::shared_ptr<Memory>(parent.image.memory->fork_clone());
    image.filesystem = std::make_unique<Filesystem>(*parent.image.filesystem);
    image.acceleration = std::make_unique<FixedComponents>(*parent.image.acceleration);
    image.acceleration->calls = image.acceleration->bytes = 0;
    image.executor = std::make_unique<ExecutionEngine>();
    image.executable = parent.image.executable;
    image.heap_end = parent.image.heap_end;
    image.signal_trampoline = parent.image.signal_trampoline;
    image.syscalls = std::make_unique<Syscalls>(*image.memory, *image.filesystem, config_, scheduler_);
    image.syscalls->set_io_uring_context(io_uring_context_, image.memory);
    image.syscalls->inherit_process(*parent.image.syscalls, request.flags);
    if (request.flags & clone_child_settid)
        require_write(*image.memory, request.child_tid, 4);
    const auto tid = scheduler_.spawn_process(cpu, request.stack ? request.stack : cpu.sp,
                                              (request.flags & clone_settls) ? request.tls : cpu.tls,
                                              (request.flags & clone_child_cleartid) ? request.child_tid : 0);
    child->pid = tid;
    try {
        auto &task = scheduler_.find(tid);
        task.task_limit_charge = std::move(task_limit_charge);
        inherit_semaphore_task(task);
        task.rseq = threads::rseq_fork(scheduler_.current().rseq, request.flags & clone_vm);
        image.syscalls->inherit_shared_memory_space(*parent.image.syscalls, request.flags & clone_vm, tid);
        image.syscalls->set_process_identity(tid, child->parent);
        task.signals = signals::fork_thread(scheduler_.current().signals, request.flags & clone_vm,
                                            request.flags & clone_vfork);
        if (request.flags & clone_parent_settid)
            parent.image.memory->store(request.parent_tid, tid, 4);
        if (request.flags & clone_child_settid)
            image.memory->store(request.child_tid, tid, 4);
        image.syscalls->terminal_manager()->process_forked(tid, parent.pid);
        process::pidfd_process_created(*this, tid, credentials);
        processes_.emplace(tid, std::move(child));
    } catch (...) {
        process::pidfd_process_reaped(*this, tid);
        scheduler_.discard(tid);
        throw;
    }
    if (request.flags & clone_vfork) {
        processes_.at(tid)->vfork_parent = scheduler_.current().tid;
        scheduler_.current().vfork_child = tid;
        scheduler_.block({}, tid);
    }
    ptrace_->clone(scheduler_.current_tid(), tid, request.flags, request.exit_signal);
    return tid;
}
void Machine::exec(const std::string &path, const std::vector<std::string> &arguments,
                   const std::vector<std::string> &environment, const std::string &working_directory) {
    auto &process = current_process();
    if (!ptrace_->can_exec(scheduler_.current().tid, process.pid))
        guest_error(95);
    for (const auto tid : scheduler_.live_tids(process.pid))
        if (tid != scheduler_.current_tid() && ptrace_->wants_exit_event(tid))
            guest_error(95); // Retiring traced sibling threads during exec needs its own event barrier.
    const auto limits = process.image.syscalls->resource_limits();
    if (process::pidfd_nproc_check_pending(*this, process.pid)) {
        const auto credentials = process::pidfd_credentials(*this, process.pid);
        if (uid_limits_.task_limit_exceeded(credentials.real_uid, *limits))
            guest_error(11);
        process::pidfd_clear_nproc_check(*this, process.pid);
    }
    check_exec_arguments(*limits, path, arguments, environment);
    auto replacement = std::make_unique<ProcessImage>();
    replacement->memory = std::make_shared<Memory>(process.image.memory->limit());
    replacement->filesystem = std::make_unique<Filesystem>(*process.image.filesystem);
    replacement->acceleration = std::make_unique<FixedComponents>(config_.enable_fixed_acceleration != 0);
    replacement->executor = std::make_unique<ExecutionEngine>();
    load_process_executable(*replacement, path, arguments, environment, working_directory);
    limit_exec_image(*replacement, *limits);
    replacement->syscalls =
        std::make_unique<Syscalls>(*replacement->memory, *replacement->filesystem, config_, scheduler_);
    replacement->syscalls->set_io_uring_context(io_uring_context_, replacement->memory);
    replacement->syscalls->inherit_process(*process.image.syscalls, 0, true);
    replacement->syscalls->set_process_identity(process.pid, process.parent);
    auto &task = scheduler_.current();
    auto previous_name = std::move(task.name);
    try {
        replacement->syscalls->exec_reset(replacement->executable, replacement->entry.sp,
                                          replacement->heap_end);
    } catch (...) {
        task.name = std::move(previous_name);
        throw;
    }
    task.name = std::move(previous_name);
    process.pending_exec = std::move(replacement);
}
void Machine::archive_image(Process &process) {
    auto &image = process.image;
    if (image.executor) {
        process.instructions += image.executor->executed_instructions;
        process.decoded_blocks += image.executor->decoded_blocks;
        process.cache_hits += image.executor->cache_hits;
    }
    if (image.syscalls) {
        process.calls += image.syscalls->calls;
        process.unsupported += image.syscalls->unsupported_count;
        if (image.syscalls->last_unsupported)
            process.last_unsupported = image.syscalls->last_unsupported;
    }
    if (image.acceleration) {
        process.acceleration_calls += image.acceleration->calls;
        process.acceleration_bytes += image.acceleration->bytes;
    }
}
void Machine::release_tasks(Process &process, bool preserve_current_sem_undo) {
    for (auto tid : scheduler_.live_tids(process.pid))
        process.image.syscalls->release_task_resources(scheduler_.find(tid),
                                                       preserve_current_sem_undo && tid == scheduler_.current_tid());
}
void Machine::release_vfork(Process &process) {
    if (process.vfork_parent) {
        ptrace_->vfork_done(process.vfork_parent, process.pid);
        if (auto *parent = scheduler_.lookup(process.vfork_parent))
            parent->vfork_child = 0;
        scheduler_.wake(process.vfork_parent, process.pid);
        ptrace_->preserve_event_syscall_result(process.vfork_parent);
        process.vfork_parent = 0;
    }
}
void Machine::commit_exec(Process &process) {
    if (!process.pending_exec)
        return;
    // Settle both serial execution and syscall CPU time before exec rekeys a
    // nonleader TID or removes any task carrying its accumulated counters.
    stop_accounting();
    process.image.syscalls->commit_ipc_exec();
    const auto exec_tid = scheduler_.current_tid();
    for (const auto tid : scheduler_.live_tids(process.pid))
        if (tid != exec_tid)
            prepare_ptrace_exit(process.pid, tid, false, 0);
    release_tasks(process, true);
    // Drop all retired peers' signals while the old timer bookkeeping
    // exists. The surviving task and process retain ordinary pending signals,
    // including ITIMER_REAL's SI_KERNEL notifications, across exec.
    const auto old_timer = [](const signals::Info &info) { return info.code == -2; };
    process.signals->pending.discard_if(old_timer);
    const auto survivor_tid = scheduler_.current().tid;
    for (const auto tid : scheduler_.task_ids(process.pid)) {
        auto &pending = scheduler_.lookup(tid)->signals.pending;
        if (tid == survivor_tid)
            pending.discard_if(old_timer);
        else
            pending.discard(UINT64_MAX);
    }
    if (process.timers)
        process.timers->exec_reset();
    process.image.syscalls->release_shared_memory_space(process.pid);
    archive_image(process);
    auto survivor_signals = std::move(scheduler_.current().signals);
    // exec unshares the disposition table even when CLONE_SIGHAND shared it.
    auto fresh_signals = signals::fork_process(*process.signals);
    fresh_signals->pending = std::move(process.signals->pending);
    signals::exec_reset(*fresh_signals, survivor_signals);
    process.signals = std::move(fresh_signals);
    scheduler_.replace_process_image(process.pid, process.pending_exec->entry);
    threads::rseq_reset(scheduler_.current().rseq);
    scheduler_.current().signals = std::move(survivor_signals);
    process.image = std::move(*process.pending_exec);
    process.pending_exec.reset();
    ptrace_->exec(exec_tid, process.pid);
    // This runtime does not grant set-ID/file capabilities on exec. A normal
    // exec restores dumpability while retaining the original pidfd identity.
    auto credentials = process::pidfd_credentials(*this, process.pid);
    credentials.dumpable = true;
    process::pidfd_update_credentials(*this, process.pid, credentials);
    process.execed = true;
    scheduler_.current().name =
        process.image.executable.substr(process.image.executable.find_last_of('/') + 1, 15);
    release_vfork(process);
    for (auto &[pid, child] : processes_) {
        (void)pid;
        if (child->parent == process.pid)
            child->parent_tid = process.pid;
    }
    if (exec_event)
        exec_event(process.pid);
}
void Machine::notify_parent(Process &process, int status, unsigned signal) {
    auto *parent = find_process(process.parent);
    if (!parent || !signal)
        return;
    const auto &action = parent->signals->actions->entries[16];
    const bool stopped = (status & 0xff) == 0x7f || status == 0xffff;
    if (!(stopped && (action.flags & signals::sa_no_cldstop))) {
        auto hooks = signal_hooks();
        int code = status == 0xffff          ? 6
                   : (status & 0xff) == 0x7f ? 5
                   : (status & 0x7f)         ? ((status & 0x80) ? 3 : 2)
                                             : 1;
        int value = (status == 0xffff)        ? 18
                    : (status & 0xff) == 0x7f ? ((status >> 8) & 255)
                    : (status & 0x7f)         ? (status & 0x7f)
                                              : ((status >> 8) & 255);
        const auto credentials = process::pidfd_credentials(*this, process.pid);
        auto info = signals::Info::child(process.pid, credentials.real_uid, value, code);
        info.signal = signal;
        signals::send_process(hooks, signal_process(*parent), info);
    }
    if (child_event)
        child_event(parent->pid, process.pid, signal, status);
}
void Machine::finish_process(Process &process, int status) {
    if (process.state == Process::State::Zombie)
        return;
    if (accounting_pid_ == process.pid)
        stop_accounting();
    discard_pending_signals(process);
    if (process.timers)
        process.timers->clear();
    process.state = Process::State::Zombie;
    process.status = status;
    process.maximum_rss = std::max(process.maximum_rss, process.image.memory->resident());
    process.image.syscalls->terminal_manager()->process_exited(process.pid);
    process.image.syscalls->release_shared_memory_space(process.pid);
    process.image.syscalls->close_descriptors();
    process::pidfd_process_exited(*this, process.pid);
    drain_terminal_signals(process);
    release_vfork(process);
    auto auto_reap = [this](Process &child) {
        auto *parent = find_process(child.parent);
        if (!parent) {
            if (child.pid != 1)
                child.reaped = true;
        } else if (child.exit_signal == 17) {
            const auto &action = parent->signals->actions->entries[16];
            if (action.handler == 1 || (action.flags & signals::sa_no_cldwait))
                child.reaped = true;
        }
        if (child.reaped) {
            process::pidfd_process_reaped(*this, child.pid);
            scheduler_.reap_process(child.pid);
        }
    };
    for (auto &[pid, child] : processes_) {
        (void)pid;
        if (!child->reaped && child->parent == process.pid) {
            child->parent = process.pid == 1 ? 0 : 1;
            child->parent_tid = child->parent;
            child->image.syscalls->set_process_identity(child->pid, child->parent);
            if (child->state == Process::State::Zombie && !child->trace_exit_held) {
                if (child->parent)
                    notify_parent(*child, child->status, child->exit_signal);
                auto_reap(*child);
            }
        }
    }
    if (!process.trace_exit_held) {
        notify_parent(process, status, process.exit_signal);
        auto_reap(process);
    }
    if (process.pid == 1) {
        main_exited_ = true;
        main_exit_code_ = (status & 0x7f) ? 128 + (status & 0x7f) : (status >> 8) & 255;
        for (auto &[pid, child] : processes_) {
            if (pid == 1)
                continue;
            if (child->state != Process::State::Zombie) {
                prepare_ptrace_exit(pid, 0, true, 9);
                release_tasks(*child);
                scheduler_.exit_process(pid, 137);
                discard_pending_signals(*child);
                if (child->timers)
                    child->timers->clear();
                child->state = Process::State::Zombie;
                child->status = 9;
                child->image.syscalls->terminal_manager()->process_exited(pid);
                child->image.syscalls->release_shared_memory_space(pid);
                child->image.syscalls->close_descriptors();
                process::pidfd_process_exited(*this, pid);
            }
            // No guest reaper remains when PID 1 exits, including for zombies
            // that were already waiting before namespace shutdown.
            child->reaped = true;
            process::pidfd_process_reaped(*this, pid);
            scheduler_.reap_process(pid);
        }
    }
}
void Machine::terminate(uint32_t pid, unsigned signal, bool core) {
    auto *process = find_process(pid);
    if (!process)
        guest_error(3);
    if (process->state == Process::State::Zombie)
        return;
    const int status = (signal & 127) | (core ? 128 : 0);
    if (defer_ptrace_exit(pid, 0, true, status))
        return;
    prepare_ptrace_exit(pid, 0, true, status);
    release_tasks(*process);
    scheduler_.exit_process(pid, 128 + signal);
    finish_process(*process, status);
}
} // namespace operit::arm64
