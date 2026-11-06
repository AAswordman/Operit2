#include "cpu/aarch64/optimizer.h"
#include <algorithm>
namespace operit::arm64 {
void optimize_graph(BlockGraph &graph) {
    // Only pure, legally decoded integer operations participate. Every fault,
    // helper with undeclared effects and control edge closes the DCE window.
    const auto pure=[](const InstructionNode &n) {
        if(n.instruction.encoding==Encoding::General || n.effects.memory || n.effects.control)return false;
        switch(n.instruction.op) {
        case Op::Hint:case Op::Address:case Op::MoveWide:case Op::AddImmediate:case Op::AddRegister:
        case Op::AddCarry:case Op::LogicalImmediate:case Op::LogicalRegister:case Op::Bitfield:
        case Op::Extract:case Op::TwoSource:case Op::OneSource:case Op::Multiply:case Op::MultiplyHigh:
        case Op::ConditionalSelect:case Op::ConditionalCompare:return true;
        default:return false;
        }
    };
    // A compile-time evaluator is independent of the assembly operand ABI.
    // These selected semantics never access memory. No guest code is executed
    // by this evaluator and no executable host memory is allocated.
    static thread_local Memory unused_memory(0);
    for(auto &block:graph.blocks) {
        uint32_t known=0;bool known_flags=false;CpuState constants;
        for(auto &n:block->instructions) {
            if(pure(n) && (n.effects.gp_read&known)==n.effects.gp_read && (!n.effects.flags_read || known_flags)) {
                constants.pc=n.pc;select_semantic(n.instruction)(constants,unused_memory,n.instruction);
                known|=n.effects.gp_write;
                if(n.effects.flags_write)known_flags=true;
                const auto writes=n.effects.gp_write;
                if(writes && !(writes&(writes-1)) && !n.effects.flags_write) {
                    const auto reg=unsigned(__builtin_ctz(writes));n.constant_result=true;n.constant=constants.get(reg,64,true);
                }
            } else {
                known&=~n.effects.gp_write;
                if(n.effects.flags_write)known_flags=false;
            }
        }
        auto &nodes=block->instructions;
        for(size_t n=0;n<nodes.size();++n) {
            auto &node=nodes[n];const auto writes=node.effects.gp_write;
            if(node.instruction.encoding==Encoding::Noop) {node.dead_result=true;++graph.eliminated;continue;}
            if(!pure(node) || !writes || node.effects.flags_write)continue;
            for(size_t next=n+1;next<nodes.size();++next) {
                const auto &following=nodes[next];
                if(!pure(following) || (following.effects.gp_read&writes))break;
                if((following.effects.gp_write&writes)==writes) {node.dead_result=true;++graph.eliminated;break;}
            }
        }
        bool flags_needed=true;
        for(auto it=nodes.rbegin();it!=nodes.rend();++it) {
            if(!pure(*it)) {flags_needed=true;continue;}
            it->discard_flags=it->effects.flags_write && !flags_needed;
            flags_needed=it->effects.flags_read || (flags_needed && !it->effects.flags_write);
        }
    }
}
}
