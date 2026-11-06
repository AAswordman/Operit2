#pragma once
#include "cpu/aarch64/state.h"
#include "cpu/aarch64/execution_diagnostics.h"
#include <array>
#include <atomic>
#include <cstddef>
#include <exception>
namespace operit::arm64 {
class ExecutionEngine;
class Memory;
// One guest residency ABI spans every graph: arguments X0-X2, the first
// callee-saved locals X19-X20, SP, and SIMD argument/result registers V0-V7.
// Other registers stay in shadow; helpers and exits materialize this same set.
inline constexpr std::array<uint8_t,6> resident_gp{0,1,2,19,20,31};
inline constexpr std::array<uint8_t,8> resident_vector{0,1,2,3,4,5,6,7};
struct BlockGraph;
struct StaticOperation;
enum class StopReason { Budget, Syscall, Failure };
// This prefix is an assembly ABI. executor_abi.h and the assertions below are
// checked by every target build, including cross-compiled iOS archives.
struct alignas(16) ExecutionFrame {
    std::array<uint64_t,6> gp{};
    alignas(16) std::array<std::array<uint8_t,16>,8> vectors{};
    uint64_t flags=0,completed=0,budget=0;
    const StaticOperation *resume=nullptr;
    const void *fast_first=nullptr,*fast_second=nullptr;
    const uint64_t *mapping_epoch=nullptr;
    const std::atomic<uint64_t> *code_epoch=nullptr;
    uint64_t epoch=0,file_generation=0,proof_generation=0;
    uint64_t host_fpcr=0,host_fpsr=0,guest_fpcr=0,pending_pc=0,block_hits=0;
    CpuState shadow;
    ExecutionEngine *engine=nullptr;
    Memory *memory=nullptr;
    BlockGraph *graph=nullptr;
    std::exception_ptr exception;
    StopReason reason=StopReason::Budget;
    const void *fast_pages=nullptr;
    uint64_t block_base=0;
    const void *file_observations=nullptr;
#if OPERIT_EXECUTION_DIAGNOSTICS
    ExecutionDiagnostics diagnostics;
#endif
    void publish(uint32_t gp_mask=UINT32_MAX,uint32_t vector_mask=UINT32_MAX);
    void reload(uint32_t gp_mask=UINT32_MAX,uint32_t vector_mask=UINT32_MAX);
};
static_assert(offsetof(ExecutionFrame,gp)==0 && offsetof(ExecutionFrame,vectors)==48);
static_assert(offsetof(ExecutionFrame,flags)==176 && offsetof(ExecutionFrame,completed)==184);
static_assert(offsetof(ExecutionFrame,budget)==192 && offsetof(ExecutionFrame,resume)==200);
static_assert(offsetof(ExecutionFrame,fast_first)==208 && offsetof(ExecutionFrame,mapping_epoch)==224);
static_assert(offsetof(ExecutionFrame,code_epoch)==232 && offsetof(ExecutionFrame,epoch)==240);
static_assert(offsetof(ExecutionFrame,file_generation)==248 && offsetof(ExecutionFrame,host_fpcr)==264);
static_assert(offsetof(ExecutionFrame,guest_fpcr)==280 && offsetof(ExecutionFrame,pending_pc)==288);
static_assert(offsetof(ExecutionFrame,block_hits)==296 && offsetof(ExecutionFrame,shadow)==304);
static_assert(offsetof(ExecutionFrame,shadow)+offsetof(CpuState,pc)==1072);
static_assert(offsetof(ExecutionFrame,shadow)+offsetof(CpuState,tls)==1080);
static_assert(offsetof(ExecutionFrame,shadow)+offsetof(CpuState,fpsr)==1096);
static_assert(offsetof(ExecutionFrame,graph)==1152);
static_assert(offsetof(ExecutionFrame,fast_pages)==1176 && offsetof(ExecutionFrame,block_base)==1184);
static_assert(offsetof(ExecutionFrame,file_observations)==1192);
#if OPERIT_EXECUTION_DIAGNOSTICS
static_assert(offsetof(ExecutionFrame,diagnostics)==EF_DIAGNOSTICS);
#endif
extern "C" void operit_static_execute(ExecutionFrame *frame,const StaticOperation *entry);
extern "C" const StaticOperation *operit_static_slow(ExecutionFrame *frame,const StaticOperation *op) noexcept;
}
