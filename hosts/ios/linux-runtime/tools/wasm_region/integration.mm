#import <TargetConditionals.h>
#if TARGET_OS_IPHONE
#import <UIKit/UIKit.h>
#else
#import <Cocoa/Cocoa.h>
#endif
#include "cpu/aarch64/execution_engine.h"
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <algorithm>
#include <mutex>
#include <thread>
using namespace operit::arm64;
namespace {
constexpr uint64_t entry=0x10000,data=0x6000000000;
void require(bool ok,const char *why) {if(!ok)throw std::runtime_error(why);}
void prepare(Memory &m,const std::vector<uint32_t> &words,bool shared=false) {
    m.set_parallel_access(false);m.map(entry,0x10000,read_perm|write_perm|exec_perm);m.copy_in(entry,words.data(),words.size()*4);
    m.map(data,8192,read_perm|write_perm,false,shared);m.store<8>(data,data);m.store<8>(data+8,17);
}
Execution scalar(CpuState &cpu,Memory &memory,uint64_t budget) {
    Execution result{Trap::Budget,0};
    while(result.instructions<budget) {
        auto i=decode(memory.load<4>(cpu.pc,exec_perm));
        bool syscall=select_semantic(i)(cpu,memory,i);++result.instructions;
        if(syscall) {result.trap=Trap::Syscall;break;}
    }
    return result;
}
void check(std::string name,const std::vector<uint32_t> &words,CpuState initial,uint64_t budget,int offload=1,bool shared=false,bool verify_offload=true) {
    const auto require=[&](bool ok,const char *why){if(!ok)throw std::runtime_error(name+": "+why);};
    Memory actual_mem(1<<22),reference_mem(1<<22);prepare(actual_mem,words,shared);prepare(reference_mem,words,shared);
    initial.pc=entry;initial.x[3]=data;auto actual=initial,reference=initial;
    ExecutionEngine engine;
    const auto start=std::chrono::steady_clock::now();auto result=engine.run(actual,actual_mem,budget);
    const double ms=std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-start).count();
    auto expected=scalar(reference,reference_mem,budget);
    require(actual.x==reference.x && actual.pc==reference.pc && actual.sp==reference.sp && actual.nzcv==reference.nzcv && actual.v==reference.v,"CPU mismatch");
    require(actual.fpcr==reference.fpcr && actual.fpsr==reference.fpsr && actual.tls==reference.tls,"extended state mismatch");
    require(result.instructions==expected.instructions && result.trap==expected.trap,"retirement/trap mismatch");
    std::vector<uint8_t> a(8192),b(8192);actual_mem.copy_out(data,a.data(),a.size());reference_mem.copy_out(data,b.data(),b.size());require(a==b,"memory mismatch");
    if(verify_offload)require(offload ? engine.wasm.executed_instructions>0:engine.wasm.executed_instructions==0,"unexpected offload coverage");
    std::printf("{\"case\":\"%s\",\"retired\":%llu,\"wasm\":%llu,\"calls\":%llu,\"blocks\":%u,\"size\":%u,\"ms\":%.3f}\n",name.c_str(),
        (unsigned long long)result.instructions,(unsigned long long)engine.wasm.executed_instructions,(unsigned long long)engine.wasm.calls,engine.wasm.largest_block_count,engine.wasm.largest_region,ms);
}
void benchmark(std::string name,const std::vector<uint32_t> &words,uint64_t iterations) {
    Memory memory(1<<22);prepare(memory,words);ExecutionEngine engine;
    std::vector<double> native_ms,wasm_ms;CpuState expected;
    auto *times=&native_ms;auto *state=&expected;
    dispatch_sync(dispatch_get_main_queue(),^{
        Memory native_memory(1<<22);prepare(native_memory,words);ExecutionEngine native;
        for(unsigned n=0;n<8;++n) {
            CpuState cpu;cpu.pc=entry;cpu.x[2]=iterations;
            auto begin=std::chrono::steady_clock::now();native.run(cpu,native_memory,UINT32_MAX);
            if(n)times->push_back(std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-begin).count());
            *state=cpu;
        }
    });
    for(unsigned n=0;n<8;++n) {
        CpuState cpu;cpu.pc=entry;cpu.x[2]=iterations;
        auto begin=std::chrono::steady_clock::now();engine.run(cpu,memory,UINT32_MAX);
        if(n)wasm_ms.push_back(std::chrono::duration<double,std::milli>(std::chrono::steady_clock::now()-begin).count());
        require(cpu.x==expected.x && cpu.pc==expected.pc && cpu.nzcv==expected.nzcv,"benchmark state mismatch");
    }
    std::sort(native_ms.begin(),native_ms.end());std::sort(wasm_ms.begin(),wasm_ms.end());
    std::printf("{\"benchmark\":\"%s\",\"native_ms\":%.3f,\"wasm_ms\":%.3f,\"calls\":%llu,\"wasm\":%llu}\n",name.c_str(),native_ms[3],wasm_ms[3],(unsigned long long)engine.wasm.calls,(unsigned long long)engine.wasm.executed_instructions);
}
void optimizer_tests() {
    CpuState initial;initial.sp=0x6000000800;initial.nzcv=13;
    initial.x[0]=UINT64_MAX;initial.x[1]=0x100000001;initial.x[2]=0x8000000000000000;
    const std::vector<uint32_t> constants{
        0xd2800164, // mov x4,#11 (dead)
        0xd2800184, // mov x4,#12
        0xd28000a0, // mov x0,#5
        0x91001c01, // add x1,x0,#7 -> 12
        0x11000425, // add w5,w1,#1 -> 13
        0x9100403f, // add sp,x1,#16 -> 28
        0x910013e7, // add x7,sp,#4 -> 32
        0xd4000001
    };
    WasmRegion folded(constants);
    require(folded.eliminated_instructions==1 && folded.constant_results==6,"shared constant/DCE optimizer was bypassed");
    require(folded.instruction_count==constants.size(),"optimization changed guest instruction count");
    WasmRegion literal_results(std::vector<uint32_t>{0xd28001e4,0xd2800184,0xd28000a0,0xd2800181,0xd28001a5,0x9100403f,0xd2800407,0xd4000001});
    require(folded.module()==literal_results.module(),"Wasm emission ignored folded constants or dead results");
    // Equivalent non-flag-setting forms must produce identical Wasm, proving
    // the lowering consumes discard_flags rather than only reporting it.
    for(auto [flagged,plain]:std::vector<std::pair<uint32_t,uint32_t>>{{0xb1000801,0x91000801},{0xea020001,0x8a020001},{0xfa410000,0xd503201f}}) {
        WasmRegion optimized(std::vector<uint32_t>{flagged,0xf100005f,0xd4000001});
        WasmRegion equivalent(std::vector<uint32_t>{plain,0xf100005f,0xd4000001});
        require(optimized.module()==equivalent.module(),"Wasm emission ignored discarded flags");
    }
    const std::vector<uint32_t> flags{
        0xb100041f, // adds xzr,x0,#1: discarding NZCV must not write SP
        0xab21401f, // adds xzr,x0,w1,uxtw: same for extended registers
        0xb1000801, // adds x1,x0,#2: retain the GP result
        0xea030062, // ands x2,x3,x3: retain AND, discard NZCV
        0xfa410000, // ccmp x0,x1,#0,eq: only writes dead flags
        0xf100005f, // cmp x2,#0: live at the syscall exit
        0xd4000001
    };
    WasmRegion unflagged(flags);
    require(unflagged.discarded_flags>=4,"shared flags optimizer was bypassed");
    for(const auto &test:std::vector<std::pair<std::string,std::vector<uint32_t>>>{{"optimizer_constants",constants},{"optimizer_flags",flags}}) {
        // Every interior cut exercises the native precise-budget fallback;
        // a complete block must execute the optimized Wasm itself.
        for(unsigned budget=1;budget<=test.second.size();++budget)
            check(test.first+"_"+std::to_string(budget),test.second,initial,budget,1,false,budget==test.second.size());
    }
    const std::vector<uint32_t> join{0xd28000a0,0xb4000041,0xd2800120,0x91000402,0xd4000001};
    WasmRegion joined(join);
    require(joined.block_count==3 && joined.eliminated_instructions==0 && joined.constant_results==2,"optimizer crossed a conditional edge");
    for(unsigned taken=0;taken<2;++taken) {
        initial.x[1]=taken;
        check("optimizer_join_"+std::to_string(taken),join,initial,100);
    }
    // An external target still terminates the optimization window.
    WasmRegion external(std::vector<uint32_t>{0xd28000a0,0xb4000801,0xd2800120,0xd4000001});
    require(external.block_count==2 && external.eliminated_instructions==0,"optimizer crossed an external conditional edge");
    const std::vector<uint32_t> faulting{0xd28000a0,0xb1000442,0xf9400061,0xd2800120,0xf100005f,0xd4000001};
    WasmRegion guarded(faulting);
    require(guarded.eliminated_instructions==0 && guarded.discarded_flags==0,"optimizer crossed a memory fault boundary");
    auto memory=std::make_unique<Memory>(1<<22),reference_memory=std::make_unique<Memory>(1<<22);prepare(*memory,faulting);prepare(*reference_memory,faulting);
    initial.pc=entry;initial.x[3]=data+8192;auto actual=initial,reference=initial;ExecutionEngine engine;
    bool actual_fault=false,reference_fault=false;
    try {engine.run(actual,*memory,100);}catch(const MemoryFault &) {actual_fault=true;}
    try {scalar(reference,*reference_memory,100);}catch(const MemoryFault &) {reference_fault=true;}
    require(actual_fault && reference_fault && engine.wasm.executed_instructions>0,"optimized Wasm fault path was not exercised");
    require(actual.x==reference.x && actual.sp==reference.sp && actual.pc==reference.pc && actual.nzcv==reference.nzcv,"optimized Wasm lost state before a fault");
}
void tests() {
    optimizer_tests();
    {
        std::mutex mutex;std::condition_variable cv;bool first_waiting=false,second_serviced=false;
        bool first_ok=false,second_ok=false,overlapped=false;
        auto transfer=[] {WasmTransfer t;t.state.resize(WasmRegion::state_size);uint32_t remaining=1;
            std::memcpy(t.state.data()+WasmRegion::remaining_offset,&remaining,sizeof(remaining));return t;};
        std::thread first([&] {
            auto t=transfer();first_ok=wasm_webkit_run(UINT64_MAX-1,t,[&](WasmTransfer &request) {
                std::unique_lock<std::mutex> lock(mutex);first_waiting=true;cv.notify_all();
                overlapped=cv.wait_for(lock,std::chrono::seconds(10),[&] {return second_serviced;});request.stop=true;
            });
        });
        std::thread second([&] {
            {std::unique_lock<std::mutex> lock(mutex);if(!cv.wait_for(lock,std::chrono::seconds(10),[&] {return first_waiting;}))return;}
            auto t=transfer();second_ok=wasm_webkit_run(UINT64_MAX-2,t,[&](WasmTransfer &request) {
                {std::lock_guard<std::mutex> lock(mutex);second_serviced=true;}cv.notify_all();request.stop=true;
            });
        });
        first.join();second.join();wasm_webkit_forget(UINT64_MAX-1);wasm_webkit_forget(UINT64_MAX-2);
        require(first_ok && second_ok && overlapped,"independent sessions failed to make concurrent progress");
    }
    CpuState cpu;cpu.x[2]=20000;cpu.sp=0x6000000800;cpu.tls=123;cpu.nzcv=13;cpu.v[3][5]=79;
    const std::vector<uint32_t> loop{0x91000400,0x91000821,0xd1000442,0xb5ffffa2,0xd4000001};
    for(unsigned tail=0;tail<8;++tail)check("budget_"+std::to_string(tail),loop,cpu,40000+tail);
    check("syscall",loop,cpu,100000);
    std::vector<uint32_t> cfg{0x91000400,0xb4000061,0x91000c84,0x14000002,0x91001ca5,0xd1000442,0xb5ffff42,0xd4000001};
    for(unsigned word=0;word<2;++word)for(unsigned sense=0;sense<2;++sense)for(unsigned choice=0;choice<2;++choice) {cfg[1]=(word ? 0x34000061:0xb4000061)|(sense<<24);cpu.x[1]=(uint64_t(word)<<32)|choice;check("cfg_"+std::to_string(word)+"_"+std::to_string(sense)+"_"+std::to_string(choice),cfg,cpu,200000);}
    for(unsigned width=0;width<2;++width)for(unsigned cond=0;cond<16;++cond) {
        // Conditions all observe flags from SUBS, including zero/negative/overflow/carry cases.
        cpu.x[0]=UINT64_MAX;cpu.x[2]=20000;
        std::vector<uint32_t> flags{0xb1000400u-(width ? 0:0x80000000u),0xf1000442u-(width ? 0:0x80000000u),0x54000040u|cond,0x910004a5,0xb5ffff82,0xd4000001};
        check("flags_"+std::to_string(width)+"_"+std::to_string(cond),flags,cpu,50003);
    }
    for(uint64_t value:{0u,7u})for(unsigned budget=1;budget<=5;++budget) {
        auto state=cpu;state.x[0]=value;
        check("conditional_fallthrough",{0xf100001f,0x54000060,0xf9400461,0x91000442,0xd4000001},state,budget,1,false,budget==5);
    }
    check("memory",{0xf9400460,0x91000400,0xf9000460,0xd1000442,0xb5ffff82,0xd4000001},cpu,100003);
    {auto loads=cpu;loads.x[2]=0x8000ffff800080ffull;check("narrow_load_extension",{0xf9000462,0x39402060,0x39c02061,0x79401062,0x79c01064,0xb9400865,0xd4000001},loads,7);}
    check("dynamic_base",{0xf9400063,0xf9400460,0x91000400,0xf9000460,0xd1000442,0xb5ffff62,0xd4000001},cpu,120003);
    auto boundary=cpu;boundary.x[2]=1;
    check("memory_side_exit",{0xf9000460,0x91400463,0xf9400461,0xd1000442,0xb5ffff82,0xd4000001},boundary,10003);
    check("cross_page",{0x913ff063,0xf9400060,0xd4000001},boundary,10003);
    check("shared_fallback",{0xf9400460,0x91000400,0xf9000460,0xd1000442,0xb5ffff82,0xd4000001},cpu,100003,false,true);
    auto exclusive=cpu;exclusive.x[2]=123;
    check("exclusive_success",{0xc85f7c60,0xc8017c62,0xd4000001},exclusive,100);
    check("exclusive_same_page_write",{0xc85f7c60,0xf9000462,0xc8017c62,0xd4000001},exclusive,100);
    check("exclusive_page_fault",{0xc85f7c60,0x91400464,0xf9400085,0xc8017c62,0xd4000001},exclusive,100);
    check("exclusive_other_page_write",{0xc85f7c60,0x91400464,0xf9000082,0xc8017c62,0xd4000001},exclusive,100);
    {
        Memory memory(1<<22);prepare(memory,{0xf9400460,0xf9000461,0xd4000001});ExecutionEngine engine;
        CpuState state;state.pc=entry;state.x[3]=data;state.x[1]=23;
        engine.run(state,memory,100);require(engine.wasm.executed_instructions && memory.load<8>(data+8)==23,"permission setup");
        memory.protect(data,4096,read_perm);state.pc=entry;state.x[1]=42;
        bool fault=false;try {engine.run(state,memory,100);}catch(const MemoryFault &) {fault=true;}
        require(fault && memory.load<8>(data+8)==23,"revoked write permission ignored");
        memory.protect(data,4096,read_perm|write_perm);state.pc=entry;
        engine.run(state,memory,100);require(memory.load<8>(data+8)==42,"restored write permission ignored");
    }
    {
        Memory memory(1<<22);prepare(memory,{0xf9400460,0xf9400481,0xf9000480,0xd4000001});ExecutionEngine engine;
        const auto alias=data^uint64_t{0x10001000};require(WasmRegion::page_slot(data)==WasmRegion::page_slot(alias),"collision setup");
        memory.map(alias,4096,read_perm|write_perm);memory.store<8>(alias+8,29);
        CpuState state;state.pc=entry;state.x[3]=data;state.x[4]=alias;
        engine.run(state,memory,100);require(engine.wasm.executed_instructions && state.x[0]==17 && state.x[1]==29 && memory.load<8>(alias+8)==17,"colliding page tags or writeback incorrect");
    }
    for(uint32_t call:{0x94000008u,0xd63f0080u}) {
        std::vector<uint32_t> words(11,0xd503201f);words[0]=call;words[1]=0x91000400;words[2]=0xd4000001;words[8]=0x910043ff;words[9]=0xb1001400;words[10]=0xd65f03c0;
        auto memory=std::make_unique<Memory>(1<<22),reference_memory=std::make_unique<Memory>(1<<22);prepare(*memory,words);prepare(*reference_memory,words);ExecutionEngine engine;
        CpuState warm;warm.pc=entry+32;warm.x[30]=entry+8;engine.run(warm,*memory,100);
        for(unsigned budget:{2u,3u,4u,5u,6u,7u,100u}) {
            CpuState state;state.pc=entry;state.sp=data+512;state.x[4]=entry+32;auto reference=state;const auto dispatches=engine.wasm.dispatches;
            const auto result=engine.run(state,*memory,budget),expected=scalar(reference,*reference_memory,budget);
            require(state.x==reference.x && state.sp==reference.sp && state.pc==reference.pc && state.nzcv==reference.nzcv && result.instructions==expected.instructions && result.trap==expected.trap,"linked call state or budget mismatch");
            if(budget==100)require(engine.wasm.dispatches>dispatches && engine.wasm.dispatches-dispatches<=2,"linked call did not resume caller");
        }
    }
    for(uint32_t target:{0xd4000001u,0xd61f00c0u}) {
        std::vector<uint32_t> words(9,0xd503201f);words[0]=0x94000008;words[1]=0x91000400;words[2]=0xd4000001;words[8]=target;
        auto memory=std::make_unique<Memory>(1<<22),reference_memory=std::make_unique<Memory>(1<<22);prepare(*memory,words);prepare(*reference_memory,words);ExecutionEngine engine;
        CpuState warm;warm.pc=entry+32;warm.x[6]=entry+8;engine.run(warm,*memory,100);
        CpuState state;state.pc=entry;state.x[6]=entry+8;auto reference=state;const auto dispatches=engine.wasm.dispatches;
        const auto result=engine.run(state,*memory,100),expected=scalar(reference,*reference_memory,100);
        require(state.x==reference.x && state.pc==reference.pc && state.nzcv==reference.nzcv && result.instructions==expected.instructions && result.trap==expected.trap,"linked nonreturn exit mismatch");
        require(engine.wasm.dispatches-dispatches<=2,"linked nonreturn exit was not exercised");
    }
    {
        auto memory=std::make_unique<Memory>(1<<22);prepare(*memory,{0xd63f0080,0xd4000001});ExecutionEngine engine;
        CpuState state;state.pc=entry;bool fault=false;try {engine.run(state,*memory,100);}catch(const MemoryFault &) {fault=true;}
        require(fault,"empty code slot at address zero did not fault");
    }
    {
        std::vector<uint32_t> words(10,0xd503201f);words[0]=0xd63f0080;words[1]=0xd65f03c0;words[2]=0xd4000001;words[8]=0xd63f00a0;words[9]=0xd65f03c0;
        auto memory=std::make_unique<Memory>(1<<22),reference_memory=std::make_unique<Memory>(1<<22);prepare(*memory,words);prepare(*reference_memory,words);ExecutionEngine engine;
        CpuState warm;warm.pc=entry+32;warm.x[5]=entry+8;engine.run(warm,*memory,100);
        for(unsigned budget:{1u,2u,3u,31u,32u,33u,63u,64u,65u,100u,101u,127u}) {
            CpuState state;state.pc=entry;state.x[4]=entry+32;state.x[5]=entry;auto reference=state;const auto dispatches=engine.wasm.dispatches;
            const auto result=engine.run(state,*memory,budget),expected=scalar(reference,*reference_memory,budget);
            require(state.x==reference.x && state.pc==reference.pc && result.instructions==expected.instructions && result.trap==expected.trap,"linked recursion budget mismatch");
            if(budget==100)require(engine.wasm.dispatches-dispatches<50,"linked recursion was not exercised");
        }
    }
    {
        std::vector<uint32_t> words(9,0xd503201f);words[0]=0xd61f0080;words[2]=0xd4000001;words[8]=0x17fffff8;
        auto memory=std::make_unique<Memory>(1<<22),reference_memory=std::make_unique<Memory>(1<<22);prepare(*memory,words);prepare(*reference_memory,words);ExecutionEngine engine;
        CpuState warm;warm.pc=entry;warm.x[4]=entry+8;engine.run(warm,*memory,100);
        CpuState state;state.pc=entry+32;state.x[4]=entry+32;auto reference=state;const auto dispatches=engine.wasm.dispatches;
        const auto result=engine.run(state,*memory,100),expected=scalar(reference,*reference_memory,100);
        require(state.x==reference.x && state.pc==reference.pc && result.instructions==expected.instructions && result.trap==expected.trap,"linked jump cycle mismatch");
        require(engine.wasm.dispatches>dispatches && engine.wasm.dispatches-dispatches<50,"linked jump cycle was not exercised");
    }
    auto big=std::vector<uint32_t>(12288,0x91000400);big.push_back(0xd1000442);big.push_back(0xb5000002u|((-12289u&0x7ffff)<<5));big.push_back(0xd4000001);
    check("large_region",big,cpu,1000007);
    std::vector<uint32_t> many;
    for(unsigned n=0;n<320;++n)many.insert(many.end(),{0xb4000041,0x91000400,0x91000484});
    many.push_back(0xd1000442);many.push_back(0xb5000002u|((-uint32_t(many.size())&0x7ffff)<<5));many.push_back(0xd4000001);
    check("many_blocks",many,cpu,1000007);
    check("direct_call",{0x94000004,0xd1000442,0xb5ffffc2,0xd4000001,0x91000400,0xd65f03c0},cpu,100003);
    cpu.x[8]=entry+16;
    check("indirect_call",{0xd63f0100,0xd1000442,0xb5ffffc2,0xd4000001,0x91000400,0xd65f03c0},cpu,100003);
    for(unsigned wide=0;wide<2;++wide)for(unsigned sub=0;sub<2;++sub)for(auto a:{uint64_t{0},uint64_t{0x7fffffff},uint64_t{0x7fffffffffffffff},UINT64_MAX}) {
        cpu.x[0]=a;cpu.x[1]=1;
        check("flags_boundary",{(wide ? 0xab010000u:0x2b010000u)|(sub ? 0x40000000u:0),0x17ffffff},cpu,2);
    }
    for(unsigned seed=0;seed<16;++seed) {
        cpu.x[0]=UINT64_MAX-seed*73;cpu.x[1]=uint64_t(seed)<<47;cpu.x[2]=17;
        check("integer_mix",{0xaa010000,0xca000021,0x9b020000,0xd1000442,0xb5ffff82,0xd4000001},cpu,10003);
        check("word_sp",{0x11000400,0x0b000021,0x910023ff,0xd1000442,0xb5ffff82,0xd4000001},cpu,10003);
    }
    CpuState mixed;
    for(unsigned r=0;r<31;++r)mixed.x[r]=0xfedcba9876543210ull+r*17;
    for(unsigned r=0;r<32;++r)for(unsigned b=0;b<16;++b)mixed.v[r][b]=r*3+b;
    mixed.x[4]=1;mixed.tls=0x123456789abcdef0; mixed.sp=data+512;
    const std::vector<uint32_t> extended{0xa9410460,0xa9010460,0xa8c10460,0xa9810460,0x69400460,0xf8647860,0x38a46860,0x38e46860,0x78a4c860,0xb9800860,0x39001460,0xf8408460,0xf8008c60,0xad400460,0xad000460,0x3dc00060,0x3d800060,0xfd400060,0xfd000060,0xbd400060,0xbd000060,0x93437420,0xb34f1c20,0xd3452420,0x13074820,0x331d2020,0x93c24420,0x13823420,0x8b22c820,0xb228420,0xeb821020,0x8b420c20,0xeac23420,0x6aa21c20,0x9a820020,0xda821420,0x5a82c020,0x1a82d420,0xfa42002d,0x3a45182e,0x9ac22020,0x1ac22420,0x9ac22820,0x1ac22c20,0x9ac20820,0x1ac20c20,0x5ac00020,0xdac00c20,0x5ac00420,0xdac00820,0x5ac01020,0xdac01420,0x9b221020,0x9ba21020,0xd53bd040,0xd53b4200,0xd51bd041,0xd51b4201};
    for(unsigned n=0;n<extended.size();++n)for(unsigned flags:{0u,7u,15u}) {
        mixed.nzcv=flags;check("extended_"+std::to_string(n)+"_"+std::to_string(flags),{extended[n],0xd4000001},mixed,2);
    }
    for(uint32_t op:{0x0ea0f820u,0x4ea0f820u,0x4ee0f820u,0x2ea0f820u,0x6ea0f820u,0x6ee0f820u})for(unsigned alias:{0u,1u})for(uint64_t bits:{0ull,0x8000000000000000ull,0x7ff0000000000001ull,0xfff8000080000000ull}) {
        auto f=mixed;std::memcpy(f.v[1].data(),&bits,8);std::memcpy(f.v[1].data()+8,&bits,8);f.fpcr=0x400000;f.fpsr=1;
        op|=alias;require(WasmRegion::supports(decode(op)),"vector sign operation not compiled");check("vector_sign",{op,0xd4000001},f,2);
    }
    const std::vector<uint32_t> floating{0x9e660020,0x1e260020,0x9e670020,0x1e270020,0x9eae0020,0x9eaf0020,0x1e6e1000,0x1e60c020,0x1e214020,0x1e204020,0x4e221c20,0x4e621c20,0x4ea21c20,0x4ee21c20,0x6e221c20,0x6e621c20,0x6ea21c20,0x6ee21c20,0x1e622820,0x1e623820,0x1e620820,0x1e621820,0x1e61c020,0x1e622020,0x1e602028,0x1e620c20,0x1f421020,0x1f429020,0x1f621020,0x1f629020,0x9e620020,0x9e630020,0x9e780020,0x1e780020};
    for(unsigned n=0;n<floating.size();++n)for(unsigned status:{0u,16u})for(double input:{1.25,-2.5,0.0,-0.0,0x1p-1074,0x0.fffffffffffffp-1022,0x1p-1022,0x1p500,0x1.fffffffffffffp1023,__builtin_inf(),__builtin_nan("")}) {
        auto f=mixed;f.x[1]=0xfedcba9876543210;f.fpsr=status;
        const double b=-2.75,c=4.2;std::memcpy(f.v[1].data(),&input,8);std::memcpy(f.v[2].data(),&b,8);std::memcpy(f.v[4].data(),&c,8);
        const bool require_fma_offload=n>=26 && n<=29 && status==16 && input==1.25;
        check("floating_"+std::to_string(n)+"_"+std::to_string(status),{floating[n],0xd4000001},f,2,true,false,require_fma_offload);
    }
    benchmark("resident",loop,30000000);benchmark("large_region",big,1000);
    {
        const std::vector<uint32_t> memory_loop{0xf9400460,0x91000400,0xf9000460,0xd1000442,0xb5ffff82,0xd4000001};
        auto a=std::make_unique<Memory>(1<<22),b=std::make_unique<Memory>(1<<22);prepare(*a,memory_loop);prepare(*b,memory_loop);
        ExecutionEngine first,second;
        auto run=[&](ExecutionEngine &engine,Memory &memory,uint64_t expected) {
            CpuState state;state.pc=entry;state.x[2]=20;state.x[3]=data;
            auto result=engine.run(state,memory,1000);
            require(result.trap==Trap::Syscall && state.x[0]==expected && memory.load<8>(data+8)==expected,"cached session memory mismatch");
        };
        run(first,*a,37);const auto uploads=first.wasm.module_uploads,bytes=first.wasm.module_bytes;
        require(uploads>0 && bytes>0,"initial module not uploaded");
        run(first,*a,57);
        require(first.wasm.module_uploads==uploads && first.wasm.module_bytes==bytes,"cached module uploaded again");
        run(second,*b,37);const auto second_uploads=second.wasm.module_uploads;
        const auto first_faults=first.wasm.page_faults,second_faults=second.wasm.page_faults;
        run(first,*a,77);run(second,*b,57);
        require(first.wasm.module_uploads==uploads && second.wasm.module_uploads==second_uploads,"session switch reuploaded module");
        require(first.wasm.page_faults==first_faults && second.wasm.page_faults==second_faults,"session switch lost resident page");
        std::exception_ptr errors[2];
        std::thread one([&] {try {run(first,*a,97);}catch(...) {errors[0]=std::current_exception();}});
        std::thread two([&] {try {run(second,*b,77);}catch(...) {errors[1]=std::current_exception();}});
        one.join();two.join();
        for(auto error:errors)if(error)std::rethrow_exception(error);
        require(first.wasm.module_uploads==uploads && second.wasm.module_uploads==second_uploads,"concurrent sessions lost module cache");
    }
    Memory m(1<<22);prepare(m,loop);ExecutionEngine engine;cpu={};cpu.x[2]=20000;cpu.pc=entry;
    engine.run(cpu,m,40003);require(engine.wasm.executed_instructions>0,"resume first run");
    auto expected=cpu;
    engine.run(cpu,m,40005);scalar(expected,m,40005);require(cpu.x==expected.x && cpu.pc==expected.pc,"resume mismatch");
    m.store<4>(entry,0x91001c00);cpu={};cpu.pc=entry;cpu.x[2]=20000;expected=cpu;
    engine.run(cpu,m,40003);scalar(expected,m,40003);require(cpu.x==expected.x && cpu.pc==expected.pc,"modified code ignored");
    m.protect(entry,0x10000,read_perm);cpu.pc=entry;
    bool fault=false;try {engine.run(cpu,m,5000);}catch(const MemoryFault &) {fault=true;}require(fault,"execute revocation ignored");
}
}
#if !TARGET_OS_IPHONE
int operit_wasm_cli_main(int,char **);
int main(int argc,char **argv) {
    @autoreleasepool {
        [NSApplication sharedApplication];[NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];[NSApp finishLaunching];
        int status=0;
        std::thread worker([&] {try {if(argc>1)status=operit_wasm_cli_main(argc,argv);else tests();}catch(const std::exception &e) {std::fprintf(stderr,"%s\n",e.what());status=1;}dispatch_async(dispatch_get_main_queue(),^{CFRunLoopStop(CFRunLoopGetMain());});});
        CFRunLoopRun();
        worker.join();return status;
    }
}

#else
@interface OperitWasmApp : UIResponder <UIApplicationDelegate>
@end
@implementation OperitWasmApp
@end
@interface OperitWasmScene : UIResponder <UIWindowSceneDelegate>
@property(nonatomic,strong) UIWindow *window;
@end
@implementation OperitWasmScene
- (void)scene:(UIScene *)scene willConnectToSession:(UISceneSession *)session options:(UISceneConnectionOptions *)options {
    (void)session;(void)options;
    self.window=[[UIWindow alloc] initWithWindowScene:(UIWindowScene *)scene];
    self.window.rootViewController=[UIViewController new];[self.window makeKeyAndVisible];
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED,0),^{
        @autoreleasepool {
            NSString *directory=NSSearchPathForDirectoriesInDomains(NSDocumentDirectory,NSUserDomainMask,YES).firstObject;
            freopen([[directory stringByAppendingPathComponent:@"integration.log"] fileSystemRepresentation],"w",stdout);
            int status=0;try {tests();}catch(const std::exception &e) {fprintf(stdout,"failure: %s\n",e.what());status=1;}
            fflush(stdout);
            NSData *result=[NSJSONSerialization dataWithJSONObject:@{@"status":@(status)} options:0 error:nil];
            [result writeToFile:[directory stringByAppendingPathComponent:@"results.json"] atomically:YES];
        }
    });
}
@end
int main(int argc,char **argv) { @autoreleasepool {return UIApplicationMain(argc,argv,nil,NSStringFromClass(OperitWasmApp.class));} }
#endif
