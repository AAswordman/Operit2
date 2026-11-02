#pragma once
#include <cstdint>
#include <functional>

namespace operit::arm64 {
class Memory;
class DescriptorTable;
struct Descriptor;
namespace process {
class ResourceLimits;
}
namespace files {
constexpr uint32_t allocation_keep_size = 0x01;
constexpr uint32_t allocation_punch_hole = 0x02;
constexpr uint32_t allocation_collapse_range = 0x08;
constexpr uint32_t allocation_zero_range = 0x10;
constexpr uint32_t allocation_insert_range = 0x20;
constexpr uint32_t allocation_unshare_range = 0x40;

// Queue guest SIGXFSZ (25) on the calling task. This is invoked after releasing
// Memory and memfd guards; it must not send a signal to the embedding process.
using AllocationSizeSignal = std::function<void()>;

// Linux syscall 47. Returns zero or throws GuestError with a Linux errno.
// DescriptorTable supplies the calling process's RLIMIT_FSIZE. Wire signal_file_size
// to guest signal delivery at the syscall boundary. No host rlimit is modified.
int64_t fallocate(Memory &memory, DescriptorTable &descriptors, uint64_t fd, uint32_t mode,
                  int64_t offset, int64_t length,
                  const AllocationSizeSignal &signal_file_size = {});
int64_t fallocate(Memory &memory, const Descriptor &entry, const process::ResourceLimits &limits,
                  uint32_t mode, int64_t offset, int64_t length,
                  const AllocationSizeSignal &signal_file_size = {});
} // namespace files
} // namespace operit::arm64
