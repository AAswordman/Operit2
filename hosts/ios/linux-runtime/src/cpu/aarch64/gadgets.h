#pragma once
#include "cpu/aarch64/decoder.h"
#include "cpu/aarch64/state.h"
#include "memory/translation_layout.h"
#include <cstddef>

namespace operit::arm64 {
// All entry points are compiled into the signed image. These records are data;
// decoding never allocates executable memory or copies host instructions.
struct NativeGadget {
    const void *entry = nullptr;
    uint64_t immediate = 0, auxiliary = 0;
    // Operand bytes are CpuState slot indices, with SP/ZR resolved at decode.
    uint8_t rd = 0, rn = 0, rm = 0, ra = 0, shift = 0, flags = 0;
};
static_assert(sizeof(NativeGadget) == 32);
NativeGadget decode_gadget(const Instruction &instruction, uint64_t pc);
extern "C" uint64_t operit_arm64_gadgets_run(CpuState *, const GadgetMemory *,
                                           const NativeGadget *, uint64_t count);
} // namespace operit::arm64
