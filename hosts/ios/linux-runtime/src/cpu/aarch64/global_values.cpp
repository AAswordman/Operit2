#include "cpu/aarch64/optimizer.h"
#include <algorithm>
#include <array>
#include <map>
#include <unordered_map>

namespace operit::arm64 {
bool schedulable_integer(const InstructionNode &n) {
    if(n.instruction.encoding==Encoding::General || n.effects.memory || n.effects.control)return false;
    switch(n.instruction.op) {
    case Op::Hint:case Op::Address:case Op::MoveWide:case Op::AddImmediate:case Op::AddRegister:
    case Op::AddCarry:case Op::LogicalImmediate:case Op::LogicalRegister:case Op::Bitfield:
    case Op::Extract:case Op::TwoSource:case Op::OneSource:case Op::Multiply:case Op::MultiplyHigh:
    case Op::ConditionalSelect:case Op::ConditionalCompare:return true;
    default:return false;
    }
}
namespace {
using Id=uint32_t;
using Registers=std::array<Id,33>; // GP/SP and NZCV
struct Definition {
    Id parent;
    bool constant=false;
    uint64_t bits=0;
};
struct SsaInstruction {
    InstructionNode *node;
    Registers before{},after{};
};
struct SsaBlock {
    Registers input{},output{};
    std::vector<size_t> predecessors;
    std::vector<SsaInstruction> instructions;
    bool external=false;
};
// SSA names and phi inputs are fixed before value numbering. Equality only
// ever joins proven equivalents; an unvisited predecessor is never a value.
class Values {
    BlockGraph &graph;
    std::vector<Definition> definitions{{0}};
    std::vector<SsaBlock> blocks;
    std::map<uint64_t,Id> constants;
    Id create() {auto id=Id(definitions.size());definitions.push_back({id});return id;}
    Id root(Id id) {
        auto r=id;while(definitions[r].parent!=r)r=definitions[r].parent;
        while(definitions[id].parent!=id) {auto next=definitions[id].parent;definitions[id].parent=r;id=next;}
        return r;
    }
    bool join(Id output,Id value) {
        output=root(output);value=root(value);if(output==value)return false;
        // Constants must remain the representative. All other joins have a
        // deterministic direction to make cyclic phi/expression graphs finite.
        if(definitions[output].constant || (!definitions[value].constant && output<value))std::swap(output,value);
        definitions[output].parent=value;return true;
    }
    Id constant(uint64_t bits) {
        auto found=constants.find(bits);if(found!=constants.end())return found->second;
        auto id=create();definitions[id].constant=true;definitions[id].bits=bits;constants[bits]=id;return id;
    }
    static bool boundary(const InstructionNode &n) {
        if(schedulable_integer(n))return false;
        // Ordinary memory and direct control have explicit GP/NZCV effects.
        // Everything else is a conservative SSA kill, including helpers,
        // syscalls, atomics and operations with partially described effects.
        switch(n.instruction.op) {
        case Op::LoadStore:case Op::LoadPair:case Op::LoadLiteral:
            return n.instruction.encoding==Encoding::General;
        case Op::Branch:case Op::ConditionalBranch:case Op::CompareBranch:case Op::TestBranch:return false;
        default:return true;
        }
    }
    std::array<uint64_t,12> expression(const SsaInstruction &s) {
        const auto &n=*s.node;const auto &i=n.instruction;
        const auto operand=[&](unsigned reg) -> Id {
            return n.effects.gp_read&(uint32_t{1}<<reg) ? root(s.before[reg]):constant(0);
        };
        // Decoded fields are the semantic opcode. Do not include destination
        // numbers, nor unused register fields embedded in immediate encodings.
        uint64_t immediate=i.immediate;
        if(i.op==Op::Address)immediate=i.flags ? align_down(n.pc)+(i.immediate<<12):n.pc+i.immediate;
        const bool rn=i.op!=Op::MoveWide && i.op!=Op::Address && i.op!=Op::Hint;
        const bool rm=i.op==Op::AddRegister || i.op==Op::AddCarry || i.op==Op::LogicalRegister ||
            i.op==Op::Extract || i.op==Op::TwoSource || i.op==Op::Multiply ||
            i.op==Op::MultiplyHigh || i.op==Op::ConditionalSelect || (i.op==Op::ConditionalCompare && !(i.flags&2));
        const bool rd=i.op==Op::Bitfield || (i.op==Op::MoveWide &&
            (i.encoding==Encoding::Keep32 || i.encoding==Encoding::Keep64));
        return {unsigned(i.op),unsigned(i.encoding),i.width,i.shift,i.flags,immediate,i.auxiliary,
            rn ? operand(i.rn):0,rm ? operand(i.rm):0,i.op==Op::Multiply ? operand(i.ra):0,
            rd ? operand(i.rd):0,n.effects.flags_read ? root(s.before[32]):0};
    }
    bool fold(const SsaInstruction &s) {
        const auto &n=*s.node;
        CpuState cpu;cpu.pc=n.pc;
        for(unsigned r=0;r<33;++r) {
            const bool read=r==32 ? n.effects.flags_read:bool(n.effects.gp_read&(uint32_t{1}<<r));
            if(!read)continue;
            const auto &v=definitions[root(s.before[r])];if(!v.constant)return false;
            if(r==32)cpu.nzcv=v.bits;else cpu.set(r,v.bits,64,true);
        }
        static thread_local Memory unused_memory(0);
        n.semantic(cpu,unused_memory,n.instruction);
        bool changed=false;
        for(unsigned r=0;r<33;++r)if(s.before[r]!=s.after[r])
            changed|=join(s.after[r],constant(r==32 ? cpu.nzcv:cpu.get(r,64,true)));
        return changed;
    }
public:
    Values(BlockGraph &g,bool global):graph(g),blocks(g.blocks.size()) {
        std::unordered_map<uint64_t,size_t> indices;
        for(size_t b=0;b<blocks.size();++b)indices[g.blocks[b]->pc]=b;
        for(size_t b=0;b<blocks.size();++b) {
            auto &block=blocks[b];block.external=!global || g.blocks[b]->pc==g.address;
            for(auto &id:block.input)id=create();
            auto current=block.input;
            for(auto &n:g.blocks[b]->instructions) {
                SsaInstruction s{&n,current,current};
                for(unsigned r=0;r<33;++r)if(boundary(n) ||
                    (r==32 ? n.effects.flags_write:bool(n.effects.gp_write&(uint32_t{1}<<r))))s.after[r]=create();
                current=s.after;block.instructions.push_back(s);
            }
            block.output=current;
        }
        if(global)for(size_t b=0;b<blocks.size();++b) {
            const auto &last=g.blocks[b]->instructions.back();
            for(auto pc:g.blocks[b]->successors) {
                auto found=indices.find(pc);if(found==indices.end())continue;
                // BL's synthetic return edge cannot carry facts from before
                // the call. RET/indirect misses resolve a fresh root graph.
                if(last.instruction.encoding==Encoding::BranchLink && pc==last.pc+4) {
                    blocks[found->second].external=true;continue;
                }
                blocks[found->second].predecessors.push_back(b);
            }
        }
        // Disconnected components (including cycles with no root path) also
        // receive unknown entry values; no facts are invented for them.
        std::vector<bool> reachable(blocks.size());
        if(global) {
            std::vector<size_t> work;
            for(size_t b=0;b<blocks.size();++b)if(blocks[b].external || blocks[b].predecessors.empty()) {reachable[b]=true;work.push_back(b);}
            std::vector<std::vector<size_t>> successors(blocks.size());
            for(size_t b=0;b<blocks.size();++b)for(auto p:blocks[b].predecessors)successors[p].push_back(b);
            while(!work.empty()) {auto b=work.back();work.pop_back();for(auto next:successors[b])if(!reachable[next]) {reachable[next]=true;work.push_back(next);}}
        }
        for(size_t b=0;b<blocks.size();++b)if(!reachable[b] || blocks[b].predecessors.empty())blocks[b].external=true;
    }
    void solve() {
        // A bounded compile-time fixed point; stopping early only loses
        // optimization. All joins completed so far remain valid.
        for(unsigned iteration=0;iteration<16;++iteration) {
            bool changed=false;
            for(auto &b:blocks)if(!b.external)for(unsigned r=0;r<33;++r) {
                Id common=0;bool different=false;const auto self=root(b.input[r]);
                for(auto p:b.predecessors) {
                    auto value=root(blocks[p].output[r]);if(value==self)continue;
                    if(common && common!=value){different=true;break;}common=value;
                }
                if(common && !different)changed|=join(b.input[r],common);
            }
            std::map<std::array<uint64_t,12>,Id> expressions;
            for(auto &b:blocks)for(auto &s:b.instructions) {
                auto &n=*s.node;if(!schedulable_integer(n))continue;
                changed|=fold(s);
                const auto writes=n.effects.gp_write;
                if(!writes || (writes&(writes-1)) || n.effects.flags_write)continue;
                auto result=s.after[__builtin_ctz(writes)];
                const auto key=expression(s);
                auto [it,inserted]=expressions.emplace(key,result);
                if(!inserted)changed|=join(result,it->second);
                // A 64-bit architectural copy preserves the exact value ID.
                const auto &i=n.instruction;
                if(i.width==64 && ((i.op==Op::AddImmediate && !i.immediate) ||
                   (i.op==Op::LogicalRegister && i.flags==1 && !i.shift && i.rn==31)))
                    changed|=join(result,i.op==Op::AddImmediate ? s.before[i.rn]:
                        (i.rm==31 ? constant(0):s.before[i.rm]));
            }
            if(!changed)break;
        }
    }
    bool apply() {
        bool changed=false;
        for(auto &b:blocks) {
            auto current=b.input;
            for(auto &s:b.instructions) {
                auto &n=*s.node;
                // Removed writes were never materialized. They must not
                // become the source of a newly introduced architectural copy.
                if(n.dead_result)continue;
                const auto writes=n.effects.gp_write;
                if(schedulable_integer(n) && writes && !(writes&(writes-1)) && !n.effects.flags_write) {
                    const unsigned dest=__builtin_ctz(writes);const auto value=root(s.after[dest]);
                    const auto &definition=definitions[value];
                    if(definition.constant && !n.constant_result) {
                        n.constant_result=true;n.constant=definition.bits;n.copy_register=-1;
                        ++graph.ssa_constants;changed=true;
                    } else if(!n.constant_result) {
                        // Use a currently available architectural value; SSA
                        // names alone do not allocate persistent temporaries.
                        for(unsigned r=0;r<32;++r)if(root(current[r])==value) {
                            if(r==dest) {n.dead_result=true;++graph.eliminated;++graph.cse_eliminated;changed=true;}
                            else if(n.copy_register!=int(r)) {n.copy_register=int8_t(r);++graph.cse_copies;changed=true;}
                            break;
                        }
                    }
                }
                if(!n.dead_result)for(unsigned r=0;r<33;++r)
                    if(s.before[r]!=s.after[r])current[r]=s.after[r];
            }
        }
        return changed;
    }
};
}
void optimize_global_values(BlockGraph &graph) {
    // Bound both analysis latency and the overlap created by root-only graphs.
    // Large discovery graphs retain the existing local optimizer and multi-
    // entry cache; making every such graph root-only multiplies startup work.
    size_t instructions=0;
    for(const auto &block:graph.blocks)instructions+=block->instructions.size();
    if(instructions>1024 || graph.blocks.size()>256)return;
    Values local(graph,false);local.solve();local.apply();
    Values global(graph,true);global.solve();
    // External/indirect entry into an interior block must build its own root
    // graph if transformations depend on incoming SSA facts.
    graph.root_entry_only=global.apply();
}
}
