#include "cpu/aarch64/block_graph.h"
#include "acceleration/fixed_components.h"
#include "cpu/aarch64/optimizer.h"
#include <algorithm>
#include <deque>
#include <iterator>
#include <map>
#include <unordered_set>
namespace operit::arm64 {
namespace {
struct Successors {
    std::array<uint64_t,2> values{};
    size_t count=0;
    const uint64_t *begin() const {return values.data();}
    const uint64_t *end() const {return values.data()+count;}
};
struct DiscoveryPage {
    Memory::CodeSnapshot snapshot;
    std::array<uint64_t,page_size/4/64> discovered{};
};
Successors successors(const InstructionNode &n) {
    const auto &i=n.instruction;
    switch(i.op) {
    case Op::Branch: return {{n.pc+i.immediate,n.pc+4},(i.raw>>31) ? 2u:1u};
    case Op::ConditionalBranch: case Op::CompareBranch: case Op::TestBranch: return {{n.pc+i.immediate,n.pc+4},2};
    case Op::BranchRegister: case Op::Svc: case Op::Fault: case Op::Unknown: return {};
    default: return {{n.pc+4,0},1};
    }
}
bool ends(const Instruction &i) {
    return i.terminal || i.op==Op::Branch || i.op==Op::BranchRegister || i.op==Op::ConditionalBranch ||
           i.op==Op::CompareBranch || i.op==Op::TestBranch || i.op==Op::Svc || i.op==Op::Fault || i.op==Op::Unknown;
}
// Both backends split before every possible entry and after every control edge.
// Each conditional fallthrough must start a block
// even when its target lies outside this region.
void partition_graph(BlockGraph &graph,const std::vector<InstructionNode> &nodes) {
    std::unordered_set<uint64_t> leaders{graph.address};
    for(size_t i=0;i<nodes.size();++i) {
        const auto &node=nodes[i];
        const InstructionNode *previous=i ? &nodes[i-1]:nullptr;
        // Preserve the previous-address lookup's unsigned wraparound.
        if(node.pc==0 && !nodes.empty() && nodes.back().pc==UINT64_MAX-3)previous=&nodes.back();
        if(!previous || previous->pc!=node.pc-4 || ends(previous->instruction))leaders.insert(node.pc);
        if(ends(node.instruction))for(auto target:successors(node))leaders.insert(target);
    }
    for(const auto &node:nodes) {
        if(graph.blocks.empty() || leaders.count(node.pc)) {
            auto block=std::make_unique<BasicBlock>();block->pc=node.pc;
            graph.entries.emplace(node.pc,block.get());graph.blocks.push_back(std::move(block));
        }
        auto &block=*graph.blocks.back();
        block.instructions.push_back(node);
        const auto edges=successors(node);block.successors.assign(edges.begin(),edges.end());
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
    std::vector<InstructionNode> nodes;nodes.reserve(words.size());
    std::unordered_set<uint64_t> addresses;addresses.reserve(words.size());
    for(auto [at,word]:words) {
        if(at&3)throw Failure("unaligned instruction address "+hex(at));
        if(!addresses.insert(at).second)throw Failure("duplicate instruction address "+hex(at));
        InstructionNode node;node.pc=at;node.instruction=decode(word);
        node.effects=instruction_effects(node.instruction);node.semantic=select_semantic(node.instruction);
        nodes.push_back(node);
    }
    std::sort(nodes.begin(),nodes.end(),[](const auto &a,const auto &b){return a.pc<b.pc;});
    const auto entry=std::lower_bound(nodes.begin(),nodes.end(),pc,[](const auto &node,uint64_t address){return node.pc<address;});
    if(entry==nodes.end() || entry->pc!=pc)throw Failure("missing region entry "+hex(pc));
    partition_graph(*graph,nodes);optimize_graph(*graph);
    return graph;
}
std::unique_ptr<BlockGraph> build_graph(uint64_t pc,Memory &memory,const FixedComponents *intrinsics,
                                       GraphEntryKnown entry_known,const void *entry_context,GraphDiscoveryPolicy policy,
                                       GraphCallHot call_hot,const void *call_context) {
    if(pc&3) throw Failure("unaligned instruction address "+hex(pc));
    auto graph=std::make_unique<BlockGraph>(); graph->address=pc;
    OPERIT_EXEC_DIAGNOSTIC(
        graph->diagnostics=execution_diagnostics::active;
        auto *diagnostics=graph->diagnostics;
        execution_diagnostics::Timer build_timer(diagnostics,"compile.ns",pc);
        if(diagnostics) {diagnostics->count("compile.attempts");++diagnostics->pcs[pc].graph_builds;}
    )
    std::vector<InstructionNode> nodes;
    std::unordered_map<uint64_t,DiscoveryPage> pages;
    std::deque<uint64_t> queue{pc};
    // Half-open union of static backward-edge ranges, and calls waiting for
    // a covering range. A range is only a heuristic, not proof of hotness.
    std::map<uint64_t,uint64_t> loop_ranges;
    std::map<uint64_t,uint64_t> pending_calls;
    std::unordered_set<uint64_t> promoted_targets;
    const auto in_loop=[&](uint64_t address) {
        auto it=loop_ranges.upper_bound(address);
        return it!=loop_ranges.begin() && address<(--it)->second;
    };
    const auto add_loop=[&](uint64_t start,uint64_t last) {
        // The caller excludes the final aligned address, where this wraps.
        uint64_t end=last+4;
        auto it=loop_ranges.lower_bound(start);
        if(it!=loop_ranges.begin()) {
            auto previous=std::prev(it);
            if(previous->second>=start)it=previous;
        }
        while(it!=loop_ranges.end() && it->first<=end) {
            start=std::min(start,it->first);end=std::max(end,it->second);
            it=loop_ranges.erase(it);
        }
        loop_ranges.emplace(start,end);
        auto pending=pending_calls.lower_bound(start);
        while(pending!=pending_calls.end() && pending->first<end) {
            queue.push_back(pending->second);
            pending=pending_calls.erase(pending);
        }
    };
    // Discover unique instructions first. Splitting at all leaders afterwards
    // prevents overlapping entries and represents a backedge exactly once.
    while(!queue.empty() && nodes.size()<16384) {
        auto at=queue.front(); queue.pop_front();
        const auto page=align_down(at);auto found=pages.find(page);
        const auto index=unsigned((at-page)/4);
        if(!(at&3) && found!=pages.end() && (found->second.discovered[index/64] & (uint64_t{1}<<(index%64)))) {
            OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.already_seen");) continue;
        }
        // The successor is owned by another graph. Leave an external edge so
        // that the regular link path validates its current code and lifetime.
        if(at!=pc && entry_known && !promoted_targets.count(at) && entry_known(entry_context,at)) {
            OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.cached_boundary");)
            continue;
        }
        if(intrinsics && at!=pc && intrinsics->contains(at)) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.intrinsic_boundary");) continue;}
        if(at&3) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.unaligned_successor");) continue;}
        if(found==pages.end()) {
            if(pages.size()==64) {OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count("discover.page_limit");) continue;}
            try { found=pages.emplace(page,DiscoveryPage{memory.code_snapshot(at),{}}).first; }
            catch(const Failure &) {
                OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)diagnostics->count(at==pc ? "discover.required_fetch_fault":"discover.speculative_fetch_fault");)
                if(at==pc) throw; else continue;
            }
        }
        auto instruction=decode((*found->second.snapshot.words)[index]);
        InstructionNode node;node.pc=at;node.instruction=instruction;node.effects=instruction_effects(instruction);
        node.semantic=select_semantic(instruction);nodes.push_back(node);
        found->second.discovered[index/64]|=uint64_t{1}<<(index%64);
        auto edges=successors(node);
        const bool call=instruction.op==Op::Branch && (instruction.raw>>31);
        const bool promoted=call && call_hot && call_hot(call_context,at,edges.values[0]);
        if(promoted)promoted_targets.insert(edges.values[0]);
        const bool deferred=call && policy!=GraphDiscoveryPolicy::Eager &&
                            !promoted && (policy!=GraphDiscoveryPolicy::DeferNonLoopCalls || !in_loop(at));
        OPERIT_EXEC_DIAGNOSTIC(if(deferred && diagnostics)diagnostics->count("discover.deferred_call_target");)
        if(deferred && policy==GraphDiscoveryPolicy::DeferNonLoopCalls)pending_calls.emplace(at,edges.values[0]);
        if(policy==GraphDiscoveryPolicy::DeferNonLoopCalls && !call && ends(instruction)) {
            for(auto target:edges)if(target<at && target<=UINT64_MAX-4 && at<=UINT64_MAX-4)add_loop(target,at);
        }
        for(size_t i=deferred ? 1:0;i<edges.count;++i)queue.push_back(edges.values[i]);
    }
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics) {
        diagnostics->count("compile.instructions",nodes.size());diagnostics->count("compile.pages",pages.size());
        if(!queue.empty()) {diagnostics->count("discover.instruction_limit");diagnostics->count("discover.undiscovered_queue",queue.size());}
    })
    std::sort(nodes.begin(),nodes.end(),[](const auto &a,const auto &b){return a.pc<b.pc;});
    for(const auto &node:nodes) {
        const auto at=node.pc;
        OPERIT_EXEC_DIAGNOSTIC(if(diagnostics)++diagnostics->pcs[at].instruction_compiles;)
        const auto page=align_down(at);const auto index=uint32_t((at-page)/4);
        auto &snapshot=pages.at(page).snapshot;
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
    optimize_graph(*graph);
#if OPERIT_ARM64_GLOBAL_VALUES
    optimize_global_values(*graph);
#endif
    lower_graph(*graph);
    OPERIT_EXEC_DIAGNOSTIC(if(diagnostics) {
        diagnostics->count("compile.blocks",graph->blocks.size());diagnostics->count("compile.dependencies",graph->dependencies.size());
        diagnostics->count("compile.proofs",graph->proofs.size());diagnostics->count("compile.eliminated",graph->eliminated);diagnostics->count("compile.fused",graph->fused);
        diagnostics->count("compile.ssa_constants",graph->ssa_constants);
        diagnostics->count("compile.cse_copies",graph->cse_copies);
        diagnostics->count("compile.cse_eliminated",graph->cse_eliminated);
        diagnostics->count("compile.scheduled_loads",graph->scheduled_loads);
        diagnostics->count("compile.root_entry_only",graph->root_entry_only);
        for(const auto &block:graph->blocks)diagnostics->count("compile.operations",block->code.size());
    })
    return graph;
}
}
