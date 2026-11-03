#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <optional>
#include <vector>

namespace operit::arm64 {
class DescriptorTable;
class Memory;
class Scheduler;
struct Task;

namespace io_uring {
class Ring;
class CompletionContext;

// A context belongs to one Machine, not one guest process. These bounds also
// cover completed work until the coordinator drains or abandons it.
constexpr uint32_t max_async_in_flight_per_ring = 64;
constexpr uint32_t max_async_in_flight_per_context = 64;
constexpr uint32_t max_async_in_flight_global = 128;

struct MemoryRange {
    uint64_t address, length;
};
// The lease must not retain the Memory object, and must reject an unmap or
// replacement of any captured byte even if the address becomes writable
// again. Capture and validation run only at the coordinator's worker barrier.
using MappingLease = std::function<bool(const Memory &)>;
using CaptureMappingLease = std::function<MappingLease(Memory &, const std::vector<MemoryRange> &)>;
struct AsyncSubmission {
    std::shared_ptr<CompletionContext> context;
    std::weak_ptr<Memory> memory;
    CaptureMappingLease capture_mapping;
};

// One context per Machine. Destruction must run on its coordinator thread;
// in-flight native I/O is abandoned without waiting for the host operation.
// Its global worker quota remains charged until those operations really end.
std::shared_ptr<CompletionContext> make_completion_context();
// Call at every Machine quantum boundary and while all guest tasks are idle,
// after all workers have stopped and before retrying blocked guest operations.
void service_completions(const std::shared_ptr<CompletionContext> &context, Scheduler &scheduler);

struct SetupResult {
    int fd;
    std::shared_ptr<Ring> ring;
};

// Execute the ordinary runtime scalar read/write path. This callback must not
// block the guest task; requests are restricted to regular files by enter().
using ExecuteIo = std::function<int64_t(bool writing, uint32_t fd, uint64_t address,
                                        uint64_t length, uint64_t offset, bool positioned)>;
using InterruptPending = std::function<bool()>;

SetupResult setup(Memory &memory, DescriptorTable &descriptors, uint32_t entries,
                  uint64_t params_address);
int64_t enter(Memory &memory, DescriptorTable &descriptors, Scheduler &scheduler,
              const std::shared_ptr<Ring> &ring, uint32_t to_submit, uint32_t min_complete,
              uint32_t flags, uint64_t argument, uint64_t argument_size,
              const ExecuteIo &execute_io, const InterruptPending &interrupt_pending,
              const AsyncSubmission *async = nullptr);
int64_t register_op(Memory &memory, const std::shared_ptr<Ring> &ring, uint32_t opcode,
                    uint64_t argument, uint32_t count, bool async_files = false);

// The caller performs normal mmap address selection and resource-limit checks.
// Only call this for a descriptor whose open description owns this ring.
void map_ring(const std::shared_ptr<Ring> &ring, Memory &memory, uint64_t address,
              uint64_t length, unsigned permissions, uint64_t flags, uint64_t offset,
              bool replace);

// Refresh before polling any io_uring descriptor, including epoll retries.
void refresh_readiness(const std::shared_ptr<Ring> &ring);

// Scheduler::interrupt calls this before completing an interrupted task.
// A partial submission or a nonempty CQ takes precedence over EINTR.
std::optional<int64_t> interrupt_wait(Task &task);
} // namespace io_uring
} // namespace operit::arm64
