#include "runtime/tasks/executor.h"
#include <atomic>
#include <condition_variable>
#include <cstddef>
#include <exception>
#include <limits>
#include <mutex>
#include <stdexcept>
#include <thread>
#include <utility>

namespace operit::arm64 {
struct ParallelExecutor::Impl {
    const uint32_t count;
    std::vector<std::thread> workers;
    std::mutex mutex, claim_mutex;
    std::condition_variable generation_ready, batch_finished;
    bool stopping = false, running = false;
    uint64_t generation = 0;
    uint32_t finished_workers = 0;
    const std::vector<Job> *jobs = nullptr;
    const std::vector<uint64_t> *eligible_cpu_masks = nullptr;
    std::vector<uint8_t> claimed;
    std::exception_ptr failure;
    std::atomic<size_t> next{0};
    std::atomic<uint32_t> active{0}, peak{0}, last_peak{0};
    std::atomic<uint64_t> completed_batches{0};

    explicit Impl(uint32_t requested) : count(requested) {
        if (!count)
            throw std::invalid_argument("ParallelExecutor requires at least one virtual CPU");
        workers.reserve(count - 1);
        try {
            for (uint32_t cpu = 1; cpu < count; ++cpu)
                workers.emplace_back([this, cpu] { worker(cpu); });
        } catch (...) {
            // A partially created pool still owns joinable threads.
            {
                std::lock_guard<std::mutex> lock(mutex);
                stopping = true;
            }
            generation_ready.notify_all();
            for (auto &thread : workers)
                thread.join();
            throw;
        }
    }
    ~Impl() {
        {
            std::unique_lock<std::mutex> lock(mutex);
            stopping = true;
            generation_ready.notify_all();
            // A published generation must finish even during shutdown. Workers
            // drain it before observing shutdown, so run() retains its barrier.
            batch_finished.wait(lock, [this] { return !running; });
        }
        for (auto &thread : workers)
            thread.join();
    }
    void execute(const Job &job, uint32_t cpu) noexcept {
        const uint32_t current = active.fetch_add(1, std::memory_order_relaxed) + 1;
        uint32_t observed = peak.load(std::memory_order_relaxed);
        while (observed < current &&
               !peak.compare_exchange_weak(observed, current, std::memory_order_relaxed)) {
        }
        std::exception_ptr error;
        // No executor lock is held while invoking guest work.
        try {
            job(cpu);
        } catch (...) {
            error = std::current_exception();
        }
        active.fetch_sub(1, std::memory_order_relaxed);
        if (error) {
            std::lock_guard<std::mutex> lock(mutex);
            if (!failure)
                failure = std::move(error);
        }
    }
    size_t claim_eligible(const std::vector<uint64_t> &masks, uint32_t cpu, size_t &cursor) {
        if (cpu >= 64)
            return masks.size();
        const uint64_t bit = uint64_t{1} << cpu;
        while (cursor < masks.size()) {
            const size_t index = cursor++;
            // Skip ineligible entries without holding any executor lock.
            if (!(masks[index] & bit))
                continue;
            std::lock_guard<std::mutex> lock(claim_mutex);
            if (!claimed[index]) {
                claimed[index] = 1;
                return index;
            }
        }
        return masks.size();
    }
    void drain(const std::vector<Job> &batch, const std::vector<uint64_t> *masks, uint32_t cpu) noexcept {
        if (count == 1) {
            for (const auto &job : batch)
                execute(job, 0);
            return;
        }
        if (masks) {
            size_t cursor = 0;
            for (;;) {
                const size_t index = claim_eligible(*masks, cpu, cursor);
                if (index == batch.size())
                    return;
                execute(batch[index], cpu);
            }
        }
        for (;;) {
            const size_t index = next.fetch_add(1, std::memory_order_relaxed);
            if (index >= batch.size())
                return;
            execute(batch[index], cpu);
        }
    }
    void worker(uint32_t cpu) {
        uint64_t observed_generation = 0;
        std::unique_lock<std::mutex> lock(mutex);
        for (;;) {
            generation_ready.wait(lock, [this, &observed_generation] {
                return stopping || generation != observed_generation;
            });
            if (generation == observed_generation)
                return; // Shutdown with no pending batch.
            observed_generation = generation;
            const auto *batch = jobs;
            const auto *masks = eligible_cpu_masks;
            lock.unlock();
            drain(*batch, masks, cpu);
            lock.lock();
            ++finished_workers;
            if (finished_workers == count - 1)
                batch_finished.notify_all();
        }
    }
    void run(const std::vector<Job> &batch, const std::vector<uint64_t> *masks = nullptr) {
        {
            std::lock_guard<std::mutex> lock(mutex);
            if (stopping)
                throw std::logic_error("ParallelExecutor is stopping");
            if (running)
                throw std::logic_error("ParallelExecutor::run is already active");
            if (generation == std::numeric_limits<uint64_t>::max())
                throw std::overflow_error("ParallelExecutor batch counter overflow");
            if (batch.size() > std::numeric_limits<size_t>::max() - count)
                throw std::length_error("ParallelExecutor batch is too large");
            if (masks) {
                if (masks->size() != batch.size())
                    throw std::invalid_argument("ParallelExecutor requires one CPU mask per job");
                const uint64_t available = count >= 64 ? UINT64_MAX : (uint64_t{1} << count) - 1;
                for (uint64_t mask : *masks)
                    if (!(mask & available))
                        throw std::invalid_argument("ParallelExecutor job has no eligible virtual CPU");
            }
            // Allocate/reset claims before the generation is visible to workers.
            if (masks && count > 1)
                claimed.assign(batch.size(), 0);
            else
                claimed.clear();
            jobs = &batch;
            eligible_cpu_masks = masks;
            failure = nullptr;
            finished_workers = 0;
            next.store(0, std::memory_order_relaxed);
            active.store(0, std::memory_order_relaxed);
            peak.store(0, std::memory_order_relaxed);
            running = true;
            ++generation;
        }
        generation_ready.notify_all();
        drain(batch, masks, 0);
        std::exception_ptr error;
        {
            std::unique_lock<std::mutex> lock(mutex);
            batch_finished.wait(lock, [this] { return finished_workers == count - 1; });
            last_peak.store(peak.load(std::memory_order_relaxed), std::memory_order_release);
            completed_batches.fetch_add(1, std::memory_order_release);
            jobs = nullptr;
            eligible_cpu_masks = nullptr;
            error = std::move(failure);
            running = false;
        }
        batch_finished.notify_all();
        if (error)
            std::rethrow_exception(error);
    }
};

ParallelExecutor::ParallelExecutor(uint32_t count) : impl_(std::make_unique<Impl>(count)) {}
ParallelExecutor::~ParallelExecutor() = default;
void ParallelExecutor::run(const std::vector<Job> &jobs) {
    impl_->run(jobs);
}
void ParallelExecutor::run(const std::vector<Job> &jobs, const std::vector<uint64_t> &eligible_cpu_masks) {
    impl_->run(jobs, &eligible_cpu_masks);
}
uint32_t ParallelExecutor::size() const noexcept {
    return impl_->count;
}
uint32_t ParallelExecutor::last_peak_active() const noexcept {
    return impl_->last_peak.load(std::memory_order_acquire);
}
uint64_t ParallelExecutor::batches() const noexcept {
    return impl_->completed_batches.load(std::memory_order_acquire);
}
} // namespace operit::arm64
