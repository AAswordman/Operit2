#include "linux/io_uring/io_uring.h"
#include "linux/syscalls.h"

namespace operit::arm64 {
int64_t Syscalls::dispatch_io_uring(CpuState &cpu) {
    const auto &a = cpu.x;
    if (a[8] == 425) {
        auto result = io_uring::setup(memory_, descriptors_, uint32_t(a[0]), a[1]);
        descriptors_.get(result.fd).description->io_uring = std::move(result.ring);
        return result.fd;
    }
    auto ring = descriptors_.get(uint32_t(a[0])).description->io_uring;
    if (!ring)
        guest_error(95);
    io_uring::AsyncSubmission async{
        io_uring_context_, io_uring_memory_,
        [](Memory &memory, const std::vector<io_uring::MemoryRange> &ranges) {
            std::vector<std::pair<uint64_t, uint64_t>> requested;
            requested.reserve(ranges.size());
            for (const auto &range : ranges)
                requested.emplace_back(range.address, range.length);
            return memory.capture_write_lease(requested);
        },
    };
    if (a[8] == 427)
        return io_uring::register_op(memory_, ring, uint32_t(a[1]), a[2], uint32_t(a[3]),
                                     bool(io_uring_context_) && !io_uring_memory_.expired());
    return io_uring::enter(
        memory_, descriptors_, scheduler_, ring, uint32_t(a[1]), uint32_t(a[2]), uint32_t(a[3]),
        a[4], a[5],
        [this](bool writing, uint32_t fd, uint64_t address, uint64_t length, uint64_t offset,
               bool positioned) { return io(writing, fd, address, length, offset, positioned, false); },
        [this] { return interrupt_for_pending_signal(); }, &async);
}
} // namespace operit::arm64
