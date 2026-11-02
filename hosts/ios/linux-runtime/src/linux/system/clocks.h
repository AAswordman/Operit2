#pragma once
#include <cstdint>

namespace operit::arm64 {
struct CpuState;
class Memory;
class ProcessController;
namespace system {
// AArch64 syscalls 113, 114, 153 and 165. Throws GuestError on Linux errors.
// The caller's execution context must be active on ProcessController.
int64_t dispatch_clocks(CpuState &cpu, Memory &memory, ProcessController &processes);
} // namespace system
} // namespace operit::arm64
