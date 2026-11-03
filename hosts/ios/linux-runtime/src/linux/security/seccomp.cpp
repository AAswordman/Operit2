#include "linux/security/seccomp.h"

#include "cpu/aarch64/state.h"
#include "linux/signals/signals.h"
#include "memory/address_space.h"
#include "runtime/tasks/scheduler.h"
#include <algorithm>
#include <array>
#include <cstddef>
#include <new>
#include <utility>
#include <vector>

namespace operit::arm64::security {
namespace {
constexpr uint32_t audit_arch_aarch64 = 0xc00000b7;
constexpr uint32_t ret_kill_process = 0x80000000;
constexpr uint32_t ret_kill_thread = 0x00000000;
constexpr uint32_t ret_trap = 0x00030000;
constexpr uint32_t ret_errno = 0x00050000;
constexpr uint32_t ret_user_notif = 0x7fc00000;
constexpr uint32_t ret_trace = 0x7ff00000;
constexpr uint32_t ret_log = 0x7ffc0000;
constexpr uint32_t ret_allow = 0x7fff0000;
constexpr uint32_t action_mask = 0xffff0000;
constexpr size_t maximum_program_length = 4096;
constexpr size_t maximum_path_length = 32768;
constexpr size_t data_size = 64;

struct Instruction {
    uint16_t code;
    uint8_t jt, jf;
    uint32_t k;
};

uint64_t little_endian(const uint8_t *bytes, size_t count) {
    uint64_t value = 0;
    for (size_t i = 0; i < count; ++i)
        value |= uint64_t(bytes[i]) << (8 * i);
    return value;
}

void store_little_endian(uint8_t *bytes, uint64_t value, size_t count) {
    for (size_t i = 0; i < count; ++i)
        bytes[i] = uint8_t(value >> (8 * i));
}

// Linux first checks classic BPF, then seccomp_check_filter's smaller opcode
// set. The separate scratch-word pass mirrors check_load_and_stores().
bool valid_program(const std::vector<Instruction> &program) {
    const size_t size = program.size();
    if (!size || size > maximum_program_length)
        return false;
    for (size_t pc = 0; pc < size; ++pc) {
        const auto &i = program[pc];
        switch (i.code) {
        case 0x20: // LD W ABS
            if (i.k >= data_size || (i.k & 3))
                return false;
            break;
        case 0x60: case 0x61: case 0x02: case 0x03: // scratch load/store
            if (i.k >= 16)
                return false;
            break;
        case 0x34: // DIV K
            if (!i.k)
                return false;
            break;
        case 0x64: case 0x74: // shift by K
            if (i.k >= 32)
                return false;
            break;
        case 0x05: // JA
            if (i.k >= size - pc - 1)
                return false;
            break;
        case 0x15: case 0x1d: case 0x25: case 0x2d:
        case 0x35: case 0x3d: case 0x45: case 0x4d:
            if (i.jt >= size - pc - 1 || i.jf >= size - pc - 1)
                return false;
            break;
        case 0x00: case 0x01: case 0x80: case 0x81: // IMM, LEN
        case 0x04: case 0x0c: case 0x14: case 0x1c: // ADD, SUB
        case 0x24: case 0x2c: case 0x3c:             // MUL, DIV X
        case 0x44: case 0x4c: case 0x54: case 0x5c: // OR, AND
        case 0x6c: case 0x7c: case 0x84:             // shifts X, NEG
        case 0xa4: case 0xac:                         // XOR
        case 0x06: case 0x16: case 0x07: case 0x87: // RET, TAX, TXA
            break;
        default:
            return false;
        }
    }
    if (program.back().code != 0x06 && program.back().code != 0x16)
        return false;

    std::vector<uint16_t> masks(size, UINT16_MAX);
    uint16_t initialized = 0;
    for (size_t pc = 0; pc < size; ++pc) {
        const auto &i = program[pc];
        initialized &= masks[pc];
        switch (i.code) {
        case 0x02: case 0x03:
            initialized |= uint16_t(1u << i.k);
            break;
        case 0x60: case 0x61:
            if (!(initialized & uint16_t(1u << i.k)))
                return false;
            break;
        case 0x05:
            masks[pc + 1 + i.k] &= initialized;
            initialized = UINT16_MAX;
            break;
        case 0x15: case 0x1d: case 0x25: case 0x2d:
        case 0x35: case 0x3d: case 0x45: case 0x4d:
            masks[pc + 1 + i.jt] &= initialized;
            masks[pc + 1 + i.jf] &= initialized;
            initialized = UINT16_MAX;
            break;
        }
    }
    return true;
}

uint32_t data_word(const CpuState &cpu, uint32_t offset) {
    if (offset == 0)
        return uint32_t(cpu.x[8]);
    if (offset == 4)
        return audit_arch_aarch64;
    if (offset == 8 || offset == 12)
        return uint32_t(cpu.pc >> (offset == 12 ? 32 : 0));
    const uint32_t argument = (offset - 16) / 8;
    return uint32_t(cpu.x[argument] >> ((offset & 4) ? 32 : 0));
}

uint32_t run_program(const std::vector<Instruction> &program, const CpuState &cpu) {
    uint32_t a = 0, x = 0;
    std::array<uint32_t, 16> scratch{};
    for (size_t pc = 0; pc < program.size(); ++pc) {
        const auto &i = program[pc];
        const uint32_t rhs = (i.code & 8) ? x : i.k;
        switch (i.code) {
        case 0x00: a = i.k; break;
        case 0x01: x = i.k; break;
        case 0x20: a = data_word(cpu, i.k); break;
        case 0x60: a = scratch[i.k]; break;
        case 0x61: x = scratch[i.k]; break;
        case 0x80: a = data_size; break;
        case 0x81: x = data_size; break;
        case 0x02: scratch[i.k] = a; break;
        case 0x03: scratch[i.k] = x; break;
        case 0x04: case 0x0c: a += rhs; break;
        case 0x14: case 0x1c: a -= rhs; break;
        case 0x24: case 0x2c: a *= rhs; break;
        case 0x34: case 0x3c:
            if (!rhs)
                return ret_kill_thread;
            a /= rhs;
            break;
        case 0x44: case 0x4c: a |= rhs; break;
        case 0x54: case 0x5c: a &= rhs; break;
        case 0x64: case 0x6c: a <<= rhs & 31; break;
        case 0x74: case 0x7c: a >>= rhs & 31; break;
        case 0x84: a = uint32_t(0) - a; break;
        case 0xa4: case 0xac: a ^= rhs; break;
        case 0x05: pc += i.k; break;
        case 0x15: case 0x1d: pc += a == rhs ? i.jt : i.jf; break;
        case 0x25: case 0x2d: pc += a > rhs ? i.jt : i.jf; break;
        case 0x35: case 0x3d: pc += a >= rhs ? i.jt : i.jf; break;
        case 0x45: case 0x4d: pc += (a & rhs) ? i.jt : i.jf; break;
        case 0x06: return i.k;
        case 0x16: return a;
        case 0x07: x = a; break;
        case 0x87: a = x; break;
        default: return ret_kill_process;
        }
    }
    return ret_kill_process;
}
} // namespace

struct SeccompFilter {
    std::vector<Instruction> program;
    std::shared_ptr<const SeccompFilter> previous;
    size_t path_length;

    SeccompFilter(std::vector<Instruction> instructions, std::shared_ptr<const SeccompFilter> parent,
                  size_t length)
        : program(std::move(instructions)), previous(std::move(parent)), path_length(length) {}

    ~SeccompFilter() {
        // Every node was allocated non-const. Detach uniquely owned parents
        // before releasing them, so a long shared chain cannot exhaust the host stack.
        while (previous && previous.use_count() == 1) {
            auto next = std::move(const_cast<SeccompFilter *>(previous.get())->previous);
            previous.reset();
            previous = std::move(next);
        }
    }
};

struct SeccompAccess {
    static const std::shared_ptr<const SeccompFilter> &filter(const SeccompState &state) { return state.filter_; }

    static int64_t install(SeccompState &state, Memory &memory, uint64_t address, Scheduler *scheduler,
                           bool synchronize, bool esrch) {
        if (synchronize && (!scheduler || &scheduler->current().seccomp != &state ||
                            scheduler->current().state == Task::State::Exited))
            return -95;
        std::array<uint8_t, 16> header{};
        memory.copy_out(address, header.data(), header.size());
        const size_t length = little_endian(header.data(), 2);
        if (!length || length > maximum_program_length)
            return -22;
        if (!state.no_new_privileges_)
            return -13; // Guest CAP_SYS_ADMIN is not available.

        const uint64_t instructions = little_endian(header.data() + 8, 8);
        std::vector<uint8_t> bytes(length * 8);
        memory.copy_out(instructions, bytes.data(), bytes.size());
        std::vector<Instruction> program;
        program.reserve(length);
        for (size_t n = 0; n < length; ++n) {
            const auto *p = bytes.data() + n * 8;
            program.push_back({uint16_t(little_endian(p, 2)), p[2], p[3], uint32_t(little_endian(p + 4, 4))});
        }
        if (!valid_program(program))
            return -22;
        const size_t previous_length = state.filter_ ? state.filter_->path_length + 4 : 0;
        if (length + previous_length > maximum_path_length)
            return -12;

        std::vector<uint32_t> tids;
        if (synchronize) {
            // Syscall dispatch and task creation/retirement run on the same
            // coordinator after the guest workers' barrier. Keep that boundary
            // through validation and the non-throwing commit below.
            tids = scheduler->live_tids(scheduler->current().pid);
            for (const auto tid : tids) {
                const auto &peer = scheduler->lookup(tid)->seccomp;
                if (&peer == &state || !peer.filter_)
                    continue;
                auto ancestor = state.filter_.get();
                while (ancestor && ancestor != peer.filter_.get())
                    ancestor = ancestor->previous.get();
                if (!ancestor)
                    return esrch ? -3 : int64_t(tid);
            }
        }
        auto filter = std::make_shared<SeccompFilter>(std::move(program), state.filter_,
                                                       length + previous_length);
        if (synchronize) {
            for (const auto tid : tids) {
                auto &peer = scheduler->lookup(tid)->seccomp;
                peer.filter_ = filter;
                if (state.no_new_privileges_)
                    peer.no_new_privileges_ = true;
            }
        } else {
            state.filter_ = std::move(filter);
        }
        return 0;
    }
};

SeccompDecision seccomp_evaluate(const SeccompState &state, const CpuState &cpu) {
    uint32_t best = ret_allow;
    for (auto filter = SeccompAccess::filter(state); filter; filter = filter->previous) {
        const uint32_t result = run_program(filter->program, cpu);
        // ACTION_ONLY is a signed 32-bit comparison, not unsigned ordering.
        if (((result & action_mask) ^ 0x80000000u) < ((best & action_mask) ^ 0x80000000u))
            best = result;
    }
    const uint16_t data = uint16_t(best);
    switch (best & action_mask) {
    case ret_allow: return {SeccompAction::Allow, 0, data};
    case ret_log: return {SeccompAction::Log, 0, data};
    case ret_errno: return {SeccompAction::Errno, -int64_t(std::min<uint32_t>(data, 4095)), data};
    case ret_trap: return {SeccompAction::Trap, 0, data};
    case ret_kill_thread: return {SeccompAction::KillThread, 0, data};
    case ret_kill_process: return {SeccompAction::KillProcess, 0, data};
    case ret_user_notif: case ret_trace:
        // There is neither an attached listener nor a seccomp ptrace event.
        return {SeccompAction::Errno, -38, data};
    default: return {SeccompAction::KillProcess, 0, data};
    }
}

void seccomp_force_trap(signals::ProcessState &process, signals::ThreadState &thread, uint64_t &signal_mask,
                        const CpuState &cpu, uint16_t data) {
    constexpr int sigsys = 31;
    auto &action = process.actions->entries[sigsys - 1];
    if (action.handler == 1 || (signal_mask & signals::bit(sigsys)))
        action = {};
    signal_mask &= ~signals::bit(sigsys);
    signals::Info info;
    info.signal = sigsys;
    info.error = data;
    info.code = 1; // SYS_SECCOMP
    store_little_endian(info.payload.data(), cpu.pc, 8);
    store_little_endian(info.payload.data() + 8, uint32_t(cpu.x[8]), 4);
    store_little_endian(info.payload.data() + 12, audit_arch_aarch64, 4);
    thread.restart.reset();
    if (!thread.pending.enqueue(info, true))
        throw Failure("cannot queue forced seccomp SIGSYS");
}

int64_t seccomp_syscall(SeccompState &state, Memory &memory, uint64_t op_arg, uint64_t flags_arg,
                        uint64_t args, Scheduler *scheduler) {
    const uint32_t op = uint32_t(op_arg), flags = uint32_t(flags_arg);
    try {
        switch (op) {
        case 0: // SECCOMP_SET_MODE_STRICT is not implemented.
            return flags || args ? -22 : -95;
        case 1: // SECCOMP_SET_MODE_FILTER
            if (flags & ~uint32_t{63})
                return -22;
            if (flags & ~uint32_t{1 | 16})
                return -95;
            return SeccompAccess::install(state, memory, args, scheduler, flags & 1, flags & 16);
        case 2: { // SECCOMP_GET_ACTION_AVAIL
            if (flags)
                return -22;
            std::array<uint8_t, 4> raw{};
            memory.copy_out(args, raw.data(), raw.size());
            switch (uint32_t(little_endian(raw.data(), raw.size()))) {
            case ret_kill_process: case ret_kill_thread: case ret_trap: case ret_errno:
            case ret_log: case ret_allow:
                return 0;
            default:
                return -95;
            }
        }
        case 3: { // SECCOMP_GET_NOTIF_SIZES; ABI sizes are independent of listener support.
            if (flags)
                return -22;
            std::array<uint8_t, 6> sizes{};
            store_little_endian(sizes.data(), 80, 2);
            store_little_endian(sizes.data() + 2, 24, 2);
            store_little_endian(sizes.data() + 4, data_size, 2);
            memory.copy_in(args, sizes.data(), sizes.size());
            return 0;
        }
        default:
            return -22;
        }
    } catch (const std::bad_alloc &) {
        return -12;
    }
}

int64_t seccomp_prctl(SeccompState &state, Memory &memory, uint64_t option, uint64_t arg2, uint64_t arg3,
                      uint64_t arg4, uint64_t arg5) {
    switch (uint32_t(option)) {
    case 21: return state.mode();
    case 22:
        if (arg2 == 1)
            return -95;
        if (arg2 == 2)
            return seccomp_syscall(state, memory, 1, 0, arg3);
        return -22;
    case 38:
        if (arg2 != 1 || arg3 || arg4 || arg5)
            return -22;
        state.no_new_privileges_ = true;
        return 0;
    case 39:
        if (arg2 || arg3 || arg4 || arg5)
            return -22;
        return state.no_new_privileges_ ? 1 : 0;
    default:
        return -22;
    }
}
} // namespace operit::arm64::security
