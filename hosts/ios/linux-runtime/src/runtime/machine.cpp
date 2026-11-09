#include "runtime/machine.h"
#include "cpu/aarch64/instruction_fault.h"
#include "linux/files/record_lock.h"
#include "linux/io_uring/io_uring.h"
#include "linux/process/pidfd.h"
#include "runtime/process/image_loader.h"
#include <algorithm>
#include <cerrno>
#include <cstdio>
#include <unistd.h>
#include <thread>

namespace operit::arm64 {
Machine::Machine(const ort_arm64_config &config) : config_(config) {
    io_uring_context_ = io_uring::make_completion_context();
    scheduler_.set_process_controller(this);
    const auto available = std::max(1u, std::thread::hardware_concurrency());
    execution_threads_ = config.execution_threads ? config.execution_threads : std::min(4u, available);
    if (execution_threads_ > 64)
        throw Failure("execution_threads must be in 1..64, or zero for automatic selection");
    scheduler_.set_execution_cpus(execution_threads_);
    auto process = std::make_unique<Process>();
    auto &image = process->image;
    image.memory = std::make_shared<Memory>(config.resident_memory_limit ? config.resident_memory_limit
                                                                         : default_memory_limit);
    image.filesystem = std::make_unique<Filesystem>(config.root_directory);
    image.acceleration = std::make_unique<FixedComponents>(config.enable_fixed_acceleration != 0);
    image.executor = std::make_unique<ExecutionEngine>();
    image.syscalls = std::make_unique<Syscalls>(*image.memory, *image.filesystem, config, scheduler_);
    image.syscalls->set_io_uring_context(io_uring_context_, image.memory);
    image.syscalls->set_process_identity(1, 0);
    processes_.emplace(1, std::move(process));
    configure_terminal(*processes_.at(1));
    process::PidfdCredentials credentials;
    credentials.real_uid = ::getuid();
    credentials.effective_uid = credentials.saved_uid = ::geteuid();
    credentials.real_gid = ::getgid();
    credentials.effective_gid = credentials.saved_gid = ::getegid();
    process::pidfd_process_created(*this, 1, credentials);
    scheduler_.current().task_limit_charge = uid_limits_.track_task(credentials.real_uid);
    initialize_ptrace();
    bind_runtime_signal_queues();
}
Machine::~Machine() {
    executor_.reset();
    process::pidfd_vm_exit(*this);
    files::record_lock_vm_exit(scheduler_);
    // Descriptors can outlive their process through shared open descriptions.
    // Do not let final PTY closure call back into a partially destroyed VM.
    for (auto &[pid, process] : processes_) {
        (void)pid;
        process->image.syscalls->terminal_manager()->set_hooks({});
    }
}
Process &Machine::current_process() {
    return *processes_.at(scheduler_.current().pid);
}
Process *Machine::find_process(uint32_t pid) {
    auto it = processes_.find(pid ? pid : scheduler_.current().pid);
    return it == processes_.end() || it->second->reaped ? nullptr : it->second.get();
}
Syscalls *Machine::process_syscalls(uint32_t pid) {
    auto *process = find_process(pid);
    return process ? process->image.syscalls.get() : nullptr;
}
const Process &Machine::require_process(uint32_t pid) const {
    auto it = processes_.find(pid ? pid : scheduler_.current().pid);
    if (it == processes_.end() || it->second->reaped)
        guest_error(3);
    return *it->second;
}
int Machine::bind(const char *source, const char *target, uint32_t flags) {
    try {
        if (load_attempted_)
            throw Failure("bind mounts must be registered before loading a guest");
        if (!source || !target || (flags & ~ORT_ARM64_BIND_READ_ONLY))
            guest_error(22);
        processes_.at(1)->image.filesystem->bind(source, target, flags & ORT_ARM64_BIND_READ_ONLY);
        error.clear();
        return 0;
    } catch (const GuestError &failure) {
        error = "bind failed with Linux errno " + std::to_string(failure.number);
    } catch (const std::exception &failure) {
        error = failure.what();
    }
    return -1;
}
int Machine::load(const char *path, const char *const *argv, const char *const *envp) {
    try {
        if (load_attempted_)
            throw Failure("load is allowed once per VM; create another VM to retry");
        load_attempted_ = true;
        if (!path || path[0] != '/' || !argv || !argv[0])
            throw Failure("load needs a guest absolute path and argv[0]");
        std::vector<std::string> arguments, environment;
        for (size_t j = 0; argv && argv[j]; ++j) {
            if (j == 4096)
                guest_error(7);
            arguments.emplace_back(argv[j]);
        }
        for (size_t j = 0; envp && envp[j]; ++j) {
            if (j == 4096)
                guest_error(7);
            environment.emplace_back(envp[j]);
        }
        auto &image = processes_.at(1)->image;
        load_process_executable(image, path, arguments, environment, "/");
        scheduler_.current().cpu = image.entry;
        image.syscalls->set_heap(image.heap_end);
        image.syscalls->set_process_image(image.executable, image.entry.sp);
        loaded_ = true;
        error.clear();
        return 0;
    } catch (const std::exception &failure) {
        error = failure.what();
    } catch (const GuestError &failure) {
        error = "ELF load failed with Linux errno " + std::to_string(failure.number);
    }
    return -1;
}
ort_arm64_result Machine::run(uint64_t budget) {
    ort_arm64_status status = ORT_ARM64_BUDGET;
    try {
        if (!loaded_)
            throw Failure("no executable loaded");
        started_ = true;
        if (main_exited_)
            status = ORT_ARM64_EXITED;
        else if (failed_)
            status = failure_status_;
        else {
            while (budget && !main_exited_) {
                poll_runtime_events();
                if (schedule_pending_) {
                    // RR/multi-task execution must settle before selection.
                    if (!can_defer_accounting())
                        stop_accounting();
                    if (!scheduler_.select(false)) {
                        if (!scheduler_.exited())
                            status = ORT_ARM64_WAITING;
                        break;
                    }
                    schedule_pending_ = false;
                    collect_reaped();
                }
                if (run_parallel(budget))
                    continue;
                auto &process = current_process();
                begin_accounting(process.pid);
                auto &task = scheduler_.current();
                const uint64_t available = execution_threads_ == 64 ? UINT64_MAX :
                                           (uint64_t{1} << execution_threads_) - 1;
                const auto cpu = uint32_t(__builtin_ctzll(task.cpu_affinity & available));
                auto &image = process.image;
                image.memory->set_parallel_access(false);
                image.memory->synchronize_shared(true);
                const bool in_traced_syscall = task.ptrace_syscall || task.ptrace_exit_pending;
                const bool fatal_pending = signals::pending_mask(*process.signals, task.signals) &
                                           signals::bit(signals::kill_signal);
                signals::Delivery delivery;
                if (!in_traced_syscall || fatal_pending)
                    delivery = deliver_signals(task, process, cpu);
                if (task.ptrace_stopped) {
                    schedule_pending_ = true;
                    quantum_remaining_ = 262144;
                    continue;
                }
                if (delivery.effect != signals::Effect::None && delivery.effect != signals::Effect::Handler) {
                    auto hooks = signal_hooks();
                    hooks.effect(process.pid, delivery);
                    if (task.state != Task::State::Runnable || task.stopped || task.ptrace_deferred_exit ||
                        main_exited_) {
                        schedule_pending_ = !main_exited_;
                        quantum_remaining_ = 262144;
                        continue;
                    }
                }
                if (task.ptrace_exit_pending) {
                    wait_completed(task.tid);
                    if (task.ptrace_stopped) {
                        schedule_pending_ = true;
                        quantum_remaining_ = 262144;
                        continue;
                    }
                }
                const bool resumed_syscall = bool(task.ptrace_syscall);
                const auto before = image.executor->executed_instructions;
                const auto queue_fault = [&](int number, int code, uint64_t address, bool executed) {
                    const auto completed = image.executor->executed_instructions - before;
                    const auto charged = std::min(budget, completed + 1);
                    budget -= charged;
                    quantum_remaining_ -= std::min(quantum_remaining_, charged);
                    if (executed)
                        ++image.executor->executed_instructions;
                    task.cpu.exclusive_valid = false;
                    signals::force_fault(*process.signals, task.signals, task.signal_mask, number, address,
                                         code);
                    if (!quantum_remaining_) {
                        schedule_pending_ = true;
                        quantum_remaining_ = 262144;
                    }
                };
                Execution execution{Trap::Syscall, 0};
                try {
                    // A sole ordinary task has no competing quantum to rotate.
                    // Let its resident WebKit execution consume a large run;
                    // timers, RT policies and multiple tasks keep their quantum.
                    if (wasm_webkit_available() && can_defer_accounting())
                        quantum_remaining_=std::min<uint64_t>(budget,1ull<<28);
                    if (!resumed_syscall)
                        execution = image.executor->run(task.cpu, *image.memory,
                                                           std::min(budget, quantum_remaining_),
                                                           image.acceleration.get());
                } catch (const InstructionFault &fault) {
                    threads::rseq_event(task.rseq, threads::RseqEvent::Preempt);
                    const bool breakpoint = fault.kind == InstructionFault::Kind::Breakpoint;
                    if (config_.trace_syscalls)
                        std::fprintf(stderr,
                                     "guest instruction fault pid=%u tid=%u pc=%#llx address=%#llx "
                                     "instruction=%#010x kind=%s signal=%d code=1 image=%s\n",
                                     process.pid, task.tid, static_cast<unsigned long long>(fault.pc),
                                     static_cast<unsigned long long>(fault.pc), fault.instruction,
                                     breakpoint                                        ? "BRK"
                                     : fault.kind == InstructionFault::Kind::Undefined ? "UDF"
                                     : fault.kind == InstructionFault::Kind::Halt      ? "HLT"
                                                                                       : "privileged",
                                     breakpoint ? 5 : 4, image.executable.c_str());
                    queue_fault(breakpoint ? 5 : 4, 1, fault.pc, true);
                    continue;
                } catch (const MemoryFault &fault) {
                    threads::rseq_event(task.rseq, threads::RseqEvent::Preempt);
                    const int number = fault.kind == MemoryFault::Kind::Bus ? 7 : 11;
                    const int code = fault.kind == MemoryFault::Kind::Unmapped ? 1 : 2;
                    if (config_.trace_syscalls)
                        std::fprintf(stderr,
                                     "guest memory fault pid=%u tid=%u pc=%#llx address=%#llx "
                                     "permissions=%u kind=%s signal=%d code=%d image=%s\n",
                                     process.pid, task.tid, static_cast<unsigned long long>(task.cpu.pc),
                                     static_cast<unsigned long long>(fault.address), fault.permissions,
                                     fault.kind == MemoryFault::Kind::Unmapped     ? "unmapped"
                                     : fault.kind == MemoryFault::Kind::Protection ? "protection"
                                                                                   : "bus",
                                     number, code, image.executable.c_str());
                    queue_fault(number, code, fault.address, false);
                    continue;
                }
                threads::rseq_event(task.rseq, threads::RseqEvent::Preempt);
                budget -= execution.instructions;
                quantum_remaining_ -= execution.instructions;
                if (execution.trap == Trap::Syscall) {
                    // Charge instructions under the old scheduling policy before
                    // a syscall can change it; syscall CPU time starts anew.
                    stop_accounting();
                    begin_accounting(process.pid);
                    accounting_syscall_ = true;
                    if (!dispatch_guest_syscall(task, process, resumed_syscall)) {
                        schedule_pending_ = true;
                        quantum_remaining_ = 262144;
                        continue;
                    }
                }
                process.maximum_rss = std::max(process.maximum_rss, image.memory->resident());
                if (process.pending_exec)
                    commit_exec(process);
                if (execution.trap == Trap::Syscall) {
                    auto &completed = scheduler_.current();
                    if (completed.state == Task::State::Runnable && !completed.ptrace_stopped)
                        wait_completed(completed.tid);
                    drain_terminal_signals(process);
                }
                if (execution.trap == Trap::Syscall || !quantum_remaining_) {
                    schedule_pending_ = !main_exited_;
                    quantum_remaining_ = 262144;
                    if (execution.trap == Trap::Syscall) {
                        ++process.voluntary_switches;
                        ++scheduler_.current().voluntary_switches;
                    } else {
                        ++process.involuntary_switches;
                        ++scheduler_.current().involuntary_switches;
                    }
                }
            }
            if (main_exited_)
                status = ORT_ARM64_EXITED;
        }
    } catch (const Failure &failure) {
        error = std::string(failure.what()) + " at PC " + hex(scheduler_.current().cpu.pc);
        failed_ = true;
        status = failure_status_ = failure.status;
    } catch (const GuestError &failure) {
        error = "unhandled Linux errno " + std::to_string(failure.number) + " at PC " +
                hex(scheduler_.current().cpu.pc);
        failed_ = true;
        status = failure_status_ = ORT_ARM64_FAULT;
    } catch (const std::exception &failure) {
        error = failure.what();
        failed_ = true;
        status = failure_status_ = ORT_ARM64_FAULT;
    }
    try {
        stop_accounting();
    } catch (const std::exception &failure) {
        error = failure.what();
        failed_ = true;
        status = failure_status_ = ORT_ARM64_FAULT;
        accounting_pid_ = accounting_tid_ = 0;
    }
    // WAITING, EXITED and failure returns may never enter another parallel
    // batch. Release retired worker images before handing control to the host.
    collect_thread_executions();
    auto result = retired_;
    result.status = status;
    result.execution_threads = execution_threads_;
    result.maximum_parallel_tasks = maximum_parallel_tasks_;
    result.exit_code = main_exit_code_;
    result.pc = scheduler_.current().cpu.pc;
    for (const auto &[pid, process] : processes_) {
        (void)pid;
        const auto &image = process->image;
        result.instructions += process->instructions + image.executor->executed_instructions;
        result.decoded_blocks += process->decoded_blocks + image.executor->decoded_blocks;
        result.block_cache_hits += process->cache_hits + image.executor->cache_hits;
        result.resident_bytes += image.memory->resident();
        result.syscalls += process->calls + image.syscalls->calls;
        result.unsupported_syscalls += process->unsupported + image.syscalls->unsupported_count;
        if (image.syscalls->last_unsupported)
            result.last_unsupported_syscall = image.syscalls->last_unsupported;
        else if (process->last_unsupported)
            result.last_unsupported_syscall = process->last_unsupported;
        result.accelerated_calls += process->acceleration_calls + image.acceleration->calls;
        result.accelerated_bytes += process->acceleration_bytes + image.acceleration->bytes;
    }
    return result;
}
} // namespace operit::arm64
