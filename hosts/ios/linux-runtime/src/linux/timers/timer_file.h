#pragma once
#include "base/fd.h"
#include <cstdint>
#include <memory>

namespace operit::arm64 {
class TimerService;
namespace timers {
struct State;
}

struct TimerTime {
    int64_t seconds = 0;
    int64_t nanoseconds = 0;
};
struct TimerSpec {
    TimerTime interval;
    TimerTime value;
};

class TimerFile {
    std::shared_ptr<TimerService> service_;
    std::shared_ptr<timers::State> state_;
    TimerFile(std::shared_ptr<TimerService> service, std::shared_ptr<timers::State> state);

  public:
    static constexpr uint32_t absolute = 1;
    static constexpr uint32_t cancel_on_set = 2;
    static std::shared_ptr<TimerFile> create(int32_t clock_id);
    ~TimerFile();
    TimerFile(const TimerFile &) = delete;
    TimerFile &operator=(const TimerFile &) = delete;

    // The descriptor only carries readiness. Every guest read must call read().
    FD duplicate_descriptor() const;
    TimerSpec set_time(uint32_t flags, const TimerSpec &value);
    TimerSpec get_time();
    // Consumes all pending expirations; throws guest EAGAIN or ECANCELED.
    uint64_t read();
    int32_t clock_id() const;
};
} // namespace operit::arm64
