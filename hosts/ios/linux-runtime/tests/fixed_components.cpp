#include "acceleration/fixed_components.h"
#include "base/sha256.h"
#include "cpu/aarch64/interpreter.h"
#include "elf/loader.h"
#include "memory/fault.h"
#include <cstdio>
#include <chrono>
#include <future>
#include <vector>
#include <stdexcept>
#include <unistd.h>

using namespace operit::arm64;
static void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
static const FixedSymbols &fixture_symbols(Filesystem &filesystem) {
    const auto elf = filesystem.read_file("/lib/ld-musl-aarch64.so.1", 32 << 20);
    const auto digest = sha256(elf.data(), elf.size());
    for (const auto &manifest : fixed_musl_manifests)
        if (manifest.sha256 == digest)
            return manifest.symbols;
    throw std::runtime_error("fixture must be a pinned musl ELF");
}
static void require_equal(const CpuState &left, const CpuState &right) {
    require(left.x == right.x && left.v == right.v && left.pc == right.pc && left.sp == right.sp &&
                left.tls == right.tls && left.nzcv == right.nzcv && left.fpcr == right.fpcr &&
                left.fpsr == right.fpsr && left.exclusive_address == right.exclusive_address &&
                left.exclusive_epoch == right.exclusive_epoch && left.exclusive_size == right.exclusive_size &&
                left.exclusive_valid == right.exclusive_valid,
            "copy helper changed architectural state");
}
static void copy_pipeline_fallbacks() {
    constexpr uint64_t private_page = 0x10000, shared_page = 0x11000;
    constexpr uint64_t executable_page = 0x12000, file_page = 0x13000;
    Memory memory(8 * page_size);
    memory.map(private_page, page_size, read_perm | write_perm);
    memory.map(shared_page, page_size, read_perm | write_perm, false, true);
    memory.map(executable_page, page_size, read_perm | write_perm | exec_perm);
    std::unique_ptr<FILE, decltype(&std::fclose)> file(std::tmpfile(), &std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()), page_size), "create file-backed copy fixture");
    memory.map_file(file_page, page_size, read_perm, fileno(file.get()), {"/copy-test"});
    std::array<uint8_t, 128> data{};
    data.fill(0x5a);
    memory.copy_in(private_page, data.data(), data.size());
    memory.copy_in(shared_page, data.data(), data.size());
    memory.copy_in(executable_page, data.data(), data.size());
    memory.copy_out(file_page, data.data(), data.size());
    const auto decline = [&](uint64_t destination, uint64_t source) {
        std::array<uint8_t, 64> before{}, after{}, registers{};
        memory.copy_out(destination, before.data(), before.size());
        {
            Memory::PreparedAccess prepared(memory);
            require(prepared.add(destination, 64, write_perm) && prepared.add(source, 64, read_perm) &&
                        prepared.lock(),
                    "fallback fixture did not prepare");
            require(!prepared.copy_pipeline(destination, source, registers.data(), registers.data(), 64),
                    "copy pipeline accepted unsafe pages");
        }
        memory.copy_out(destination, after.data(), after.size());
        require(before == after, "declined copy pipeline changed destination");
    };
    decline(private_page + 64, private_page); // Same physical page.
    decline(shared_page, private_page);
    decline(private_page, shared_page);
    decline(executable_page, private_page);
    decline(private_page, file_page);
}
static void helper_budget_differential(const char *root) {
    constexpr uint64_t start = 0x10000, size = 8 * page_size;
    constexpr uint64_t lengths[] = {0, 1, 2, 3, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65,
                                    127, 128, 129, 159, 160, 161, 255, 256, 257, 511,
                                    1024, 1264, 4096};
    constexpr uint64_t budgets[] = {1, 2, 3, 5, 7, 9, 10, 11, 17, 31, 64, 127};
    Filesystem filesystem(root);
    const auto &symbols = fixture_symbols(filesystem);
    Memory actual_memory(32 << 20), reference_memory(32 << 20);
    FixedComponents fixed(true), disabled(false);
    ElfLoader actual_loader(actual_memory, filesystem, 32 << 20, fixed);
    ElfLoader reference_loader(reference_memory, filesystem, 32 << 20, disabled);
    const auto image = actual_loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
    reference_loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
    for (auto *memory : {&actual_memory, &reference_memory}) {
        memory->map(start, size, read_perm | write_perm);
        memory->map(0x4000, page_size, read_perm | write_perm | exec_perm);
        memory->store<4>(0x4000, 0x14000000); // b .
    }
    Interpreter accelerated, ordinary;
    std::vector<uint8_t> initial(size), actual_bytes(size), expected_bytes(size);
    for (size_t index = 0; index < initial.size(); ++index)
        initial[index] = uint8_t(index % 251 + 1);
    for (size_t variant = 0; variant < 8; ++variant) {
        const size_t symbol = variant == 0 ? 0 : variant <= 4 ? 1 : variant == 6 ? 3 : 2;
        const uint64_t calls_before = fixed.calls;
        for (const uint64_t length : lengths) {
            auto data = initial;
            CpuState state{};
            for (size_t index = 0; index < state.x.size(); ++index)
                state.x[index] = 0x123456789abcdef0ull + index;
            state.pc = image.base + symbols[symbol].value;
            state.x[0] = 0x12007;
            state.x[1] = 0x10003;
            state.x[2] = length;
            state.x[30] = 0x4000;
            if (variant == 1)
                state.x[0] = state.x[1];
            else if (variant == 2)
                state.x[0] = state.x[1] + 5;
            else if (variant == 3)
                state.x[1] = state.x[0] + 5;
            else if (variant == 5)
                state.x[1] = 0x1ff;
            else if (variant == 6) {
                state.x[0] = start + size - length - 1;
                state.x[1] = 0;
                state.x[2] = 0;
                data[size - 1] = 0;
            } else if (variant == 7) {
                state.x[0] = 0x12010;
                state.x[1] = 0;
            }
            for (unsigned mode = 0; mode < 2; ++mode) {
                actual_memory.copy_in(start, data.data(), data.size());
                reference_memory.copy_in(start, data.data(), data.size());
                CpuState actual = state, reference = state;
                bool complete = false;
                for (size_t slice = 0; slice < 4096; ++slice) {
                    const uint64_t budget = mode ? 8192 : budgets[(slice + length) % std::size(budgets)];
                    const auto a = accelerated.run(actual, actual_memory, budget, &fixed);
                    const auto b = ordinary.run(reference, reference_memory, budget);
                    try {
                        require(a.instructions == b.instructions && a.trap == b.trap,
                                "different retirement or trap");
                        require_equal(actual, reference);
                        actual_memory.copy_out(start, actual_bytes.data(), actual_bytes.size());
                        reference_memory.copy_out(start, expected_bytes.data(), expected_bytes.size());
                        require(actual_bytes == expected_bytes, "different memory");
                    } catch (const std::exception &error) {
                        std::fprintf(stderr, "helper variant=%zu length=%llu mode=%u slice=%zu budget=%llu "
                                             "actual_pc=%llx reference_pc=%llx: %s\n",
                                     variant, static_cast<unsigned long long>(length), mode, slice,
                                     static_cast<unsigned long long>(budget),
                                     static_cast<unsigned long long>(actual.pc),
                                     static_cast<unsigned long long>(reference.pc), error.what());
                        throw;
                    }
                    if (actual.pc == 0x4000) {
                        complete = true;
                        break;
                    }
                }
                require(complete, "budgeted helper did not return");
            }
        }
        require(fixed.calls > calls_before, "budgeted helper variant never accelerated");
    }
}
static void resumed_copy(const char *root) {
    struct Case {
        uint64_t source, destination, length;
        bool parallel;
    };
    constexpr uint64_t start = 0x10000, size = 1 << 20;
    const Case cases[] = {{start + 0x100, start + 0x80100, 262144, false},
                          {start + 0xffa, start + 0x80ff9, 131101, false},
                          {start + 0x200, start + 0x801ff, 131072, true},
                          {start + 0x200, start + 0x241, 131072, true}};
    std::vector<uint8_t> initial(size);
    for (size_t index = 0; index < initial.size(); ++index)
        initial[index] = uint8_t(index * 37 + index / 127);
    for (const auto &test : cases) {
        Filesystem filesystem(root);
        const auto &symbols = fixture_symbols(filesystem);
        Memory accelerated_memory(32 << 20), reference_memory(32 << 20);
        FixedComponents fixed(true), disabled(false);
        ElfLoader accelerated_loader(accelerated_memory, filesystem, 32 << 20, fixed);
        ElfLoader reference_loader(reference_memory, filesystem, 32 << 20, disabled);
        const auto image = accelerated_loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
        reference_loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
        for (auto *memory : {&accelerated_memory, &reference_memory}) {
            memory->map(start, size, read_perm | write_perm);
            memory->copy_in(start, initial.data(), initial.size());
            memory->map(0x4000, page_size, read_perm | write_perm | exec_perm);
            memory->store<4>(0x4000, 0x14000000); // b .
            memory->set_parallel_access(test.parallel);
        }
        CpuState actual{}, reference{};
        actual.pc = image.base + symbols[0].value;
        actual.x[0] = test.destination;
        actual.x[1] = test.source;
        actual.x[2] = test.length;
        actual.x[30] = 0x4000;
        reference = actual;
        Interpreter accelerated, ordinary;
        constexpr uint64_t budgets[] = {1, 7, 9, 10, 11, 31, 997, 4096};
        std::vector<uint8_t> actual_bytes(size), expected_bytes(size);
        bool complete = false;
        for (size_t slice = 0; slice < 4096; ++slice) {
            const uint64_t budget = budgets[slice % std::size(budgets)];
            const auto a = accelerated.run(actual, accelerated_memory, budget, &fixed);
            const auto b = ordinary.run(reference, reference_memory, budget);
            require(a.instructions == b.instructions && a.trap == b.trap,
                    "copy helper retired a different instruction count");
            require_equal(actual, reference);
            accelerated_memory.copy_out(start, actual_bytes.data(), actual_bytes.size());
            reference_memory.copy_out(start, expected_bytes.data(), expected_bytes.size());
            require(actual_bytes == expected_bytes, "copy helper changed memory at a budget boundary");
            if (actual.pc == 0x4000) {
                complete = true;
                break;
            }
        }
        require(complete, "resumed memcpy did not return");
        require(fixed.calls > 1, "resumed memcpy loop was never accelerated");
    }
}
static void resumed_copy_fault(const char *root) {
    Filesystem filesystem(root);
    const auto &symbols = fixture_symbols(filesystem);
    Memory actual_memory(32 << 20), reference_memory(32 << 20);
    FixedComponents fixed(true), disabled(false);
    ElfLoader actual_loader(actual_memory, filesystem, 32 << 20, fixed);
    ElfLoader reference_loader(reference_memory, filesystem, 32 << 20, disabled);
    const auto image = actual_loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
    reference_loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
    for (auto *memory : {&actual_memory, &reference_memory}) {
        memory->map(0x10000, 4 * page_size, read_perm | write_perm);
        std::array<uint8_t, 64> data{};
        data.fill(0x65);
        memory->copy_in<64>(0x10000, data.data());
        memory->copy_in<64>(0x12000, data.data());
    }
    CpuState state{};
    state.pc = image.base + symbols[0].value + fixed_copy_loop_offset;
    state.x[1] = 0x10000 - 16;
    state.x[2] = 4096;
    state.x[3] = 0x12000 - 16;
    state.x[6] = 0x12345678;
    CpuState warm = state;
    require(fixed.execute(warm, actual_memory, 10) == 10, "resident copy loop declined");
    for (auto *memory : {&actual_memory, &reference_memory}) {
        std::array<uint8_t, 64> data{};
        data.fill(0x65);
        memory->copy_in<64>(0x12000, data.data());
        memory->protect(0x10000, page_size, 0);
    }
    CpuState actual = state, reference = state;
    require(!fixed.execute(actual, actual_memory, 10), "copy helper bypassed source protection");
    require_equal(actual, state);
    Interpreter accelerated, ordinary;
    auto fault = [](Interpreter &interpreter, CpuState &cpu, Memory &memory, FixedComponents *helpers) {
        try {
            interpreter.run(cpu, memory, 10, helpers);
        } catch (const MemoryFault &error) {
            return std::tuple{error.address, error.permissions, error.kind};
        }
        throw std::runtime_error("protected copy unexpectedly succeeded");
    };
    require(fault(accelerated, actual, actual_memory, &fixed) ==
                fault(ordinary, reference, reference_memory, nullptr),
            "copy helper changed memory fault details");
    require_equal(actual, reference);
    require(accelerated.executed_instructions == ordinary.executed_instructions,
            "copy helper changed retirement before a fault");
    std::array<uint8_t, 64> actual_bytes{}, expected_bytes{};
    actual_memory.copy_out<64>(0x12000, actual_bytes.data());
    reference_memory.copy_out<64>(0x12000, expected_bytes.data());
    require(actual_bytes == expected_bytes, "copy helper changed writes before a fault");

    actual_memory.protect(0x10000, page_size, read_perm | write_perm);
    warm = state;
    require(fixed.execute(warm, actual_memory, 10) == 10, "restored copy loop declined");
    const uint64_t code = state.pc;
    actual_memory.protect(align_down(code), page_size, read_perm | write_perm | exec_perm);
    const auto original = actual_memory.load<4>(code);
    actual_memory.store<4>(code, original ^ 1);
    actual = state;
    require(!fixed.execute(actual, actual_memory, 10), "modified copy loop retained acceleration");
    require_equal(actual, state);
}
static void parallel_helpers(const char *root) {
    Filesystem filesystem(root);
    const auto &symbols = fixture_symbols(filesystem);
    Memory memory(32 << 20);
    FixedComponents source(true);
    ElfLoader loader(memory, filesystem, 32 << 20, source);
    const auto image = loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
    memory.map(0x10000, 2 * page_size, read_perm | write_perm);
    std::array<uint8_t, 32> bytes;
    bytes.fill(0x5a);
    memory.copy_in<32>(0x10000, bytes.data());
    memory.copy_in<32>(0x11000, bytes.data());
    auto copy = [&](FixedComponents &fixed, uint64_t destination, uint64_t input) {
        CpuState cpu;
        cpu.pc = image.base + symbols[0].value;
        cpu.x[0] = destination;
        cpu.x[1] = input;
        cpu.x[2] = bytes.size();
        cpu.x[30] = 0x4000;
        require(fixed.execute(cpu, memory) && cpu.pc == 0x4000, "parallel memcpy did not finish");
    };
    copy(source, 0x11000, 0x10000); // Fault in authenticated code before the progress test.
    // Cross-page fixed accesses use the original raw-span fallback. They must
    // never attempt to upgrade a retained reader gate to the writer gate.
    std::array<uint8_t, 32> crossing;
    crossing.fill(0x37);
    memory.copy_in<32>(0x10ff0, crossing.data());
    copy(source, 0x10000, 0x10ff0);
    std::array<uint8_t, 32> copied;
    memory.copy_out<32>(0x10000, copied.data());
    require(copied == crossing, "cross-page helper fallback corrupted bytes");
    copy(source, 0x10ff0, 0x10000);
    memory.copy_out<32>(0x10ff0, copied.data());
    require(copied == crossing, "cross-page helper destination fallback corrupted bytes");
    memory.copy_in<32>(0x10000, bytes.data());
    memory.copy_in<32>(0x11000, bytes.data());
    std::future<void> hot;
    bool progressed = false;
    {
        memory_detail::AccessSynchronization::ReadGuard peer(true);
        hot = std::async(std::launch::async, [&] {
            FixedComponents fixed(source);
            copy(fixed, 0x11000, 0x10000);
        });
        progressed = hot.wait_for(std::chrono::seconds(2)) == std::future_status::ready;
    }
    hot.get();
    require(progressed, "hot memcpy still waited for a global memory writer gate");
    std::vector<std::future<void>> jobs;
    for (unsigned id = 0; id < 4; ++id) {
        jobs.push_back(std::async(std::launch::async, [&, id] {
            FixedComponents fixed(source);
            // Reverse source/destination order on alternating workers. A fixed
            // physical page order must prevent an AB/BA locking deadlock.
            for (unsigned index = 0; index < 1000; ++index)
                copy(fixed, id & 1 ? 0x10000 : 0x11000, id & 1 ? 0x11000 : 0x10000);
        }));
    }
    for (auto &job : jobs)
        job.get();
    jobs.clear();
    for (unsigned id = 0; id < 4; ++id) {
        jobs.push_back(std::async(std::launch::async, [&, id] {
            FixedComponents fixed(source);
            for (unsigned index = 0; index < 1000; ++index) {
                CpuState cpu{};
                const uint64_t input = id & 1 ? 0x11000 : 0x10000;
                const uint64_t output = id & 1 ? 0x10000 : 0x11000;
                cpu.pc = image.base + symbols[0].value + fixed_copy_loop_offset;
                cpu.x[1] = input - 16;
                cpu.x[2] = 4096;
                cpu.x[3] = output - 16;
                for (unsigned reg = 6; reg <= 13; ++reg)
                    cpu.x[reg] = 0x5a5a5a5a5a5a5a5aull;
                require(fixed.execute(cpu, memory, 10) == 10, "parallel resumed memcpy declined");
            }
        }));
    }
    for (auto &job : jobs)
        job.get();
    std::array<uint8_t, 32> output;
    memory.copy_out<32>(0x10000, output.data());
    require(output == bytes, "parallel helper copy corrupted source bytes");
    memory.copy_out<32>(0x11000, output.data());
    require(output == bytes, "parallel helper copy corrupted destination bytes");
}

int main(int argc, char **argv) {
    try {
        require(argc == 2, "expected root directory");
        Filesystem filesystem(argv[1]);
        const auto &symbols = fixture_symbols(filesystem);
        Memory memory(32 << 20);
        FixedComponents fixed(true), disabled(false);
        ElfLoader loader(memory, filesystem, 32 << 20, fixed);
        const auto image = loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
        auto elf = filesystem.read_file("/lib/ld-musl-aarch64.so.1", 32 << 20);
        require(fixed.recognizes(elf), "fixture must be the pinned musl ELF");
        require(!disabled.recognizes(elf), "disabled registry accepted ELF");
        bool built_snapshot = false;
        auto snapshots = [&] {
            built_snapshot = true;
            return std::vector<ExecutableSnapshot>{};
        };
        disabled.register_image_lazy(elf, image.base, snapshots);
        require(!built_snapshot, "disabled registry built snapshots");
        elf.push_back(0);
        require(!fixed.recognizes(elf), "changed ELF length accepted");
        fixed.register_image_lazy(elf, image.base, snapshots);
        require(!built_snapshot, "wrong-size ELF built snapshots");
        elf.pop_back();
        elf.back() ^= 1;
        require(!fixed.recognizes(elf), "changed whole-file fingerprint accepted");
        fixed.register_image_lazy(elf, image.base, snapshots);
        require(!built_snapshot, "changed whole-file fingerprint built snapshots");
        elf.back() ^= 1;
        fixed.register_image_lazy(elf, image.base, snapshots);
        require(built_snapshot, "authenticated ELF did not build snapshots");
        elf.back() ^= 1;
        FixedComponents tampered(true);
        tampered.register_image(elf, image.base, {});
        memory.map(0x10000, 3 * page_size, read_perm | write_perm);
        const char original[] = "abcdefghijklmnop";
        memory.copy_in(0x10000, original, sizeof(original));
        CpuState cpu;
        auto invoke = [&](size_t symbol, uint64_t x0, uint64_t x1, uint64_t x2) {
            cpu.pc = image.base + symbols[symbol].value;
            cpu.x[0] = x0;
            cpu.x[1] = x1;
            cpu.x[2] = x2;
            cpu.x[30] = 0x4000;
            bool result = fixed.execute(cpu, memory);
            if (result)
                require(cpu.pc == 0x4000, "helper did not return through LR");
            return result;
        };
        require(invoke(0, 0x11000, 0x10000, sizeof(original)), "memcpy not accelerated");
        char bytes[sizeof(original)]{};
        memory.copy_out(0x11000, bytes, sizeof(bytes));
        require(!std::memcmp(bytes, original, sizeof(bytes)), "memcpy result mismatch");
        require(invoke(1, 0x10002, 0x10000, 12), "overlapping memmove not accelerated");
        memory.copy_out(0x10000, bytes, 14);
        require(!std::memcmp(bytes, "ababcdefghijkl", 14), "memmove overlap corrupted source");
        require(invoke(2, 0x11000, 0x1ff, 16), "memset not accelerated");
        memory.copy_out(0x11000, bytes, 16);
        for (unsigned j = 0; j < 16; ++j)
            require(uint8_t(bytes[j]) == 255, "memset must truncate fill byte");
        memory.copy_in(0x12ffc, "end", 4);
        require(invoke(3, 0x12ffc, 0, 0) && cpu.x[0] == 3, "strlen failed at mapped page end");
        memory.copy_in(0x12ffc, "xxxx", 4);
        require(!invoke(3, 0x12ffc, 0, 0), "strlen read beyond mapped pages");
        require(!invoke(0, 0x11000, 0x10000, max_io + 1), "large operation did not fall back");
        require(!invoke(0, 0x12fff, 0x10000, 2), "invalid destination did not fall back");
        require(invoke(0, 0, 0, 0), "zero-length copy should not access pointers");
        cpu.pc = image.base + symbols[0].value;
        require(!tampered.execute(cpu, memory), "tampered ELF registered helpers");
        require(!disabled.execute(cpu, memory), "disabled registry executed helper");
        const uint64_t symbol = image.base + symbols[0].value;
        memory.protect(align_down(symbol), page_size, read_perm | write_perm | exec_perm);
        auto instruction = memory.load(symbol, 4);
        memory.store(symbol, instruction ^ 1, 4);
        require(!invoke(0, 0x11000, 0x10000, 8), "modified executable code retained native acceleration");
        memory.store(symbol, instruction, 4);
        require(invoke(0, 0x11000, 0x10000, 8), "restored executable did not revalidate");
        memory.unmap(align_down(symbol), page_size);
        require(!invoke(0, 0x11000, 0x10000, 8), "unmapped code retained native acceleration");
        require(fixed.calls == 6, "unexpected accelerated call count");
        copy_pipeline_fallbacks();
        helper_budget_differential(argv[1]);
        parallel_helpers(argv[1]);
        resumed_copy(argv[1]);
        resumed_copy_fault(argv[1]);
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    } catch (...) {
        std::fprintf(stderr, "unexpected guest error\n");
        return 1;
    }
}
