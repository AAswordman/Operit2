#include "linux/signals/frame.h"
#include <cstddef>
#include <cstring>

namespace operit::arm64::signals {
namespace {
constexpr uint32_t fpsimd_magic = 0x46508001, esr_magic = 0x45535201;
struct ContextHeader {
    uint32_t magic = 0, size = 0;
};
struct alignas(16) FpsimdContext {
    ContextHeader header{fpsimd_magic, 528};
    uint32_t fpsr = 0, fpcr = 0;
    std::array<std::array<uint8_t, 16>, 32> registers{};
};
struct alignas(16) MachineContext {
    uint64_t fault_address = 0;
    std::array<uint64_t, 31> registers{};
    uint64_t sp = 0, pc = 0, pstate = 0;
    alignas(16) std::array<uint8_t, 4096> reserved{};
};
struct alignas(16) UserContext {
    uint64_t flags = 0, link = 0;
    AltStack stack;
    uint64_t signal_mask = 0;
    std::array<uint8_t, 120> unused{};
    MachineContext machine;
};
struct alignas(16) SignalFrame {
    Info info;
    UserContext context;
};
struct FrameRecord {
    uint64_t fp, lr;
};
static_assert(sizeof(FpsimdContext) == 528);
static_assert(offsetof(MachineContext, registers) == 8);
static_assert(offsetof(MachineContext, sp) == 256);
static_assert(offsetof(MachineContext, pc) == 264);
static_assert(offsetof(MachineContext, pstate) == 272);
static_assert(offsetof(MachineContext, reserved) == 288);
static_assert(sizeof(MachineContext) == 4384);
static_assert(offsetof(UserContext, signal_mask) == 40);
static_assert(offsetof(UserContext, machine) == 176);
static_assert(offsetof(SignalFrame, context) == 128);
static_assert(sizeof(SignalFrame) == 4688);
bool physically_on_stack(const AltStack &stack, uint64_t sp) {
    return !(stack.flags & ss_disable) && sp > stack.pointer && sp - stack.pointer <= stack.size;
}
bool on_stack(const AltStack &stack, uint64_t sp) {
    return !(stack.flags & ss_autodisarm) && physically_on_stack(stack, sp);
}
bool valid_altstack(const AltStack &stack, bool restoring) {
    uint32_t flags = stack.flags & ~ss_autodisarm;
    if (flags == ss_disable)
        return true;
    if (flags && !(restoring && flags == ss_onstack))
        return false;
    return stack.pointer < address_limit && stack.size <= address_limit - stack.pointer &&
           stack.size >= minimum_signal_stack;
}
} // namespace

AltStack visible_altstack(const ThreadState &thread, uint64_t sp) {
    auto result = thread.altstack;
    result.flags &= ss_autodisarm;
    if (thread.altstack.flags & ss_disable)
        result.flags |= ss_disable;
    else if (on_stack(thread.altstack, sp))
        result.flags |= ss_onstack;
    return result;
}
void change_altstack(ThreadState &thread, const AltStack &stack, uint64_t sp) {
    if (on_stack(thread.altstack, sp))
        guest_error(1);
    if ((stack.flags & ~ss_autodisarm) != 0 && (stack.flags & ~ss_autodisarm) != ss_disable)
        guest_error(22);
    if (!(stack.flags & ss_disable) && stack.size < minimum_signal_stack)
        guest_error(12);
    if (!valid_altstack(stack, false))
        guest_error(22);
    if (stack.flags & ss_disable)
        thread.altstack = {};
    else {
        thread.altstack = stack;
        thread.altstack.padding = 0;
    }
}
uint64_t install_trampoline(Memory &memory, uint64_t preferred) {
    auto address = align_down(preferred);
    for (unsigned tries = 0; !memory.free(address, page_size); ++tries) {
        if (tries >= 4096 || address < page_size * 2)
            guest_error(12);
        address -= page_size;
    }
    memory.map(address, page_size, read_perm | write_perm);
    const uint32_t instructions[] = {0xd2801168, 0xd4000001, 0xd4200000};
    memory.copy_in(address, instructions, sizeof(instructions));
    memory.protect(address, page_size, read_perm | exec_perm);
    memory.describe(address, page_size, {"[sigtramp]", 0, 0, 0, false});
    return address;
}
bool enter_handler(Memory &memory, CpuState &cpu, ThreadState &thread, uint64_t saved_mask, const Info &info,
                   const Action &action, uint64_t trampoline) {
    const auto stack = thread.altstack;
    uint64_t top = cpu.sp;
    bool use_altstack = on_stack(stack, cpu.sp);
    if ((action.flags & sa_onstack) && !(stack.flags & ss_disable) && !use_altstack) {
        if (stack.size > address_limit - stack.pointer)
            return false;
        top = stack.pointer + stack.size;
        use_altstack = true;
    }
    constexpr uint64_t total = sizeof(SignalFrame) + sizeof(FrameRecord);
    if (top < total)
        return false;
    const uint64_t address = (top - total) & ~uint64_t{15};
    if ((use_altstack && address < stack.pointer) || !memory.accessible(address, total, write_perm))
        return false;
    SignalFrame frame{};
    frame.info = info;
    frame.context.stack = visible_altstack(thread, cpu.sp);
    frame.context.signal_mask = saved_mask & ~unblockable;
    auto &machine = frame.context.machine;
    if (info.code > 0 && (bit(info.signal) & (bit(4) | bit(5) | bit(7) | bit(8) | bit(11))))
        machine.fault_address = info.fault_address();
    machine.registers = cpu.x;
    machine.sp = cpu.sp;
    machine.pc = cpu.pc;
    machine.pstate = uint64_t(cpu.nzcv & 15) << 28;
    FpsimdContext fpsimd;
    fpsimd.fpsr = uint32_t(cpu.fpsr);
    fpsimd.fpcr = uint32_t(cpu.fpcr);
    fpsimd.registers = cpu.v;
    std::memcpy(machine.reserved.data(), &fpsimd, sizeof(fpsimd));
    FrameRecord record{cpu.x[29], cpu.x[30]};
    memory.copy_in(address, &frame, sizeof(frame));
    memory.copy_in(address + sizeof(frame), &record, sizeof(record));
    cpu.x[0] = info.signal;
    if (action.flags & sa_siginfo) {
        cpu.x[1] = address;
        cpu.x[2] = address + offsetof(SignalFrame, context);
    }
    cpu.x[29] = address + sizeof(frame);
    cpu.x[30] = (action.flags & sa_restorer) ? action.restorer : trampoline;
    cpu.sp = address;
    cpu.pc = action.handler;
    cpu.exclusive_valid = false;
    if (stack.flags & ss_autodisarm)
        thread.altstack = {};
    return true;
}
bool restore_frame(Memory &memory, CpuState &cpu, ThreadState &thread, uint64_t &mask) {
    if ((cpu.sp & 15) || !memory.accessible(cpu.sp, sizeof(SignalFrame), read_perm))
        return false;
    SignalFrame frame;
    memory.copy_out(cpu.sp, &frame, sizeof(frame));
    const auto &machine = frame.context.machine;
    // EL0t AArch64 only; no DAIF interrupt masking or EL1 mode may be forged.
    if ((machine.pstate & 0x3df) || (machine.pc & 3) || (machine.sp & 15) || machine.pc >= address_limit ||
        machine.sp >= address_limit || !valid_altstack(frame.context.stack, true))
        return false;
    std::optional<FpsimdContext> fpsimd;
    bool terminated = false;
    size_t offset = 0;
    while (offset + sizeof(ContextHeader) <= machine.reserved.size()) {
        ContextHeader header;
        std::memcpy(&header, machine.reserved.data() + offset, sizeof(header));
        if (!header.magic && !header.size) {
            terminated = true;
            break;
        }
        if (!header.magic || header.size < 16 || (header.size & 15) ||
            header.size > machine.reserved.size() - offset)
            return false;
        if (header.magic == fpsimd_magic) {
            if (fpsimd || header.size != sizeof(FpsimdContext))
                return false;
            FpsimdContext value;
            std::memcpy(&value, machine.reserved.data() + offset, sizeof(value));
            fpsimd = value;
        } else if (header.magic != esr_magic || header.size != 16)
            return false;
        offset += header.size;
    }
    if (!terminated || !fpsimd)
        return false;
    cpu.x = machine.registers;
    cpu.sp = machine.sp;
    cpu.pc = machine.pc;
    cpu.nzcv = uint8_t(machine.pstate >> 28) & 15;
    cpu.fpcr = fpsimd->fpcr;
    cpu.fpsr = fpsimd->fpsr;
    cpu.v = fpsimd->registers;
    cpu.exclusive_valid = false;
    mask = frame.context.signal_mask & ~unblockable;
    thread.altstack = frame.context.stack;
    thread.altstack.flags &= ~ss_onstack;
    thread.altstack.padding = 0;
    if (thread.altstack.flags & ss_disable)
        thread.altstack = {};
    thread.restore_mask.reset();
    thread.interrupted_mask.reset();
    thread.restart.reset();
    thread.wait_set.reset();
    thread.suspending = false;
    return true;
}
} // namespace operit::arm64::signals
