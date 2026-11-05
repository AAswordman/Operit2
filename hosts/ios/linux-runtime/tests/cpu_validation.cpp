#include "cpu/aarch64/cache.h"
#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/instruction_fault.h"
#include "memory/fault.h"
#include "cpu/aarch64/interpreter.h"
#include <cstring>
#include <cstdio>
#include <stdexcept>
#include <unistd.h>
#include <sys/mman.h>
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
    for (unsigned host_rounding = 0; host_rounding < 4; ++host_rounding) {
        for (unsigned guest_rounding = 0; guest_rounding < 4; ++guest_rounding) {
            for (const uint64_t host_status : {uint64_t{0}, uint64_t{2}, uint64_t{0x9f},
                                               uint64_t{1} << 27}) {
                check(uint64_t{host_rounding} << 22, host_status,
                      uint64_t{guest_rounding} << 22, false);
                check(uint64_t{host_rounding} << 22, host_status,
                      uint64_t{guest_rounding} << 22, true);
            }
        }
    }
}
void floating_scalar_bits() {
    Memory memory(page_size);
    constexpr uint64_t inputs[] = {0, 0x8000000000000000ull, 0x7ff0000000000001ull,
                                   0x7ff80000ffc17c01ull, 0xfff000007f800001ull,
                                   0x0000000000000001ull, 0x0123456789abcdefull};
    for (unsigned type : {0u, 1u, 3u}) {
        if (type == 3 && !host_supports_half_floating_point())
            continue;
        const unsigned bytes = type == 3 ? 2 : type == 1 ? 8 : 4;
        for (unsigned rd : {0u, 1u, 31u}) {
            for (const auto bits : inputs) {
                CpuState cpu{};
                for (auto &v : cpu.v)
                    v.fill(0xa5);
                std::memcpy(cpu.v[1].data(), &bits, sizeof(bits));
                cpu.fpsr = 0x10;
                cpu.fpcr = (uint64_t{1} << 25) | (uint64_t{1} << 24);
                const auto before = cpu.v;
                auto expected = before;
                expected[rd].fill(0);
                std::memcpy(expected[rd].data(), &bits, bytes);
                execute(cpu, memory, decode(0x1e204020 | (type << 22) | rd)); // FMOV Rd, R1.
                require(cpu.v == expected, "scalar FMOV changed raw bits or upper lanes");
                require(cpu.fpsr == 0x10, "scalar FMOV raised a floating exception");
            }
        }
    }
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
void code_cache_revalidation() {
    constexpr uint64_t first = 0x10000, second = first + 16, other = 0x1011000;
    Memory memory(3 * page_size);
    memory.set_parallel_access(false);
    memory.map(first, page_size, read_perm | write_perm | exec_perm);
    memory.map(other, page_size, read_perm | write_perm | exec_perm);
    const auto put = [&](uint64_t at, unsigned value) {
        memory.store<4>(at, 0xd2800000 | (value << 5)); // MOV X0, #value.
        memory.store<4>(at + 4, 0xd65f03c0); // RET.
    };
    put(first, 7);
    put(second, 9);
    put(other, 13);
    Interpreter interpreter;
    const auto run = [&](uint64_t at, unsigned value) {
        CpuState cpu{};
        cpu.pc = at;
        require(interpreter.run(cpu, memory, 1).instructions == 1 && cpu.x[0] == value,
                "code page cache executed stale bytes");
    };
    run(first, 7);
    run(second, 9);
    const auto decoded = interpreter.decoded_blocks;
    memory.store<4>(first + 128, 0xd503201f);
    run(first, 7);
    run(second, 9);
    require(interpreter.decoded_blocks == decoded, "unmodified blocks were decoded again");
    put(first, 11);
    run(second, 9); // Refreshing another block must not certify the old first block.
    run(first, 11);
    put(second, 17);
    run(first, 11);
    run(second, 17);
    run(other, 13);
    put(first, 19);
    run(first, 19);
    run(other, 13);
    auto child = memory.fork_clone();
    child->store<4>(first, 0xd28002e0); // MOV X0, #23.
    run(first, 19);
    CpuState cpu{};
    cpu.pc = first;
    interpreter.run(cpu, *child, 1);
    require(cpu.x[0] == 23, "code cache leaked across address spaces");
    run(first, 19);
}


void code_cache_full_block() {
    constexpr uint64_t address = 0x1003c; // Exercise all 64 instructions, including the block tail.
    Memory memory(page_size);
    memory.set_parallel_access(false);
    memory.map(align_down(address), page_size, read_perm | write_perm | exec_perm);
    for (size_t index = 0; index < 64; ++index)
        memory.store<4>(address + index * 4, 0x91000400); // ADD X0, X0, #1.
    Interpreter interpreter;
    const auto run = [&](uint64_t expected) {
        CpuState cpu{};
        cpu.pc = address;
        const auto result = interpreter.run(cpu, memory, 64);
        require(result.instructions == 64 && cpu.x[0] == expected && cpu.pc == address + 256,
                "full-size block validation or retirement incorrect");
    };
    run(64);
    memory.store<4>(address + 512, 0xd503201f);
    run(64); // Validate after an unrelated write without changing this block.
    const auto decoded = interpreter.decoded_blocks;
    memory.store<4>(address + 512, 0xd503201f);
    run(64);
    require(interpreter.decoded_blocks == decoded, "unchanged full-size block decoded again");
    memory.store<4>(address + 63 * 4, 0x91000800); // ADD X0, X0, #2 in the final instruction.
    run(65);
    memory.set_parallel_access(true);
    memory.store<4>(address, 0x91000c00); // Parallel validation keeps its locked path.
    run(67);
    memory.set_parallel_access(false);
    memory.store<4>(address + 63 * 4, 0x91001000);
    run(69);
}

void file_code_revalidation() {
    char name[] = "/tmp/operit-code-page-XXXXXX";
    const int fd = mkstemp(name);
    require(fd >= 0, "create code page file fixture");
    unlink(name);
    struct Close { int fd; ~Close() { close(fd); } } close_file{fd};
    require(ftruncate(fd, page_size) == 0, "size code page file fixture");
    constexpr uint64_t address = 0x20040;
    uint32_t words[] = {0xd28000e0, 0xd65f03c0}; // MOV X0, #7; RET.
    require(pwrite(fd, words, sizeof(words), address % page_size) == sizeof(words), "write code page fixture");
    Memory memory(page_size);
    memory.set_parallel_access(false);
    Memory::Backing backing;
    backing.shared = true;
    memory.map_file(align_down(address), page_size, read_perm | exec_perm, fd, backing);
    Interpreter interpreter;
    const auto run = [&] {
        CpuState cpu{};
        cpu.pc = address;
        interpreter.run(cpu, memory, 1);
        return cpu.x[0];
    };
    require(run() == 7, "initial file code page incorrect");
    words[0] = 0xd2800160; // MOV X0, #11.
    require(pwrite(fd, words, sizeof(words), address % page_size) == sizeof(words), "replace file code");
    memory.synchronize_shared();
    require(run() == 11, "external file code modification was missed");
    {
        // No msync: vnode write notifications alone cannot replace byte validation.
        void *mapping = mmap(nullptr, page_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        require(mapping != MAP_FAILED, "map external code writer");
        struct Unmap { void *data; ~Unmap() { munmap(data, page_size); } } unmap{mapping};
        const uint32_t instruction = 0xd28001a0; // MOV X0, #13.
        std::memcpy(static_cast<uint8_t *>(mapping) + address % page_size,
                    &instruction, sizeof(instruction));
        memory.synchronize_shared();
        require(run() == 13, "unflushed shared-mapping code modification was missed");
    }
    memory.protect(align_down(address), page_size, read_perm);
    try {
        run();
        throw std::runtime_error("code page cache bypassed execute protection");
    } catch (const MemoryFault &fault) {
        require(fault.kind == MemoryFault::Kind::Protection && fault.address == address,
                "cached code reported wrong protection fault");
    }
    memory.protect(align_down(address), page_size, read_perm | exec_perm);
    require(run() == 13, "restored code mapping incorrect");
    require(ftruncate(fd, 0) == 0, "truncate code page fixture");
    memory.synchronize_shared();
    try {
        run();
        throw std::runtime_error("code page cache bypassed file truncation");
    } catch (const MemoryFault &fault) {
        require(fault.kind == MemoryFault::Kind::Bus && fault.address == address,
                "cached code reported wrong truncation fault");
    }
}
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
        floating_scalar_bits();
        floating_dependency_chain();
        floating_interpreter_dispatch();
        unsigned_to_double_dispatch();
        compare_register_dispatch();
#endif
        code_cache_revalidation();
        code_cache_full_block();
        file_code_revalidation();
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
