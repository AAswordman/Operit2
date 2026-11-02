#include "cpu/aarch64/instruction_fault.h"
#include "cpu/aarch64/interpreter.h"
#include <cstdio>
#include <stdexcept>
using namespace operit::arm64;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
void faults(uint32_t opcode, InstructionFault::Kind expected) {
    CpuState cpu;
    Memory memory(4096);
    try {
        execute(cpu, memory, decode(opcode));
    } catch (const InstructionFault &fault) {
        require(fault.kind == expected, "architectural fault had wrong kind");
        return;
    }
    throw std::runtime_error("architectural fault instruction accepted: " + hex(opcode));
}
void rejects(uint32_t opcode) {
    CpuState cpu;
    Memory memory(4096);
    try {
        execute(cpu, memory, decode(opcode));
    } catch (const Failure &failure) {
        require(failure.status == ORT_ARM64_UNSUPPORTED, "invalid encoding was not unsupported");
        return;
    }
    throw std::runtime_error("reserved/unsupported encoding accepted: " + hex(opcode));
}
} // namespace
int main() {
    try {
        faults(0x00000000, InstructionFault::Kind::Undefined);
        faults(0xd4200000, InstructionFault::Kind::Breakpoint);
        faults(0xd4400000, InstructionFault::Kind::Halt);
        faults(0xd69f03e0, InstructionFault::Kind::Privileged);
        rejects(0x0f400420); // reserved non-scalar 1D immediate shift
        rejects(0x0ee28420); // reserved non-scalar 1D ADD
        rejects(0x0ee0b820); // reserved non-scalar 1D ABS
        rejects(0x6e022c20); // INS halfword, unaligned source imm4
        rejects(0x9ac54c83); // CRC32X is not in the advertised guest profile
        rejects(0xdc000000); // reserved SIMD literal load (opc=3)
        std::puts("reserved encodings and unavailable extension checks passed");
        return 0;
    } catch (const std::exception &e) {
        std::fprintf(stderr, "%s\n", e.what());
        return 1;
    }
}
