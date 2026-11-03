#pragma once
#include "memory/address_space.h"
#include <algorithm>

namespace operit::arm64::threads {
// Linux's versioned, little-endian sched_attr ABI. Utilization controls and
// deadline/custom runtime scheduling are not implemented by this scheduler.
struct SchedulingAttributes {
    static constexpr uint32_t version0_size = 48, current_size = 56;
    static constexpr uint64_t reset_on_fork = 1, reclaim = 2, deadline_overrun = 4;
    static constexpr uint64_t keep_policy = 8, keep_parameters = 16;
    static constexpr uint64_t utilization = 32 | 64, known_flags = 127;
    uint32_t size = current_size, policy = 0;
    uint64_t flags = 0;
    int32_t nice = 0;
    uint32_t priority = 0;
    uint64_t runtime = 0, deadline = 0, period = 0;
    uint32_t utilization_min = 0, utilization_max = 0;

    static SchedulingAttributes read(Memory &memory, uint64_t address) {
        const Memory::AccessGuard access(memory);
        if (!memory.accessible(address, 4, read_perm))
            guest_error(14);
        uint32_t size = uint32_t(memory.load(address, 4));
        if (!size)
            size = version0_size;
        auto size_error = [&] {
            // Linux attempts to report the supported size even if that store
            // faults; the primary error remains E2BIG.
            if (memory.accessible(address, 4, write_perm))
                memory.store(address, current_size, 4);
            guest_error(7);
        };
        if (size < version0_size || size > page_size)
            size_error();
        if (!memory.accessible(address, size, read_perm))
            guest_error(14);
        for (uint32_t offset = current_size; offset < size; ++offset)
            if (memory.load(address + offset, 1))
                size_error();
        SchedulingAttributes value;
        value.size = size;
        value.policy = uint32_t(memory.load(address + 4, 4));
        value.flags = memory.load(address + 8, 8);
        value.nice = std::clamp(int32_t(memory.load(address + 16, 4)), int32_t{-20}, int32_t{19});
        value.priority = uint32_t(memory.load(address + 20, 4));
        value.runtime = memory.load(address + 24, 8);
        value.deadline = memory.load(address + 32, 8);
        value.period = memory.load(address + 40, 8);
        // Intermediate ABI sizes are valid too; missing bytes are zero-filled.
        for (uint32_t offset = version0_size; offset < std::min(size, current_size); ++offset) {
            const auto byte = uint32_t(memory.load(address + offset, 1));
            if (offset < 52)
                value.utilization_min |= byte << (8 * (offset - 48));
            else
                value.utilization_max |= byte << (8 * (offset - 52));
        }
        if ((value.flags & utilization) && size < current_size)
            guest_error(22);
        return value;
    }
    void write(Memory &memory, uint64_t address, uint32_t requested_size) const {
        const Memory::AccessGuard access(memory);
        const uint32_t count = std::min(requested_size, current_size);
        if (!Memory::valid_range(address, requested_size) || !memory.accessible(address, count, write_perm))
            guest_error(14);
        memory.store(address, count, 4);
        memory.store(address + 4, policy, 4);
        memory.store(address + 8, flags, 8);
        memory.store(address + 16, uint32_t(nice), 4);
        memory.store(address + 20, priority, 4);
        memory.store(address + 24, runtime, 8);
        memory.store(address + 32, deadline, 8);
        memory.store(address + 40, period, 8);
        for (uint32_t offset = version0_size; offset < count; ++offset) {
            const auto value = offset < 52 ? utilization_min : utilization_max;
            const auto shift = 8 * (offset < 52 ? offset - 48 : offset - 52);
            memory.store(address + offset, uint8_t(value >> shift), 1);
        }
    }
};
} // namespace operit::arm64::threads
