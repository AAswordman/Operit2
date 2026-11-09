#include "runtime/machine.h"
#include "cpu/aarch64/instruction_fault.h"
#include <algorithm>
#include <exception>
#include <sys/resource.h>
#if defined(__APPLE__)
#include <mach/mach.h>
#include <pthread.h>
#endif

namespace operit::arm64 {
namespace {
CpuUsage worker_cpu_time() {
#if defined(__APPLE__)
    thread_basic_info_data_t info{};
    mach_msg_type_number_t count = THREAD_BASIC_INFO_COUNT;
    if (thread_info(pthread_mach_thread_np(pthread_self()), THREAD_BASIC_INFO,
                    reinterpret_cast<thread_info_t>(&info), &count) != KERN_SUCCESS)
        throw Failure("cannot measure guest worker CPU time");
    const uint64_t user = uint64_t(info.user_time.seconds) * 1000000000 +
                          uint64_t(info.user_time.microseconds) * 1000;
    const uint64_t system = uint64_t(info.system_time.seconds) * 1000000000 +
                            uint64_t(info.system_time.microseconds) * 1000;
#else
    rusage usage{};
    if (::getrusage(RUSAGE_THREAD, &usage))
        throw Failure("cannot measure guest worker CPU time");
    const uint64_t user = uint64_t(usage.ru_utime.tv_sec) * 1000000000 +
                          uint64_t(usage.ru_utime.tv_usec) * 1000;
    const uint64_t system = uint64_t(usage.ru_stime.tv_sec) * 1000000000 +
                            uint64_t(usage.ru_stime.tv_usec) * 1000;
#endif
    return {user, system, user + system};
}
struct BatchTask {
    uint32_t tid = 0, pid = 0, cpu = 0;
    uint64_t budget = 0, instructions = 0, blocks = 0, hits = 0, calls = 0, bytes = 0;
    CpuUsage usage;
    Execution execution{Trap::Budget, 0};
    ExecutionEngine *executor = nullptr;
    FixedComponents *acceleration = nullptr;
    Memory *memory = nullptr;
    Task *task = nullptr;
    std::exception_ptr failure;
};
} // namespace

void Machine::collect_thread_executions() {
    // Call only between execution batches, after every worker has joined.
    // Cached engines own address spaces and their file mappings. An exited
    // task (or an image replaced by exec) must not keep those handles alive
    // until another parallel batch happens to run.
    for (auto it = thread_executions_.begin(); it != thread_executions_.end();) {
        const auto *task = scheduler_.lookup(it->first);
        const auto *process = task ? find_process(task->pid) : nullptr;
        if (!process || task->state == Task::State::Exited ||
            process->state != Process::State::Running ||
            it->second.memory != process->image.memory ||
            it->second.source != process->image.acceleration.get())
            it = thread_executions_.erase(it);
        else
            ++it;
    }
}
bool Machine::run_parallel(uint64_t &budget) {
    // Ptrace operates on coordinated CPU and memory snapshots. Keep traced VMs
    // serial until the parallel executor can stop individual jobs at a trap.
    if (execution_threads_ < 2 || budget < 8192 || !ptrace_->empty())
        return false;
    if (scheduler_.sole_ordinary_task())
        return false;
    const auto selected = scheduler_.runnable_batch(execution_threads_, 2);
    if (selected.size() < 2)
        return false;
    stop_accounting();
    for (const auto [tid, cpu] : selected) {
        (void)cpu;
        const auto &task = scheduler_.find(tid);
        if (task.ptrace_syscall || task.ptrace_exit_pending)
            return false;
    }
    // No guest CPU is executing while access mode changes. Include aliases in
    // other address spaces before starting any worker in this batch.
    for (auto &[pid, process] : processes_) {
        (void)pid;
        if (process->state == Process::State::Running)
            process->image.memory->set_parallel_access(true);
    }
    std::vector<BatchTask> batch;
    batch.reserve(selected.size());
    for (const auto [tid, cpu] : selected) {
        Scheduler::ScopedTask context(scheduler_, tid);
        auto &task = scheduler_.current();
        auto *process = find_process(task.pid);
        if (!process || process->state != Process::State::Running)
            continue;
        auto &image = process->image;
        image.memory->synchronize_shared();
        const auto delivery = deliver_signals(task, *process, cpu);
        if (delivery.effect != signals::Effect::None && delivery.effect != signals::Effect::Handler)
            signal_hooks().effect(process->pid, delivery);
        if (task.state != Task::State::Runnable || task.stopped || task.ptrace_stopped ||
            task.ptrace_deferred_exit || main_exited_)
            continue;
        auto &engine = thread_executions_[tid];
        if (engine.memory != image.memory || engine.source != image.acceleration.get() ||
            engine.generation != image.acceleration->generation()) {
            engine.memory = image.memory;
            engine.source = image.acceleration.get();
            engine.generation = image.acceleration->generation();
            engine.executor = std::make_unique<ExecutionEngine>();
            // Digest manifests and executable snapshots remain checked. Each
            // worker owns its helper progress and cache; guest memory is shared.
            engine.acceleration = std::make_unique<FixedComponents>(*image.acceleration);
            engine.acceleration->calls = engine.acceleration->bytes = 0;
        }
        BatchTask work;
        work.tid = tid;
        work.cpu = cpu;
        work.pid = process->pid;
        work.task = &task;
        work.memory = image.memory.get();
        work.executor = engine.executor.get();
        work.acceleration = engine.acceleration.get();
        batch.push_back(std::move(work));
    }
    batch.erase(std::remove_if(batch.begin(), batch.end(), [this](const BatchTask &work) {
        return work.task->state != Task::State::Runnable || work.task->stopped || work.task->ptrace_stopped ||
               work.task->ptrace_deferred_exit ||
               processes_.at(work.pid)->state != Process::State::Running;
    }), batch.end());
    if (batch.empty()) {
        schedule_pending_ = !main_exited_;
        return true;
    }
    const uint64_t quantum = std::min<uint64_t>(32768, budget / batch.size());
    std::vector<ParallelExecutor::Job> jobs;
    std::vector<uint64_t> eligible;
    jobs.reserve(batch.size());
    for (auto &work : batch) {
        work.budget = quantum;
        eligible.push_back(uint64_t{1} << work.cpu);
        jobs.emplace_back([&work](uint32_t cpu) {
            work.cpu = cpu;
            const auto before_cpu = worker_cpu_time();
            const auto instructions = work.executor->executed_instructions;
            const auto blocks = work.executor->decoded_blocks;
            const auto hits = work.executor->cache_hits;
            const auto calls = work.acceleration->calls;
            const auto bytes = work.acceleration->bytes;
            try {
                work.execution = work.executor->run(work.task->cpu, *work.memory, work.budget,
                                                        work.acceleration);
            } catch (...) {
                work.failure = std::current_exception();
            }
            const auto after_cpu = worker_cpu_time();
            work.usage = {after_cpu.user_nanoseconds - before_cpu.user_nanoseconds,
                          after_cpu.system_nanoseconds - before_cpu.system_nanoseconds,
                          after_cpu.runtime_nanoseconds - before_cpu.runtime_nanoseconds};
            work.instructions = work.executor->executed_instructions - instructions;
            work.blocks = work.executor->decoded_blocks - blocks;
            work.hits = work.executor->cache_hits - hits;
            work.calls = work.acceleration->calls - calls;
            work.bytes = work.acceleration->bytes - bytes;
        });
    }
    if (!executor_)
        executor_ = std::make_unique<ParallelExecutor>(execution_threads_);
    executor_->run(jobs, eligible);
    maximum_parallel_tasks_ = std::max(maximum_parallel_tasks_, executor_->last_peak_active());

    // Every CPU is at the barrier. Account all execution before dispatching any
    // syscall that could fork, exec, change mappings, reap or terminate a peer.
    for (auto &work : batch) {
        threads::rseq_event(work.task->rseq, threads::RseqEvent::Preempt);
        auto &process = *processes_.at(work.pid);
        process.instructions += work.instructions;
        process.decoded_blocks += work.blocks;
        process.cache_hits += work.hits;
        process.acceleration_calls += work.calls;
        process.acceleration_bytes += work.bytes;
        process.user_nanoseconds += work.usage.user_nanoseconds;
        process.system_nanoseconds += work.usage.system_nanoseconds;
        process.runtime_nanoseconds += work.usage.runtime_nanoseconds;
        process.maximum_rss = std::max(process.maximum_rss, work.memory->resident());
        work.task->user_nanoseconds += work.usage.user_nanoseconds;
        work.task->system_nanoseconds += work.usage.system_nanoseconds;
        work.task->runtime_nanoseconds += work.usage.runtime_nanoseconds;
        const auto limit = process.image.syscalls->resource_limits()->get(process::Resource::RealtimeTime);
        scheduler_.account_runtime(work.tid, work.usage.runtime_nanoseconds,
                                   limit.soft != process::resource_infinity);
        budget -= std::min(budget, work.instructions);
    }
    for (auto &work : batch) {
        auto *remaining = scheduler_.lookup(work.tid);
        auto *owner = find_process(work.pid);
        if (!remaining || !owner || remaining->pid != work.pid ||
            owner->image.memory.get() != work.memory)
            continue; // A preceding syscall in this batch executed a new image.
        auto &task = *remaining;
        auto &process = *owner;
        if (task.state != Task::State::Runnable || task.stopped || task.ptrace_stopped ||
            task.ptrace_deferred_exit ||
            process.state != Process::State::Running)
            continue;
        task.last_cpu = work.cpu;
        Scheduler::ScopedTask context(scheduler_, work.tid);
        if (work.failure) {
            try {
                std::rethrow_exception(work.failure);
            } catch (const InstructionFault &fault) {
                ++process.instructions;
                budget -= std::min<uint64_t>(budget, 1);
                task.cpu.exclusive_valid = false;
                signals::force_fault(*process.signals, task.signals, task.signal_mask,
                                     fault.kind == InstructionFault::Kind::Breakpoint ? 5 : 4,
                                     fault.pc, 1);
            } catch (const MemoryFault &fault) {
                budget -= std::min<uint64_t>(budget, 1);
                task.cpu.exclusive_valid = false;
                signals::force_fault(*process.signals, task.signals, task.signal_mask,
                                     fault.kind == MemoryFault::Kind::Bus ? 7 : 11, fault.address,
                                     fault.kind == MemoryFault::Kind::Unmapped ? 1 : 2);
            }
            continue;
        }
        if (work.execution.trap == Trap::Syscall) {
            begin_accounting(process.pid);
            accounting_syscall_ = true;
            if (!dispatch_guest_syscall(task, process, false)) {
                stop_accounting();
                continue;
            }
            ++process.voluntary_switches;
            ++task.voluntary_switches;
            if (process.pending_exec)
                commit_exec(process);
            auto &completed = scheduler_.current();
            if (completed.state == Task::State::Runnable && !completed.ptrace_stopped)
                wait_completed(completed.tid);
            drain_terminal_signals(process);
            stop_accounting();
        } else {
            ++process.involuntary_switches;
            ++task.involuntary_switches;
        }
    }
    collect_thread_executions();
    quantum_remaining_ = 262144;
    schedule_pending_ = !main_exited_;
    return true;
}
} // namespace operit::arm64
