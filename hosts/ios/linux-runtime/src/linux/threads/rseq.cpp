#include "linux/threads/rseq.h"
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
#include "memory/fault.h"

namespace operit::arm64::threads {
namespace {
constexpr uint32_t original_size = rseq_original_size;
constexpr uint32_t unregister_flag = 1;
constexpr uint64_t cs_offset = 8, flags_offset = 16, node_offset = 20, cid_offset = 24;

// Linux access_ok checks the address range, not whether pages are mapped.
bool user_range(uint64_t address, uint64_t size) noexcept {
    return address < address_limit && size <= address_limit - address;
}

struct CriticalSection {
    uint32_t version = 0, flags = 0;
    uint64_t start = 0, length = 0, abort = 0;
};

bool fixup(Memory &memory, CpuState &cpu, RseqState &state) {
    const uint64_t pointer = memory.load(state.address + cs_offset, 8);
    CriticalSection cs;
    if (pointer) {
        if (pointer >= address_limit)
            return false;
        cs.version = uint32_t(memory.load(pointer, 4));
        cs.flags = uint32_t(memory.load(pointer + 4, 4));
        cs.start = memory.load(pointer + 8, 8);
        cs.length = memory.load(pointer + 16, 8);
        cs.abort = memory.load(pointer + 24, 8);
        if (cs.version || cs.start >= address_limit || cs.length >= address_limit - cs.start ||
            cs.abort >= address_limit || cs.abort - cs.start < cs.length)
            return false;
        if (uint32_t(memory.load(cs.abort - 4, 4)) != state.signature)
            return false;
    }
    if (cpu.pc - cs.start >= cs.length) {
        state.events = 0;
        memory.store(state.address + cs_offset, 0, 8);
        return true;
    }
    // Since v6.12 even the formerly supported NO_RESTART flags are rejected.
    if (cs.flags || memory.load(state.address + flags_offset, 4))
        return false;
    const uint32_t events = state.events;
    state.events = 0;
    if (events) {
        memory.store(state.address + cs_offset, 0, 8);
        cpu.pc = cs.abort;
    }
    return true;
}
} // namespace

int64_t rseq_syscall(Memory &memory, RseqState &state, uint64_t address,
                     uint32_t length, uint32_t flags, uint32_t signature) {
    if (flags & unregister_flag) {
        if ((flags & ~unregister_flag) || !state.address || state.address != address ||
            state.length != length)
            return -22;
        if (state.signature != signature)
            return -1;
        try {
            const Memory::AccessGuard guard(memory);
            memory.store(address, 0, 4);
            memory.store(address + 4, UINT32_MAX, 4);
            memory.store(address + node_offset, 0, 4);
            memory.store(address + cid_offset, 0, 4);
        } catch (const MemoryFault &) {
            return -14;
        } catch (const GuestError &) {
            return -14;
        }
        state.address = 0;
        state.length = state.signature = state.events = 0;
        state.notify = false;
        return 0;
    }
    if (flags)
        return -22;
    if (state.address) {
        if (state.address != address || state.length != length)
            return -22;
        if (state.signature != signature)
            return -1;
        return -16;
    }
    if (length < original_size || (address & (original_size - 1)))
        return -22;
    if (!user_range(address, length))
        return -14;
    state.address = address;
    state.length = length;
    state.signature = signature;
    state.events = 0;
    state.notify = address != 0;
    return 0;
}

void rseq_event(RseqState &state, RseqEvent event) noexcept {
    if (!state.address)
        return;
    state.events |= static_cast<uint32_t>(event);
    state.notify = true;
}

bool rseq_resume(Memory &memory, CpuState &cpu, RseqState &state,
                 uint32_t cpu_id, uint32_t mm_cid, bool in_syscall) {
    if (!state.address || !state.notify)
        return true;
    state.notify = false;
    try {
        const Memory::AccessGuard guard(memory);
        if (!in_syscall && !fixup(memory, cpu, state))
            return false;
        memory.store(state.address, cpu_id, 4);
        memory.store(state.address + 4, cpu_id, 4);
        memory.store(state.address + node_offset, 0, 4);
        memory.store(state.address + cid_offset, mm_cid, 4);
        // If a kernel-only notification skipped register fixup, ensure its
        // events are handled before the next actual user-mode return.
        if (in_syscall && state.events)
            state.notify = true;
        return true;
    } catch (const MemoryFault &) {
        return false;
    } catch (const GuestError &) {
        return false;
    }
}

RseqState rseq_fork(const RseqState &parent, bool clone_vm) noexcept {
    if (clone_vm)
        return {};
    RseqState child = parent;
    // The new task must publish its selected CPU before it executes.
    child.notify = child.address != 0;
    return child;
}

void rseq_reset(RseqState &state) noexcept {
    state = {};
}
} // namespace operit::arm64::threads
