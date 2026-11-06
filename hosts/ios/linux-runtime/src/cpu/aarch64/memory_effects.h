#pragma once
#include "cpu/aarch64/decoder.h"
#include <cstdint>
namespace operit::arm64 {
// Bit 31 names SP for integer effects. ZR never appears in an effect set.
struct Effects {
    uint32_t gp_read=0, gp_write=0, vector_read=0, vector_write=0;
    bool flags_read=false, flags_write=false, memory=false, control=false;
};
Effects instruction_effects(const Instruction &instruction);
}
