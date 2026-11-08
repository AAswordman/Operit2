#include "cpu/aarch64/execution_diagnostics.h"
#include "cpu/aarch64/execution_frame.h"
#if OPERIT_EXECUTION_DIAGNOSTICS
#include "cpu/aarch64/block_graph.h"
#include <algorithm>
#include <fstream>
#include <iomanip>
#include <map>
#include <memory>
#include <sstream>
#include <unordered_map>
#include <utility>
#include <vector>
#endif

namespace operit::arm64 {
#if OPERIT_EXECUTION_DIAGNOSTICS
namespace {
struct InstructionSnapshot {
    uint64_t pc;
    uint32_t raw;
    unsigned op,encoding;
};
// Resolved scalar values only: neither the live sidecar nor its node pointer
// may be dereferenced after graph reclamation during a later JSON dump.
struct OperationSnapshot {
    uint64_t cursor,pc,entry,immediate,auxiliary,operands;
    uint16_t index;
    StaticRecipe recipe;
};
struct BlockSnapshot {
    uint64_t graph_pc,pc;
    std::array<uint8_t,7> gp;
    std::array<uint8_t,8> vectors;
    std::vector<InstructionSnapshot> instructions;
    std::vector<OperationSnapshot> operations;
};
struct Location { size_t block,operation; };
constexpr size_t snapshot_operation_limit=1u<<20;
uint64_t pointer_value(const void *pointer) {return reinterpret_cast<uintptr_t>(pointer);}
uint64_t operands(const StaticOperation &op,uint16_t prefix) {
    return uint64_t(op.a)|(uint64_t(op.b)<<16)|(uint64_t(op.d)<<32)|(uint64_t(prefix)<<48);
}
std::string json_hex(uint64_t value,unsigned width=0) {
    std::ostringstream text;text<<"\"0x"<<std::hex<<std::setfill('0')<<std::setw(width)<<value<<'"';return text.str();
}
const char *handler(uint64_t auxiliary) {return execution_diagnostic_static_code_name(uint8_t(auxiliary>>56));}
template<class T,size_t N> void array_json(std::ostream &out,const std::array<T,N> &values) {
    out<<'[';for(size_t i=0;i<N;++i){if(i)out<<',';out<<uint64_t(values[i]);}out<<']';
}
void operation_json(std::ostream &out,const OperationSnapshot &op) {
    out<<"{\"cursor\":"<<json_hex(op.cursor)<<",\"pc\":"<<json_hex(op.pc)<<",\"handler\":\""<<handler(op.auxiliary)
       <<"\",\"entry\":"<<json_hex(op.entry)<<",\"prefix\":"<<uint16_t(op.operands>>48)
       <<",\"a\":"<<uint16_t(op.operands)<<",\"b\":"<<uint16_t(op.operands>>16)<<",\"d\":"<<uint16_t(op.operands>>32)
       <<",\"index\":"<<op.index<<",\"immediate\":"<<json_hex(op.immediate)<<",\"auxiliary\":"<<json_hex(op.auxiliary);
    if(op.recipe.valid) {
        out<<",\"recipe\":{\"handler\":\""<<execution_diagnostic_static_code_name(uint8_t(op.recipe.code))
           <<"\",\"bits\":"<<op.recipe.bits<<",\"d\":"<<unsigned(op.recipe.d)<<",\"a\":"<<unsigned(op.recipe.a)
           <<",\"b\":"<<unsigned(op.recipe.b)<<",\"flags\":"<<unsigned(op.recipe.flags)<<'}';
    }
    out<<'}';
}
void block_json(std::ostream &out,const BlockSnapshot &block,size_t operation_index) {
    out<<"{\"graph_pc\":"<<json_hex(block.graph_pc)<<",\"block_pc\":"<<json_hex(block.pc)
       <<",\"operation_index\":"<<operation_index<<",\"guest_instruction_count\":"<<block.instructions.size()
       <<",\"gp_roles\":";array_json(out,block.gp);out<<",\"vector_roles\":";array_json(out,block.vectors);
    out<<",\"instructions\":[";
    for(size_t i=0;i<block.instructions.size();++i) {
        if(i)out<<',';
        const auto &instruction=block.instructions[i];
        out<<"{\"pc\":"<<json_hex(instruction.pc)<<",\"raw\":"<<json_hex(instruction.raw,8)
           <<",\"op\":"<<instruction.op<<",\"encoding\":"<<instruction.encoding<<'}';
    }
    out<<"],\"lowered_sequence\":[";
    for(size_t i=0;i<block.operations.size();++i) {
        if(i)out<<',';
        operation_json(out,block.operations[i]);
    }
    out<<"]}";
}
bool matches(const ExecutionDiagnosticSite &site,const OperationSnapshot &snapshot) {
    return site.cursor==snapshot.cursor && site.pc==snapshot.pc && site.entry==snapshot.entry &&
        site.immediate==snapshot.immediate && site.auxiliary==snapshot.auxiliary && site.operands==snapshot.operands;
}
constexpr const char *event_names[]={
    "block_admitted","slow_entry","memory_read_attempt","memory_write_attempt",
    "memory_read_cross_page","memory_write_cross_page","page_cache_read_hit","page_cache_write_hit",
    "translation_read_fallback","translation_write_fallback","fallback_way0_hit","fallback_way1_hit",
    "file_backed_probe","file_validated_hit","file_observation_hit","file_lock_attempt",
    "file_lock_acquired","file_lock_blocked","local_edge","cross_edge_attempt",
    "fallthrough_local","fallthrough_cross","bridge_entry","bridge_transferred",
    "indirect_edge","indirect_cache_entry","indirect_cache_hit","next_elided",
    "vector_selected_resident_read","vector_selected_spill_read","gp_bridge_spill","gp_bridge_reload",
    "vector_bridge_spill","vector_bridge_reload",
    "lazy_fp_enter","bridge_validate_call","active_fp_host_switch","bridge_fp_guest_restore",
    "reserved38","native_proof_attempt","native_proof_success","profile_superblock_entry","guest_aot_entry",
    "guest_aot_indirect","guest_aot_trace","guest_aot_trace_fallthrough","guest_aot_trace_natural",
    "guest_aot_scheduled_instructions","guest_aot_register_memory_attempt","guest_aot_register_memory_success",
    "guest_aot_long_trace_entry","guest_aot_long_trace_edge"
};
static_assert(sizeof(event_names)/sizeof(event_names[0])==ED_EVENT_GUEST_AOT_LONG_TRACE_EDGE+1);
static_assert(sizeof(event_names)/sizeof(event_names[0])<=ED_EVENT_CAPACITY);
constexpr const char *binding_names[]={"gp_read_resident","gp_read_spill","gp_write_resident","gp_write_spill",
    "vector_read_resident","vector_read_spill","vector_write_resident","vector_write_spill"};
void bindings_json(std::ostream &out,const std::array<uint64_t,8> &bindings) {
    out<<'{';for(size_t i=0;i<bindings.size();++i){if(i)out<<',';out<<'"'<<binding_names[i]<<"\":"<<bindings[i];}out<<'}';
}
void branches_json(std::ostream &out,const std::array<uint64_t,32> &entries,const std::array<uint64_t,32> &taken) {
    constexpr const char *conditions[]={"eq","ne","hs","lo","mi","pl","vs","vc","hi","ls","ge","lt","gt","le","al","nv"};
    out<<'[';bool comma=false;
    for(size_t i=0;i<entries.size();++i)if(entries[i] || taken[i]) {
        if(comma)out<<',';
        comma=true;
        out<<"{\"condition\":\""<<conditions[i&15]<<"\",\"condition_code\":"<<(i&15)
           <<",\"local_taken_target\":"<<(i>=16 ? "true":"false")<<",\"entries\":"<<entries[i]
           <<",\"taken\":"<<taken[i]<<",\"not_taken\":"<<(entries[i]-taken[i])<<'}';
    }
    out<<']';
}
}

struct ExecutionDiagnosticsStorage {
    std::vector<ExecutionDiagnosticSite> sites;
    std::vector<BlockSnapshot> blocks;
    std::unordered_map<uint64_t,std::vector<Location>> locations;
    std::unordered_map<const uint64_t*,std::weak_ptr<uint64_t>> observed;
    size_t operation_count=0;
    uint64_t metadata_dropped_graphs=0,metadata_failures=0;
    explicit ExecutionDiagnosticsStorage(size_t sets):sites(sets*ED_SITE_WAYS) {}
};
ExecutionDiagnostics::~ExecutionDiagnostics() {delete storage;}

bool execution_diagnostics_reset(ExecutionFrame &frame,unsigned sample_shift,size_t site_sets) noexcept {
    if(sample_shift>30 || !site_sets || site_sets>(1u<<20) || (site_sets&(site_sets-1)))return false;
    try {
        auto fresh=std::make_unique<ExecutionDiagnosticsStorage>(site_sets);
        auto &d=frame.diagnostics;
        delete d.storage;d.storage=fresh.release();d.sites=d.storage->sites.data();
        d.handlers.fill(0);d.profiles.fill(0);d.events.fill(0);
        d.branch_conditions.fill(0);d.branch_taken.fill(0);
        d.compare_branch_conditions.fill(0);d.compare_branch_taken.fill(0);
        d.sample_interval_mask=(uint64_t(1)<<(sample_shift+1))-1;
        d.random_state=0x9e3779b97f4a7c15ULL;
        d.random_state^=d.random_state<<13;d.random_state^=d.random_state>>7;d.random_state^=d.random_state<<17;
        d.sample_remaining=(d.random_state&d.sample_interval_mask)+1;
        d.site_set_mask=site_sets-1;d.samples=0;d.collisions=0;d.dropped=0;
        return true;
    } catch(...) {return false;}
}
void execution_diagnostics_observe_graph(ExecutionFrame &frame,const BlockGraph &graph) noexcept {
    auto *storage=frame.diagnostics.storage;if(!storage)return;
    try {
        const auto found=storage->observed.find(graph.lifetime.get());
        if(found!=storage->observed.end() && found->second.lock()==graph.lifetime)return;
        size_t operations=0;for(const auto &block:graph.blocks)operations+=block->code.size();
        if(operations>snapshot_operation_limit-storage->operation_count) {
            ++storage->metadata_dropped_graphs;return;
        }
        std::vector<BlockSnapshot> pending;pending.reserve(graph.blocks.size());
        for(const auto &block:graph.blocks) {
            BlockSnapshot snapshot{graph.address,block->pc,resident_gp,resident_vector,{},{}};
            snapshot.instructions.reserve(block->instructions.size());snapshot.operations.reserve(block->code.size());
            for(const auto &node:block->instructions)snapshot.instructions.push_back(
                {node.pc,node.instruction.raw,unsigned(node.instruction.op),unsigned(node.instruction.encoding)});
            for(size_t i=0;i<block->code.size();++i) {
                const auto &op=block->code[i];
                const auto info=operation_info(op);
                snapshot.operations.push_back({pointer_value(&op),info.pc,pointer_value(op.entry),op.immediate,
                                               info.auxiliary,operands(op,info.prefix),op.index,
                                               i<block->recipes.size() ? block->recipes[i]:StaticRecipe{}});
            }
            pending.push_back(std::move(snapshot));
        }
        storage->blocks.reserve(storage->blocks.size()+pending.size());
        for(auto &block:pending) {
            const auto index=storage->blocks.size();storage->blocks.push_back(std::move(block));
            storage->operation_count+=storage->blocks.back().operations.size();
            for(size_t op=0;op<storage->blocks.back().operations.size();++op)
                storage->locations[storage->blocks.back().operations[op].cursor].push_back({index,op});
        }
        storage->observed[graph.lifetime.get()]=graph.lifetime;
    } catch(...) {++storage->metadata_failures;}
}
void execution_diagnostics_count(ExecutionFrame &frame,ExecutionDiagnosticEvent event,uint64_t amount) noexcept {
    const auto index=unsigned(event);if(index<frame.diagnostics.events.size())frame.diagnostics.events[index]+=amount;
}

bool execution_diagnostics_write_json(const ExecutionFrame &frame,const char *path,size_t top) noexcept {
    if(!path || !*path)return false;
    try {
        const auto &d=frame.diagnostics;const auto *storage=d.storage;
        std::ofstream out(path,std::ios::out|std::ios::trunc);if(!out)return false;
        uint64_t entries=0;for(auto count:d.handlers)entries+=count;
        out<<"{\"schema\":\"operit.execution-diagnostics.v1\",\"enabled\":true,\"handler_entries\":"<<entries
           <<",\"notes\":[\"Handlers and path events are exact invocation counts, not time samples.\","
             "\"Cursor samples use random uniform intervals; no periodic stride or extrapolated coverage.\","
             "\"Register totals count selected operand bindings; conditional or faulting handlers can skip accesses.\","
             "\"A Block sample represents a header attempt, including an epoch or budget rejection.\","
             "\"Top PC totals include administrative handlers; inspect per-site handler and raw sequence.\","
             "\"Graph snapshots own their data; indirect and bridge records may have no original block mapping.\"],"
             "\"sampler\":{\"interval_min\":1,\"interval_max\":"<<(d.sample_interval_mask+1)
           <<",\"interval_mean\":"<<(double(d.sample_interval_mask+2)/2)
           <<",\"site_sets\":"<<(d.sites ? d.site_set_mask+1:0)<<",\"ways\":"<<ED_SITE_WAYS
           <<",\"samples\":"<<d.samples<<",\"colliding_probes\":"<<d.collisions<<",\"dropped_samples\":"<<d.dropped
           <<",\"retained_samples\":"<<(d.samples-d.dropped)<<"},\"handlers\":{";
        bool comma=false;
        for(size_t i=0;i<d.handlers.size();++i)if(d.handlers[i]) {
            if(comma)out<<',';
            comma=true;out<<'"'<<execution_diagnostic_static_code_name(uint8_t(i))<<"\":"<<d.handlers[i];
        }
        out<<"},\"events\":{";
        for(size_t i=0;i<sizeof(event_names)/sizeof(event_names[0]);++i) {
            if(i)out<<',';
            out<<'"'<<event_names[i]<<"\":"<<d.events[i];
        }
        out<<"},\"branch_conditions\":";branches_json(out,d.branch_conditions,d.branch_taken);
        out<<",\"compare_immediate_branch_conditions\":";branches_json(out,d.compare_branch_conditions,d.compare_branch_taken);
        size_t profile_count=0;const auto *profiles=execution_diagnostic_profiles(profile_count);
        std::array<uint64_t,8> totals{};out<<",\"register_profiles\":[";comma=false;
        for(size_t i=0;i<profile_count;++i)if(d.profiles[i]) {
            if(comma)out<<',';
            comma=true;std::array<uint64_t,8> bindings{};
            for(size_t j=0;j<bindings.size();++j){bindings[j]=profiles[i].bindings[j];totals[j]+=bindings[j]*d.profiles[i];}
            out<<"{\"id\":"<<i<<",\"entries\":"<<d.profiles[i]<<",\"flags\":"<<unsigned(profiles[i].flags)<<",\"bindings\":";
            bindings_json(out,bindings);out<<'}';
        }
        out<<"],\"register_operand_bindings\":";bindings_json(out,totals);
        std::vector<const ExecutionDiagnosticSite*> sites;std::map<uint64_t,uint64_t> pc_samples;
        if(storage)for(const auto &site:storage->sites)if(site.cursor && site.samples) {
            sites.push_back(&site);pc_samples[site.pc]+=site.samples;
        }
        std::sort(sites.begin(),sites.end(),[](const auto *a,const auto *b){return a->samples!=b->samples ? a->samples>b->samples:a->cursor<b->cursor;});
        std::vector<std::pair<uint64_t,uint64_t>> pcs(pc_samples.begin(),pc_samples.end());
        std::sort(pcs.begin(),pcs.end(),[](const auto &a,const auto &b){return a.second!=b.second ? a.second>b.second:a.first<b.first;});
        out<<",\"metadata\":{\"blocks\":"<<(storage ? storage->blocks.size():0)
           <<",\"operations\":"<<(storage ? storage->operation_count:0)
           <<",\"record_bytes\":"<<OP_SIZE<<",\"cold_record_bytes\":"<<OM_SIZE<<",\"operation_limit\":"<<snapshot_operation_limit
           <<",\"dropped_graphs\":"<<(storage ? storage->metadata_dropped_graphs:0)
           <<",\"failures\":"<<(storage ? storage->metadata_failures:0)<<"},\"top_guest_pcs\":[";
        for(size_t i=0;i<pcs.size() && (!top || i<top);++i) {
            if(i)out<<',';
            out<<"{\"pc\":"<<json_hex(pcs[i].first)<<",\"samples\":"<<pcs[i].second<<'}';
        }
        out<<"],\"retained_sites\":"<<sites.size()<<",\"top_sites\":[";
        for(size_t i=0;i<sites.size() && (!top || i<top);++i) {
            if(i)out<<',';
            const auto &site=*sites[i];
            out<<"{\"cursor\":"<<json_hex(site.cursor)<<",\"pc\":"<<json_hex(site.pc)<<",\"entry\":"<<json_hex(site.entry)
               <<",\"handler\":\""<<handler(site.auxiliary)<<"\",\"samples\":"<<site.samples<<",\"profile\":"<<site.profile
               <<",\"immediate\":"<<json_hex(site.immediate)<<",\"auxiliary\":"<<json_hex(site.auxiliary)
               <<",\"a\":"<<uint16_t(site.operands)<<",\"b\":"<<uint16_t(site.operands>>16)
               <<",\"d\":"<<uint16_t(site.operands>>32)<<",\"prefix\":"<<uint16_t(site.operands>>48)<<",\"mappings\":[";
            size_t match_count=0;
            const auto found=storage->locations.find(site.cursor);
            if(found!=storage->locations.end())for(const auto &location:found->second) {
                const auto &block=storage->blocks[location.block];
                if(!matches(site,block.operations[location.operation]))continue;
                if(match_count<4){if(match_count)out<<',';block_json(out,block,location.operation);}
                ++match_count;
            }
            out<<"],\"mapping_matches\":"<<match_count<<'}';
        }
        out<<"]}\n";out.flush();return bool(out);
    } catch(...) {return false;}
}
#else
bool execution_diagnostics_reset(ExecutionFrame &,unsigned,size_t) noexcept {return true;}
void execution_diagnostics_observe_graph(ExecutionFrame &,const BlockGraph &) noexcept {}
void execution_diagnostics_count(ExecutionFrame &,ExecutionDiagnosticEvent,uint64_t) noexcept {}
bool execution_diagnostics_write_json(const ExecutionFrame &,const char *,size_t) noexcept {return false;}
#endif
}
