#pragma once
#include <cstdint>
#include <functional>
#include <memory>
#include <vector>

namespace operit::arm64 {
// One reusable pool per VM. count includes the thread calling run(): it is
// virtual CPU 0; the count - 1 persistent workers own IDs 1 through count - 1.
class ParallelExecutor final {
  public:
    using Job = std::function<void(uint32_t virtual_cpu)>;

    // count must be positive. count == 1 creates no host worker threads.
    explicit ParallelExecutor(uint32_t count);
    // The owning thread destroys the executor after its final run() returns.
    ~ParallelExecutor();
    ParallelExecutor(const ParallelExecutor &) = delete;
    ParallelExecutor &operator=(const ParallelExecutor &) = delete;
    ParallelExecutor(ParallelExecutor &&) = delete;
    ParallelExecutor &operator=(ParallelExecutor &&) = delete;

    // Runs every job exactly once, then waits for every worker to acknowledge
    // the batch. The first captured job exception is rethrown after that barrier;
    // other jobs still run. The caller must keep jobs unchanged until return.
    // Concurrent/reentrant run() on this executor throws std::logic_error.
    void run(const std::vector<Job> &jobs);
    // One mask per job; bit n permits guest vCPU n. Extra bits outside size()
    // are ignored, but every mask must select at least one existing vCPU.
    // uint64_t masks select CPUs 0..63; higher CPUs cannot claim masked work.
    // Both vectors must remain unchanged until return. Invalid masks/counts
    // throw std::invalid_argument before publishing or executing the batch.
    void run(const std::vector<Job> &jobs, const std::vector<uint64_t> &eligible_cpu_masks);

    uint32_t size() const noexcept;
    // Peak simultaneously active jobs in the most recently completed batch.
    uint32_t last_peak_active() const noexcept;
    // Completed batches, including empty batches and batches whose jobs threw.
    uint64_t batches() const noexcept;

  private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};
} // namespace operit::arm64
