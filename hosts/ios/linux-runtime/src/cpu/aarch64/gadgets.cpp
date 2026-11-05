#include "cpu/aarch64/gadgets.h"
#include "cpu/aarch64/gadget_offsets.h"
#include <type_traits>

namespace operit::arm64 {
static_assert(std::is_standard_layout_v<NativeGadget>);
static_assert(offsetof(NativeGadget, entry) == G_ENTRY);
static_assert(offsetof(NativeGadget, immediate) == G_IMMEDIATE);
static_assert(offsetof(NativeGadget, auxiliary) == G_AUXILIARY);
static_assert(offsetof(NativeGadget, rd) == G_RD);
static_assert(offsetof(NativeGadget, rn) == G_RN);
static_assert(offsetof(NativeGadget, rm) == G_RM);
static_assert(offsetof(NativeGadget, ra) == G_RA);
static_assert(offsetof(NativeGadget, shift) == G_SHIFT);
static_assert(offsetof(NativeGadget, flags) == G_FLAGS);
static_assert(sizeof(NativeGadget) == G_SIZE);
static_assert(std::is_standard_layout_v<CpuState> && offsetof(CpuState, x) == 0);
static_assert(offsetof(CpuState, sp) == G_CPU_SP);
static_assert(offsetof(CpuState, nzcv) == G_CPU_NZCV);
static_assert(offsetof(CpuState, gadget_zero) == G_CPU_ZERO);
static_assert(G_ZERO_SLOT <= UINT8_MAX);
static_assert(offsetof(GadgetMemory, translations) == 0 && offsetof(GadgetMemory, epoch) == 8 &&
              offsetof(GadgetMemory, file_generation) == 16);
#if defined(__aarch64__)
enum GadgetEntry {
#define GADGET_BASIC(name) name,
#define GADGET_BINARY(name, operation, lhs, rhs, immediate, sets_flags, invert) GADGET_BASIC(name)
#define GADGET_CARRY(name, operation, lhs, rhs, sets_flags) GADGET_BASIC(name)
#define GADGET_PRODUCT(name, operation, lhs, rhs, accumulator, accumulated) GADGET_BASIC(name)
#define GADGET_MEMORY(name, size, writing, pair) GADGET_BASIC(name)
#include "cpu/aarch64/gadget_entries.inc"
#undef GADGET_MEMORY
#undef GADGET_BASIC
#undef GADGET_BINARY
#undef GADGET_CARRY
#undef GADGET_PRODUCT
};
// Indices select semantics once at decode time. No guest flag/mode decoding
// remains in these arithmetic, logical, carry or multiply handlers.
constexpr GadgetEntry arithmetic_entries[2][2][4] = {
    {{add_reg32, adds_reg32, sub_reg32, subs_reg32},
     {add_imm32, adds_imm32, sub_imm32, subs_imm32}},
    {{add_reg64, adds_reg64, sub_reg64, subs_reg64},
     {add_imm64, adds_imm64, sub_imm64, subs_imm64}},
};
constexpr GadgetEntry logical_entries[2][3][4] = {
    {{and_reg32, orr_reg32, eor_reg32, ands_reg32},
     {and_inv32, orr_inv32, eor_inv32, ands_inv32},
     {and_imm32, orr_imm32, eor_imm32, ands_imm32}},
    {{and_reg64, orr_reg64, eor_reg64, ands_reg64},
     {and_inv64, orr_inv64, eor_inv64, ands_inv64},
     {and_imm64, orr_imm64, eor_imm64, ands_imm64}},
};
constexpr GadgetEntry carry_entries[2][4] = {
    {adc32, adcs32, sbc32, sbcs32}, {adc64, adcs64, sbc64, sbcs64},
};
constexpr GadgetEntry product_entries[2][2][2] = {
    {{mul32, mneg32}, {madd32, msub32}},
    {{mul64, mneg64}, {madd64, msub64}},
};
extern "C" const void *const operit_arm64_gadget_entries[];
NativeGadget decode_gadget(const Instruction &i, uint64_t pc) {
    NativeGadget result{nullptr, i.immediate, i.auxiliary, i.rd, i.rn, i.rm, i.ra, i.shift, i.flags};
    constexpr uint8_t stack = G_CPU_SP / sizeof(uint64_t);
    const auto sp = [](uint8_t index) { return index == 31 ? stack : index; };
#define ENTRY(name) result.entry = operit_arm64_gadget_entries[name]
    switch (i.dispatch) {
    case Dispatch::Noop:
        ENTRY(noop);
        break;
    // Only fallthrough is consumed here. A taken branch returns before retirement
    // and uses the existing linked-block dispatcher and accounting.
    case Dispatch::ConditionalBranch:
        ENTRY(conditional);
        break;
    case Dispatch::CompareBranch32:
        ENTRY(compare_branch32);
        break;
    case Dispatch::CompareBranch64:
        ENTRY(compare_branch64);
        break;
    case Dispatch::TestBranch:
        ENTRY(test_branch);
        break;
    case Dispatch::Address:
        result.immediate = i.flags ? (pc & ~uint64_t{4095}) + (i.immediate << 12) : pc + i.immediate;
        ENTRY(move64);
        break;
    case Dispatch::Move32:
        ENTRY(move32);
        break;
    case Dispatch::Move64:
        ENTRY(move64);
        break;
    case Dispatch::Keep32:
        ENTRY(keep32);
        break;
    case Dispatch::Keep64:
        ENTRY(keep64);
        break;
    case Dispatch::AddImmediate32:
        ENTRY(arithmetic_entries[0][1][i.flags & 3]);
        break;
    case Dispatch::AddImmediate64:
        ENTRY(arithmetic_entries[1][1][i.flags & 3]);
        break;
    case Dispatch::AddRegister32:
    case Dispatch::AddRegister64:
        // Extended/shifted forms retain the existing implementation for now.
        if ((i.flags & decoded::extended) || i.shift)
            return {};
        ENTRY(arithmetic_entries[i.width == 64][0][i.flags & 3]);
        break;
    case Dispatch::CompareRegister64:
        ENTRY(subs_reg64);
        break;
    case Dispatch::AddCarry32:
        ENTRY(carry_entries[0][i.flags & 3]);
        break;
    case Dispatch::AddCarry64:
        ENTRY(carry_entries[1][i.flags & 3]);
        break;
    case Dispatch::LogicalImmediate32:
        ENTRY(logical_entries[0][2][i.flags & 3]);
        break;
    case Dispatch::LogicalImmediate64:
        ENTRY(logical_entries[1][2][i.flags & 3]);
        break;
    case Dispatch::LogicalRegister32:
    case Dispatch::LogicalRegister64:
        if (i.shift)
            return {};
        if (i.rn == 31 && i.flags == 1) {
            if (i.dispatch == Dispatch::LogicalRegister32) {
                ENTRY(copy32);
            } else {
                ENTRY(copy64);
            }
            break;
        }
        ENTRY(logical_entries[i.width == 64][bool(i.flags & 16)][i.flags & 3]);
        break;
    case Dispatch::Multiply32:
    case Dispatch::Multiply64:
        if (i.flags & 7)
            return {};
        ENTRY(product_entries[i.width == 64][i.ra != 31][bool(i.flags & 8)]);
        break;
    case Dispatch::Select32:
        ENTRY(select32);
        break;
    case Dispatch::Select64:
        ENTRY(select64);
        break;
    case Dispatch::Load8:
        ENTRY(load1);
        break;
    case Dispatch::Load16:
        ENTRY(load2);
        break;
    case Dispatch::Load32:
        ENTRY(load4);
        break;
    case Dispatch::Load64:
        ENTRY((i.flags & decoded::address_mode) == 0 ? load8_offset : load8);
        break;
    case Dispatch::Store8:
        ENTRY(store1);
        break;
    case Dispatch::Store16:
        ENTRY(store2);
        break;
    case Dispatch::Store32:
        ENTRY(store4);
        break;
    case Dispatch::Store64:
        ENTRY((i.flags & decoded::address_mode) == 0 ? store8_offset : store8);
        break;
    case Dispatch::LoadPair32:
        ENTRY(pair_load4);
        break;
    case Dispatch::LoadPair64:
        ENTRY((i.flags & decoded::address_mode) == 0 ? pair_load8_offset : pair_load8);
        break;
    case Dispatch::StorePair32:
        ENTRY(pair_store4);
        break;
    case Dispatch::StorePair64:
        ENTRY((i.flags & decoded::address_mode) == 0 ? pair_store8_offset : pair_store8);
        break;
    default:
        return {};
    }
#undef ENTRY
    if (i.dispatch == Dispatch::AddImmediate32 || i.dispatch == Dispatch::AddImmediate64) {
        result.rn = sp(i.rn);
        if (!(i.flags & decoded::set_flags))
            result.rd = sp(i.rd);
    }
    if ((i.dispatch == Dispatch::LogicalImmediate32 || i.dispatch == Dispatch::LogicalImmediate64) &&
        (i.flags & 3) != 3)
        result.rd = sp(i.rd);
    if (i.dispatch >= Dispatch::Load8 && i.dispatch <= Dispatch::StorePair64)
        result.rn = sp(i.rn);
    // Map architectural zero operands onto a real, read-only zero slot. SP
    // operands have already been resolved above. No per-read ZR check remains.
    for (auto *operand : {&result.rd, &result.rn, &result.rm, &result.ra})
        if (*operand == 31)
            *operand = G_ZERO_SLOT;
    return result;
}
#else
NativeGadget decode_gadget(const Instruction &, uint64_t) {
    return {};
}
extern "C" uint64_t operit_arm64_gadgets_run(CpuState *, const GadgetMemory *, const NativeGadget *,
                                             uint64_t) {
    return 0;
}
#endif
} // namespace operit::arm64
