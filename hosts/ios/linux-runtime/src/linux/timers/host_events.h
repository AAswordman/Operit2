#pragma once
#include "base/fd.h"
#include <cstdint>
#include <memory>
#include <optional>

namespace operit::arm64::timers {
// Host discontinuity notifications are kept separate from timer arithmetic.
class HostEvents {
    struct Impl;
    std::unique_ptr<Impl> impl_;

  public:
    HostEvents();
    ~HostEvents();
    HostEvents(const HostEvents &) = delete;
    HostEvents &operator=(const HostEvents &) = delete;
    void wake() noexcept;
    void wait(std::optional<int64_t> nanoseconds);
    // Called under the service lock, including before arming a new timer.
    bool clock_set();
    bool can_cancel_on_set() const;
};
} // namespace operit::arm64::timers
