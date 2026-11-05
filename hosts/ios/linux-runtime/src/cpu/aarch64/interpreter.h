#pragma once
#include "acceleration/fixed_components.h"
#include "cpu/aarch64/decoder.h"
#include "cpu/aarch64/gadgets.h"
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
#include <unordered_map>
#include <vector>

namespace operit::arm64 {
enum class Trap { Budget, Syscall };
struct Execution {
    Trap trap;
    uint64_t instructions;
};
class Interpreter {
    struct CachedInstruction : Instruction {
        // An address in run's static dispatch table, never generated host code.
        mutable const void *entry;
        const NativeGadget *gadget;
        CachedInstruction(const Instruction &instruction, const void *target, const NativeGadget *native)
            : Instruction(instruction), entry(target), gadget(native) {}
    };
    struct Block {
        uint64_t address = 0, epoch = 0, links_generation = 0;
        std::vector<CachedInstruction> instructions;
        std::vector<NativeGadget> gadgets;
        // Keep raw code contiguous: epoch revalidation need not scan decoded operands.
        std::vector<uint32_t> words;
        std::array<Block *, 2> successors{};
        uint8_t instruction_count = 0;
        bool accelerated = false;
    };
    struct Lookup {
        uint64_t pc = UINT64_MAX;
        Block *block = nullptr;
    };
    // Rotating generations retain the recent working set without clearing every hot block.
    std::array<std::unordered_map<uint64_t, Block>, 2> blocks_;
    std::array<Lookup, 16384> lookups_{};
    const Memory *memory_ = nullptr;
    const FixedComponents *acceleration_ = nullptr;
    uint64_t acceleration_generation_ = 0;
    unsigned active_generation_ = 0;
    uint64_t cache_generation_ = 1;
    bool revalidate(Block &block, Memory &memory);
    Block *find_block(uint64_t address, Memory &memory, const void *const *dispatch);
    Block decode_block(uint64_t address, Memory &memory, const void *const *dispatch);

  public:
    Interpreter() = default;
    Interpreter(const Interpreter &) = delete;
    Interpreter &operator=(const Interpreter &) = delete;
    uint64_t executed_instructions = 0, decoded_blocks = 0, cache_hits = 0;
    uint64_t native_gadget_instructions = 0;
    bool use_native_gadgets = true;
    Execution run(CpuState &cpu, Memory &memory, uint64_t budget, FixedComponents *acceleration = nullptr);
};
/* Returns true for SVC. Exceptions retain the faulting instruction's PC. */
bool execute(CpuState &cpu, Memory &memory, const Instruction &instruction);
} // namespace operit::arm64
