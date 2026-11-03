#pragma once
#include <cstdint>

namespace operit::arm64 {
class Memory;
struct CpuState;

namespace threads {
// Linux rseq ABI size through mm_cid and its allocation alignment.
inline constexpr uint32_t rseq_feature_size = 28;
inline constexpr uint32_t rseq_original_size = 32;
inline constexpr uint32_t rseq_alignment = 32;

// One instance per guest task. Only the coordinator may mutate it, while that
// task is not executing; fork without CLONE_VM copies it, exec/exit discard it.
struct RseqState {
    uint64_t address = 0;
    uint32_t length = 0, signature = 0, events = 0;
    bool notify = false;
};

enum class RseqEvent : uint32_t { Preempt = 1, Signal = 2, Migrate = 4 };

// Syscall arguments are the AArch64 ABI values, truncated to their u32 types.
// The return value is zero or a negative Linux errno. Registration schedules
// a resume notification; inaccessible mappings can instead fault on resume.
int64_t rseq_syscall(Memory &memory, RseqState &state, uint64_t address,
                     uint32_t length, uint32_t flags, uint32_t signature);
void rseq_event(RseqState &state, RseqEvent event) noexcept;

// Consume a pending notification before guest execution, using the CPU slot
// actually assigned to this task. A false result must force SIGSEGV and prevent
// guest execution or construction of the original signal frame. With a signal
// handler, set Signal and call this on the final saved CpuState after syscall
// restart adjustment, but before constructing the signal frame. Set in_syscall
// only for a kernel-only resume with no user registers to fix up.
bool rseq_resume(Memory &memory, CpuState &cpu, RseqState &state,
                 uint32_t cpu_id, uint32_t mm_cid, bool in_syscall = false);

// Mark Preempt at every execution boundary before its CPU can run another task;
// mark Migrate if the next actual CPU differs, and resume before that task runs.
// Both serial and parallel execution and host-visible budget boundaries count.
// The caller must ensure that no two running tasks share one guest CPU. Under
// that condition the CPU slot is also a valid per-mm concurrency ID.
RseqState rseq_fork(const RseqState &parent, bool clone_vm) noexcept;
void rseq_reset(RseqState &state) noexcept;
} // namespace threads
} // namespace operit::arm64
