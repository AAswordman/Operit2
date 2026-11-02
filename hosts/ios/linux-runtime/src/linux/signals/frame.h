#pragma once
#include "linux/signals/signals.h"

namespace operit::arm64::signals {
bool enter_handler(Memory &memory, CpuState &cpu, ThreadState &thread, uint64_t saved_mask, const Info &info,
                   const Action &action, uint64_t trampoline);
} // namespace operit::arm64::signals
