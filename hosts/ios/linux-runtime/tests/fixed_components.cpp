#include "acceleration/fixed_components.h"
#include "elf/loader.h"
#include <cstdio>
#include <stdexcept>

using namespace operit::arm64;
static void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
int main(int argc, char **argv) {
    try {
        require(argc == 2, "expected root directory");
        Filesystem filesystem(argv[1]);
        Memory memory(32 << 20);
        FixedComponents fixed(true), disabled(false);
        ElfLoader loader(memory, filesystem, 32 << 20, fixed);
        const auto image = loader.load("/lib/ld-musl-aarch64.so.1", 0x6000000000, true);
        auto elf = filesystem.read_file("/lib/ld-musl-aarch64.so.1", 32 << 20);
        require(fixed.recognizes(elf), "fixture must be the pinned musl ELF");
        require(!disabled.recognizes(elf), "disabled registry accepted ELF");
        elf.back() ^= 1;
        require(!fixed.recognizes(elf), "changed whole-file fingerprint accepted");
        FixedComponents tampered(true);
        tampered.register_image(elf, image.base, {});
        memory.map(0x10000, 3 * page_size, read_perm | write_perm);
        const char original[] = "abcdefghijklmnop";
        memory.copy_in(0x10000, original, sizeof(original));
        CpuState cpu;
        auto invoke = [&](size_t symbol, uint64_t x0, uint64_t x1, uint64_t x2) {
            cpu.pc = image.base + fixed_musl_symbols[symbol].value;
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
        cpu.pc = image.base + fixed_musl_symbols[0].value;
        require(!tampered.execute(cpu, memory), "tampered ELF registered helpers");
        require(!disabled.execute(cpu, memory), "disabled registry executed helper");
        const uint64_t symbol = image.base + fixed_musl_symbols[0].value;
        memory.protect(align_down(symbol), page_size, read_perm | write_perm | exec_perm);
        auto instruction = memory.load(symbol, 4);
        memory.store(symbol, instruction ^ 1, 4);
        require(!invoke(0, 0x11000, 0x10000, 8), "modified executable code retained native acceleration");
        memory.store(symbol, instruction, 4);
        require(invoke(0, 0x11000, 0x10000, 8), "restored executable did not revalidate");
        memory.unmap(align_down(symbol), page_size);
        require(!invoke(0, 0x11000, 0x10000, 8), "unmapped code retained native acceleration");
        require(fixed.calls == 6, "unexpected accelerated call count");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    } catch (...) {
        std::fprintf(stderr, "unexpected guest error\n");
        return 1;
    }
}
