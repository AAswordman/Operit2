#include "cpu/aarch64/memory_effects.h"
namespace operit::arm64 {
Effects instruction_effects(const Instruction &i) {
    Effects e{};
    auto read=[&](unsigned r,bool sp=false){ if(r<31 || sp) e.gp_read|=uint32_t{1}<<r; };
    auto write=[&](unsigned r,bool sp=false){ if(r<31 || sp) e.gp_write|=uint32_t{1}<<r; };
    const auto raw=i.raw;
    switch(i.op) {
    case Op::Hint: case Op::Barrier: case Op::Fault: case Op::Unknown: break;
    case Op::Svc: e.control=true; break;
    case Op::Address: write(i.rd); break;
    case Op::MoveWide: if(i.encoding==Encoding::Keep32 || i.encoding==Encoding::Keep64) read(i.rd); write(i.rd); break;
    case Op::AddImmediate: read(i.rn,true); write(i.rd,!(i.flags&decoded::set_flags)); e.flags_write=i.flags&decoded::set_flags; break;
    case Op::AddRegister: read(i.rn,i.flags&decoded::extended); read(i.rm); write(i.rd,(i.flags&decoded::extended)&&!(i.flags&decoded::set_flags)); e.flags_write=i.flags&1; break;
    case Op::AddCarry: read(i.rn);read(i.rm);write(i.rd); e.flags_read=true;e.flags_write=i.flags&1;break;
    case Op::LogicalImmediate: read(i.rn);write(i.rd);e.flags_write=((raw>>29)&3)==3;break;
    case Op::LogicalRegister: read(i.rn);read(i.rm);write(i.rd);e.flags_write=((raw>>29)&3)==3;break;
    case Op::Bitfield: read(i.rn);read(i.rd);write(i.rd);break;
    case Op::Extract: case Op::TwoSource: case Op::MultiplyHigh: read(i.rn);read(i.rm);write(i.rd);break;
    case Op::OneSource: read(i.rn);write(i.rd);break;
    case Op::Multiply: read(i.rn);read(i.rm);read(i.ra);write(i.rd);break;
    case Op::ConditionalSelect: read(i.rn);read(i.rm);write(i.rd);e.flags_read=true;break;
    case Op::ConditionalCompare: read(i.rn); if(!(raw&(1u<<11))) read(i.rm);e.flags_read=e.flags_write=true;break;
    case Op::Branch: if(raw>>31) write(30);e.control=true;break;
    case Op::BranchRegister: read(i.rn);if(i.encoding==Encoding::BranchLinkRegister)write(30);e.control=true;break;
    case Op::ConditionalBranch: e.flags_read=true;e.control=true;break;
    case Op::CompareBranch: case Op::TestBranch:read(i.rd);e.control=true;break;
    case Op::VectorMemory:
        e.memory=true;read(i.rn,true);if(raw&(1u<<23)){write(i.rn,true);read(i.rm);}
        e.vector_read=e.vector_write=UINT32_MAX;break;
    case Op::LoadStore: case Op::LoadPair:
        e.memory=true;read(i.rn,true);if((i.flags&3)==decoded::register_offset)read(i.rm);
        if((i.flags&3)==decoded::post_index || (i.flags&3)==decoded::pre_index)write(i.rn,true);
        if(i.op==Op::VectorMemory || (raw&(1u<<26))) { e.vector_read=e.vector_write=UINT32_MAX; }
        else if(i.op==Op::LoadPair ? bool(raw&(1u<<22)) : bool((raw>>22)&3)) {write(i.rd);if(i.op==Op::LoadPair)write(i.ra);}
        else {read(i.rd);if(i.op==Op::LoadPair)read(i.ra);} break;
    case Op::LoadLiteral:e.memory=true;if(raw&(1u<<26))e.vector_write=uint32_t{1}<<i.rd;else write(i.rd);break;
    case Op::Simd:
        switch(i.encoding) {
        case Encoding::ScalarFma64:e.vector_read=(uint32_t{1}<<i.rn)|(uint32_t{1}<<i.rm)|(uint32_t{1}<<i.ra);e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::ScalarAdd64:case Encoding::ScalarSubtract64:case Encoding::ScalarMultiply64:case Encoding::ScalarDivide64:
            e.vector_read=(uint32_t{1}<<i.rn)|(uint32_t{1}<<i.rm);e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::ScalarUnary32:case Encoding::ScalarUnary64:
        case Encoding::ScalarSqrt64:e.vector_read=uint32_t{1}<<i.rn;e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::ScalarCompare64:e.vector_read=uint32_t{1}<<i.rn;if(!(raw&8))e.vector_read|=uint32_t{1}<<i.rm;e.flags_write=true;break;
        case Encoding::ScalarSelect64:e.vector_read=(uint32_t{1}<<i.rn)|(uint32_t{1}<<i.rm);e.vector_write=uint32_t{1}<<i.rd;e.flags_read=true;break;
        case Encoding::ScalarUnsignedToDouble64:case Encoding::ScalarSignedToDouble64:
        case Encoding::ScalarUnsignedToDouble32:case Encoding::ScalarSignedToDouble32:
            read(i.rn);e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::ScalarDoubleToSigned32:case Encoding::ScalarDoubleToSigned64:
            e.vector_read=uint32_t{1}<<i.rn;write(i.rd);break;
        case Encoding::VectorImmediate:e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::VectorLogical:
            e.vector_read=(uint32_t{1}<<i.rn)|(uint32_t{1}<<i.rm);
            if(i.flags>=5)e.vector_read|=uint32_t{1}<<i.rd;
            e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::VectorToRegister32:case Encoding::VectorToRegister64:case Encoding::VectorHighToRegister:
            e.vector_read=uint32_t{1}<<i.rn;write(i.rd);break;
        case Encoding::RegisterToVector32:case Encoding::RegisterToVector64:
            read(i.rn);e.vector_write=uint32_t{1}<<i.rd;break;
        case Encoding::RegisterToVectorHigh:read(i.rn);e.vector_read=e.vector_write=uint32_t{1}<<i.rd;break;
        default:e.vector_read=e.vector_write=UINT32_MAX;e.gp_read=e.gp_write=UINT32_MAX;e.flags_read=e.flags_write=true;break;
        }break;
    case Op::Cache:read(i.rd);e.memory=true;break;
    case Op::Exclusive:
        e.memory=true;
        if(i.encoding==Encoding::LoadAcquire) {read(i.rn,true);write(i.rd);}
        else e.gp_read=e.gp_write=UINT32_MAX;
        break;
    case Op::System:
        switch(i.encoding) {
        case Encoding::ReadTls:write(i.rd);break;
        case Encoding::WriteTls:read(i.rd);break;
        case Encoding::ReadFlags:write(i.rd);e.flags_read=true;break;
        case Encoding::WriteFlags:read(i.rd);e.flags_write=true;break;
        default:e.gp_read=e.gp_write=UINT32_MAX;e.flags_read=e.flags_write=true;break;
        }break;
    }
    return e;
}
}
