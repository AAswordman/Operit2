#pragma once
#include "base/fd.h"
#include <cstdint>

namespace operit::arm64::timers {
struct State {
    FD reader, writer;
    int32_t clock_id;
    int64_t deadline = 0, interval = 0;
    uint64_t expirations = 0;
    int error = 0;
    bool wall_clock = false;
    bool active = false;
    bool expired = false;
    bool cancel_on_set = false;
    bool cancelled = false;
    bool ready = false;
    State(FD read, FD write, int32_t clock)
        : reader(std::move(read)), writer(std::move(write)), clock_id(clock) {}
};
} // namespace operit::arm64::timers
