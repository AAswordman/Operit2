#pragma once
#include "base/fd.h"
#include "base/host_io.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <cerrno>
#include <cstdint>
namespace operit::arm64 {
class EventCounter {
    FD writer_;
    uint64_t counter_;
    bool semaphore_;

  public:
    EventCounter(FD writer, uint64_t initial, bool semaphore)
        : writer_(std::move(writer)), counter_(initial), semaphore_(semaphore) {
        if (initial) {
            const char ready = 1;
            check_host(host_write(writer_.get(), &ready, 1));
        }
    }
    uint64_t read(int reader) {
        if (!counter_)
            guest_error(11);
        uint64_t result = semaphore_ ? 1 : counter_;
        counter_ -= result;
        if (!counter_) {
            char ready;
            check_host(::read(reader, &ready, 1));
        }
        return result;
    }
    void write(uint64_t value) {
        if (value == UINT64_MAX)
            guest_error(22);
        if (value > UINT64_MAX - 1 - counter_)
            guest_error(11);
        if (value && !counter_) {
            const char ready = 1;
            check_host(host_write(writer_.get(), &ready, 1));
        }
        counter_ += value;
    }
};
} // namespace operit::arm64
