#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace operit::arm64 {
class Memory;
class Scheduler;
}

namespace operit::arm64::system {
// Linux LP64 AArch64 ABI. Padding is explicit and is always zero on output.
struct alignas(8) LinuxSysinfo {
    int64_t uptime;
    std::array<uint64_t, 3> loads;
    uint64_t totalram, freeram, sharedram, bufferram, totalswap, freeswap;
    uint16_t procs, pad;
    uint32_t alignment;
    uint64_t totalhigh, freehigh;
    uint32_t mem_unit, reserved;
};
static_assert(sizeof(LinuxSysinfo) == 112);
static_assert(alignof(LinuxSysinfo) == 8);
static_assert(offsetof(LinuxSysinfo, uptime) == 0);
static_assert(offsetof(LinuxSysinfo, loads) == 8);
static_assert(offsetof(LinuxSysinfo, totalram) == 32);
static_assert(offsetof(LinuxSysinfo, freeram) == 40);
static_assert(offsetof(LinuxSysinfo, sharedram) == 48);
static_assert(offsetof(LinuxSysinfo, bufferram) == 56);
static_assert(offsetof(LinuxSysinfo, totalswap) == 64);
static_assert(offsetof(LinuxSysinfo, freeswap) == 72);
static_assert(offsetof(LinuxSysinfo, procs) == 80);
static_assert(offsetof(LinuxSysinfo, pad) == 82);
static_assert(offsetof(LinuxSysinfo, alignment) == 84);
static_assert(offsetof(LinuxSysinfo, totalhigh) == 88);
static_assert(offsetof(LinuxSysinfo, freehigh) == 96);
static_assert(offsetof(LinuxSysinfo, mem_unit) == 104);
static_assert(offsetof(LinuxSysinfo, reserved) == 108);

struct MemoryInformation {
    uint64_t total_bytes = 0, resident_bytes = 0;
    uint64_t shared_bytes = 0, buffer_bytes = 0;
    uint64_t swap_bytes = 0, free_swap_bytes = 0;
};
struct TaskInformation {
    uint64_t processes = 0, live_tasks = 0, zombie_tasks = 0;
    uint64_t runnable_tasks = 0, waiting_tasks = 0, stopped_tasks = 0;
};
// Capture these together from the guest executor while its topology is stable.
// CPU indices are guest IDs, never host hardware IDs. A single NUMA node is the
// default; executors with multiple guest nodes supply their actual node IDs.
struct CpuLocation {
    uint32_t guest_cpu, guest_cpu_count;
    uint32_t guest_node = 0, guest_node_count = 1;
};
struct Snapshot {
    uint64_t uptime_nanoseconds = 0;
    std::array<uint64_t, 3> loads{}; // 16 fractional bits, as in Linux sysinfo.
    MemoryInformation memory;
    TaskInformation tasks;
};
// Own one state per VM, shared by fork/exec and all syscall contexts. Observe at
// scheduling/state-change boundaries, including wait/idle, to retain load history.
// Each previous runnable count applies until the next observation. The three
// averages are real-time exponential averages over 1, 5 and 15 minutes.
struct SamplingState {
    bool started = false;
    uint64_t boot_nanoseconds = 0, observed_nanoseconds = 0, runnable_tasks = 0;
    std::array<long double, 3> loads{};
};

// A continuous monotonic clock including host suspend, used only as a timebase;
// uptime is its difference from the VM's boot timestamp, never host uptime.
uint64_t information_clock_nanoseconds();
SamplingState start_information(const Scheduler &scheduler,
                                uint64_t now_nanoseconds = information_clock_nanoseconds());
TaskInformation task_information(const Scheduler &scheduler);
// Memory::limit() is the effective config/default resident budget. Count only
// whole guest pages. resident_bytes follows Memory::resident() accounting, not
// virtual mappings or the host application's RSS. This allocator has no swap,
// separate block buffers or highmem pool; shared_bytes denotes a shmem pool,
// not MAP_SHARED aliases. An owner with such pools can supply their counters.
MemoryInformation memory_information(const Memory &memory);
// For the current per-address-space budget model: sum distinct Memory objects'
// budgets and resident charges. CLONE_VM aliases are counted once. COW pages in
// distinct address spaces remain charged once per space, as in Memory::resident.
MemoryInformation memory_information(const std::vector<const Memory *> &address_spaces);
void observe_information(SamplingState &state, const TaskInformation &tasks,
                         uint64_t now_nanoseconds = information_clock_nanoseconds());
void observe_information(SamplingState &state, const Scheduler &scheduler,
                         uint64_t now_nanoseconds = information_clock_nanoseconds());
Snapshot snapshot_information(SamplingState &state, const MemoryInformation &memory,
                              const TaskInformation &tasks,
                              uint64_t now_nanoseconds = information_clock_nanoseconds());
Snapshot snapshot_information(SamplingState &state, const Memory &memory, const Scheduler &scheduler,
                              uint64_t now_nanoseconds = information_clock_nanoseconds());
LinuxSysinfo make_sysinfo(const Snapshot &snapshot);

// Syscall 179. Writes all 112 bytes in guest little-endian order, including zero
// padding. Errors use GuestError (Linux errno), as does the Syscalls dispatcher.
int64_t sysinfo(Memory &memory, uint64_t destination, const Snapshot &snapshot);
// Syscall 168. The executor supplies the actual guest CPU/node and their counts;
// zero counts or out-of-range IDs are EINVAL. Either output pointer can be NULL.
// The obsolete cache is ignored, including its address. Pointer faults are EFAULT.
int64_t getcpu(Memory &memory, uint64_t cpu_address, uint64_t node_address,
              const CpuLocation &location, uint64_t cache_address = 0);
} // namespace operit::arm64::system
