#pragma once
#include <cstdint>
#include <functional>

namespace operit::arm64 {
class DescriptorTable;
struct Descriptor;
}

namespace operit::arm64::files {
// Linux ABI values, independent of the host's close_range/fcntl headers.
inline constexpr uint32_t close_range_unshare = 1u << 1;
inline constexpr uint32_t close_range_cloexec = 1u << 2;
using CloseRangeBeforeClose = std::function<void(const Descriptor &)>;

// Linux AArch64 syscall 436. The unsigned [first, last] interval is inclusive;
// UINT32_MAX is an ordinary upper bound. Returns zero or throws GuestError.
// Invalid ranges/flags are rejected before unsharing or changing descriptors.
// UNSHARE detaches descriptor state while retaining shared OpenDescriptions,
// including when the interval contains no open descriptors.
//
// before_close runs once per existing FD to be closed, while its Descriptor is
// still valid, for the caller's record-lock/terminal cleanup. It is not called
// for CLOEXEC. GuestError from an individual close/hook is ignored, as for Linux
// close_range; other callback exceptions propagate. The callback must not mutate
// the table. As with DescriptorTable itself, callers serialize table access.
int64_t close_range(DescriptorTable &descriptors, uint32_t first, uint32_t last, uint32_t flags,
                    const CloseRangeBeforeClose &before_close);
} // namespace operit::arm64::files
