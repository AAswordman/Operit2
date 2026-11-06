#pragma once
#include "cpu/aarch64/decoder.h"
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
namespace operit::arm64 {
using Semantic = bool (*)(CpuState &, Memory &, const Instruction &);
// Called by the compiler only; the executable image contains the selected address.
Semantic select_semantic(const Instruction &instruction);
}
