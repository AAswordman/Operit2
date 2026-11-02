#pragma once
#include "linux/timers/timer_file.h"
#include <memory>

namespace operit::arm64 {
class TimerService {
    struct Impl;
    std::unique_ptr<Impl> impl_;
    TimerService();
    friend class TimerFile;
    static std::shared_ptr<TimerService> acquire();
    std::shared_ptr<timers::State> create(int32_t clock_id);
    void remove(const std::shared_ptr<timers::State> &state);
    TimerSpec set_time(timers::State &state, uint32_t flags, const TimerSpec &value);
    TimerSpec get_time(timers::State &state);
    uint64_t read(timers::State &state);

  public:
    ~TimerService();
    TimerService(const TimerService &) = delete;
    TimerService &operator=(const TimerService &) = delete;
};
} // namespace operit::arm64
