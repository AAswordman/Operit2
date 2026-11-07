#pragma once
#include "cpu/aarch64/decoder.h"
#include <vector>
namespace operit::arm64 {
// Lower the shared optimized instruction graph to baseline Wasm. Regions import
// the same memory; guest addresses are 64 bit, linear-memory offsets are 32 bit.
struct WasmRegion {
    enum Exit { Budget=0, Branch=1, Capability=2, Syscall=3, Assist=4 };
    static constexpr unsigned state_size=1024, pc_offset=256, flags_offset=264;
    static constexpr unsigned retired_offset=268, remaining_offset=272, proofs_offset=276;
    static constexpr unsigned tls_offset=304, entry_offset=320, fault_offset=328;
    static constexpr unsigned perms_offset=336, size_offset=340, vector_offset=512;
    static constexpr unsigned page_table=65536, page_slots=65536, code_table=page_table+page_slots*16;
    static constexpr unsigned dirty_queue=code_table+page_slots*16, data_offset=dirty_queue+page_slots*4, dirty_count=416;
    struct Compiler;
    std::vector<uint8_t> bytes;
    std::vector<std::pair<uint64_t,unsigned>> entries;
    unsigned instruction_count=0, block_count=0;
    unsigned eliminated_instructions=0,constant_results=0,discarded_flags=0;
    static unsigned page_slot(uint64_t address) {return ((address>>12)^(address>>28))&(page_slots-1);}
    static std::vector<uint8_t> dispatcher();
    static bool supports(const Instruction &instruction);
    WasmRegion(const std::vector<std::pair<uint64_t,uint32_t>> &words,uint64_t entry);
    explicit WasmRegion(const std::vector<uint32_t> &words, uint64_t entry=0x10000);
    const std::vector<uint8_t> &module() const { return bytes; }
};
}
