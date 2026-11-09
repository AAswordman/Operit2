#include "cpu/aarch64/decoder.h"
#include "cpu/aarch64/cache.h"
#include "cpu/aarch64/instruction_fault.h"
#include <optional>

namespace operit::arm64 {
namespace {
std::optional<InstructionFault::Kind> el0_fault(uint32_t raw) {
    using Kind = InstructionFault::Kind;
    if (!(raw & 0xffff0000))
        return Kind::Undefined; // UDF #imm16.
    switch (raw & 0xffe0001f) {
    case 0xd4200000:
        return Kind::Breakpoint;
    case 0xd4400000:
        return Kind::Halt;
    case 0xd4000002: // HVC.
    case 0xd4000003: // SMC.
    case 0xd4a00001: // DCPS1/2/3.
    case 0xd4a00002:
    case 0xd4a00003:
        return Kind::Privileged;
    default:
        break;
    }
    switch (raw) {
    case 0xd69f03e0: // ERET.
    case 0xd69f0bff: // ERETAA.
    case 0xd69f0fff: // ERETAB.
    case 0xd6bf03e0: // DRPS.
        return Kind::Privileged;
    default:
        break;
    }
    switch (raw & 0xfffff0ff) {
    case 0xd50040bf: // MSR SPSel, #imm.
    case 0xd500409f: // MSR PAN, #imm.
    case 0xd500407f: // MSR UAO, #imm.
    case 0xd501401f: // MSR ALLINT, #imm.
    case 0xd50340df: // MSR DAIFSet, #imm.
    case 0xd50340ff: // MSR DAIFClr, #imm.
        return Kind::Privileged;
    default:
        break;
    }
    // SYS/SYSL and MRS/MSR register encodings carry their minimum EL in op1.
    // EL0 system operations (op1 == 3) must retain unsupported diagnostics when
    // their actual operation has not yet been implemented.
    const bool system_register = (raw & 0xffd00000) == 0xd5100000;
    const bool system_operation = (raw & 0xffd80000) == 0xd5080000;
    if (system_register || system_operation) {
        const auto op1 = (raw >> 16) & 7;
        if (op1 != 3)
            return Kind::Privileged;
        const auto reg = (raw >> 5) & 0xffff;
        if (system_register &&
            (reg == 0xda11 || (!(raw & (1u << 21)) && (reg == 0xde83 || reg == 0xd801 || reg == 0xd807 ||
                                                       reg == 0xdf00 || reg == 0xdf01 || reg == 0xdf02))))
            return Kind::Privileged;
    }
    return {};
}
bool bit_masks(Instruction &i, bool logical) {
    const unsigned n = (i.raw >> 22) & 1, s = (i.raw >> 10) & 63, r = (i.raw >> 16) & 63;
    const unsigned selector = (n << 6) | ((~s) & 63);
    if (selector < 2 || (i.width == 32 && n))
        return false;
    const unsigned length = 31 - __builtin_clz(selector), element = 1u << length;
    const unsigned levels = element - 1;
    if (logical && (s & levels) == levels)
        return false;
    const uint64_t write = rotate(mask((s & levels) + 1), r & levels, element);
    const uint64_t top = mask(((s - r) & levels) + 1);
    i.immediate = i.auxiliary = 0;
    for (unsigned offset = 0; offset < i.width; offset += element) {
        i.immediate |= write << offset;
        i.auxiliary |= top << offset;
    }
    return true;
}
void normalize(Instruction &i) {
    const uint32_t r = i.raw;
    const bool wide = i.width == 64;
    using D = Encoding;
    switch (i.op) {
    case Op::Exclusive:
        if ((r & 0x3ffffc00) == 0x08dffc00) {
            i.width = 8u << (r >> 30);
            i.encoding = D::LoadAcquire;
        }
        break;
    case Op::Hint: {
        const unsigned hint = (r >> 5) & 127;
        if (hint == 0 || hint == 1 || hint == 7 || hint == 25 || hint == 27 || hint == 29 || hint == 31 ||
            hint == 32 || hint == 34 || hint == 36 || hint == 38)
            i.encoding = D::Noop;
        break;
    }
    case Op::Svc:
        if (!((r >> 5) & 0xffff))
            i.encoding = D::Syscall;
        break;
    case Op::Branch:
        i.encoding = r >> 31 ? D::BranchLink : D::Branch;
        break;
    case Op::BranchRegister:
        i.encoding = (r & 0xfffffc1f) == 0xd63f0000 ? D::BranchLinkRegister : D::BranchRegister;
        break;
    case Op::ConditionalBranch:
        i.flags = r & 15;
        i.encoding = D::ConditionalBranch;
        break;
    case Op::CompareBranch:
        i.flags = (r >> 24) & 1;
        i.encoding = wide ? D::CompareBranch64 : D::CompareBranch32;
        break;
    case Op::TestBranch:
        i.shift = ((r >> 19) & 31) | ((r >> 26) & 32);
        i.flags = (r >> 24) & 1;
        i.encoding = D::TestBranch;
        break;
    case Op::Address:
        i.flags = r >> 31;
        i.encoding = D::Address;
        break;
    case Op::MoveWide: {
        const unsigned shift = ((r >> 21) & 3) * 16, op = (r >> 29) & 3;
        if (shift >= i.width || op == 1)
            break;
        i.immediate = uint64_t((r >> 5) & 0xffff) << shift;
        i.auxiliary = ~(uint64_t{0xffff} << shift);
        if (op == 0)
            i.immediate = ~i.immediate;
        i.encoding = op == 3 ? (wide ? D::Keep64 : D::Keep32) : (wide ? D::Move64 : D::Move32);
        break;
    }
    case Op::AddImmediate:
        i.flags = (r >> 29) & 3;
        i.encoding = wide ? D::AddImmediate64 : D::AddImmediate32;
        break;
    case Op::AddRegister: {
        i.flags = (r >> 29) & 3;
        if (r & (1u << 21)) {
            const unsigned option = (r >> 13) & 7;
            i.shift = (r >> 10) & 7;
            if (i.shift > 4 || ((r >> 22) & 3))
                break;
            i.flags |= decoded::extended;
            if (option & 4)
                i.flags |= decoded::signed_extension;
            i.immediate = 8u << (option & 3);
            i.auxiliary = mask(i.immediate);
        } else {
            const unsigned type = (r >> 22) & 3;
            i.shift = (r >> 10) & 63;
            if (type == 3 || i.shift >= i.width)
                break;
            i.flags |= type << decoded::shift_type;
        }
        if ((r & 0xffe0fc1f) == 0xeb00001f)
            i.encoding = D::CompareRegister64;
        else
            i.encoding = wide ? D::AddRegister64 : D::AddRegister32;
        break;
    }
    case Op::AddCarry:
        i.flags = (r >> 29) & 3;
        i.encoding = wide ? D::AddCarry64 : D::AddCarry32;
        break;
    case Op::LogicalImmediate:
        if (bit_masks(i, true)) {
            i.flags = (r >> 29) & 3;
            i.encoding = wide ? D::LogicalImmediate64 : D::LogicalImmediate32;
        }
        break;
    case Op::LogicalRegister:
        i.shift = (r >> 10) & 63;
        if (i.shift >= i.width)
            break;
        i.flags = ((r >> 29) & 3) | (((r >> 21) & 1) << 4) | (((r >> 22) & 3) << 2);
        i.encoding = wide ? D::LogicalRegister64 : D::LogicalRegister32;
        break;
    case Op::Bitfield:
        if (((r >> 29) & 3) == 3 || ((r >> 22) & 1) != wide || ((r >> 10) & 63) >= i.width ||
            ((r >> 16) & 63) >= i.width || !bit_masks(i, false))
            break;
        i.shift = (r >> 16) & 63;
        i.flags = (r >> 29) & 3;
        i.encoding = wide ? D::Bitfield64 : D::Bitfield32;
        break;
    case Op::Extract:
        i.shift = (r >> 10) & 63;
        if (i.shift < i.width && ((r >> 22) & 1) == wide)
            i.encoding = wide ? D::Extract64 : D::Extract32;
        break;
    case Op::ConditionalSelect:
        i.flags = ((r >> 30) & 1) | (((r >> 10) & 1) << 1);
        i.shift = (r >> 12) & 15;
        i.encoding = wide ? D::Select64 : D::Select32;
        break;
    case Op::ConditionalCompare:
        if (!(r & (1u << 29)))
            break;
        i.flags = ((r >> 30) & 1) | (((r >> 11) & 1) << 1);
        i.shift = (r >> 12) & 15;
        i.encoding = wide ? D::Compare64 : D::Compare32;
        break;
    case Op::TwoSource:
        i.flags = (r >> 10) & 63;
        if (i.flags == 2 || i.flags == 3 || (i.flags >= 8 && i.flags <= 11))
            i.encoding = wide ? D::TwoSource64 : D::TwoSource32;
        break;
    case Op::OneSource:
        i.flags = (r >> 10) & 63;
        if (!i.rm && (i.flags <= 5) && (i.flags != 3 || wide))
            i.encoding = wide ? D::OneSource64 : D::OneSource32;
        break;
    case Op::Multiply:
        i.flags = ((r >> 21) & 7) | (((r >> 15) & 1) << 3);
        if (!(i.flags & 7) || (wide && ((i.flags & 7) == 1 || (i.flags & 7) == 5)))
            i.encoding = wide ? D::Multiply64 : D::Multiply32;
        break;
    case Op::MultiplyHigh:
        if (wide && i.ra == 31) {
            i.flags = (r >> 23) & 1;
            i.encoding = D::MultiplyHigh;
        }
        break;
    case Op::LoadStore: {
        const bool vector = r & (1u << 26);
        unsigned size = r >> 30, op = (r >> 22) & 3;
        const bool unprivileged = (r & 0x3b200c00) == 0x38000800;
        if (unprivileged && (vector || (size == 3 && op >= 2) || (size == 2 && op == 3))) {
            i.op = Op::Fault;
            i.flags = static_cast<uint8_t>(InstructionFault::Kind::Undefined);
            i.terminal = true;
            break;
        }
        if (vector) {
            size |= (op & 2) << 1;
            if (size > 4)
                break;
            op &= 1;
        }
        if (!vector && size == 3 && op == 2) {
            i.encoding = D::Noop;
            break;
        }
        if (!vector && ((size == 3 && op > 1) || (size == 2 && op == 3)))
            break;
        i.shift = size;
        i.flags = (op >= 2 ? decoded::signed_load : 0) |
                  (op == 3 || (op == 1 && size < 3) ? decoded::word_result : 0);
        if ((r & 0x3b000000) == 0x39000000)
            i.immediate = ((r >> 10) & 4095) << size;
        else if ((r & 0x00200c00) == 0x00200800) {
            const unsigned option = (r >> 13) & 7;
            if (option != 2 && option != 3 && option != 6 && option != 7)
                break;
            i.flags |= decoded::register_offset | (!(option & 1) ? decoded::offset_word : 0) |
                       (option & 4 ? decoded::offset_signed : 0) |
                       (r & (1u << 12) ? decoded::offset_scaled : 0);
        } else {
            const unsigned mode = (r >> 10) & 3;
            if (r & (1u << 21))
                break;
            // LDTR/STTR (mode 2) share unscaled addressing without writeback.
            i.immediate = sign_extend((r >> 12) & 511, 9);
            i.flags |= mode == 1 ? decoded::post_index : (mode == 3 ? decoded::pre_index : 0);
        }
        constexpr D loads[] = {D::Load8, D::Load16, D::Load32, D::Load64};
        constexpr D stores[] = {D::Store8, D::Store16, D::Store32, D::Store64};
        if (vector) {
            constexpr D vector_loads[] = {D::General, D::General, D::VectorLoad32, D::VectorLoad64,
                                          D::VectorLoad128};
            constexpr D vector_stores[] = {D::General, D::General, D::VectorStore32, D::VectorStore64,
                                           D::VectorStore128};
            i.encoding = op ? vector_loads[size] : vector_stores[size];
        } else
            i.encoding = op ? loads[size] : stores[size];
        break;
    }
    case Op::LoadPair: {
        const unsigned op = r >> 30, mode = (r >> 23) & 3;
        const bool load = (r >> 22) & 1;
        const bool vector = r & (1u << 26);
        if (op == 3 || (!vector && op == 1 && !load))
            break;
        const unsigned size = vector ? 4u << op : (op == 2 ? 8 : 4);
        i.immediate = sign_extend((r >> 15) & 127, 7) * size;
        i.flags = mode == 1 ? decoded::post_index : (mode == 3 ? decoded::pre_index : 0);
        if (vector) {
            constexpr D loads[] = {D::VectorLoadPair32, D::VectorLoadPair64, D::VectorLoadPair128};
            constexpr D stores[] = {D::VectorStorePair32, D::VectorStorePair64, D::VectorStorePair128};
            i.encoding = load ? loads[op] : stores[op];
        } else {
            if (op == 1)
                i.flags |= decoded::signed_load;
            if (op == 0)
                i.flags |= decoded::word_result;
            i.encoding = load ? (size == 8 ? D::LoadPair64 : D::LoadPair32)
                              : (size == 8 ? D::StorePair64 : D::StorePair32);
        }
        break;
    }
    case Op::LoadLiteral: {
        if (r & (1u << 26))
            break;
        const unsigned op = r >> 30;
        if (op == 3) {
            i.encoding = D::Noop;
            break;
        }
        i.immediate = sign_extend((r >> 5) & 0x7ffff, 19) * 4;
        i.flags = op == 2 ? decoded::signed_load : 0;
        i.encoding = op == 1 ? D::LoadLiteral64 : D::LoadLiteral32;
        break;
    }
    case Op::Simd: {
        const uint32_t move = r & 0xfffffc00;
#if defined(__aarch64__)
        const auto unary=r&0xffbffc00;
        if(unary==0x1e204000 || unary==0x1e20c000 || unary==0x1e214000) {
            i.width=(r&(1u<<22)) ? 64:32;
            i.flags=unary==0x1e204000 ? 0:unary==0x1e20c000 ? 1:2;
            i.encoding=i.width==64 ? D::ScalarUnary64:D::ScalarUnary32;
        } else if ((r & 0xffe08000) == 0x1f400000)
            i.encoding = D::ScalarFma64;
        else if (move == 0x1e61c000)
            i.encoding = D::ScalarSqrt64;
        else if (move == 0x9e630000)
            i.encoding = D::ScalarUnsignedToDouble64;
        else if (move == 0x9e620000)
            i.encoding = D::ScalarSignedToDouble64;
        else if (move == 0x1e630000 || move == 0x1e620000) {
            i.width = 32;
            i.encoding = move == 0x1e630000 ? D::ScalarUnsignedToDouble32 : D::ScalarSignedToDouble32;
        } else if (move == 0x1e780000 || move == 0x9e780000) {
            i.width = (r >> 31) ? 64 : 32;
            i.encoding = i.width == 64 ? D::ScalarDoubleToSigned64 : D::ScalarDoubleToSigned32;
        } else if ((r & 0xffe00c00) == 0x1e600800) {
            switch ((r >> 12) & 15) {
            case 0: i.encoding = D::ScalarMultiply64; break;
            case 1: i.encoding = D::ScalarDivide64; break;
            case 2: i.encoding = D::ScalarAdd64; break;
            case 3: i.encoding = D::ScalarSubtract64; break;
            default: break;
            }
        } else if ((r & 0xffe0fc07) == 0x1e602000 && (!(r&8) || !((r>>16)&31)))
            i.encoding = D::ScalarCompare64;
        else if ((r & 0xffe00c00) == 0x1e600c00) {
            i.flags = (r >> 12) & 15;
            i.encoding = D::ScalarSelect64;
        } else if ((r & 0xffe0fc00) == 0x4e60cc00)
            i.encoding = D::VectorFma2D;
        else
#endif
        if (move == 0x9e660000 || move == 0x1e260000)
            i.encoding = move == 0x9e660000 ? D::VectorToRegister64 : D::VectorToRegister32;
        else if (move == 0x9e670000 || move == 0x1e270000)
            i.encoding = move == 0x9e670000 ? D::RegisterToVector64 : D::RegisterToVector32;
        else if (move == 0x9eae0000)
            i.encoding = D::VectorHighToRegister;
        else if (move == 0x9eaf0000)
            i.encoding = D::RegisterToVectorHigh;
        else if ((r & 0xff201fe0) == 0x1e201000) {
            const unsigned type = (r >> 22) & 3;
            if (type > 1)
                break;
            const unsigned width = type ? 64 : 32, exponent_bits = type ? 11 : 8;
            const unsigned fraction_bits = width - exponent_bits - 1;
            const uint64_t immediate = (r >> 13) & 255, repeated = (immediate >> 6) & 1;
            const uint64_t exponent = ((repeated ^ 1) << (exponent_bits - 1)) |
                                      ((repeated ? mask(exponent_bits - 3) : 0) << 2) |
                                      ((immediate >> 4) & 3);
            i.immediate = ((immediate >> 7) << (width - 1)) | (exponent << fraction_bits) |
                          ((immediate & 15) << (fraction_bits - 4));
            i.encoding = D::VectorImmediate;
        } else if ((r & 0x9f20fc00) == 0x0e201c00) {
            i.flags = ((r >> 22) & 3) | (((r >> 29) & 1) << 2);
            i.shift = r & (1u << 30) ? 16 : 8;
            i.encoding = D::VectorLogical;
        }
        break;
    }
    case Op::System: {
        const unsigned code = (r >> 5) & 0xffff;
        const bool reading = (r >> 21) & 1;
        if (code == 0xde82)
            i.encoding = reading ? D::ReadTls : D::WriteTls;
        else if (code == 0xda10)
            i.encoding = reading ? D::ReadFlags : D::WriteFlags;
        break;
    }
    default:
        break;
    }
}
} // namespace
Instruction decode(uint32_t raw) {
    Instruction i;
    i.raw = raw;
    i.rd = raw & 31;
    i.rn = (raw >> 5) & 31;
    i.rm = (raw >> 16) & 31;
    i.ra = (raw >> 10) & 31;
    i.width = (raw >> 31) ? 64 : 32;
    if (auto fault = el0_fault(raw)) {
        i.op = Op::Fault;
        i.flags = static_cast<uint8_t>(*fault);
        i.terminal = true;
    } else if ((raw & 0xfffff01f) == 0xd503201f)
        i.op = Op::Hint;
    else if ((raw & 0xffe0001f) == 0xd4000001) {
        i.op = Op::Svc;
        i.terminal = true;
    } else if ((raw & 0x7c000000) == 0x14000000) {
        i.op = Op::Branch;
        i.immediate = sign_extend(raw & 0x3ffffff, 26) * 4;
        i.terminal = true;
    } else if ((raw & 0xfffffc1f) == 0xd61f0000 || (raw & 0xfffffc1f) == 0xd63f0000 ||
               (raw & 0xfffffc1f) == 0xd65f0000) {
        i.op = Op::BranchRegister;
        i.terminal = true;
    } else if ((raw & 0xff000010) == 0x54000000) {
        i.op = Op::ConditionalBranch;
        i.immediate = sign_extend((raw >> 5) & 0x7ffff, 19) * 4;
        i.terminal = true;
    } else if ((raw & 0x7e000000) == 0x34000000) {
        i.op = Op::CompareBranch;
        i.immediate = sign_extend((raw >> 5) & 0x7ffff, 19) * 4;
        i.terminal = true;
    } else if ((raw & 0x7e000000) == 0x36000000) {
        i.op = Op::TestBranch;
        i.immediate = sign_extend((raw >> 5) & 0x3fff, 14) * 4;
        i.terminal = true;
    } else if ((raw & 0x1f000000) == 0x10000000) {
        i.op = Op::Address;
        i.immediate = sign_extend(((uint64_t(raw >> 5) & 0x7ffff) << 2) | ((raw >> 29) & 3), 21);
    } else if ((raw & 0x1f800000) == 0x12800000)
        i.op = Op::MoveWide;
    else if ((raw & 0x1f800000) == 0x11000000) {
        i.op = Op::AddImmediate;
        i.immediate = ((raw >> 10) & 0xfff) << (((raw >> 22) & 1) * 12);
    } else if ((raw & 0x1f000000) == 0x0b000000)
        i.op = Op::AddRegister;
    else if ((raw & 0x1fe0fc00) == 0x1a000000)
        i.op = Op::AddCarry;
    else if ((raw & 0x1f000000) == 0x0a000000)
        i.op = Op::LogicalRegister;
    else if ((raw & 0x1f800000) == 0x12000000)
        i.op = Op::LogicalImmediate;
    else if ((raw & 0x1f800000) == 0x13000000)
        i.op = Op::Bitfield;
    else if ((raw & 0x1fa00000) == 0x13800000)
        i.op = Op::Extract;
    else if ((raw & 0x1fe00800) == 0x1a800000)
        i.op = Op::ConditionalSelect;
    else if ((raw & 0x1fe00410) == 0x1a400000)
        i.op = Op::ConditionalCompare;
    else if ((raw & 0x7fe00000) == 0x1ac00000)
        i.op = Op::TwoSource;
    else if ((raw & 0x7fe00000) == 0x5ac00000)
        i.op = Op::OneSource;
    else if ((raw & 0x7fe08000) == 0x1b400000 || (raw & 0x7fe08000) == 0x1bc00000)
        i.op = Op::MultiplyHigh;
    else if ((raw & 0x1f000000) == 0x1b000000)
        i.op = Op::Multiply;
    else if ((raw & 0x3b000000) == 0x18000000)
        i.op = Op::LoadLiteral;
    else if ((raw & 0x3a000000) == 0x28000000)
        i.op = Op::LoadPair;
    else if ((raw & 0x3b000000) == 0x39000000 || (raw & 0x3b000000) == 0x38000000)
        i.op = Op::LoadStore;
    else if ((raw & 0xfff00000) == 0xd5300000 || (raw & 0xfff00000) == 0xd5100000)
        i.op = Op::System;
    else if (cache_profile::maintenance(raw))
        i.op = Op::Cache;
    else if ((raw & 0x3f000000) == 0x08000000 || (raw & 0xfffff0ff) == 0xd503305f)
        i.op = Op::Exclusive;
    else if ((raw & 0x0e000000) == 0x0c000000)
        i.op = Op::VectorMemory;
    else if ((raw & 0x0e000000) == 0x0e000000)
        i.op = Op::Simd;
    else if ((raw & 0xfffff0ff) == 0xd503309f || (raw & 0xfffff0ff) == 0xd50330bf ||
             (raw & 0xfffff0ff) == 0xd50330df)
        i.op = Op::Barrier;
    if (i.op == Op::Unknown)
        i.terminal = true;
    normalize(i);
    return i;
}

} // namespace operit::arm64
