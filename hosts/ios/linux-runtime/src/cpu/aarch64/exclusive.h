#pragma once
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
namespace operit::arm64 {
bool execute_exclusive(CpuState &cpu, Memory &memory, uint32_t instruction);
}
