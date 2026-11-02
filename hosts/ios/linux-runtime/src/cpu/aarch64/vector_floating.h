#pragma once
#include "cpu/aarch64/state.h"

namespace operit::arm64 {
bool execute_vector_fixed(CpuState &cpu, uint32_t raw);
bool execute_vector_floating_indexed(CpuState &cpu, uint32_t raw);
// These entry points also accept their scalar Advanced SIMD forms. A false
// result leaves architectural state untouched and permits another decoder.
bool execute_vector_floating_conversion(CpuState &cpu, uint32_t raw);
bool execute_vector_floating_pairwise(CpuState &cpu, uint32_t raw);
bool execute_scalar_floating_simd(CpuState &cpu, uint32_t raw);
bool execute_vector_floating(CpuState &cpu, uint32_t raw);
} // namespace operit::arm64
