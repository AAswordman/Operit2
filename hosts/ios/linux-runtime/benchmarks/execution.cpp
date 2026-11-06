#include "cpu/aarch64/execution_engine.h"
#include <chrono>
#include <cstdio>
using namespace operit::arm64;
int main() {
    Memory memory(1<<20);memory.set_parallel_access(false);
    memory.map(0x10000,page_size,read_perm|write_perm|exec_perm);
    const uint32_t words[]={0x91000400,0x91000821,0x17fffffe};
    memory.copy_in(0x10000,words,sizeof(words));
    ExecutionEngine engine;CpuState cpu;cpu.pc=0x10000;
    engine.run(cpu,memory,3000);cpu.pc=0x10000;cpu.x.fill(0);
    const auto start=std::chrono::steady_clock::now();
    const auto result=engine.run(cpu,memory,30000000);
    const auto seconds=std::chrono::duration<double>(std::chrono::steady_clock::now()-start).count();
    std::printf("instructions=%llu seconds=%.6f MIPS=%.2f checksum=%llu blocks=%llu slow=%llu materializations=%llu\n",
        (unsigned long long)result.instructions,seconds,result.instructions/seconds/1e6,
        (unsigned long long)(cpu.x[0]+cpu.x[1]),(unsigned long long)engine.decoded_blocks,
        (unsigned long long)engine.slow_calls,(unsigned long long)engine.materializations);
    return cpu.x[0]==10000000 && cpu.x[1]==20000000 ? 0:1;
}
