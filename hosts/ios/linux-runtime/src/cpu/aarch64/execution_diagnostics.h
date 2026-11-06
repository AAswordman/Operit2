#pragma once
#include "cpu/aarch64/executor_abi.h"
#include <array>
#include <cstddef>
#include <cstdint>

namespace operit::arm64 {
struct ExecutionFrame;
struct BlockGraph;

enum class ExecutionDiagnosticEvent : unsigned {
    BlockAdmitted=ED_EVENT_BLOCK_ADMITTED,
    SlowEntry=ED_EVENT_SLOW_ENTRY,
    MemoryReadAttempt=ED_EVENT_MEMORY_READ_ATTEMPT,
    MemoryWriteAttempt=ED_EVENT_MEMORY_WRITE_ATTEMPT,
    MemoryReadCrossPage=ED_EVENT_MEMORY_READ_CROSS_PAGE,
    MemoryWriteCrossPage=ED_EVENT_MEMORY_WRITE_CROSS_PAGE,
    PageCacheReadHit=ED_EVENT_PAGE_CACHE_READ_HIT,
    PageCacheWriteHit=ED_EVENT_PAGE_CACHE_WRITE_HIT,
    TranslationReadFallback=ED_EVENT_TRANSLATION_READ_FALLBACK,
    TranslationWriteFallback=ED_EVENT_TRANSLATION_WRITE_FALLBACK,
    FallbackWay0Hit=ED_EVENT_FALLBACK_WAY0_HIT,
    FallbackWay1Hit=ED_EVENT_FALLBACK_WAY1_HIT,
    FileBackedProbe=ED_EVENT_FILE_BACKED_PROBE,
    FileValidatedHit=ED_EVENT_FILE_VALIDATED_HIT,
    FileObservationHit=ED_EVENT_FILE_OBSERVATION_HIT,
    FileLockAttempt=ED_EVENT_FILE_LOCK_ATTEMPT,
    FileLockAcquired=ED_EVENT_FILE_LOCK_ACQUIRED,
    FileLockBlocked=ED_EVENT_FILE_LOCK_BLOCKED,
    LocalEdge=ED_EVENT_LOCAL_EDGE,
    CrossEdgeAttempt=ED_EVENT_CROSS_EDGE_ATTEMPT,
    FallthroughLocal=ED_EVENT_FALLTHROUGH_LOCAL,
    FallthroughCross=ED_EVENT_FALLTHROUGH_CROSS,
    BridgeEntry=ED_EVENT_BRIDGE_ENTRY,
    BridgeTransferred=ED_EVENT_BRIDGE_TRANSFERRED,
    IndirectEdge=ED_EVENT_INDIRECT_EDGE,
    IndirectCacheEntry=ED_EVENT_INDIRECT_CACHE_ENTRY,
    IndirectCacheHit=ED_EVENT_INDIRECT_CACHE_HIT,
    NextElided=ED_EVENT_NEXT_ELIDED,
    VectorSelectedResidentRead=ED_EVENT_VECTOR_SELECTED_RESIDENT_READ,
    VectorSelectedSpillRead=ED_EVENT_VECTOR_SELECTED_SPILL_READ,
    GpBridgeSpill=ED_EVENT_GP_BRIDGE_SPILL,
    GpBridgeReload=ED_EVENT_GP_BRIDGE_RELOAD,
    VectorBridgeSpill=ED_EVENT_VECTOR_BRIDGE_SPILL,
    VectorBridgeReload=ED_EVENT_VECTOR_BRIDGE_RELOAD,
    LazyFpEnter=ED_EVENT_LAZY_FP_ENTER,
    BridgeValidateCall=ED_EVENT_BRIDGE_VALIDATE_CALL,
    ActiveFpHostSwitch=ED_EVENT_ACTIVE_FP_HOST_SWITCH,
    BridgeFpGuestRestore=ED_EVENT_BRIDGE_FP_GUEST_RESTORE,
    NativeProofAttempt=ED_EVENT_NATIVE_PROOF_ATTEMPT,
    NativeProofSuccess=ED_EVENT_NATIVE_PROOF_SUCCESS,
};

#if OPERIT_EXECUTION_DIAGNOSTICS
// Counts selected register operand bindings in a handler template, rather than
// host instructions. A conditional/faulting handler can skip some bindings;
// flags expose that limitation instead of labelling these actual spill traffic.
// flags: 1 = conditional operands, 2 = faultable memory, 4 = optional writeback,
//        8 = FMA third source counted separately by the selector events above.
struct ExecutionDiagnosticProfile {
    std::array<uint8_t,8> bindings;
    uint8_t flags;
};
const ExecutionDiagnosticProfile *execution_diagnostic_profiles(size_t &count) noexcept;
const char *execution_diagnostic_static_code_name(uint8_t code) noexcept;

struct alignas(64) ExecutionDiagnosticSite {
    uint64_t cursor=0,pc=0,entry=0,samples=0;
    uint64_t immediate=0,auxiliary=0,operands=0,profile=0;
};
static_assert(sizeof(ExecutionDiagnosticSite)==ED_SITE_SIZE);
static_assert(offsetof(ExecutionDiagnosticSite,samples)==24);
static_assert(offsetof(ExecutionDiagnosticSite,profile)==56);
struct ExecutionDiagnosticsStorage;
struct ExecutionDiagnostics {
    std::array<uint64_t,ED_HANDLER_CAPACITY> handlers{};
    std::array<uint64_t,ED_PROFILE_CAPACITY> profiles{};
    std::array<uint64_t,ED_EVENT_CAPACITY> events{};
    uint64_t sample_remaining=1,sample_interval_mask=511;
    uint64_t random_state=0x9e3779b97f4a7c15ULL;
    ExecutionDiagnosticSite *sites=nullptr;
    uint64_t site_set_mask=0,samples=0,collisions=0,dropped=0;
    std::array<uint64_t,32> branch_conditions{},branch_taken{};
    std::array<uint64_t,32> compare_branch_conditions{},compare_branch_taken{};
    ExecutionDiagnosticsStorage *storage=nullptr;
    ExecutionDiagnostics()=default;
    ~ExecutionDiagnostics();
    ExecutionDiagnostics(const ExecutionDiagnostics &)=delete;
    ExecutionDiagnostics &operator=(const ExecutionDiagnostics &)=delete;
};
static_assert(offsetof(ExecutionDiagnostics,handlers)==ED_HANDLERS);
static_assert(offsetof(ExecutionDiagnostics,profiles)==ED_PROFILES);
static_assert(offsetof(ExecutionDiagnostics,events)==ED_EVENTS);
static_assert(offsetof(ExecutionDiagnostics,sample_remaining)==ED_SAMPLE_REMAINING);
static_assert(offsetof(ExecutionDiagnostics,sample_interval_mask)==ED_SAMPLE_INTERVAL_MASK);
static_assert(offsetof(ExecutionDiagnostics,random_state)==ED_RANDOM_STATE);
static_assert(offsetof(ExecutionDiagnostics,sites)==ED_SITES);
static_assert(offsetof(ExecutionDiagnostics,site_set_mask)==ED_SITE_SET_MASK);
static_assert(offsetof(ExecutionDiagnostics,samples)==ED_SAMPLES);
static_assert(offsetof(ExecutionDiagnostics,collisions)==ED_COLLISIONS);
static_assert(offsetof(ExecutionDiagnostics,dropped)==ED_DROPPED);
static_assert(offsetof(ExecutionDiagnostics,branch_conditions)==ED_BRANCH_CONDITIONS);
static_assert(offsetof(ExecutionDiagnostics,branch_taken)==ED_BRANCH_TAKEN);
static_assert(offsetof(ExecutionDiagnostics,compare_branch_conditions)==ED_COMPARE_BRANCH_CONDITIONS);
static_assert(offsetof(ExecutionDiagnostics,compare_branch_taken)==ED_COMPARE_BRANCH_TAKEN);
#endif

// All calls require the owning engine to be outside operit_static_execute and
// quiescent. Counters are per frame, non-atomic, and never shared across engines.
// reset is transactional on allocation failure. site_sets must be a power of
// two (1..2^20); sample_shift is 0..30. Four 64-byte sites occupy each set.
// Intervals are uniform in [1, 2^(sample_shift+1)] (mean 2^sample_shift+0.5).
// Calling reset once before collection enables cursor sampling. Without it,
// aggregate counters still work and samples are reported as dropped.
bool execution_diagnostics_reset(ExecutionFrame &frame,unsigned sample_shift=8,
                                 size_t site_sets=16384) noexcept;
// Call after lowering, before first use, also for already-cached graphs after
// reset. Idempotent per graph lifetime. Copies owned instruction/operation data;
// neither dump nor assembly sampling dereferences an expired graph or cursor.
void execution_diagnostics_observe_graph(ExecutionFrame &frame,const BlockGraph &graph) noexcept;
// top==0 emits all retained sites. Writes one complete JSON document; failure
// is returned and never injected into guest execution. Disabled builds return
// false from write_json and otherwise provide no-op functions.
bool execution_diagnostics_write_json(const ExecutionFrame &frame,const char *path,
                                      size_t top=256) noexcept;
void execution_diagnostics_count(ExecutionFrame &frame,ExecutionDiagnosticEvent event,
                                 uint64_t amount=1) noexcept;
}
