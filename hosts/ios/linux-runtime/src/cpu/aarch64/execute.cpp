#include "cpu/aarch64/cache.h"
#include "cpu/aarch64/exclusive.h"
#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/instruction_fault.h"
#include "cpu/aarch64/interpreter.h"
#include "cpu/aarch64/simd.h"
#include "cpu/aarch64/vector_memory.h"
#include <atomic>
#include <chrono>

namespace operit::arm64 {
namespace {
[[noreturn]] void unsupported(uint32_t raw) {
    throw Failure("unsupported AArch64 instruction " + hex(raw), ORT_ARM64_UNSUPPORTED);
}
uint64_t shifted(uint64_t value, unsigned type, unsigned count, unsigned bits) {
    value &= mask(bits);
    if (count >= bits)
        throw Failure("invalid shift", ORT_ARM64_UNSUPPORTED);
    switch (type) {
    case 0:
        return (value << count) & mask(bits);
    case 1:
        return value >> count;
    case 2:
        return static_cast<uint64_t>(sign_extend(value, bits) >> count) & mask(bits);
    default:
        return rotate(value, count, bits);
    }
}
std::pair<uint64_t, uint64_t> bit_masks(uint32_t raw, unsigned bits, bool logical) {
    unsigned n = (raw >> 22) & 1, s = (raw >> 10) & 63, r = (raw >> 16) & 63;
    unsigned selector = (n << 6) | ((~s) & 63), len = 0;
    if (selector < 2 || (bits == 32 && n))
        unsupported(raw);
    while ((selector >> (len + 1)) != 0)
        ++len;
    unsigned levels = (1u << len) - 1, element = 1u << len;
    s &= levels;
    r &= levels;
    if (logical && s == levels)
        unsupported(raw);
    const uint64_t w = rotate(mask(s + 1), r, element), t = mask(((s - r) & levels) + 1);
    uint64_t wm = 0, tm = 0;
    for (unsigned offset = 0; offset < bits; offset += element) {
        wm |= w << offset;
        tm |= t << offset;
    }
    return {wm, tm};
}
} // namespace

bool execute(CpuState &cpu, Memory &memory, const Instruction &i) {
    const uint32_t r = i.raw;
    const unsigned bits = i.width;
    uint64_t next = cpu.pc + 4;
    bool syscall = false;
    auto get = [&cpu, bits](unsigned index) { return cpu.get(index, bits); };
    auto set = [&cpu, bits](unsigned index, uint64_t value) { cpu.set(index, value, bits); };
    switch (i.op) {
    case Op::Fault:
        throw InstructionFault(cpu.pc, r, static_cast<InstructionFault::Kind>(i.flags));
    case Op::Hint:
        // Guest HWCAP does not expose PAuth: SP signing/authentication HINTs are architectural NOPs.
        // NOP/YIELD and BTI are also safe hints. WFE/WFI still need a scheduler.
        if (((r >> 5) & 127) != 0 && ((r >> 5) & 127) != 1 && ((r >> 5) & 127) != 32 &&
            ((r >> 5) & 127) != 34 && ((r >> 5) & 127) != 36 && ((r >> 5) & 127) != 38 &&
            ((r >> 5) & 127) != 25 && ((r >> 5) & 127) != 27 && ((r >> 5) & 127) != 29 &&
            ((r >> 5) & 127) != 31 && ((r >> 5) & 127) != 7)
            unsupported(r);
        break;
    case Op::Svc:
        if ((r >> 5) & 0xffff)
            unsupported(r);
        syscall = true;
        break;
    case Op::Branch:
        if (r >> 31)
            cpu.x[30] = next;
        next = cpu.pc + i.immediate;
        break;
    case Op::BranchRegister: {
        uint64_t target = cpu.get(i.rn);
        if ((r & 0xfffffc1f) == 0xd63f0000)
            cpu.x[30] = next;
        next = target;
        break;
    }
    case Op::ConditionalBranch:
        if (cpu.condition(r & 15))
            next = cpu.pc + i.immediate;
        break;
    case Op::CompareBranch:
        if ((get(i.rd) == 0) != bool((r >> 24) & 1))
            next = cpu.pc + i.immediate;
        break;
    case Op::TestBranch: {
        unsigned bit = ((r >> 19) & 31) | ((r >> 26) & 32);
        if (((cpu.get(i.rd) >> bit) & 1) == ((r >> 24) & 1))
            next = cpu.pc + i.immediate;
        break;
    }
    case Op::Address:
        cpu.set(i.rd, (r >> 31) ? (cpu.pc & ~uint64_t{4095}) + (i.immediate << 12) : cpu.pc + i.immediate);
        break;
    case Op::MoveWide: {
        unsigned shift = ((r >> 21) & 3) * 16, op = (r >> 29) & 3;
        if (shift >= bits || op == 1)
            unsupported(r);
        uint64_t value = uint64_t((r >> 5) & 0xffff) << shift;
        if (op == 0)
            value = ~value;
        if (op == 3)
            value = (get(i.rd) & ~(uint64_t{0xffff} << shift)) | value;
        set(i.rd, value);
        break;
    }
    case Op::AddImmediate: {
        bool subtract = (r >> 30) & 1, flags = (r >> 29) & 1;
        uint64_t value =
            cpu.add(cpu.get(i.rn, bits, true), subtract ? ~i.immediate : i.immediate, subtract, bits, flags);
        cpu.set(i.rd, value, bits, !flags);
        break;
    }
    case Op::AddRegister: {
        bool subtract = (r >> 30) & 1, flags = (r >> 29) & 1, extended = (r >> 21) & 1;
        uint64_t value;
        unsigned shift = (r >> 10) & 63;
        if (extended) {
            shift = (r >> 10) & 7;
            unsigned option = (r >> 13) & 7, width = 8u << (option & 3);
            if (shift > 4 || ((r >> 22) & 3))
                unsupported(r);
            value = cpu.get(i.rm) & mask(width);
            if (option & 4)
                value = sign_extend(value, width);
            value <<= shift;
        } else {
            if (((r >> 22) & 3) == 3)
                unsupported(r);
            value = shifted(get(i.rm), (r >> 22) & 3, shift, bits);
        }
        value = cpu.add(cpu.get(i.rn, bits, extended), subtract ? ~value : value, subtract, bits, flags);
        cpu.set(i.rd, value, bits, extended && !flags);
        break;
    }
    case Op::AddCarry: {
        bool subtract = (r >> 30) & 1;
        uint64_t value = get(i.rm);
        set(i.rd, cpu.add(get(i.rn), subtract ? ~value : value, cpu.nzcv & 2, bits, (r >> 29) & 1));
        break;
    }
    case Op::LogicalRegister:
    case Op::LogicalImmediate: {
        uint64_t rhs = i.op == Op::LogicalImmediate ? bit_masks(r, bits, true).first
                                                    : shifted(get(i.rm), (r >> 22) & 3, (r >> 10) & 63, bits);
        if (i.op == Op::LogicalRegister && (r & (1u << 21)))
            rhs = ~rhs;
        unsigned op = (r >> 29) & 3;
        uint64_t lhs = get(i.rn), value = op == 1 ? lhs | rhs : (op == 2 ? lhs ^ rhs : lhs & rhs);
        if (op == 3)
            cpu.logical_flags(value, bits);
        cpu.set(i.rd, value, bits, i.op == Op::LogicalImmediate && op != 3);
        break;
    }
    case Op::Bitfield: {
        unsigned op = (r >> 29) & 3, s = (r >> 10) & 63, shift = (r >> 16) & 63;
        if (op == 3 || ((r >> 22) & 1) != (bits == 64) || s >= bits || shift >= bits)
            unsupported(r);
        auto masks = bit_masks(r, bits, false);
        uint64_t source = get(i.rn), dest = op == 1 ? get(i.rd) : 0;
        uint64_t bot = (dest & ~masks.first) | (rotate(source, shift, bits) & masks.first);
        uint64_t top = op == 0 && ((source >> s) & 1) ? UINT64_MAX : dest;
        set(i.rd, (top & ~masks.second) | (bot & masks.second));
        break;
    }
    case Op::Extract: {
        unsigned shift = (r >> 10) & 63;
        if (shift >= bits || ((r >> 22) & 1) != (bits == 64))
            unsupported(r);
        set(i.rd, shift ? (get(i.rm) >> shift) | (get(i.rn) << (bits - shift)) : get(i.rm));
        break;
    }
    case Op::ConditionalSelect: {
        uint64_t value = get(i.rn);
        if (!cpu.condition((r >> 12) & 15)) {
            value = get(i.rm);
            if (r & (1u << 30))
                value = ~value;
            if (r & (1u << 10))
                ++value;
        }
        set(i.rd, value);
        break;
    }
    case Op::ConditionalCompare:
        if (!(r & (1u << 29)))
            unsupported(r);
        if (cpu.condition((r >> 12) & 15)) {
            uint64_t rhs = (r & (1u << 11)) ? i.rm : get(i.rm);
            bool subtract = (r >> 30) & 1;
            cpu.add(get(i.rn), subtract ? ~rhs : rhs, subtract, bits, true);
        } else
            cpu.nzcv = r & 15;
        break;
    case Op::TwoSource: {
        uint64_t lhs = get(i.rn), rhs = get(i.rm), value;
        unsigned op = (r >> 10) & 63;
        if (op == 2)
            value = rhs ? lhs / rhs : 0;
        else if (op == 3) {
            int64_t a = sign_extend(lhs, bits), b = sign_extend(rhs, bits);
            value = b == 0 ? 0 : (a == INT64_MIN && b == -1 ? uint64_t(a) : uint64_t(a / b));
        } else if (op >= 8 && op <= 11)
            value = shifted(lhs, op - 8, rhs & (bits - 1), bits);
        else
            unsupported(r);
        set(i.rd, value);
        break;
    }
    case Op::OneSource: {
        uint64_t value = get(i.rn), result = 0;
        if (i.rm != 0)
            unsupported(r);
        unsigned op = (r >> 10) & 63;
        if (op == 0)
            for (unsigned j = 0; j < bits; ++j)
                result |= ((value >> j) & 1) << (bits - 1 - j);
        else if (op >= 1 && op <= 3) {
            unsigned group = 1u << op;
            if (group * 8 > bits)
                unsupported(r);
            for (unsigned j = 0; j < bits / 8; ++j)
                result |= ((value >> (j * 8)) & 255) << (((j / group) * group + group - 1 - j % group) * 8);
        } else if (op == 4 || op == 5) {
            uint64_t count_value = op == 5 && (value >> (bits - 1)) ? (~value & mask(bits)) : value;
            unsigned start = bits;
            while (result < start && ((count_value >> (bits - 1 - result)) & 1) == 0)
                ++result;
            if (op == 5 && result)
                --result;
        } else
            unsupported(r);
        set(i.rd, result);
        break;
    }
    case Op::Multiply: {
        unsigned op = (r >> 21) & 7;
        uint64_t product;
        if (op == 0)
            product = get(i.rn) * get(i.rm);
        else if (bits == 64 && op == 1)
            product = uint64_t(int64_t(int32_t(cpu.get(i.rn, 32))) * int64_t(int32_t(cpu.get(i.rm, 32))));
        else if (bits == 64 && op == 5)
            product = cpu.get(i.rn, 32) * cpu.get(i.rm, 32);
        else
            unsupported(r);
        set(i.rd, (r & (1u << 15)) ? get(i.ra) - product : get(i.ra) + product);
        break;
    }
    case Op::MultiplyHigh: {
        if (bits != 64 || i.ra != 31)
            unsupported(r);
        unsigned __int128 product;
        if ((r >> 23) & 1)
            product = static_cast<unsigned __int128>(get(i.rn)) * get(i.rm);
        else
            product =
                static_cast<__int128>(static_cast<int64_t>(get(i.rn))) * static_cast<int64_t>(get(i.rm));
        set(i.rd, uint64_t(product >> 64));
        break;
    }
    case Op::LoadLiteral: {
        const bool vector = r & (1u << 26);
        unsigned op = r >> 30;
        if (op == 3) {
            if (vector)
                unsupported(r);
            break; // PRFM is an advisory prefetch.
        }
        uint64_t address = cpu.pc + sign_extend((r >> 5) & 0x7ffff, 19) * 4;
        if (vector) {
            std::array<uint8_t, 16> value{};
            memory.copy_out(address, value.data(), 4u << op);
            cpu.v[i.rd] = value;
            break;
        }
        uint64_t value = memory.load(address, op == 1 ? 8 : 4);
        cpu.set(i.rd, op == 2 ? uint64_t(sign_extend(value, 32)) : value);
        break;
    }
    case Op::LoadStore: {
        const bool vector = r & (1u << 26);
        unsigned size = 1u << (r >> 30), op = (r >> 22) & 3;
        const bool unprivileged = (r & 0x3b200c00) == 0x38000800;
        // LDTR/STTR have no SIMD or prefetch encoding. Reject reserved size/op
        // combinations before the generic PRFM path can turn them into a NOP.
        if (unprivileged && (vector || (size == 8 && op >= 2) || (size == 4 && op == 3)))
            throw InstructionFault(cpu.pc, r, InstructionFault::Kind::Undefined);
        if (vector) {
            unsigned scale = (r >> 30) | ((op & 2) << 1);
            if (scale > 4)
                unsupported(r);
            size = 1u << scale;
            op &= 1;
        }
        if (size == 8 && op == 2)
            break; // PRFM
        if ((size == 8 && op > 1) || (size == 4 && op == 3))
            unsupported(r);
        uint64_t base = cpu.get(i.rn, 64, true), address;
        bool writeback = false;
        uint64_t updated = base;
        if ((r & 0x3b000000) == 0x39000000)
            address = base + ((r >> 10) & 4095) * size;
        else if ((r & 0x00200c00) == 0x00200800) {
            unsigned option = (r >> 13) & 7;
            if (option != 2 && option != 3 && option != 6 && option != 7)
                unsupported(r);
            uint64_t offset = cpu.get(i.rm, option & 1 ? 64 : 32);
            if (option & 4)
                offset = sign_extend(offset, option & 1 ? 64 : 32);
            address = base + ((r & (1u << 12)) ? offset * size : offset);
        } else {
            if (r & (1u << 21))
                unsupported(r);
            unsigned mode = (r >> 10) & 3;
            // Mode 2 is the integer unprivileged imm9 form: byte offset, no
            // writeback. At EL0 it uses the same Memory permission checks.
            updated = base + sign_extend((r >> 12) & 511, 9);
            address = mode == 1 ? base : updated;
            writeback = mode == 1 || mode == 3;
        }
        if (vector) {
            if (op == 0)
                memory.copy_in(address, cpu.v[i.rd].data(), size);
            else {
                std::array<uint8_t, 16> value{};
                memory.copy_out(address, value.data(), size);
                cpu.v[i.rd] = value;
            }
        } else if (op == 0)
            memory.store(address, cpu.get(i.rd), size);
        else {
            uint64_t value = memory.load(address, size);
            if (op >= 2)
                value = sign_extend(value, size * 8);
            cpu.set(i.rd, value, op == 3 || (op == 1 && size < 8) ? 32 : 64);
        }
        if (writeback)
            cpu.set(i.rn, updated, 64, true);
        break;
    }
    case Op::LoadPair: {
        bool vector = r & (1u << 26);
        unsigned op = r >> 30, mode = (r >> 23) & 3;
        bool load = (r >> 22) & 1;
        if (op == 3 || (!vector && op == 1 && !load))
            unsupported(r);
        unsigned size = vector ? (4u << op) : (op == 2 ? 8 : 4);
        uint64_t base = cpu.get(i.rn, 64, true), updated = base + sign_extend((r >> 15) & 127, 7) * size;
        uint64_t address = mode == 1 ? base : updated;
        unsigned rt2 = (r >> 10) & 31;
        if (!memory.accessible(address, size * 2, load ? read_perm : write_perm))
            throw Failure("guest pair access fault at " + hex(address));
        if (vector) {
            if (load) {
                std::array<uint8_t, 16> a{}, b{};
                memory.copy_out(address, a.data(), size);
                memory.copy_out(address + size, b.data(), size);
                cpu.v[i.rd] = a;
                cpu.v[rt2] = b;
            } else {
                memory.copy_in(address, cpu.v[i.rd].data(), size);
                memory.copy_in(address + size, cpu.v[rt2].data(), size);
            }
        } else if (load) {
            uint64_t a = memory.load(address, size), b = memory.load(address + size, size);
            if (op == 1) {
                a = sign_extend(a, 32);
                b = sign_extend(b, 32);
            }
            cpu.set(i.rd, a, op == 0 ? 32 : 64);
            cpu.set(rt2, b, op == 0 ? 32 : 64);
        } else {
            memory.store(address, cpu.get(i.rd), size);
            memory.store(address + size, cpu.get(rt2), size);
        }
        if (mode == 1 || mode == 3)
            cpu.set(i.rn, updated, 64, true);
        break;
    }
    case Op::System: {
        unsigned code = (r >> 5) & 0xffff;
        bool reading = (r >> 21) & 1;
        if (code == 0xde82) {
            if (reading)
                cpu.set(i.rd, cpu.tls);
            else
                cpu.tls = cpu.get(i.rd);
        } else if (code == 0xda10) {
            if (reading)
                cpu.set(i.rd, uint64_t(cpu.nzcv) << 28);
            else
                cpu.nzcv = (cpu.get(i.rd) >> 28) & 15;
        } else if (code == 0xda20) {
            if (reading)
                cpu.set(i.rd, read_floating_control(cpu));
            else {
                const uint64_t value = cpu.get(i.rd);
                // IOE/DZE/OFE/UFE/IXE/IDE request real exception trapping. The
                // runtime does not implement traps, so never silently mask them.
                if (value & uint64_t{0x00009f00})
                    throw Failure("unsupported guest FPCR exception trap bits " + hex(value),
                                  ORT_ARM64_UNSUPPORTED);
                write_floating_control(cpu, value);
            }
        } else if (code == 0xda21) {
            if (reading)
                cpu.set(i.rd, read_floating_status(cpu));
            else
                write_floating_status(cpu, cpu.get(i.rd));
        } else if (reading && code == 0xd801)
            cpu.set(i.rd, cache_profile::ctr_el0);
        else if (reading && code == 0xd807)
            cpu.set(i.rd, cache_profile::dczid_el0);
        else if (reading && code == 0xdf00)
            cpu.set(i.rd, 1000000000);
        else if (reading && (code == 0xdf02 || code == 0xdf01)) {
            auto now = std::chrono::steady_clock::now().time_since_epoch();
            cpu.set(i.rd, std::chrono::duration_cast<std::chrono::nanoseconds>(now).count());
        } else
            unsupported(r);
        break;
    }
    case Op::Cache: {
        if (!cache_profile::maintenance(r))
            unsupported(r);
        const uint64_t address = cpu.get(i.rd);
        if ((r & 0xffffffe0) == cache_profile::dc_zva) {
            const std::array<uint8_t, cache_profile::line_bytes> zeros{};
            memory.copy_in<cache_profile::line_bytes>(
                address & ~(cache_profile::line_bytes - 1), zeros.data());
        } else
            memory.synchronize_code(address);
        break;
    }
    case Op::Barrier:
        std::atomic_thread_fence(std::memory_order_seq_cst);
        break;
    case Op::Exclusive:
        if (!execute_exclusive(cpu, memory, r))
            unsupported(r);
        break;
    case Op::VectorMemory:
        if (!execute_vector_memory(cpu, memory, r))
            unsupported(r);
        break;
    case Op::Simd:
        if (!execute_simd(cpu, r) && !execute_floating_point(cpu, r))
            unsupported(r);
        break;
    default:
        unsupported(r);
    }
    cpu.pc = next;
    return syscall;
}
} // namespace operit::arm64
