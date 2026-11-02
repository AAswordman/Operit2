#include "cpu/aarch64/exclusive.h"
#include <atomic>
#include <cstring>
namespace operit::arm64 {
bool execute_exclusive(CpuState &cpu, Memory &memory, uint32_t r) {
    if ((r & 0xfffff0ff) == 0xd503305f) {
        cpu.exclusive_valid = false;
        return true;
    }
    if ((r & 0x3f000000) != 0x08000000)
        return false;
    const unsigned size = 1u << (r >> 30), rn = (r >> 5) & 31, rt = r & 31;
    const unsigned status = (r >> 16) & 31, rt2 = (r >> 10) & 31;
    const bool load = r & (1u << 22), ordered = r & (1u << 23), pair = r & (1u << 21);
    const bool acquire_release = r & (1u << 15);
    // Bit 23 distinguishes the non-exclusive LDAR/STLR forms. Its pair
    // encodings belong to optional atomic extensions, never to LDXP/STXP.
    if ((ordered && (pair || !acquire_release)) || (!pair && rt2 != 31) ||
        ((load || ordered) && status != 31) || (pair && size < 4))
        return false;
    // Select the architecturally permitted UNDEFINED outcome for overlapping
    // constrained-unpredictable operands, before touching memory or registers.
    if ((pair && load && rt == rt2) ||
        (!load && !ordered && status != 31 &&
         (status == rt || (pair && status == rt2) || (rn != 31 && status == rn))))
        return false;
    const unsigned bytes = size * (pair ? 2 : 1);
    const uint64_t address = cpu.get(rn, 64, true);
    if (address % bytes)
        throw Failure("unaligned exclusive access at " + hex(address));
    if (load) {
        const Memory::AtomicAccess access(memory, address, read_perm);
        uint8_t values[16]{};
        memory.copy_out(address, values, bytes);
        uint64_t first = 0, second = 0;
        std::memcpy(&first, values, size);
        if (pair)
            std::memcpy(&second, values + size, size);
        cpu.set(rt, first, size == 8 ? 64 : 32);
        if (pair)
            cpu.set(rt2, second, size == 8 ? 64 : 32);
        if (!ordered) {
            cpu.exclusive_address = address;
            cpu.exclusive_size = bytes;
            cpu.exclusive_epoch = memory.exclusive_epoch(address);
            cpu.exclusive_valid = true;
        }
        if (acquire_release)
            std::atomic_thread_fence(std::memory_order_acquire);
    } else if (ordered) {
        const Memory::AtomicAccess access(memory, address, write_perm);
        std::atomic_thread_fence(std::memory_order_release);
        memory.store(address, cpu.get(rt), size);
    } else {
        bool success = false;
        if (cpu.exclusive_valid && cpu.exclusive_address == address && cpu.exclusive_size == bytes) {
            const Memory::AtomicAccess access(memory, address, read_perm | write_perm);
            success = cpu.exclusive_epoch == memory.exclusive_epoch(address);
            cpu.exclusive_valid = false;
            if (success) {
                if (acquire_release)
                    std::atomic_thread_fence(std::memory_order_release);
                const uint64_t first = cpu.get(rt), second = cpu.get(rt2);
                uint8_t values[16]{};
                std::memcpy(values, &first, size);
                if (pair)
                    std::memcpy(values + size, &second, size);
                // The page lock covers both reservation validation and the
                // complete pair store, including all ordinary accesses.
                memory.copy_in(address, values, bytes);
            }
        } else
            cpu.exclusive_valid = false;
        cpu.set(status, success ? 0 : 1, 32);
    }
    return true;
}
} // namespace operit::arm64
