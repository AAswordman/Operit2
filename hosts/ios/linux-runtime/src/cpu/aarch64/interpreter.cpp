#include "cpu/aarch64/interpreter.h"
#include "acceleration/scalar_ops.h"
#include "acceleration/vector_ops.h"
#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/simd.h"
#include "cpu/aarch64/vector_floating.h"
#include "cpu/aarch64/vector_integer.h"
#include "cpu/aarch64/vector_lanes.h"
#include "cpu/aarch64/vector_narrow.h"
#include "cpu/aarch64/vector_shift.h"
#include <algorithm>
#include <cstring>

namespace operit::arm64 {
namespace {
constexpr size_t generation_capacity = 65536;
constexpr size_t maximum_block_instructions = 64;
static_assert(maximum_block_instructions <= UINT8_MAX);

#if defined(__aarch64__)
double read_fp64(const CpuState &cpu, unsigned reg) {
    double value;
    std::memcpy(&value, cpu.v[reg].data(), sizeof(value));
    return value;
}
void write_fp64(CpuState &cpu, unsigned reg, double value) {
    cpu.v[reg].fill(0);
    std::memcpy(cpu.v[reg].data(), &value, sizeof(value));
}
#endif

// Keeping retirement local avoids a memory update for every guest instruction.
// The cursor advances only after an instruction succeeds, including during unwinding.
template <class InstructionType> struct Retirement {
    uint64_t &total, &completed, &total_hits, &completed_hits;
    const InstructionType *&begin, *&cursor;
    ~Retirement() {
        total += completed + (begin ? uint64_t(cursor - begin) : 0);
        total_hits += completed_hits;
    }
};
} // namespace

bool Interpreter::revalidate(Block &block, Memory &memory) {
    return memory.validate_code(block.address, block.words.data(),
                                block.words.size() * sizeof(uint32_t), block.epoch);
}

Interpreter::Block Interpreter::decode_block(uint64_t address, Memory &memory,
                                             const void *const *dispatch) {
    Block block;
    block.address = address;
    block.epoch = memory.code_epoch;
    block.accelerated = acceleration_ && acceleration_->contains(address);
    // A page bounds speculative decoding: later unmapped pages cannot fault an earlier instruction.
    const size_t available =
        std::min<uint64_t>(maximum_block_instructions, (page_size - address % page_size) / 4);
    std::array<uint32_t, maximum_block_instructions> words;
    memory.copy_out(address, words.data(), available * sizeof(uint32_t), exec_perm);
    std::array<Instruction, maximum_block_instructions> instructions;
    size_t count = 0;
    do {
        auto &instruction = instructions[count];
        instruction = decode(words[count]);
        ++count;
        if (instruction.op == Op::ConditionalBranch || instruction.op == Op::CompareBranch ||
            instruction.op == Op::TestBranch) {
            // These operations do not use auxiliary; cache the taken prefix's retirement count.
            instruction.auxiliary = count;
        } else if (instruction.terminal)
            break;
    } while (count < available);
    block.instruction_count = static_cast<uint8_t>(count);
    block.words.assign(words.begin(), words.begin() + count);
    block.instructions.reserve(count);
    for (size_t index = 0; index < count; ++index) {
        const auto &instruction = instructions[index];
        const unsigned entry = instruction.dispatch == Dispatch::General && instruction.op == Op::Simd
                                   ? static_cast<unsigned>(Dispatch::Count)
                                   : static_cast<unsigned>(instruction.dispatch);
        block.instructions.emplace_back(instruction, dispatch[entry]);
    }
    ++decoded_blocks;
    return block;
}

Interpreter::Block *Interpreter::find_block(uint64_t address, Memory &memory,
                                            const void *const *dispatch) {
    const auto refresh = [&](Block &block) {
        if (block.epoch == memory.code_epoch || revalidate(block, memory))
            ++cache_hits;
        else
            block = decode_block(address, memory, dispatch);
        return &block;
    };
    auto &current = blocks_[active_generation_];
    auto found = current.find(address);
    if (found != current.end())
        return refresh(found->second);
    if (current.size() >= generation_capacity) {
        active_generation_ ^= 1;
        blocks_[active_generation_].clear();
        ++cache_generation_;
        lookups_.fill({});
    }
    auto &active = blocks_[active_generation_];
    auto old = blocks_[active_generation_ ^ 1].extract(address);
    if (!old.empty())
        return refresh(active.insert(std::move(old)).position->second);
    return &active.emplace(address, decode_block(address, memory, dispatch)).first->second;
}

Execution Interpreter::run(CpuState &cpu, Memory &memory, uint64_t budget, FixedComponents *acceleration) {
    Memory::ExecutionScope memory_execution(memory);
    if (acceleration && !acceleration->active())
        acceleration = nullptr;
    const uint64_t acceleration_generation = acceleration ? acceleration->generation() : 0;
    if (memory_ != &memory || acceleration_ != acceleration ||
        acceleration_generation_ != acceleration_generation) {
        for (auto &generation : blocks_)
            generation.clear();
        lookups_.fill({});
        ++cache_generation_;
        memory_ = &memory;
        acceleration_ = acceleration;
        acceleration_generation_ = acceleration_generation;
    }
    uint64_t count = 0, hits = 0, block_instructions = 0;
    const CachedInstruction *begin = nullptr, *cursor = nullptr, *end = nullptr;
    Retirement<CachedInstruction> retirement{executed_instructions, count, cache_hits, hits, begin, cursor};
    Block *previous = nullptr, *block = nullptr;
    uint64_t previous_generation = 0, epoch = 0, pc = cpu.pc;
    static const void *const dispatch[] = {
        &&General,
        &&Noop,
        &&Syscall,
        &&Branch,
        &&BranchLink,
        &&BranchRegister,
        &&BranchLinkRegister,
        &&ConditionalBranch,
        &&CompareBranch32,
        &&CompareBranch64,
        &&TestBranch,
        &&Address,
        &&Move32,
        &&Move64,
        &&Keep32,
        &&Keep64,
        &&AddImmediate32,
        &&AddImmediate64,
        &&AddRegister32,
        &&AddRegister64,
        &&CompareRegister64,
        &&AddCarry32,
        &&AddCarry64,
        &&LogicalImmediate32,
        &&LogicalImmediate64,
        &&LogicalRegister32,
        &&LogicalRegister64,
        &&Bitfield32,
        &&Bitfield64,
        &&Extract32,
        &&Extract64,
        &&Select32,
        &&Select64,
        &&Compare32,
        &&Compare64,
        &&TwoSource32,
        &&TwoSource64,
        &&OneSource32,
        &&OneSource64,
        &&Multiply32,
        &&Multiply64,
        &&MultiplyHigh,
        &&Load8,
        &&Load16,
        &&Load32,
        &&Load64,
        &&Store8,
        &&Store16,
        &&Store32,
        &&Store64,
        &&LoadPair32,
        &&LoadPair64,
        &&StorePair32,
        &&StorePair64,
        &&LoadLiteral32,
        &&LoadLiteral64,
        &&ReadTls,
        &&WriteTls,
        &&ReadFlags,
        &&WriteFlags,
        &&VectorLoad32,
        &&VectorLoad64,
        &&VectorLoad128,
        &&VectorStore32,
        &&VectorStore64,
        &&VectorStore128,
        &&VectorLoadPair32,
        &&VectorLoadPair64,
        &&VectorLoadPair128,
        &&VectorStorePair32,
        &&VectorStorePair64,
        &&VectorStorePair128,
        &&VectorToRegister32,
        &&VectorToRegister64,
        &&RegisterToVector32,
        &&RegisterToVector64,
        &&VectorHighToRegister,
        &&RegisterToVectorHigh,
        &&VectorImmediate,
        &&VectorLogical,
        &&ScalarFma64,
        &&ScalarSqrt64,
        &&ScalarUnsignedToDouble64,
        &&ScalarAdd64,
        &&ScalarCompare64,
        &&ScalarSelect64,
        &&VectorFma2D,
        &&ResolveSimd,
    };
    static_assert(std::size(dispatch) == static_cast<unsigned>(Dispatch::Count) + 1);

    while (count < budget) {
        if (pc & 3) {
            cpu.pc = pc;
            throw Failure("unaligned instruction address " + hex(pc));
        }
        block = nullptr;
        // The last executed block is still alive here: only find_block can rotate
        // generations. The common successor avoids hashing and relinking entirely.
        if (previous && previous->links_generation == cache_generation_) {
            block = previous->successors[0];
            if (block && block->address == pc) {
                ++hits;
                goto block_ready;
            }
            block = previous->successors[1];
            if (block && block->address != pc)
                block = nullptr;
        }
        if (!block) {
            auto &lookup = lookups_[((pc >> 2) ^ (pc >> 17)) & (lookups_.size() - 1)];
            if (lookup.pc == pc) {
                block = lookup.block;
                ++hits;
            } else {
                cpu.pc = pc;
                block = find_block(pc, memory, dispatch);
                lookup = {pc, block};
            }
        } else
            ++hits;
        if (previous && previous_generation == cache_generation_) {
            if (previous->links_generation != cache_generation_) {
                previous->successors = {block, nullptr};
                previous->links_generation = cache_generation_;
            } else if (previous->successors[0] != block) {
                previous->successors[1] = previous->successors[0];
                previous->successors[0] = block;
            }
        }
    block_ready:
        // FixedComponents validates the whole authenticated helper image itself.
        // Do not compare its entry block first; a declined helper still takes the
        // ordinary block validation path below, including mapping/permission checks.
        if (block->accelerated) {
            cpu.pc = pc;
            uint64_t completed;
            try {
                completed = acceleration->execute(cpu, memory, budget - count);
            } catch (...) {
                count += acceleration->completed_instructions();
                throw;
            }
            if (completed) {
                count += completed;
                pc = cpu.pc;
                previous = block;
                previous_generation = cache_generation_;
                continue;
            }
        }
        if (block->epoch != memory.code_epoch) {
            cpu.pc = pc;
            // A direct-cache/successor hit keeps its node through invalidation:
            // revalidate in place instead of looking up the same PC in either map.
            if (!revalidate(*block, memory))
                *block = decode_block(pc, memory, dispatch);
        }
        epoch = block->epoch;
        begin = cursor = block->instructions.data();
        block_instructions = std::min<uint64_t>(block->instruction_count, budget - count);
        end = begin + block_instructions;
        goto *cursor->entry;

    General:
        {
            cpu.pc = pc;
            const bool syscall = execute(cpu, memory, *cursor);
            pc = cpu.pc;
            ++cursor;
            if (syscall)
                goto syscall_return;
            if (cursor == end)
                goto block_finished;
            if (memory.code_epoch != epoch)
                goto block_interrupted;
            goto *cursor->entry;
        }
    ResolveSimd:
        // Resolve by executing once in execute_simd's existing priority order.
        // No speculative register/FP-state copies, and no duplicated FP semantics.
        cpu.pc = pc;
#define RESOLVE_SIMD(Name, Function)                                                                          \
    if (Function(cpu, cursor->raw)) {                                                                         \
        cursor->entry = &&Name;                                                                               \
        goto checked_advance;                                                                                \
    }
        RESOLVE_SIMD(VectorNarrow, execute_vector_narrow)
        RESOLVE_SIMD(VectorShift, execute_vector_shift)
        RESOLVE_SIMD(VectorArithmetic, execute_vector_arithmetic)
        RESOLVE_SIMD(VectorUnary, execute_vector_unary)
        RESOLVE_SIMD(VectorWiden, execute_vector_widen)
        RESOLVE_SIMD(VectorReduction, execute_vector_reduction)
        RESOLVE_SIMD(VectorInteger, execute_vector_integer)
        RESOLVE_SIMD(VectorFloating, execute_vector_floating)
        RESOLVE_SIMD(Simd, execute_simd)
        RESOLVE_SIMD(Floating, execute_floating_point)
#undef RESOLVE_SIMD
        goto General;

#define SIMD_OPERATION(Name, Function)                                                                        \
    Name:                                                                                                    \
    cpu.pc = pc;                                                                                             \
    if (!Function(cpu, cursor->raw))                                                                          \
        goto General;                                                                                        \
    goto checked_advance;
        SIMD_OPERATION(VectorNarrow, execute_vector_narrow)
        SIMD_OPERATION(VectorShift, execute_vector_shift)
        SIMD_OPERATION(VectorArithmetic, execute_vector_arithmetic)
        SIMD_OPERATION(VectorUnary, execute_vector_unary)
        SIMD_OPERATION(VectorWiden, execute_vector_widen)
        SIMD_OPERATION(VectorReduction, execute_vector_reduction)
        SIMD_OPERATION(VectorInteger, execute_vector_integer)
        SIMD_OPERATION(VectorFloating, execute_vector_floating)
        SIMD_OPERATION(Simd, execute_simd)
        SIMD_OPERATION(Floating, execute_floating_point)
#undef SIMD_OPERATION

#if defined(__aarch64__)
    ScalarFma64: {
        const double a = read_fp64(cpu, cursor->rn), b = read_fp64(cpu, cursor->rm),
                     c = read_fp64(cpu, cursor->ra);
        double result;
        {
            FloatingPointEnvironment environment(cpu);
            asm volatile("fmadd %d0, %d1, %d2, %d3" : "=w"(result) : "w"(a), "w"(b), "w"(c));
        }
        write_fp64(cpu, cursor->rd, result);
        goto checked_advance;
    }
    ScalarSqrt64: {
        const double value = read_fp64(cpu, cursor->rn);
        double result;
        {
            FloatingPointEnvironment environment(cpu);
            asm volatile("fsqrt %d0, %d1" : "=w"(result) : "w"(value));
        }
        write_fp64(cpu, cursor->rd, result);
        goto checked_advance;
    }
    ScalarUnsignedToDouble64: {
        const uint64_t value = cpu.get(cursor->rn);
        double result;
        {
            FloatingPointEnvironment environment(cpu);
            asm volatile("ucvtf %d0, %x1" : "=w"(result) : "r"(value));
        }
        write_fp64(cpu, cursor->rd, result);
        goto checked_advance;
    }
    ScalarAdd64: {
        const double a = read_fp64(cpu, cursor->rn), b = read_fp64(cpu, cursor->rm);
        double result;
        {
            FloatingPointEnvironment environment(cpu);
            asm volatile("fadd %d0, %d1, %d2" : "=w"(result) : "w"(a), "w"(b));
        }
        write_fp64(cpu, cursor->rd, result);
        goto checked_advance;
    }
    ScalarCompare64: {
        const double a = read_fp64(cpu, cursor->rn), b = read_fp64(cpu, cursor->rm);
        uint64_t flags;
        {
            FloatingPointEnvironment environment(cpu);
            asm volatile("fcmp %d1, %d2\n\tmrs %0, nzcv" : "=r"(flags) : "w"(a), "w"(b) : "cc");
        }
        cpu.nzcv = (flags >> 28) & 15;
        goto checked_advance;
    }
    ScalarSelect64: {
        const double value = read_fp64(cpu, cpu.condition(cursor->flags) ? cursor->rn : cursor->rm);
        write_fp64(cpu, cursor->rd, value);
        goto checked_advance;
    }
    VectorFma2D: {
        using Vector = double __attribute__((vector_size(16)));
        Vector value, a, b;
        std::memcpy(&value, cpu.v[cursor->rd].data(), sizeof(value));
        std::memcpy(&a, cpu.v[cursor->rn].data(), sizeof(a));
        std::memcpy(&b, cpu.v[cursor->rm].data(), sizeof(b));
        {
            FloatingPointEnvironment environment(cpu);
            asm volatile("fmla %0.2d, %1.2d, %2.2d" : "+w"(value) : "w"(a), "w"(b));
        }
        std::memcpy(cpu.v[cursor->rd].data(), &value, sizeof(value));
        goto checked_advance;
    }
#else
    ScalarFma64:
    ScalarSqrt64:
    ScalarUnsignedToDouble64:
    ScalarAdd64:
    ScalarCompare64:
    ScalarSelect64:
    VectorFma2D:
        goto General;
#endif

    Noop:
        goto advance;
    Syscall:
        pc += 4;
        ++cursor;
        goto syscall_return;
    Branch:
        pc += cursor->immediate;
        goto terminal;
    BranchLink:
        cpu.x[30] = pc + 4;
        pc += cursor->immediate;
        goto terminal;
    BranchRegister:
        pc = cpu.get(cursor->rn);
        goto terminal;
    BranchLinkRegister:
        {
            const auto target = cpu.get(cursor->rn);
            cpu.x[30] = pc + 4;
            pc = target;
            goto terminal;
        }
    // Fallthrough retains the former block boundary's code-epoch check.
#define CONDITIONAL_BRANCH(Name, ...)                                                                        \
    Name:                                                                                                   \
    if (__VA_ARGS__)                                                                                        \
        goto conditional_taken;                                                                             \
    goto checked_advance;
        CONDITIONAL_BRANCH(ConditionalBranch, scalar::condition(cpu, cursor->flags))
        CONDITIONAL_BRANCH(CompareBranch32, (cpu.get(cursor->rd, 32) == 0) != bool(cursor->flags))
        CONDITIONAL_BRANCH(CompareBranch64, (cpu.get(cursor->rd) == 0) != bool(cursor->flags))
        CONDITIONAL_BRANCH(TestBranch, ((cpu.get(cursor->rd) >> cursor->shift) & 1) == cursor->flags)
#undef CONDITIONAL_BRANCH
    conditional_taken:
        // Skip the decoded suffix without charging it to the guest budget.
        pc += cursor->immediate;
        count += cursor->auxiliary;
        goto block_retired;
    Address:
        cpu.set(cursor->rd,
                cursor->flags ? (pc & ~uint64_t{4095}) + (cursor->immediate << 12) : pc + cursor->immediate);
        goto advance;
    Move32:
        cpu.set(cursor->rd, uint32_t(cursor->immediate));
        goto advance;
    Move64:
        cpu.set(cursor->rd, cursor->immediate);
        goto advance;
    Keep32:
        cpu.set(cursor->rd, (cpu.get(cursor->rd) & cursor->auxiliary) | cursor->immediate, 32);
        goto advance;
    Keep64:
        cpu.set(cursor->rd, (cpu.get(cursor->rd) & cursor->auxiliary) | cursor->immediate);
        goto advance;
    CompareRegister64:
        scalar::arithmetic<64>(cpu, cpu.get(cursor->rn), cpu.get(cursor->rm),
                               decoded::set_flags | decoded::subtract);
        goto advance;

#define REGISTER_OPERATION(Name, Family, ...)                                                                \
    Name:                                                                                                    \
    Family::__VA_ARGS__(cpu, *cursor);                                                                       \
    goto advance;
        REGISTER_OPERATION(AddImmediate32, scalar, add_immediate<32>)
        REGISTER_OPERATION(AddImmediate64, scalar, add_immediate<64>)
        REGISTER_OPERATION(AddRegister32, scalar, add_register<32>)
        REGISTER_OPERATION(AddRegister64, scalar, add_register<64>)
        REGISTER_OPERATION(AddCarry32, scalar, add_carry<32>)
        REGISTER_OPERATION(AddCarry64, scalar, add_carry<64>)
        REGISTER_OPERATION(LogicalImmediate32, scalar, logical<32, true>)
        REGISTER_OPERATION(LogicalImmediate64, scalar, logical<64, true>)
        REGISTER_OPERATION(LogicalRegister32, scalar, logical<32, false>)
        REGISTER_OPERATION(LogicalRegister64, scalar, logical<64, false>)
        REGISTER_OPERATION(Bitfield32, scalar, bitfield<32>)
        REGISTER_OPERATION(Bitfield64, scalar, bitfield<64>)
        REGISTER_OPERATION(Extract32, scalar, extract<32>)
        REGISTER_OPERATION(Extract64, scalar, extract<64>)
        REGISTER_OPERATION(Select32, scalar, select<32>)
        REGISTER_OPERATION(Select64, scalar, select<64>)
        REGISTER_OPERATION(Compare32, scalar, compare<32>)
        REGISTER_OPERATION(Compare64, scalar, compare<64>)
        REGISTER_OPERATION(TwoSource32, scalar, two_source<32>)
        REGISTER_OPERATION(TwoSource64, scalar, two_source<64>)
        REGISTER_OPERATION(OneSource32, scalar, one_source<32>)
        REGISTER_OPERATION(OneSource64, scalar, one_source<64>)
        REGISTER_OPERATION(Multiply32, scalar, multiply<32>)
        REGISTER_OPERATION(Multiply64, scalar, multiply<64>)
        REGISTER_OPERATION(MultiplyHigh, scalar, multiply_high)
        REGISTER_OPERATION(VectorToRegister32, vector, to_register<4>)
        REGISTER_OPERATION(VectorToRegister64, vector, to_register<8>)
        REGISTER_OPERATION(RegisterToVector32, vector, from_register<4>)
        REGISTER_OPERATION(RegisterToVector64, vector, from_register<8>)
        REGISTER_OPERATION(VectorHighToRegister, vector, high_to_register)
        REGISTER_OPERATION(RegisterToVectorHigh, vector, from_register_high)
        REGISTER_OPERATION(VectorImmediate, vector, immediate)
        REGISTER_OPERATION(VectorLogical, vector, logical)
#undef REGISTER_OPERATION

#define LOAD_OPERATION(Name, Family, ...)                                                                    \
    Name:                                                                                                    \
    cpu.pc = pc;                                                                                             \
    Family::__VA_ARGS__(cpu, memory, *cursor);                                                               \
    goto advance;
        LOAD_OPERATION(Load8, scalar, load<1>)
        LOAD_OPERATION(Load16, scalar, load<2>)
        LOAD_OPERATION(Load32, scalar, load<4>)
        LOAD_OPERATION(Load64, scalar, load<8>)
        LOAD_OPERATION(LoadPair32, scalar, pair<4, true>)
        LOAD_OPERATION(LoadPair64, scalar, pair<8, true>)
        LOAD_OPERATION(VectorLoad32, vector, load<4>)
        LOAD_OPERATION(VectorLoad64, vector, load<8>)
        LOAD_OPERATION(VectorLoad128, vector, load<16>)
        LOAD_OPERATION(VectorLoadPair32, vector, pair<4, true>)
        LOAD_OPERATION(VectorLoadPair64, vector, pair<8, true>)
        LOAD_OPERATION(VectorLoadPair128, vector, pair<16, true>)
#undef LOAD_OPERATION

#define STORE_OPERATION(Name, Family, ...)                                                                   \
    Name:                                                                                                    \
    cpu.pc = pc;                                                                                             \
    Family::__VA_ARGS__(cpu, memory, *cursor);                                                               \
    goto checked_advance;
        STORE_OPERATION(Store8, scalar, store<1>)
        STORE_OPERATION(Store16, scalar, store<2>)
        STORE_OPERATION(Store32, scalar, store<4>)
        STORE_OPERATION(Store64, scalar, store<8>)
        STORE_OPERATION(StorePair32, scalar, pair<4, false>)
        STORE_OPERATION(StorePair64, scalar, pair<8, false>)
        STORE_OPERATION(VectorStore32, vector, store<4>)
        STORE_OPERATION(VectorStore64, vector, store<8>)
        STORE_OPERATION(VectorStore128, vector, store<16>)
        STORE_OPERATION(VectorStorePair32, vector, pair<4, false>)
        STORE_OPERATION(VectorStorePair64, vector, pair<8, false>)
        STORE_OPERATION(VectorStorePair128, vector, pair<16, false>)
#undef STORE_OPERATION

    LoadLiteral32:
        cpu.pc = pc;
        scalar::literal<4>(cpu, memory, *cursor, pc);
        goto advance;
    LoadLiteral64:
        cpu.pc = pc;
        scalar::literal<8>(cpu, memory, *cursor, pc);
        goto advance;
    ReadTls:
        cpu.set(cursor->rd, cpu.tls);
        goto advance;
    WriteTls:
        cpu.tls = cpu.get(cursor->rd);
        goto advance;
    ReadFlags:
        cpu.set(cursor->rd, uint64_t(cpu.nzcv) << 28);
        goto advance;
    WriteFlags:
        cpu.nzcv = (cpu.get(cursor->rd) >> 28) & 15;
        goto advance;

    checked_advance:
        pc += 4;
        ++cursor;
        if (cursor == end)
            goto block_finished;
        if (memory.code_epoch != epoch)
            goto block_interrupted;
        goto *cursor->entry;
    advance:
        pc += 4;
        ++cursor;
        if (cursor != end)
            goto *cursor->entry;
        goto block_finished;
    terminal:
        ++cursor;
    block_finished:
        // Normal block retirement needs no division by CachedInstruction's stride.
        count += block_instructions;
        goto block_retired;
    block_interrupted:
        count += cursor - begin;
    block_retired:
        begin = cursor = nullptr;
        previous = block;
        previous_generation = cache_generation_;
    }
    cpu.pc = pc;
    return {Trap::Budget, count};

syscall_return:
    count += cursor - begin;
    begin = cursor = nullptr;
    cpu.pc = pc;
    return {Trap::Syscall, count};
}
} // namespace operit::arm64
