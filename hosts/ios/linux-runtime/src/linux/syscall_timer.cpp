#include "linux/syscalls.h"
#include "linux/timers/timer_file.h"
#include "runtime/process/controller.h"

namespace operit::arm64 {
int64_t Syscalls::dispatch_timer(CpuState &cpu) {
    const auto &a = cpu.x;
    auto read_spec = [this](uint64_t address) {
        check_buffer(address, 32, read_perm);
        return TimerSpec{{int64_t(memory_.load(address, 8)), int64_t(memory_.load(address + 8, 8))},
                         {int64_t(memory_.load(address + 16, 8)), int64_t(memory_.load(address + 24, 8))}};
    };
    auto write_spec = [this](uint64_t address, const TimerSpec &value) {
        check_buffer(address, 32, write_perm);
        memory_.store(address, uint64_t(value.interval.seconds), 8);
        memory_.store(address + 8, uint64_t(value.interval.nanoseconds), 8);
        memory_.store(address + 16, uint64_t(value.value.seconds), 8);
        memory_.store(address + 24, uint64_t(value.value.nanoseconds), 8);
    };
    switch (a[8]) {
    case 102:
    case 103:
    case 107:
    case 108:
    case 109:
    case 110:
    case 111:
        if (auto *processes = scheduler_.processes())
            return processes->process_timer(cpu, memory_);
        guest_error(3);
    case 85: { // timerfd_create
        const uint32_t flags = uint32_t(a[1]);
        if (flags & ~uint32_t{2048 | 524288})
            guest_error(22);
        auto timer = TimerFile::create(int32_t(a[0]));
        int descriptor = descriptors_.insert(timer->duplicate_descriptor(), "", 2 | int(flags));
        auto &description = *descriptors_.get(descriptor).description;
        description.timer = std::move(timer);
        description.scheduler_io = true;
        return descriptor;
    }
    case 86: { // timerfd_settime: Linux AArch64 itimerspec has four 64-bit fields.
        TimerSpec value = read_spec(a[2]);
        auto timer = descriptors_.get(uint32_t(a[0])).description->timer;
        if (!timer)
            guest_error(22);
        TimerSpec previous = timer->set_time(uint32_t(a[1]), value);
        if (a[3])
            write_spec(a[3], previous);
        return 0;
    }
    case 87: { // timerfd_gettime
        auto timer = descriptors_.get(uint32_t(a[0])).description->timer;
        if (!timer)
            guest_error(22);
        write_spec(a[1], timer->get_time());
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
