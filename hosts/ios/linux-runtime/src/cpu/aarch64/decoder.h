#pragma once
#include "base/support.h"

namespace operit::arm64 {
enum class Op : uint8_t {
    Unknown,
    Hint,
    Svc,
    Branch,
    BranchRegister,
    ConditionalBranch,
    CompareBranch,
    TestBranch,
    Address,
    MoveWide,
    AddImmediate,
    AddRegister,
    AddCarry,
    LogicalRegister,
    LogicalImmediate,
    Bitfield,
    Extract,
    ConditionalSelect,
    ConditionalCompare,
    TwoSource,
    OneSource,
    Multiply,
    MultiplyHigh,
    LoadStore,
    LoadLiteral,
    LoadPair,
    System,
    Cache,
    Simd,
    VectorMemory,
    Exclusive,
    Barrier,
    Fault
};

// Normalized operand encodings consumed by the static execution compiler.
enum class Encoding : uint8_t {
    General,
    Noop,
    Syscall,
    Branch,
    BranchLink,
    BranchRegister,
    BranchLinkRegister,
    ConditionalBranch,
    CompareBranch32,
    CompareBranch64,
    TestBranch,
    Address,
    Move32,
    Move64,
    Keep32,
    Keep64,
    AddImmediate32,
    AddImmediate64,
    AddRegister32,
    AddRegister64,
    CompareRegister64,
    AddCarry32,
    AddCarry64,
    LogicalImmediate32,
    LogicalImmediate64,
    LogicalRegister32,
    LogicalRegister64,
    Bitfield32,
    Bitfield64,
    Extract32,
    Extract64,
    Select32,
    Select64,
    Compare32,
    Compare64,
    TwoSource32,
    TwoSource64,
    OneSource32,
    OneSource64,
    Multiply32,
    Multiply64,
    MultiplyHigh,
    Load8,
    Load16,
    Load32,
    Load64,
    Store8,
    Store16,
    Store32,
    Store64,
    LoadPair32,
    LoadPair64,
    StorePair32,
    StorePair64,
    LoadLiteral32,
    LoadLiteral64,
    ReadTls,
    WriteTls,
    ReadFlags,
    WriteFlags,
    VectorLoad32,
    VectorLoad64,
    VectorLoad128,
    VectorStore32,
    VectorStore64,
    VectorStore128,
    VectorLoadPair32,
    VectorLoadPair64,
    VectorLoadPair128,
    VectorStorePair32,
    VectorStorePair64,
    VectorStorePair128,
    VectorToRegister32,
    VectorToRegister64,
    RegisterToVector32,
    RegisterToVector64,
    VectorHighToRegister,
    RegisterToVectorHigh,
    VectorImmediate,
    VectorLogical,
    ScalarFma64,
    ScalarSqrt64,
    ScalarUnsignedToDouble64,
    ScalarAdd64,
    ScalarCompare64,
    ScalarSelect64,
    VectorFma2D,
    ScalarMultiply64,
    ScalarDivide64,
    ScalarSubtract64,
    ScalarUnary32, ScalarUnary64,
    Count
};

namespace decoded {
constexpr unsigned set_flags = 1, subtract = 2;
constexpr unsigned shift_type = 2, extended = 16, signed_extension = 32;
constexpr unsigned address_mode = 3, register_offset = 1, post_index = 2, pre_index = 3;
constexpr unsigned signed_load = 4, word_result = 8, offset_word = 16, offset_signed = 32, offset_scaled = 64;
} // namespace decoded

struct Instruction {
    uint64_t immediate = 0, auxiliary = 0;
    uint32_t raw = 0;
    Op op = Op::Unknown;
    Encoding encoding = Encoding::General;
    uint8_t rd = 0, rn = 0, rm = 0, ra = 0, width = 64, shift = 0, flags = 0;
    bool terminal = false;
};
static_assert(sizeof(Instruction) == 32);
Instruction decode(uint32_t raw);
} // namespace operit::arm64
