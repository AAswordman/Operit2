#include "cpu/aarch64/wasm_region.h"
#include "cpu/aarch64/block_graph.h"
#include "memory/address_space.h"
#include "cpu/aarch64/cache.h"
#include <algorithm>
#include <functional>
#include <map>
namespace operit::arm64 {
using Bytes=std::vector<uint8_t>;
constexpr unsigned gp=6;
struct Binary : Bytes {
    unsigned depth=0;
    void u(uint64_t value) { do { auto b = value & 127; value >>= 7; push_back(b | (value ? 128 : 0)); } while (value); }
    void s(int64_t value) {
        for (;;) { auto b = value & 127; value >>= 7;
            bool done = (value == 0 && !(b & 64)) || (value == -1 && (b & 64));
            push_back(b | (done ? 0 : 128)); if (done) break;
        }
    }
    void op(uint8_t value) { push_back(value);if(value==2 || value==3 || value==4)++depth;else if(value==11 && depth)--depth; }
    void append(const Bytes &bytes) { insert(end(), bytes.begin(), bytes.end()); }
    void section(unsigned id, const Binary &bytes) { op(id); u(bytes.size()); append(bytes); }
    void name(const std::string &text) { u(text.size()); insert(end(), text.begin(), text.end()); }
    void get(unsigned id) {if(id>=gp && id<gp+32) {op(0x20);u(0);mem(0x29,(id-gp)*8);}else {op(0x20);u(id>=gp+32 ? id-32:id);}}
    void set(unsigned id) {if(id>=gp && id<gp+32) {op(0x21);u(20);op(0x20);u(0);op(0x20);u(20);mem(0x37,(id-gp)*8);}else {op(0x21);u(id>=gp+32 ? id-32:id);}}
    void i32(uint32_t value) { op(0x41); s(static_cast<int32_t>(value)); }
    void i64(uint64_t value) { op(0x42); s(static_cast<int64_t>(value)); }
    void f64(double value) {op(0x44);auto *p=reinterpret_cast<const uint8_t*>(&value);insert(end(),p,p+8);}
    void mem(unsigned opcode, unsigned offset = 0) { op(opcode); u(0); u(offset); }
};
struct WasmRegion::Compiler {
    Binary code;
    uint64_t entry;
    std::unique_ptr<BlockGraph> graph;
    std::map<uint64_t,unsigned> entries;
    unsigned current_block=0,current_depth=0;
    bool self_loop=false;
    static Instruction normalized(Instruction i) {
        if(i.op==Op::Simd) {
            if((i.raw&0xffa00c00)==0x1e200800 && ((i.raw>>12)&15)<4) {
                constexpr Encoding ops[]={Encoding::ScalarMultiply64,Encoding::ScalarDivide64,Encoding::ScalarAdd64,Encoding::ScalarSubtract64};i.encoding=ops[(i.raw>>12)&15];
            }
            if((i.raw&0xffa0fc07)==0x1e202000)i.encoding=Encoding::ScalarCompare64;
            if((i.raw&0x7fbefc00)==0x1e220000)i.encoding=Encoding::ScalarUnsignedToDouble64;
            if((i.raw&0xff800000)==0x1f000000)i.encoding=Encoding::ScalarFma64;
            if((i.raw&0xffe0fc07)==0x1e602000)i.encoding=Encoding::ScalarCompare64;
        }return i;
    }
    static bool exclusive(uint32_t r) {
        if((r&0xfffff0ff)==0xd503305f)return true;
        if((r&0x3f000000)!=0x08000000)return false;
        const unsigned size=1u<<(r>>30),rn=(r>>5)&31,rt=r&31,status=(r>>16)&31,rt2=(r>>10)&31;
        const bool load=r&(1u<<22),ordered=r&(1u<<23),pair=r&(1u<<21);
        return !(ordered && (pair || !(r&(1u<<15)))) && (pair || rt2==31) && (!(load || ordered) || status==31) && (!pair || size>=4) &&
            !(pair && load && rt==rt2) && !(!load && !ordered && status!=31 && (status==rt || (pair && status==rt2) || (rn!=31 && status==rn)));
    }
    static bool supported(Instruction i) {
        i=normalized(i);
        const auto copy=i.raw&0xbfe0fc00;
        if(i.op==Op::Simd && ((i.raw&0x7fbffc00)==0x1e380000 ||
            ((copy==0x0e000c00 || copy==0x0e000400 || copy==0x0e003c00 || copy==0x0e002c00 || (i.raw&0xffe0fc00)==0x4e001c00) && ((i.raw>>16)&31) && __builtin_ctz((i.raw>>16)&31)<=3) ||
            (i.raw&0x9ff80c00)==0x0f000400 || (i.raw&0x9f20fc00)==0x0e208400 ||
            (((i.raw&0x9fbffc00)==0x0ea0f800) && (!(i.raw&(1u<<22)) || (i.raw&(1u<<30))))))return true;
        if(i.op==Op::Cache && (i.raw&0xffffffe0)==cache_profile::dc_zva)return true;
        if(i.op==Op::System && (i.raw&(1u<<21)) && (((i.raw>>5)&65535)==0xd801 || ((i.raw>>5)&65535)==0xd807))return true;
        if(i.op==Op::Exclusive && exclusive(i.raw))return true;
        if(i.encoding==Encoding::General)return false;
        switch(i.op) {
        case Op::Hint:return i.encoding==Encoding::Noop;
        case Op::Address:case Op::MoveWide:case Op::AddImmediate:return true;
        case Op::AddRegister:case Op::LogicalImmediate:case Op::LogicalRegister:
        case Op::MultiplyHigh:case Op::Multiply:case Op::Bitfield:case Op::Extract:case Op::ConditionalSelect:
        case Op::ConditionalCompare:case Op::TwoSource:case Op::OneSource:
        case Op::LoadStore:case Op::LoadPair:case Op::LoadLiteral:return true;
        case Op::Simd:
            switch(i.encoding) {
            case Encoding::VectorToRegister32:case Encoding::VectorToRegister64:case Encoding::VectorHighToRegister:
            case Encoding::RegisterToVector32:case Encoding::RegisterToVector64:case Encoding::RegisterToVectorHigh:
            case Encoding::VectorImmediate:case Encoding::VectorLogical:case Encoding::ScalarUnary32:case Encoding::ScalarUnary64:
            case Encoding::ScalarAdd64:case Encoding::ScalarSubtract64:case Encoding::ScalarMultiply64:case Encoding::ScalarDivide64:
            case Encoding::ScalarSqrt64:case Encoding::ScalarCompare64:case Encoding::ScalarSelect64:
            case Encoding::ScalarFma64:case Encoding::ScalarUnsignedToDouble64:return true;
            default:return false;
            }
        case Op::Cache:return (i.raw&0xffffffe0)==cache_profile::dc_zva;
        case Op::System:return i.encoding==Encoding::ReadTls || i.encoding==Encoding::WriteTls || i.encoding==Encoding::ReadFlags || i.encoding==Encoding::WriteFlags ||
            ((i.raw&(1u<<21)) && (((i.raw>>5)&65535)==0xd801 || ((i.raw>>5)&65535)==0xd807));
        case Op::ConditionalBranch:case Op::CompareBranch:case Op::TestBranch:return true;
        case Op::Branch:return i.encoding==Encoding::Branch || i.encoding==Encoding::BranchLink;
        case Op::BranchRegister:return i.encoding==Encoding::BranchRegister || i.encoding==Encoding::BranchLinkRegister;
        case Op::Svc:return i.encoding==Encoding::Syscall;
        default:return false;
        }
    }
    void reg(unsigned r,bool stack=false) { if(r==31 && !stack)code.i64(0);else code.get(gp+r); }
    void result(const Instruction &i,bool stack=false) {
        truncate(i.width);
        if(i.rd==31 && !stack)code.op(0x1a);else code.set(gp+i.rd);
    }
    void advance(unsigned count) {
        code.get(1);code.i32(count);code.op(0x6b);code.set(1);
    }
    void finish(Exit reason,const std::function<void()> &pc,unsigned count=0) {
        if(count)advance(count);
        pc();code.set(5);code.i32(reason);code.set(43);code.op(0x0c);code.u(code.depth-1);
    }
    void finish(Exit reason,uint64_t pc,unsigned count=0) {finish(reason,[&]{code.i64(pc);},count);}
    void admission(uint64_t pc,unsigned count) {
        code.get(1);code.i32(count);code.op(0x49);code.op(0x04);code.op(0x40);
        finish(Budget,pc);code.op(0x0b);
    }
    void jump(uint64_t pc,unsigned depth,unsigned count) {
        advance(count);auto at=entries.find(pc);
        if(at==entries.end()) {code.i64(pc);code.set(5);code_slot();linked_transfer(pc,depth,false);}
        else if(self_loop && at->second==current_block) {code.op(0x0c);code.u(depth-current_depth);}
        else if(at->second>current_block && at->second/64==current_block/64) {
            code.op(0x0c);code.u(at->second-current_block-1+depth-current_depth+unsigned(self_loop));
        } else {
            if(graph->blocks.size()>1) {code.i32(at->second);code.set(38);}
            code.op(0x0c);code.u(depth);
        }
    }
    void linked_transfer(uint64_t pc,unsigned depth,bool call) {
        {
            code.get(44);code.mem(0x29);code.get(5);code.op(0x51);
            code.get(44);code.mem(0x28,8);code.op(0x45);code.op(0x45);code.op(0x71);
            code.get(0);code.mem(0x28,420);code.i32(32);code.op(0x49);code.op(0x71);code.op(0x04);code.op(0x40);
            code.get(0);code.mem(0x28,384);code.set(51);
            code.get(0);code.get(44);code.mem(0x28,8);code.mem(0x36,384);
            code.get(0);code.get(44);code.mem(0x28,12);code.mem(0x36,entry_offset);
            code.get(0);code.get(42);code.mem(0x36,flags_offset);
            code.get(0);code.get(0);code.mem(0x28,420);code.i32(1);code.op(0x6a);code.mem(0x36,420);
            code.get(0);code.get(1);code.get(44);code.mem(0x28,8);code.op(0x11);code.u(0);code.u(0);code.set(50);
            code.get(0);code.get(0);code.mem(0x28,420);code.i32(1);code.op(0x6b);code.mem(0x36,420);
            code.get(0);code.mem(0x28,remaining_offset);code.set(1);
            code.get(0);code.mem(0x28,flags_offset);code.set(42);
            code.get(0);code.get(51);code.mem(0x36,384);
            if(call && entries.count(pc+4)) {
                code.get(50);code.i32(Branch);code.op(0x46);
                code.get(0);code.mem(0x29,pc_offset);code.i64(pc+4);code.op(0x51);code.op(0x71);code.op(0x04);code.op(0x40);
                jump(pc+4,depth+2,0);code.op(0x0b);
            }
            code.get(0);code.mem(0x29,pc_offset);code.set(5);code.get(50);code.set(43);code.op(0x0c);code.u(code.depth-1);
            code.op(0x0b);
        }
        finish(Branch,[&]{code.get(5);});
    }
    void code_slot() {
        code.get(5);code.i64(2);code.op(0x88);code.get(5);code.i64(18);code.op(0x88);code.op(0x85);code.op(0xa7);
        code.i32(page_slots-1);code.op(0x71);code.i32(4);code.op(0x74);code.i32(code_table);code.op(0x6a);code.set(44);
    }
    explicit Compiler(const std::vector<std::pair<uint64_t,uint32_t>> &words,uint64_t address) : entry(address) {
        if(words.empty() || words.size()>16384)throw std::runtime_error("region size must be 1..16384");
        graph=build_instruction_graph(words,entry);
        for(unsigned b=0;b<graph->blocks.size();++b) {
            const auto &block=*graph->blocks[b];entries.emplace(block.pc,b);
            for(const auto &node:block.instructions)
                if(!supported(node.instruction))throw std::runtime_error("unsupported ARM64 region instruction "+hex(node.instruction.raw));
        }
        // Logical GP IDs 6..37 address shared state; local 20 stages GP writes.
        code.op(0x02);code.op(0x40); // one common side-exit epilogue
        code.op(0x03);code.op(0x40);
        if(graph->blocks.size()==1)emit_block(0,0);
        else {
            // Two-level br_table keeps compiler nesting bounded as regions grow.
            const unsigned groups=(graph->blocks.size()+63)/64;
            for(unsigned g=0;g<groups;++g) {code.op(0x02);code.op(0x40);}
            code.get(38);code.i32(6);code.op(0x76);table(groups);
            for(unsigned g=0;g<groups;++g) {
                code.op(0x0b);const auto count=unsigned(std::min<size_t>(64,graph->blocks.size()-g*64));
                for(unsigned n=0;n<count;++n) {code.op(0x02);code.op(0x40);}
                code.get(38);code.i32(63);code.op(0x71);table(count);
                for(unsigned n=0;n<count;++n) {code.op(0x0b);emit_block(g*64+n,groups-1-g+count-1-n);}
            }
        }
        code.op(0x0b);code.op(0x00);code.op(0x0b);
        code.get(0);code.get(5);code.mem(0x37,pc_offset);
        code.get(0);code.get(42);code.mem(0x36,flags_offset);
        code.get(0);code.get(2);code.get(1);code.op(0x6b);code.mem(0x36,retired_offset);
        code.get(0);code.get(1);code.mem(0x36,remaining_offset);
        code.get(43);code.op(0x0b);
        Binary prefix;
        prefix.u(9);prefix.u(3);prefix.op(0x7f);prefix.u(1);prefix.op(0x7e);prefix.u(1);prefix.op(0x7f);prefix.u(3);prefix.op(0x7e);prefix.u(3);prefix.op(0x7f);prefix.u(3);prefix.op(0x7c);prefix.u(2);prefix.op(0x7e);prefix.u(2);prefix.op(0x7f);prefix.u(1);prefix.op(0x7e);
        // Retirement is the entry budget minus what remains, including nested calls.
        prefix.get(1);prefix.set(2);prefix.get(0);prefix.mem(0x28,flags_offset);prefix.set(42);
        prefix.get(0);prefix.mem(0x28,entry_offset);prefix.set(38);
        prefix.append(code);code=std::move(prefix);
    }
    void table(unsigned count) {
        code.op(0x0e);code.u(count);for(unsigned n=0;n<count;++n)code.u(n);code.u(0);
    }
    void emit_block(unsigned b,unsigned depth) {
        const auto &basic=*graph->blocks[b];const auto &block=basic.instructions;
        self_loop=std::find(basic.successors.begin(),basic.successors.end(),basic.pc)!=basic.successors.end();
        if(self_loop) {code.op(0x03);code.op(0x40);++depth;}
        current_block=b;current_depth=depth;
            admission(block.front().pc,block.size());
            for(unsigned k=0;k<block.size();++k) {
                const auto &node=block[k];const auto pc=node.pc;auto i=normalized(node.instruction);
                // Keep source indices for budgets and fault exits even when a
                // node emits no code or becomes a single constant assignment.
                if(node.dead_result)continue;
                if(node.constant_result) {
                    code.i64(node.constant);code.set(gp+unsigned(__builtin_ctz(node.effects.gp_write)));continue;
                }
                // Retain the backend's adjacent-add peephole after consuming
                // shared optimizer decisions, without crossing a CFG boundary.
                if(i.op==Op::AddImmediate && !(i.flags&decoded::set_flags) && i.rd==i.rn) {
                    auto sum=(i.flags&decoded::subtract) ? -i.immediate:i.immediate;
                    while(k+1<block.size()) {
                        const auto &next_node=block[k+1];const auto &next=next_node.instruction;
                        if(next_node.dead_result || next_node.constant_result || next.op!=i.op || next.width!=i.width ||
                           next.rd!=i.rd || next.rn!=i.rn || (next.flags&decoded::set_flags))break;
                        sum+=(next.flags&decoded::subtract) ? -next.immediate:next.immediate;++k;
                    }
                    i.immediate=sum;i.flags=0;
                }
                switch(i.op) {
                case Op::Hint:break;
                case Op::Address:
                    code.i64(i.flags ? (pc&~uint64_t{4095})+(i.immediate<<12):pc+i.immediate);
                    if(i.rd==31)code.op(0x1a);else code.set(gp+i.rd);break;
                case Op::MoveWide:
                    if((i.raw>>29&3)==3) {reg(i.rd);code.i64(i.auxiliary);code.op(0x83);}
                    code.i64(i.immediate);if((i.raw>>29&3)==3)code.op(0x84);result(i);break;
                case Op::AddImmediate:case Op::AddRegister: {
                    const bool flags=(i.flags&decoded::set_flags) && !node.discard_flags;
                    reg(i.rn,i.op==Op::AddImmediate || (i.flags&decoded::extended));if(flags) {truncate(i.width);code.set(39);}
                    if(i.op==Op::AddImmediate)code.i64(i.immediate);
                    else if(i.flags&decoded::extended) {reg(i.rm);extend(i.immediate,i.flags&decoded::signed_extension);if(i.shift) {code.i64(i.shift);code.op(0x86);}}
                    else shifted(i.rm,(i.flags>>decoded::shift_type)&3,i.shift,i.width);
                    if(flags) {truncate(i.width);code.set(40);code.get(39);code.get(40);}
                    code.op(i.flags&decoded::subtract ? 0x7d:0x7c);
                    if(flags) {truncate(i.width);code.set(41);arithmetic_flags(i);code.get(41);}
                    result(i,(i.op==Op::AddImmediate || (i.flags&decoded::extended)) && !(i.flags&decoded::set_flags));break;
                }
                case Op::LogicalImmediate:case Op::LogicalRegister:
                    reg(i.rn);
                    if(i.op==Op::LogicalImmediate)code.i64(i.immediate);
                    else {shifted(i.rm,(i.flags>>2)&3,i.shift,i.width);if(i.flags&16) {code.i64(UINT64_MAX);code.op(0x85);}}
                    code.op((i.flags&3)==3 ? 0x83:0x83+(i.flags&3));
                    if((i.flags&3)==3 && !node.discard_flags) {truncate(i.width);code.set(41);logical_flags(i.width);code.get(41);}result(i);break;
                case Op::Multiply:
                    reg(i.ra);reg(i.rn);if(i.flags&7)extend(32,(i.flags&7)==1);reg(i.rm);if(i.flags&7)extend(32,(i.flags&7)==1);code.op(0x7e);code.op(i.flags&8 ? 0x7d:0x7c);result(i);break;
                case Op::MultiplyHigh:
                    multiply_high(i);break;
                case Op::Exclusive:atomic(i,pc,k);break;
                case Op::Cache:
                    reg(i.rd);code.i64(~uint64_t{63});code.op(0x83);code.set(5);translate(pc,k,64,write_perm);
                    for(unsigned offset=0;offset<64;offset+=8) {code.get(3);code.i64(0);code.mem(0x37,offset);}break;
                case Op::LoadStore:case Op::LoadPair:case Op::LoadLiteral:
                    memory(i,pc,k);break;
                case Op::Bitfield: {
                    const auto wm=i.immediate,tm=i.auxiliary;
                    if(i.flags==1) {reg(i.rd);code.i64(~wm);code.op(0x83);}else code.i64(0);
                    shifted(i.rn,3,i.shift,i.width);code.i64(wm);code.op(0x83);code.op(0x84);code.i64(tm);code.op(0x83);
                    if(i.flags==0) {reg(i.rn);code.i64((i.raw>>10)&63);code.op(0x88);code.i64(1);code.op(0x83);code.i64(UINT64_MAX);code.op(0x7e);code.i64(~tm);code.op(0x83);code.op(0x84);}
                    else if(i.flags==1) {reg(i.rd);code.i64(~tm);code.op(0x83);code.op(0x84);}
                    result(i);break;
                }
                case Op::Extract:
                    reg(i.rm);truncate(i.width);if(i.shift) {code.i64(i.shift);code.op(0x88);reg(i.rn);code.i64(i.width-i.shift);code.op(0x86);code.op(0x84);}result(i);break;
                case Op::ConditionalSelect:
                    condition(i.shift);code.op(0x04);code.op(0x7e);reg(i.rn);code.op(0x05);reg(i.rm);
                    if(i.flags&1) {code.i64(UINT64_MAX);code.op(0x85);}if(i.flags&2) {code.i64(1);code.op(0x7c);}
                    code.op(0x0b);result(i);break;
                case Op::ConditionalCompare: {
                    if(node.discard_flags)break;
                    condition(i.shift);code.op(0x04);code.op(0x40);
                    reg(i.rn);truncate(i.width);code.set(39);
                    if(i.flags&2)code.i64(i.rm);else {reg(i.rm);truncate(i.width);}code.set(40);
                    code.get(39);code.get(40);code.op(i.flags&1 ? 0x7d:0x7c);truncate(i.width);code.set(41);
                    auto arithmetic=i;arithmetic.flags=(i.flags&1) ? decoded::subtract:0;arithmetic_flags(arithmetic);
                    code.op(0x05);code.i32(i.raw&15);code.set(42);code.op(0x0b);break;
                }
                case Op::TwoSource:two_source(i);break;
                case Op::OneSource:one_source(i);break;
                case Op::Simd:floating(i,pc,k);break;
                case Op::System:
                    if(i.encoding==Encoding::General) {code.i64(((i.raw>>5)&65535)==0xd801 ? cache_profile::ctr_el0:cache_profile::dczid_el0);result(i);}
                    else if(i.encoding==Encoding::ReadTls) {code.get(0);code.mem(0x29,tls_offset);result(i);}
                    else if(i.encoding==Encoding::WriteTls) {code.get(0);reg(i.rd);code.mem(0x37,tls_offset);}
                    else if(i.encoding==Encoding::ReadFlags) {code.get(42);code.op(0xad);code.i64(28);code.op(0x86);result(i);}
                    else {reg(i.rd);code.i64(28);code.op(0x88);code.op(0xa7);code.i32(15);code.op(0x71);code.set(42);}break;
                case Op::ConditionalBranch:
                    condition(i.flags);code.op(0x04);code.op(0x40);jump(pc+i.immediate,depth+1,k+1);code.op(0x0b);break;
                case Op::CompareBranch:case Op::TestBranch:
                    reg(i.rd);
                    if(i.op==Op::TestBranch) {code.i64(uint64_t{1}<<i.shift);code.op(0x83);}
                    else if(i.width==32)code.op(0xa7);
                    code.op(i.op==Op::CompareBranch && i.width==32 ? 0x45:0x50);if(i.flags)code.op(0x45);
                    code.op(0x04);code.op(0x40);jump(pc+i.immediate,depth+1,k+1);code.op(0x0b);break;
                case Op::Branch:
                    if(i.encoding==Encoding::BranchLink) {code.i64(pc+4);code.set(gp+30);}
                    if(i.encoding==Encoding::BranchLink && !entries.count(pc+i.immediate)) {
                        advance(k+1);code.i64(pc+i.immediate);code.set(5);code_slot();linked_transfer(pc,depth,true);
                    } else jump(pc+i.immediate,depth,k+1);break;
                case Op::BranchRegister:
                    reg(i.rn);code.set(5);
                    if(i.encoding==Encoding::BranchLinkRegister) {code.i64(pc+4);code.set(gp+30);}
                    advance(k+1);
                    code_slot();
                    code.get(44);code.mem(0x29);code.get(5);code.op(0x51);
                    code.get(44);code.mem(0x28,8);code.get(0);code.mem(0x28,384);code.op(0x46);code.op(0x71);
                    code.op(0x04);code.op(0x40);code.get(44);code.mem(0x28,12);code.set(38);
                    code.op(0x0c);code.u(depth+1);code.op(0x0b);
                    if(i.encoding==Encoding::BranchLinkRegister)linked_transfer(pc,depth,true);
                    else if((i.raw&0xfffffc1fu)!=0xd65f0000u)linked_transfer(pc,depth,false);
                    else finish(Branch,[&]{code.get(5);});break;
                case Op::Svc:finish(Syscall,pc+4,k+1);break;
                default:break;
                }
            }
            const auto last=block.back().instruction.op;
            if(last!=Op::Branch && last!=Op::BranchRegister && last!=Op::Svc)jump(block.back().pc+4,depth,block.size());
        if(self_loop)code.op(0x0b);
    }
    void truncate(unsigned bits) {if(bits==32) {code.op(0xa7);code.op(0xad);}}
    void arithmetic_flags(const Instruction &i) {
        const bool sub=i.flags&decoded::subtract;
        code.get(41);code.i64(i.width-1);code.op(0x88);code.op(0xa7);code.i32(3);code.op(0x74);
        code.get(41);code.op(0x50);code.i32(2);code.op(0x74);code.op(0x72);
        if(sub) {code.get(39);code.get(40);code.op(0x5a);}
        else {code.get(41);code.get(39);code.op(0x54);}
        code.i32(1);code.op(0x74);code.op(0x72);
        code.get(39);code.get(sub ? 40:41);code.op(0x85);
        code.get(sub ? 39:40);code.get(41);code.op(0x85);code.op(0x83);
        code.i64(i.width-1);code.op(0x88);code.op(0xa7);code.op(0x72);code.set(42);
    }
    void flag(unsigned bit) {code.get(42);code.i32(bit);code.op(0x71);code.op(0x45);code.op(0x45);}
    void condition(unsigned cond) {
        switch(cond>>1) {
        case 0:flag(4);break;
        case 1:flag(2);break;
        case 2:flag(8);break;
        case 3:flag(1);break;
        case 4:flag(2);flag(4);code.op(0x45);code.op(0x71);break;
        case 5:case 6:
            flag(8);flag(1);code.op(0x46);
            if((cond>>1)==6) {flag(4);code.op(0x45);code.op(0x71);}break;
        default:code.i32(1);break;
        }
        if((cond&1) && cond!=15)code.op(0x45);
    }
    void extend(unsigned bits,bool sign) {
        if(bits<64) {if(sign) {code.i64(64-bits);code.op(0x86);code.i64(64-bits);code.op(0x87);}else {code.i64(mask(bits));code.op(0x83);}}
    }
    void shifted(unsigned r,unsigned type,unsigned count,unsigned bits) {
        reg(r);if(type==2)extend(bits,true);else truncate(bits);
        if(count) {
            if(type==3 && bits==32) {code.op(0xa7);code.i32(count);code.op(0x78);code.op(0xad);}
            else {code.i64(count);constexpr unsigned ops[]={0x86,0x88,0x87,0x8a};code.op(ops[type]);}
        }
    }
    void logical_flags(unsigned bits) {
        code.get(41);code.i64(bits-1);code.op(0x88);code.op(0xa7);code.i32(3);code.op(0x74);
        code.get(41);code.op(0x50);code.i32(2);code.op(0x74);code.op(0x72);code.set(42);
    }
    void two_source(const Instruction &i) {
        if(i.flags>=8) {
            reg(i.rn);if(i.flags==10)extend(i.width,true);else truncate(i.width);
            reg(i.rm);code.i64(i.width-1);code.op(0x83);
            if(i.flags==11 && i.width==32) {code.op(0xa7);code.set(3);code.op(0xa7);code.get(3);code.op(0x78);code.op(0xad);}
            else {constexpr unsigned ops[]={0x86,0x88,0x87,0x8a};code.op(ops[i.flags-8]);}
        } else {
            reg(i.rn);extend(i.width,i.flags==3);code.set(39);reg(i.rm);extend(i.width,i.flags==3);code.set(40);
            code.get(40);code.op(0x50);code.op(0x04);code.op(0x7e);code.i64(0);code.op(0x05);
            if(i.flags==3) {code.get(39);code.i64(uint64_t{1}<<63);code.op(0x51);code.get(40);code.i64(UINT64_MAX);code.op(0x51);code.op(0x71);}else code.i32(0);
            code.op(0x04);code.op(0x7e);code.get(39);code.op(0x05);code.get(39);code.get(40);code.op(i.flags==3 ? 0x7f:0x80);code.op(0x0b);code.op(0x0b);
        }
        result(i);
    }
    void one_source(const Instruction &i) {
        if(i.flags>=4) {
            reg(i.rn);truncate(i.width);code.set(39);
            if(i.flags==5) {code.get(39);code.get(39);code.i64(i.width-1);code.op(0x88);code.i64(UINT64_MAX);code.op(0x7e);code.op(0x85);truncate(i.width);code.set(39);}
            code.get(39);code.op(0x79);code.i64(64-i.width+(i.flags==5));code.op(0x7d);
        } else {
            code.i64(0);
            const unsigned unit=i.flags ? 8:1,group=i.flags ? (8u<<i.flags):i.width;
            for(unsigned n=0;n<i.width;n+=unit) {
                reg(i.rn);code.i64(n);code.op(0x88);code.i64(mask(unit));code.op(0x83);
                code.i64(n/group*group+group-unit-n%group);code.op(0x86);code.op(0x84);
            }
        }
        result(i);
    }
    void multiply_high(const Instruction &i) {
        reg(i.rn);code.set(39);reg(i.rm);code.set(40);
        code.get(39);truncate(32);code.get(40);truncate(32);code.op(0x7e);code.i64(32);code.op(0x88);code.set(48);
        code.get(39);code.i64(32);code.op(0x88);code.get(40);truncate(32);code.op(0x7e);code.get(48);code.op(0x7c);code.set(49);
        code.get(39);truncate(32);code.get(40);code.i64(32);code.op(0x88);code.op(0x7e);code.get(49);truncate(32);code.op(0x7c);code.i64(32);code.op(0x88);code.set(48);
        code.get(39);code.i64(32);code.op(0x88);code.get(40);code.i64(32);code.op(0x88);code.op(0x7e);code.get(49);code.i64(32);code.op(0x88);code.op(0x7c);code.get(48);code.op(0x7c);
        if(!i.flags) {
            code.get(39);code.i64(63);code.op(0x87);code.get(40);code.op(0x83);code.op(0x7d);
            code.get(40);code.i64(63);code.op(0x87);code.get(39);code.op(0x83);code.op(0x7d);
        }result(i);
    }
    void assist_if(uint64_t pc,unsigned retired) {
        code.op(0x04);code.op(0x40);finish(Assist,pc,retired);code.op(0x0b);
    }
    void normal(unsigned local,uint64_t pc,unsigned retired,unsigned bits=64) {
        code.get(local);code.op(0xbd);code.i64(0x7fffffffffffffffull);code.op(0x83);code.set(39);
        code.get(39);code.i64(bits==64 ? 0x0010000000000000ull:0x3810000000000000ull);code.op(0x54);code.get(39);code.op(0x50);code.op(0x45);code.op(0x71);
        code.get(39);code.i64(bits==64 ? 0x7ff0000000000000ull:0x47f0000000000000ull);code.op(0x5a);code.op(0x72);assist_if(pc,retired);
    }
    void vector(unsigned r,unsigned lane=0) {code.get(0);code.mem(0x29,vector_offset+r*16+lane*8);}
    void vector_integer(const Instruction &i) {
        const auto r=i.raw,copy=r&0xbfe0fc00;
        const unsigned count=r&(1u<<30) ? 16:8,out=vector_offset+i.rd*16;
        constexpr unsigned loads[]={0x31,0x33,0x35,0x29},stores[]={0x3c,0x3d,0x3e,0x37};
        if((r&0x9f20fc00)==0x0e208400) {
            const unsigned shift=(r>>22)&3,size=1u<<shift;
            for(unsigned offset=0;offset<count;offset+=size) {
                code.get(0);code.get(0);code.mem(loads[shift],vector_offset+i.rn*16+offset);
                code.get(0);code.mem(loads[shift],vector_offset+i.rm*16+offset);code.op(r&(1u<<29) ? 0x7d:0x7c);code.mem(stores[shift],out+offset);
            }
        } else if((r&0x9ff80c00)==0x0f000400) {
            const unsigned cmode=(r>>12)&15,imm=((r>>5)&31)|((r>>11)&224);const bool invert=r&(1u<<29);
            unsigned size;uint64_t value;
            if(cmode<8) {size=4;value=uint64_t(imm)<<((cmode/2)*8);}
            else if(cmode<12) {size=2;value=uint64_t(imm)<<(((cmode-8)/2)*8);}
            else if(cmode<14) {size=4;const unsigned shift=(cmode-11)*8;value=(uint64_t(imm)<<shift)|mask(shift);}
            else if(cmode==14) {size=invert ? 8:1;value=imm;if(invert) {value=0;for(unsigned b=0;b<8;++b)if(imm&(1u<<b))value|=uint64_t{255}<<(b*8);}}
            else {size=invert ? 8:4;const unsigned eb=invert ? 11:8,fb=size*8-eb-1;const uint64_t repeated=(imm>>6)&1;
                value=(uint64_t(imm>>7)<<(size*8-1))|((((repeated^1)<<(eb-1))|((repeated ? mask(eb-3):0)<<2)|((imm>>4)&3))<<fb)|(uint64_t(imm&15)<<(fb-4));}
            if(invert && cmode<14)value=~value;
            uint64_t lane=0;for(unsigned b=0;b<8;b+=size)lane|=(value&mask(size*8))<<(b*8);
            for(unsigned b=0;b<count;b+=8) {code.get(0);code.i64(lane);if(cmode<12 && (cmode&1)) {vector(i.rd,b/8);code.op(invert ? 0x83:0x84);}code.mem(0x37,out+b);}
        } else {
            const unsigned imm5=(r>>16)&31,shift=__builtin_ctz(imm5),size=1u<<shift,lane=imm5>>(shift+1);
            if(copy==0x0e000c00 || (r&0xffe0fc00)==0x4e001c00) {reg(i.rn);extend(size*8,false);}
            else {code.get(0);code.mem(loads[shift],vector_offset+i.rn*16+lane*size);}
            code.set(39);
            if(copy==0x0e003c00 || copy==0x0e002c00) {code.get(39);if(copy==0x0e002c00)extend(size*8,true);if(count==8)truncate(32);if(i.rd==31)code.op(0x1a);else code.set(gp+i.rd);return;}
            if((r&0xffe0fc00)==0x4e001c00) {code.get(0);code.get(39);code.mem(stores[shift],out+lane*size);return;}
            for(unsigned offset=0;offset<count;offset+=size) {code.get(0);code.get(39);code.mem(stores[shift],out+offset);}
        }
        if(count==8) {code.get(0);code.i64(0);code.mem(0x37,out+8);}
    }
    void floating(const Instruction &i,uint64_t pc,unsigned retired) {
        using E=Encoding;const auto e=i.encoding;
        const unsigned out=vector_offset+i.rd*16,bits=(i.raw&(1u<<22)) ? 64:32;
        if((i.raw&0x9fbffc00)==0x0ea0f800) {
            const uint64_t sign=bits==64 ? 0x8000000000000000ull:0x8000000080000000ull;
            for(unsigned lane=0;lane<2;++lane) {code.get(0);if(lane && !(i.raw&(1u<<30)))code.i64(0);else {vector(i.rn,lane);code.i64(i.raw&(1u<<29) ? sign:~sign);code.op(i.raw&(1u<<29) ? 0x85:0x83);}code.mem(0x37,out+lane*8);}return;
        }
        if(e==E::General && (i.raw&0x7fbffc00)!=0x1e380000) {vector_integer(i);return;}
        if(e==E::VectorToRegister32 || e==E::VectorToRegister64 || e==E::VectorHighToRegister) {
            vector(i.rn,e==E::VectorHighToRegister);if(e==E::VectorToRegister32)truncate(32);
            if(i.rd==31)code.op(0x1a);else code.set(gp+i.rd);return;
        }
        if(e==E::RegisterToVectorHigh) {code.get(0);reg(i.rn);code.mem(0x37,out+8);return;}
        if(e==E::RegisterToVector32 || e==E::RegisterToVector64 || e==E::VectorImmediate || e==E::ScalarUnary32 || e==E::ScalarUnary64 || e==E::ScalarSelect64) {
            code.get(0);
            if(e==E::VectorImmediate)code.i64(i.immediate);
            else if(e==E::RegisterToVector32 || e==E::RegisterToVector64) {reg(i.rn);if(e==E::RegisterToVector32)truncate(32);}
            else if(e==E::ScalarSelect64) {condition(i.flags);code.op(0x04);code.op(0x7e);vector(i.rn);code.op(0x05);vector(i.rm);code.op(0x0b);}
            else {vector(i.rn);const unsigned bits=e==E::ScalarUnary32 ? 32:64;if(i.flags==1) {code.i64(mask(bits-1));code.op(0x83);}if(i.flags==2) {code.i64(uint64_t{1}<<(bits-1));code.op(0x85);}truncate(bits);}
            code.mem(0x37,out);code.get(0);code.i64(0);code.mem(0x37,out+8);return;
        }
        if(e==E::VectorLogical) {
            for(unsigned lane=0;lane<2;++lane) {
                code.get(0);
                if(lane*8>=i.shift)code.i64(0);
                else {
                    vector(i.rn,lane);code.set(39);vector(i.rm,lane);code.set(40);
                    if(!(i.flags&4)) {code.get(39);code.get(40);if(i.flags&1) {code.i64(UINT64_MAX);code.op(0x85);}code.op(i.flags&2 ? 0x84:0x83);}
                    else if(!(i.flags&3)) {code.get(39);code.get(40);code.op(0x85);}
                    else {
                        vector(i.rd,lane);code.set(41);
                        const unsigned select=i.flags==5 ? 41:i.flags==6 ? 40:40;
                        code.get(select);code.get(i.flags==5 ? 39:i.flags==6 ? 39:41);code.op(0x83);
                        code.get(select);code.i64(UINT64_MAX);code.op(0x85);code.get(i.flags==5 ? 40:i.flags==6 ? 41:39);code.op(0x83);code.op(0x84);
                    }
                }
                code.mem(0x37,out+lane*8);
            }return;
        }
        // Baseline Wasm arithmetic has nearest-even rounding and no FPSR.
        // With IXC already sticky, finite normal operations can only repeat
        // IXC; exceptional operands/results and other FPCR modes assist natively.
        code.get(0);code.mem(0x29,352);code.op(0x50);code.op(0x45);assist_if(pc,retired);
        if(e!=E::ScalarCompare64) {code.get(0);code.mem(0x29,360);code.i64(16);code.op(0x83);code.op(0x50);assist_if(pc,retired);}
        const bool conversion=e==E::ScalarUnsignedToDouble64;
        const bool to_integer=(i.raw&0x7fbffc00)==0x1e380000;
        const auto load_float=[&](unsigned r) {code.get(0);code.mem(bits==64 ? 0x2b:0x2a,vector_offset+r*16);if(bits==32)code.op(0xbb);};
        if(conversion) {reg(i.rn);extend(i.width,!(i.raw&(1u<<16)));code.op(bits==64 ? (i.raw&(1u<<16) ? 0xba:0xb9):(i.raw&(1u<<16) ? 0xb5:0xb4));if(bits==32)code.op(0xbb);code.set(47);}
        else {
            load_float(i.rn);code.set(45);normal(45,pc,retired,bits);
            if(to_integer) {
                code.get(45);code.f64(i.width==64 ? -0x1p63:-0x1p31);code.op(0x63);
                code.get(45);code.f64(i.width==64 ? 0x1p63:0x1p31);code.op(0x66);code.op(0x72);assist_if(pc,retired);
                code.get(45);code.op(0xb0);result(i);return;
            }
            if(e==E::ScalarSqrt64) {code.get(45);if(bits==32)code.op(0xb6);code.op(bits==64 ? 0x9f:0x91);if(bits==32)code.op(0xbb);code.set(47);}
            else {
                if(e==E::ScalarCompare64 && (i.raw&8))code.f64(0);else load_float(i.rm);code.set(46);normal(46,pc,retired,bits);
                if(e==E::ScalarCompare64) {
                    code.get(45);code.get(46);code.op(0x61);code.op(0x04);code.op(0x7f);code.i32(6);code.op(0x05);
                    code.get(45);code.get(46);code.op(0x63);code.op(0x04);code.op(0x7f);code.i32(8);code.op(0x05);code.i32(2);code.op(0x0b);code.op(0x0b);code.set(42);return;
                }
                code.get(45);if(bits==32 && e!=E::ScalarFma64)code.op(0xb6);
                if(e==E::ScalarFma64 && bool(i.raw&(1u<<15))!=bool(i.raw&(1u<<21)))code.op(0x9a);
                code.get(46);if(bits==32 && e!=E::ScalarFma64)code.op(0xb6);
                if(e==E::ScalarFma64) {
                    load_float(i.ra);code.set(47);normal(47,pc,retired,bits);code.get(47);
                    if(i.raw&(1u<<21))code.op(0x9a);
                    code.op(0x10);code.u(bits==64 ? 0:1);
                } else {const unsigned op=e==E::ScalarAdd64 ? 0xa0:e==E::ScalarSubtract64 ? 0xa1:e==E::ScalarMultiply64 ? 0xa2:0xa3;code.op(bits==64 ? op:op-14);if(bits==32)code.op(0xbb);}
                code.set(47);
            }
        }
        if(bits==32) {code.get(47);code.op(0xb6);code.op(0xbb);code.set(47);}
        normal(47,pc,retired,bits);
        if(e==E::ScalarMultiply64 || e==E::ScalarDivide64) {
            code.get(47);code.f64(0);code.op(0x61);code.get(45);code.f64(0);code.op(0x62);code.op(0x71);
            if(e==E::ScalarMultiply64) {code.get(46);code.f64(0);code.op(0x62);code.op(0x71);}assist_if(pc,retired);
        }
        code.get(0);code.get(47);if(bits==32) {code.op(0xb6);code.op(0xbc);code.op(0xad);code.mem(0x37,out);}else code.mem(0x39,out);code.get(0);code.i64(0);code.mem(0x37,out+8);
    }
    void translate(uint64_t pc,unsigned retired,unsigned size,unsigned permissions,bool invalidate=true) {
        // The aligned page tag carries permissions in its otherwise zero low bits.
        code.get(5);code.i64(12);code.op(0x88);code.get(5);code.i64(28);code.op(0x88);code.op(0x85);code.op(0xa7);
        code.i32(page_slots-1);code.op(0x71);code.i32(4);code.op(0x74);code.i32(page_table);code.op(0x6a);code.set(44);
        code.get(44);code.mem(0x29);code.i64(~uint64_t{4095}|permissions);code.op(0x83);
        code.get(5);code.i64(~uint64_t{4095});code.op(0x83);code.i64(permissions);code.op(0x84);code.op(0x52);
        code.get(5);code.op(0xa7);code.i32(4095);code.op(0x71);code.i32(4096-size);code.op(0x4b);code.op(0x72);
        code.op(0x04);code.op(0x40);
        code.get(0);code.get(5);code.mem(0x37,fault_offset);code.get(0);code.i32(permissions);code.mem(0x36,perms_offset);code.get(0);code.i32(size);code.mem(0x36,size_offset);
        finish(Capability,pc,retired);code.op(0x0b);
        code.get(44);code.mem(0x28,8);code.get(5);code.op(0xa7);code.i32(4095);code.op(0x71);code.op(0x6a);code.set(3);
        if(permissions&write_perm) {
            code.get(44);code.mem(0x28,12);code.i32(8);code.op(0x71);code.op(0x45);code.op(0x04);code.op(0x40);
            code.get(0);code.mem(0x28,dirty_count);code.set(4);
            code.i32(dirty_queue);code.get(4);code.i32(2);code.op(0x74);code.op(0x6a);
            code.get(44);code.i32(page_table);code.op(0x6b);code.i32(4);code.op(0x76);code.mem(0x36);
            code.get(0);code.get(4);code.i32(1);code.op(0x6a);code.mem(0x36,dirty_count);
            code.get(44);code.get(44);code.mem(0x28,12);code.i32(8);code.op(0x72);code.mem(0x36,12);
            code.op(0x0b);
            if(invalidate) {
                code.get(0);code.mem(0x28,412);code.op(0x04);code.op(0x40);
                code.get(0);code.get(0);code.mem(0x29,400);code.get(5);code.op(0x85);code.i64(~uint64_t{4095});code.op(0x83);code.op(0x50);code.op(0x45);code.mem(0x36,412);
                code.op(0x0b);
            }
        }
    }
    void atomic(const Instruction &i,uint64_t pc,unsigned retired) {
        const auto r=i.raw;
        if((r&0xfffff0ff)==0xd503305f) {code.get(0);code.i32(0);code.mem(0x36,412);return;}
        const unsigned size=1u<<(r>>30),bytes=size*(r&(1u<<21) ? 2:1),status=(r>>16)&31;
        const bool load=r&(1u<<22),ordered=r&(1u<<23),pair=r&(1u<<21);
        reg(i.rn,true);code.set(5);
        code.get(5);code.i64(bytes-1);code.op(0x83);code.op(0x50);code.op(0x45);assist_if(pc,retired);
        if(!load && !ordered) {
            code.get(0);code.mem(0x28,412);code.get(0);code.mem(0x29,400);code.get(5);code.op(0x51);code.op(0x71);
            code.get(0);code.mem(0x28,408);code.i32(bytes);code.op(0x46);code.op(0x71);code.op(0x04);code.op(0x40);
        }
        translate(pc,retired,bytes,load ? read_perm:write_perm,ordered);
        constexpr unsigned loads[]={0x31,0x33,0x35,0x29},stores[]={0x3c,0x3d,0x3e,0x37};const unsigned shift=r>>30;
        for(unsigned n=0;n<(pair ? 2u:1u);++n) {
            const unsigned rt=n ? i.ra:i.rd;code.get(3);
            if(load) {code.mem(loads[shift],n*size);if(rt==31)code.op(0x1a);else code.set(gp+rt);}
            else {reg(rt);code.mem(stores[shift],n*size);}
        }
        if(!ordered) {
            if(load) {
                code.get(0);code.get(5);code.mem(0x37,400);code.get(0);code.i32(bytes);code.mem(0x36,408);
                code.get(0);code.i32(1);code.mem(0x36,412);
            } else {
                if(status!=31) {code.i64(0);code.set(gp+status);}code.op(0x05);if(status!=31) {code.i64(1);code.set(gp+status);}code.op(0x0b);
                code.get(0);code.i32(0);code.mem(0x36,412);
            }
        }
    }
    void memory(const Instruction &i,uint64_t pc,unsigned retired) {
        if(i.encoding==Encoding::Noop)return;
        const bool vector=i.raw&(1u<<26),pair=i.op==Op::LoadPair,literal=i.op==Op::LoadLiteral;
        const bool load=literal || (pair ? bool(i.raw&(1u<<22)):bool((i.raw>>22)&(vector ? 1:3)));
        const unsigned size=literal ? (i.encoding==Encoding::LoadLiteral64 ? 8:4):pair ? (vector ? 4u<<(i.raw>>30):((i.raw>>30)==2 ? 8:4)):1u<<i.shift;
        const unsigned mode=i.flags&decoded::address_mode;
        if(literal)code.i64(pc+i.immediate);
        else {
            reg(i.rn,true);
            if(!pair && mode==decoded::register_offset) {
                reg(i.rm);extend(i.flags&decoded::offset_word ? 32:64,i.flags&decoded::offset_signed);
                if(i.flags&decoded::offset_scaled) {code.i64(i.shift);code.op(0x86);}code.op(0x7c);
            } else if(mode!=decoded::post_index) {code.i64(i.immediate);code.op(0x7c);}
        }
        code.set(5);translate(pc,retired,size*(pair ? 2:1),load ? read_perm:write_perm);
        for(unsigned n=0;n<(pair ? 2u:1u);++n) {
            const unsigned r=n ? i.ra:i.rd,offset=n*size;
            if(vector) {
                for(unsigned lane=0;lane<(size==16 ? 2u:1u);++lane) {
                    if(load) {code.get(0);code.get(3);code.mem(size==4 ? 0x35:0x29,offset+lane*8);code.mem(0x37,vector_offset+r*16+lane*8);}
                    else {code.get(3);code.get(0);code.mem(0x29,vector_offset+r*16+lane*8);code.mem(size==4 ? 0x3e:0x37,offset+lane*8);}
                }
                if(load && size<16) {code.get(0);code.i64(0);code.mem(0x37,vector_offset+r*16+8);}
            } else {
                code.get(3);
                if(load) {
                    constexpr unsigned u[]={0x31,0x33,0x35,0x29},s[]={0x30,0x32,0x34,0x29};
                    unsigned shift=size==8 ? 3:size==4 ? 2:size==2 ? 1:0;
                    code.mem((i.flags&decoded::signed_load ? s:u)[shift],offset);
                    if((i.flags&(decoded::signed_load|decoded::word_result))==(decoded::signed_load|decoded::word_result))truncate(32);
                    if(r==31)code.op(0x1a);else code.set(gp+r);
                } else {reg(r);code.mem(size==8 ? 0x37:size==4 ? 0x3e:size==2 ? 0x3d:0x3c,offset);}
            }
        }
        if(!literal && (mode==decoded::post_index || mode==decoded::pre_index)) {
            // The saved effective address survives destination/base overlap.
            code.get(5);if(mode==decoded::post_index) {code.i64(i.immediate);code.op(0x7c);}code.set(gp+i.rn);
        }
    }
    Binary module() const {
        Binary out;out.append({0,97,115,109,1,0,0,0});
        Binary types;types.append({2,0x60,2,0x7f,0x7f,1,0x7f,0x60,3,0x7c,0x7c,0x7c,1,0x7c});out.section(1,types);
        Binary imports;imports.u(4);imports.name("env");imports.name("memory");imports.op(2);imports.op(0);imports.u(data_offset/65536);imports.name("env");imports.name("table");imports.append({1,0x70,0,0});imports.name("env");imports.name("fma");imports.op(0);imports.u(1);imports.name("env");imports.name("fma32");imports.op(0);imports.u(1);out.section(2,imports);
        Binary funcs;funcs.append({1,0});out.section(3,funcs);
        Binary exports;exports.u(2);exports.name("run");exports.append({0,2});exports.name("memory");exports.append({2,0});out.section(7,exports);
        Binary body;body.u(1);body.u(code.size());body.append(code);out.section(10,body);return out;
    }
};
std::vector<uint8_t> WasmRegion::dispatcher() {
    Binary code;
    code.u(1);code.u(4);code.op(0x7f); // retired, slot, module, reason
    code.i32(0);code.i32(0);code.mem(0x36,420);
    code.op(0x02);code.op(0x40);code.op(0x03);code.op(0x40);
    code.i32(0);code.mem(0x29,pc_offset);code.i64(2);code.op(0x88);
    code.i32(0);code.mem(0x29,pc_offset);code.i64(18);code.op(0x88);code.op(0x85);code.op(0xa7);
    code.i32(page_slots-1);code.op(0x71);code.i32(4);code.op(0x74);code.i32(code_table);code.op(0x6a);code.set(1);
    code.get(1);code.mem(0x29);code.i32(0);code.mem(0x29,pc_offset);code.op(0x52);
    code.get(1);code.mem(0x28,8);code.op(0x45);code.op(0x72);
    code.op(0x04);code.op(0x40);code.i32(Branch);code.set(3);code.op(0x0c);code.u(2);code.op(0x0b);
    code.get(1);code.mem(0x28,8);code.set(2);
    code.i32(0);code.get(1);code.mem(0x28,12);code.mem(0x36,entry_offset);
    code.i32(0);code.get(2);code.mem(0x36,384);
    code.i32(0);code.i32(0);code.mem(0x28,remaining_offset);code.get(2);code.op(0x11);code.u(0);code.u(0);code.set(3);
    code.get(0);code.i32(0);code.mem(0x28,retired_offset);code.op(0x6a);code.set(0);
    code.i32(0);code.get(0);code.mem(0x36,retired_offset);
    code.i32(0);code.i32(0);code.mem(0x28,388);code.i32(1);code.op(0x6a);code.mem(0x36,388);
    code.get(3);code.i32(Branch);code.op(0x46);code.op(0x0d);code.u(0);
    code.op(0x0b);code.op(0x0b);code.i32(0);code.get(0);code.mem(0x36,retired_offset);
    code.get(3);code.op(0x0b);
    Binary out;out.append({0,97,115,109,1,0,0,0});
    Binary types;types.append({2,0x60,2,0x7f,0x7f,1,0x7f,0x60,0,1,0x7f});out.section(1,types);
    Binary imports;imports.u(2);imports.name("env");imports.name("memory");imports.op(2);imports.op(0);imports.u(data_offset/65536);
    imports.name("env");imports.name("table");imports.append({1,0x70,0,0});out.section(2,imports);
    Binary funcs;funcs.append({1,1});out.section(3,funcs);
    Binary exports;exports.u(1);exports.name("run");exports.append({0,0});out.section(7,exports);
    Binary body;body.u(1);body.u(code.size());body.append(code);out.section(10,body);return out;
}
bool WasmRegion::supports(const Instruction &i) {return Compiler::supported(i);}
WasmRegion::WasmRegion(const std::vector<std::pair<uint64_t,uint32_t>> &words,uint64_t entry) {
    Compiler compiler(words,entry);bytes=compiler.module();
    for(auto [pc,selector]:compiler.entries)entries.emplace_back(pc,selector);
    instruction_count=words.size();block_count=compiler.graph->blocks.size();
    eliminated_instructions=compiler.graph->eliminated;
    for(const auto &block:compiler.graph->blocks)for(const auto &node:block->instructions) {
        constant_results+=node.constant_result && !node.dead_result;discarded_flags+=node.discard_flags;
    }
}
WasmRegion::WasmRegion(const std::vector<uint32_t> &words,uint64_t entry) : WasmRegion([&] {
    std::vector<std::pair<uint64_t,uint32_t>> result;
    for(unsigned n=0;n<words.size();++n)result.emplace_back(entry+4*n,words[n]);return result;
}(),entry) {}
}
