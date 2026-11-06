#include "cpu/aarch64/block_graph.h"
#include "cpu/aarch64/execution_frame.h"
#include <algorithm>
namespace operit::arm64 {
namespace {
struct Operand {unsigned role;uint16_t home;};
Operand gp(unsigned reg,bool stack=false) {
    if(reg==31 && !stack) return {7,0};
    auto found=std::find(resident_gp.begin(),resident_gp.end(),reg);
    const auto offset=offsetof(ExecutionFrame,shadow)+(reg==31 ? offsetof(CpuState,sp) : offsetof(CpuState,x)+reg*8);
    return {found==resident_gp.end() ? 6u : unsigned(found-resident_gp.begin()),uint16_t(offset)};
}
Operand vec(unsigned reg) {
    return {reg<resident_vector.size() ? reg:8u,
            uint16_t(offsetof(ExecutionFrame,shadow)+offsetof(CpuState,v)+reg*16)};
}
}
void lower_graph(BlockGraph &g) {
    using S=StaticCode;using E=Encoding;
    uint64_t reservation=0;
    for(auto bp=g.blocks.rbegin();bp!=g.blocks.rend();++bp) {
        auto &block=**bp;auto &out=block.code;
        reservation+=block.instructions.size();
        // Both arrays stop growing before any executable pointers are exposed.
        const auto append=[&](StaticOperation op,const InstructionNode &node,S code,uint16_t prefix,uint64_t value=0) {
            if(out.size()>=OP_DETACHED_INDEX)throw Failure("static block operation index overflow");
            op.index=uint16_t(out.size());
            if(operation_control_edge(code))op.b=prefix;
            out.push_back(op);block.metadata.push_back(make_operation_metadata(node,code,prefix,value));
        };
        StaticOperation header;header.entry=static_entry(S::Block);
        // Reserve the forward DAG once; only backedges and external entries
        // revisit this header. Interior entries retain precise block prefixes.
        header.immediate=reservation;append(header,block.instructions.front(),S::Block,0);
        header.entry=static_entry(S::BlockBody);append(header,block.instructions.front(),S::BlockBody,0);
        for(size_t n=0;n<block.instructions.size();++n) {
            auto &node=block.instructions[n];g.source[node.pc]=&node;
            if(node.dead_result) {
                OPERIT_EXEC_DIAGNOSTIC(if(g.diagnostics)++g.diagnostics->instruction("eliminated",node.pc,node.instruction,unsigned(S::Nop)).compiled;)
                continue;
            }
            auto i=node.instruction; StaticOperation op;op.immediate=i.immediate;
            auto a=gp(i.rn),b=gp(i.rm),d=gp(i.rd);unsigned flags=0;
            S code=S::Helper; uint64_t meta=0;
            bool prepare=false;Operand prepared{7,0};uint64_t prepare_meta=0,prepare_shift=0;unsigned prepare_bits=i.width;
            if(i.encoding!=E::General) switch(i.op) {
            case Op::Hint:code=S::Nop;break;
            case Op::Address:code=S::Move;b={7,0};op.immediate=(i.flags ? align_down(node.pc)+(i.immediate<<12) : node.pc+i.immediate);break;
            case Op::MoveWide:
                if(i.encoding==E::Keep32 || i.encoding==E::Keep64) { code=S::Keep;a=d;meta=(i.raw>>21)&3; }
                else code=S::Move;
                b={7,0};break;
            case Op::AddImmediate:
                code=i.flags&2 ? S::Sub:S::Add;flags=i.flags&1;a=gp(i.rn,true);d=gp(i.rd,!flags);b={7,0};break;
            case Op::AddRegister:
                flags=i.flags&1;
                if(!(i.flags&decoded::extended) && !i.shift)code=i.flags&2 ? S::Sub:S::Add;
                else {
                    code=i.flags&2 ? S::SubPrepared:S::AddPrepared;prepare=true;prepared=b;prepare_shift=i.shift;
                    if(i.flags&decoded::extended) {
                        a=gp(i.rn,true);d=gp(i.rd,!flags);
                        prepare_meta=(uint64_t(64-i.immediate)<<10)|(1u<<9)|((i.flags&decoded::signed_extension) ? 1u<<8:0);
                    } else prepare_meta=uint64_t((i.flags>>decoded::shift_type)&3)<<6;
                }
                break;
            case Op::AddCarry:code=i.flags&2 ? S::Sbc:S::Adc;flags=i.flags&1;break;
            case Op::LogicalImmediate:case Op::LogicalRegister:
                {
                    const S choices[]={S::And,S::Or,S::Xor,S::And};code=choices[i.flags&3];flags=(i.flags&3)==3;
                    if(i.op==Op::LogicalImmediate)b={7,0};
                    else if((i.flags&16) || i.shift) {
                        const S prepared_codes[]={S::AndPrepared,S::OrPrepared,S::XorPrepared,S::AndPrepared};code=prepared_codes[i.flags&3];
                        prepare=true;prepared=b;prepare_shift=i.shift;prepare_meta=(uint64_t((i.flags>>2)&3)<<6)|((i.flags&16) ? 1u<<16:0);
                    }
                }break;
            case Op::ConditionalCompare:
                code=S::ConditionalCompare;flags=i.shift*2+(i.flags&1);meta=i.raw&15;
                d={7,0};if(i.flags&2){b={7,0};op.immediate=i.rm;}break;
            case Op::System:
                switch(i.encoding) {
                case E::ReadTls:code=S::ReadTls;break;
                case E::WriteTls:code=S::WriteTls;a=gp(i.rd);break;
                case E::ReadFlags:code=S::ReadFlags;break;
                case E::WriteFlags:code=S::WriteFlags;a=gp(i.rd);break;
                default:break;
                }break;
            case Op::TwoSource:
                switch(i.flags) {case 2:code=S::Udiv;break;case 3:code=S::Sdiv;break;case 8:code=S::Lsl;break;case 9:code=S::Lsr;break;case 10:code=S::Asr;break;case 11:code=S::Ror;break;default:break;}break;
            case Op::Multiply:
                if(!(i.flags&7)) {
                    if(!i.flags && i.ra==31)code=S::Mul;
                    else {code=i.flags&8 ? S::Msub:S::Madd;prepare=true;prepared=gp(i.ra);}
                }break;
            case Op::MultiplyHigh:code=i.flags ? S::UmulHigh:S::SmulHigh;break;
            case Op::Bitfield:code=S::Bitfield;flags=i.flags;op.extra=i.auxiliary;meta=i.shift|uint64_t((i.raw>>10)&63)<<6;break;
            case Op::Extract:code=S::Extract;op.immediate=i.shift;break;
            case Op::OneSource:code=S::Unary;flags=i.flags;break;
            case Op::ConditionalSelect:code=S::Select;op.immediate=i.shift;meta=i.flags;break;
            case Op::Branch:code=(i.raw>>31) ? S::Call:S::Jump;op.immediate=node.pc+i.immediate;d=gp(30);break;
            case Op::BranchRegister:code=i.encoding==E::BranchLinkRegister ? S::IndirectCall:S::Indirect;a=gp(i.rn);d=gp(30);break;
            case Op::ConditionalBranch:code=S::Branch;meta=i.flags;break;
            case Op::CompareBranch:code=S::CompareBranch;a=gp(i.rd);meta=i.flags;break;
            case Op::TestBranch:code=S::TestBranch;a=gp(i.rd);meta=i.flags;op.immediate=i.shift;break;
            case Op::LoadStore:
                {
                    if(i.encoding>=E::Load8 && i.encoding<=E::Store64) {
                        code=i.encoding<=E::Load64 ? (i.flags&decoded::signed_load ? S::SignedLoad:S::Load):S::Store;
                        flags=(code==S::SignedLoad && (i.flags&decoded::word_result)) ? 1:0;
                        a=gp(i.rn,true);b=d;meta=1u<<i.shift;
                    } else if(i.encoding>=E::VectorLoad32 && i.encoding<=E::VectorStore128) {
                        code=i.encoding<=E::VectorLoad128 ? S::VectorLoad:S::VectorStore;
                        a=gp(i.rn,true);d=vec(i.rd);b=d;meta=1u<<i.shift;
                    }
                    if(code!=S::Helper && (i.flags&3)==decoded::register_offset) {
                        prepare=true;prepared=gp(i.rm);prepare_bits=64;
                        prepare_shift=(i.flags&decoded::offset_scaled) ? i.shift:0;
                        if(i.flags&decoded::offset_word)prepare_meta=(uint64_t{32}<<10)|(1u<<9)|((i.flags&decoded::offset_signed) ? 1u<<8:0);
                        switch(code) {
                        case S::Load:code=S::LoadPrepared;break;case S::Store:code=S::StorePrepared;break;
                        case S::SignedLoad:code=S::SignedLoadPrepared;break;
                        case S::VectorLoad:code=S::VectorLoadPrepared;break;case S::VectorStore:code=S::VectorStorePrepared;break;
                        default:break;
                        }
                        op.immediate=0;
                    } else if(code!=S::Helper && (i.flags&3)>=decoded::post_index) {
                        meta|=uint64_t(i.flags&3)<<32;
                        switch(code) {
                        case S::Load:code=S::LoadWriteback;break;case S::Store:code=S::StoreWriteback;break;
                        case S::SignedLoad:code=S::SignedLoadWriteback;break;
                        case S::VectorLoad:code=S::VectorLoadWriteback;break;case S::VectorStore:code=S::VectorStoreWriteback;break;
                        default:break;
                        }
                    }
                }break;
            case Op::LoadPair: {
                const bool vector=i.raw&(1u<<26),load=i.raw&(1u<<22);
                a=gp(i.rn,true);b=gp(i.ra);
                if(vector) {d=vec(i.rd);b=vec(i.ra);code=load ? S::VectorPairLoad:S::VectorPairStore;}
                else {code=load ? S::PairLoad:S::PairStore;flags=(i.raw>>30)==1;}
                meta=(vector ? 4u<<(i.raw>>30):((i.raw>>30)==2 ? 8u:4u))|(uint64_t((i.raw>>23)&3)<<32);
                break;
            }
            case Op::Simd:
                switch(i.encoding) {
                case E::RegisterToVector32:case E::RegisterToVector64:case E::RegisterToVectorHigh:
                    code=S::GpToVector;flags=i.encoding==E::RegisterToVectorHigh;break;
                case E::VectorToRegister32:case E::VectorToRegister64:case E::VectorHighToRegister:
                    code=S::VectorToGp;flags=i.encoding==E::VectorHighToRegister;break;
                case E::VectorImmediate:code=S::VectorImmediate;i.width=(i.raw&(1u<<22)) ? 64:32;break;
                case E::ScalarUnary32:case E::ScalarUnary64:code=S::FUnary;flags=i.flags;break;
                case E::ScalarUnsignedToDouble64:code=S::UnsignedToDouble;break;
                case E::ScalarAdd64:code=S::Fadd;break;case E::ScalarSubtract64:code=S::Fsub;break;
                case E::ScalarMultiply64:code=S::Fmul;break;case E::ScalarDivide64:code=S::Fdiv;break;
                case E::ScalarSqrt64:code=S::Fsqrt;break;
                case E::ScalarFma64:code=S::Fma;flags=((i.raw>>15)&1)|((i.raw>>20)&2);meta=vec(i.ra).role;op.immediate=vec(i.ra).home;break;
                case E::ScalarCompare64:code=S::Fcmp;flags=(i.raw>>3)&1;break;
                case E::ScalarSelect64:code=S::Fselect;meta=i.flags;break;
                default:break;
                }
                if(code!=S::Helper) {
                    a=vec(i.rn);b=vec(i.rm);d=vec(i.rd);
                    if(code==S::GpToVector || code==S::UnsignedToDouble)a=gp(i.rn);
                    if(code==S::VectorToGp)d=gp(i.rd);
                }break;
            default:break;
            }
            if(node.constant_result) {
                code=S::Move;op.immediate=node.constant;b={7,0};meta=0;flags=0;prepare=false;
                const auto reg=unsigned(__builtin_ctz(node.effects.gp_write));d=gp(reg,true);
            }
            if(node.discard_flags)flags=0;
            // A fused compare/branch remains two retired instructions. The
            // block guard reserves their whole budget before either executes.
            if(code==S::Sub && flags && i.rd==31 && b.role==7 && n+1<block.instructions.size() &&
               block.instructions[n+1].instruction.op==Op::ConditionalBranch) {
                const auto &branch=block.instructions[++n];g.source[branch.pc]=&branch;
                code=S::CompareImmediateBranch;meta=branch.instruction.flags;
                op.target=reinterpret_cast<const void*>(branch.pc+branch.instruction.immediate);
                ++g.fused;
                OPERIT_EXEC_DIAGNOSTIC(if(g.diagnostics)++g.diagnostics->instruction("fused_branch",branch.pc,branch.instruction,unsigned(S::CompareImmediateBranch)).compiled;)
            }
            if(code==S::Branch || code==S::CompareBranch || code==S::TestBranch) {
                op.target=reinterpret_cast<const void*>(node.pc+i.immediate);
                if(code!=S::TestBranch)op.immediate=node.pc+i.immediate;
            }
            if(prepare) {
                StaticOperation preparation;preparation.a=prepared.home;
                preparation.immediate=prepare_shift;
                unsigned form=unsigned((prepare_meta>>6)&3);
                if(prepare_meta&(1u<<9)) {
                    const auto width=64-unsigned((prepare_meta>>10)&63);
                    const unsigned index=width==8 ? 0:width==16 ? 1:width==32 ? 2:3;
                    form=4+index*2+unsigned((prepare_meta>>8)&1);
                }
                if(prepare_meta&(1u<<16))form+=12;
                const bool identity=prepare_shift==0 && prepare_meta==0;
                preparation.entry=static_entry(identity ? S::Operand:S::Prepare,prepare_bits,7,prepared.role,7,identity ? 0:form);
                // Identity preparations retain the original diagnostic Prepare kind.
                append(preparation,node,S::Prepare,0,prepare_meta);
            }
            op.a=a.home;op.b=b.home;op.d=d.home;
            unsigned bits=i.op==Op::Address ? 64:i.width;
            if(code==S::Load || code==S::Store || code==S::SignedLoad || code==S::VectorLoad || code==S::VectorStore ||
               code==S::PairLoad || code==S::PairStore || code==S::VectorPairLoad || code==S::VectorPairStore ||
               code==S::LoadPrepared || code==S::StorePrepared || code==S::SignedLoadPrepared || code==S::VectorLoadPrepared || code==S::VectorStorePrepared || code==S::LoadWriteback || code==S::StoreWriteback ||
               code==S::SignedLoadWriteback || code==S::VectorLoadWriteback || code==S::VectorStoreWriteback)bits=unsigned(meta)*8;
            if(code==S::Branch || code==S::CompareBranch || code==S::TestBranch || code==S::CompareImmediateBranch)flags=unsigned(meta);
            // A local edge owns its target block for the graph's lifetime.
            // Fuse that edge with the target guard, avoiding a dispatch whose
            // only purpose would be to enter the Block handler.
            if((code==S::Jump || code==S::Call) && g.entries.count(op.immediate))flags|=1;
            if(code==S::Branch || code==S::CompareBranch || code==S::TestBranch || code==S::CompareImmediateBranch) {
                const auto target=(code==S::TestBranch || code==S::CompareImmediateBranch) ? uint64_t(op.target):op.immediate;
                if(g.entries.count(target) && g.entries.count(block.instructions[n].pc+4))
                    flags|=(code==S::Branch || code==S::CompareImmediateBranch) ? 16:2;
            }
            op.entry=static_entry(code,bits,d.role,a.role,b.role,flags);
            // Every native metadata consumer keeps one direct record load. The
            // original semantic node and full diagnostic metadata stay cold.
            switch(code) {
            case S::Keep:case S::Bitfield:op.b=uint16_t(meta);break;
            case S::ConditionalCompare:op.d=uint16_t(meta);break;
            case S::Select:case S::Fma:op.extra=meta;break;
            case S::Fselect:op.immediate=meta;break;
            case S::PairLoad:case S::PairStore:case S::VectorPairLoad:case S::VectorPairStore:
            case S::LoadWriteback:case S::StoreWriteback:case S::SignedLoadWriteback:
            case S::VectorLoadWriteback:case S::VectorStoreWriteback:op.extra=(meta>>32)&0xff;break;
            default:break;
            }
            // Direct Call's guest target is recovered from its cold node when
            // linking. Both call forms retain a full-width return address.
            if(code==S::Call || code==S::IndirectCall)op.immediate=node.pc+4;
            OPERIT_EXEC_DIAGNOSTIC(if(g.diagnostics) {
                const auto kind=code==S::Helper ? execution_diagnostics::helper_reason(node.instruction):"lowered";
                ++g.diagnostics->instruction(kind,node.pc,node.instruction,unsigned(code)).compiled;
            })
            append(op,node,code,uint16_t(n+1),meta);
        }
        // One explicit edge gives ordinary and conditional fallthrough a
        // stable target; no C++ ABI transition at an internal link.
        const auto &last=block.instructions.back();
        StaticOperation edge;edge.immediate=last.pc+4;
        edge.entry=static_entry(S::Next,64,7,7,7,g.entries.count(edge.immediate) ? 1:0);
        append(edge,last,S::Next,uint16_t(block.instructions.size()));
        out.front().target=block.metadata.data();
    }
    for(auto &block:g.blocks)for(auto &op:block->code) {
        const auto info=operation_info(op);const auto code=info.code;
        if(code==S::Jump || code==S::Call || code==S::Next || code==S::Branch || code==S::CompareBranch || code==S::TestBranch || code==S::CompareImmediateBranch) {
            const auto pc=code==S::Call ? info.pc+info.node->instruction.immediate:
                (code==S::TestBranch || code==S::CompareImmediateBranch) ? uint64_t(op.target):op.immediate;
            auto found=g.entries.find(pc);op.target=found==g.entries.end() ? nullptr :
                found->second->code.data()+(pc>info.node->pc);
            OPERIT_EXEC_DIAGNOSTIC(if(g.diagnostics)g.diagnostics->count(found==g.entries.end() ? "compile.links.external":"compile.links.local");)
            // TestBranch keeps its bit in immediate; b remains edge retirement.
            // A fused compare keeps its full immediate and resolves the branch
            // guest target from the second original instruction on a slow miss.
        }
    }
}
}
