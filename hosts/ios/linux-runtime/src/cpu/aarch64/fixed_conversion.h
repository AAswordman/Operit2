#pragma once
#include "cpu/aarch64/state.h"

namespace operit::arm64 {
// SCVTF/UCVTF and FCVTZS/FCVTZU between W/X and H/S/D, with a fractional
// immediate. Half arithmetic requires host FEAT_FP16; scales are 1..32/64.
bool execute_fixed_conversion(CpuState &cpu, uint32_t instruction);
} // namespace operit::arm64
