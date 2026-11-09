#include "cpu/aarch64/execution_engine.h"
#include "cpu/aarch64/deopt_map.h"
#include "cpu/aarch64/floating_point.h"
#include <algorithm>
#include <cstddef>
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
#include "memory/memory_diagnostics.h"
#endif
namespace operit::arm64 {
#if OPERIT_ARM64_ADAPTIVE_CALLS
extern "C" void operit_static_hot_bridge();
static const void *hot_bridge_entry() { return reinterpret_cast<const void *>(&operit_static_hot_bridge); }
#endif
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
static_assert(sizeof(execution_diagnostics::static_code_names)/sizeof(*execution_diagnostics::static_code_names)==unsigned(StaticCode::Count));
static_assert(sizeof(execution_diagnostics::encoding_names)/sizeof(*execution_diagnostics::encoding_names)==unsigned(Encoding::Count));
namespace {
unsigned diagnostic_environment(const char *name,unsigned fallback,unsigned maximum) noexcept {
    const auto *text=std::getenv(name);if(!text || !*text)return fallback;
    char *end=nullptr;const auto value=std::strtoul(text,&end,10);
    return end && !*end && value<=maximum ? unsigned(value):fallback;
}
void diagnostic_transfer(execution_diagnostics::State &diagnostics,const ExecutionFrame &frame,const char *site,
                         uint32_t gp=UINT32_MAX,uint32_t vector=UINT32_MAX) {
    if(frame.graph)diagnostics.transfer(site,resident_gp,resident_vector,gp,vector);
}
void diagnostic_slow(execution_diagnostics::State &diagnostics,const ExecutionFrame &frame,const StaticOperation &op,const OperationInfo &info) {
    const auto code=info.code;
    const auto *source=&op;
    if(code==StaticCode::Bridge)source=reinterpret_cast<const BridgePlan*>(op.immediate)->source;
    if(code==StaticCode::IndirectDispatch)source=reinterpret_cast<const IndirectCache*>(op.immediate)->source;
    const char *reason="native_memory_guard";
    switch(code) {
    case StaticCode::Block:case StaticCode::BlockBody:
        reason=frame.memory->code_epoch.load(std::memory_order_relaxed)!=frame.epoch ? "block_epoch":"block_budget";break;
    case StaticCode::Bridge:reason="bridge_guard";break;
    case StaticCode::IndirectDispatch:reason="indirect_dispatch_miss";break;
    case StaticCode::Next:case StaticCode::Jump:case StaticCode::Call:case StaticCode::Indirect:case StaticCode::IndirectCall:
    case StaticCode::Branch:case StaticCode::CompareBranch:case StaticCode::TestBranch:case StaticCode::CompareImmediateBranch:
        reason="unlinked_control_edge";break;
    case StaticCode::Helper:reason="helper";break;
    default:break;
    }
    const auto pc=source && source!=&op ? operation_info(*source).pc:info.pc;
    ++diagnostics.pcs[pc].slow_calls;
    diagnostics.count(std::string("slow.static.")+execution_diagnostics::static_code_names[unsigned(code)]);
    if(frame.graph) {
        auto found=frame.graph->source.find(pc);
        if(found!=frame.graph->source.end()) {
            const auto &i=found->second->instruction;
            if(code==StaticCode::Helper)reason=execution_diagnostics::helper_reason(i);
            ++diagnostics.instruction(reason,pc,i,unsigned(code)).calls;
        }
    }
    diagnostics.count(std::string("slow.reason.")+reason);
}
}
ExecutionEngine::~ExecutionEngine() {
    if(execution_diagnostics::auto_dump_enabled())dump_diagnostics("engine_destroy");
}
void ExecutionEngine::dump_diagnostics(const char *phase) const noexcept {
    try {
        // Noether's writer produces a standalone JSON document. Give every
        // cumulative boundary a distinct filename and link it from our JSONL.
        const auto *prefix=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_NATIVE_OUTPUT");
        if(!prefix || !*prefix) {
            prefix=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_OUTPUT");
            if(prefix && (!std::strcmp(prefix,"stderr") || !std::strcmp(prefix,"-")))prefix=nullptr;
        }
        std::string native_path;bool native_written=false;
        if(prefix && *prefix && diagnostics_.native_initialized) {
            char suffix[112];std::snprintf(suffix,sizeof(suffix),".engine-%p.snapshot-%llu.native.json",static_cast<const void*>(this),
                                         static_cast<unsigned long long>(diagnostics_.snapshot+1));
            native_path=std::string(prefix)+suffix;
            const auto top=diagnostic_environment("OPERIT_EXECUTION_DIAGNOSTICS_NATIVE_TOP",256,1u<<20);
            native_written=execution_diagnostics_write_json(frame_,native_path.c_str(),top);
        }
        diagnostics_.dump(this,phase,executed_instructions,cache_hits,slow_calls,materializations,graphs_.size(),entries_.size(),native_path.c_str(),native_written);
        const auto *memory=std::getenv("OPERIT_EXECUTION_DIAGNOSTICS_MEMORY");
        if((memory && *memory && std::strcmp(memory,"0")) || (!memory && execution_diagnostics::clock_enabled())) {
            ::flockfile(stderr);
            std::fprintf(stderr,"operit-memory-boundary engine=%p snapshot=%llu clock=%llu phase=%s retired=%llu\n",static_cast<const void*>(this),
                         static_cast<unsigned long long>(diagnostics_.snapshot),static_cast<unsigned long long>(diagnostics_.clock_sequence),
                         phase ? phase:"manual",static_cast<unsigned long long>(executed_instructions));
            memory_detail::MemoryDiagnostics::dump(stderr);
            ::funlockfile(stderr);
        }
    } catch(...) {}
}
#endif
static_assert(offsetof(memory_detail::ReservationState,token)==8);
static_assert(memory_detail::PageMutex::native_state_offset()==0 && memory_detail::PageMutex::native_waiting_offset()==4);
void ExecutionEngine::erase_entries(const BlockGraph &graph) {
    // Overlapping graphs may discover the same PC. Erase only entries that
    // this graph actually owns; another graph's existing entry must survive.
    for(const auto &block:graph.blocks) {
        const auto found=entries_.find(block->pc);
        if(found!=entries_.end() && found->second.graph==&graph)entries_.erase(found);
    }
}
ExecutionEngine::Entry ExecutionEngine::find(uint64_t pc) {
    OPERIT_EXEC_DIAGNOSTIC(
        execution_diagnostics::ValidationScope validation_site(&diagnostics_,"find");
        diagnostics_.count("cache.find");
    )
    auto found=entries_.find(pc);
    if(found!=entries_.end()) {
        auto entry=found->second;
        try {
            if(entry.graph->current(*memory_,memory_->code_proof_generation()) || entry.graph->validate(*memory_)) {
                OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("cache.find_hit");)
                ++cache_hits;return entry;
            }
        } catch(const MemoryFault &) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("cache.invalidate.memory_fault");)
            // A speculative successor can lose execute permission before it
            // is taken. Rebuild from the required PC to defer that fault.
        }
        // The caller has detached its frame and published its old bindings.
        // Invalidate target records before reclaiming them. The engine
        // generation separately protects C++ source references across select().
        OPERIT_EXEC_DIAGNOSTIC(
            diagnostics_.count("cache.invalidate.graph");++diagnostics_.pcs[entry.graph->address].invalidations;
        )
        ++cache_generation_;
        ++*entry.graph->lifetime;
        erase_entries(*entry.graph);
        graphs_.erase(std::remove_if(graphs_.begin(),graphs_.end(),[&](const auto &graph){return graph.get()==entry.graph;}),graphs_.end());
    }
    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("cache.find_miss");)
    if(graphs_.size()>=graph_cache_capacity) {
        // Native bridges mark their destination too: a graph can stay hot for
        // millions of edges without returning through find().
        while(graphs_.front()->referenced) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("cache.eviction.second_chance");)
            graphs_.front()->referenced=0;
            auto used=std::move(graphs_.front());graphs_.pop_front();graphs_.push_back(std::move(used));
        }
        const auto victim=graphs_.front().get();
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("cache.eviction.capacity");++diagnostics_.pcs[victim->address].evictions;)
        ++cache_generation_;
        ++*victim->lifetime;
        erase_entries(*victim);
        graphs_.pop_front();
    }
    std::unique_ptr<BlockGraph> graph;
    for(;;) {
        graph=build_graph(pc,*memory_,intrinsics_,[](const void *context,uint64_t address) {
            const auto &entries=*static_cast<const decltype(entries_) *>(context);
            return entries.find(address)!=entries.end();
        },&entries_,OPERIT_ARM64_RETAIN_LOOP_CALLS ? GraphDiscoveryPolicy::DeferNonLoopCalls:
                    OPERIT_ARM64_DEFER_CALL_TARGETS ? GraphDiscoveryPolicy::DeferCallTargets:GraphDiscoveryPolicy::Eager
#if OPERIT_ARM64_ADAPTIVE_CALLS
        ,[](const void *context,uint64_t source,uint64_t target) {
            const auto &calls=*static_cast<const decltype(hot_calls_) *>(context);
            const auto found=calls.find(source);return found!=calls.end() && found->second==target;
        },&hot_calls_
#endif
        );
        try {
            OPERIT_EXEC_DIAGNOSTIC(execution_diagnostics::ValidationScope validation_site(&diagnostics_,"compiled");)
            if(graph->validate(*memory_)) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("compile.accepted");) break;}
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("compile.retry.validation_changed");)
        }
        catch(const MemoryFault &) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("compile.retry.memory_fault");)
            // Re-fetch the required entry before treating a speculative page
            // observation race as the current guest's instruction fault.
        }
    }
    OPERIT_EXEC_DIAGNOSTIC(execution_diagnostics_observe_graph(frame_,*graph);)
    for(auto &block:graph->blocks) {
        ++decoded_blocks;compiled_operations+=block->code.size();
        if(graph->root_entry_only && block->pc!=graph->address)continue;
        const auto inserted=entries_.emplace(block->pc,Entry{graph.get(),block.get()}).second;
        OPERIT_EXEC_DIAGNOSTIC(if(!inserted) {diagnostics_.count("cache.entry_collision");++diagnostics_.pcs[block->pc].entry_collisions;})
        (void)inserted;
    }
    eliminated_values+=graph->eliminated;fused_operations+=graph->fused;
    const auto entry=entries_.at(pc);graphs_.push_back(std::move(graph));return entry;
}
const StaticOperation *ExecutionEngine::select(uint64_t pc) {
    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("select.calls");)
    if(frame_.graph) {
        OPERIT_EXEC_DIAGNOSTIC(diagnostic_transfer(diagnostics_,frame_,"publish.select");)
        frame_.graph->referenced=1;frame_.publish();++materializations;frame_.graph=nullptr;
    }
    frame_.shadow.pc=pc;
    while(frame_.completed<frame_.budget && intrinsics_ && intrinsics_->contains(pc)) {
        uint64_t retired;
        try {retired=intrinsics_->execute(frame_.shadow,*memory_,frame_.budget-frame_.completed);}
        catch(...) {frame_.completed+=intrinsics_->completed_instructions();throw;}
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("intrinsic.calls");diagnostics_.count("intrinsic.retired",retired);)
        if(!retired) break;
        frame_.completed+=retired;pc=frame_.shadow.pc;
    }
    if(frame_.completed==frame_.budget) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("select.budget_stop");) return nullptr;}
    const auto entry=find(pc);frame_.graph=entry.graph;entry.graph->referenced=1;
    OPERIT_EXEC_DIAGNOSTIC(diagnostic_transfer(diagnostics_,frame_,"reload.select");)
    frame_.reload();
    frame_.epoch=entry.graph->validated_epoch;
    frame_.guest_fpcr=read_floating_control(frame_.shadow);
    return entry.block->code.data();
}
const StaticOperation *ExecutionEngine::link(const StaticOperation &source,uint64_t pc) {
    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("link.calls");)
    if(intrinsics_ && intrinsics_->contains(pc)) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("link.intrinsic");) return select(pc);}
    // Resolve source-owned cold information before select can reclaim it.
    const auto source_info=operation_info(source);
    const auto source_pc=source_info.pc;const auto opcode=source_info.code;
    auto *old=frame_.graph;const auto generation=cache_generation_;
    const auto next=select(pc);
    // select() can reclaim the source itself. Do not inspect source references
    // after any reclamation, even if the selected target remains alive.
    if(!next || generation!=cache_generation_) {
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count(!next ? "link.budget_stop":"link.cache_generation_changed");)
        return next;
    }
    auto &edge=const_cast<StaticOperation &>(source);
    std::unique_ptr<BridgePlan> *owned=nullptr;
    IndirectTarget *target=nullptr;
    if(opcode==StaticCode::Indirect || opcode==StaticCode::IndirectCall) {
        auto &cache=old->indirect_caches[&source];
        if(!cache) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("pic.created");) cache=std::make_unique<IndirectCache>();}
        auto &pic=*cache;
        pic.generation=old->lifetime.get();
        pic.expected_generation=*pic.generation;pic.source=&source;
        pic.operation.entry=static_entry(StaticCode::IndirectDispatch);
        pic.operation.immediate=reinterpret_cast<uint64_t>(&pic);pic.operation.index=OP_DETACHED_INDEX;
        pic.metadata=detached_operation_metadata(source_pc,StaticCode::IndirectDispatch);
        edge.target=&pic.operation;
        const auto set=unsigned(((pc>>2)^(pc>>9))&31);
        auto slot=set*4;bool found=false;
        for(unsigned n=0;n<4;++n)if(pic.targets[slot+n].pc==pc || !pic.targets[slot+n].operation){slot+=n;found=true;break;}
        if(!found) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("pic.slot_replacement");) slot+=pic.replacement[set]++&3;}
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count(pic.targets[slot].operation ? "pic.slot_refresh":"pic.slot_empty");)
        target=&pic.targets[slot];target->pc=pc;target->operation=next;
        if(old==frame_.graph) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("link.same_graph.indirect");++diagnostics_.links[{source_pc,pc,false,true,0,0}];)
            return next;
        }
        owned=&pic.bridge_for(slot);
    } else {
        if(old==frame_.graph) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("link.same_graph.direct");++diagnostics_.links[{source_pc,pc,false,false,0,0}];)
            edge.target=next;return next;
        }
        owned=&old->bridges[&source];
    }
    if(!*owned) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("bridge.plan_created");) *owned=std::make_unique<BridgePlan>();}
    else {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("bridge.plan_refreshed");)}
    auto &plan=**owned;plan.source=&source;
    plan.target_lifetime=frame_.graph->lifetime;
    plan.generation=plan.target_lifetime.get();plan.expected_generation=*plan.generation;
    plan.target_epoch=&frame_.graph->validated_epoch;plan.target_scope=&frame_.graph->validated_scope;plan.graph=frame_.graph;
    plan.proofs=frame_.graph->proofs.data();plan.proof_count=frame_.graph->proofs.size();
    OPERIT_EXEC_DIAGNOSTIC(
        const bool indirect=opcode==StaticCode::Indirect || opcode==StaticCode::IndirectCall;
        diagnostics_.count(indirect ? "link.cross_graph.indirect":"link.cross_graph.direct");
        ++diagnostics_.links[{source_pc,pc,true,indirect,0,0}];
    )
#if OPERIT_ARM64_ADAPTIVE_CALLS
    // Recompiling a large graph costs more; require proportionally more
    // observed calls before considering it worth rebuilding.
    const auto initial_hot_count=uint32_t(std::min<size_t>(std::max<size_t>(old->blocks.size(),1),256)*4096);
    const auto prior_hot_count=plan.operation.entry==hot_bridge_entry() && plan.metadata.origin==pc ? plan.metadata.payload:initial_hot_count;
#endif
    plan.operation.entry=static_entry(StaticCode::Bridge);plan.operation.index=OP_DETACHED_INDEX;
    plan.metadata=detached_operation_metadata(pc,StaticCode::Bridge);
#if OPERIT_ARM64_ADAPTIVE_CALLS
    const auto previous_hot=hot_calls_.find(source_pc);
    if(opcode==StaticCode::Call && (previous_hot==hot_calls_.end() || previous_hot->second!=pc) &&
       (hot_calls_.size()<4096 || previous_hot!=hot_calls_.end())) {
        plan.operation.entry=hot_bridge_entry();plan.metadata.payload=prior_hot_count;
    }
#endif
    plan.operation.immediate=reinterpret_cast<uint64_t>(&plan);plan.operation.target=next;
    if(target)target->operation=&plan.operation;else edge.target=&plan.operation;return next;
}
#if OPERIT_ARM64_ADAPTIVE_CALLS
const StaticOperation *ExecutionEngine::promote_call(const BridgePlan &plan,uint64_t pc) {
    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("adaptive.attempts");execution_diagnostics::Timer timer(&diagnostics_,"adaptive.ns");)
    if(frame_.completed==frame_.budget) {
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("adaptive.budget_deferred");)
        const_cast<BridgePlan &>(plan).metadata.payload=1;
        return select(pc);
    }
    const auto source_pc=operation_info(*plan.source).pc;
    auto *old=frame_.graph;const auto root=old->address;
    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("adaptive.replaced_blocks",old->blocks.size());)
    hot_calls_[source_pc]=pc;
    frame_.publish();++materializations;frame_.graph=nullptr;frame_.shadow.pc=pc;
    ++cache_generation_;++*old->lifetime;erase_entries(*old);
    graphs_.erase(std::remove_if(graphs_.begin(),graphs_.end(),[&](const auto &graph){return graph.get()==old;}),graphs_.end());
    try {
        const auto owner=find(root);
        const auto target=owner.graph->entries.find(pc);
        if(!owner.graph->root_entry_only && target!=owner.graph->entries.end()) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("adaptive.success");diagnostics_.count("adaptive.new_blocks",owner.graph->blocks.size());)
            ++adaptive_promotions;owner.graph->referenced=1;frame_.graph=owner.graph;
            frame_.reload();frame_.epoch=owner.graph->validated_epoch;
            frame_.guest_fpcr=read_floating_control(frame_.shadow);
            return target->second->code.data();
        }
    } catch(const MemoryFault &) {
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("adaptive.speculative_fault");)
        // The speculative caller may have changed; the actual target is the
        // required fetch and must determine the guest-visible fault instead.
    }
    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("adaptive.fallback");)
    return select(pc);
}
#endif
const StaticOperation *ExecutionEngine::tail(uint64_t pc) {
    return tail_from(*frame_.graph->entries.at(pc),pc);
}
const StaticOperation *ExecutionEngine::tail_from(const BasicBlock &block,uint64_t pc) {
    OPERIT_EXEC_DIAGNOSTIC(
        diagnostics_.count("suffix.budget_entries");
        diagnostics_.count(pc==block.pc ? "suffix.full_block":"suffix.continuation");
        diagnostic_transfer(diagnostics_,frame_,"publish.suffix");
    )
    frame_.publish();++materializations;frame_.shadow.pc=pc;
    for(const auto &n:block.instructions) {
        if(n.pc<pc)continue;
        if(frame_.completed==frame_.budget) {
            continuation_entry_=block.pc;continuation_pc_=frame_.shadow.pc;
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("suffix.budget_stop");diagnostic_transfer(diagnostics_,frame_,"reload.suffix_budget");)
            frame_.reload();return nullptr;
        }
        frame_.shadow.pc=n.pc;
        OPERIT_EXEC_DIAGNOSTIC(
            auto &instruction_diagnostics=diagnostics_.instruction("budget_suffix",n.pc,n.instruction,unsigned(StaticCode::Helper));
            ++instruction_diagnostics.calls;
            execution_diagnostics::InstructionFaultScope fault_scope(instruction_diagnostics);
        )
        if(n.semantic(frame_.shadow,*memory_,n.instruction)) {
            OPERIT_EXEC_DIAGNOSTIC(
                ++instruction_diagnostics.retired;diagnostics_.count("suffix.retired");diagnostics_.count("suffix.budget_retired");
                diagnostics_.count("suffix.syscall");diagnostic_transfer(diagnostics_,frame_,"reload.suffix_syscall");
            )
            ++frame_.completed;frame_.reason=StopReason::Syscall;frame_.reload();return nullptr;
        }
        OPERIT_EXEC_DIAGNOSTIC(++instruction_diagnostics.retired;diagnostics_.count("suffix.retired");diagnostics_.count("suffix.budget_retired");)
        ++frame_.completed;
        if(frame_.shadow.pc!=n.pc+4 || memory_->code_epoch.load(std::memory_order_relaxed)!=frame_.epoch) {
            OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count(frame_.shadow.pc!=n.pc+4 ? "suffix.control_exit":"suffix.epoch_exit");)
            break;
        }
    }
    // select() publishes old cached bindings. Refresh them first, since a
    // reference tail has just updated the architectural backing state.
    OPERIT_EXEC_DIAGNOSTIC(diagnostic_transfer(diagnostics_,frame_,"reload.suffix_finish");)
    frame_.reload();return select(frame_.shadow.pc);
}
const StaticOperation *ExecutionEngine::slow(const StaticOperation &op) {
    leave_native();
    ++slow_calls;
    const auto info=operation_info(op);
    OPERIT_EXEC_DIAGNOSTIC(diagnostic_slow(diagnostics_,frame_,op,info);)
    frame_.resume=&op;
    const auto code=info.code;
    switch(code) {
    case StaticCode::Block:case StaticCode::BlockBody:
        frame_.shadow.pc=info.pc;
        if(memory_->code_epoch.load(std::memory_order_relaxed)!=frame_.epoch)return select(info.pc);
        return tail(info.pc);
    case StaticCode::ScheduledLoads: {
        // The private-page scheduling guard has no architectural effects.
        // On a miss execute the original suffix, including partial effects
        // and retirement if the second load faults.
        frame_.completed=frame_.block_base+info.prefix-1;
        for(const auto &block:frame_.graph->blocks)
            if(info.pc>=block->pc && info.pc<=block->instructions.back().pc) {
                try {return tail_from(*block,info.pc);}
                catch(...) {frame_.reload();throw;}
            }
        throw Failure("scheduled load lost its source block");
    }
    case StaticCode::IndirectDispatch:return link(*reinterpret_cast<const IndirectCache*>(op.immediate)->source,frame_.pending_pc);
    case StaticCode::Bridge: {
        const auto &plan=*reinterpret_cast<const BridgePlan*>(op.immediate);
#if OPERIT_ARM64_ADAPTIVE_CALLS
        if(op.entry==hot_bridge_entry() && !plan.metadata.payload)return promote_call(plan,info.pc);
#endif
        OPERIT_EXEC_DIAGNOSTIC(
            diagnostics_.count("bridge.slow_calls");
            execution_diagnostics::ValidationScope validation_site(&diagnostics_,"bridge");
            const char *miss="unclassified";
            if(!plan.generation || *plan.generation!=plan.expected_generation)miss="target_lifetime";
            else if(*plan.target_epoch!=frame_.epoch)miss="target_epoch";
            else if(!frame_.proof_generation)miss="no_serial_scope";
            else if(*plan.target_scope!=UINT64_MAX && *plan.target_scope!=frame_.proof_generation) {
                miss=plan.proof_count ? "scope_changed":"scope_changed_no_proofs";
                for(uint64_t n=0;n<plan.proof_count;++n) {
                    const auto &proof=plan.proofs[n];
                    if(proof.proof->epoch!=frame_.epoch) {miss="proof_epoch";break;}
                    if(proof.proof->generation!=UINT64_MAX && proof.proof->generation!=frame_.proof_generation) {miss="proof_scope";break;}
                    if((proof.proof->verified&proof.mask)!=proof.mask) {miss="proof_lines";break;}
                }
            }
            diagnostics_.count(std::string("bridge.miss.")+miss);
        )
        // A new file-observation scope need not export the source graph and
        // resolve its target again. Revalidate the guarded target in place,
        // then let the assembly bridge transfer the still-resident roles.
        if(frame_.proof_generation && plan.generation && *plan.generation==plan.expected_generation) {
            try {
                OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("bridge.revalidate.attempts");)
                const bool current=plan.graph->current(*memory_,frame_.proof_generation);
                const bool valid=current || plan.graph->validate(*memory_);
                if(valid && plan.graph->validated_epoch==frame_.epoch) {
                    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count(current ? "bridge.revalidate.current_resume":"bridge.revalidate.validated_resume");)
                    return &op;
                }
                OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count(valid ? "bridge.revalidate.frame_epoch_disagrees":"bridge.revalidate.invalid");)
            } catch(const MemoryFault &) {
                OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("bridge.revalidate.memory_fault");)
                // Rebuild from the required entry if a speculative successor
                // became inaccessible, just as in find().
            }
        }
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("bridge.relink");)
        return link(*plan.source,info.pc);
    }
    case StaticCode::Next:return link(op,op.immediate);
    case StaticCode::Jump:return link(op,op.immediate);
    case StaticCode::Call:return link(op,info.pc+info.node->instruction.immediate);
    case StaticCode::Indirect:case StaticCode::IndirectCall: {
        return link(op,frame_.pending_pc);
    }
    case StaticCode::Branch:case StaticCode::CompareBranch:case StaticCode::TestBranch:
        return link(op,info.pc+info.node->instruction.immediate);
    case StaticCode::CompareImmediateBranch:
        // Lowering only fuses consecutive instructions in the same block.
        return link(op,info.pc+4+(info.node+1)->instruction.immediate);
    default:break;
    }
    frame_.completed=frame_.block_base+info.prefix-1;
    const auto &node=*info.node;const auto &e=node.effects;
    OPERIT_EXEC_DIAGNOSTIC(
        auto &instruction_diagnostics=diagnostics_.instruction(code==StaticCode::Helper ? execution_diagnostics::helper_reason(node.instruction):"native_memory_guard",node.pc,node.instruction,unsigned(code));
        diagnostic_transfer(diagnostics_,frame_,"publish.semantic",e.gp_read|e.gp_write,e.vector_read|e.vector_write);
    )
    frame_.publish(e.gp_read|e.gp_write,e.vector_read|e.vector_write);
    frame_.shadow.pc=info.pc;
    bool syscall;
    try {syscall=node.semantic(frame_.shadow,*memory_,node.instruction);}
    catch(...) {
        OPERIT_EXEC_DIAGNOSTIC(++instruction_diagnostics.faults;diagnostics_.count("semantic.fault");diagnostic_transfer(diagnostics_,frame_,"reload.semantic_fault",e.gp_write,e.vector_write);)
        frame_.reload(e.gp_write,e.vector_write);throw;
    }
    OPERIT_EXEC_DIAGNOSTIC(++instruction_diagnostics.retired;diagnostics_.count("semantic.retired");diagnostic_transfer(diagnostics_,frame_,"reload.semantic",e.gp_write,e.vector_write);)
    ++frame_.completed;frame_.reload(e.gp_write,e.vector_write);
    frame_.guest_fpcr=read_floating_control(frame_.shadow);
    if(syscall) {OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("semantic.syscall");) frame_.reason=StopReason::Syscall;return nullptr;}
    if(frame_.completed==frame_.budget || e.control ||
       memory_->code_epoch.load(std::memory_order_relaxed)!=frame_.epoch) {
        OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count(frame_.completed==frame_.budget ? "semantic.exit.budget":e.control ? "semantic.exit.control":"semantic.exit.epoch");)
        return select(frame_.shadow.pc);
    }
    return &op+1;
}
extern "C" const StaticOperation *operit_static_slow(ExecutionFrame *frame,const StaticOperation *op) noexcept {
    // Only a block header has index zero. Capture before slow/select may reclaim
    // the record, so failure publication needs no expired cold metadata.
    const bool block_header=op->index==0;
    try {
        const auto next=frame->engine->slow(*op);
        if(next)frame->engine->enter_native();
        return next;
    }
    catch(...) {
        if(frame->graph && !block_header) {
            OPERIT_EXEC_DIAGNOSTIC(if(frame->graph->diagnostics)diagnostic_transfer(*frame->graph->diagnostics,*frame,"publish.failure");)
            frame->publish();
        }
        frame->exception=std::current_exception();frame->reason=StopReason::Failure;return nullptr;}
}
Execution ExecutionEngine::run(CpuState &cpu,Memory &memory,uint64_t budget,FixedComponents *intrinsics) {
    OPERIT_EXEC_DIAGNOSTIC(
        execution_diagnostics::ActiveScope diagnostics_scope(diagnostics_);
        if(!diagnostics_.native_initialized) {
            diagnostics_.native_initialized=true;
            const auto shift=diagnostic_environment("OPERIT_EXECUTION_DIAGNOSTICS_SAMPLE_SHIFT",8,30);
            const auto sets=diagnostic_environment("OPERIT_EXECUTION_DIAGNOSTICS_SITE_SETS",16384,1u<<20);
            diagnostics_.native_sampling=execution_diagnostics_reset(frame_,shift,sets);
            diagnostics_.count(diagnostics_.native_sampling ? "native.sampling_initialized":"native.sampling_initialization_failed");
        }
        if(diagnostics_.clock_pending && diagnostics_.clock_cpu==&cpu) {
            diagnostics_.clock_pending=false;diagnostics_.clock_result=cpu.x[0];diagnostics_.guest_pc=cpu.pc;
            dump_diagnostics("guest_clock_gettime_resume");
        }
        diagnostics_.count("run.calls");diagnostics_.last_budget=budget;
        const auto diagnose_finish=[&](bool failed) {
            diagnostics_.count("run.retired",frame_.completed);diagnostics_.count("native.block_entries",frame_.block_hits);
            diagnostics_.count(failed ? "run.failure":frame_.reason==StopReason::Syscall ? "run.syscall":"run.budget");
            diagnostics_.guest_pc=cpu.pc;diagnostics_.last_syscall=frame_.reason==StopReason::Syscall ? cpu.x[8]:0;
            if(!failed && frame_.reason==StopReason::Syscall && cpu.x[8]==113 && cpu.x[0]==1 && execution_diagnostics::clock_enabled()) {
                diagnostics_.clock_pending=true;diagnostics_.clock_cpu=&cpu;++diagnostics_.clock_sequence;
                diagnostics_.clock_pc=cpu.pc-4;diagnostics_.clock_arg0=cpu.x[0];diagnostics_.clock_arg1=cpu.x[1];diagnostics_.clock_result=0;
                dump_diagnostics("guest_clock_gettime_enter");
            }
        };
    )
    Memory::ExecutionScope scope(memory);
    if(intrinsics && !intrinsics->active())intrinsics=nullptr;
    const auto generation=intrinsics ? intrinsics->generation():0;
    if(memory_!=&memory || memory_identity_!=memory.identity() || intrinsics_!=intrinsics || intrinsic_generation_!=generation) {
        OPERIT_EXEC_DIAGNOSTIC(
            diagnostics_.count("cache.reset");diagnostics_.count("cache.reset.graphs",graphs_.size());
            if(memory_!=&memory)diagnostics_.count("cache.reset.memory_pointer");
            if(memory_identity_!=memory.identity())diagnostics_.count("cache.reset.memory_identity");
            if(intrinsics_!=intrinsics)diagnostics_.count("cache.reset.intrinsics_pointer");
            if(intrinsic_generation_!=generation)diagnostics_.count("cache.reset.intrinsics_generation");
        )
        continuation_pc_=0;entries_.clear();graphs_.clear();wasm.clear();++cache_generation_;memory_=&memory;memory_identity_=memory.identity();intrinsics_=intrinsics;intrinsic_generation_=generation;
#if OPERIT_ARM64_ADAPTIVE_CALLS
        hot_calls_.clear();
#endif
    }
    frame_.shadow=cpu;frame_.graph=nullptr;frame_.completed=0;frame_.budget=budget;frame_.exception=nullptr;
    frame_.reason=StopReason::Budget;frame_.engine=this;frame_.memory=&memory;frame_.block_hits=0;
    frame_.file_generation=memory.execution_generation();frame_.proof_generation=memory.code_proof_generation();
    frame_.file_observations=memory.file_observations();
    const auto view=memory.execution_view();frame_.fast_pages=view.pages;frame_.fast_first=view.first;frame_.fast_second=view.second;frame_.mapping_epoch=view.mapping_epoch;frame_.code_epoch=&memory.code_epoch;
    try {
        const auto resume_pc=continuation_pc_,resume_entry=continuation_entry_;
        continuation_pc_=0;
        const StaticOperation *entry=nullptr;bool resumed=false;
#if OPERIT_ARM64_WASM
        int wasm_reason=WasmRegion::Branch;
        const auto wasm_retired=wasm.run(frame_.shadow,memory,budget,wasm_reason,intrinsics);
        frame_.completed=wasm_retired;
        if(wasm_retired) {
            if(wasm_reason==WasmRegion::Syscall)frame_.reason=StopReason::Syscall;
            else if(frame_.completed<budget)entry=select(frame_.shadow.pc);
            resumed=true;
        }
#endif
        if(!resumed && cpu.pc==resume_pc && resume_pc!=resume_entry && (!intrinsics_ || !intrinsics_->contains(cpu.pc))) {
            const auto found=entries_.find(resume_entry);
            if(found!=entries_.end()) {
                auto &owner=*found->second.graph;auto &block=*found->second.block;
                bool valid=false;
                OPERIT_EXEC_DIAGNOSTIC(execution_diagnostics::ValidationScope validation_site(&diagnostics_,"continuation");)
                try {valid=owner.current(memory,frame_.proof_generation) || owner.validate(memory);}
                catch(const MemoryFault &) {} // Rebuild from the required PC below.
                if(valid && std::any_of(block.instructions.begin(),block.instructions.end(),
                                       [&](const auto &node){return node.pc==cpu.pc;})) {
                    OPERIT_EXEC_DIAGNOSTIC(diagnostics_.count("continuation.resumed");)
                    frame_.graph=&owner;owner.referenced=1;
                    OPERIT_EXEC_DIAGNOSTIC(diagnostic_transfer(diagnostics_,frame_,"reload.continuation");)
                    frame_.reload();frame_.epoch=owner.validated_epoch;
                    entry=tail_from(block,cpu.pc);resumed=true;
                }
            }
        }
        if(!resumed)entry=select(cpu.pc);
        if(entry) {
            enter_native();operit_static_execute(&frame_,entry);leave_native();
        }
        // On a semantic failure its backing state may contain partial effects;
        // reload happened in slow(), so publishing preserves those effects.
        if(frame_.exception)std::rethrow_exception(frame_.exception);
        OPERIT_EXEC_DIAGNOSTIC(diagnostic_transfer(diagnostics_,frame_,"publish.run_exit");)
        frame_.publish();cpu=frame_.shadow;
    } catch(...) {
        leave_native();
        // tail() operates directly on backing state, including partial LDP
        // effects. Do not overwrite it with stale cached values after a fault.
        cpu=frame_.shadow;executed_instructions+=frame_.completed;cache_hits+=frame_.block_hits;
        OPERIT_EXEC_DIAGNOSTIC(diagnose_finish(true);)
        throw;
    }
    executed_instructions+=frame_.completed;cache_hits+=frame_.block_hits;
    OPERIT_EXEC_DIAGNOSTIC(diagnose_finish(false);)
#if OPERIT_ARM64_WASM
    wasm.record(cpu,executed_instructions,frame_.completed,frame_.reason==StopReason::Syscall);
#endif
    return {frame_.reason==StopReason::Syscall ? Trap::Syscall:Trap::Budget,frame_.completed};
}
}
