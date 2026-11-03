#include "cpu/aarch64/cache.h"
#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/instruction_fault.h"
#include "memory/fault.h"
#include "cpu/aarch64/interpreter.h"
#include <cstring>
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
#if defined(__aarch64__)
void floating_environment() {
    struct Restore {
        uint64_t control, status;
        Restore() {
            asm volatile("mrs %0, fpcr\n\tmrs %1, fpsr" : "=r"(control), "=r"(status) : : "memory");
        }
        ~Restore() {
            asm volatile("msr fpcr, %0\n\tmsr fpsr, %1" : : "r"(control), "r"(status) : "memory");
        }
    } restore;

    const auto check = [](uint64_t host_control, uint64_t host_status, uint64_t guest_control,
                          bool raise_exception) {
        asm volatile("msr fpcr, %0\n\tmsr fpsr, %1" : : "r"(host_control), "r"(host_status) : "memory");
        CpuState cpu{};
        cpu.fpcr = guest_control;
        cpu.fpsr = 0x10;
        double value;
        const double one = 1.0, zero = 0.0;
        {
            FloatingPointEnvironment environment(cpu);
            if (raise_exception)
                asm volatile("fdiv %d0, %d1, %d2" : "=w"(value) : "w"(one), "w"(zero) : "memory");
            else
                asm volatile("fadd %d0, %d1, %d1" : "=w"(value) : "w"(one) : "memory");
        }
        uint64_t control, status;
        asm volatile("mrs %0, fpcr\n\tmrs %1, fpsr" : "=r"(control), "=r"(status) : : "memory");
        require(control == host_control, "floating helper changed host FPCR");
        require(status == host_status, "floating helper changed host FPSR");
        require(cpu.fpsr == (raise_exception ? 0x12 : 0x10), "guest floating exception flags incorrect");
    };
    constexpr uint64_t rounding_mode = uint64_t{1} << 22;
    check(0, 0, 0, false);
    check(0, 0, 0, true);
    check(0, 1, 0, false);
    check(0, 1, 0, true);
    check(rounding_mode, 0, 0, true);
    check(0, 1, rounding_mode, true);
}
void floating_dependency_chain() {
    CpuState cpu{};
    Memory memory(4096);
    const auto set = [&cpu](unsigned reg, double value) {
        std::memcpy(cpu.v[reg].data(), &value, sizeof(value));
    };
    const auto get = [&cpu](unsigned reg) {
        double value;
        std::memcpy(&value, cpu.v[reg].data(), sizeof(value));
        return value;
    };
    set(0, 1.0);
    set(1, 0.5);
    set(2, 1.0000000001);
    set(3, 1e18);
    set(8, 1.0);
    double expected = 1.0;
    const double multiplier = 1.0000000001, increment = 0.5;
    for (unsigned iteration = 0; iteration < 100; ++iteration) {
        asm volatile("fmadd %d0, %d1, %d2, %d3" : "=w"(expected)
                     : "w"(expected), "w"(multiplier), "w"(increment));
        execute(cpu, memory, decode(0x1f420504)); // fmadd d4, d8, d2, d1
        if (get(4) != expected)
            std::fprintf(stderr, "FMADD iteration %u: expected %.17g, got %.17g\n",
                         iteration, expected, get(4));
        require(get(4) == expected, "FMADD dependency result incorrect");
        execute(cpu, memory, decode(0x1e632080)); // fcmp d4, d3
        require(cpu.nzcv == 8, "FCMP dependency flags incorrect");
        execute(cpu, memory, decode(0x1e64cc08)); // fcsel d8, d0, d4, gt
        require(get(8) == expected, "FCSEL dependency result incorrect");
    }
}
void floating_interpreter_dispatch() {
    Memory memory(4096);
    memory.map(0x10000, 4096, read_perm | write_perm | exec_perm);
    Interpreter interpreter;
    constexpr uint64_t inputs[] = {0, 0x8000000000000000ull, 0x3ff0000000000000ull,
                                   0x4008000000000000ull, 0x7ff0000000000000ull,
                                   0x7ff0000000000001ull, 0x0000000000000001ull};
    constexpr uint32_t operations[] = {0x1f420504, 0x1e61c000, 0x1e612808,
                                       0x1e632080, 0x1e64cc08, 0x4e7ecfbf};
    for (const auto instruction : operations) {
        for (unsigned alias = 0; alias < 4; ++alias) {
            uint32_t word = instruction;
            const unsigned rd = word & 31;
            if (alias == 1)
                word = (word & ~(31u << 5)) | (rd << 5);
            else if (alias == 2 && instruction != 0x1e61c000)
                word = (word & ~(31u << 16)) | (rd << 16);
            else if (alias == 3 && (instruction & 0xffe08000) == 0x1f400000)
                word = (word & ~(31u << 10)) | (rd << 10);
            memory.store<4>(0x10000, word);
            for (unsigned sample = 0; sample < std::size(inputs); ++sample) {
                CpuState reference{}, actual{};
                reference.pc = 0x10000;
                reference.fpcr = (sample & 1) ? uint64_t{1} << 22 : 0;
                reference.fpsr = (sample & 1) ? 0x12 : 0;
                reference.nzcv = sample & 15;
                for (unsigned reg = 0; reg < reference.v.size(); ++reg) {
                    const uint64_t low = inputs[(sample + reg) % std::size(inputs)];
                    const uint64_t high = inputs[(sample + reg + 3) % std::size(inputs)];
                    std::memcpy(reference.v[reg].data(), &low, sizeof(low));
                    std::memcpy(reference.v[reg].data() + 8, &high, sizeof(high));
                }
                actual = reference;
                execute(reference, memory, decode(word));
                const auto result = interpreter.run(actual, memory, 1);
                if (reference.v != actual.v || reference.fpsr != actual.fpsr ||
                    reference.nzcv != actual.nzcv || reference.pc != actual.pc) {
                    std::fprintf(stderr, "FP dispatch mismatch: opcode=%08x alias=%u sample=%u\n",
                                 word, alias, sample);
                    throw std::runtime_error("optimized FP dispatch differs from interpreter");
                }
                require(result.instructions == 1, "optimized FP dispatch retired wrong instruction count");
            }
        }
    }
}
void unsigned_to_double_dispatch() {
    Memory memory(4096);
    memory.map(0x10000, 4096, read_perm | write_perm | exec_perm);
    Interpreter interpreter;
    constexpr uint64_t inputs[] = {0, 1, (uint64_t{1} << 53) - 1, uint64_t{1} << 53,
                                   (uint64_t{1} << 53) + 1, uint64_t{1} << 63, UINT64_MAX};
    for (unsigned rn : {0u, 7u, 31u}) {
        for (unsigned rd : {0u, 7u, 31u}) {
            const uint32_t word = 0x9e630000 | (rn << 5) | rd;
            memory.store<4>(0x10000, word);
            for (const auto value : inputs) {
                for (unsigned rounding = 0; rounding < 4; ++rounding) {
                    CpuState reference{};
                    reference.pc = 0x10000;
                    reference.fpcr = uint64_t{rounding} << 22;
                    reference.fpsr = 0x12;
                    if (rn != 31)
                        reference.x[rn] = value;
                    reference.v[rd].fill(0xff);
                    CpuState actual = reference;
                    execute(reference, memory, decode(word));
                    const auto result = interpreter.run(actual, memory, 1);
                    require(actual.v == reference.v && actual.x == reference.x &&
                                actual.fpsr == reference.fpsr && actual.fpcr == reference.fpcr &&
                                actual.pc == reference.pc && result.instructions == 1,
                            "UCVTF dispatch differs from interpreter");
                }
            }
        }
    }
}
void compare_register_dispatch() {
    Memory memory(4096);
    memory.map(0x10000, 4096, read_perm | write_perm | exec_perm);
    Interpreter interpreter;
    constexpr uint64_t inputs[] = {0, 1, 2, INT64_MAX, uint64_t{1} << 63, UINT64_MAX};
    for (unsigned rn : {0u, 7u, 31u}) {
        for (unsigned rm : {0u, 7u, 31u}) {
            const uint32_t word = 0xeb00001f | (rm << 16) | (rn << 5);
            memory.store<4>(0x10000, word);
            for (const auto lhs : inputs) {
                for (const auto rhs : inputs) {
                    CpuState reference{};
                    reference.pc = 0x10000;
                    if (rn != 31)
                        reference.x[rn] = lhs;
                    if (rm != 31)
                        reference.x[rm] = rhs;
                    CpuState actual = reference;
                    execute(reference, memory, decode(word));
                    const auto result = interpreter.run(actual, memory, 1);
                    require(actual.x == reference.x && actual.nzcv == reference.nzcv &&
                                actual.pc == reference.pc && result.instructions == 1,
                            "CMP register dispatch differs from interpreter");
                }
            }
        }
    }
}
#endif
void cache_operations() {
    CpuState cpu;
    Memory memory(4096);
    memory.map(0x10000, 4096, read_perm | write_perm | exec_perm);
    std::array<uint8_t, 192> contents;
    contents.fill(0xa5);
    memory.copy_in(0x10000, contents.data(), contents.size());
    execute(cpu, memory, decode(0xd53b0023)); // MRS X3, CTR_EL0: Node/V8 probe.
    require(cpu.x[3] == cache_profile::ctr_el0, "CTR_EL0 guest geometry");
    require((4u << (cpu.x[3] & 15)) == cache_profile::line_bytes &&
                (4u << ((cpu.x[3] >> 16) & 15)) == cache_profile::line_bytes,
            "CTR_EL0 line sizes disagree with implementation");
    execute(cpu, memory, decode(0xd53b00e3)); // MRS X3, DCZID_EL0.
    require(cpu.x[3] == cache_profile::dczid_el0 && !(cpu.x[3] & 16),
            "DCZID_EL0 must advertise the implemented ZVA block size");
    cpu.x[5] = 0x10051;
    for (const auto operation : {cache_profile::dc_cvau, cache_profile::dc_cvac,
                                 cache_profile::dc_civac, cache_profile::ic_ivau}) {
        const auto epoch = memory.code_epoch.load();
        require(decode(operation | 5).op == Op::Cache, "cache operation not decoded");
        execute(cpu, memory, decode(operation | 5));
        require(memory.code_epoch.load() > epoch, "cache operation did not invalidate decoded code");
    }
    execute(cpu, memory, decode(cache_profile::dc_zva | 5));
    memory.copy_out(0x10000, contents.data(), contents.size());
    for (size_t index = 0; index < contents.size(); ++index)
        require(contents[index] == (index >= 64 && index < 128 ? 0 : 0xa5),
                "DC ZVA must zero exactly the aligned guest block");
    cpu.x[5] = 0x10fff;
    execute(cpu, memory, decode(cache_profile::dc_zva | 5));
    std::array<uint8_t, cache_profile::line_bytes> last{};
    memory.copy_out(0x10fc0, last.data(), last.size());
    for (const auto byte : last)
        require(byte == 0, "DC ZVA crossed the final mapped cache line");
    memory.protect(0x10000, 4096, read_perm);
    try {
        execute(cpu, memory, decode(cache_profile::dc_zva | 5));
        throw std::runtime_error("DC ZVA bypassed guest write protection");
    } catch (const MemoryFault &fault) {
        require(fault.kind == MemoryFault::Kind::Protection, "DC ZVA wrong fault kind");
    }
    cpu.x[5] = 0x20000;
    for (const auto operation : {cache_profile::dc_cvau, cache_profile::ic_ivau,
                                 cache_profile::dc_zva}) {
        try {
            execute(cpu, memory, decode(operation | 5));
            throw std::runtime_error("cache operation accepted an unmapped guest address");
        } catch (const MemoryFault &fault) {
            require(fault.kind == MemoryFault::Kind::Unmapped, "cache operation wrong fault kind");
        }
    }
}
} // namespace
int main() {
    try {
        faults(0x00000000, InstructionFault::Kind::Undefined);
        faults(0xd4200000, InstructionFault::Kind::Breakpoint);
        faults(0xd4400000, InstructionFault::Kind::Halt);
        faults(0xd69f03e0, InstructionFault::Kind::Privileged);
        faults(0xd51b0023, InstructionFault::Kind::Privileged); // MSR CTR_EL0, X3.
        faults(0xd51b00e3, InstructionFault::Kind::Privileged); // MSR DCZID_EL0, X3.
#if defined(__aarch64__)
        floating_environment();
        floating_dependency_chain();
        floating_interpreter_dispatch();
        unsigned_to_double_dispatch();
        compare_register_dispatch();
#endif
        cache_operations();
        rejects(0x0f400420); // reserved non-scalar 1D immediate shift
        rejects(0x0ee28420); // reserved non-scalar 1D ADD
        rejects(0x0ee0b820); // reserved non-scalar 1D ABS
        rejects(0x6e022c20); // INS halfword, unaligned source imm4
        rejects(0x9ac54c83); // CRC32X is not in the advertised guest profile
        rejects(0xdc000000); // reserved SIMD literal load (opc=3)
        std::puts("cache operations, reserved encodings and unavailable extension checks passed");
        return 0;
    } catch (const std::exception &e) {
        std::fprintf(stderr, "%s\n", e.what());
        return 1;
    }
}
