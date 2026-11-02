#include "runtime/machine.h"
#include "linux/error_mapping.h"
#include <time.h>

namespace operit::arm64 {
uint64_t process_boottime_nanoseconds() {
    timespec value{};
#if defined(__APPLE__)
    // system/clocks.cpp maps guest CLOCK_BOOTTIME to this same host clock.
    constexpr auto clock = CLOCK_MONOTONIC;
#else
    constexpr auto clock = CLOCK_BOOTTIME;
#endif
    check_host(::clock_gettime(clock, &value));
    if (value.tv_sec < 0 || value.tv_nsec < 0 || value.tv_nsec >= 1000000000)
        guest_error(5);
    const auto nanoseconds = static_cast<unsigned __int128>(value.tv_sec) * 1000000000 +
                             uint64_t(value.tv_nsec);
    if (nanoseconds > UINT64_MAX)
        guest_error(75);
    return uint64_t(nanoseconds);
}

ProcessAccounting Machine::proc_stat_accounting(uint32_t pid, uint32_t tid, bool whole_group) const {
    const auto process = processes_.find(pid);
    if (process == processes_.end() || process->second->reaped)
        guest_error(2);
    const auto *task = scheduler_.lookup(tid);
    if (!task || task->pid != pid || (task->state == Task::State::Exited && task->tid != pid))
        guest_error(2);
    const auto *birth = whole_group ? scheduler_.lookup(pid) : task;
    if (!birth || birth->pid != pid)
        guest_error(2);
    ProcessAccounting result;
    result.self = snapshot_resource_usage(*process->second, whole_group ? nullptr : task);
    result.children = snapshot_child_usage(*process->second);
    result.start_boottime_nanoseconds = birth->start_boottime_nanoseconds;
    return result;
}
} // namespace operit::arm64
