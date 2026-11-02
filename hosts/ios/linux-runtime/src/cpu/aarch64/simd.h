#pragma once
#include "cpu/aarch64/state.h"

namespace operit::arm64 {
bool execute_simd(CpuState &cpu, uint32_t instruction);
}
