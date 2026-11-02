#pragma once
#include "cpu/aarch64/state.h"
namespace operit::arm64 {
#if defined(__aarch64__)
// Only statically assembled instructions run here. No guest opcode becomes host code.
class FloatingContext {
    CpuState &cpu_;
    uint64_t control_, status_;

  public:
    explicit FloatingContext(CpuState &cpu) : cpu_(cpu) {
        asm volatile("mrs %0, fpcr\n mrs %1, fpsr" : "=r"(control_), "=r"(status_));
        asm volatile("msr fpcr, %0\n msr fpsr, %1" ::"r"(cpu.fpcr), "r"(cpu.fpsr) : "memory");
    }
    ~FloatingContext() {
        asm volatile("mrs %0, fpsr" : "=r"(cpu_.fpsr));
        asm volatile("msr fpcr, %0\n msr fpsr, %1" ::"r"(control_), "r"(status_) : "memory");
    }
};
#endif
} // namespace operit::arm64
