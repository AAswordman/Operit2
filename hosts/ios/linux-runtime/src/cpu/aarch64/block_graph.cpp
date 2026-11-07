#include "cpu/aarch64/block_graph.h"
#include "acceleration/fixed_components.h"
#include "cpu/aarch64/optimizer.h"
#include <algorithm>
#include <deque>
#include <map>
#include <set>
namespace operit::arm64 {
namespace {
std::vector<uint64_t> successors(const InstructionNode &n) {
    const auto &i=n.instruction;
    switch(i.op) {
    case Op::Branch: return (i.raw>>31) ? std::vector<uint64_t>{n.pc+i.immediate,n.pc+4} : std::vector<uint64_t>{n.pc+i.immediate};
    case Op::ConditionalBranch: case Op::CompareBranch: case Op::TestBranch: return {n.pc+i.immediate,n.pc+4};
    case Op::BranchRegister: case Op::Svc: case Op::Fault: case Op::Unknown: return {};
    default: return {n.pc+4};
    }
}
bool ends(const Instruction &i) {
    return i.terminal || i.op==Op::Branch || i.op==Op::BranchRegister || i.op==Op::ConditionalBranch ||
           i.op==Op::CompareBranch || i.op==Op::TestBranch || i.op==Op::Svc || i.op==Op::Fault || i.op==Op::Unknown;
}
// Both backends split before every possible entry and after every control edge.
// Optimizations are block-local: a conditional fallthrough must start a block
// even when its target lies outside this region.
void partition_graph(BlockGraph &graph,const std::map<uint64_t,InstructionNode> &nodes) {
    std::set<uint64_t> leaders{graph.address};
    for(const auto &[pc,node]:nodes) {
        auto previous=nodes.find(pc-4);
        if(previous==nodes.end() || ends(previous->second.instruction))leaders.insert(pc);
        if(ends(node.instruction))for(auto target:successors(node))leaders.insert(target);
    }
    for(const auto &[pc,node]:nodes) {
        if(graph.blocks.empty() || leaders.count(pc)) {
            auto block=std::make_unique<BasicBlock>();block->pc=pc;
            graph.entries.emplace(pc,block.get());graph.blocks.push_back(std::move(block));
        }
        auto &block=*graph.blocks.back();
        block.instructions.push_back(node);block.successors=successors(node);
    }
}

}
bool BlockGraph::current(Memory &memory,uint64_t scope) const {
    if(validated_epoch!=memory.code_epoch.load(std::memory_order_relaxed)) {
        OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation("current_epoch_miss");)
        return false;
    }
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation(!scope ? "current_no_scope":validated_scope==UINT64_MAX ? "current_private_hit":validated_scope==scope ? "current_scope_hit":"current_scope_miss");)
    return scope && (validated_scope==scope || validated_scope==UINT64_MAX);
}
bool BlockGraph::validate(Memory &memory) {
    OPERIT_EXEC_DIAGNOSTIC(
        if(diagnostics)diagnostics->validation("calls");
        execution_diagnostics::Timer timer(diagnostics,"validate.ns");
        execution_diagnostics::ExceptionCounter exceptions(diagnostics,"validate.exception");
    )
    const auto before=memory.code_epoch.load(std::memory_order_relaxed);
    bool private_code=true;
    for(size_t start=0;start<dependencies.size();) {
        const auto &first=dependencies[start];auto end=start+1;
        uint64_t mask=Memory::CodeProof::mask(first.offset,first.count);
        while(end<dependencies.size()) {
            const auto &next=dependencies[end];
            if(align_down(next.address)!=align_down(first.address) || next.words!=first.words || next.proof!=first.proof)break;
            mask|=Memory::CodeProof::mask(next.offset,next.count);++end;
        }
        OPERIT_EXEC_DIAGNOSTIC(if(diagnostics) {
            diagnostics->validation("groups");
            diagnostics->validation("required_lines",unsigned(__builtin_popcountll(mask)));
        })
        uint64_t epoch=0;
        if(!memory.validate_code_lines(first.address,first.words,mask,epoch,first.proof.get())) {
            OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation("line_proof_miss");)
            for(auto index=start;index<end;++index) {
                const auto &d=dependencies[index];
                OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation("snapshot_checks");)
                if(!memory.validate_code_snapshot_cached(d.address,d.words,d.offset,d.count,epoch,d.proof.get())) {
                    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation("snapshot_mismatch");)
                    return false;
                }
            }
        } else {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation("line_proof_hit");)}
        private_code&=first.proof && memory.owns_code_proof(*first.proof) && first.proof->generation==UINT64_MAX;
        start=end;
    }
    if(memory.code_epoch.load(std::memory_order_relaxed)!=before) {
        OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation("final_epoch_changed");)
        return false;
    }
    validated_epoch=before; validated_scope=private_code ? UINT64_MAX:memory.code_proof_generation();
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->validation(private_code ? "success_private":"success_observed");)
    return true;
}
std::unique_ptr<BlockGraph> build_instruction_graph(const std::vector<std::pair<uint64_t,uint32_t>> &words,uint64_t pc) {
    auto graph=std::make_unique<BlockGraph>();graph->address=pc;
    std::map<uint64_t,InstructionNode> nodes;
    for(auto [at,word]:words) {
        if(at&3)throw Failure("unaligned instruction address "+hex(at));
        InstructionNode node;node.pc=at;node.instruction=decode(word);
        node.effects=instruction_effects(node.instruction);node.semantic=select_semantic(node.instruction);
        if(!nodes.emplace(at,node).second)throw Failure("duplicate instruction address "+hex(at));
    }
    if(!nodes.count(pc))throw Failure("missing region entry "+hex(pc));
    partition_graph(*graph,nodes);optimize_graph(*graph);
    return graph;
}
std::unique_ptr<BlockGraph> build_graph(uint64_t pc,Memory &memory,const FixedComponents *intrinsics) {
    if(pc&3) throw Failure("unaligned instruction address "+hex(pc));
    auto graph=std::make_unique<BlockGraph>(); graph->address=pc;
    OPERIT_EXEC_DIAGNOSTIC(
        graph->diagnostics=execution_diagnostics::active;
        auto *diagnostics=graph->diagnostics;
        execution_diagnostics::Timer build_timer(diagnostics,"compile.ns",pc);
        if(diagnostics) {diagnostics->count("compile.attempts");++diagnostics->pcs[pc].graph_builds;}
    )
    std::map<uint64_t,InstructionNode> nodes;
    std::map<uint64_t,Memory::CodeSnapshot> pages;
    std::deque<uint64_t> queue{pc};
    // Discover unique instructions first. Splitting at all leaders afterwards
    // prevents overlapping entries and represents a backedge exactly once.
    while(!queue.empty() && nodes.size()<16384) {
        auto at=queue.front(); queue.pop_front();
        if(nodes.count(at)) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.already_seen");) continue;}
        if(intrinsics && at!=pc && intrinsics->contains(at)) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.intrinsic_boundary");) continue;}
        if(at&3) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.unaligned_successor");) continue;}
        const auto page=align_down(at); auto found=pages.find(page);
        if(found==pages.end()) {
            if(pages.size()==64) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.page_limit");) continue;}
            try { found=pages.emplace(page,memory.code_snapshot(at)).first; }
            catch(const Failure &) {
                OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count(at==pc ? "discover.required_fetch_fault":"discover.speculative_fetch_fault");)
                if(at==pc) throw; else continue;
            }
        }
        const auto index=unsigned((at-page)/4);
        auto instruction=decode((*found->second.words)[index]);
        InstructionNode node;node.pc=at;node.instruction=instruction;node.effects=instruction_effects(instruction);
        node.semantic=select_semantic(instruction);nodes.emplace(at,node);
        auto edges=successors(node);
        for(auto target:edges)queue.push_back(target);
    }
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics) {
        diagnostics->count("compile.instructions",nodes.size());diagnostics->count("compile.pages",pages.size());
        if(!queue.empty()) {diagnostics->count("discover.instruction_limit");diagnostics->count("discover.undiscovered_queue",queue.size());}
    })
    for(const auto &[at,node]:nodes) {
        OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)++diagnostics->pcs[at].instruction_compiles;)
        (void)node;const auto page=align_down(at);const auto index=uint32_t((at-page)/4);
        auto &snapshot=pages.at(page);
        if(!graph->dependencies.empty() && align_down(graph->dependencies.back().address)==page &&
           graph->dependencies.back().offset+graph->dependencies.back().count==index)++graph->dependencies.back().count;
        else graph->dependencies.push_back({at,snapshot.words,snapshot.proof,index,1});
    }
    partition_graph(*graph,nodes);
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)for(const auto &block:graph->blocks)++diagnostics->pcs[block->pc].block_compiles;)
    // Aggregate required lines per owned snapshot proof. This immutable list
    // lets native bridges consume proofs established by another overlapping
    // graph in the same serial observation scope.
    for(const auto &dependency:graph->dependencies) {
        if(!dependency.proof) {graph->proofs.clear();break;}
        auto found=std::find_if(graph->proofs.begin(),graph->proofs.end(),
            [&](const auto &entry){return entry.proof==dependency.proof.get();});
        const auto mask=Memory::CodeProof::mask(dependency.offset,dependency.count);
        if(found==graph->proofs.end())graph->proofs.push_back({dependency.proof.get(),mask});
        else found->mask|=mask;
    }
    optimize_graph(*graph); lower_graph(*graph);
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics) {
        diagnostics->count("compile.blocks",graph->blocks.size());diagnostics->count("compile.dependencies",graph->dependencies.size());
        diagnostics->count("compile.proofs",graph->proofs.size());diagnostics->count("compile.eliminated",graph->eliminated);diagnostics->count("compile.fused",graph->fused);
        for(const auto &block:graph->blocks)diagnostics->count("compile.operations",block->code.size());
    })
    return graph;
}
}
