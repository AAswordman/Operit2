#pragma once
#include "cpu/aarch64/wasm_region.h"
#include "cpu/aarch64/state.h"
#include <functional>
#include <memory>
#include <unordered_map>
namespace operit::arm64 {
class Memory;
class FixedComponents;
struct WasmTransfer {
    struct Page {unsigned slot=0,permissions=0;uint64_t address=0;std::vector<uint8_t> bytes;};
    std::vector<uint8_t> state;
    std::vector<Page> pages;
    std::vector<unsigned> invalidate;
    std::shared_ptr<const WasmRegion> module;
    uint64_t module_id=0,wasm_retired=0,dispatches=0,module_uploads=0,module_bytes=0;
    int reason=WasmRegion::Branch;
    bool stop=false;
};
bool wasm_webkit_available();
void wasm_webkit_forget(uint64_t session);
bool wasm_webkit_run(uint64_t session,WasmTransfer &transfer,const std::function<void(WasmTransfer&)> &service);
class WasmExecutor {
    struct Cached {
        uint64_t id=0;
        std::shared_ptr<const WasmRegion> region;
    };
    struct Resident {uint64_t address,revision;unsigned permissions;bool file;std::vector<uint8_t> bytes;};
    std::unordered_map<uint64_t,Cached> cache_;
    std::unordered_map<uint64_t,std::shared_ptr<const std::vector<uint32_t>>> code_;
    std::unordered_map<unsigned,Resident> resident_;
    uint64_t identity_=0,session_=0,epoch_=0;
    Cached compile(uint64_t pc,Memory &memory,const FixedComponents *intrinsics);
  public:
    ~WasmExecutor();
    void clear();
    uint64_t executed_instructions=0,calls=0,compiled_regions=0,compiled_instructions=0;
    uint64_t capability_exits=0,failed_calls=0,transport_ns=0;
    uint64_t native_instructions=0,dispatches=0,services=0,page_faults=0,code_misses=0;
    uint64_t compile_ns=0,assist_ns=0,commit_ns=0;
    std::unordered_map<uint32_t,uint64_t> assists;
    void record(const CpuState &cpu,uint64_t total,uint64_t completed,bool syscall) const;
    uint64_t module_uploads=0,module_bytes=0;
    unsigned largest_region=0,largest_block_count=0;
    uint32_t run(CpuState &cpu,Memory &memory,uint64_t budget,int &reason,const FixedComponents *intrinsics);
};
}
