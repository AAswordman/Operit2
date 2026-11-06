#include "cpu/aarch64/execution_engine.h"
#include "execution_test_support.h"
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <unistd.h>
#include <sys/mman.h>
#include <algorithm>
#include <thread>
using namespace operit::arm64;
namespace {
void require(bool value,const char *why) {if(!value)throw std::runtime_error(why);}
void same(const CpuState &a,const CpuState &b) {
    require(a.x==b.x && a.sp==b.sp && a.pc==b.pc && a.v==b.v && a.nzcv==b.nzcv && a.fpcr==b.fpcr && a.fpsr==b.fpsr,"architectural state mismatch");
}
void map(Memory &m,const uint32_t *words,size_t count) {
    m.set_parallel_access(false);m.map(0x10000,page_size,read_perm|write_perm|exec_perm);m.copy_in(0x10000,words,count*4);
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
    constexpr unsigned targets=4100;
    Memory memory(32*page_size);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,7);memory.map(0x20000,17*page_size,7);
    memory.store<4>(0x10000,0xd61f0100); // br x8
    const uint32_t target[]{0x91000400,0xd61f0120};
    for(unsigned n=0;n<targets;++n)memory.copy_in(0x20000+n*16,target,sizeof(target));
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;cpu.x[9]=0x10000;
    for(unsigned n=0;n<targets;++n){cpu.x[8]=0x20000+n*16;engine.run(cpu,memory,3);}
    require(cpu.x[0]==targets && cpu.pc==0x10000,"cache reclamation retained a dangling bridge");
    cpu.x[8]=0x20000;engine.run(cpu,memory,3);require(cpu.x[0]==targets+1,"reclaimed target did not recompile");
    ExecutionEngine refresh;cpu={};cpu.pc=0x20000+(targets-1)*16;cpu.x[9]=0x10000;refresh.run(cpu,memory,1);
    cpu.pc=0x10000;cpu.x[8]=0x20000;refresh.run(cpu,memory,300);
    for(unsigned n=1;n<=4094;++n){cpu.pc=0x20000+n*16;refresh.run(cpu,memory,1);}
    cpu.pc=0x10000;const auto slow=refresh.slow_calls;refresh.run(cpu,memory,3000);
    require(refresh.slow_calls-slow<10,"generation change left existing bridges permanently cold");
    // Reclaim unrelated older graphs while the source, its PIC and target
    // remain alive. Their warmed native edges must survive that eviction.
    ExecutionEngine stable;cpu={};cpu.x[9]=0x10000;
    for(unsigned n=0;n<4090;++n){cpu.pc=0x20000+n*16;stable.run(cpu,memory,1);}
    cpu.pc=0x10000;cpu.x[8]=0x20000+4090*16;stable.run(cpu,memory,300);
    const auto warmed=stable.slow_calls;stable.run(cpu,memory,300);
    const auto steady=stable.slow_calls-warmed;
    for(unsigned n=4091;n<=4096;++n){cpu.pc=0x20000+n*16;stable.run(cpu,memory,1);}
    cpu.pc=0x10000;const auto before=stable.slow_calls;stable.run(cpu,memory,300);
    require(stable.slow_calls-before<=steady+1,"unrelated eviction discarded warmed native edges");
}

}
int main(){try{prefixes();native_prefixes();forward_regions();compact_operation_prefixes();local_control_prefixes();residency_boundaries();quantum_continuations();native_system_and_conditions();native_scalar_moves();native_writebacks();native_file_reads();native_file_page_lock();loops();memory_and_fault();native_pairs();pair_faults();shared_code_snapshots();file_code_proofs();code_validation_spans();batched_code_lines();fast_memory_guards();native_page_cache();associative_tlb();vector_code_write();vector_memory_paths();floating_paths();floating_host_environment();graph_bridges();polymorphic_edges();file_bridge_proofs();bridge_reclamation();std::puts("static execution backend passed");return 0;}catch(const std::exception &e){std::fprintf(stderr,"%s\n",e.what());return 1;}}
