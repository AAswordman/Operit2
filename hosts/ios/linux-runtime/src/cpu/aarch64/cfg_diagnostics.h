#pragma once
// Build every consumer of ExecutionEngine/BlockGraph with the same value.
// With the flag absent or 0, all counters, clocks, tables and hooks disappear.
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
#define OPERIT_EXEC_DIAGNOSTIC(...) __VA_ARGS__
#include "cpu/aarch64/decoder.h"
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <map>
#include <mutex>
#include <string>
#include <tuple>
#include <vector>
namespace operit::arm64::execution_diagnostics {
inline constexpr const char *op_names[]={
    "Unknown","Hint","Svc","Branch","BranchRegister","ConditionalBranch",
    "CompareBranch","TestBranch","Address","MoveWide","AddImmediate","AddRegister",
    "AddCarry","LogicalRegister","LogicalImmediate","Bitfield","Extract","ConditionalSelect",
    "ConditionalCompare","TwoSource","OneSource","Multiply","MultiplyHigh","LoadStore",
    "LoadLiteral","LoadPair","System","Cache","Simd","VectorMemory",
    "Exclusive","Barrier","Fault"
};
inline constexpr const char *encoding_names[]={
    "General","Noop","Syscall","Branch","BranchLink","BranchRegister",
    "BranchLinkRegister","ConditionalBranch","CompareBranch32","CompareBranch64","TestBranch","Address",
    "Move32","Move64","Keep32","Keep64","AddImmediate32","AddImmediate64",
    "AddRegister32","AddRegister64","CompareRegister64","AddCarry32","AddCarry64","LogicalImmediate32",
    "LogicalImmediate64","LogicalRegister32","LogicalRegister64","Bitfield32","Bitfield64","Extract32",
    "Extract64","Select32","Select64","Compare32","Compare64","TwoSource32",
    "TwoSource64","OneSource32","OneSource64","Multiply32","Multiply64","MultiplyHigh",
    "Load8","Load16","Load32","Load64","Store8","Store16",
    "Store32","Store64","LoadPair32","LoadPair64","StorePair32","StorePair64",
    "LoadLiteral32","LoadLiteral64","ReadTls","WriteTls","ReadFlags","WriteFlags",
    "VectorLoad32","VectorLoad64","VectorLoad128","VectorStore32","VectorStore64","VectorStore128",
    "VectorLoadPair32","VectorLoadPair64","VectorLoadPair128","VectorStorePair32","VectorStorePair64","VectorStorePair128",
    "VectorToRegister32","VectorToRegister64","RegisterToVector32","RegisterToVector64","VectorHighToRegister","RegisterToVectorHigh",
    "VectorImmediate","VectorLogical","ScalarFma64","ScalarSqrt64","ScalarUnsignedToDouble64","ScalarAdd64",
    "ScalarCompare64","ScalarSelect64","VectorFma2D","ScalarMultiply64","ScalarDivide64","ScalarSubtract64",
    "ScalarUnary32","ScalarUnary64","ScalarSignedToDouble64",
    "ScalarUnsignedToDouble32","ScalarSignedToDouble32","ScalarDoubleToSigned32","ScalarDoubleToSigned64","LoadAcquire"
};
inline constexpr const char *static_code_names[]={
    "Block","Bridge","IndirectDispatch","Next","Helper","Nop",
    "Move","Keep","Add","Sub","Adc","Sbc",
    "And","Or","Xor","Lsl","Lsr","Asr",
    "Ror","Mul","Udiv","Sdiv","Select","Operand",
    "Prepare","AddPrepared","SubPrepared","AndPrepared","OrPrepared","XorPrepared",
    "Madd","Msub","SmulHigh","UmulHigh","Bitfield","Extract",
    "Unary","Load","Store","SignedLoad","VectorLoad","VectorStore",
    "PairLoad","PairStore","VectorPairLoad","VectorPairStore","LoadPrepared","StorePrepared",
    "SignedLoadPrepared","VectorLoadPrepared","VectorStorePrepared","Fadd","Fsub","Fmul",
    "Fdiv","Fsqrt","Fma","Fcmp","Fselect","GpToVector",
    "VectorToGp","VectorImmediate","UnsignedToDouble","Jump","Call","Indirect",
    "IndirectCall","Branch","CompareBranch","TestBranch","CompareImmediateBranch","ReadTls",
    "WriteTls","ReadFlags","WriteFlags","ConditionalCompare","LoadWriteback","StoreWriteback",
    "SignedLoadWriteback","VectorLoadWriteback","VectorStoreWriteback","FUnary","BlockBody","ScheduledLoads","SignedToDouble","DoubleToSigned",
    "SmaddLong","UmaddLong","SmsubLong","UmsubLong","VectorLogical","LoadAcquire"
};

inline const char *name(const char *const *names,size_t size,unsigned value) {
    return value<size ? names[value]:"out_of_range";
}
inline const char *helper_reason(const Instruction &i) {
    if(i.op==Op::Svc)return "helper_syscall";
    if(i.op==Op::Unknown || i.op==Op::Fault)return "helper_fault_or_unknown";
    return i.encoding==Encoding::General ? "helper_general_encoding":"helper_unsupported_static";
}
inline bool clock_enabled() noexcept {
    const auto *value=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_CLOCK");
    return value && *value && std::strcmp(value,"0");
}
inline bool auto_dump_enabled() noexcept {
    const auto *value=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_OUTPUT");
    const auto *native=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_NATIVE_OUTPUT");
    return (value && *value) || (native && *native) || clock_enabled();
}
// Limit each potentially large detail table; aggregate counters stay exact.
// Explicit zero retains the complete table for one-off audits.
inline size_t output_top() noexcept {
    const auto *text=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_TOP");
    if(!text || !*text || *text=='-')return 256;
    char *end=nullptr;const auto value=std::strtoull(text,&end,10);
    return end && !*end && value<=(1u<<20) ? size_t(value):256;
}
template<class Table,class Rank>
auto ranked_rows(const Table &table,size_t limit,Rank rank) {
    std::vector<const typename Table::value_type*> rows;
    rows.reserve(table.size());
    for(const auto &row:table)rows.push_back(&row);
    auto better=[&](const auto *a,const auto *b) {
        const auto ar=rank(a->second),br=rank(b->second);
        return ar!=br ? ar>br:a->first<b->first;
    };
    const auto count=limit ? std::min(limit,rows.size()):rows.size();
    std::partial_sort(rows.begin(),rows.begin()+count,rows.end(),better);
    rows.resize(count);return rows;
}
struct PcCounts {
    uint64_t graph_builds=0,block_compiles=0,instruction_compiles=0,slow_calls=0;
    uint64_t invalidations=0,evictions=0,entry_collisions=0,build_ns=0;
};
struct InstructionCounts {
    uint64_t compiled=0,calls=0,retired=0,faults=0;
    void add(const InstructionCounts &other) {
        compiled+=other.compiled;calls+=other.calls;retired+=other.retired;faults+=other.faults;
    }
};
struct InstructionFaultScope {
    InstructionCounts &counts;int before;
    explicit InstructionFaultScope(InstructionCounts &counts):counts(counts),before(std::uncaught_exceptions()) {}
    ~InstructionFaultScope() {if(std::uncaught_exceptions()>before)++counts.faults;}
};
// kind, guest PC, original raw word, original Op, original Encoding, StaticCode.
using InstructionKey=std::tuple<std::string,uint64_t,uint32_t,unsigned,unsigned,unsigned>;
// source PC, destination PC, cross-graph, indirect, changed GP/vector role masks.
using LinkKey=std::tuple<uint64_t,uint64_t,bool,bool,unsigned,unsigned>;
// site, full-mask request, selected resident GP count, selected vector count.
using TransferKey=std::tuple<std::string,bool,unsigned,unsigned>;
struct State {
    std::map<std::string,uint64_t> counters;
    std::map<uint64_t,PcCounts> pcs;
    std::map<InstructionKey,InstructionCounts> instructions;
    std::map<LinkKey,uint64_t> links;
    std::map<TransferKey,uint64_t> transfers;
    const char *validation_site="unspecified";
    bool native_initialized=false,native_sampling=false;
    bool clock_pending=false;
    const void *clock_cpu=nullptr;
    uint64_t clock_sequence=0,clock_pc=0,clock_arg0=0,clock_arg1=0,clock_result=0;
    uint64_t guest_pc=0,last_syscall=0,last_budget=0;
    mutable uint64_t snapshot=0;
    void count(const std::string &key,uint64_t amount=1) {counters[key]+=amount;}
    uint64_t get(const char *key) const {
        const auto found=counters.find(key);return found==counters.end() ? 0:found->second;
    }
    void validation(const char *reason,uint64_t amount=1) {
        count(std::string("validate.")+validation_site+"."+reason,amount);
    }
    InstructionCounts &instruction(const char *kind,uint64_t pc,const Instruction &i,unsigned code) {
        return instructions[{kind,pc,i.raw,unsigned(i.op),unsigned(i.encoding),code}];
    }
    template<class G,class V>
    void transfer(const char *site,const G &gp,const V &vector,uint32_t gm=UINT32_MAX,uint32_t vm=UINT32_MAX) {
        unsigned g=0,v=0;
        for(auto r:gp)g+=bool(gm&(uint32_t{1}<<r));
        for(auto r:vector)v+=bool(vm&(uint32_t{1}<<r));
        ++transfers[{site,gm==UINT32_MAX && vm==UINT32_MAX,g,v}];
    }
    // One complete, cumulative JSON object per line. No dependencies beyond
    // the C++ standard library. Explicit dumps default to stderr; automatic
    // destruction dumps require OUTPUT or CLOCK. File output is append-only.
    void dump(const void *engine,const char *phase,uint64_t retired,uint64_t cache_hits,
              uint64_t slow_calls,uint64_t materializations,size_t graph_count,size_t entry_count,const char *native_path=nullptr,bool native_written=false) const noexcept {
        try {
            static std::mutex mutex;
            const std::lock_guard<std::mutex> guard(mutex);
            const auto *path=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_OUTPUT");
            const bool file=path && *path && std::strcmp(path,"stderr") && std::strcmp(path,"-");
            // Build derived histograms before writing so allocation failure
            // cannot leave half a JSON line in a shared diagnostic stream.
            using RawKey=std::tuple<std::string,uint32_t,unsigned,unsigned,unsigned>;
            std::map<RawKey,InstructionCounts> raw;
            for(const auto &[key,value]:instructions)
                raw[{std::get<0>(key),std::get<2>(key),std::get<3>(key),std::get<4>(key),std::get<5>(key)}].add(value);
            const auto top=output_top();
            const auto pc_rows=ranked_rows(pcs,top,[](const PcCounts &v) {
                return std::make_tuple(v.slow_calls,v.graph_builds,v.invalidations,v.evictions,v.instruction_compiles);
            });
            auto instruction_rank=[](const InstructionCounts &v) {return std::make_tuple(v.calls,v.retired,v.compiled,v.faults);};
            const auto instruction_rows=ranked_rows(instructions,top,instruction_rank);
            const auto raw_rows=ranked_rows(raw,top,instruction_rank);
            const auto link_rows=ranked_rows(links,top,[](uint64_t v){return v;});
            InstructionCounts instruction_total;
            for(const auto &[key,value]:instructions)instruction_total.add(value);
            FILE *out=file ? std::fopen(path,"a"):stderr;
            if(!out) {
                std::fputs("{\"schema\":\"operit.execution.v1\",\"error\":\"output_open_failed\"}\n",stderr);
                return;
            }
            auto string=[&](const char *text) {
                std::fputc('"',out);
                for(const unsigned char *p=reinterpret_cast<const unsigned char*>(text ? text:"");*p;++p) {
                    if(*p=='"' || *p=='\\')std::fputc('\\',out);
                    if(*p<32)std::fprintf(out,"\\u%04x",unsigned(*p));else std::fputc(*p,out);
                }
                std::fputc('"',out);
            };
            auto number=[&](const char *key,uint64_t value) {
                string(key);std::fprintf(out,":%llu",static_cast<unsigned long long>(value));
            };
            auto hex=[&](const char *key,uint64_t value) {
                string(key);std::fprintf(out,":\"0x%llx\"",static_cast<unsigned long long>(value));
            };
            auto comma=[&] {std::fputc(',',out);};
            auto instruction_counts=[&](const InstructionCounts &value) {
                number("compiled",value.compiled);comma();number("calls",value.calls);comma();
                number("retired",value.retired);comma();number("faults",value.faults);
            };
            auto instruction_identity=[&](const std::string &kind,uint32_t word,unsigned op,unsigned encoding,unsigned code) {
                std::fputs("\"kind\":",out);string(kind.c_str());comma();hex("raw",word);comma();
                number("op_id",op);std::fputs(",\"op\":",out);string(name(op_names,sizeof(op_names)/sizeof(*op_names),op));comma();
                number("encoding_id",encoding);std::fputs(",\"encoding\":",out);string(name(encoding_names,sizeof(encoding_names)/sizeof(*encoding_names),encoding));comma();
                number("static_code_id",code);std::fputs(",\"static_code\":",out);string(name(static_code_names,sizeof(static_code_names)/sizeof(*static_code_names),code));comma();
            };
            std::fputs("{\"schema\":\"operit.execution.v1\",\"engine\":",out);
            char id[40];std::snprintf(id,sizeof(id),"%p",engine);string(id);comma();number("snapshot",++snapshot);
            std::fputs(",\"phase\":",out);string(phase);
            std::fputs(",\"cumulative\":true,\"native_output\":",out);string(native_path);
            comma();number("native_output_written",native_written);
            std::fputs(",\"coverage\":{\"native_block_entries\":true,\"native_fast_bridge_traversals\":false,\"native_fast_pic_hits\":false},\"totals\":{",out);
            number("retired_instructions",retired);comma();number("cache_hits_including_find",cache_hits);comma();
            number("slow_calls",slow_calls);comma();number("legacy_materializations",materializations);comma();
            number("live_graphs",graph_count);comma();number("live_entries",entry_count);comma();
            number("native_block_entries",get("native.block_entries"));comma();
            number("budget_suffix_retired",get("suffix.budget_retired"));comma();
            number("suffix_retired",get("suffix.retired"));comma();
            const double budget_fraction=retired ? double(get("suffix.budget_retired"))/double(retired):0;
            std::fprintf(out,"\"budget_suffix_retired_fraction\":%.17g}",budget_fraction);
            std::fputs(",\"detail_tables\":{",out);number("top",top);
            auto table_rows=[&](const char *key,size_t total,size_t written) {
                comma();string(key);std::fputs(":{",out);number("total_rows",total);comma();
                number("written_rows",written);comma();number("truncated",written<total);std::fputc('}',out);
            };
            table_rows("pc",pcs.size(),pc_rows.size());
            table_rows("instructions",instructions.size(),instruction_rows.size());
            table_rows("raw",raw.size(),raw_rows.size());table_rows("links",links.size(),link_rows.size());
            std::fputs(",\"instruction_totals\":{",out);instruction_counts(instruction_total);std::fputs("}}",out);
            std::fputs(",\"guest\":{",out);hex("pc",guest_pc);comma();number("last_syscall",last_syscall);comma();number("last_budget",last_budget);
            std::fputs("},\"clock\":{",out);number("sequence",clock_sequence);comma();hex("pc",clock_pc);comma();
            number("original_x8",113);comma();number("original_x0",clock_arg0);comma();hex("original_x1",clock_arg1);comma();
            number("resume_x0",clock_result);std::fputs("},\"counters\":{",out);
            bool first=true;
            for(const auto &[key,value]:counters) {if(!first)comma();first=false;number(key.c_str(),value);}
            std::fputs("},\"pc\":[",out);first=true;
            for(const auto *row:pc_rows) {
                const auto &[pc,value]=*row;
                if(!first)comma();first=false;std::fputc('{',out);hex("pc",pc);comma();
                number("graph_builds",value.graph_builds);comma();number("block_compiles",value.block_compiles);comma();
                number("instruction_compiles",value.instruction_compiles);comma();number("slow_calls",value.slow_calls);comma();
                number("invalidations",value.invalidations);comma();number("evictions",value.evictions);comma();
                number("entry_collisions",value.entry_collisions);comma();number("build_ns",value.build_ns);std::fputc('}',out);
            }
            std::fputs("],\"instructions\":[",out);first=true;
            for(const auto *row:instruction_rows) {
                const auto &[key,value]=*row;
                if(!first)comma();first=false;std::fputc('{',out);hex("pc",std::get<1>(key));comma();
                instruction_identity(std::get<0>(key),std::get<2>(key),std::get<3>(key),std::get<4>(key),std::get<5>(key));
                instruction_counts(value);std::fputc('}',out);
            }
            std::fputs("],\"raw\":[",out);first=true;
            for(const auto *row:raw_rows) {
                const auto &[key,value]=*row;
                if(!first)comma();first=false;std::fputc('{',out);
                instruction_identity(std::get<0>(key),std::get<1>(key),std::get<2>(key),std::get<3>(key),std::get<4>(key));
                instruction_counts(value);std::fputc('}',out);
            }
            std::fputs("],\"links\":[",out);first=true;
            for(const auto *row:link_rows) {
                const auto &[key,value]=*row;
                if(!first)comma();first=false;std::fputc('{',out);hex("source_pc",std::get<0>(key));comma();hex("target_pc",std::get<1>(key));comma();
                number("cross_graph",std::get<2>(key));comma();number("indirect",std::get<3>(key));comma();
                number("gp_changed_mask",std::get<4>(key));comma();number("vector_changed_mask",std::get<5>(key));comma();
                number("installations",value);std::fputc('}',out);
            }
            std::fputs("],\"transfers\":[",out);first=true;
            for(const auto &[key,value]:transfers) {
                if(!first)comma();first=false;std::fputs("{\"site\":",out);string(std::get<0>(key).c_str());comma();
                number("full_mask",std::get<1>(key));comma();number("resident_gp",std::get<2>(key));comma();
                number("resident_vector",std::get<3>(key));comma();number("calls",value);comma();
                number("register_bytes",value*(std::get<2>(key)*8+std::get<3>(key)*16));
                // publish/reload also transfer NZCV, even for empty masks.
                comma();number("nzcv_transfers",value);std::fputc('}',out);
            }
            std::fputs("]}\n",out);std::fflush(out);if(file)std::fclose(out);
        } catch(...) {} // Diagnostics must never turn a guest exit into a fault.
    }
};
inline thread_local State *active=nullptr;
struct ActiveScope {
    State *previous;
    explicit ActiveScope(State &state):previous(active) {active=&state;}
    ~ActiveScope() {active=previous;}
};
struct ValidationScope {
    State *state;const char *previous;
    ValidationScope(State *state,const char *site):state(state),previous(state ? state->validation_site:nullptr) {
        if(state)state->validation_site=site;
    }
    ~ValidationScope() {if(state)state->validation_site=previous;}
};
struct ExceptionCounter {
    State *state;const char *key;int before;
    ExceptionCounter(State *state,const char *key):state(state),key(key),before(std::uncaught_exceptions()) {}
    ~ExceptionCounter() {if(state && std::uncaught_exceptions()>before)state->count(key);}
};
struct Timer {
    State *state;const char *key;uint64_t pc;
    std::chrono::steady_clock::time_point start;
    Timer(State *state,const char *key,uint64_t pc=0):state(state),key(key),pc(pc),start(state ? std::chrono::steady_clock::now():std::chrono::steady_clock::time_point{}) {}
    ~Timer() {
        if(!state)return;
        const auto ns=uint64_t(std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now()-start).count());
        state->count(key,ns);if(pc)state->pcs[pc].build_ns+=ns;
    }
};
}
#else
#define OPERIT_EXEC_DIAGNOSTIC(...)
#endif
