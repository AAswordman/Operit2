#include "linux/system/information.h"
#include "linux/error_mapping.h"
#include "memory/address_space.h"
#include "runtime/process/controller.h"
#include "runtime/tasks/scheduler.h"
#include <cmath>
#include <set>
#include <time.h>
#if defined(__APPLE__)
#include <mach/mach_time.h>
#endif

namespace operit::arm64::system {
namespace {
constexpr uint64_t nanoseconds_per_second = 1000000000;
constexpr uint64_t load_scale = uint64_t{1} << 16;

uint64_t checked_add(uint64_t first, uint64_t second) {
    if (second > UINT64_MAX - first)
        guest_error(75); // EOVERFLOW
    return first + second;
}
uint64_t task_count(const TaskInformation &tasks) {
    const uint64_t live = checked_add(checked_add(tasks.runnable_tasks, tasks.waiting_tasks),
                                     tasks.stopped_tasks);
    const uint64_t total = checked_add(tasks.live_tasks, tasks.zombie_tasks);
    if (live != tasks.live_tasks || tasks.zombie_tasks > tasks.processes || tasks.processes > total ||
        (!tasks.processes && total))
        guest_error(22);
    return total;
}
void validate_memory(const MemoryInformation &memory) {
    if (memory.resident_bytes > memory.total_bytes || memory.shared_bytes > memory.resident_bytes ||
        memory.buffer_bytes > memory.resident_bytes - memory.shared_bytes ||
        memory.free_swap_bytes > memory.swap_bytes)
        guest_error(22);
}
uint64_t fixed_load(long double value) {
    if (!std::isfinite(value) || value < 0)
        guest_error(22);
    value = std::floor(value * load_scale + 0.5L);
    // Comparing to 2^64 avoids a rounded UINT64_MAX on platforms where long
    // double has the same precision as double, including Apple AArch64.
    if (!std::isfinite(value) || value >= std::ldexp(1.0L, 64))
        guest_error(75);
    return static_cast<uint64_t>(value);
}
void put_le(uint8_t *destination, uint64_t value, unsigned bytes) {
    for (unsigned index = 0; index < bytes; ++index)
        destination[index] = static_cast<uint8_t>(value >> (index * 8));
}
bool put_cpu_id(Memory &memory, uint64_t address, uint32_t value) {
    if (!address)
        return true;
    if (!memory.accessible(address, sizeof(uint32_t), write_perm))
        return false;
    std::array<uint8_t, sizeof(uint32_t)> bytes{};
    put_le(bytes.data(), value, bytes.size());
    try {
        memory.copy_in(address, bytes.data(), bytes.size());
    } catch (const MemoryFault &) {
        return false;
    }
    return true;
}
} // namespace

uint64_t information_clock_nanoseconds() {
#if defined(__APPLE__)
    static const mach_timebase_info_data_t timebase = [] {
        mach_timebase_info_data_t value{};
        if (::mach_timebase_info(&value) != KERN_SUCCESS || !value.numer || !value.denom)
            guest_error(5);
        return value;
    }();
    const auto value = static_cast<unsigned __int128>(::mach_continuous_time()) * timebase.numer /
                       timebase.denom;
    if (value > UINT64_MAX)
        guest_error(75);
    return static_cast<uint64_t>(value);
#else
    struct timespec now{};
    check_host(::clock_gettime(CLOCK_BOOTTIME, &now));
    if (now.tv_sec < 0 || now.tv_nsec < 0 || now.tv_nsec >= int64_t(nanoseconds_per_second))
        guest_error(5);
    const auto value = static_cast<unsigned __int128>(now.tv_sec) * nanoseconds_per_second +
                       static_cast<uint64_t>(now.tv_nsec);
    if (value > UINT64_MAX)
        guest_error(75);
    return static_cast<uint64_t>(value);
#endif
}
SamplingState start_information(const Scheduler &scheduler, uint64_t now_nanoseconds) {
    SamplingState state{};
    state.started = true;
    state.boot_nanoseconds = state.observed_nanoseconds = now_nanoseconds;
    state.runnable_tasks = task_information(scheduler).runnable_tasks;
    return state;
}
TaskInformation task_information(const Scheduler &scheduler) {
    TaskInformation result{};
    std::set<uint32_t> processes;
    if (const auto *controller = scheduler.processes())
        for (uint32_t pid : controller->process_ids())
            processes.insert(pid);
    for (uint32_t tid : scheduler.live_tids()) {
        const auto *task = scheduler.lookup(tid);
        if (!task || task->state == Task::State::Exited)
            guest_error(3);
        processes.insert(task->pid);
        ++result.live_tasks;
        if (task->stopped)
            ++result.stopped_tasks;
        else if (task->state == Task::State::Runnable)
            ++result.runnable_tasks;
        else
            ++result.waiting_tasks;
    }
    result.processes = processes.size();
    // Exited worker threads have no waitable process record. A retained exited
    // process leader does, including a leader whose other threads still run.
    for (uint32_t pid : processes) {
        const auto *leader = scheduler.lookup(pid);
        if (!leader || leader->state == Task::State::Exited)
            ++result.zombie_tasks;
    }
    (void)task_count(result);
    return result;
}
MemoryInformation memory_information(const Memory &memory) {
    MemoryInformation result{};
    result.total_bytes = memory.limit() - memory.limit() % page_size;
    result.resident_bytes = memory.resident();
    validate_memory(result);
    return result;
}
MemoryInformation memory_information(const std::vector<const Memory *> &address_spaces) {
    MemoryInformation result{};
    std::set<const Memory *> distinct;
    for (const Memory *memory : address_spaces) {
        if (!memory)
            guest_error(22);
        if (!distinct.insert(memory).second)
            continue;
        const auto current = memory_information(*memory);
        result.total_bytes = checked_add(result.total_bytes, current.total_bytes);
        result.resident_bytes = checked_add(result.resident_bytes, current.resident_bytes);
    }
    return result;
}
void observe_information(SamplingState &state, const TaskInformation &tasks, uint64_t now_nanoseconds) {
    (void)task_count(tasks);
    if (!state.started || state.observed_nanoseconds < state.boot_nanoseconds ||
        now_nanoseconds < state.observed_nanoseconds)
        guest_error(22);
    const long double elapsed = static_cast<long double>(now_nanoseconds - state.observed_nanoseconds) /
                                nanoseconds_per_second;
    const long double periods[] = {60, 300, 900};
    auto loads = state.loads;
    for (size_t index = 0; index < loads.size(); ++index) {
        (void)fixed_load(loads[index]);
        // expm1 retains precision when sampling each interpreter quantum.
        const long double weight = -std::expm1(-elapsed / periods[index]);
        loads[index] += (static_cast<long double>(state.runnable_tasks) - loads[index]) * weight;
        (void)fixed_load(loads[index]);
    }
    // Commit only after every input and conversion has been checked.
    state.loads = loads;
    state.observed_nanoseconds = now_nanoseconds;
    state.runnable_tasks = tasks.runnable_tasks;
}
void observe_information(SamplingState &state, const Scheduler &scheduler, uint64_t now_nanoseconds) {
    observe_information(state, task_information(scheduler), now_nanoseconds);
}
Snapshot snapshot_information(SamplingState &state, const MemoryInformation &memory,
                              const TaskInformation &tasks, uint64_t now_nanoseconds) {
    validate_memory(memory);
    auto next = state;
    observe_information(next, tasks, now_nanoseconds);
    Snapshot snapshot{};
    snapshot.uptime_nanoseconds = now_nanoseconds - next.boot_nanoseconds;
    snapshot.memory = memory;
    snapshot.tasks = tasks;
    for (size_t index = 0; index < snapshot.loads.size(); ++index)
        snapshot.loads[index] = fixed_load(next.loads[index]);
    state = next;
    return snapshot;
}
Snapshot snapshot_information(SamplingState &state, const Memory &memory, const Scheduler &scheduler,
                              uint64_t now_nanoseconds) {
    return snapshot_information(state, memory_information(memory), task_information(scheduler),
                                now_nanoseconds);
}
LinuxSysinfo make_sysinfo(const Snapshot &snapshot) {
    validate_memory(snapshot.memory);
    const uint64_t threads = task_count(snapshot.tasks);
    LinuxSysinfo result{};
    const uint64_t seconds = snapshot.uptime_nanoseconds / nanoseconds_per_second;
    result.uptime = static_cast<int64_t>(
        seconds + (snapshot.uptime_nanoseconds % nanoseconds_per_second != 0));
    result.loads = snapshot.loads;
    result.totalram = snapshot.memory.total_bytes;
    result.freeram = result.totalram - snapshot.memory.resident_bytes;
    result.sharedram = snapshot.memory.shared_bytes;
    result.bufferram = snapshot.memory.buffer_bytes;
    result.totalswap = snapshot.memory.swap_bytes;
    result.freeswap = snapshot.memory.free_swap_bytes;
    // Native Linux narrows nr_threads to unsigned short; retain that ABI even
    // above 65535. Snapshot::tasks retains the full checked 64-bit counts.
    result.procs = static_cast<uint16_t>(threads);
    result.mem_unit = 1;
    return result;
}
int64_t sysinfo(Memory &memory, uint64_t destination, const Snapshot &snapshot) {
    if (!memory.accessible(destination, sizeof(LinuxSysinfo), write_perm))
        guest_error(14);
    const auto value = make_sysinfo(snapshot);
    std::array<uint8_t, sizeof(LinuxSysinfo)> bytes{};
    put_le(bytes.data(), static_cast<uint64_t>(value.uptime), 8);
    for (size_t index = 0; index < value.loads.size(); ++index)
        put_le(bytes.data() + 8 + index * 8, value.loads[index], 8);
    put_le(bytes.data() + 32, value.totalram, 8);
    put_le(bytes.data() + 40, value.freeram, 8);
    put_le(bytes.data() + 48, value.sharedram, 8);
    put_le(bytes.data() + 56, value.bufferram, 8);
    put_le(bytes.data() + 64, value.totalswap, 8);
    put_le(bytes.data() + 72, value.freeswap, 8);
    put_le(bytes.data() + 80, value.procs, 2);
    put_le(bytes.data() + 88, value.totalhigh, 8);
    put_le(bytes.data() + 96, value.freehigh, 8);
    put_le(bytes.data() + 104, value.mem_unit, 4);
    try {
        memory.copy_in(destination, bytes.data(), bytes.size());
    } catch (const MemoryFault &) {
        guest_error(14);
    }
    return 0;
}
int64_t getcpu(Memory &memory, uint64_t cpu_address, uint64_t node_address,
              const CpuLocation &location, uint64_t cache_address) {
    (void)cache_address;
    if (!location.guest_cpu_count || location.guest_cpu >= location.guest_cpu_count ||
        !location.guest_node_count || location.guest_node >= location.guest_node_count)
        guest_error(22);
    // Both stores are attempted independently, as with Linux put_user: a bad
    // second pointer does not undo a valid first output (or vice versa).
    const bool cpu_written = put_cpu_id(memory, cpu_address, location.guest_cpu);
    const bool node_written = put_cpu_id(memory, node_address, location.guest_node);
    if (!cpu_written || !node_written)
        guest_error(14);
    return 0;
}
} // namespace operit::arm64::system
