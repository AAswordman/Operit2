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
    Interpreter *interpreter = nullptr;
    FixedComponents *acceleration = nullptr;
    Memory *memory = nullptr;
    Task *task = nullptr;
    std::exception_ptr failure;
};
} // namespace

bool Machine::run_parallel(uint64_t &budget) {
    if (execution_threads_ < 2 || budget < 8192)
        return false;
    auto tids = scheduler_.runnable_tids(execution_threads_);
    if (tids.size() < 2)
        return false;
    stop_accounting();
    // No guest CPU is executing while access mode changes. Include aliases in
    // other address spaces before starting any worker in this batch.
    for (auto &[pid, process] : processes_) {
        (void)pid;
        if (process->state == Process::State::Running)
            process->image.memory->set_parallel_access(true);
    }
    std::vector<BatchTask> batch;
    batch.reserve(tids.size());
    for (const auto tid : tids) {
        Scheduler::ScopedTask context(scheduler_, tid);
        auto &task = scheduler_.current();
        auto *process = find_process(task.pid);
        if (!process || process->state != Process::State::Running)
            continue;
        auto &image = process->image;
        image.memory->synchronize_shared();
        const auto delivery = signals::deliver(*image.memory, task.cpu, *process->signals, task.signals,
                                               task.signal_mask, image.signal_trampoline);
        if (delivery.effect != signals::Effect::None && delivery.effect != signals::Effect::Handler)
            signal_hooks().effect(process->pid, delivery);
        if (task.state != Task::State::Runnable || task.stopped || main_exited_)
            continue;
        auto &engine = thread_executions_[tid];
        if (engine.memory != image.memory || engine.source != image.acceleration.get() ||
            engine.generation != image.acceleration->generation()) {
            engine.memory = image.memory;
            engine.source = image.acceleration.get();
            engine.generation = image.acceleration->generation();
            engine.interpreter = std::make_unique<Interpreter>();
            // Digest manifests and executable snapshots remain checked. Each
            // worker owns its helper progress and cache; guest memory is shared.
            engine.acceleration = std::make_unique<FixedComponents>(*image.acceleration);
            engine.acceleration->calls = engine.acceleration->bytes = 0;
        }
        BatchTask work;
        work.tid = tid;
        work.pid = process->pid;
        work.task = &task;
        work.memory = image.memory.get();
        work.interpreter = engine.interpreter.get();
        work.acceleration = engine.acceleration.get();
        batch.push_back(std::move(work));
    }
    batch.erase(std::remove_if(batch.begin(), batch.end(), [this](const BatchTask &work) {
        return work.task->state != Task::State::Runnable || work.task->stopped ||
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
        eligible.push_back(work.task->cpu_affinity);
        jobs.emplace_back([&work](uint32_t cpu) {
            work.cpu = cpu;
            const auto before_cpu = worker_cpu_time();
            const auto instructions = work.interpreter->executed_instructions;
            const auto blocks = work.interpreter->decoded_blocks;
            const auto hits = work.interpreter->cache_hits;
            const auto calls = work.acceleration->calls;
            const auto bytes = work.acceleration->bytes;
            try {
                work.execution = work.interpreter->run(work.task->cpu, *work.memory, work.budget,
                                                        work.acceleration);
            } catch (...) {
                work.failure = std::current_exception();
            }
            const auto after_cpu = worker_cpu_time();
            work.usage = {after_cpu.user_nanoseconds - before_cpu.user_nanoseconds,
                          after_cpu.system_nanoseconds - before_cpu.system_nanoseconds,
                          after_cpu.runtime_nanoseconds - before_cpu.runtime_nanoseconds};
            work.instructions = work.interpreter->executed_instructions - instructions;
            work.blocks = work.interpreter->decoded_blocks - blocks;
            work.hits = work.interpreter->cache_hits - hits;
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
        if (task.state != Task::State::Runnable || task.stopped || process.state != Process::State::Running)
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
            const auto number = task.cpu.x[8];
            const auto code = int(task.cpu.x[0]) & 255;
            signals::remember_syscall(task.signals, task.cpu, true);
            process.image.syscalls->handle(task.cpu);
            // Release this retired thread's queue before dispatching a peer's
            // next syscall in the same batch. Process-wide exit clears all queues.
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
                finish_process(process,
                               number == 94 ? code << 8 : process.leader_status.value_or(code << 8));
            ++process.voluntary_switches;
            ++task.voluntary_switches;
            if (process.pending_exec)
                commit_exec(process);
            drain_terminal_signals(process);
            stop_accounting();
        } else {
            ++process.involuntary_switches;
            ++task.involuntary_switches;
        }
    }
    for (auto it = thread_executions_.begin(); it != thread_executions_.end();) {
        const auto *task = scheduler_.lookup(it->first);
        if (!task || task->state == Task::State::Exited)
            it = thread_executions_.erase(it);
        else
            ++it;
    }
    quantum_remaining_ = 4096;
    schedule_pending_ = !main_exited_;
    return true;
}
} // namespace operit::arm64
