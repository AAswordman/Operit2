#pragma once
#include "acceleration/fixed_manifest.h"
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"

namespace operit::arm64::fixed {
class FixedExecution {
    const FixedManifest &image_;
    uint64_t base_, budget_, epoch_, block_completed_ = 0;
    uint64_t &completed_;

    template <uint64_t Address> uint64_t code_address() const {
        constexpr size_t symbol = [] {
            for (size_t index = 0; index < compiled_musl_symbols.size(); ++index) {
                const auto &program = compiled_musl_symbols[index];
                if (Address >= program.value && Address - program.value < program.size)
                    return index;
            }
            return compiled_musl_symbols.size();
        }();
        static_assert(symbol < compiled_musl_symbols.size(), "fixed program address outside a known symbol");
        return base_ + image_.symbols[symbol].value + (Address - compiled_musl_symbols[symbol].value);
    }

  public:
    CpuState &cpu;
    Memory &memory;
    bool complete = false;

    FixedExecution(CpuState &state, Memory &address_space, const FixedManifest &image, uint64_t base,
                   uint64_t budget, uint64_t &completed)
        : image_(image), base_(base), budget_(budget), epoch_(address_space.code_epoch),
          completed_(completed), cpu(state), memory(address_space) {}
    template <uint64_t Address, unsigned Instructions> bool begin() {
        if (Instructions > budget_ - completed_)
            return false;
        block_completed_ = completed_;
        cpu.pc = code_address<Address>();
        return true;
    }
    template <uint64_t Address, unsigned Preceding> void access() {
        completed_ = block_completed_ + Preceding;
        cpu.pc = code_address<Address>();
    }
    template <uint64_t Next, unsigned Completed> bool stored() {
        if (memory.code_epoch == epoch_)
            return true;
        partial<Next, Completed>();
        return false;
    }
    template <uint64_t Next, unsigned Completed> void partial() {
        completed_ = block_completed_ + Completed;
        cpu.pc = code_address<Next>();
    }
    template <unsigned Instructions, uint64_t Next> void finish() {
        partial<Next, Instructions>();
    }
    template <unsigned Instructions> void returned() {
        completed_ = block_completed_ + Instructions;
        cpu.pc = cpu.x[30];
        complete = true;
    }
};
} // namespace operit::arm64::fixed
