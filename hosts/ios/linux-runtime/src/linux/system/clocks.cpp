#include "linux/system/clocks.h"
#include "cpu/aarch64/state.h"
#include "linux/abi.h"
#include "linux/error_mapping.h"
#include "memory/address_space.h"
#include "runtime/process/controller.h"
#include <array>
#include <chrono>
#include <ctime>

namespace operit::arm64::system {
namespace {
constexpr uint64_t nanoseconds_per_second = 1000000000;
constexpr uint64_t microseconds_per_second = 1000000;
// This is the guest USER_HZ published by AT_CLKTCK, independent of host HZ.
constexpr uint64_t ticks_per_second = 100;
constexpr uint64_t nanoseconds_per_tick = nanoseconds_per_second / ticks_per_second;
struct CpuClock {
    uint32_t id;
    bool per_thread;
    unsigned sample;
};
CpuClock cpu_clock(int32_t id) {
    if (id == 2 || id == 3)
        return {0, id == 3, 2};
    const auto encoded = uint32_t(id);
    // The low-bits value 3 belongs to FD-backed clocks, never a host PID/FD.
    if (id >= 0 || (encoded & 3) == 3)
        guest_error(22);
    return {~encoded >> 3, bool(encoded & 4), encoded & 3};
}
void writable(Memory &memory, uint64_t address, size_t size) {
    if (!address || !memory.accessible(address, size, write_perm))
        guest_error(14);
}
void store_timespec(Memory &memory, uint64_t address, const linux_abi::Timespec &value) {
    writable(memory, address, sizeof(value));
    memory.copy_in(address, &value, sizeof(value));
}
linux_abi::Timespec cpu_time(ProcessController &processes, int32_t id, bool resolution) {
    const auto clock = cpu_clock(id);
    const auto usage = processes.cpu_usage(clock.id, clock.per_thread, !resolution);
    if (resolution) {
        if (clock.sample != 2)
            return {0, 1000}; // Native per-thread user/system samples use microseconds.
        timespec value{};
        check_host(::clock_getres(CLOCK_THREAD_CPUTIME_ID, &value));
        return {value.tv_sec, value.tv_nsec};
    }
    const auto nanoseconds = clock.sample == 0   ? usage.user_nanoseconds + usage.system_nanoseconds
                             : clock.sample == 1 ? usage.user_nanoseconds
                                                 : usage.runtime_nanoseconds;
    return {int64_t(nanoseconds / nanoseconds_per_second), int64_t(nanoseconds % nanoseconds_per_second)};
}
clockid_t host_clock(int32_t id) {
    switch (id) {
    case 0:
        return CLOCK_REALTIME;
    case 1:
        return CLOCK_MONOTONIC;
    case 4:
        return CLOCK_MONOTONIC_RAW;
    case 5:
#if defined(__APPLE__)
        return CLOCK_REALTIME;
#else
        return CLOCK_REALTIME_COARSE;
#endif
    case 6:
#if defined(__APPLE__)
        return CLOCK_MONOTONIC;
#else
        return CLOCK_MONOTONIC_COARSE;
#endif
    case 7:
#if defined(__APPLE__)
        return CLOCK_MONOTONIC;
#else
        return CLOCK_BOOTTIME;
#endif
    default:
        guest_error(22);
    }
}
int64_t clock_gettime(Memory &memory, ProcessController &processes, int32_t id, uint64_t address,
                      bool resolution) {
    linux_abi::Timespec value{};
    if (id == 2 || id == 3 || id < 0) {
        value = cpu_time(processes, id, resolution);
    } else {
        timespec native{};
        const auto clock = host_clock(id);
        check_host(resolution ? ::clock_getres(clock, &native) : ::clock_gettime(clock, &native));
        value = {native.tv_sec, native.tv_nsec};
    }
    // clock_getres(NULL) still validates the clock and its target identity.
    if (!resolution || address)
        store_timespec(memory, address, value);
    return 0;
}
int64_t getrusage(Memory &memory, ProcessController &processes, int32_t who, uint64_t address) {
    ResourceScope scope;
    switch (who) {
    case 0:
        scope = ResourceScope::Process;
        break;
    case -1:
        scope = ResourceScope::Children;
        break;
    case -2:
        scope = ResourceScope::Thread;
        break;
    default:
        guest_error(22);
    }
    writable(memory, address, 18 * sizeof(uint64_t));
    const auto usage = processes.resource_usage(scope);
    const auto user = usage.cpu.user_nanoseconds / 1000;
    const auto system = usage.cpu.system_nanoseconds / 1000;
    std::array<uint64_t, 18> values{};
    values[0] = user / microseconds_per_second;
    values[1] = user % microseconds_per_second;
    values[2] = system / microseconds_per_second;
    values[3] = system % microseconds_per_second;
    values[4] = usage.maximum_rss / 1024;
    values[8] = usage.minor_faults;
    values[9] = usage.major_faults;
    values[11] = usage.input_blocks;
    values[12] = usage.output_blocks;
    values[16] = usage.voluntary_switches;
    values[17] = usage.involuntary_switches;
    memory.copy_in(address, values.data(), sizeof(values));
    return 0;
}
int64_t times(Memory &memory, ProcessController &processes, uint64_t address) {
    if (address) {
        writable(memory, address, 4 * sizeof(uint64_t));
        const auto self = processes.resource_usage(ResourceScope::Process);
        const auto children = processes.resource_usage(ResourceScope::Children);
        const std::array<uint64_t, 4> values{self.cpu.user_nanoseconds / nanoseconds_per_tick,
                                             self.cpu.system_nanoseconds / nanoseconds_per_tick,
                                             children.cpu.user_nanoseconds / nanoseconds_per_tick,
                                             children.cpu.system_nanoseconds / nanoseconds_per_tick};
        memory.copy_in(address, values.data(), sizeof(values));
    }
    // Linux specifies an arbitrary monotonic origin, shared by all guest tasks.
    const auto elapsed = std::chrono::steady_clock::now().time_since_epoch();
    return std::chrono::duration_cast<std::chrono::nanoseconds>(elapsed).count() / nanoseconds_per_tick;
}
} // namespace
int64_t dispatch_clocks(CpuState &cpu, Memory &memory, ProcessController &processes) {
    const auto &arguments = cpu.x;
    switch (arguments[8]) {
    case 113:
    case 114:
        return clock_gettime(memory, processes, int32_t(arguments[0]), arguments[1], arguments[8] == 114);
    case 153:
        return times(memory, processes, arguments[0]);
    case 165:
        return getrusage(memory, processes, int32_t(arguments[0]), arguments[1]);
    default:
        guest_error(38);
    }
}
} // namespace operit::arm64::system
