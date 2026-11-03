#pragma once
#include "cpu/aarch64/state.h"

namespace operit::arm64 {
// Baseline FP controls, plus FZ16 when half arithmetic is available. Exception
// trapping and the later AFP controls are not advertised by this implementation.
// AHP, DN, FZ, RMode, FZ16; QC and the six cumulative exception flags.
// AArch64 NZCV is separate: the AArch32 FPSR condition bits are RES0 here.
inline constexpr uint64_t floating_control_mask = 0x07c80000;
inline constexpr uint64_t floating_status_mask = 0x0800009f;
inline uint64_t read_floating_control(const CpuState &cpu) noexcept {
    return cpu.fpcr & floating_control_mask;
}
uint64_t read_floating_status(const CpuState &cpu) noexcept;
void write_floating_control(CpuState &cpu, uint64_t value) noexcept;
void write_floating_status(CpuState &cpu, uint64_t value) noexcept;
bool host_supports_half_floating_point() noexcept;

#if defined(__aarch64__)
// Keep the host's controls and sticky flags private. Start each instruction with
// empty host flags, then merge only architectural cumulative exception flags.
// All arithmetic inside this scope is statically assembled ARM instructions.
class FloatingPointEnvironment {
    CpuState &cpu_;
    uint64_t control_, status_;

  public:
    explicit FloatingPointEnvironment(CpuState &cpu) noexcept : cpu_(cpu) {
        asm volatile("mrs %0, fpcr\n\tmrs %1, fpsr" : "=r"(control_), "=r"(status_));
        const uint64_t control = read_floating_control(cpu);
        asm volatile("msr fpcr, %0\n\tmsr fpsr, xzr" : : "r"(control) : "memory");
    }
    ~FloatingPointEnvironment() noexcept {
        uint64_t flags;
        asm volatile("mrs %0, fpsr\n\tmsr fpcr, %1\n\tmsr fpsr, %2"
                     : "=&r"(flags) : "r"(control_), "r"(status_) : "memory");
        cpu_.fpsr |= flags & 0x9f;
    }
    FloatingPointEnvironment(const FloatingPointEnvironment &) = delete;
    FloatingPointEnvironment &operator=(const FloatingPointEnvironment &) = delete;
};
#endif
bool execute_floating_point(CpuState &cpu, uint32_t instruction);
} // namespace operit::arm64
