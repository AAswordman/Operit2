#pragma once
#include "base/support.h"

namespace operit::arm64 {
struct InstructionFault : Failure {
    enum class Kind : uint8_t { Undefined, Breakpoint, Halt, Privileged };
    uint64_t pc;
    uint32_t instruction;
    Kind kind;

    InstructionFault(uint64_t address, uint32_t raw, Kind reason)
        : Failure(std::string(description(reason)) + " " + hex(raw) + " at PC " + hex(address)), pc(address),
          instruction(raw), kind(reason) {}

  private:
    static const char *description(Kind reason) {
        switch (reason) {
        case Kind::Undefined:
            return "AArch64 undefined instruction";
        case Kind::Breakpoint:
            return "AArch64 breakpoint";
        case Kind::Halt:
            return "AArch64 halt at EL0";
        case Kind::Privileged:
            return "AArch64 privileged instruction at EL0";
        }
        return "AArch64 instruction fault";
    }
};
} // namespace operit::arm64
