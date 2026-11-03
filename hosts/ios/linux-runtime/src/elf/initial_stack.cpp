#include "base/random.h"
#include "elf/loader.h"
#include "linux/threads/rseq.h"

namespace operit::arm64 {
uint64_t build_initial_stack(Memory &memory, const char *const *argv, const char *const *envp,
                             const Image &main, uint64_t interpreter_base) {
    constexpr uint64_t top = 0xffffffff0000;
    constexpr uint64_t size = 8 << 20;
    memory.map(top - size, size, read_perm | write_perm);
    memory.describe(top - size, size, {"[stack]", 0, 0, 0, false});
    uint64_t cursor = top;
    size_t string_bytes = 0;
    auto string = [&](const char *value) {
        const size_t length = strnlen(value, (1 << 20) + 1);
        if (length > (1 << 20) || string_bytes + length + 1 > (1 << 20))
            throw Failure("argv/environment exceeds 1 MiB");
        string_bytes += length + 1;
        cursor -= length + 1;
        memory.copy_in(cursor, value, length + 1);
        return cursor;
    };
    // Linux lays out argv strings consecutively in ascending address order,
    // followed by the environment. Programs such as libuv use that region for
    // process titles, so correct pointers alone are not sufficient.
    const uint64_t executable = string(main.path.c_str());
    auto vector = [&](const char *const *entries) {
        size_t count = 0;
        while (entries && entries[count]) {
            if (count >= 4096)
                throw Failure("too many argument/environment entries");
            ++count;
        }
        std::vector<uint64_t> pointers(count);
        for (size_t j = count; j > 0; --j)
            pointers[j - 1] = string(entries[j - 1]);
        return pointers;
    };
    const auto env = vector(envp);
    const auto args = vector(argv);
    const uint64_t platform = string("aarch64");
    uint8_t random[16];
    random_bytes(random, sizeof(random));
    cursor -= sizeof(random);
    memory.copy_in(cursor, random, sizeof(random));
    uint64_t random_address = cursor;
    std::vector<uint64_t> values{args.size()};
    values.insert(values.end(), args.begin(), args.end());
    values.push_back(0);
    values.insert(values.end(), env.begin(), env.end());
    values.push_back(0);
    const std::pair<uint64_t, uint64_t> auxiliary[] = {{3, main.phdr},
                                                       {4, main.phent},
                                                       {5, main.phnum},
                                                       {6, page_size},
                                                       {7, interpreter_base},
                                                       {8, 0},
                                                       {9, main.entry},
                                                       {11, ::getuid()},
                                                       {12, ::geteuid()},
                                                       {13, ::getgid()},
                                                       {14, ::getegid()},
                                                       {15, platform},
                                                       {16, 0},
                                                       {17, 100},
                                                       {23, 0},
                                                       {25, random_address},
                                                       {26, 0},
                                                       {27, threads::rseq_feature_size},
                                                       {28, threads::rseq_alignment},
                                                       {31, executable},
                                                       {0, 0}};
    for (const auto &entry : auxiliary) {
        values.push_back(entry.first);
        values.push_back(entry.second);
    }
    cursor = (cursor - values.size() * 8) & ~uint64_t{15};
    memory.copy_in(cursor, values.data(), values.size() * 8);
    return cursor;
}
} // namespace operit::arm64
