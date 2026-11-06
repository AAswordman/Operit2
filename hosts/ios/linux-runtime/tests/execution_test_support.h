#pragma once
#include "cpu/aarch64/semantics.h"
namespace operit::arm64 {
inline bool execute_one(CpuState &cpu,Memory &memory,const Instruction &instruction) {
    return select_semantic(instruction)(cpu,memory,instruction);
}
}
