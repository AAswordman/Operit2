#include "cpu/aarch64/gadgets.h"
#include "acceleration/scalar_ops.h"
#include "cpu/aarch64/interpreter.h"
#include <array>
#include <cstdio>
#include <stdexcept>
#include <unistd.h>

using namespace operit::arm64;
namespace {
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
#if defined(__aarch64__)
bool equal(const CpuState &a, const CpuState &b) {
    return a.x == b.x && a.v == b.v && a.sp == b.sp && a.pc == b.pc && a.nzcv == b.nzcv && a.tls == b.tls &&
           a.fpcr == b.fpcr && a.fpsr == b.fpsr && a.exclusive_valid == b.exclusive_valid &&
           a.exclusive_address == b.exclusive_address && a.exclusive_epoch == b.exclusive_epoch &&
           a.exclusive_size == b.exclusive_size;
}
uint64_t random_word(uint64_t &seed) {
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    return seed;
}
void registers() {
    struct Case {
        uint32_t word;
        bool rn, rm, condition;
    };
    constexpr Case cases[] = {
        {0xd503201f, false, false, false}, {0x10000000, false, false, false},
        {0x90000000, false, false, false}, {0x52824680, false, false, false},
        {0xd2824680, false, false, false}, {0x12824680, false, false, false},
        {0x92824680, false, false, false}, {0x72824680, false, false, false},
        {0xf2824680, false, false, false}, {0x11048c00, true, false, false},
        {0x91048c00, true, false, false},  {0x31048c00, true, false, false},
        {0xb1048c00, true, false, false},  {0x51048c00, true, false, false},
        {0xd1048c00, true, false, false},  {0x71048c00, true, false, false},
        {0xf1048c00, true, false, false},  {0x0b000000, true, true, false},
        {0x8b000000, true, true, false},   {0x2b000000, true, true, false},
        {0xab000000, true, true, false},   {0x4b000000, true, true, false},
        {0xcb000000, true, true, false},   {0x6b000000, true, true, false},
        {0xeb000000, true, true, false},   {0x1a000000, true, true, false},
        {0x9a000000, true, true, false},   {0x3a000000, true, true, false},
        {0xba000000, true, true, false},   {0x5a000000, true, true, false},
        {0xda000000, true, true, false},   {0x7a000000, true, true, false},
        {0xfa000000, true, true, false},   {0x0a000000, true, true, false},
        {0x8a000000, true, true, false},   {0x2a000000, true, true, false},
        {0xaa000000, true, true, false},   {0x4a000000, true, true, false},
        {0xca000000, true, true, false},   {0x6a000000, true, true, false},
        {0xea000000, true, true, false},   {0x0a200000, true, true, false},
        {0x8a200000, true, true, false},   {0x2a200000, true, true, false},
        {0xaa200000, true, true, false},   {0x4a200000, true, true, false},
        {0xca200000, true, true, false},   {0x6a200000, true, true, false},
        {0xea200000, true, true, false},   {0x12001c00, true, false, false},
        {0x92001c00, true, false, false},  {0x32001c00, true, false, false},
        {0xb2001c00, true, false, false},  {0x52001c00, true, false, false},
        {0xd2001c00, true, false, false},  {0x72001c00, true, false, false},
        {0xf2001c00, true, false, false},  {0x1b001c00, true, true, false},
        {0x9b001c00, true, true, false},   {0x1b009c00, true, true, false},
        {0x9b009c00, true, true, false},   {0x1a800000, true, true, true},
        {0x9a800000, true, true, true},    {0x1a800400, true, true, true},
        {0x9a800400, true, true, true},    {0x5a800000, true, true, true},
        {0xda800000, true, true, true},    {0x5a800400, true, true, true},
        {0xda800400, true, true, true},
        // Accumulator X0 aliases inputs/destination; XZR selects MUL/MNEG.
        {0x1b000000, true, true, false},  {0x9b000000, true, true, false},
        {0x1b008000, true, true, false},  {0x9b008000, true, true, false},
        {0x1b007c00, true, true, false},  {0x9b007c00, true, true, false},
        {0x1b00fc00, true, true, false},  {0x9b00fc00, true, true, false},
    };
    Memory memory(page_size);
    memory.set_parallel_access(false);
    GadgetMemory view;
    require(memory.gadget_memory(view), "serial translation view unavailable");
    uint64_t seed = 0x6879737461746963ULL, checked = 0;
    for (const auto &item : cases)
        for (unsigned rd : {0u, 7u, 31u})
            for (unsigned rn : {0u, 7u, 31u})
                for (unsigned rm : {0u, 7u, 31u})
                    for (unsigned flags = 0; flags < 16; ++flags)
                        for (unsigned condition = 0; condition < (item.condition ? 16u : 1u); ++condition) {
                            uint32_t word = item.word;
                            if (word != 0xd503201f)
                                word = (word & ~31u) | rd;
                            if (item.rn)
                                word |= rn << 5;
                            if (item.rm)
                                word |= rm << 16;
                            if (item.condition)
                                word |= condition << 12;
                            const auto instruction = decode(word);
                            std::array<NativeGadget, 2> program{decode_gadget(instruction, 0x10000),
                                                                decode_gadget(decode(0xd503201f), 0x10004)};
                            require(program[0].entry, "register test instruction has no gadget");
                            CpuState expected{};
                            for (auto &value : expected.x)
                                value = random_word(seed);
                            expected.sp = random_word(seed);
                            expected.nzcv = flags;
                            expected.pc = 0x10000;
                            CpuState actual = expected;
                            execute(expected, memory, instruction);
                            execute(expected, memory, decode(0xd503201f));
                            const auto count =
                                operit_arm64_gadgets_run(&actual, &view, program.data(), program.size());
                            actual.pc += count * 4;
                            if (count != 2 || !equal(actual, expected)) {
                                std::fprintf(
                                    stderr,
                                    "register gadget mismatch: %08x NZCV=%u X0=%llx/%llx SP=%llx/%llx\n",
                                    word, flags, (unsigned long long)actual.x[0],
                                    (unsigned long long)expected.x[0], (unsigned long long)actual.sp,
                                    (unsigned long long)expected.sp);
                                throw std::runtime_error("register gadget differs from reference execution");
                            }
                            ++checked;
                        }
    std::printf("%llu register/condition differential cases passed\n", (unsigned long long)checked);
}
// Exercise mode changes, carry dependencies, aliases and all prefix budgets
// inside a single threaded invocation, rather than restarting after each ALU op.
void arithmetic_chains() {
    constexpr auto binary = [](uint32_t word, unsigned rd, unsigned rn, unsigned rm) {
        return word | rd | (rn << 5) | (rm << 16);
    };
    constexpr uint32_t words[] = {
        binary(0xab000000, 0, 1, 2),   // ADDS X0, X1, X2
        binary(0x9a000000, 3, 0, 4),   // ADC X3, X0, X4
        binary(0x7a000000, 5, 3, 6),   // SBCS W5, W3, W6
        binary(0xea000000, 7, 5, 8),   // ANDS X7, X5, X8
        binary(0x9a800000, 9, 0, 3),   // CSEL X9, X0, X3, EQ
        binary(0x9b000000, 10, 9, 11) | (12u << 10), // MADD
        binary(0x1b007c00, 10, 10, 11),             // MUL W10, W10, W11
        binary(0x9b00fc00, 11, 10, 11),             // MNEG X11, X10, X11
        binary(0x9b008000, 0, 0, 11),  // MSUB X0, X0, X11, X0
        0x910007ff,                   // ADD SP, SP, #1
        0xf10007ff,                   // SUBS XZR, SP, #1
        binary(0xba000000, 0, 0, 31),  // ADCS X0, X0, XZR
        binary(0xaa200000, 1, 0, 1),   // ORN X1, X0, X1
        0x72001c3f,                   // ANDS WZR, W1, #255
        binary(0xfa000000, 31, 0, 1),  // SBCS XZR, X0, X1
        binary(0x5a800400, 2, 0, 1),   // CSNEG W2, W0, W1, EQ
        0xd503201f,
    };
    Memory memory(page_size);
    memory.set_parallel_access(false);
    GadgetMemory view;
    require(memory.gadget_memory(view), "arithmetic-chain translation view unavailable");
    std::array<NativeGadget, std::size(words)> program;
    for (size_t index = 0; index < program.size(); ++index) {
        program[index] = decode_gadget(decode(words[index]), 0x10000 + 4 * index);
        require(program[index].entry, "arithmetic-chain instruction has no gadget");
    }
    uint64_t seed = 0x616c75636861696eULL;
    for (unsigned sample = 0; sample < 128; ++sample) {
        CpuState initial{};
        for (auto &value : initial.x)
            value = random_word(seed);
        initial.sp = random_word(seed);
        initial.nzcv = sample & 15;
        initial.pc = 0x10000;
        CpuState expected = initial;
        for (size_t count = 0; count <= program.size(); ++count) {
            CpuState actual = initial;
            actual.gadget_zero = random_word(seed);
            require(operit_arm64_gadgets_run(&actual, &view, program.data(), count) == count,
                    "arithmetic-chain budget mismatch");
            actual.pc += 4 * count;
            require(actual.gadget_zero == 0, "discarded writes modified the zero source slot");
            require(equal(actual, expected), "arithmetic-chain differs from reference execution");
            if (count < program.size())
                execute(expected, memory, decode(words[count]));
        }
    }
}
void memory_operations() {
    constexpr uint32_t words[] = {
        0x39400420, 0x79400420, 0xb9400420, 0xf9400420, 0x39800420, 0x39c00420, 0x79800420, 0x79c00420,
        0xb9800420, 0x39000420, 0x79000420, 0xb9000420, 0xf9000420, 0xf8408420, 0xf8408c20, 0xf8008420,
        0xf8008c20, 0xf8626820, 0xf8226820, 0xb8625820, 0xb8225820, 0xa9400c20, 0xa9000c20, 0x29400c20,
        0x29000c20, 0x69400c20, 0xa8c10c20, 0xa9c10c20, 0xa8810c20, 0xa9810c20,
    };
    for (const auto original : words) {
        for (unsigned destination : {0u, 1u, 3u, 31u})
            for (unsigned base : {1u, 31u})
                for (unsigned second : {0u, 3u, 31u}) {
                    const bool pair = decode(original).op == Op::LoadPair;
                    if (!pair && second == 31)
                        continue;
                    uint32_t word = (original & ~0x3ffu) | (base << 5) | destination;
                    if (pair)
                        word = (word & ~(31u << 10)) | (second << 10);
                    Memory reference(2 * page_size), memory(2 * page_size);
                    reference.set_parallel_access(false);
                    memory.set_parallel_access(false);
                    reference.map(0x20000, page_size, read_perm | write_perm);
                    memory.map(0x20000, page_size, read_perm | write_perm);
                    std::array<uint8_t, 256> initial{};
                    for (unsigned index = 0; index < initial.size(); ++index)
                        initial[index] = index ^ 0xa5;
                    reference.copy_in(0x20000, initial.data(), initial.size());
                    memory.copy_in(0x20000, initial.data(), initial.size());
                    CpuState expected{};
                    expected.pc = 0x10000;
                    expected.x[0] = 0xfedcba9876543210ULL;
                    expected.x[1] = 0x20040;
                    expected.x[2] = 8;
                    expected.x[3] = 0x9988776655443322ULL;
                    expected.sp = 0x20040;
                    expected.nzcv = 15;
                    CpuState actual = expected;
                    const auto instruction = decode(word);
                    const auto location = scalar::address(actual, instruction);
                    (void)memory.load<8>(location.value);
                    const auto reservation = memory.exclusive_epoch(location.value);
                    GadgetMemory view;
                    require(memory.gadget_memory(view), "memory gadget view unavailable");
                    std::array<NativeGadget, 2> program{decode_gadget(instruction, expected.pc),
                                                        decode_gadget(decode(0xd503201f), expected.pc + 4)};
                    require(program[0].entry, "memory test instruction has no gadget");
                    execute(expected, reference, instruction);
                    execute(expected, reference, decode(0xd503201f));
                    const auto count =
                        operit_arm64_gadgets_run(&actual, &view, program.data(), program.size());
                    actual.pc += count * 4;
                    require(count == 2 && equal(actual, expected),
                            "memory gadget register/writeback mismatch");
                    std::array<uint8_t, 256> wanted{}, received{};
                    reference.copy_out(0x20000, wanted.data(), wanted.size());
                    memory.copy_out(0x20000, received.data(), received.size());
                    require(wanted == received, "memory gadget modified wrong bytes");
                    if ((instruction.dispatch >= Dispatch::Store8 &&
                         instruction.dispatch <= Dispatch::Store64) ||
                        instruction.dispatch == Dispatch::StorePair32 ||
                        instruction.dispatch == Dispatch::StorePair64)
                        require(memory.exclusive_epoch(location.value) != reservation,
                                "native store retained an exclusive reservation");
                }
    }
}
void conditional_fallthrough() {
    Memory memory(page_size);
    memory.set_parallel_access(false);
    GadgetMemory view;
    require(memory.gadget_memory(view), "branch view unavailable");
    for (uint32_t raw : {0x54000040u, 0x34000040u, 0x35000040u, 0xb4000040u, 0xb5000040u, 0x36000040u,
                         0x37000040u, 0xb6f80040u, 0xb7f80040u})
        for (unsigned condition = 0; condition < (raw == 0x54000040 ? 16u : 1u); ++condition)
            for (unsigned flags = 0; flags < 16; ++flags)
                for (uint64_t value : {uint64_t{0}, uint64_t{1}, uint64_t{1} << 63, UINT64_MAX}) {
                    const auto instruction = decode(raw | condition);
                    CpuState expected{};
                    expected.pc = 0x10000;
                    expected.x[0] = value;
                    expected.nzcv = flags;
                    CpuState actual = expected;
                    execute(expected, memory, instruction);
                    const bool taken = expected.pc != actual.pc + 4;
                    std::array<NativeGadget, 2> program{decode_gadget(instruction, actual.pc),
                                                        decode_gadget(decode(0xd503201f), actual.pc + 4)};
                    const auto count =
                        operit_arm64_gadgets_run(&actual, &view, program.data(), program.size());
                    require(count == (taken ? 0u : 2u),
                            "branch consumed a taken edge or stopped fallthrough");
                    if (taken)
                        execute(actual, memory, instruction);
                    else {
                        actual.pc += count * 4;
                        execute(expected, memory, decode(0xd503201f));
                    }
                    require(equal(actual, expected), "branch gadget modified architectural state");
                }
}
void interpreter_boundaries() {
    Memory reference(2 * page_size), memory(2 * page_size);
    reference.set_parallel_access(false);
    memory.set_parallel_access(false);
    constexpr uint32_t words[] = {0x91000400, 0x91000821, 0xf9000040, 0xf9400043,
                                  0xf1000c64, 0x9a81c005, 0xba0100c6, 0x17fffff9};
    for (auto *target : {&reference, &memory}) {
        target->map(0x10000, page_size, read_perm | write_perm | exec_perm);
        target->map(0x20000, page_size, read_perm | write_perm);
        target->copy_in(0x10000, words, sizeof(words));
    }
    Interpreter ordinary, native;
    ordinary.use_native_gadgets = false;
    CpuState expected{}, actual{};
    expected.pc = actual.pc = 0x10000;
    expected.x[2] = actual.x[2] = 0x20000;
    for (const uint64_t budget : {1, 2, 3, 5, 7, 11, 4096}) {
        const auto first = ordinary.run(expected, reference, budget);
        const auto second = native.run(actual, memory, budget);
        require(first.trap == second.trap && first.instructions == second.instructions &&
                    equal(actual, expected),
                "gadget budget/branch boundary mismatch");
        require(reference.load<8>(0x20000) == memory.load<8>(0x20000), "gadget loop bytes mismatch");
    }
    require(native.native_gadget_instructions > 100, "threaded spans did not run");
    const auto accelerated = native.native_gadget_instructions;
    memory.set_parallel_access(true);
    ordinary.run(expected, reference, 100);
    native.run(actual, memory, 100);
    require(equal(actual, expected) && native.native_gadget_instructions == accelerated,
            "parallel execution entered serial gadgets");
}
void memory_fallbacks() {
    const auto declines = [](Memory &memory, uint32_t word, uint64_t address) {
        GadgetMemory view;
        require(memory.gadget_memory(view), "fallback view unavailable");
        CpuState before{};
        before.pc = 0x10000;
        before.x[0] = 0x123456789abcdef0ULL;
        before.x[1] = address;
        before.nzcv = 11;
        CpuState after = before;
        std::array<NativeGadget, 2> program{decode_gadget(decode(word), before.pc),
                                            decode_gadget(decode(0xd503201f), before.pc + 4)};
        require(operit_arm64_gadgets_run(&after, &view, program.data(), 2) == 0 && equal(before, after),
                "declined memory access changed registers or retired an instruction");
    };
    constexpr uint32_t load = 0xf9400020, store = 0xf9000020;
    for (unsigned scenario = 0; scenario < 7; ++scenario) {
        Memory memory(3 * page_size);
        memory.set_parallel_access(false);
        memory.map(0x20000, 2 * page_size, read_perm | write_perm, false, scenario == 0);
        if (scenario != 1)
            memory.store<8>(0x20000, 17);
        switch (scenario) {
        case 0: // Shared storage uses the locking C++ path, for both loads and stores.
            declines(memory, load, 0x20000);
            declines(memory, store, 0x20000);
            break;
        case 1:
            declines(memory, load, 0x20000);
            break; // Nonresident page.
        case 2:    // Never perform part of a cross-page load/store/pair.
            declines(memory, load, 0x20fff);
            declines(memory, store, 0x20fff);
            declines(memory, 0xa9000c20, 0x20ff8);
            break;
        case 3:
            memory.protect(0x20000, page_size, read_perm | write_perm | exec_perm);
            (void)memory.load<8>(0x20000);
            declines(memory, store, 0x20000);
            break;
        case 4:
            memory.protect(0x20000, page_size, read_perm);
            (void)memory.load<8>(0x20000);
            declines(memory, store, 0x20000);
            break;
        case 5:
            declines(memory, load, uint64_t{1} << 48);
            break;
        case 6: // A mapping epoch change makes a previously resident entry stale.
            memory.protect(0x20000, page_size, read_perm);
            declines(memory, load, 0x20000);
            break;
        }
        if (scenario != 1)
            require(memory.load<8>(0x20000) == 17, "declined store changed bytes");
    }
    {
        Memory parent(page_size);
        parent.set_parallel_access(false);
        parent.map(0x20000, page_size, read_perm | write_perm);
        parent.store<8>(0x20000, 17);
        auto child = parent.fork_clone();
        (void)parent.load<8>(0x20000);
        declines(parent, store, 0x20000);
        require(parent.load<8>(0x20000) == 17 && child->load<8>(0x20000) == 17,
                "native store bypassed fork COW");
        parent.store<8>(0x20000, 29);
        require(child->load<8>(0x20000) == 17, "fallback failed to preserve fork COW");
    }
    char name[] = "/tmp/operit-gadget-file-XXXXXX";
    const int descriptor = mkstemp(name);
    require(descriptor >= 0, "create file-backed gadget fixture");
    unlink(name);
    struct Close {
        int fd;
        ~Close() {
            close(fd);
        }
    } close_file{descriptor};
    require(ftruncate(descriptor, page_size) == 0, "size file-backed gadget fixture");
    for (bool shared : {false, true}) {
        Memory memory(page_size);
        memory.set_parallel_access(false);
        Memory::Backing backing;
        backing.shared = shared;
        memory.map_file(0x20000, page_size, read_perm | write_perm, descriptor, backing);
        (void)memory.load<8>(0x20000);
        declines(memory, load, 0x20000);
        declines(memory, store, 0x20000);
        memory.store<8>(0x20000, 17); // Also test an already privately copied file page.
        declines(memory, load, 0x20000);
        declines(memory, store, 0x20000);
        require(memory.load<8>(0x20000) == 17, "declined file store changed bytes");
    }
}
void owned_file_pages() {
    char name[] = "/tmp/operit-owned-gadget-XXXXXX";
    const int fd = mkstemp(name);
    require(fd >= 0, "create private file fixture");
    unlink(name);
    struct Close { int fd; ~Close() { close(fd); } } close_file{fd};
    require(ftruncate(fd, page_size) == 0, "size private file fixture");
    Memory memory(page_size);
    memory.set_parallel_access(false);
    Memory::Backing backing;
    memory.map_file(0x20000, page_size, read_perm | write_perm, fd, backing);
    memory.store<8>(0x20000, 17); // Resolve file COW before native access.
    const auto program = [](uint32_t word) {
        return std::array<NativeGadget, 2>{decode_gadget(decode(word), 0x10000),
                                          decode_gadget(decode(0xd503201f), 0x10004)};
    };
    auto load = program(0xf9400020), store = program(0xf9000020);
    CpuState cpu{};
    cpu.x[1] = 0x20000;
    for (unsigned iteration = 0; iteration < 2; ++iteration) {
        Memory::ExecutionScope scope(memory);
        GadgetMemory view;
        require(memory.gadget_memory(view), "private file view unavailable");
        require(operit_arm64_gadgets_run(&cpu, &view, load.data(), 2) == 0,
                "native private file load skipped execution-scope validation");
        require(memory.load<8>(0x20000) == (iteration ? 29 : 17), "private file value changed");
        require(memory.gadget_memory(view), "validated private file view unavailable");
        require(operit_arm64_gadgets_run(&cpu, &view, load.data(), 2) == 2 &&
                    cpu.x[0] == (iteration ? 29 : 17), "owned file load did not use direct access");
        cpu.x[0] = 29;
        require(operit_arm64_gadgets_run(&cpu, &view, store.data(), 2) == 2,
                "owned file store did not use direct access");
    }
    auto child = memory.fork_clone();
    {
        Memory::ExecutionScope scope(memory);
        (void)memory.load<8>(0x20000);
        GadgetMemory view;
        require(memory.gadget_memory(view), "forked file view unavailable");
        cpu.x[0] = 31;
        require(operit_arm64_gadgets_run(&cpu, &view, store.data(), 2) == 0,
                "owned file store bypassed fork COW");
        memory.store<8>(0x20000, 31);
        require(child->load<8>(0x20000) == 29, "owned file COW changed child storage");
    }
    require(ftruncate(fd, 0) == 0, "truncate private file fixture");
    {
        Memory::ExecutionScope scope(memory);
        GadgetMemory view;
        require(memory.gadget_memory(view), "truncated file view unavailable");
        require(operit_arm64_gadgets_run(&cpu, &view, load.data(), 2) == 0,
                "owned file load bypassed fresh truncation check");
        try {
            (void)memory.load<8>(0x20000);
            throw std::runtime_error("owned file fallback missed truncation");
        } catch (const MemoryFault &fault) {
            require(fault.kind == MemoryFault::Kind::Bus, "wrong owned file truncation fault");
        }
    }
}
void fault_retirement() {
    for (bool protection : {false, true}) {
        Memory memory(2 * page_size);
        memory.set_parallel_access(false);
        memory.map(0x10000, page_size, read_perm | write_perm | exec_perm);
        const uint32_t program[] = {0x91000400, 0xf8008420, 0xd503201f}; // ADD; STR post-index; NOP.
        memory.copy_in(0x10000, program, sizeof(program));
        if (protection) {
            memory.map(0x20000, page_size, read_perm | write_perm);
            memory.store<8>(0x20000, 23);
            memory.protect(0x20000, page_size, read_perm);
            (void)memory.load<8>(0x20000);
        }
        CpuState cpu{};
        cpu.pc = 0x10000;
        cpu.x[1] = 0x20000;
        Interpreter interpreter;
        try {
            interpreter.run(cpu, memory, 3);
            throw std::runtime_error("gadget failed to defer a memory fault");
        } catch (const MemoryFault &fault) {
            require(fault.kind == (protection ? MemoryFault::Kind::Protection : MemoryFault::Kind::Unmapped),
                    "gadget fallback returned wrong fault kind");
            require(cpu.pc == 0x10004 && cpu.x[0] == 1 && cpu.x[1] == 0x20000 &&
                        interpreter.executed_instructions == 1 && interpreter.native_gadget_instructions == 1,
                    "gadget fault charged/writeback occurred before the fault");
        }
    }
}
#endif
} // namespace
int main() {
    try {
#if defined(__aarch64__)
        registers();
        arithmetic_chains();
        memory_operations();
        conditional_fallthrough();
        interpreter_boundaries();
        fault_retirement();
        memory_fallbacks();
        owned_file_pages();
#endif
        std::puts("static gadget differential, memory, budget and fault checks passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
}
