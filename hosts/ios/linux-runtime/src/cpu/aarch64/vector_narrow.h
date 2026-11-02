#pragma once
#include "cpu/aarch64/state.h"
namespace operit::arm64 {
bool execute_vector_narrow(CpuState &cpu, uint32_t raw);
}
