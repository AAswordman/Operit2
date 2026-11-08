#include "cpu/aarch64/execution_engine.h"
#include <chrono>
#include <cstdio>
#include <cstring>
using namespace operit::arm64;
int main(int argc,char **argv) {
    const bool loads=argc==2 && !std::strcmp(argv[1],"loads");
    if(argc!=2 || (!loads && std::strcmp(argv[1],"cse")))return 2;
    const uint32_t cse[]{0x8b0600a3,0x14000001,0x8b0600a3,0x8b030067,0x910004e8,0x17fffffb};
    const uint32_t load[]{0xf94004a0,0x91000402,0xf94000a1,0x8b010043,0x17fffffc};
    const auto *words=loads ? load:cse;const auto count=loads ? std::size(load):std::size(cse);
    Memory memory(1<<20);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,read_perm|write_perm|exec_perm);memory.copy_in(0x10000,words,count*4);
    memory.map(0x20000,page_size,read_perm|write_perm);memory.store<8>(0x20000,11);memory.store<8>(0x20008,17);
    CpuState initial;initial.pc=0x10000;initial.x[5]=loads ? 0x20000:11;initial.x[6]=17;
    ExecutionEngine engine;auto cpu=initial;engine.run(cpu,memory,30000);
    cpu=initial;auto expected=initial;
    for(size_t n=0;n<count;++n) {
        const auto i=decode(memory.load<4>(expected.pc,exec_perm));select_semantic(i)(expected,memory,i);
    }
    const auto start=std::chrono::steady_clock::now();const auto result=engine.run(cpu,memory,300000000);
    const auto elapsed=std::chrono::duration<double>(std::chrono::steady_clock::now()-start).count();
    if(result.instructions!=300000000 || cpu.x!=expected.x || cpu.pc!=expected.pc || cpu.nzcv!=expected.nzcv)return 1;
    std::printf("{\"mode\":\"%s\",\"seconds\":%.9f,\"instructions\":%llu,\"slow_calls\":%llu,\"checksum\":%llu}\n",
        argv[1],elapsed,(unsigned long long)result.instructions,(unsigned long long)engine.slow_calls,
        (unsigned long long)(loads ? cpu.x[0]+cpu.x[1]+cpu.x[2]+cpu.x[3]:cpu.x[3]+cpu.x[7]+cpu.x[8]));
}
