#pragma once
#include "acceleration/fixed_components.h"
#include "cpu/aarch64/block_graph.h"
#include "cpu/aarch64/execution_frame.h"
#include "cpu/aarch64/wasm_executor.h"
#include <unordered_map>
#include <deque>
namespace operit::arm64 {
enum class Trap { Budget, Syscall };
struct Execution { Trap trap; uint64_t instructions; };
class ExecutionEngine {
    OPERIT_EXEC_DIAGNOSTIC(execution_diagnostics::State diagnostics_;)
    struct Entry { BlockGraph *graph; BasicBlock *block; };
    std::unordered_map<uint64_t,Entry> entries_;
    std::deque<std::unique_ptr<BlockGraph>> graphs_;
    Memory *memory_=nullptr;
    FixedComponents *intrinsics_=nullptr;
    uint64_t intrinsic_generation_=0,memory_identity_=0,cache_generation_=1;
    ExecutionFrame frame_;
    memory_detail::AccessSynchronization::ReadGuard native_read_;
    uint64_t continuation_pc_=0,continuation_entry_=0;
    void erase_entries(const BlockGraph &graph);
    Entry find(uint64_t pc);
    const StaticOperation *select(uint64_t pc);
    const StaticOperation *tail(uint64_t pc);
    const StaticOperation *tail_from(const BasicBlock &block,uint64_t pc);
    const StaticOperation *link(const StaticOperation &source,uint64_t pc);
  public:
    ExecutionEngine()=default;
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
    ~ExecutionEngine();
    // Cumulative JSONL snapshot. OUTPUT chooses stderr (or "-") or a file.
    // Call between run() invocations, on the engine's owning thread.
    void dump_diagnostics(const char *phase="manual") const noexcept;
#else
    void dump_diagnostics(const char * = "manual") const noexcept {}
#endif
    ExecutionEngine(const ExecutionEngine &)=delete;
    ExecutionEngine &operator=(const ExecutionEngine &)=delete;
    ExecutionEngine(ExecutionEngine &&)=delete;
    ExecutionEngine &operator=(ExecutionEngine &&)=delete;
    WasmExecutor wasm;
    uint64_t executed_instructions=0,decoded_blocks=0,cache_hits=0;
    uint64_t compiled_operations=0,eliminated_values=0,fused_operations=0;
    uint64_t slow_calls=0,materializations=0;
    Execution run(CpuState &cpu,Memory &memory,uint64_t budget,FixedComponents *intrinsics=nullptr);
    void enter_native() { native_read_.lock(frame_.fast_first!=nullptr); }
    void leave_native() noexcept { native_read_.unlock(); }
    const StaticOperation *slow(const StaticOperation &op);
};
}
