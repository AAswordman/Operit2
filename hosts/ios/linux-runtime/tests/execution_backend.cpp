#include "cpu/aarch64/execution_engine.h"
#if OPERIT_ARM64_GUEST_AOT
#include "cpu/aarch64/guest_aot.h"
#endif
#if OPERIT_ARM64_PROFILE_SUPERBLOCKS
#include "cpu/aarch64/profile_superblocks.h"
#endif
#include "execution_test_support.h"
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <stdexcept>
#include <unistd.h>
#include <sys/mman.h>
#include <algorithm>
#include <thread>
#include <unordered_set>
using namespace operit::arm64;
namespace {
void require(bool value,const char *why) {if(!value)throw std::runtime_error(why);}
void same(const CpuState &a,const CpuState &b) {
    require(a.x==b.x && a.sp==b.sp && a.pc==b.pc && a.v==b.v && a.nzcv==b.nzcv && a.fpcr==b.fpcr && a.fpsr==b.fpsr,"architectural state mismatch");
}
void map(Memory &m,const uint32_t *words,size_t count) {
    m.set_parallel_access(false);m.map(0x10000,page_size,read_perm|write_perm|exec_perm);m.copy_in(0x10000,words,count*4);
}
void lowering_address_independence() {
    const uint32_t words[]{0xf9400420,0xf9000862,0x394000a4,0x390000e6,0xb9400128,0xb900016a,
                           0xf8408dad,0xf80085ef,0x3dc00231,0x3d800273};
    std::vector<std::pair<uint64_t,uint32_t>> first,second;
    for(size_t n=0;n<std::size(words);++n) {
        first.emplace_back(0x10000+n*4,words[n]);second.emplace_back(0x20000+n*4,words[n]);
    }
    auto a=build_instruction_graph(first,0x10000);lower_graph(*a);
    auto b=build_instruction_graph(second,0x20000);lower_graph(*b);
    const auto &left=a->entries.at(0x10000)->code,&right=b->entries.at(0x20000)->code;
    require(left.size()==right.size(),"lowering changed with guest address");
    for(size_t n=2;n+1<left.size();++n) {
        const auto &x=left[n],&y=right[n];const auto i=operation_info(x),j=operation_info(y);
        require(x.entry==y.entry && x.immediate==y.immediate && x.a==y.a && x.b==y.b && x.d==y.d &&
                x.extra==y.extra && i.pc+0x10000==j.pc && i.code==j.code && i.prefix==j.prefix &&
                i.auxiliary==j.auxiliary,"lowering reused address-dependent metadata");
    }
}
void gp_operand_mapping_paths() {
    for(bool parallel:{false,true})for(unsigned bits:{32u,64u})for(unsigned reg=0;reg<32;++reg) {
        const uint32_t words[]{
            (bits==64 ? 0x8b000000u:0x0b000000u)|(reg<<16)|(31u<<5)|reg,
            (bits==64 ? 0x91000400u:0x11000400u)|(reg<<5)|31u,
            (bits==64 ? 0x91000800u:0x11000800u)|(31u<<5)|reg,
            0x14000000u};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        ExecutionEngine engine;
        for(unsigned budget=0;budget<=4;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x123456789abcdef0ull;actual.nzcv=0xa;
            for(unsigned i=0;i<actual.x.size();++i)actual.x[i]=0x1234567800000000ull+i*17;
            auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            require(engine.run(actual,memory,budget).instructions==budget,"GP operand mapping retirement mismatch");
            same(actual,expected);
        }
    }
}
void shifted_add_sub_paths() {
    struct Registers {unsigned d,a,b;};
    const Registers registers[]{{0,0,0},{13,13,13},{19,0,13},{13,19,30},{30,13,0},{31,0,13}};
    for(bool parallel:{false,true})for(unsigned bits:{32u,64u})for(unsigned mode=0;mode<3;++mode)
    for(const auto r:registers)for(unsigned shift:{1u,bits-1})for(unsigned sub=0;sub<2;++sub)
    for(unsigned flags=0;flags<2;++flags) {
        const uint32_t word=0x0b000000u | (bits==64 ? 0x80000000u:0u) | (sub<<30) | (flags<<29) |
            (mode<<22) | (r.b<<16) | (shift<<10) | (r.a<<5) | r.d;
        const uint32_t words[]{word,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        ExecutionEngine engine;
        for(uint64_t value:{0ull,1ull,0x8000000080000001ull,UINT64_MAX})for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[0]=value;actual.x[13]=~value;
            actual.x[19]=value^0x5555555555555555ull;actual.x[30]=value+1;actual.nzcv=0xa;
            auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;engine.run(actual,memory,budget);
            require(engine.executed_instructions-before==budget,"shifted add/sub retirement mismatch");same(actual,expected);
        }
    }
}
void vector_logical_paths() {
    struct Registers {unsigned d,a,b;};
    const Registers registers[]{{0,1,2},{0,0,2},{1,0,1},{0,0,0},{13,14,15},
                                {13,13,15},{13,14,13},{13,13,13},{7,13,2},{13,7,14},
                                {31,31,7},{31,7,31},{31,31,31}};
    for(bool parallel:{false,true})for(unsigned bits:{64u,128u})for(unsigned op=0;op<8;++op)
    for(const auto r:registers) {
        const uint32_t word=0x0e201c00u | (bits==128 ? 1u<<30:0) | ((op>>2)<<29) |
            ((op&3)<<22) | (r.b<<16) | (r.a<<5) | r.d;
        const uint32_t words[]{word,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto effects=instruction_effects(decode(word));
        const auto reads=(uint32_t{1}<<r.a)|(uint32_t{1}<<r.b)|(op>=5 ? uint32_t{1}<<r.d:0);
        require(effects.vector_read==reads && effects.vector_write==(uint32_t{1}<<r.d) &&
                !effects.gp_read && !effects.gp_write && !effects.flags_read && !effects.flags_write &&
                !effects.memory && !effects.control,"vector logical effects mismatch");
        const auto graph=build_graph(0x10000,memory);
        const auto role=[](unsigned reg){return reg<8 ? reg:8;};
        require(graph->entries.at(0x10000)->code[2].entry==
                static_entry(StaticCode::VectorLogical,bits,role(r.d),role(r.a),role(r.b),op),
                "vector logical native handler not installed");
        ExecutionEngine engine;
        for(unsigned seed:{0u,1u,93u})for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.nzcv=0xa;
            for(unsigned reg=0;reg<32;++reg)for(unsigned byte=0;byte<16;++byte)
                actual.v[reg][byte]=uint8_t(seed+reg*37+byte*19);
            auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;engine.run(actual,memory,budget);
            require(engine.executed_instructions-before==budget,"vector logical retirement mismatch");same(actual,expected);
        }
        memory.store<4>(0x10000,word^(1u<<30));
        CpuState actual;actual.pc=0x10000;actual.nzcv=7;
        for(unsigned reg=0;reg<32;++reg)for(unsigned byte=0;byte<16;++byte)
            actual.v[reg][byte]=uint8_t(reg*13+byte*29);
        auto expected=actual;
        for(unsigned n=0;n<2;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,2);same(actual,expected);
    }
}
void load_acquire_paths() {
    struct Registers {unsigned d,a;};
    const Registers registers[]{{0,1},{13,14},{30,13},{13,13},{31,1},{0,31}};
    for(bool parallel:{false,true})for(unsigned size=0;size<4;++size)for(const auto r:registers) {
        const uint32_t word=0x08dffc00u|(size<<30)|(r.a<<5)|r.d;
        const uint32_t words[]{word,0x14000000};
        Memory memory(1<<20);map(memory,words,2);memory.map(0x20000,page_size,read_perm|write_perm);
        memory.store<8>(0x20000,0xfedcba9876543210ull);memory.set_parallel_access(parallel);
        const auto instruction=decode(word);
        require(instruction.encoding==Encoding::LoadAcquire && instruction.width==(8u<<size),"load acquire decoding");
        const auto effects=instruction_effects(instruction);
        require(effects.memory && effects.gp_read==(uint32_t{1}<<r.a) &&
                effects.gp_write==(r.d==31 ? 0u:uint32_t{1}<<r.d),"load acquire effects");
        auto graph=build_graph(0x10000,memory);bool installed=false;
        for(const auto &op:graph->entries.at(0x10000)->code) {
            const auto info=operation_info(op);
            if(info.node && info.node->instruction.raw==word && info.code==StaticCode::LoadAcquire)installed=true;
        }
        require(installed,"load acquire native handler not installed");
        ExecutionEngine engine;
        for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.nzcv=0xa;actual.sp=0x20000;
            for(unsigned n=0;n<31;++n)actual.x[n]=0x20000;
            actual.exclusive_valid=true;actual.exclusive_address=0x34560;actual.exclusive_size=8;actual.exclusive_epoch=123;
            auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;engine.run(actual,memory,budget);
            require(engine.executed_instructions-before==budget,"load acquire retirement");same(actual,expected);
            require(actual.exclusive_valid==expected.exclusive_valid && actual.exclusive_address==expected.exclusive_address &&
                    actual.exclusive_size==expected.exclusive_size && actual.exclusive_epoch==expected.exclusive_epoch,
                    "load acquire modified exclusive monitor");
        }
    }
    for(bool parallel:{false,true})for(bool shared:{false,true}) {
        const uint32_t words[]{0xc8dffc02,0x17ffffff};
        Memory memory(1<<20);map(memory,words,2);memory.map(0x20000,page_size,read_perm|write_perm,false,shared);
        memory.store<8>(0x20000,42);memory.set_parallel_access(parallel);
        CpuState cpu;cpu.pc=0x10000;cpu.x[0]=0x20000;ExecutionEngine engine;
        engine.run(cpu,memory,100);const auto before=engine.slow_calls;engine.run(cpu,memory,1000);
        require(cpu.x[2]==42,"load acquire loop result");
        if(!parallel && !shared)require(engine.slow_calls-before<5,"private load acquire did not warm");
        else require(engine.slow_calls-before>100,"shared or parallel load acquire bypassed slow path");
    }
    for(bool denied:{false,true}) {
        const uint32_t words[]{0xc8dffc02,0x14000000};
        Memory memory(1<<20);map(memory,words,2);memory.map(0x20000,page_size,read_perm|write_perm);
        if(denied)memory.protect(0x20000,page_size,0);
        CpuState actual;actual.pc=0x10000;actual.x[0]=0x20000+!denied;actual.x[2]=91;actual.nzcv=3;
        auto expected=actual;bool reference_fault=false,actual_fault=false;ExecutionEngine engine;
        try {execute_one(expected,memory,decode(words[0]));}catch(const Failure &) {reference_fault=true;}
        try {engine.run(actual,memory,1);}catch(const Failure &) {actual_fault=true;}
        require(reference_fault && actual_fault && !engine.executed_instructions,"load acquire fault retirement");same(actual,expected);
    }
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),page_size),"load acquire file fixture");
    uint64_t value=71;require(pwrite(fileno(file.get()),&value,8,0)==8,"load acquire file contents");
    const uint32_t words[]{0xc8dffc02,0x17ffffff};Memory memory(1<<20);map(memory,words,2);
    Memory::Backing backing;memory.map_file(0x20000,page_size,read_perm,fileno(file.get()),backing);
    CpuState cpu;cpu.pc=0x10000;cpu.x[0]=0x20000;ExecutionEngine engine;engine.run(cpu,memory,100);
    const auto before=engine.slow_calls;engine.run(cpu,memory,100);
    require(cpu.x[2]==71 && engine.slow_calls-before>20,"file load acquire did not fall back");
    value=99;require(pwrite(fileno(file.get()),&value,8,0)==8,"load acquire file update");
    engine.run(cpu,memory,2);require(cpu.x[2]==99,"load acquire missed file update");
    Memory writer(1<<20);Memory::Backing shared_backing;shared_backing.shared=true;
    writer.map_file(0x30000,page_size,read_perm|write_perm,fileno(file.get()),shared_backing);
    writer.store<8>(0x30000,0);
    std::atomic<bool> stop{false};
    std::thread background([&]{for(unsigned n=0;n<20000 && !stop.load(std::memory_order_relaxed);++n)
        writer.store<8>(0x30000,n&1 ? UINT64_MAX:0);});
    bool consistent=true;
    try {for(unsigned n=0;n<200;++n) {engine.run(cpu,memory,100);consistent&=cpu.x[2]==0 || cpu.x[2]==UINT64_MAX;}}
    catch(...) {stop.store(true,std::memory_order_relaxed);background.join();throw;}
    stop.store(true,std::memory_order_relaxed);background.join();
    require(consistent,"load acquire file page bypassed exclusive lock");
    require(!ftruncate(fileno(file.get()),0),"load acquire file truncate");cpu.pc=0x10000;
    try {engine.run(cpu,memory,1);require(false,"load acquire ignored file truncation");}
    catch(const MemoryFault &fault) {require(fault.kind==MemoryFault::Kind::Bus && fault.address==0x20000,"load acquire file fault");}
    require(memory_detail::AccessSynchronization::read_depth()==0,"load acquire leaked reader lock");
}
void decoded_node_consistency() {
    std::vector<uint32_t> encodings{0,0xffffffff,0xd503201f,0x91000400,0x14000000,0xf9400000};
    uint32_t random=0x2d319ac7;
    for(unsigned n=0;n<2048;++n) {
        random^=random<<13;random^=random>>17;random^=random<<5;
        encodings.push_back(random);
    }
    // Rebuild at several guest addresses, including repeated and zero words.
    for(unsigned round=0;round<3;++round) {
        const uint64_t base=0x10000+round*0x10000;
        std::vector<std::pair<uint64_t,uint32_t>> words;
        for(size_t n=0;n<encodings.size();++n)words.emplace_back(base+n*4,encodings[n]);
        auto graph=build_instruction_graph(words,base);
        size_t checked=0;
        for(const auto &block:graph->blocks)for(const auto &node:block->instructions) {
            const auto index=(node.pc-base)/4;
            require(index<encodings.size(),"decoded node address outside input");
            const auto expected=decode(encodings[index]);
            const auto effects=instruction_effects(expected);
            const auto &actual=node.instruction;
            require(actual.immediate==expected.immediate && actual.auxiliary==expected.auxiliary &&
                    actual.raw==expected.raw && actual.op==expected.op && actual.encoding==expected.encoding &&
                    actual.rd==expected.rd && actual.rn==expected.rn && actual.rm==expected.rm &&
                    actual.ra==expected.ra && actual.width==expected.width && actual.shift==expected.shift &&
                    actual.flags==expected.flags && actual.terminal==expected.terminal,
                    "decoded node differs from direct decoder");
            require(node.effects.gp_read==effects.gp_read && node.effects.gp_write==effects.gp_write &&
                    node.effects.vector_read==effects.vector_read && node.effects.vector_write==effects.vector_write &&
                    node.effects.flags_read==effects.flags_read && node.effects.flags_write==effects.flags_write &&
                    node.effects.memory==effects.memory && node.effects.control==effects.control &&
                    node.semantic==select_semantic(expected),"decoded node analysis mismatch");
            ++checked;
        }
        require(checked==encodings.size(),"decoded node count mismatch");
    }
}
#if OPERIT_EXECUTION_DIAGNOSTICS
void diagnostic_recipe_paths() {
    const uint32_t words[]{
        0x8b0600a3, // add x3,x5,x6
        0xcb0614a4, // sub x4,x5,x6,lsl #5
        0xf94000a0, // ldr x0,[x5]
        0xf94004a1, // ldr x1,[x5,#8]
        0xa9000440, // stp x0,x1,[x2]
        0xf100043f, // cmp x1,#1
        0x54000040, // b.eq +8
        0xd28000e3, // mov x3,#7
        0x17fffff8, // b to start
    };
    Memory memory(1<<20);map(memory,words,std::size(words));
    auto graph=build_graph(0x10000,memory);
    size_t checked=0;
    for(const auto &block:graph->blocks) {
        require(block->recipes.size()==block->code.size(),"recipe and operation count mismatch");
        for(size_t i=0;i<block->code.size();++i) {
            const auto &recipe=block->recipes[i];
            if(!recipe.valid)continue;
            require(static_entry(recipe.code,recipe.bits,recipe.d,recipe.a,recipe.b,recipe.flags)==recipe.selected,
                    "diagnostic recipe selects different handler");
            ++checked;
        }
    }
    require(checked>=6,"too few diagnostic recipes checked");
}
#endif
#if OPERIT_ARM64_GUEST_AOT
#if OPERIT_ARM64_GUEST_AOT_TRACES && OPERIT_ARM64_GUEST_AOT_TRACE_MAX_BLOCKS >= 3
void long_guest_trace_paths() {
    for(bool parallel:{false,true})for(uint64_t address:{0x20000ull,0x30000ull})
    for(uint64_t first:{0ull,UINT64_MAX})for(uint64_t second:{0ull,UINT64_MAX}) {
        const uint32_t words[]{0x9100054a,0xb400006a,0x9100056b,0xb400006b,
                               0xf94000cb,0x9100056c,0x910005ad,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);m->store<8>(0x20000,42);
            m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const auto blocks=guest_trace_block_count(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(blocks==3,"three-block trace was not installed");
        if(blocks!=3)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<4;++pass) {
            if(pass==2)for(auto *m:{&memory,&reference})m->store<4>(0x10008,0x9100096b);
            if(pass==3)for(auto *m:{&memory,&reference})m->store<4>(0x10010,0xf94004cb);
            for(unsigned budget=0;budget<=8;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=first;
                actual.x[11]=second;actual.x[12]=17;actual.x[13]=19;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"three-block trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"three-block trace retirement mismatch");
            }
        }
    }
    for(bool parallel:{false,true})for(uint64_t address:{0x20000ull,0x30000ull})
    for(uint64_t condition:{0ull,UINT64_MAX})for(unsigned budget=0;budget<=7;++budget) {
        const uint32_t words[]{0x9100054a,0xb400006a,0xf94000cb,0x9100056c,
                               0x9100058d,0x910005ad,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);m->store<8>(0x20000,73);
            m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const auto blocks=guest_trace_block_count(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(blocks==3,"mixed three-block trace was not installed");
        if(blocks!=3)continue;
        CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=condition;
        actual.x[11]=3;actual.x[12]=5;actual.x[13]=7;actual.nzcv=9;
        auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
        try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
        catch(const MemoryFault &) {expected_fault=true;}
        ExecutionEngine engine;
        try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
        require(actual_fault==expected_fault,"mixed three-block trace fault mismatch");same(actual,expected);
        require(engine.executed_instructions==completed,"mixed three-block trace retirement mismatch");
    }
#if OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY
    for(bool parallel:{false,true})for(uint64_t source:{0x20000ull,0x30000ull})
    for(uint64_t target:{0x20000ull,0x30000ull})for(uint64_t offset:{0ull,8ull})
    for(uint64_t condition:{0ull,UINT64_MAX})for(unsigned budget=0;budget<=7;++budget) {
        const uint32_t words[]{0x9100054a,0xb400006a,0xf86968cb,0xb400006b,
                               0xf86968ec,0x9100058d,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);m->store<8>(0x20000,42);
            m->store<8>(0x20008,0);m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const auto blocks=guest_trace_block_count(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(blocks==3,"register memory three-block trace was not installed");
        if(blocks!=3)continue;
        CpuState actual;actual.pc=0x10000;actual.x[6]=source;actual.x[7]=target;
        actual.x[9]=offset;actual.x[10]=condition;actual.x[11]=3;actual.x[12]=5;actual.x[13]=7;actual.nzcv=9;
        auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
        try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
        catch(const MemoryFault &) {expected_fault=true;}
        ExecutionEngine engine;
        try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
        require(actual_fault==expected_fault,"register memory three-block trace fault mismatch");same(actual,expected);
        require(engine.executed_instructions==completed,"register memory three-block trace retirement mismatch");
    }
#endif
}
#endif
void guest_aot_integer_paths() {
    std::vector<uint32_t> instructions;
    for(unsigned width:{0u,1u}) {
        for(unsigned op:{2u,3u,8u,9u,10u,11u})
            instructions.push_back((width<<31)|0x1ac00000u|(11u<<16)|(op<<10)|(10u<<5)|12u);
        for(unsigned op:{0u,1u,2u,3u})for(unsigned condition:{0u,1u,10u,14u,15u})
            instructions.push_back((width<<31)|0x1a800000u|((op>>1)<<30)|((op&1)<<10)|
                                   (11u<<16)|(condition<<12)|(10u<<5)|12u);
    }
    const uint64_t values[]{0,1,31,32,63,64,65,0x80000000ull,0xffffffffull,
                            0x8000000000000000ull,0x7fffffffffffffffull,UINT64_MAX};
    for(uint32_t word:instructions)for(bool parallel:{false,true}) {
        const uint32_t words[]{word,0x910004a5,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"integer AOT fixture was not installed");
        if(length!=2)continue;
        ExecutionEngine engine;
        for(uint64_t left:values)for(uint64_t right:values)for(unsigned flags=0;flags<16;++flags)
        for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x20000;actual.x[5]=UINT64_MAX;
            actual.x[10]=left;actual.x[11]=right;actual.x[12]=0x123456789abcdef0ull;actual.nzcv=flags;
            auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;
            engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-before==budget,"integer AOT retirement mismatch");
        }
        // A cached translation must stop matching when its source instruction changes.
        memory.store<4>(0x10000,0x9100054c); // add x12,x10,#1
        CpuState actual;actual.pc=0x10000;actual.x[10]=19;actual.nzcv=9;auto expected=actual;
        for(unsigned i=0;i<2;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,2);same(actual,expected);
    }
}
void guest_aot_multiply_paths() {
    struct Registers {unsigned d,n,m,a;};
    const Registers registers[]{{12,10,11,13},{10,10,11,10},{11,10,11,11},{13,10,11,13},
                                {12,31,11,13},{12,10,31,13},{12,10,11,31},{31,10,11,13},
                                {0,0,2,3},{0,0,1,2},{19,20,30,0},{30,19,1,2},
                                {0,31,31,31},{31,0,1,31}};
    const uint64_t values[]{0,1,0xffffffffull,0x8000000000000000ull,UINT64_MAX};
    for(unsigned width:{0u,1u})for(unsigned sub:{0u,1u})for(const auto r:registers)
    for(bool parallel:{false,true}) {
        const uint32_t word=(width<<31)|0x1b000000u|(r.m<<16)|(sub<<15)|(r.a<<10)|(r.n<<5)|r.d;
        const uint32_t words[]{word,0x910004a5,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        // Constant folding reduces an all-zero MSUB to one record, so a
        // two-record AOT translation must not be installed for it.
        const bool folded=sub && r.n==31 && r.m==31 && r.a==31;
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT") && length!=2 && !folded) {
            char message[96];std::snprintf(message,sizeof(message),"multiply AOT fixture was not installed: 0x%08x",word);
            throw std::runtime_error(message);
        }
        if(folded)require(length!=2,"constant-folded multiply installed mismatched records");
        if(length!=2 && !folded)continue;
        ExecutionEngine engine;
        for(uint64_t left:values)for(uint64_t right:values)for(uint64_t addend:values)
        for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x20000;actual.x[5]=UINT64_MAX;
            actual.x[0]=left;actual.x[1]=right;actual.x[2]=addend;actual.x[3]=right;
            actual.x[19]=left;actual.x[20]=right;actual.x[30]=addend;
            actual.x[10]=left;actual.x[11]=right;actual.x[12]=0x123456789abcdef0ull;actual.x[13]=addend;actual.nzcv=9;
            auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;
            engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-before==budget,"multiply AOT retirement mismatch");
        }
        memory.store<4>(0x10000,word^(1u<<15));
        CpuState actual;actual.pc=0x10000;actual.x[10]=19;actual.x[11]=5;actual.x[13]=73;actual.nzcv=9;
        auto expected=actual;
        for(unsigned i=0;i<2;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,2);same(actual,expected);
    }
}
void guest_aot_shifted_paths() {
    std::vector<uint32_t> instructions;
    for(unsigned width:{0u,1u}) {
        for(unsigned sub:{0u,1u})instructions.push_back((width<<31)|0x0b000000u|(sub<<30)|
                                                        (11u<<16)|(6u<<10)|(10u<<5)|12u);
        for(unsigned op=0;op<4;++op)for(unsigned invert=0;invert<2;++invert)
            instructions.push_back((width<<31)|0x0a000000u|(op<<29)|(invert<<21)|
                                   (11u<<16)|(6u<<10)|(10u<<5)|12u);
        for(unsigned op:{0u,2u})instructions.push_back((width<<31)|0x0a000000u|(op<<29)|(1u<<21)|
                                                       (11u<<16)|(10u<<5)|12u);
    }
    for(const auto word:instructions)for(bool parallel:{false,true}) {
        const uint32_t words[]{word,0x9100054a,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"shifted AOT fixture was not installed");
        if(length!=2)continue;
        ExecutionEngine engine;
        for(uint64_t value:{0ull,1ull,0x8000000080000001ull,UINT64_MAX})for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[10]=value;actual.x[11]=~value;
            actual.x[12]=0x123456789abcdef0ull;actual.nzcv=9;auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-before==budget,"shifted AOT retirement mismatch");
        }
    }
#if OPERIT_ARM64_GUEST_AOT_BRANCHES
    // A sampled interpreter loop includes a writeback load followed by two
    // shifted arithmetic instructions and a backward conditional branch.
    for(bool parallel:{false,true})for(unsigned budget=0;budget<=29;++budget) {
        const uint32_t words[]{0x38401661,0x0b020022,0x0b022842,0x4a421842,0xeb13007f,0x54ffff61,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.map(0x20000,page_size,read_perm|write_perm);
        memory.store<8>(0x20000,0x123456789abcdef0ull);memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==6,"shifted loop AOT fixture was not installed");
        if(length!=6)continue;
        CpuState actual;actual.pc=0x10000;actual.x[19]=0x20000;actual.x[3]=0x20004;
        actual.x[2]=0x87654321;actual.nzcv=9;auto expected=actual;
        for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        ExecutionEngine engine;engine.run(actual,memory,budget);same(actual,expected);
        require(engine.executed_instructions==budget,"shifted loop AOT retirement mismatch");
    }
#endif
}
void guest_aot_unscaled_paths() {
    std::vector<uint32_t> instructions;
    for(unsigned shift=0;shift<4;++shift)for(unsigned load=0;load<2;++load)for(unsigned mode:{1u,3u})
        instructions.push_back(0x38000000u|(shift<<30)|(load<<22)|(8u<<12)|(mode<<10)|(6u<<5)|11u);
    for(uint32_t word:{0xf84084c6u,0xf8008cc6u,0xf85f8febu,0xf81f87ffu,0xf85f80cbu,0xf81f88cbu,
                       0xb85f84c6u,0xb81f8cc6u,0x38401661u,0xf8008c13u,0xf8408400u})
        instructions.push_back(word);
    for(const auto word:instructions)for(bool parallel:{false,true})for(bool readonly:{false,true})
    for(uint64_t address:{0x20020ull,0x20fffull,0x30000ull}) {
        const uint32_t words[]{word,0x9100054a,0x14000000};
        Memory actual_memory(1<<20),expected_memory(1<<20);
        for(auto *m:{&actual_memory,&expected_memory}) {
            map(*m,words,std::size(words));m->map(0x20000,page_size,read_perm|write_perm);
            m->store<8>(0x20018,0xfedcba9876543210ull);m->store<8>(0x20020,0x123456789abcdef0ull);
            m->store<8>(0x20028,0x9876543210abcdefull);m->store<8>(0x20ff8,0xaabbccddeeff1122ull);
            if(readonly)m->protect(0x20000,page_size,read_perm);
            m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,actual_memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"unscaled AOT fixture was not installed");
        if(length!=2)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<2;++pass)for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=address;actual.x[6]=address;
            actual.x[0]=address;actual.x[19]=address;
            actual.x[10]=UINT64_MAX;actual.x[11]=0x1122334455667788ull;actual.nzcv=9;
            auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
            try {for(;completed<budget;++completed)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &) {expected_fault=true;}
            const auto before=engine.executed_instructions;
            try {engine.run(actual,actual_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
            require(actual_fault==expected_fault,"unscaled AOT fault mismatch");same(actual,expected);
            require(engine.executed_instructions-before==completed,"unscaled AOT retirement mismatch");
            for(uint64_t at:{0x20018ull,0x20020ull,0x20028ull,0x20ff8ull})
                require(actual_memory.load<8>(at)==expected_memory.load<8>(at),"unscaled AOT memory mismatch");
        }
        for(auto *m:{&actual_memory,&expected_memory})m->store<4>(0x10000,0x910004c6);
        CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=7;auto expected=actual;
        for(unsigned i=0;i<2;++i)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,actual_memory,2);same(actual,expected);
    }
}
void guest_aot_paths() {
    guest_aot_integer_paths();
    guest_aot_multiply_paths();
    guest_aot_unscaled_paths();
    guest_aot_shifted_paths();
#if OPERIT_ARM64_GUEST_AOT_TRACES && OPERIT_ARM64_GUEST_AOT_TRACE_MAX_BLOCKS >= 3
    long_guest_trace_paths();
#endif
#if OPERIT_ARM64_GUEST_AOT_REGISTER_MEMORY
    for(unsigned option:{2u,3u,6u,7u})for(bool scaled:{false,true})for(bool writing:{false,true})
    for(bool parallel:{false,true})for(uint64_t offset:{0ull,1ull,0x7ffull,0xffffffffull,UINT64_MAX})
    for(unsigned budget=0;budget<=3;++budget) {
        const uint32_t memory_word=0xf8200800u|(writing ? 0u:1u<<22)|(9u<<16)|(option<<13)|
                                   (unsigned(scaled)<<12)|(6u<<5)|11u;
        const uint32_t words[]{0x9100054a,memory_word,0x9100056c,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);
            for(uint64_t p=0x20000;p<0x21000;p+=8)m->store<8>(p,p*0x102030405ull);
            m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==3,"register memory AOT fixture was not installed");
        if(length!=3)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==2)for(auto *m:{&memory,&reference})m->store<4>(0x10004,memory_word^(1u<<12));
            CpuState actual;actual.pc=0x10000;actual.x[6]=0x20800;actual.x[9]=offset;
            actual.x[10]=UINT64_MAX;actual.x[11]=0x123456789abcdef0ull;actual.x[12]=13;actual.nzcv=9;
            auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
            try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &) {expected_fault=true;}
            const auto before=engine.executed_instructions;
            try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
            require(actual_fault==expected_fault,"register memory AOT fault mismatch");same(actual,expected);
            require(engine.executed_instructions-before==completed,"register memory AOT retirement mismatch");
            std::array<uint8_t,page_size> actual_bytes{},expected_bytes{};
            memory.copy_out(0x20000,actual_bytes.data(),actual_bytes.size());
            reference.copy_out(0x20000,expected_bytes.data(),expected_bytes.size());
            require(actual_bytes==expected_bytes,"register memory AOT data mismatch");
        }
    }
    for(uint32_t word:{0x386968c6u,0x786968c9u,0xb87f68cbu,0xf8696bebu,
                       0x382968c6u,0x782968c9u,0xb83f68cbu,0xf8296bffu})
    for(bool parallel:{false,true})for(unsigned budget=0;budget<=3;++budget) {
        const uint32_t words[]{0x9100054a,word,0x9100056c,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);
            m->store<8>(0x20800,0x0123456789abcdefull);m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==3,"register memory alias AOT fixture was not installed");
        if(length!=3)continue;
        CpuState actual;actual.pc=0x10000;actual.sp=0x20800;actual.x[6]=0x20800;
        actual.x[9]=0;actual.x[10]=7;actual.x[11]=0xaabbccdd;actual.nzcv=9;
        auto expected=actual;
        for(unsigned n=0;n<budget;++n)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));
        ExecutionEngine engine;engine.run(actual,memory,budget);same(actual,expected);
        require(engine.executed_instructions==budget,"register memory alias AOT retirement mismatch");
        require(memory.load<8>(0x20800)==reference.load<8>(0x20800),"register memory alias AOT data mismatch");
    }
#endif
#if OPERIT_ARM64_GUEST_AOT_TRACES
    for(bool parallel:{false,true})for(uint64_t value:{0ull,1ull,UINT64_MAX})for(unsigned budget=0;budget<=5;++budget) {
        const uint32_t words[]{0x0b0b194c,0xb400006a,0x4a2b194c,0x9100054a,0x910005ad,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const bool installed=is_guest_trace_entry(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"shifted arithmetic trace was not installed");
        if(!installed)continue;
        CpuState actual;actual.pc=0x10000;actual.x[10]=value;actual.x[11]=~value;actual.nzcv=9;
        auto expected=actual;
        for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        ExecutionEngine engine;engine.run(actual,memory,budget);same(actual,expected);
        require(engine.executed_instructions==budget,"shifted arithmetic trace retirement mismatch");
    }
    for(bool parallel:{false,true})for(uint64_t source:{0x20000ull,0x30000ull})
    for(uint64_t target:{0x20000ull,0x30000ull})for(uint64_t condition:{0ull,UINT64_MAX}) {
        const uint32_t words[]{0x9100054a,0xf80084cb,0xb400006a,
                               0xf84084ec,0x9100058d,0x910005ad,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        for(auto *m:{&memory,&reference}) {
            map(*m,words,std::size(words));m->map(0x20000,page_size,read_perm|write_perm);
            m->store<8>(0x20000,42);m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const bool installed=is_guest_trace_entry(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"unscaled memory trace was not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==2)for(auto *m:{&memory,&reference})m->store<4>(0x1000c,0x9100054c);
            for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=source;actual.x[7]=target;
                actual.x[10]=condition;actual.x[11]=0x123456789abcdef0ull;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"unscaled trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"unscaled trace retirement mismatch");
                require(memory.load<8>(0x20000)==reference.load<8>(0x20000),"unscaled trace memory mismatch");
            }
        }
    }
    for(bool parallel:{false,true})for(uint64_t address:{0x20000ull,0x30000ull}) {
        const uint32_t words[]{0x9b0bb54c,0xb400006a,0xf94000cb,0x9100056c,0x910005ad,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));
        memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,0x123456789abcdef0ull);
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const bool installed=is_guest_trace_entry(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"multiply AOT trace was not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==2)memory.store<4>(0x10008,0x9100054b);
            for(uint64_t left:{0ull,1ull,UINT64_MAX})for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=left;
                actual.x[11]=3;actual.x[12]=7;actual.x[13]=17;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"multiply AOT trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"multiply AOT trace retirement mismatch");
            }
        }
    }
#if OPERIT_ARM64_GUEST_AOT_TRACE_REGISTER_MEMORY
    for(bool parallel:{false,true})for(uint64_t source:{0x20000ull,0x30000ull})
    for(uint64_t target:{0x20000ull,0x30000ull})for(unsigned budget=0;budget<=5;++budget) {
        const uint32_t words[]{0x9100054a,0xf82968cb,0xb400006a,
                               0xf86968ec,0x9100058d,0x910005ad,0x14000000};
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);m->store<8>(0x20000,42);m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const bool installed=is_guest_trace_entry(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"register memory store trace was not installed");
        if(!installed)continue;
        const auto trace=reinterpret_cast<uintptr_t>(graph->entries.at(0x10000)->code[2].entry);
        require(!is_guest_trace_entry(nullptr) && !is_guest_trace_entry(static_entry(StaticCode::Block)) &&
                !is_guest_trace_entry(reinterpret_cast<const void*>(trace+1)),"trace entry lookup accepted non-entry");
        CpuState actual;actual.pc=0x10000;actual.x[6]=source;actual.x[7]=target;actual.x[9]=0;
        actual.x[10]=0;actual.x[11]=0x123456789abcdef0ull;auto expected=actual;
        unsigned completed=0;bool expected_fault=false,actual_fault=false;
        try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
        catch(const MemoryFault &) {expected_fault=true;}
        ExecutionEngine engine;
        try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
        require(actual_fault==expected_fault,"register memory store trace fault mismatch");same(actual,expected);
        require(engine.executed_instructions==completed,"register memory store trace retirement mismatch");
        require(memory.load<8>(0x20000)==reference.load<8>(0x20000),"register memory store trace data mismatch");
    }
    for(bool parallel:{false,true})for(uint64_t source:{0x20000ull,0x30000ull})
    for(uint64_t target:{0x20000ull,0x30000ull})for(uint64_t condition:{0ull,UINT64_MAX}) {
        const uint32_t words[]{0x9100054a,0xf86968cb,0xb400006a,
                               0xf86968ec,0x9100058d,0x910005ad,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));
        memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,0x123456789abcdef0ull);
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const bool installed=is_guest_trace_entry(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"register memory conditional trace was not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<4;++pass) {
            if(pass==2)memory.store<4>(0x10010,0x9100098d);
            if(pass==3)memory.store<4>(0x10000,0x9100094a);
            for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=source;actual.x[7]=target;
                actual.x[9]=0;actual.x[10]=condition;actual.x[11]=3;actual.x[12]=5;actual.x[13]=7;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"register memory conditional trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"register memory conditional trace retirement mismatch");
            }
        }
    }
    for(bool parallel:{false,true})for(uint64_t address:{0x20000ull,0x30000ull}) {
        const uint32_t words[]{0xf86968cb,0x9100056c,0x9100058d,0xb5ffffc9};
        Memory memory(1<<20);map(memory,words,std::size(words));
        memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,0x123456789abcdef0ull);
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const bool installed=is_guest_trace_entry(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"register memory natural trace was not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==2)memory.store<4>(0x10004,0x9100096c);
            for(unsigned budget=0;budget<=3;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[9]=0;
                actual.x[11]=3;actual.x[12]=5;actual.x[13]=7;auto expected=actual;
                unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"register memory natural trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"register memory natural trace retirement mismatch");
            }
        }
    }
#endif
    for(bool memory_target:{false,true})for(bool parallel:{false,true})
    for(uint64_t address:{0x20000ull,0x30000ull}) {
        const uint32_t words[]{0x9100054a,0xb400006a,
                               memory_target ? 0xf94000cbu:0x9100054bu,0x9100056c,
                               0x910005ad,0x14000000};
        Memory memory(1<<20);map(memory,words,std::size(words));
        memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,0x123456789abcdef0ull);
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const auto &source=*graph->entries.at(0x10000);
        const bool installed=is_guest_trace_entry(source.code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"guest AOT trace fixture was not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==2)memory.store<4>(0x1000c,0x9100096c); // change the traced target block
            for(uint64_t x10:{0ull,UINT64_MAX})for(unsigned budget=0;budget<=7;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=x10;
                actual.x[11]=0x1234;actual.x[12]=0x5678;actual.x[13]=97;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"guest AOT trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"guest AOT trace retirement mismatch");
            }
        }
    }
    for(bool parallel:{false,true})for(uint64_t address:{0x20000ull,0x30000ull}) {
        // The backward branch makes the second instruction a leader, leaving
        // the first block with one instruction and a natural forward edge.
        const uint32_t words[]{0x9100054a,0xf94000cb,0x9100056c,0xb5ffffc9};
        Memory memory(1<<20);map(memory,words,std::size(words));
        memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,0x123456789abcdef0ull);
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        const auto &source=*graph->entries.at(0x10000);
        const bool installed=is_guest_trace_entry(source.code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"natural guest AOT trace fixture was not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==2)memory.store<4>(0x10008,0x9100096c);
            for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[9]=1;
                actual.x[10]=UINT64_MAX;actual.x[11]=0x1234;actual.x[12]=0x5678;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"natural guest AOT trace fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"natural guest AOT trace retirement mismatch");
            }
        }
    }
#endif
#if OPERIT_ARM64_GUEST_AOT_INDIRECT
    {
        const std::array<std::array<uint32_t,3>,3> programs{{
            {{0xf9400400,0xd65f03c0,0x14000000}}, // ldr x0,[x0,#8]; ret
            {{0x9100114a,0xd61f0140,0x14000000}}, // add x10,x10,#4; br x10
            {{0x910013de,0xd63f03c0,0x14000000}}, // add x30,x30,#4; blr x30
        }};
        const uint32_t target_words[]{0x9100056b,0x14000000};
        for(unsigned program=0;program<programs.size();++program)for(bool parallel:{false,true})
        for(uint64_t target:{0x20000ull,0x40000ull}) {
            Memory memory(1<<20);map(memory,programs[program].data(),programs[program].size());
            memory.map(0x20000,page_size,read_perm|write_perm|exec_perm);
            memory.copy_in(0x20000,target_words,sizeof(target_words));
            memory.map(0x30000,page_size,read_perm|write_perm);memory.store<8>(0x30008,0xfedcba9876543210ull);
            memory.set_parallel_access(parallel);
            auto graph=build_graph(0x10000,memory);
            const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
            if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"guest AOT indirect fixture was not installed");
            if(length!=2)continue;
            ExecutionEngine engine;
            for(unsigned pass=0;pass<3;++pass)for(unsigned budget=0;budget<=4;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[0]=0x30000;actual.x[10]=target-4;
                actual.x[30]=program==2 ? target-4:target;actual.x[11]=123;auto expected=actual;
                unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"guest AOT indirect fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"guest AOT indirect retirement mismatch");
            }
        }
        for(bool parallel:{false,true}) {
            Memory memory(1<<20);map(memory,programs[0].data(),programs[0].size());
            memory.map(0x20000,page_size,read_perm|write_perm|exec_perm);
            memory.copy_in(0x20000,target_words,sizeof(target_words));
            memory.map(0x30000,page_size,read_perm|write_perm);memory.store<8>(0x30008,42);
            memory.set_parallel_access(parallel);
            auto graph=build_graph(0x10000,memory);
            const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
            if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"guest AOT indirect invalidation fixture was not installed");
            if(length!=2)continue;
            ExecutionEngine engine;
            for(unsigned pass=0;pass<4;++pass) {
                if(pass==2)memory.store<4>(0x20000,0x9100096b); // add x11,x11,#2
                if(pass==3)memory.protect(0x20000,page_size,read_perm|write_perm);
                CpuState actual;actual.pc=0x10000;actual.x[0]=0x30000;actual.x[30]=0x20000;actual.x[11]=123;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<3;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,3);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"guest AOT indirect invalidation fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"guest AOT indirect invalidation retirement mismatch");
            }
        }
        {
            Memory memory(1<<20);map(memory,programs[0].data(),programs[0].size());
            const auto graph=build_graph(0x10000,memory);
            const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
            if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"guest AOT indirect load-fault fixture was not installed");
            if(length==2)for(unsigned budget=0;budget<=3;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[0]=0x30000;actual.x[30]=0x20000;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                ExecutionEngine engine;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"guest AOT indirect load fault mismatch");same(actual,expected);
                require(engine.executed_instructions==completed,"guest AOT indirect load-fault retirement mismatch");
            }
        }
    }
#endif
#if OPERIT_ARM64_GUEST_AOT_BRANCHES
    for(const uint32_t branch:{0x54000040u,0x54000041u,0x54000042u,0x54000043u,
                               0x54000044u,0x54000045u,0x54000046u,0x54000047u,
                               0x54000048u,0x54000049u,0x5400004au,0x5400004bu,
                               0x5400004cu,0x5400004du,0x5400004eu,0x5400004fu,
                               0xb400004au,0x3500004au,0x3700004au,0xb6f8004au}) {
        const uint32_t words[]{0x9100054a,branch,0x9100056b,0x9100096b,0xf94000c5};
        Memory branch_memory(1<<20);map(branch_memory,words,std::size(words));
        auto branch_graph=build_graph(0x10000,branch_memory);
        const auto &branch_block=*branch_graph->entries.at(0x10000);
        const auto length=guest_aot_entry_length(branch_block.code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"branch AOT fixture was not installed");
        if(length!=2)continue;
        ExecutionEngine engine;
        for(const uint64_t x10:{0ull,1ull,0x7ffffffffffffffeull,0xffffffffffffffffull})
            for(unsigned flags=0;flags<16;++flags)for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=0x30000;actual.x[10]=x10;
                actual.x[11]=0x1234;actual.nzcv=flags;auto expected=actual;
                unsigned completed=0;bool reference_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,branch_memory,decode(branch_memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,branch_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(reference_fault==actual_fault,"branch AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"branch AOT retirement mismatch");
            }
    }
    {
        // Pre-populate a distant target graph so the taken edge must use the
        // normal cross-graph link and be invalidated after guest code changes.
        const uint32_t words[]{0x9100054a,0xb407ffea,0xf94000c5};
        const uint32_t target[]{0x9100056b,0x14000000};
        Memory branch_memory(1<<20);map(branch_memory,words,std::size(words));
        branch_memory.map(0x20000,page_size,read_perm|write_perm|exec_perm);
        branch_memory.copy_in(0x20000,target,sizeof(target));
        auto graph=build_graph(0x10000,branch_memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==2,"cross-graph AOT fixture was not installed");
        if(length==2) {
            ExecutionEngine engine;CpuState warm;warm.pc=0x20000;engine.run(warm,branch_memory,1);
            for(unsigned pass=0;pass<3;++pass) {
                if(pass==2)branch_memory.store<4>(0x20000,0x9100096b);
                CpuState actual;actual.pc=0x10000;actual.x[10]=UINT64_MAX;actual.x[11]=7;auto expected=actual;
                for(unsigned n=0;n<3;++n)execute_one(expected,branch_memory,decode(branch_memory.load<4>(expected.pc,exec_perm)));
                engine.run(actual,branch_memory,3);same(actual,expected);
            }
        }
    }
#endif
    struct Case {std::array<uint32_t,4> words;};
    const Case cases[]{
        {{{0x9100054a,0x8b09014b,0xca0d016c,0xf94000c5}}},
        {{{0x1100054a,0x110043ff,0x2a0a03eb,0xf94000c5}}},
        {{{0xf2a2468a,0xca09014b,0x9100056c,0xf94000c5}}},
        {{{0x92401d6a,0xd348fd4b,0xb3401d6c,0xf94000c5}}},
        {{{0x12001d5f,0x110007eb,0x33001d6b,0xf94000c5}}},
        {{{0xb100054a,0xeb09015f,0xea09014b,0xf94000c5}}},
        {{{0x3100054a,0x72001d5f,0x1100056b,0xf94000c5}}},
    };
    for(const auto &test:cases) {
        // A fault after the pure sequence tests state publication and prefix.
        Memory memory(1<<20);map(memory,test.words.data(),test.words.size());
        auto graph=build_graph(0x10000,memory);
        const auto &block=*graph->entries.at(0x10000);
        if(!is_guest_aot_entry(block.code[2].entry)) {
            require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT fixture was not installed");
            continue;
        }
        ExecutionEngine engine;
        for(unsigned seed=0;seed<32;++seed)for(unsigned budget=0;budget<=5;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0xfedcba9876540000ull+seed;
            uint64_t state=0x9e3779b97f4a7c15ull+seed;
            for(auto &value:actual.x) {state^=state<<13;state^=state>>7;state^=state<<17;value=state;}
            actual.x[6]=0x30000;actual.nzcv=seed&15;auto expected=actual;
            bool reference_fault=false,actual_fault=false;unsigned completed=0;
            try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &) {reference_fault=true;}
            const auto before=engine.executed_instructions;
            try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
            require(reference_fault==actual_fault,"guest AOT fault mismatch");same(actual,expected);
            require(engine.executed_instructions-before==completed,"guest AOT retirement mismatch");
        }
        memory.store<4>(0x10000,0x9100094a); // add x10,x10,#2
        CpuState actual;actual.pc=0x10000;actual.x[10]=123;auto expected=actual;
        for(unsigned n=0;n<3;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,3);same(actual,expected);
    }
    {
        const uint32_t words[]{0x91000529,0x9100054a,0x9100056b,0x9100058c,
                               0x910005ad,0x910005ce,0x910005ef,0x91000610,
                               0x91000631,0x91000652,0x91000529,0x9100054a,0xf94000c5};
        Memory memory(1<<20);map(memory,words,std::size(words));
        auto graph=build_graph(0x10000,memory);
        const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==12,"long guest AOT fixture was not installed");
        if(length==12) {
            ExecutionEngine engine;
            for(unsigned seed=0;seed<32;++seed)for(unsigned budget=0;budget<=14;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=0x30000;actual.nzcv=seed&15;
                for(unsigned n=9;n<=18;++n)actual.x[n]=UINT64_MAX-seed*n;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"long guest AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"long guest AOT retirement mismatch");
            }
        }
    }
    // The first fixture sequence appears after a constant-folded producer.
    const uint32_t folded[]{0xd280008a,0x9100054a,0x8b09014b,0xca0d016c,0xf94000c5};
    Memory memory(1<<20);map(memory,folded,std::size(folded));
    auto graph=build_graph(0x10000,memory);
    const auto &block=*graph->entries.at(0x10000);
    require(block.instructions[1].constant_result,"guest AOT folded fixture was not optimized");
    bool folded_installed=false;
    for(const auto &op:block.code) {
        const auto info=operation_info(op);
        if(info.node && info.node->instruction.raw==0x9100054a) {
#if OPERIT_ARM64_GUEST_AOT_FOLDED
            folded_installed=is_guest_aot_entry(op.entry);
#else
            require(op.entry==static_entry(StaticCode::Move,64,6,7,7,0),"guest AOT installed into a folded block");
#endif
        }
    }
#if OPERIT_ARM64_GUEST_AOT_FOLDED
    if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(folded_installed,"folded AOT fixture was not installed");
    if(folded_installed) {
        ExecutionEngine engine;
        for(unsigned pass=0;pass<2;++pass) {
            // Changing the folded producer forces validation and rebuilding.
            if(pass)memory.store<4>(0x10000,0xd28000ea);
            for(unsigned seed=0;seed<32;++seed)for(unsigned budget=0;budget<=5;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=0x30000;actual.nzcv=seed&15;
                for(unsigned n=0;n<31;++n)if(n!=6)actual.x[n]=UINT64_MAX-seed*(n+1);
                auto expected=actual;unsigned completed=0;bool reference_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(reference_fault==actual_fault,"folded AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"folded AOT retirement mismatch");
            }
        }
    }
    for(const uint32_t address:{0x1000004au,0xb000000au})for(const uint64_t base:{0x10000ull,0x21000ull}) {
        const uint32_t words[]{address,0x9100054b,0xf94000c5};
        Memory address_memory(1<<20);address_memory.set_parallel_access(false);
        address_memory.map(base,page_size,read_perm|write_perm|exec_perm);
        address_memory.copy_in(base,words,sizeof(words));
        const auto address_graph=build_graph(base,address_memory);
        const auto &address_block=*address_graph->entries.at(base);
        require(address_block.instructions[0].constant_result && address_block.instructions[1].constant_result,
                "address fixture was not folded");
        const bool installed=guest_aot_entry_length(address_block.code[2].entry)==2;
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"address AOT fixture was not installed");
        if(installed) {
            ExecutionEngine engine;
            for(unsigned seed=0;seed<16;++seed)for(unsigned budget=0;budget<=3;++budget) {
                CpuState actual;actual.pc=base;actual.x[6]=0x30000;actual.x[10]=UINT64_MAX-seed;
                actual.x[11]=seed;actual.nzcv=seed;auto expected=actual;
                unsigned completed=0;bool reference_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,address_memory,decode(address_memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,address_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(reference_fault==actual_fault,"address AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"address AOT retirement mismatch");
            }
        }
    }
    {
        const uint32_t words[]{0xf94000cb,0x1000004a,0x9100054b,0x14000000};
        Memory address_memory(1<<20);map(address_memory,words,std::size(words));
        address_memory.map(0x20000,page_size,read_perm|write_perm);
        address_memory.store<8>(0x20000,0x123456789abcdef0ull);
        const auto address_graph=build_graph(0x10000,address_memory);
        const auto &address_block=*address_graph->entries.at(0x10000);
        const bool installed=guest_aot_entry_length(address_block.code[2].entry)==3;
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"post-load address AOT fixture was not installed");
        if(installed) {
            ExecutionEngine engine;
            for(const uint64_t address:{0x20000ull,0x30000ull})for(unsigned budget=0;budget<=4;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=99;actual.x[11]=77;
                auto expected=actual;unsigned completed=0;bool reference_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,address_memory,decode(address_memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,address_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(reference_fault==actual_fault,"post-load address AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"post-load address AOT retirement mismatch");
            }
        }
    }
#else
    (void)folded_installed;
#endif
    {
        // A folded instruction outside the replacement window must not
        // prevent the following unchanged operations from being translated.
        const uint32_t words[]{0xd2800090,0x9100054a,0x8b09014b,0xca0d016c,0xf94000c5};
        Memory partial_memory(1<<20);map(partial_memory,words,std::size(words));
        auto partial_graph=build_graph(0x10000,partial_memory);
        const auto &partial=*partial_graph->entries.at(0x10000);
        require(partial.instructions[0].constant_result,"partial AOT fixture was not optimized");
        bool installed=false;
        for(const auto &op:partial.code) {
            const auto info=operation_info(op);
            if(info.node && info.pc==0x10004 && is_guest_aot_entry(op.entry))installed=true;
        }
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"partial AOT fixture was not installed");
        if(installed) {
            ExecutionEngine engine;
            for(unsigned seed=0;seed<32;++seed)for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=0x30000;actual.nzcv=seed&15;
                for(unsigned n=0;n<31;++n)if(n!=6)actual.x[n]=UINT64_MAX-seed*(n+1);
                auto expected=actual;unsigned completed=0;bool reference_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,partial_memory,decode(partial_memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,partial_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(reference_fault==actual_fault,"partial AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"partial AOT retirement mismatch");
            }
        }
    }
    {
        // An eliminated write before the window has no StaticOperation; the
        // following sequence must still enter through its own metadata.
        const uint32_t words[]{0xd2800090,0xd28000b0,0x9100054a,0x8b09014b,0xca0d016c,0xf94000c5};
        Memory partial_memory(1<<20);map(partial_memory,words,std::size(words));
        auto partial_graph=build_graph(0x10000,partial_memory);
        const auto &partial=*partial_graph->entries.at(0x10000);
        require(partial.instructions[0].dead_result,"partial AOT dead-write fixture was not optimized");
        bool installed=false;
        for(const auto &op:partial.code) {
            const auto info=operation_info(op);
            if(info.node && info.pc==0x10008 && is_guest_aot_entry(op.entry))installed=true;
        }
        if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(installed,"partial AOT dead-write fixture was not installed");
        if(installed) {
            ExecutionEngine engine;
            for(unsigned budget=0;budget<=7;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[6]=0x30000;
                actual.x[9]=9;actual.x[10]=10;actual.x[13]=13;auto expected=actual;
                unsigned completed=0;bool reference_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,partial_memory,decode(partial_memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,partial_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(reference_fault==actual_fault,"partial AOT dead-write fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"partial AOT dead-write retirement mismatch");
            }
        }
    }
    const uint32_t memory_words[]{0x9100054a,0xf94000cb,0x8b0a016c,0xf90004cc,0x14000000};
    for(const uint64_t address:{0x20000ull,0x20ff8ull,0x20ffcull}) {
        Memory actual_memory(1<<20),expected_memory(1<<20);
        for(auto *m:{&actual_memory,&expected_memory}) {
            map(*m,memory_words,std::size(memory_words));
            m->map(0x20000,page_size,read_perm|write_perm);
            m->store<8>(0x20000,0x123456789abcdef0ull);
            m->store<8>(0x20ff8,0xfedcba9876543210ull);
        }
        auto memory_graph=build_graph(0x10000,actual_memory);
        const auto &memory_block=*memory_graph->entries.at(0x10000);
        if(!is_guest_aot_entry(memory_block.code[2].entry)) {
            require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT memory fixture was not installed");
            continue;
        }
        ExecutionEngine engine;
        for(unsigned budget=0;budget<=5;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[10]=0x1234567890ull;
            actual.nzcv=9;auto expected=actual;bool expected_fault=false,actual_fault=false;unsigned completed=0;
            try {for(;completed<budget;++completed)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &) {expected_fault=true;}
            const auto before=engine.executed_instructions;
            try {engine.run(actual,actual_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
            require(actual_fault==expected_fault,"guest AOT memory fault mismatch");same(actual,expected);
            require(engine.executed_instructions-before==completed,"guest AOT memory retirement mismatch");
            require(actual_memory.load<8>(0x20000)==expected_memory.load<8>(0x20000) &&
                    actual_memory.load<8>(0x20008)==expected_memory.load<8>(0x20008) &&
                    actual_memory.load<8>(0x20ff8)==expected_memory.load<8>(0x20ff8),"guest AOT memory contents mismatch");
        }
    }
    {
        // The load overwrites its own dirty address register with a W result.
        // Test cache hits, fallback, and faults with each retirement budget.
        const uint32_t words[]{0x9100214a,0xb940014a,0x9100054b,0x14000000};
        for(bool parallel:{false,true})for(uint64_t address:{0x20000ull,0x20fffull,0x30000ull}) {
            Memory actual_memory(1<<20),expected_memory(1<<20);
            for(auto *m:{&actual_memory,&expected_memory}) {
                map(*m,words,std::size(words));m->map(0x20000,page_size,read_perm|write_perm);
                m->store<4>(0x20000,0x87654321);m->set_parallel_access(parallel);
            }
            const auto graph=build_graph(0x10000,actual_memory);
            const auto length=guest_aot_entry_length(graph->entries.at(0x10000)->code[2].entry);
            if(std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"))require(length==3,"guest AOT alias fixture was not installed");
            if(length!=3)continue;
            ExecutionEngine engine;
            for(unsigned pass=0;pass<2;++pass)for(unsigned budget=0;budget<=4;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[10]=address-8;actual.x[11]=UINT64_MAX;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,actual_memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"guest AOT aliased load fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"guest AOT aliased load retirement mismatch");
            }
        }
    }
    for(unsigned size:{1u,2u,4u,8u})for(bool parallel:{false,true})for(bool readonly:{false,true}) {
        const auto shift=unsigned(__builtin_ctz(size));
        const uint32_t words[]{0x394000cbu|(shift<<30),0x390004cbu|(shift<<30),0x14000000};
        Memory actual_memory(1<<20),expected_memory(1<<20);
        for(auto *m:{&actual_memory,&expected_memory}) {
            map(*m,words,std::size(words));m->map(0x20000,page_size,read_perm|write_perm);
            m->store<8>(0x20000,0xfedcba9876543210ull);m->store<8>(0x20008,0x123456789abcdef0ull);
            if(readonly)m->protect(0x20000,page_size,read_perm);
            m->set_parallel_access(parallel);
        }
        auto graph=build_graph(0x10000,actual_memory);
        if(!is_guest_aot_entry(graph->entries.at(0x10000)->code[2].entry)) {
            require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT width fixture was not installed");
            continue;
        }
        ExecutionEngine engine;
        for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[6]=0x20000;actual.x[11]=UINT64_MAX;actual.nzcv=7;
            auto expected=actual;bool expected_fault=false,actual_fault=false;unsigned completed=0;
            uint64_t expected_address=0,actual_address=0;unsigned expected_permissions=0,actual_permissions=0;
            try {for(;completed<budget;++completed)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &e) {expected_fault=true;expected_address=e.address;expected_permissions=e.permissions;}
            const auto before=engine.executed_instructions;
            try {engine.run(actual,actual_memory,budget);}
            catch(const MemoryFault &e) {actual_fault=true;actual_address=e.address;actual_permissions=e.permissions;}
            require(actual_fault==expected_fault && actual_address==expected_address && actual_permissions==expected_permissions,
                    "guest AOT width or permission fault mismatch");same(actual,expected);
            require(engine.executed_instructions-before==completed,"guest AOT width retirement mismatch");
            require(actual_memory.load<8>(0x20000)==expected_memory.load<8>(0x20000) &&
                    actual_memory.load<8>(0x20008)==expected_memory.load<8>(0x20008),"guest AOT width contents mismatch");
        }
    }
    {
        const std::array<std::array<uint32_t,3>,5> pairs{{
            {{0xa94030cb,0xa90130cb,0x14000000}},
            {{0x294030cb,0x290130cb,0x14000000}},
            {{0x694030cb,0x9100056b,0x14000000}},
            {{0xa9c130cb,0xa88130cb,0x14000000}},
            {{0xa8c130c6,0x9100056b,0x14000000}},
        }};
        for(const auto &words:pairs)for(bool parallel:{false,true})for(bool readonly:{false,true})
        for(uint64_t address:{0x20000ull,0x20ff0ull,0x20ff8ull,0x21000ull})for(unsigned budget=0;budget<=3;++budget) {
            Memory actual_memory(1<<20),expected_memory(1<<20);
            for(auto *m:{&actual_memory,&expected_memory}) {
                map(*m,words.data(),words.size());m->map(0x20000,page_size,read_perm|write_perm);
                for(uint64_t p:{0x20000ull,0x20008ull,0x20010ull,0x20018ull,0x20ff0ull,0x20ff8ull})
                    m->store<8>(p,0xdeadbeef80000000ull+p);
                if(readonly)m->protect(0x20000,page_size,read_perm);
                m->set_parallel_access(parallel);
            }
            auto graph=build_graph(0x10000,actual_memory);
            if(!is_guest_aot_entry(graph->entries.at(0x10000)->code[2].entry)) {
                require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT pair fixture was not installed");
                continue;
            }
            CpuState actual;actual.pc=0x10000;actual.x[6]=address;actual.x[11]=71;actual.x[12]=83;actual.nzcv=9;
            auto expected=actual;bool expected_fault=false,actual_fault=false;unsigned completed=0;
            uint64_t expected_address=0,actual_address=0;unsigned expected_permissions=0,actual_permissions=0;
            MemoryFault::Kind expected_kind{},actual_kind{};
            try {for(;completed<budget;++completed)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &e) {expected_fault=true;expected_address=e.address;expected_permissions=e.permissions;expected_kind=e.kind;}
            ExecutionEngine engine;
            try {engine.run(actual,actual_memory,budget);}
            catch(const MemoryFault &e) {actual_fault=true;actual_address=e.address;actual_permissions=e.permissions;actual_kind=e.kind;}
            require(actual_fault==expected_fault && actual_address==expected_address && actual_permissions==expected_permissions &&
                    actual_kind==expected_kind,"guest AOT pair fault mismatch");same(actual,expected);
            require(engine.executed_instructions==completed,"guest AOT pair retirement mismatch");
            for(uint64_t p:{0x20000ull,0x20008ull,0x20010ull,0x20018ull,0x20ff0ull,0x20ff8ull})
                require(actual_memory.load<8>(p)==expected_memory.load<8>(p),"guest AOT pair memory mismatch");
        }
    }
    for(bool parallel:{false,true}) {
        std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
        require(bool(file) && !ftruncate(fileno(file.get()),page_size),"guest AOT pair file fixture");
        const uint32_t values[]{0x80000000u,0xffffffffu};
        require(pwrite(fileno(file.get()),values,sizeof(values),0)==sizeof(values),"guest AOT pair file write");
        const uint32_t words[]{0x694030cb,0x9100056b,0x14000000};
        Memory memory(4*page_size);map(memory,words,std::size(words));Memory::Backing backing;
        memory.map_file(0x40000,page_size,read_perm,fileno(file.get()),backing);memory.set_parallel_access(parallel);
        auto graph=build_graph(0x10000,memory);
        if(!is_guest_aot_entry(graph->entries.at(0x10000)->code[2].entry)) {
            require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT pair file fixture was not installed");
            continue;
        }
        ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[6]=0x40000;
        engine.run(cpu,memory,3);
        require(cpu.x[11]==0xffffffff80000001ull && cpu.x[12]==UINT64_MAX,"guest AOT signed pair file read mismatch");
        require(!ftruncate(fileno(file.get()),0),"guest AOT pair file truncate");
        cpu.pc=0x10000;const auto before=engine.executed_instructions;
        try {engine.run(cpu,memory,3);require(false,"guest AOT pair missed file truncation");}
        catch(const MemoryFault &fault) {require(fault.address==0x40000 && fault.kind==MemoryFault::Kind::Bus,"guest AOT pair file fault mismatch");}
        require(cpu.pc==0x10000 && engine.executed_instructions==before,"guest AOT pair file fault prefix");
    }
    // A generated store modifies an instruction in the block currently being
    // executed. The next entry must observe the new code, including when the
    // store falls back to the normal invalidation path.
    for(bool parallel:{false,true}) {
        const uint32_t words[]{0xb94000cb,0xb90004cb,0x14000000};
        Memory actual_memory(1<<20),expected_memory(1<<20);
        for(auto *m:{&actual_memory,&expected_memory}) {
            map(*m,words,std::size(words));m->set_parallel_access(parallel);
        }
        auto graph=build_graph(0x10000,actual_memory);
        if(!is_guest_aot_entry(graph->entries.at(0x10000)->code[2].entry)) {
            require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT self-modifying fixture was not installed");
            continue;
        }
        ExecutionEngine engine;CpuState actual;actual.x[6]=0x10000;auto expected=actual;
        for(unsigned pass=0;pass<2;++pass) {
            actual.pc=expected.pc=0x10000;
            for(unsigned n=0;n<3;++n)execute_one(expected,expected_memory,decode(expected_memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;
            engine.run(actual,actual_memory,3);same(actual,expected);
            require(engine.executed_instructions-before==3,"guest AOT self-modifying retirement mismatch");
            require(actual_memory.load<4>(0x10004)==expected_memory.load<4>(0x10004) &&
                    actual_memory.load<4>(0x10004)==words[0],"guest AOT self-modifying code mismatch");
        }
    }
    for(bool parallel:{false,true}) {
        std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
        require(bool(file) && !ftruncate(fileno(file.get()),page_size),"guest AOT file fixture");
        uint64_t value=37;require(pwrite(fileno(file.get()),&value,8,0)==8,"guest AOT file write");
        const uint32_t words[]{0xf94000cb,0x9100056c,0x17fffffe};
        Memory memory(4*page_size);map(memory,words,std::size(words));Memory::Backing backing;
        memory.map_file(0x40000,page_size,read_perm,fileno(file.get()),backing);memory.set_parallel_access(parallel);
        auto graph=build_graph(0x10000,memory);
        if(!is_guest_aot_entry(graph->entries.at(0x10000)->code[2].entry)) {
            require(!std::getenv("OPERIT_TEST_REQUIRE_GUEST_AOT"),"guest AOT file fixture was not installed");
            continue;
        }
        ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[6]=0x40000;
        engine.run(cpu,memory,300);require(cpu.x[11]==37 && cpu.x[12]==38,"guest AOT file read mismatch");
        value=91;require(pwrite(fileno(file.get()),&value,8,0)==8,"guest AOT file modification");
        cpu.pc=0x10000;engine.run(cpu,memory,3);require(cpu.x[11]==91 && cpu.x[12]==92,"guest AOT stale file read");
        require(!ftruncate(fileno(file.get()),0),"guest AOT file truncate");
        cpu.pc=0x10000;const auto before=engine.executed_instructions;
        try {engine.run(cpu,memory,3);require(false,"guest AOT missed file truncation");}
        catch(const MemoryFault &fault) {require(fault.address==0x40000 && fault.kind==MemoryFault::Kind::Bus,"guest AOT file fault mismatch");}
        require(cpu.pc==0x10000 && engine.executed_instructions==before,"guest AOT file fault prefix");
    }
}
#endif
#if OPERIT_ARM64_PROFILE_SUPERBLOCKS
#if OPERIT_ARM64_GUEST_AOT
void combined_aot_superblock_paths() {
    const uint32_t words[]{0x9100054a,0x8b09014b,0xca0d016c,
                           0xa9425bf5,0xa94363f7,0x14000000};
    for(bool parallel:{false,true}) {
        Memory memory(1<<20),reference(1<<20);
        map(memory,words,std::size(words));map(reference,words,std::size(words));
        for(auto *m:{&memory,&reference}) {
            m->map(0x20000,page_size,read_perm|write_perm);
            m->store<8>(0x20ff0,11);m->store<8>(0x20ff8,22);m->set_parallel_access(parallel);
        }
        const auto graph=build_graph(0x10000,memory);
        const auto &code=graph->entries.at(0x10000)->code;
        const bool installed=guest_aot_entry_length(code[2].entry)==3 &&
                             is_profile_superblock_entry(code[5].entry);
        if(std::getenv("OPERIT_TEST_REQUIRE_COMBINED_AOT"))require(installed,"combined AOT and superblock not installed");
        if(!installed)continue;
        ExecutionEngine engine;
        for(unsigned pass=0;pass<2;++pass) {
            if(pass)for(auto *m:{&memory,&reference})m->store<4>(0x10010,0xd503201f);
            for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.sp=0x20fd0;actual.x[9]=7;
                actual.x[10]=13;actual.x[11]=17;actual.x[12]=19;actual.x[13]=23;actual.nzcv=9;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,reference,decode(reference.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"combined AOT fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"combined AOT retirement mismatch");
            }
        }
    }
}
#endif
void profile_superblock_paths() {
    // The training profile contains this pair; other profiles may not.
    const uint32_t words[]{0xa9425bf5,0xa94363f7,0x14000000};
    Memory memory(1<<20);map(memory,words,std::size(words));
    memory.map(0x20000,page_size,read_perm|write_perm);
    memory.store<8>(0x20ff0,11);memory.store<8>(0x20ff8,22);
    auto graph=build_graph(0x10000,memory);
    const auto &block=*graph->entries.at(0x10000);
    const auto *plain=static_entry(StaticCode::PairLoad,64,6,5,6,0);
    if(block.code[2].entry==plain) {
        require(!std::getenv("OPERIT_TEST_REQUIRE_PROFILE_SUPERBLOCK"),"profiled pair was not installed");
        return;
    }
    ExecutionEngine engine;
    // Budget 3 admits the whole block, so the second load faults inside the
    // generated sequence; smaller budgets exercise the reference suffix.
    for(unsigned budget=0;budget<=3;++budget) {
        CpuState actual;actual.pc=0x10000;actual.sp=0x20fd0;auto expected=actual;
        bool reference_fault=false,actual_fault=false;
        try {for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
        catch(const MemoryFault &) {reference_fault=true;}
        const auto retired=engine.executed_instructions;
        try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
        require(reference_fault==actual_fault,"profiled pair fault mismatch");same(actual,expected);
        require(engine.executed_instructions-retired==std::min(budget,1u),"profiled pair retirement mismatch");
    }
    // Replacing the first instruction must invalidate any installed sequence.
    memory.store<4>(0x10000,0xd503201f);
    CpuState actual;actual.pc=0x10000;actual.sp=0x20fd0;auto expected=actual;
    execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
    engine.run(actual,memory,1);same(actual,expected);
}
#endif
// These are semantic differential tests: joins, backedges, entry state changes,
// and exact fault prefixes must agree with the independent instruction path.
void global_value_paths() {
    const uint32_t constant_join[]{
        0xd28000e3, // mov x3,#7
        0xb4000060, // cbz x0,+12
        0x91000421, // add x1,x1,#1
        0x14000002, // b +8
        0x91000821, // add x1,x1,#2
        0x91001464, // add x4,x3,#5 (constant across the diamond)
        0x14000000, // b .
    };
    const uint32_t unequal_join[]{
        0xb4000060,0xd28000e3,0x14000002,0xd2800103,0x91001464,0x14000000,
    };
    const uint32_t expressions[]{
        0x8b0600a3, // add x3,x5,x6
        0x14000001, // b +4
        0x8b0600a4, // add x4,x5,x6 (available in x3)
        0xcb0600a7, // sub x7,x5,x6 (a different expression)
        0x0b0600a8, // add w8,w5,w6 (different upper-half semantics)
        0x14000000,
    };
    const uint32_t loop_phi[]{
        0xd28000e3,0x14000001,0x91000463,0x91001464,0x17fffffe,
    };
    const uint32_t unavailable[]{
        0x8b0600a3, // dead x3 assignment must not become a copy source
        0x8b0600a4,0xd2800023,0x14000000,
    };
    const uint32_t zero_sp[]{
        0xaa1f03e3, // mov x3,xzr; ZR must never be identified with SP
        0x910003e4, // mov x4,sp
        0x91000485,0x14000000,
    };
    const uint32_t flags[]{
        0xf100003f, // cmp x1,#0
        0x14000001,
        0x9a820023, // csel x3,x1,x2,eq
        0xf100005f, // cmp x2,#0
        0x9a820024,0x14000000,
    };
    const uint32_t call_return[]{
        0xd28000e3, // mov x3,#7
        0x94000003, // bl +12
        0x91001464, // must see the callee's x3, not pre-call #7
        0x14000000,
        0xd2800163, // mov x3,#11
        0xd65f03c0, // ret
    };
    const std::vector<std::vector<uint32_t>> programs{
        {std::begin(constant_join),std::end(constant_join)},
        {std::begin(unequal_join),std::end(unequal_join)},
        {std::begin(expressions),std::end(expressions)},
        {std::begin(loop_phi),std::end(loop_phi)},
        {std::begin(unavailable),std::end(unavailable)},
        {std::begin(zero_sp),std::end(zero_sp)},
        {std::begin(flags),std::end(flags)},
        {std::begin(call_return),std::end(call_return)},
    };
    for(const auto &words:programs) {
        Memory memory(1<<20);map(memory,words.data(),words.size());ExecutionEngine engine;
        // Reuse one engine across arbitrary entry points, input values, and
        // budgets; this specifically exercises cached interior entry safety.
        for(unsigned input=0;input<5;++input)for(unsigned entry=0;entry<words.size();++entry)
            for(unsigned budget=0;budget<35;++budget) {
                CpuState actual;actual.pc=0x10000+entry*4;actual.sp=0x80000+input*16;
                for(unsigned r=0;r<31;++r)actual.x[r]=0x100000001ull*(r+input);
                actual.x[0]=input&1;actual.x[30]=0x1000c;actual.nzcv=input;
                auto expected=actual;
                for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
                require(engine.run(actual,memory,budget).instructions==budget,"SSA prefix retirement");
                try {same(actual,expected);}catch(...) {
                    std::fprintf(stderr,"SSA program=%08x input=%u entry=%u budget=%u pc=%llx expected=%llx\n",
                        words.front(),input,entry,budget,(unsigned long long)actual.pc,(unsigned long long)expected.pc);
                    for(unsigned r=0;r<31;++r)if(actual.x[r]!=expected.x[r])std::fprintf(stderr,"x%u=%llx expected=%llx\n",r,(unsigned long long)actual.x[r],(unsigned long long)expected.x[r]);
                    throw;
                }
            }
    }
#if OPERIT_ARM64_GLOBAL_VALUES
    Memory constants(1<<20);map(constants,constant_join,std::size(constant_join));
    auto cg=build_graph(0x10000,constants);
    require(cg->ssa_constants && cg->root_entry_only,"cross-block constant propagation inactive");
    Memory cse(1<<20);map(cse,expressions,std::size(expressions));auto eg=build_graph(0x10000,cse);
    require(eg->cse_copies && eg->root_entry_only,"global CSE inactive");
#endif
}
void randomized_value_paths() {
    uint64_t seed=0x6c22f3ca1918ull;
    const auto random=[&]() {seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;return seed;};
    const unsigned registers[]{0,1,2,3,4,5,6,7,8,9,23,30};
    const auto random_register=[&]() {return registers[random()%std::size(registers)];};
    for(unsigned trial=0;trial<32;++trial) {
        std::vector<uint32_t> words;
        for(unsigned b=0;b<8;++b) {
            const auto rn=random_register(),rm=random_register(),rd=random_register();
            const uint32_t opcodes[]{0x8b000000u,0xcb000000u,0x0b000000u,0x4b000000u,
                0xca000000u,0x4a000000u,0xaa000000u,0x8a000000u,0xca200000u,
                0x9a000000u,0xda000000u,0x9b007c00u,0x9ac02000u,0x9ac00800u,
                0x8b000400u,0xab000000u,0xeb000000u};
            const auto opcode=opcodes[random()%std::size(opcodes)];
            words.push_back(opcode|(rm<<16)|(rn<<5)|rd);
            words.push_back(0xb4000040u); // cbz x0,+8
            words.push_back(opcode|(rm<<16)|(rn<<5)|random_register());
            const auto immediate=unsigned(random()%32),source=random_register(),dest=random_register();
            words.push_back(0x91000000u|(immediate<<10)|(source<<5)|dest);
        }
        words.push_back(0x14000000u|((uint32_t(-int(words.size())))&0x3ffffffu));
        Memory memory(1<<20);map(memory,words.data(),words.size());ExecutionEngine engine;
        for(unsigned run=0;run<16;++run) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x90000;actual.nzcv=random()&15;
            for(auto &x:actual.x)x=random();actual.x[0]=run&1;
            auto expected=actual;const auto budget=100+unsigned(random()%300);
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            require(engine.run(actual,memory,budget).instructions==budget,"random SSA retirement");same(actual,expected);
        }
    }
}
void scheduled_load_paths() {
    // Reverse-address loads separated by dependent arithmetic: fetching the
    // second value early is safe only if its destination is not used there.
    const uint32_t words[]{0xf94004a0,0x91000402,0xf94000a1,0x17fffffd};
    for(bool parallel:{false,true}) {
        Memory memory(1<<20);map(memory,words,std::size(words));memory.map(0x20000,page_size*2,read_perm|write_perm);
        memory.store<8>(0x20000,0x123456789abcdef0ull);memory.store<8>(0x20008,0xfedcba9876543210ull);
        memory.set_parallel_access(parallel);ExecutionEngine engine;
        for(unsigned budget=0;budget<40;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[5]=0x20000;auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            require(engine.run(actual,memory,budget).instructions==budget,"scheduled load prefix retirement");same(actual,expected);
        }
#if OPERIT_ARM64_MEMORY_SCHEDULING
        auto graph=build_graph(0x10000,memory);require(graph->scheduled_loads==1,"load scheduler inactive");
#endif
#if OPERIT_ARM64_MEMORY_SCHEDULING
        if(!parallel) {
            ExecutionEngine hot;CpuState cpu;cpu.pc=0x10000;cpu.x[5]=0x20000;
            hot.run(cpu,memory,4000);
            require(hot.slow_calls<8,"scheduled load guard never warmed");
        }
#endif
        // Warm the cache, then exercise reversed loads across a page boundary.
        CpuState across;across.pc=0x10000;across.x[5]=0x20ff8;auto expected=across;
        for(unsigned n=0;n<4;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(across,memory,4);same(across,expected);
    }
    // Ascending loads: the second one faults. The first destination and the
    // intervening arithmetic must be visible, with precise PC and retirement.
    const uint32_t fault_words[]{0xf94000a0,0x91000402,0xf94004a1,0x14000000};
    for(bool parallel:{false,true}) {
        Memory memory(1<<20);map(memory,fault_words,std::size(fault_words));memory.map(0x20000,page_size,read_perm|write_perm);
        memory.store<8>(0x20ff8,123);memory.set_parallel_access(parallel);ExecutionEngine engine;
        CpuState actual;actual.pc=0x10000;actual.x[5]=0x20ff8;auto expected=actual;
        bool reference_fault=false,actual_fault=false;
        try {for(unsigned n=0;n<4;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
        catch(const MemoryFault &) {reference_fault=true;}
        try {engine.run(actual,memory,4);}catch(const MemoryFault &) {actual_fault=true;}
        require(reference_fault && actual_fault,"scheduled load lost fault");same(actual,expected);
        require(engine.executed_instructions==2,"scheduled load fault retirement");
    }
    // Signed unscaled offsets straddle zero in the decoded immediate. Sort
    // them as signed offsets, not as unsigned addresses.
    for(bool reverse:{false,true})for(unsigned width:{32u,64u}) {
        const uint32_t negative[]{(width==64 ? 0xf85f80a0u:0xb85fc0a0u),0x91000402u,
            (width==64 ? 0xf94000a1u:0xb94000a1u),0x17fffffdu};
        std::vector<uint32_t> program(std::begin(negative),std::end(negative));
        if(reverse) {std::swap(program[0],program[2]);program[0]=(program[0]&~31u)|0u;program[2]=(program[2]&~31u)|1u;}
        Memory memory(1<<20);map(memory,program.data(),program.size());memory.map(0x20000,page_size,read_perm|write_perm);
        memory.store<8>(0x20000,0xfedcba9876543210ull);memory.store<8>(0x20008,0x123456789abcdef0ull);
        CpuState actual;actual.pc=0x10000;actual.x[5]=0x20008;auto expected=actual;ExecutionEngine engine;
        for(unsigned n=0;n<40;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,40);same(actual,expected);
    }
#if OPERIT_ARM64_GLOBAL_VALUES && OPERIT_ARM64_MEMORY_SCHEDULING
    // CSE may introduce a copy source absent from the original read set.
    // The scheduler must account for that dependency too.
    const uint32_t copy_dependency[]{0x8b040061,0xf94000a0,0x8b040062,0xf94004a1,0x14000000};
    Memory copied(1<<20);map(copied,copy_dependency,std::size(copy_dependency));auto cg=build_graph(0x10000,copied);
    require(cg->cse_copies && !cg->scheduled_loads,"scheduler ignored a CSE copy dependency");
#endif
    // A second load that supplies an intervening operation, changes the base,
    // or crosses a store must not be moved before it.
    for(auto middle:{0x91000422u,0x910020a5u,0xf90004a2u}) {
        const uint32_t dependent[]{0xf94000a0,middle,0xf94004a1,0x14000000};
        Memory memory(1<<20);map(memory,dependent,std::size(dependent));auto graph=build_graph(0x10000,memory);
        require(!graph->scheduled_loads,"dependent load was reordered");
    }
}

void scheduled_file_loads() {
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),page_size),"scheduled file fixture");
    const uint32_t words[]{0xf94004a0,0x91000402,0xf94000a1,0x17fffffd};
    Memory reader(4*page_size),writer(2*page_size);map(reader,words,std::size(words));
    Memory::Backing private_backing,shared_backing;shared_backing.shared=true;
    reader.map_file(0x40000,page_size,read_perm,fileno(file.get()),private_backing);
    writer.map_file(0x50000,page_size,read_perm|write_perm,fileno(file.get()),shared_backing);
    for(bool parallel:{false,true}) {
        reader.set_parallel_access(parallel);ExecutionEngine engine;
        for(unsigned iteration=0;iteration<3;++iteration) {
            writer.store<8>(0x50000,11+iteration);writer.store<8>(0x50008,17+iteration);
            CpuState cpu;cpu.pc=0x10000;cpu.x[5]=0x40000;engine.run(cpu,reader,400);
            require(cpu.x[0]==17+iteration && cpu.x[1]==11+iteration && cpu.x[2]==18+iteration,
                "scheduled file load lost synchronized values");
            require(memory_detail::AccessSynchronization::read_depth()==0,"scheduled file guard leaked reader gate");
        }
    }
}
void prefixes() {
    const uint32_t words[]={0xd28000a0,0x91000c01,0xf100203f,0x54000040,0x91000422,0x8b010040,0x17fffffa};
    Memory memory(1<<20);map(memory,words,std::size(words));ExecutionEngine engine;
    for(unsigned budget=0;budget<150;++budget) {
        CpuState reference,actual;reference.pc=0x10000;reference.x[2]=9;reference.nzcv=13;actual=reference;
        for(unsigned n=0;n<budget;++n)execute_one(reference,memory,decode(memory.load<4>(reference.pc,exec_perm)));
        const auto result=engine.run(actual,memory,budget);same(actual,reference);require(result.instructions==budget,"retirement mismatch");
    }
    require(engine.fused_operations>0,"compare branch not fused");
}
// Cross many forward entries, the former 128-byte cuts, and loop backedges.
// Compare every budget prefix in serial and parallel execution modes.
void forward_regions() {
    std::array<uint32_t,97> words;
    for(unsigned n=0;n<32;++n) {
        words[n*3]=0x91000400; // add x0,x0,#1
        words[n*3+1]=0xb4000041; // cbz x1,+8
        words[n*3+2]=0x91000442; // add x2,x2,#1
    }
    words.back()=0x17ffffa0; // b -384
    for(bool parallel:{false,true}) {
        Memory memory(page_size);map(memory,words.data(),words.size());
        memory.set_parallel_access(parallel);ExecutionEngine engine;
        for(unsigned taken:{0u,1u})for(unsigned budget=0;budget<300;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[1]=taken;auto expected=actual;
            for(unsigned n=0;n<budget;++n)
                execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            require(engine.run(actual,memory,budget).instructions==budget,"forward region retirement");
            same(actual,expected);
        }
    }
}
void quantum_continuations() {
    // Every cut lands inside the same block. Resuming must not compile a
    // second graph rooted at each instruction of the interrupted block.
    const uint32_t words[]{0xd28000a0,0x91000c01,0x91000442,0x91000863,0xf100103f,0x17fffffbu};
    Memory memory(1<<20);map(memory,words,std::size(words));ExecutionEngine engine;
    CpuState actual,expected;actual.pc=expected.pc=0x10000;
    for(unsigned n=0;n<240;++n) {
        // A caller can edit registers between runs; execute the suffix from
        // the supplied architectural state, without earlier constant facts.
        actual.x[0]=expected.x[0]=n+17;
        actual.x[3]=expected.x[3]=n*31;
        const auto budget=1+n%4;
        for(unsigned k=0;k<budget;++k)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        const auto result=engine.run(actual,memory,budget);same(actual,expected);
        require(result.instructions==budget,"continuation retirement mismatch");
    }
    require(engine.decoded_blocks<=2,"quantum suffix created overlapping graphs");
    // Revoking or changing code between cuts must still be observed before
    // the first resumed instruction. The saved continuation owns no pointer.
    ExecutionEngine changed;actual={};actual.pc=0x10000;changed.run(actual,memory,1);
    memory.store<4>(0x10004,0x91002401);changed.run(actual,memory,1);
    require(actual.x[1]==14,"continuation ignored modified code");
    memory.protect(0x10000,page_size,read_perm|write_perm);
    const auto before=changed.executed_instructions;
    try {changed.run(actual,memory,1);require(false,"continuation ignored execute permission");}
    catch(const MemoryFault &) {}
    require(changed.executed_instructions==before && actual.pc==0x10008,"continuation fault was imprecise");
}
void native_system_and_conditions() {
    for(unsigned reg:{0u,17u,31u}) {
        const uint32_t words[]{0xd51bd040u|reg,0xd53bd041u,0xd51b4200u|reg,0xd53b4202u,0x17fffffcu};
        Memory memory(1<<20);map(memory,words,std::size(words));ExecutionEngine engine;
        for(unsigned flags=0;flags<16;++flags) {
            CpuState actual;actual.pc=0x10000;actual.nzcv=flags;actual.tls=0x123456789abcdef0ull;
            if(reg<31)actual.x[reg]=(uint64_t(flags)<<28)|0xabc;
            auto expected=actual;
            for(unsigned n=0;n<53;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            engine.run(actual,memory,53);same(actual,expected);
            require(actual.tls==expected.tls,"native TLS state mismatch");
        }
        require(engine.slow_calls<50,"TLS or NZCV still uses per-instruction helper");
    }
    for(unsigned wide:{0u,1u})for(unsigned subtract:{0u,1u})for(unsigned immediate:{0u,1u})for(unsigned condition=0;condition<16;++condition) {
        const uint32_t raw=0x3a400000u|(wide<<31)|(subtract<<30)|(condition<<12)|(immediate<<11)|(17u<<16)|(19u<<5)|11;
        const uint32_t words[]{raw,0x17ffffffu};Memory memory(1<<20);map(memory,words,2);ExecutionEngine engine;
        for(unsigned flags=0;flags<16;++flags) {
            CpuState actual;actual.pc=0x10000;actual.nzcv=flags;
            actual.x[19]=flags&1 ? 0x8000000000000000ull:flags&2 ? 0x7fffffffull:17;
            actual.x[17]=flags&4 ? UINT64_MAX:flags&8 ? 0x100000000ull:17;
            auto expected=actual;
            for(unsigned n=0;n<23;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            engine.run(actual,memory,23);same(actual,expected);
        }
        require(engine.slow_calls<40,"conditional compare still uses semantic helper");
    }
}
void native_scalar_moves() {
    for(bool wide:{false,true})for(unsigned operation=0;operation<3;++operation)for(unsigned rn:{1u,17u,31u})for(unsigned rd:{0u,17u,31u}) {
        const uint32_t raw=(wide ? 0x1e604000u:0x1e204000u)+(operation<<15)|(rn<<5)|rd;
        const uint32_t words[]{raw,0x17ffffffu};Memory memory(1<<20);map(memory,words,2);ExecutionEngine engine;
        for(uint64_t bits:{uint64_t{0},uint64_t{1},uint64_t{0x8000000000000000},uint64_t{0x7ff0000000000001},UINT64_MAX}) {
            CpuState actual;actual.pc=0x10000;actual.fpsr=0x08000001;
            for(auto &v:actual.v)v.fill(0xff);
            std::memcpy(actual.v[rn].data(),&bits,8);auto expected=actual;
            for(unsigned n=0;n<15;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            engine.run(actual,memory,15);same(actual,expected);
        }
        require(engine.slow_calls<15,"scalar move still uses semantic helper");
    }
}
void native_writebacks() {
    for(bool vector:{false,true})for(unsigned size:{1u,2u,4u,8u,16u}) {
        if(vector ? size<4:size>8)continue;
        const unsigned scale=unsigned(__builtin_ctz(size));
        for(unsigned op=0;op<(vector || size==8 ? 2u:size==4 ? 3u:4u);++op)
        for(unsigned mode:{1u,3u})for(unsigned rn:{2u,31u})for(unsigned rd:{6u,31u}) {
            const unsigned opc=vector ? op|(scale==4 ? 2:0):op;
            const uint32_t raw=0x38000000u|((scale&3)<<30)|(unsigned(vector)<<26)|(opc<<22)|
                               (uint32_t{0x1f8}<<12)|(mode<<10)|(rn<<5)|rd;
            const uint32_t words[]{raw,0x17ffffffu};Memory memory(3*page_size);map(memory,words,2);
            memory.map(0x20000,page_size,read_perm|write_perm);std::array<uint8_t,256> original{};
            for(unsigned n=0;n<original.size();++n)original[n]=uint8_t(n*47+3);
            memory.copy_in(0x20000,original.data(),original.size());
            CpuState actual;actual.pc=0x10000;actual.set(rn,0x20080,64,true);actual.x[6]=0xfedcba9876543210ull;
            for(unsigned n=0;n<16;++n){actual.v[6][n]=uint8_t(n+47);actual.v[31][n]=uint8_t(255-n);}
            auto expected=actual;
            for(unsigned n=0;n<8;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            std::array<uint8_t,256> expected_bytes{},actual_bytes{};memory.copy_out(0x20000,expected_bytes.data(),256);
            memory.copy_in(0x20000,original.data(),256);ExecutionEngine engine;engine.run(actual,memory,8);same(actual,expected);
            memory.copy_out(0x20000,actual_bytes.data(),256);require(actual_bytes==expected_bytes,"native writeback data mismatch");
            require(engine.slow_calls<4,"writeback still uses semantic helper");
            actual.pc=0x10000;actual.set(rn,0x30000,64,true);const auto before=engine.executed_instructions;
            try {engine.run(actual,memory,2);require(false,"writeback ignored fault");}catch(const MemoryFault &){}
            require(actual.get(rn,64,true)==0x30000 && actual.pc==0x10000 && engine.executed_instructions==before,"writeback fault changed base or retirement");
        }
    }
}
void native_file_reads() {
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),3*page_size),"native file fixture");
    for(unsigned n=0;n<3;++n){const uint64_t value=37+n;require(pwrite(fileno(file.get()),&value,8,n*page_size)==8,"native file contents");}
    const uint32_t words[]{0xf9400002u,0xf9480003u,0xf9500004u,0x17fffffdu};
    Memory memory(8*page_size);map(memory,words,4);Memory::Backing backing;
    memory.map_file(0x40000,3*page_size,read_perm,fileno(file.get()),backing);
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[0]=0x40000;
    engine.run(cpu,memory,400);require(cpu.x[2]==37 && cpu.x[3]==38 && cpu.x[4]==39,"native file read mismatch");
    const auto slow=engine.slow_calls;
    engine.run(cpu,memory,400);
    require(engine.slow_calls-slow<=2,"file reads reentered C++ per page in one observation");
    const uint64_t changed=91;require(pwrite(fileno(file.get()),&changed,8,page_size)==8,"native file pwrite");
    engine.run(cpu,memory,4);require(cpu.x[3]==changed,"native file read missed external modification");
    require(!ftruncate(fileno(file.get()),2*page_size),"native file truncate");
    cpu.pc=0x10000;const auto before=engine.executed_instructions;
    try {engine.run(cpu,memory,4);require(false,"native file read ignored truncation");}
    catch(const MemoryFault &fault){require(fault.address==0x42000 && fault.kind==MemoryFault::Kind::Bus,"native file fault kind");}
    require(cpu.pc==0x10008 && engine.executed_instructions-before==2,"native file fault prefix");
}
void native_file_page_lock() {
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),page_size),"native file lock fixture");
    const uint32_t words[]{0xad400420u,0x17ffffffu}; // ldp q0,q1,[x1]; b -4
    Memory reader(3*page_size),writer(2*page_size);map(reader,words,2);Memory::Backing private_backing,shared_backing;shared_backing.shared=true;
    reader.map_file(0x40000,page_size,read_perm,fileno(file.get()),private_backing);
    writer.map_file(0x50000,page_size,read_perm|write_perm,fileno(file.get()),shared_backing);
    std::array<uint8_t,32> bytes{};writer.copy_in<32>(0x50000,bytes.data());
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[1]=0x40000;engine.run(cpu,reader,4);
    std::atomic<bool> stop{false};
    std::thread background([&]{unsigned iteration=0;while(iteration<20000 && !stop.load(std::memory_order_relaxed)){bytes.fill(uint8_t(++iteration));writer.copy_in<32>(0x50000,bytes.data());if(!(iteration&63))std::this_thread::yield();}});
    bool consistent=true;
    try {
        for(unsigned n=0;n<32;++n) {
            engine.run(cpu,reader,4096);const auto first=cpu.v[0][0];
            for(unsigned k=0;k<16;++k)consistent&=cpu.v[0][k]==first && cpu.v[1][k]==first;
        }
    } catch(...) {stop.store(true,std::memory_order_relaxed);background.join();throw;}
    stop.store(true,std::memory_order_relaxed);background.join();
    require(consistent,"native file pair bypassed synchronized page reads");
    require(memory_detail::AccessSynchronization::read_depth()==0,"native reader gate leaked");
    // A writer must be able to acquire the same page after native return.
    bytes.fill(127);writer.copy_in<32>(0x50000,bytes.data());engine.run(cpu,reader,2);
    require(cpu.v[0][0]==127 && cpu.v[1][15]==127,"native file page lock leaked");
}
void loops() {
    const uint32_t words[]={0x91000400,0x91000821,0x17fffffe};Memory memory(1<<20);map(memory,words,3);ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;
    auto graph=build_graph(cpu.pc,memory);size_t count=0;for(auto &b:graph->blocks)count+=b->instructions.size();require(count==3,"loop unfolded");
    engine.run(cpu,memory,300000);require(cpu.x[0]==100000 && cpu.x[1]==200000,"resident loop values lost");
    require(engine.slow_calls<5 && engine.materializations<5,"loop crosses C++ state boundary");
    memory.store<4>(0x10000,0x91000c00);cpu.pc=0x10000;engine.run(cpu,memory,3);require(cpu.x[0]==100003,"modified code executed stale operations");
}
void cached_graph_boundaries() {
    // Two unrelated entries converge on one cached block. The second graph
    // must link to it rather than re-decode its entire reachable subgraph.
    const uint32_t words[]{0x14000008,0x14000007,0xd503201f,0xd503201f,
                           0xd503201f,0xd503201f,0xd503201f,0xd503201f,
                           0x91000400,0x14000000};
    for(bool parallel:{false,true}) {
        Memory memory(1<<20);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;
        engine.run(cpu,memory,4);
        require(cpu.x[0]==1,"first cached graph produced wrong result");
        const auto before=engine.decoded_blocks;
        cpu.pc=0x10004;engine.run(cpu,memory,4);
        require(cpu.x[0]==2,"cached successor did not execute");
        require(engine.decoded_blocks==before+1,"cached successor was compiled again");
        // Once the target code changes, the cross-graph edge must validate and
        // rebuild it instead of executing the old cached operations.
        memory.store<4>(0x10020,0x91000c00);
        cpu.pc=0x10004;engine.run(cpu,memory,4);
        require(cpu.x[0]==5,"cached boundary retained stale target code");
    }
}
void deferred_call_targets() {
    const uint32_t caller[]{0x94000400,0x9100056b,0x14000000}; // bl 0x11000; add x11; b .
    const uint32_t callee[]{0x9100054a,0xd65f03c0}; // add x10; ret
    for(bool parallel:{false,true}) {
        Memory memory(1<<20);map(memory,caller,std::size(caller));
        memory.map(0x11000,page_size,read_perm|write_perm|exec_perm);
        memory.copy_in(0x11000,callee,sizeof(callee));memory.set_parallel_access(parallel);
        const auto eager=build_graph(0x10000,memory);
        const auto deferred=build_graph(0x10000,memory,nullptr,nullptr,nullptr,GraphDiscoveryPolicy::DeferCallTargets);
        require(eager->entries.count(0x11000)==1,"eager call target not discovered");
        require(deferred->entries.count(0x11000)==0 && deferred->entries.count(0x10004)==1,
                "deferred call discovery lost return continuation or compiled callee");
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==1)memory.store<4>(0x11000,0x9100094a); // add x10,x10,#2
            if(pass==2)memory.protect(0x11000,page_size,read_perm|write_perm);
            for(unsigned budget=0;budget<=6;++budget) {
                CpuState actual;actual.pc=0x10000;actual.x[10]=13;actual.x[11]=17;actual.x[30]=19;
                auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {expected_fault=true;}
                const auto before=engine.executed_instructions;
                try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                require(actual_fault==expected_fault,"deferred call target fault mismatch");same(actual,expected);
                require(engine.executed_instructions-before==completed,"deferred call retirement mismatch");
            }
        }
    }
}
void loop_call_discovery() {
    const auto branch=[](uint64_t from,uint64_t to,bool link) {
        return (link ? 0x94000000u:0x14000000u)|uint32_t(((to-from)>>2)&0x03ffffff);
    };
    const auto conditional=[](uint64_t from,uint64_t to) {
        return 0xb5000000u|(uint32_t(((to-from)>>2)&0x7ffff)<<5); // cbnz x0
    };
    const uint32_t callee[]{0x9100054a,0xd65f03c0};
    for(bool reverse:{false,true})for(bool parallel:{false,true}) {
        Memory memory(1<<20);
        // In the first layout the call is discovered before its backedge; in
        // the second, a forward jump discovers the backedge before the call.
        const uint32_t forward[]{branch(0x10000,0x11000,true),0xd1000400, // sub x0,x0,#1
                                 conditional(0x10008,0x10000),branch(0x1000c,0x12000,true),0xd65f03c0};
        const uint32_t backward[]{branch(0x10000,0x10010,false),branch(0x10004,0x11000,true),
                                  branch(0x10008,0x12000,true),branch(0x1000c,0x10010,false),
                                  conditional(0x10010,0x10004),branch(0x10014,0x13000,true),0xd65f03c0};
        if(reverse)map(memory,backward,std::size(backward));else map(memory,forward,std::size(forward));
        for(uint64_t address:{0x11000,0x12000,0x13000}) {
            memory.map(address,page_size,read_perm|write_perm|exec_perm);
            memory.copy_in(address,callee,sizeof(callee));
        }
        memory.set_parallel_access(parallel);
        auto graph=build_graph(0x10000,memory,nullptr,nullptr,nullptr,GraphDiscoveryPolicy::DeferNonLoopCalls);
        require(graph->entries.count(0x11000),"loop call target not retained");
        require(graph->entries.count(0x12000)==size_t(reverse),"call range incorrectly retained or deferred");
        require(!graph->entries.count(0x13000),"cold call target unexpectedly retained");
        if(!reverse) {
            ExecutionEngine engine;
            for(unsigned pass=0;pass<3;++pass) {
                if(pass==1)memory.store<4>(0x11000,0x9100094a);
                if(pass==2)memory.protect(0x11000,page_size,read_perm|write_perm);
                for(unsigned budget=0;budget<=20;++budget) {
                    CpuState actual;actual.pc=0x10000;actual.x[0]=2;actual.x[10]=13;
                    auto expected=actual;unsigned completed=0;bool expected_fault=false,actual_fault=false;
                    try {for(;completed<budget;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                    catch(const MemoryFault &) {expected_fault=true;}
                    const auto before=engine.executed_instructions;
                    try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
                    require(actual_fault==expected_fault,"loop call fault mismatch");same(actual,expected);
                    require(engine.executed_instructions-before==completed,"loop call retirement mismatch");
                }
            }
        }
    }
}
#if OPERIT_ARM64_ADAPTIVE_CALLS
void adaptive_call_paths() {
    const uint32_t caller[]{0x94000400,0xd1000400,0xb5ffffc0,0x14000000}; // bl; sub; cbnz; b .
    const uint32_t callee[]{0x9100054a,0xd65f03c0};
    for(bool parallel:{false,true}) {
        Memory memory(1<<20);map(memory,caller,std::size(caller));
        memory.map(0x11000,page_size,read_perm|write_perm|exec_perm);
        memory.copy_in(0x11000,callee,sizeof(callee));memory.set_parallel_access(parallel);
        auto graph=build_graph(0x10000,memory,nullptr,[](const void *,uint64_t pc){return pc==0x11000;},nullptr,
                               GraphDiscoveryPolicy::DeferCallTargets,
                               [](const void *,uint64_t source,uint64_t target){return source==0x10000 && target==0x11000;},nullptr);
        require(graph->entries.count(0x11000),"hot call did not override cached target boundary");
        auto deferred=build_graph(0x10000,memory,nullptr,nullptr,nullptr,GraphDiscoveryPolicy::DeferCallTargets);
        const unsigned threshold=unsigned(std::min<size_t>(std::max<size_t>(deferred->blocks.size(),1),256)*4096);
        ExecutionEngine engine;CpuState actual;actual.pc=0x10000;actual.x[0]=threshold+200;actual.x[10]=3;
        auto expected=actual;
        for(unsigned budget:{5*threshold+1,1u,37u,500u}) {
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            engine.run(actual,memory,budget);same(actual,expected);
        }
        require(engine.adaptive_promotions>0,"hot call did not promote");
        memory.store<4>(0x11000,0x9100094a); // modify promoted callee
        for(unsigned n=0;n<50;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,50);same(actual,expected);
        memory.protect(0x11000,page_size,read_perm|write_perm);
        unsigned completed=0;bool reference_fault=false,actual_fault=false;
        try {for(;completed<20;++completed)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
        catch(const MemoryFault &) {reference_fault=true;}
        const auto before=engine.executed_instructions;
        try {engine.run(actual,memory,20);}catch(const MemoryFault &) {actual_fault=true;}
        require(reference_fault && actual_fault,"promoted call failed to fault on revoked execute permission");
        require(engine.executed_instructions-before==completed,"promoted call fault retirement mismatch");same(actual,expected);
    }
}
void adaptive_multiple_calls() {
    const uint32_t caller[]{0x94000400,0x940007ff,0xd1000400,0xb5ffffa0,0x14000000};
    const uint32_t targets[][2]{{0x9100054a,0xd65f03c0},{0x9100056b,0xd65f03c0},{0x9100058c,0xd65f03c0}};
    Memory memory(1<<20);map(memory,caller,std::size(caller));
    for(unsigned n=0;n<3;++n) {
        const auto pc=0x11000+n*page_size;memory.map(pc,page_size,read_perm|write_perm|exec_perm);
        memory.copy_in(pc,targets[n],sizeof(targets[n]));
    }
    ExecutionEngine engine;CpuState actual;actual.pc=0x10000;actual.x[0]=160000;auto expected=actual;
    const auto compare=[&](unsigned budget) {
        for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        const auto before=engine.executed_instructions;engine.run(actual,memory,budget);
        require(engine.executed_instructions-before==budget,"multiple hot calls retired wrong instruction count");same(actual,expected);
    };
    compare(8*60000);
    require(engine.adaptive_promotions>=2,"multiple hot calls did not each promote");
    // A rewritten BL at the same call PC must be eligible for a new target.
    memory.store<4>(0x10004,0x94000bff);
    compare(8*60000);
    require(engine.adaptive_promotions>=3,"modified hot call did not promote its new target");
}
#endif
void memory_and_fault() {
    const uint32_t words[]={0x91000400,0xf9400022,0x91000442,0xf9000022,0x17fffffc};Memory memory(1<<20);map(memory,words,5);
    memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,4);
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[1]=0x20000;
    engine.run(cpu,memory,5000);require(cpu.x[0]==1000 && memory.load<8>(0x20000)==1004,"native memory loop mismatch");
    cpu.pc=0x10000;cpu.x[1]=0x30000;auto retired=engine.executed_instructions;bool fault=false;
    try {engine.run(cpu,memory,5);}catch(const MemoryFault &){fault=true;}
    require(fault && cpu.pc==0x10004 && cpu.x[0]==1001 && engine.executed_instructions-retired==1,"imprecise load fault");
    cpu.pc=0x10000;cpu.x[1]=0x20000;memory.protect(0x20000,page_size,read_perm);retired=engine.executed_instructions;
    try {engine.run(cpu,memory,5);require(false,"store permission ignored");}catch(const MemoryFault &){}
    require(cpu.pc==0x1000c && engine.executed_instructions-retired==3,"imprecise store fault");
}
void native_pairs() {
    for(bool vector:{false,true})for(unsigned size:{4u,8u,16u}) {
        if(!vector && size==16)continue;
        for(bool load:{false,true})for(unsigned mode:{0u,1u,2u,3u})for(unsigned rn:{2u,31u}) {
            const unsigned opc=vector ? (size==4 ? 0:size==8 ? 1:2):(size==8 ? 2:0);
            const uint32_t raw=0x28000000u|(opc<<30)|(vector ? 1u<<26:0)|(mode<<23)|
                (load ? 1u<<22:0)|(uint32_t{2}<<15)|(uint32_t{20}<<10)|(rn<<5)|17;
            const uint32_t words[]{raw,0x17ffffffu};Memory memory(3*page_size);map(memory,words,2);
            memory.map(0x20000,page_size,read_perm|write_perm);
            std::array<uint8_t,64> bytes{};for(unsigned n=0;n<bytes.size();++n)bytes[n]=uint8_t(n*37+129);
            memory.copy_in(0x20000,bytes.data(),bytes.size());ExecutionEngine engine;
            for(unsigned sample=0;sample<8;++sample) {
                CpuState initial;initial.pc=0x10000;initial.set(rn,0x20000,64,true);
                initial.x[17]=0xfedcba9876543210ull;initial.x[20]=0x1234567887654321ull;
                for(unsigned n=0;n<16;++n){initial.v[17][n]=uint8_t(n+63);initial.v[20][n]=uint8_t(255-n);}
                auto actual=initial,expected=initial;
                execute_one(expected,memory,decode(raw));execute_one(expected,memory,decode(words[1]));
                std::array<uint8_t,64> expected_bytes{};memory.copy_out(0x20000,expected_bytes.data(),64);
                memory.copy_in(0x20000,bytes.data(),64);
                engine.run(actual,memory,2);same(actual,expected);
                std::array<uint8_t,64> actual_bytes{};memory.copy_out(0x20000,actual_bytes.data(),64);
                require(actual_bytes==expected_bytes,"native pair memory mismatch");
                memory.copy_in(0x20000,bytes.data(),64);
            }
            require(engine.slow_calls<=10,"warmed pair repeatedly uses semantic helper");
        }
    }
}
void pair_faults() {
    for (bool load : {false,true}) for (bool protection : {false,true}) {
        // add x5,x5,#1; ldp/stp x0,x1,[x2],#16. The second half faults.
        const uint32_t words[]{0x910004a5,load ? 0xa8c10440u : 0xa8810440u};
        Memory memory(4*page_size);map(memory,words,2);
        memory.map(0x20000,page_size,read_perm|write_perm);
        memory.store<8>(0x20ff8,0xdeadbeef);
        if(protection)memory.map(0x21000,page_size,0);
        CpuState cpu;cpu.pc=0x10000;cpu.x[0]=17;cpu.x[1]=23;cpu.x[2]=0x20ff8;
        ExecutionEngine engine;bool failed=false;
        try{engine.run(cpu,memory,2);}catch(const MemoryFault &fault){
            failed=true;
            require(fault.address==0x21000 && fault.permissions==(load ? read_perm:write_perm) &&
                    fault.kind==(protection ? MemoryFault::Kind::Protection:MemoryFault::Kind::Unmapped),
                    "pair fault lost address, permissions or kind");
        }
        require(failed && cpu.pc==0x10004 && cpu.x[0]==17 && cpu.x[1]==23 && cpu.x[2]==0x20ff8 &&
                cpu.x[5]==1 && engine.executed_instructions==1,"pair fault published partial state");
        require(memory.load<8>(0x20ff8)==0xdeadbeef,"pair store modified first element before fault");
    }
}
std::unique_ptr<BlockGraph> snapshot_graph(uint64_t pc,Memory &memory,size_t count) {
    auto graph=std::make_unique<BlockGraph>();graph->address=pc;
    while(count) {
        auto snapshot=memory.code_snapshot(pc);const auto offset=(pc%page_size)/4;
        const auto take=std::min<size_t>(count,page_size/4-offset);
        graph->dependencies.push_back({pc,snapshot.words,snapshot.proof,uint32_t(offset),uint32_t(take)});
        count-=take;pc+=take*4;
    }
    return graph;
}
bool equal(const CpuState &a,const CpuState &b) {
    return a.x==b.x && a.sp==b.sp && a.pc==b.pc && a.nzcv==b.nzcv && a.v==b.v && a.tls==b.tls && a.fpcr==b.fpcr && a.fpsr==b.fpsr;
}
void native_prefixes() {
    constexpr uint32_t words[]{0x91000400,0x91000c00,0xb1000801,0x9a010002,0xda010003,
        0xaa220004,0xd3407c85,0x93407ca6,0x9b061ca7,0x1a8104e8,0x910043ff,0xf100111f,0xd503201f,0x17fffff3};
    Memory memory(page_size);map(memory,words,std::size(words));ExecutionEngine engine;
    uint64_t seed=0x9e3779b97f4a7c15ULL;
    for(unsigned sample=0;sample<256;++sample) {
        CpuState initial;initial.pc=0x10000;initial.nzcv=sample&15;
        for(auto &x:initial.x) {seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;x=seed;}
        initial.sp=seed;
        for(unsigned budget=0;budget<=3*std::size(words);++budget) {
            auto actual=initial,expected=initial;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto result=engine.run(actual,memory,budget);
            require(result.instructions==budget && equal(actual,expected),"random native prefix mismatch");
        }
    }
}

// Run at a PC whose upper bits cannot fit in a compact immediate. This mixes
// a preparation record, two full-width bitfield masks, fused retirement and
// both call forms, then cuts the execution at every guest-instruction prefix.
void compact_operation_prefixes() {
    constexpr uint64_t base=0x00001234fffff000ull;
    constexpr uint32_t prepared_add=0x8b020c20u; // add x0,x1,x2,lsl #3
    constexpr uint32_t words[]{
        prepared_add,0xf2800000u|(3u<<21)|(0xcdefu<<5)|3u,
        0xb3400000u|(7u<<16)|(42u<<10)|4u, // bfm x4,x0,#7,#42
        0xf100041fu,0x54000040u,0xca030084u,
        0x94000004u,0xd63f0180u,0x910004a5u,0x17fffff7u,
        0x910004c6u,0xd65f03c0u,0x910004e7u,0xd65f03c0u
    };
    Memory memory(page_size);memory.set_parallel_access(false);
    memory.map(base,page_size,read_perm|write_perm|exec_perm);
    memory.copy_in(base,words,sizeof(words));ExecutionEngine engine;
    for(unsigned sample=0;sample<2;++sample)for(unsigned budget=0;budget<85;++budget) {
        CpuState actual;actual.pc=base;actual.nzcv=13;
        actual.x[1]=sample ? 17:1;actual.x[2]=sample ? 3:0;
        actual.x[3]=0x123456789abcdef0ull;actual.x[4]=0xfedcba9876543210ull;
        actual.x[12]=base+12*4;auto expected=actual;
        for(unsigned n=0;n<budget;++n)
            execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        const auto result=engine.run(actual,memory,budget);
        require(result.instructions==budget,"compact high-PC retirement mismatch");same(actual,expected);
    }
    // An extra preparation record must not shift the fault's guest prefix.
    // Reusing this engine also exercises code invalidation with cold metadata.
    constexpr uint32_t fault_words[]{prepared_add,0xf9400124u,0x17fffffeu};
    memory.copy_in(base,fault_words,sizeof(fault_words));
    CpuState actual;actual.pc=base;actual.x[1]=9;actual.x[2]=3;
    actual.x[4]=0xfedcba9876543210ull;actual.x[9]=base+2*page_size;
    auto expected=actual;execute_one(expected,memory,decode(fault_words[0]));
    const auto before=engine.executed_instructions;bool faulted=false;
    try {engine.run(actual,memory,3);}catch(const MemoryFault &){faulted=true;}
    require(faulted && engine.executed_instructions-before==1,"compact preparation fault retirement mismatch");
    same(actual,expected);
}

// Exercise both outcomes, W/X widths, high bits, XZR and every condition.
// The comparison stays in the independent scalar semantics; short budgets
// also force a cold exit after every possible instruction in the CFG.
void local_edge_targets() {
#if OPERIT_ARM64_DIRECT_CHAINS
    require(static_entry(StaticCode::Next,64,7,7,7,1)!=static_entry(StaticCode::Jump,64,7,7,7,1),
            "forward fallthrough must not share a handler with backedges");
#endif
    const uint32_t words[]{0x54000060,0x91000631,0x14000002,0x91000a31,0x17fffffc};
    Memory memory(page_size);map(memory,words,std::size(words));
    auto graph=build_graph(0x10000,memory);
    unsigned local_next=0,backedges=0;
    for(const auto &block:graph->blocks)for(const auto &op:block->code) {
        const auto info=operation_info(op);
        if(!op.target)continue;
        if(info.code==StaticCode::Next && op.entry==static_entry(StaticCode::Next,64,7,7,7,1)) {
            const auto *target=static_cast<const StaticOperation*>(op.target);
            require(target->index==1 && operation_info(*target).code==StaticCode::BlockBody,
                    "local fallthrough does not target a block body");
            ++local_next;
        }
        if(info.code==StaticCode::Jump && info.node && info.node->instruction.immediate>>63) {
            const auto *target=static_cast<const StaticOperation*>(op.target);
            require(target->index==0 && operation_info(*target).code==StaticCode::Block,
                    "local backedge bypasses block guard");
            ++backedges;
        }
    }
    require(local_next && backedges,"local edge layout fixture was incomplete");
}

void unordered_instruction_graph() {
    const std::vector<std::pair<uint64_t,uint32_t>> words{
        {0x10010,0xd65f03c0}, {0x10008,0x14000002}, {0x10000,0x54000040},
        {0x1000c,0x91000631}, {0x10004,0x91000631}
    };
    const auto graph=build_instruction_graph(words,0x10000);
    const std::vector<std::vector<uint64_t>> edges{
        {0x10008,0x10004}, {0x10008}, {0x10010}, {0x10010}, {}
    };
    require(graph->blocks.size()==edges.size(),"unordered graph block count");
    for(size_t i=0;i<edges.size();++i) {
        const auto &block=*graph->blocks[i];
        require(block.pc==0x10000+i*4 && block.instructions.size()==1 && block.successors==edges[i],
                "unordered graph partition or successors");
        require(graph->entries.at(block.pc)==&block,"unordered graph entry mapping");
    }
    const std::vector<std::pair<uint64_t,uint32_t>> multi{
        {0x10020,0x91000631},{0x10010,0x91000631},{0x10000,0x91000631},
        {0x10018,0xd65f03c0},{0x10008,0xb4000060},{0x10004,0x91000631},
        {0x10024,0xd65f03c0},{0x10014,0x91000631},{0x1000c,0x91000631}
    };
    const auto partitioned=build_instruction_graph(multi,0x10000);
    const std::vector<uint64_t> starts{0x10000,0x1000c,0x10014,0x10020};
    const std::vector<size_t> lengths{3,2,2,2};
    const std::vector<std::vector<uint64_t>> exits{{0x10014,0x1000c},{0x10014},{},{}};
    require(partitioned->blocks.size()==starts.size(),"multi-instruction graph block count");
    for(size_t i=0;i<starts.size();++i) {
        const auto &block=*partitioned->blocks[i];
        require(block.pc==starts[i] && block.instructions.size()==lengths[i] &&
                block.successors==exits[i] && partitioned->entries.at(starts[i])==&block,
                "multi-instruction graph partition or successors");
        for(size_t j=0;j<lengths[i];++j)
            require(block.instructions[j].pc==starts[i]+4*j,"multi-instruction graph order");
    }
    for(unsigned mode=0;mode<3;++mode) {
        auto bad=words;
        if(mode==0)bad.push_back(words.front());
        if(mode==1)bad.push_back({0x10001,0xd503201f});
        bool rejected=false;
        try { (void)build_instruction_graph(bad,mode==2 ? 0x20000:0x10000); }
        catch(const Failure &) {rejected=true;}
        require(rejected,"invalid instruction graph accepted");
    }
}

void partition_leader_reference() {
    uint32_t random=0x6d2b79f5;
    const auto next=[&]() {random^=random<<13;random^=random>>17;random^=random<<5;return random;};
    for(unsigned run=0;run<82;++run) {
        const unsigned count=run==80 ? 600:run==81 ? 1024:16+next()%145;
        std::vector<std::pair<uint64_t,uint32_t>> words;words.reserve(count);
        uint64_t address=run==0 ? UINT64_MAX-67:0x10000;
        for(unsigned i=0;i<count;++i) {
            uint32_t word=0x91000400;
            const auto choice=next()%8;
            const auto displacement=int32_t(next()%65)-32;
            if(choice==0)word=0x14000000u|(uint32_t(displacement)&0x03ffffffu);
            else if(choice==1)word=0x54000000u|((uint32_t(displacement)&0x7ffffu)<<5);
            else if(choice==2)word=0xd65f03c0;
            words.emplace_back(address,word);
            address+=4*(1+(next()%9==0));
        }
        std::sort(words.begin(),words.end());
        const uint64_t entry=words[next()%count].first;
        std::unordered_set<uint64_t> leaders{entry};
        for(size_t i=0;i<words.size();++i) {
            const auto [pc,word]=words[i];
            const auto ends=[](uint32_t raw) {const auto op=decode(raw).op;
                return op==Op::Branch || op==Op::ConditionalBranch || op==Op::BranchRegister;};
            const auto *previous=i ? &words[i-1]:nullptr;
            if(pc==0 && words.back().first==UINT64_MAX-3)previous=&words.back();
            if(!previous || previous->first!=pc-4 || ends(previous->second))leaders.insert(pc);
            const auto instruction=decode(word);
            if(instruction.op==Op::Branch || instruction.op==Op::ConditionalBranch) {
                leaders.insert(pc+instruction.immediate);
                if(instruction.op==Op::ConditionalBranch)leaders.insert(pc+4);
            }
        }
        std::reverse(words.begin(),words.end());
        const auto graph=build_instruction_graph(words,entry);
        std::reverse(words.begin(),words.end());
        require(graph->node_storage.size()==words.size(),"partition lost graph-owned instructions");
        size_t block=0;
        for(size_t start=0;start<words.size();) {
            size_t end=start+1;while(end<words.size() && !leaders.count(words[end].first))++end;
            require(block<graph->blocks.size() && graph->blocks[block]->pc==words[start].first &&
                    graph->blocks[block]->instructions.size()==end-start,"partition leader reference mismatch");
            require(graph->blocks[block]->instructions.begin()==graph->node_storage.data()+start,
                    "partition instruction ownership mismatch");
            for(size_t n=start;n<end;++n)
                require(graph->blocks[block]->instructions[n-start].pc==words[n].first,"partition instruction order mismatch");
            ++block;start=end;
        }
        require(block==graph->blocks.size(),"partition unexpected block");
    }
}

void discovery_bitmap_boundaries() {
    // Both branch outcomes converge, and the last instruction loops back.
    // Exercise bit 63/64 and the final two slots of one page into the next.
    for(const uint64_t offset:{uint64_t{63*4},uint64_t{page_size-8}}) {
        const uint64_t start=0x10000+offset;
        const uint32_t words[]{0x54000040,0x14000001,0x17fffffe};
        Memory memory(2*page_size);memory.set_parallel_access(false);
        memory.map(0x10000,2*page_size,read_perm|write_perm|exec_perm);
        memory.copy_in(start,words,sizeof(words));
        const auto graph=build_graph(start,memory);
        size_t count=0;
        for(const auto &block:graph->blocks)count+=block->instructions.size();
        require(count==3 && graph->entries.size()==3,"bitmap discovery duplicated or lost an instruction");
        require(graph->entries.at(start)->successors==std::vector<uint64_t>{start+8,start+4},
                "bitmap discovery changed conditional edges");
        require(graph->entries.at(start+8)->successors==std::vector<uint64_t>{start},
                "bitmap discovery changed backedge");
        require(graph->dependencies.size()==(offset==page_size-8 ? 2u:1u),
                "bitmap discovery changed page dependencies");
    }
}

void discovery_limit_cross_graph() {
    constexpr uint64_t base=0x10000;
    std::vector<uint32_t> words(1200,0x91000400); // add x0,x0,#1
    words.push_back(0x14000000); // b .
    for(bool parallel:{false,true}) {
        Memory memory(1<<20);memory.set_parallel_access(false);
        memory.map(base,2*page_size,read_perm|write_perm|exec_perm);
        memory.copy_in(base,words.data(),words.size()*4);
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(base,memory);
        size_t count=0;
        for(const auto &block:graph->blocks)count+=block->instructions.size();
        require(count==512 && graph->blocks.back()->successors==std::vector<uint64_t>{base+512*4},
                "limited discovery lost the undiscovered fallthrough");
        ExecutionEngine engine;
        for(unsigned pass=0;pass<3;++pass) {
            if(pass==1)memory.store<4>(base+512*4,0x91000800); // add x0,x0,#2
            if(pass==2)memory.store<4>(base+1023*4,0x91000c00); // add x0,x0,#3
            for(unsigned budget:{0u,1u,511u,512u,513u,1023u,1024u,1025u,1200u,1205u}) {
                CpuState actual;actual.pc=base;actual.x[0]=17;
                auto expected=actual;
                for(unsigned i=0;i<budget;++i)
                    execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
                const auto result=engine.run(actual,memory,budget);
                require(result.instructions==budget && equal(actual,expected),
                        "limited discovery boundary, invalidation or retirement mismatch");
            }
        }
    }
}

void local_control_prefixes() {
    for(unsigned family=0;family<6;++family)for(unsigned bits:{32u,64u}) {
        if(family==0 && bits==32)continue;
        for(unsigned reg:{0u,31u})for(unsigned condition=0;condition<((family==0 || family==5) ? 16u:1u);++condition) {
            std::vector<uint32_t> words;
            if(family==5)words.push_back((bits==64 ? 0xf100041f:0x7100041f)|(reg<<5));
            uint32_t branch;
            if(family==0 || family==5)branch=0x54000060|condition;
            else if(family<=2)branch=(bits==64 ? 0xb4000060:0x34000060)|((family==2)<<24)|reg;
            else {
                const unsigned bit=bits==64 ? 40:7;
                branch=0x36000060|((family==4)<<24)|((bit>>5)<<31)|((bit&31)<<19)|reg;
            }
            words.insert(words.end(),{branch,0x91000631,0x14000002,0x91000a31});
            words.push_back(0x14000000|(uint32_t(-int(words.size()))&0x3ffffff));
            Memory memory(page_size);map(memory,words.data(),words.size());ExecutionEngine engine;
            for(uint64_t value:{uint64_t{0},uint64_t{1},uint64_t{1}<<40,UINT64_MAX})
                for(unsigned flags=0;flags<(family==0 ? 16u:1u);++flags)
                    for(unsigned budget=0;budget<24;++budget) {
                        CpuState actual;actual.pc=0x10000;actual.x[0]=value;actual.x[17]=100;actual.nzcv=flags;
                        auto expected=actual;
                        for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
                        const auto result=engine.run(actual,memory,budget);
                        require(result.instructions==budget && equal(actual,expected),"local control prefix mismatch");
                    }
        }
    }
    // BL has an additional destination role and must retire the link write
    // before the first guarded block in the callee executes.
    constexpr uint32_t call[]{0x94000002,0x17ffffff,0x91000631,0x17fffffe};
    Memory memory(page_size);map(memory,call,std::size(call));ExecutionEngine engine;
    for(unsigned budget=0;budget<40;++budget) {
        CpuState actual;actual.pc=0x10000;auto expected=actual;
        for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        require(engine.run(actual,memory,budget).instructions==budget && equal(actual,expected),"local call prefix mismatch");
    }
}

void external_conditional_link_targets() {
    // Warm the destination first so discovery leaves a genuinely external
    // edge. The first taken branch must resolve through the slow linker.
    for(bool parallel:{false,true})for(unsigned family=0;family<4;++family)
    for(bool taken:{false,true}) {
        const bool fused=family==3;
        const uint32_t branch=family==0 ? 0x54008000u : // b.eq +4096
                              family==1 ? 0xb4008000u : // cbz x0,+4096
                              family==2 ? 0x36008000u : // tbz w0,#0,+4096
                                          0x54007fe0u;  // b.eq +4092
        std::vector<uint32_t> source;
        if(fused)source.push_back(0xf100001fu); // cmp x0,#0
        source.insert(source.end(),{branch,0x91000484u,0x14000000u});
        const uint32_t target[]{0x91000463u,0x14000000u};
        Memory memory(2*page_size);map(memory,source.data(),source.size());
        memory.map(0x11000,page_size,7);memory.copy_in(0x11000,target,sizeof(target));
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory,nullptr,[](const void *,uint64_t pc) {
            return pc==0x11000;
        });
        const auto expected_code=fused ? StaticCode::CompareImmediateBranch:
            family==0 ? StaticCode::Branch:family==1 ? StaticCode::CompareBranch:StaticCode::TestBranch;
        bool selected=false;
        for(const auto &op:graph->entries.at(0x10000)->code)
            selected|=operation_info(op).code==expected_code && !op.target;
        require(selected,"external conditional fixture did not select an unlinked edge");
        ExecutionEngine engine;CpuState warm;warm.pc=0x11000;engine.run(warm,memory,2);
        CpuState actual;actual.pc=0x10000;actual.x[0]=taken ? 0:1;
        actual.nzcv=taken ? 4:0;actual.x[3]=10;actual.x[4]=20;
        auto expected=actual;const unsigned budget=fused ? 3:2;
        for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        const auto slow=engine.slow_calls;
        require(engine.run(actual,memory,budget).instructions==budget && equal(actual,expected),
                "external conditional linker target or state mismatch");
        if(taken)require(engine.slow_calls>slow,"external taken edge did not exercise slow linking");
    }
}

void deferred_forward_branches() {
    for(unsigned distance:{256u,260u}) {
        Memory memory(page_size);memory.set_parallel_access(false);memory.map(0x10000,page_size,7);
        memory.store<4>(0x10000,0x54000000u|((distance/4)<<5)); // b.eq target
        memory.store<4>(0x10004,0xd65f03c0);memory.store<4>(0x10000+distance,0xd65f03c0);
        const auto graph=build_graph(0x10000,memory);
        require(graph->entries.count(0x10004)==1 && graph->entries.count(0x10000+distance)==size_t(distance==256),
                "forward branch discovery boundary mismatch");
    }
    for(bool parallel:{false,true})for(unsigned family=0;family<3;++family) {
        const uint32_t branch=family==0 ? 0x54008000u : // b.eq +4096
                              family==1 ? 0xb4008000u : // cbz x0,+4096
                                          0x36008000u;  // tbz w0,#0,+4096
        const uint32_t source[]{branch,0x91000484u,0x14000000u};
        const uint32_t target[]{0x91000463u,0x14000000u};
        Memory memory(2*page_size);map(memory,source,std::size(source));
        memory.map(0x11000,page_size,7);memory.copy_in(0x11000,target,sizeof(target));
        memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        require(!graph->entries.count(0x11000),"distant forward target was discovered early");
        ExecutionEngine engine;
        for(bool taken:{false,true})for(unsigned budget=0;budget<=3;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[0]=taken ? 0:1;
            actual.nzcv=taken ? 4:0;actual.x[3]=10;actual.x[4]=20;auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto before=engine.executed_instructions;
            require(engine.run(actual,memory,budget).instructions==budget && equal(actual,expected),
                    "deferred forward branch state mismatch");
            require(engine.executed_instructions-before==budget,"deferred forward branch retirement mismatch");
        }
        // A warmed edge must observe both permission changes and new code.
        memory.protect(0x11000,page_size,read_perm|write_perm);
        CpuState actual;actual.pc=0x10000;actual.x[0]=0;actual.nzcv=4;actual.x[3]=10;
        const auto before=engine.executed_instructions;bool faulted=false;
        try {engine.run(actual,memory,2);} catch(const MemoryFault &fault) {
            faulted=fault.address==0x11000 && fault.permissions==exec_perm;
        }
        require(faulted && actual.pc==0x11000 && actual.x[3]==10 && engine.executed_instructions==before+1,
                "deferred forward target permission fault mismatch");
        memory.store<4>(0x11000,0x91000c63u); // add x3,x3,#3
        memory.protect(0x11000,page_size,7);
        actual={};actual.pc=0x10000;actual.x[0]=0;actual.nzcv=4;actual.x[3]=10;auto expected=actual;
        for(unsigned n=0;n<2;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        require(engine.run(actual,memory,2).instructions==2 && equal(actual,expected),
                "deferred forward edge retained stale target code");
    }
}

void residency_boundaries() {
    // Cross graphs with disjoint working sets, then observe every register at
    // every budget cut and at a fault in the successor (including untouched
    // high SIMD halves). Residency must not depend on either graph's contents.
    std::vector<uint32_t> first;
    for(unsigned r=0;r<31;++r)if(r<8 || r>10)
        first.push_back(0x91000400u|(r<<5)|r); // add Xr,Xr,#1
    for(unsigned r=0;r<32;++r)first.push_back(0x1e614000u|(r<<5)|r); // fneg Dr,Dr
    first.insert(first.end(),{0x910043ffu,0xd61f0100u}); // add SP,SP,#16; br X8
    const uint32_t second[]{0x91000e94u,0xf9400141u,0xd61f0120u}; // add X20,#3; ldr X1,[X10]; br X9
    for(bool parallel:{false,true}) {
        Memory memory(4*page_size);map(memory,first.data(),first.size());
        memory.map(0x11000,page_size,7);memory.copy_in(0x11000,second,sizeof(second));
        memory.map(0x20000,page_size,3);memory.store<8>(0x20000,0x123456789abcdef0ull);
        memory.set_parallel_access(parallel);ExecutionEngine engine;
        for(bool faulting:{false,true})for(unsigned budget=0;budget<2*first.size()+20;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x234560;
            for(unsigned r=0;r<31;++r)actual.x[r]=0xfedcba9876543200ull+r;
            for(unsigned r=0;r<32;++r)for(unsigned b=0;b<16;++b)actual.v[r][b]=uint8_t(17*r+b);
            actual.x[8]=0x11000;actual.x[9]=0x10000;actual.x[10]=faulting ? 0x30000:0x20000;
            auto expected=actual;unsigned retired=0;bool reference_fault=false,actual_fault=false;
            try {for(;retired<budget;++retired)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
            catch(const MemoryFault &) {reference_fault=true;}
            const auto before=engine.executed_instructions;
            try {require(engine.run(actual,memory,budget).instructions==retired,"residency budget retirement");}
            catch(const MemoryFault &) {actual_fault=true;}
            require(reference_fault==actual_fault && engine.executed_instructions-before==retired,"residency fault prefix");
            same(actual,expected);
        }
    }
}

void link_register_residency() {
    const uint32_t caller[]{
        0xaa1e03f7, // mov x23,x30
        0x2a1e03fe, // mov w30,w30: zero the upper half even on a self-copy
        0x8b1703de, // add x30,x30,x23
        0xaa0103fe, // mov x30,x1
        0xd63f03c0, // blr x30: read the old target before writing the link
        0x910022f7, // add x23,x23,#8
        0x14000000, // b .
    };
    const uint32_t callee[]{0x910007c0,0xd65f03c0}; // add x0,x30,#1; ret
    for(bool parallel:{false,true}) {
        Memory memory(4*page_size);map(memory,caller,std::size(caller));
        memory.map(0x11000,page_size,7);memory.copy_in(0x11000,callee,sizeof(callee));
        memory.set_parallel_access(parallel);ExecutionEngine engine;
        for(unsigned budget=0;budget<20;++budget) {
            CpuState actual;actual.pc=0x10000;actual.x[1]=0x11000;
            actual.x[30]=0xfedcba9800011000ull;actual.x[23]=0x123456789abcdef0ull;
            auto expected=actual;
            for(unsigned n=0;n<budget;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            require(engine.run(actual,memory,budget).instructions==budget,"link-register retirement");
            same(actual,expected);
        }
    }
}

void shared_code_snapshots() {
    constexpr uint64_t base=0x10000;
    Memory memory(4100*page_size);memory.set_parallel_access(false);
    memory.map(base,4100*page_size,read_perm|write_perm|exec_perm);
    const uint32_t instruction=0x91000400; // ADD X0, X0, #1.
    memory.store<4>(base,instruction);memory.store<4>(base+4,instruction);
    Memory::ExecutionScope scope(memory);
    auto first=snapshot_graph(base,memory,1);
    auto second=snapshot_graph(base+4,memory,1);
    require(first->dependencies[0].words==second->dependencies[0].words,"same-page regions copied code instead of sharing");
    require(first->validate(memory) && second->validate(memory),"fresh snapshots failed validation");
    const auto original=first->dependencies[0].words;
    memory.code_snapshot(base+256*page_size);
    require(memory.code_snapshot(base).words==original,"snapshot hash collision duplicated same code page");
    memory.synchronize_code(base);
    require(first->validate(memory) && first->dependencies[0].words==original,"unchanged page lost snapshot identity");
    memory.store<4>(base+128,instruction);
    require(first->validate(memory),"unrelated same-page change invalidated region");
    require((*original)[32]==0,"published code snapshot mutated");
    memory.store<4>(base,0x91000800);
    require(!first->validate(memory) && second->validate(memory),"changed code span was not distinguished");
    first=snapshot_graph(base,memory,1);
    // More pages than cache slots: a miss must compare a new snapshot correctly.
    for(unsigned page=1;page<4100;++page) memory.code_snapshot(base+page*page_size);
    require(first->validate(memory),"code snapshot eviction invalidated unchanged bytes");
    auto child=memory.fork_clone();child->set_parallel_access(false);
    child->store<4>(base,0x91000c00);
    require(!first->validate(*child) && first->validate(memory),"snapshot cache confused fork address spaces");
    memory.protect(base,page_size,read_perm);
    try { first->validate(memory);throw std::runtime_error("snapshot cache bypassed execute permission"); }
    catch(const MemoryFault &fault) { require(fault.kind==MemoryFault::Kind::Protection,"snapshot permission fault kind"); }
    memory.unmap(base,page_size);
    try { first->validate(memory);throw std::runtime_error("snapshot cache bypassed unmap"); }
    catch(const MemoryFault &) {}
    memory.map(base,page_size,read_perm|write_perm|exec_perm);
    memory.store<4>(base,0x91001000);
    require(!first->validate(memory),"snapshot cache retained replaced mapping");
}
void file_code_proofs() {
    constexpr uint64_t base=0x40000;
    constexpr uint32_t add1=0x91000400,add2=0x91000800,add3=0x91000c00;
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),2*page_size),"file proof fixture");
    require(pwrite(fileno(file.get()),&add1,4,0)==4 && pwrite(fileno(file.get()),&add1,4,4)==4,
            "file proof initial instructions");
    Memory memory(2*page_size);memory.set_parallel_access(false);
    Memory::Backing backing;backing.shared=true;
    memory.map_file(base,2*page_size,read_perm|exec_perm,fileno(file.get()),backing);
    std::unique_ptr<BlockGraph> first,second;
    {
        Memory::ExecutionScope scope(memory);
        first=snapshot_graph(base,memory,1);
        second=snapshot_graph(base+4,memory,1);
        require(first->dependencies[0].proof && first->dependencies[0].proof==second->dependencies[0].proof,"regions did not share page proof");
        require(first->validate(memory) && second->validate(memory),"initial file proof validation");
    }
    const auto epoch=memory.code_epoch.load();
    for(unsigned n=0;n<4;++n) {
        memory.synchronize_shared(true);
        require(memory.code_epoch.load()==epoch,"unchanged serial file invalidated all regions");
        Memory::ExecutionScope scope(memory);
        const auto generation=memory.code_proof_generation();
        require(!first->current(memory,generation),"file proof survived a new observation scope");
        require(first->validate(memory) && first->current(memory,generation) && second->validate(memory) && second->current(memory,generation),
                "one file page proof did not validate neighboring regions");
    }
    ExecutionEngine engine;CpuState cpu;cpu.pc=base;
    require(engine.run(cpu,memory,1).instructions==1 && cpu.x[0]==1,"initial file execution");
    require(pwrite(fileno(file.get()),&add2,4,0)==4,"external pwrite code change");
    memory.synchronize_shared(true);cpu.pc=base;cpu.x[0]=0;
    require(engine.run(cpu,memory,1).instructions==1 && cpu.x[0]==2,"pwrite retained stale region");
    {
        Memory::ExecutionScope scope(memory);
        require(!first->validate(memory) && second->validate(memory),"file proof confused covered and uncovered changes");
        require(!second->dependencies[0].proof->current(memory.code_proof_generation(),Memory::CodeProof::mask(second->dependencies[0].offset,second->dependencies[0].count)),
                "partial match published whole-line proof");
    }
    void *mapped=mmap(nullptr,2*page_size,PROT_READ|PROT_WRITE,MAP_SHARED,fileno(file.get()),0);
    require(mapped!=MAP_FAILED,"external mmap fixture");
    std::memcpy(mapped,&add3,4);
    memory.synchronize_shared(true);cpu.pc=base;cpu.x[0]=0;
    require(engine.run(cpu,memory,1).instructions==1 && cpu.x[0]==3,"mmap retained stale region");
    require(!munmap(mapped,2*page_size),"external mmap teardown");
    // One region depends on two pages; both proofs must be current.
    const uint32_t branch=0x14000001;
    require(pwrite(fileno(file.get()),&branch,4,page_size-4)==4 &&
            pwrite(fileno(file.get()),&add1,4,page_size)==4,"multi-page proof code");
    std::unique_ptr<BlockGraph> multi;
    {
        Memory::ExecutionScope scope(memory);
        multi=snapshot_graph(base+page_size-4,memory,2);
        require(multi->dependencies.size()==2 && multi->validate(memory),"multi-page proof construction");
    }
    {
        Memory::ExecutionScope scope(memory);
        const auto generation=memory.code_proof_generation();
        auto neighbor=snapshot_graph(base+4,memory,1);
        require(neighbor->validate(memory) && !multi->current(memory,generation),"first page masked stale second page");
        require(multi->validate(memory) && multi->current(memory,generation),"multi-page proof refresh");
    }
    require(!ftruncate(fileno(file.get()),0),"file proof truncation");
    cpu.pc=base;cpu.x[0]=0;const auto retired=engine.executed_instructions;
    try { engine.run(cpu,memory,1);throw std::runtime_error("truncation accepted stale page proof"); }
    catch(const MemoryFault &fault) {
        require(fault.kind==MemoryFault::Kind::Bus && cpu.pc==base && cpu.x[0]==0 &&
                engine.executed_instructions==retired,"truncation lost precise fault state");
    }
}
void code_validation_spans() {
    constexpr uint64_t base=0x20000;
    constexpr uint32_t nop=0xd503201f;
    for(bool parallel:{false,true}) for(bool file_backed:{false,true}) {
        Memory memory(page_size);memory.set_parallel_access(parallel);
        std::unique_ptr<FILE,decltype(&std::fclose)> file(nullptr,&std::fclose);
        if(file_backed) {
            file.reset(std::tmpfile());require(bool(file),"code bitmap file fixture");
            require(!ftruncate(fileno(file.get()),page_size),"code bitmap file size");
            Memory::Backing backing;backing.shared=true;
            memory.map_file(base,page_size,read_perm|write_perm|exec_perm,fileno(file.get()),backing);
        } else memory.map(base,page_size,read_perm|write_perm|exec_perm);
        std::array<uint32_t,page_size/4> words;words.fill(nop);
        memory.copy_in(base,words.data(),page_size);
        Memory::ExecutionScope scope(memory);
        const auto snapshot=memory.code_snapshot(base).words;
        for(unsigned offset:{0u,1u,31u,63u,64u,65u,127u,511u,1023u})
            for(unsigned requested:{1u,2u,32u,63u,64u,65u,128u,1024u}) {
                const auto count=std::min<unsigned>(requested,page_size/4-offset);
                BlockGraph image;
                image.dependencies.push_back({base+offset*4,snapshot,nullptr,offset,count});
                require(image.validate(memory) && image.validate(memory),"instruction bitmap rejected overlapping span");
                memory.store<4>(base+offset*4,0x91000400);
                require(!image.validate(memory),"instruction bitmap accepted changed covered word");
                memory.store<4>(base+offset*4,nop);
                require(image.validate(memory),"instruction bitmap failed restored word");
                if(offset+count<page_size/4) {
                    memory.store<4>(base+(offset+count)*4,0x91000400);
                    require(image.validate(memory),"instruction bitmap included uncovered word");
                    memory.store<4>(base+(offset+count)*4,nop);
                }
            }
    }
}
void batched_code_lines() {
    constexpr uint64_t base=0x60000;constexpr uint32_t nop=0xd503201f,changed=0x91000400;
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),page_size),"batched proof fixture");
    Memory memory(page_size);memory.set_parallel_access(false);Memory::Backing backing;backing.shared=true;
    memory.map_file(base,page_size,read_perm|write_perm|exec_perm,fileno(file.get()),backing);
    std::array<uint32_t,page_size/4> words;words.fill(nop);memory.copy_in(base,words.data(),page_size);
    BlockGraph sparse,full;
    {
        Memory::ExecutionScope scope(memory);const auto snapshot=memory.code_snapshot(base);
        for(auto [offset,count]:{std::pair{1u,1u},std::pair{31u,2u},std::pair{511u,1u},std::pair{1023u,1u}})
            sparse.dependencies.push_back({base+offset*4,snapshot.words,snapshot.proof,offset,count});
        full.dependencies.push_back({base,snapshot.words,snapshot.proof,0,page_size/4});
    }
    for(unsigned n=0;n<3;++n) {
        Memory::ExecutionScope scope(memory);
        require(sparse.validate(memory) && full.validate(memory),"batched sparse/full mask refresh");
    }
    // Byte zero shares a selected line with offset one, but is not an
    // instruction in this graph. It must not invalidate the exact spans or
    // publish a reusable proof for the mismatching line.
    require(pwrite(fileno(file.get()),&changed,4,0)==4,"batched uncovered change");
    {
        Memory::ExecutionScope scope(memory);
        require(sparse.validate(memory) && !full.validate(memory),"batch lost exact span fallback");
        require(!(sparse.dependencies.front().proof->verified&1),"batch proved a mismatching line");
    }
    require(pwrite(fileno(file.get()),&changed,4,511*4)==4,"batched covered change");
    {
        Memory::ExecutionScope scope(memory);
        require(!sparse.validate(memory),"batch accepted a covered instruction change");
    }
    require(pwrite(fileno(file.get()),&nop,4,511*4)==4,"batched covered restore");
    {
        Memory::ExecutionScope scope(memory);
        require(sparse.validate(memory),"batch rejected restored instruction spans");
        memory.protect(base,page_size,read_perm);
        bool failed=false;
        try {sparse.validate(memory);}catch(const MemoryFault &fault){failed=fault.kind==MemoryFault::Kind::Protection;}
        require(failed,"batch proof survived execute permission removal");
    }
}

void fast_memory_guards() {
    Memory memory(4*page_size);memory.set_parallel_access(false);
    const uint32_t program[]{0xf9000020,0xd4000001};map(memory,program,2);
    memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,17);
    auto child=memory.fork_clone();ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[0]=29;cpu.x[1]=0x20000;
    engine.run(cpu,memory,2);require(memory.load<8>(0x20000)==29 && child->load<8>(0x20000)==17,"native write bypassed COW");
    memory.protect(0x20000,page_size,read_perm);cpu.pc=0x10000;
    try{engine.run(cpu,memory,2);require(false,"native store bypassed revoked write permission");}catch(const MemoryFault &){}
    memory.protect(0x20000,page_size,read_perm|write_perm);const auto reservation=memory.exclusive_epoch(0x20000);
    cpu.pc=0x10000;cpu.x[0]=37;engine.run(cpu,memory,2);require(memory.exclusive_epoch(0x20000)!=reservation,"native store retained reservation token");
    const uint32_t load[]{0xf9400020,0xd4000001};memory.copy_in(0x10000,load,sizeof(load));
    for(bool parallel:{false,true})for(uint64_t address:{uint64_t{0},uint64_t{1},address_limit,UINT64_MAX,uint64_t{0x20ffc}}) {
        memory.set_parallel_access(parallel);cpu.pc=0x10000;cpu.x[0]=123;cpu.x[1]=address;const auto before=engine.executed_instructions;
        try{engine.run(cpu,memory,2);require(false,"native load accepted invalid or crossing address");}catch(const MemoryFault &){}
        require(cpu.pc==0x10000 && cpu.x[0]==123 && engine.executed_instructions==before,"invalid address lost precise state");
    }
    memory.set_parallel_access(false);memory.map(0x30000,page_size,3,false,true);auto shared=memory.fork_clone();
    memory.copy_in(0x10000,program,sizeof(program));cpu.pc=0x10000;cpu.x[0]=77;cpu.x[1]=0x30000;engine.run(cpu,memory,2);
    require(shared->load<8>(0x30000)==77,"native store lost shared-page visibility");
}
void native_page_cache() {
    constexpr std::array<uint64_t,8> addresses{page_size,8192*page_size,(uint64_t{1}<<24)-page_size,uint64_t{1}<<24,
        (uint64_t{1}<<32)-page_size,uint64_t{1}<<32,(uint64_t{1}<<32)+page_size,address_limit-page_size};
    const uint32_t program[]{0xf9400020,0xf9400422,0xf9000060,0xf9000462,0xd4000001};
    Memory memory((addresses.size()+2)*page_size);memory.set_parallel_access(false);map(memory,program,5);
    for(size_t n=0;n<addresses.size();++n) {
        memory.map(addresses[n],page_size,read_perm|write_perm);
        memory.store<8>(addresses[n],n+17);memory.store<8>(addresses[n]+8,n+101);
    }
    ExecutionEngine engine;
    auto run=[&](uint64_t read,uint64_t write) {
        CpuState cpu;cpu.pc=0x10000;cpu.x[1]=read;cpu.x[3]=write;
        require(engine.run(cpu,memory,5).instructions==5,"page cache lost retirement");return cpu;
    };
    // Two reads/writes per page exercise cache hits, full tags and bias offsets.
    for(auto address:addresses) {
        const auto first=memory.load<8>(address),second=memory.load<8>(address+8);
        const auto cpu=run(address,address);
        require(cpu.x[0]==first && cpu.x[2]==second && memory.load<8>(address+8)==second,
                "native page cache aliased a boundary or offset");
    }
    const auto first=addresses.front(),second=addresses.back();
    auto child=memory.fork_clone();run(first,second);
    require(child->load<8>(second)==addresses.size()-1+17 && memory.load<8>(second)==17,
            "native page cache bypassed COW");
    memory.protect(second,page_size,read_perm);
    try {run(first,second);require(false,"cached read allowed a revoked write");}catch(const MemoryFault &){}
    memory.protect(second,page_size,write_perm);
    try {run(second,first);require(false,"cached write allowed a revoked read");}catch(const MemoryFault &){}
    memory.unmap(second,page_size);memory.map(second,page_size,read_perm|write_perm);memory.store<8>(second,73);
    require(run(second,first).x[0]==73,"cache survived remap");
    for(bool parallel:{true,false}) {
        memory.set_parallel_access(parallel);memory.store<8>(first,83+parallel);
        require(run(first,second).x[0]==83+parallel,"cache survived parallel COW transition");
    }
    const uint32_t exclusive[]{0xf9000020,0xc85f7c22,0xf9000020,0xc8037c20,0xd4000001};
    memory.copy_in(0x10000,exclusive,sizeof(exclusive));
    CpuState cpu;cpu.pc=0x10000;cpu.x[0]=109;cpu.x[1]=first;engine.run(cpu,memory,5);
    require(cpu.x[3]==1,"cached native write retained an exclusive reservation");
    memory.copy_in(0x10000,program,sizeof(program));
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),page_size),"private file cache fixture");
    Memory::Backing backing;memory.map_file(second,page_size,read_perm|write_perm,fileno(file.get()),backing,true);
    memory.store<8>(second,97); // A private copy still needs a file-size proof each run.
    require(run(second,second).x[0]==97,"private file cache lost copied bytes");
    require(!ftruncate(fileno(file.get()),0),"private file cache truncate");
    try {run(second,first);require(false,"private file cache ignored truncation");}
    catch(const MemoryFault &fault){require(fault.kind==MemoryFault::Kind::Bus,"private file cache fault kind");}
}
void associative_tlb() {
    for(uint64_t distance:{1025,8193}) { // TLB-set and native-page-cache collisions.
    const uint64_t a=0x20000,b=a+distance*page_size;
    const uint32_t words[]{0xf9400020,0xf9400062,0x17fffffe};Memory memory(3*page_size);map(memory,words,3);
    memory.map(a,page_size,3);memory.map(b,page_size,3);memory.store<8>(a,17);memory.store<8>(b,29);
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[1]=a;cpu.x[3]=b;engine.run(cpu,memory,3000);
    require(cpu.x[0]==17 && cpu.x[2]==29 && engine.slow_calls<5,"native TLB did not preserve both colliding ways");
    memory.unmap(a,page_size);memory.map(a,page_size,3);memory.store<8>(a,41);cpu.pc=0x10000;engine.run(cpu,memory,3);
    require(cpu.x[0]==41 && cpu.x[2]==29,"native TLB retained unmapped storage");
    memory.unmap(b,page_size);cpu.pc=0x10000;const auto retired=engine.executed_instructions;
    try{engine.run(cpu,memory,3);require(false,"second native translation bypassed unmap");}catch(const MemoryFault &){}
    require(cpu.pc==0x10004 && engine.executed_instructions-retired==1,"second translation fault retirement mismatch");
    }
}
void vector_code_write() {
    constexpr uint32_t words[]{0x9100054a,0x3d800020,0xd2800022,0xd4000001,0xd503201f,0xd503201f};
    constexpr uint32_t replacement[]{0xd28000e2,0xd4000001,0xd503201f,0xd503201f};
    Memory memory(page_size);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,read_perm|write_perm|exec_perm);
    memory.copy_in(0x10000,words,sizeof(words));
    CpuState cpu;cpu.pc=0x10000;cpu.x[1]=0x10008;cpu.x[10]=27;
    std::memcpy(cpu.v[0].data(),replacement,sizeof(replacement));
    ExecutionEngine engine;
    const auto result=engine.run(cpu,memory,8);
    require(result.trap==Trap::Syscall && result.instructions==4 && cpu.pc==0x10010 &&
            cpu.x[10]==28 && cpu.x[2]==7,"vector code store executed stale code or lost post-store state");
}

void vector_memory_paths() {
    for(unsigned size:{4u,8u,16u}) for(bool store:{false,true})
    for(unsigned mode=0;mode<8;++mode) for(unsigned path=0;path<5;++path) {
        const unsigned scale=size==4 ? 2 : size==8 ? 3 : 0;
        const uint32_t width=size==16 ? 0x00800000u : scale<<30;
        uint32_t raw=0x3c000000u|width|(store ? 0u : 0x00400000u)|(1u<<5)|5u;
        int displacement=0;
        if(mode==0) { raw|=0x01000000u|(1u<<10);displacement=size; }
        else if(mode<4) {
            displacement=mode==1 ? -7 : 9;
            raw|=(uint32_t(displacement)&511u)<<12;
            if(mode==2) raw|=1u<<10;
            if(mode==3) raw|=3u<<10;
        } else {
            const unsigned option=mode==4 ? 2 : mode==5 ? 3 : mode==6 ? 6 : 7;
            raw|=0x00200800u|(2u<<16)|(option<<13)|(1u<<12);
        }
        const auto instruction=decode(raw);
        require(instruction.encoding!=Encoding::General,"vector fixture was not normalized");
        Memory memory(3*page_size),reference(3*page_size);
        memory.set_parallel_access(path==1);reference.set_parallel_access(path==1);
        const bool missing=path==3;
        const unsigned pages=missing ? 1 : 2;
        for(auto *target:{&memory,&reference}) {
            target->map(0x20000,pages*page_size,read_perm|write_perm);
            std::array<uint8_t,2*page_size> data;
            for(size_t n=0;n<data.size();++n) data[n]=uint8_t(n*19+3);
            target->copy_in(0x20000,data.data(),pages*page_size);
            if(path==4) target->protect(0x20000,2*page_size,store ? read_perm : write_perm);
        }
        CpuState initial;initial.pc=0x10000;initial.nzcv=13;initial.x[10]=27;
        const uint64_t address=(path==2 || missing) ? 0x20fffu : 0x20040u;
        const uint64_t offset=mode>=6 ? uint64_t(-3) : 3;
        initial.x[2]=offset;
        const uint64_t register_offset=mode==4 ? uint64_t(uint32_t(offset))*size : offset*size;
        initial.x[1]=mode>=4 ? address-register_offset : mode==2 ? address : address-displacement;
        for(unsigned r=0;r<32;++r) for(unsigned n=0;n<16;++n) initial.v[r][n]=uint8_t(r*7+n+11);
        auto expected=initial;expected.x[10]=expected.add(expected.x[10],1,false,64,true);expected.x[11]=expected.x[10];expected.pc+=8;
        bool expected_fault=false,actual_fault=false;
        uint64_t expected_address=0,actual_address=0;
        try { select_semantic(instruction)(expected,reference,instruction); }
        catch(const MemoryFault &fault) { expected_fault=true;expected_address=fault.address; }
        if(!expected_fault) { expected.x[10]+=2;expected.pc+=4; }
        constexpr uint32_t prefix[]{0xb100054a,0xaa0a03eb}; // adds x10,x10,#1; x11=x10
        const uint32_t words[]{prefix[0],prefix[1],raw,0x9100094a,0xd4000001};
        memory.map(0x10000,page_size,7);memory.copy_in(0x10000,words,sizeof(words));
        auto actual=initial;ExecutionEngine engine;uint64_t completed=0;
        try {completed=engine.run(actual,memory,5).instructions;}
        catch(const MemoryFault &fault) { actual_fault=true;actual_address=fault.address;completed=engine.executed_instructions; }
        if(!expected_fault)expected.pc+=4;
        require(actual_fault==expected_fault && actual_address==expected_address,
                "vector access changed fault address or completion");
        require(equal(actual,expected) && completed==(actual_fault ? 2u : 5u),
                "vector access lost live state, upper-bit clearing or writeback");
        if(store && path!=4) {
            std::array<uint8_t,2*page_size> actual_bytes{},expected_bytes{};
            memory.copy_out(0x20000,actual_bytes.data(),pages*page_size);
            reference.copy_out(0x20000,expected_bytes.data(),pages*page_size);
            require(actual_bytes==expected_bytes,"vector store changed memory commit semantics");
        }
    }
}

void floating_paths() {
#if defined(__aarch64__)
    // Native semantic helpers are an independent reference for FPCR/FPSR,
    // exceptional values, register overlap and flag consumers on either side.
    constexpr uint32_t operations[]{0x1e602800,0x1e603800,0x1e600800,0x1e601800,
                                    0x1f400000,0x1e61c000,0x1e602000,0x1e600c00};
    constexpr uint64_t values[]{0,0x8000000000000000ULL,0x3ff0000000000000ULL,
        0x4008000000000000ULL,0xbff0000000000000ULL,1,0x7ff0000000000000ULL,
        0xfff0000000000000ULL,0x7ff8123456789abcULL,0x7ff0123456789abcULL};
    Memory memory(page_size);memory.set_parallel_access(false);memory.map(0x10000,page_size,7);ExecutionEngine engine;
    for(const auto opcode:operations) for(unsigned alias=0;alias<3;++alias)
    for(unsigned control=0;control<8;++control) for(unsigned sample=0;sample<std::size(values);++sample) {
        const bool compare=opcode==0x1e602000,select=opcode==0x1e600c00;
        const unsigned rn=alias==0 ? 0 : 1,rm=alias==2 ? rn : 2,ra=3;
        const uint32_t raw=opcode|(rn<<5)|(compare || select || opcode==0x1e602800 ||
            opcode==0x1e603800 || opcode==0x1e600800 || opcode==0x1e601800 ||
            opcode==0x1f400000 ? rm<<16 : 0)|(opcode==0x1f400000 ? ra<<10 : 0)|
            (select ? (sample%16)<<12 : 0);
        const auto instruction=decode(raw);
        CpuState initial;initial.pc=0x10000;initial.x[10]=27;initial.nzcv=sample&15;
        initial.fpcr=((control&3u)<<22)|(control&4u ? 1u<<25 : 0);initial.fpsr=0x08000080;
        for(unsigned r=0;r<32;++r) {
            initial.v[r].fill(0xa5);const auto bits=values[(sample+r)%std::size(values)];
            std::memcpy(initial.v[r].data(),&bits,8);
        }
        auto expected=initial;
        expected.x[10]=expected.add(expected.x[10],1,false,64,true);expected.x[11]=expected.x[10]+1;expected.pc+=8;
        select_semantic(instruction)(expected,memory,instruction);
        expected.x[12]=expected.condition(0) ? expected.x[10] : expected.x[11];
        expected.x[10]=expected.add(expected.x[10],2,false,64,true);expected.pc+=8;
        const uint32_t words[]{0xb100054a,0x9100054b,raw,0x9a8b014c,0xb100094a,0xd4000001};
        memory.copy_in(0x10000,words,sizeof(words));auto actual=initial;
        const auto result=engine.run(actual,memory,std::size(words));expected.pc+=4;
        require(equal(actual,expected) && result.instructions==std::size(words),
                "compiled FP changed result, FPCR/FPSR, NZCV or live integer state");
    }
#endif
}

void floating_conversion_and_zero_compare() {
#if defined(__aarch64__)
    const uint64_t values[]{0,1,UINT64_MAX,0x8000000000000000ull,0x7fffffffffffffffull,
        0x3ff0000000000000ull,0xbff0000000000000ull,0x7ff0000000000000ull,
        0x7ff8123456789abcull,0x7ff0123456789abcull};
    for(bool parallel:{false,true})for(unsigned rn:{0u,7u,8u,30u,31u})
    for(uint32_t opcode:{0x9e620000u,0x1e602008u,0x1e602018u,0x1e602010u}) {
        const bool conversion=opcode==0x9e620000u,zero=opcode&8;
        const uint32_t word=opcode|(rn<<5)|(conversion ? 8u:zero ? 0u:31u<<16);
        const uint32_t words[]{word,0xd4000001};Memory memory(page_size);
        map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        require(operation_info(graph->entries.at(0x10000)->code[2]).code==
            (conversion ? StaticCode::SignedToDouble:StaticCode::Fcmp),"new FP instruction fell back to helper");
        ExecutionEngine engine;
        for(unsigned control=0;control<8;++control)for(uint64_t value:values)for(unsigned budget=0;budget<=2;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x20000;actual.nzcv=9;actual.fpsr=0x08000080;
            actual.fpcr=((control&3u)<<22)|(control&4u ? 1u<<25:0);
            for(unsigned reg=0;reg<32;++reg) {if(reg<actual.x.size())actual.x[reg]=value;actual.v[reg].fill(0xa5);std::memcpy(actual.v[reg].data(),&value,8);}
            auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto retired=engine.executed_instructions;engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-retired==budget,"new FP instruction retirement mismatch");
        }
    }
#endif
}
void native_numeric_conversions() {
#if defined(__aarch64__)
    struct Conversion {uint32_t opcode;StaticCode code;};
    const Conversion conversions[]{
        {0x1e620000,StaticCode::SignedToDouble},{0x9e620000,StaticCode::SignedToDouble},
        {0x1e630000,StaticCode::UnsignedToDouble},{0x9e630000,StaticCode::UnsignedToDouble},
        {0x1e780000,StaticCode::DoubleToSigned},{0x9e780000,StaticCode::DoubleToSigned}};
    // Include inexact integer conversions, truncation, both saturation limits,
    // subnormals, infinities and signaling/quiet NaNs. W sources contain dirty
    // upper halves; W destinations must clear them even for resident registers.
    const uint64_t values[]{0,1,UINT64_MAX,0x8000000000000000ull,0x7fffffffffffffffull,
        0xffffffff00000001ull,0x00000000ffffffffull,0x0020000000000001ull,
        0x3ff8000000000000ull,0xbff8000000000000ull,0x41dfffffffc00000ull,
        0x41e0000000000000ull,0xc1e0000000200000ull,0x43e0000000000000ull,
        0xc3e0000000000000ull,0x7ff0000000000000ull,0xfff0000000000000ull,
        0x7ff8123456789abcull,0x7ff0123456789abcull};
    for(bool parallel:{false,true})for(unsigned rn:{0u,7u,8u,30u,31u})
    for(unsigned rd:{0u,7u,8u,30u,31u})for(const auto conversion:conversions) {
        const uint32_t words[]{conversion.opcode|(rn<<5)|rd,0xd4000001};
        Memory memory(page_size);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);
        require(operation_info(graph->entries.at(0x10000)->code[2]).code==conversion.code,
                "numeric conversion fell back to helper");
        ExecutionEngine engine;
        for(unsigned control=0;control<8;++control)for(uint64_t value:values)for(unsigned budget=0;budget<=2;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x20000;actual.nzcv=9;actual.fpsr=0x08000080;
            actual.fpcr=((control&3u)<<22)|(control&4u ? (1u<<25)|(1u<<24):0);
            for(unsigned reg=0;reg<32;++reg) {
                if(reg<actual.x.size())actual.x[reg]=value;actual.v[reg].fill(0xa5);std::memcpy(actual.v[reg].data(),&value,8);
            }
            auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto retired=engine.executed_instructions;engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-retired==budget,"numeric conversion retirement mismatch");
        }
    }
#endif
}

void native_widening_multiply() {
#if defined(__aarch64__)
    struct Multiply {uint32_t opcode;StaticCode code;};
    const Multiply operations[]{
        {0x9b200000,StaticCode::SmaddLong},{0x9ba00000,StaticCode::UmaddLong},
        {0x9b208000,StaticCode::SmsubLong},{0x9ba08000,StaticCode::UmsubLong}};
    const uint64_t values[]{0,1,0x80000000ull,0x7fffffffull,UINT64_MAX,
        0xabcdef1200000001ull,0x12345678ffffffffull};
    for(bool parallel:{false,true})for(const auto operation:operations)
    for(unsigned rn:{0u,7u,30u,31u})for(unsigned rm:{8u,30u,31u})
    for(unsigned ra:{0u,8u,30u,31u})for(unsigned rd:{rn,rm,ra,6u,30u,31u}) {
        const uint32_t words[]{0xb100054a,operation.opcode|(rm<<16)|(ra<<10)|(rn<<5)|rd,
            0x9a0b014c,0xd4000001}; // Live NZCV surrounds the multiplication.
        Memory memory(page_size);map(memory,words,std::size(words));memory.set_parallel_access(parallel);
        const auto graph=build_graph(0x10000,memory);bool native=false;
        for(const auto &op:graph->entries.at(0x10000)->code)
            native|=operation_info(op).code==operation.code;
        // ZR inputs can make the optimizer fold the product to a move/copy.
        // Require the new handler for every nonconstant, retained product.
        if(rd!=31 && rn!=31 && rm!=31)
            require(native,"widening multiplication fell back to helper");
        ExecutionEngine engine;
        for(uint64_t value:values)for(unsigned budget=0;budget<=4;++budget) {
            CpuState actual;actual.pc=0x10000;actual.sp=0x20000;actual.nzcv=9;
            for(unsigned reg=0;reg<actual.x.size();++reg)actual.x[reg]=value^(uint64_t(reg)<<44);
            auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto retired=engine.executed_instructions;engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-retired==budget,"widening multiplication retirement mismatch");
        }
    }
#endif
}

void floating_host_environment() {
#if defined(__aarch64__)
    struct Restore {
        uint64_t control,status;
        Restore() {asm volatile("mrs %0, fpcr\n\tmrs %1, fpsr" : "=r"(control),"=r"(status));}
        ~Restore() {asm volatile("msr fpcr, %0\n\tmsr fpsr, %1" : : "r"(control),"r"(status) : "memory");}
    } restore;
    const uint64_t host_control=(restore.control&~(uint64_t{3}<<22))|(uint64_t{2}<<22);
    const uint64_t host_status=0x08000011;
    const auto check_host=[&]() {
        uint64_t control,status;
        asm volatile("mrs %0, fpcr\n\tmrs %1, fpsr" : "=r"(control),"=r"(status));
        require(control==host_control && status==host_status,"executor leaked guest floating environment");
    };
    // Alternate integer-only intervals, native FP and a system-register helper.
    // Every budget prefix must preserve both the guest and caller sticky flags.
    const uint32_t programs[][4]{
        {0x91000400,0x91000821,0xd503201f,0x17fffffd},
        {0x1e621820,0xd53b4403,0x91000400,0x17fffffd}, // fdiv d0,d1,d2; mrs x3,fpcr
        {0x1e621820,0xf94000a4,0x91000400,0x17fffffd}  // ldr x4,[x5] may fault
    };
    for(unsigned program=0;program<std::size(programs);++program) {
        Memory memory(2*page_size);map(memory,programs[program],4);
        memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,57);
        ExecutionEngine engine;
        for(bool fault:{false,true})for(unsigned budget=1;budget<=41;++budget) {
            CpuState expected;expected.pc=0x10000;expected.x[5]=fault ? 0x30000:0x20000;
            expected.fpcr=(uint64_t{1}<<22)|(uint64_t{1}<<25);expected.fpsr=0x08000080;
            const uint64_t one=0x3ff0000000000000ull,zero=0;
            std::memcpy(expected.v[1].data(),&one,8);std::memcpy(expected.v[2].data(),&zero,8);
            auto actual=expected;bool reference_fault=false,actual_fault=false;
            unsigned retired=0;
            for(;retired<budget;++retired) {
                try {execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));}
                catch(const MemoryFault &) {reference_fault=true;break;}
            }
            asm volatile("msr fpcr, %0\n\tmsr fpsr, %1" : : "r"(host_control),"r"(host_status) : "memory");
            const auto before=engine.executed_instructions;
            try {engine.run(actual,memory,budget);}catch(const MemoryFault &) {actual_fault=true;}
            check_host();same(actual,expected);
            require(reference_fault==actual_fault && engine.executed_instructions-before==retired,
                    "floating boundary changed retirement or fault prefix");
        }
    }
#endif
}

void graph_bridges() {
    Memory memory(3*page_size);memory.set_parallel_access(false);
    memory.map(0x10000,2*page_size,7);
    const uint32_t first[]{0x91000400,0xd61f0100}; // add x0,x0,#1; br x8
    const uint32_t second[]{0x91000e31,0x1e752a94,0xd61f0120}; // add x17,x17,#3; fadd d20,d20,d21; br x9
    memory.copy_in(0x10000,first,sizeof(first));memory.copy_in(0x11000,second,sizeof(second));
    CpuState cpu;cpu.pc=0x10000;cpu.x[8]=0x11000;cpu.x[9]=0x10000;double increment=0.5;
    std::memcpy(cpu.v[21].data(),&increment,8);ExecutionEngine engine;
    engine.run(cpu,memory,5000);double value;std::memcpy(&value,cpu.v[20].data(),8);
    require(cpu.x[0]==1000 && cpu.x[17]==3000 && value==500,"cross-graph role bridge corrupted values");
    require(engine.slow_calls<10 && engine.materializations<10,"cross-graph loop materializes state every edge");
    memory.store<4>(0x11000,0x91001631);engine.run(cpu,memory,5);
    require(cpu.x[0]==1001 && cpu.x[17]==3005,"incoming bridge survived target replacement");
    for(unsigned count=0;count<20;++count) {
        CpuState actual;actual.pc=0x10000;actual.x[8]=0x11000;actual.x[9]=0x10000;auto expected=actual;
        for(unsigned n=0;n<count;++n)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
        engine.run(actual,memory,count);same(actual,expected);
    }
}
void polymorphic_edges() {
    Memory memory(3*page_size);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,7);memory.map(0x20000,page_size,7);
    memory.store<4>(0x10000,0xd61f0100); // br x8
    for(unsigned n=0;n<32;++n) {
        const uint64_t pc=0x20000+n*32,next=0x20000+((n+1)%32)*32;
        const auto delta=next-(pc+4);
        const uint32_t adr=0x10000008u|uint32_t(delta&3)<<29|uint32_t((delta>>2)&0x7ffff)<<5;
        const uint32_t words[]{0x91000000u|((n+1)<<10),adr,0xd61f0120u};
        memory.copy_in(pc,words,sizeof(words));
    }
    CpuState cpu;cpu.pc=0x10000;cpu.x[8]=0x20000;cpu.x[9]=0x10000;ExecutionEngine engine;
    engine.run(cpu,memory,32*4*100);
    require(cpu.x[0]==52800 && cpu.pc==0x10000,"polymorphic cache selected wrong guest target");
    require(engine.slow_calls<150,"polymorphic edge repeatedly crosses C++");
    memory.store<4>(0x20000,0x91001c00); // first increment changes from 1 to 7
    engine.run(cpu,memory,32*4);
    require(cpu.x[0]==53334,"polymorphic cache retained modified target");
}
void indirect_target_replacement() {
    {
        IndirectCache cache;
        std::array<BridgePlan*,128> addresses{};
        require(cache.bridges.empty(),"indirect cache allocated unused bridges");
        for(unsigned n=0;n<128;++n) {
            const auto slot=(n*37)%128;
            auto &bridge=cache.bridge_for(slot);
            require(!bridge,"indirect bridge slot was reused");
            bridge=std::make_unique<BridgePlan>();
            bridge->expected_generation=slot+1;
            addresses[slot]=bridge.get();
        }
        require(cache.bridges.size()==128,"indirect bridge slots missing");
        for(unsigned slot=0;slot<128;++slot)
            require(cache.bridge_for(slot).get()==addresses[slot] &&
                    cache.bridge_for(slot)->expected_generation==slot+1,"indirect bridge slot lookup mismatch");
    }
    for(bool parallel:{false,true}) {
        Memory memory(8*page_size);memory.set_parallel_access(parallel);
        memory.map(0x10000,page_size,7);memory.map(0x20000,2*page_size,7);
        memory.store<4>(0x10000,0xd61f0100); // br x8
        std::vector<uint64_t> targets;
        for(uint64_t pc=0x20000;pc<0x22000 && targets.size()<6;pc+=16)
            if((((pc>>2)^(pc>>9))&31)==0)targets.push_back(pc);
        require(targets.size()==6,"recent indirect fixture needs colliding targets");
        for(unsigned n=0;n<targets.size();++n) {
            const uint32_t words[]{0x91000000u|((n+1)<<10),0xd61f0120u};
            memory.copy_in(targets[n],words,sizeof(words));
        }
        ExecutionEngine engine;
        for(unsigned round=0;round<8;++round)for(unsigned target:{0u,0u,1u,2u,3u,0u,4u,0u,5u,0u})
        for(unsigned budget:{3u,30u}) {
            CpuState actual;actual.pc=0x10000;actual.x[8]=targets[target];actual.x[9]=0x10000;
            actual.x[0]=round*100;actual.nzcv=9;auto expected=actual;
            for(unsigned i=0;i<budget;++i)execute_one(expected,memory,decode(memory.load<4>(expected.pc,exec_perm)));
            const auto retired=engine.executed_instructions;engine.run(actual,memory,budget);same(actual,expected);
            require(engine.executed_instructions-retired==budget,"recent indirect replacement retirement mismatch");
        }
    }
}
void file_bridge_proofs() {
    std::unique_ptr<FILE,decltype(&std::fclose)> file(std::tmpfile(),&std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()),page_size),"file bridge fixture");
    for(unsigned n=0;n<32;++n) {
        const uint64_t pc=0x20000+n*32,next=0x20000+((n+1)%32)*32;
        const auto delta=next-(pc+4);
        const uint32_t adr=0x10000008u|uint32_t(delta&3)<<29|uint32_t((delta>>2)&0x7ffff)<<5;
        const uint32_t words[]{0x91000000u|((n+1)<<10),adr,0xd61f0120u};
        require(pwrite(fileno(file.get()),words,sizeof(words),n*32)==sizeof(words),"file bridge code");
    }
    Memory memory(3*page_size);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,7);memory.store<4>(0x10000,0xd61f0100);
    Memory::Backing backing;backing.shared=true;
    memory.map_file(0x20000,page_size,read_perm|exec_perm,fileno(file.get()),backing);
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[8]=0x20000;cpu.x[9]=0x10000;
    engine.run(cpu,memory,32*4*100);
    const auto slow=engine.slow_calls;
    engine.run(cpu,memory,32*4*100);
    require(cpu.x[0]==105600 && cpu.pc==0x10000,"file bridge proof changed values");
    require(engine.slow_calls-slow<20,"file bridge revalidated each graph instead of reusing line proofs");
    const uint32_t changed=0x91001c00;
    require(pwrite(fileno(file.get()),&changed,4,0)==4,"file bridge modification");
    engine.run(cpu,memory,32*4);
    require(cpu.x[0]==106134,"native proof bridge retained externally changed code");
    const uint32_t unrelated=0xd503201f;
    require(pwrite(fileno(file.get()),&unrelated,4,page_size-4)==4,"file bridge unrelated modification");
    engine.run(cpu,memory,32*4);
    require(cpu.x[0]==106668,"native proof rejected unchanged covered spans");
    void *mapped=mmap(nullptr,page_size,PROT_READ|PROT_WRITE,MAP_SHARED,fileno(file.get()),0);
    require(mapped!=MAP_FAILED,"native code proof external mapping");
    const uint32_t add9=0x91002400;
    std::memcpy(mapped,&add9,4);
    require(!munmap(mapped,page_size),"native code proof external unmap");
    const auto previous=cpu.x[0];
    engine.run(cpu,memory,32*4);
    require(cpu.x[0]==previous+536,"native code proof retained mmap-written instruction");
    memory.protect(0x20000,page_size,read_perm);
    cpu.pc=0x10000;cpu.x[8]=0x20000;
    auto retired=engine.executed_instructions;const auto value=cpu.x[0];
    bool faulted=false;
    try {engine.run(cpu,memory,32*4);}
    catch(const MemoryFault &fault) {
        faulted=fault.address==0x20000 && fault.kind==MemoryFault::Kind::Protection &&
            fault.permissions==exec_perm;
    }
    require(faulted && cpu.pc==0x20000 && cpu.x[0]==value &&
            engine.executed_instructions==retired+1,"native code proof bypassed execute permission");
    memory.protect(0x20000,page_size,read_perm|exec_perm);
    cpu.pc=0x10000;engine.run(cpu,memory,32*4);
    require(cpu.x[0]==value+536,"native code proof failed permission restoration");
    require(!ftruncate(fileno(file.get()),0),"native code proof file truncation");
    cpu.pc=0x10000;cpu.x[8]=0x20000;retired=engine.executed_instructions;
    const auto before_fault=cpu.x[0];faulted=false;
    try {engine.run(cpu,memory,32*4);}
    catch(const MemoryFault &fault) {
        faulted=fault.address==0x20000 && fault.kind==MemoryFault::Kind::Bus &&
            fault.permissions==exec_perm;
    }
    require(faulted && cpu.pc==0x20000 && cpu.x[0]==before_fault &&
            engine.executed_instructions==retired+1,"native code proof lost truncation fault state");
}
void bridge_reclamation() {
    constexpr unsigned targets=graph_cache_capacity+4;
    Memory memory(32*page_size);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,7);memory.map(0x20000,((targets*16+page_size-1)/page_size)*page_size,7);
    memory.store<4>(0x10000,0xd61f0100); // br x8
    const uint32_t target[]{0x91000400,0xd61f0120};
    for(unsigned n=0;n<targets;++n)memory.copy_in(0x20000+n*16,target,sizeof(target));
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[9]=0x10000;
    for(unsigned n=0;n<targets;++n){cpu.x[8]=0x20000+n*16;engine.run(cpu,memory,3);}
    require(cpu.x[0]==targets && cpu.pc==0x10000,"cache reclamation retained a dangling bridge");
    cpu.x[8]=0x20000;engine.run(cpu,memory,3);require(cpu.x[0]==targets+1,"reclaimed target did not recompile");
    ExecutionEngine refresh;cpu={};cpu.pc=0x20000+(targets-1)*16;cpu.x[9]=0x10000;refresh.run(cpu,memory,1);
    cpu.pc=0x10000;cpu.x[8]=0x20000;refresh.run(cpu,memory,300);
    for(unsigned n=1;n<=graph_cache_capacity-2;++n){cpu.pc=0x20000+n*16;refresh.run(cpu,memory,1);}
    cpu.pc=0x10000;const auto slow=refresh.slow_calls;refresh.run(cpu,memory,3000);
    require(refresh.slow_calls-slow<10,"generation change left existing bridges permanently cold");
    // Reclaim unrelated older graphs while the source, its PIC and target
    // remain alive. Their warmed native edges must survive that eviction.
    ExecutionEngine stable;cpu={};cpu.x[9]=0x10000;
    for(unsigned n=0;n<graph_cache_capacity-6;++n){cpu.pc=0x20000+n*16;stable.run(cpu,memory,1);}
    cpu.pc=0x10000;cpu.x[8]=0x20000+(graph_cache_capacity-6)*16;stable.run(cpu,memory,300);
    const auto warmed=stable.slow_calls;stable.run(cpu,memory,300);
    const auto steady=stable.slow_calls-warmed;
    for(unsigned n=graph_cache_capacity-5;n<=graph_cache_capacity;++n){cpu.pc=0x20000+n*16;stable.run(cpu,memory,1);}
    cpu.pc=0x10000;const auto before=stable.slow_calls;stable.run(cpu,memory,300);
    require(stable.slow_calls-before<=steady+1,"unrelated eviction discarded warmed native edges");
}

}
int main(){try{
gp_operand_mapping_paths();
decoded_node_consistency();
shifted_add_sub_paths();
vector_logical_paths();
load_acquire_paths();
#if OPERIT_EXECUTION_DIAGNOSTICS
diagnostic_recipe_paths();
#endif
#if OPERIT_ARM64_GUEST_AOT
guest_aot_paths();
#endif
#if OPERIT_ARM64_PROFILE_SUPERBLOCKS
#if OPERIT_ARM64_GUEST_AOT
combined_aot_superblock_paths();
#endif
profile_superblock_paths();
#endif
global_value_paths();randomized_value_paths();scheduled_load_paths();scheduled_file_loads();prefixes();native_prefixes();forward_regions();compact_operation_prefixes();local_edge_targets();unordered_instruction_graph();partition_leader_reference();discovery_bitmap_boundaries();discovery_limit_cross_graph();local_control_prefixes();external_conditional_link_targets();deferred_forward_branches();residency_boundaries();link_register_residency();quantum_continuations();native_system_and_conditions();native_scalar_moves();native_writebacks();native_file_reads();native_file_page_lock();loops();cached_graph_boundaries();deferred_call_targets();loop_call_discovery();
#if OPERIT_ARM64_ADAPTIVE_CALLS
adaptive_call_paths();adaptive_multiple_calls();
#endif
lowering_address_independence();memory_and_fault();native_pairs();pair_faults();shared_code_snapshots();file_code_proofs();code_validation_spans();batched_code_lines();fast_memory_guards();native_page_cache();associative_tlb();vector_code_write();vector_memory_paths();floating_paths();floating_conversion_and_zero_compare();native_numeric_conversions();native_widening_multiply();floating_host_environment();graph_bridges();polymorphic_edges();indirect_target_replacement();file_bridge_proofs();bridge_reclamation();std::puts("static execution backend passed");return 0;}catch(const std::exception &e){std::fprintf(stderr,"%s\n",e.what());return 1;}}
