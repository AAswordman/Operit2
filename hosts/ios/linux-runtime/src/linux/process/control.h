#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace operit::arm64::process {
enum class MmOption : uint32_t {
    StartCode = 1,
    EndCode = 2,
    StartData = 3,
    EndData = 4,
    StartStack = 5,
    StartBrk = 6,
    Brk = 7,
    ArgStart = 8,
    ArgEnd = 9,
    EnvStart = 10,
    EnvEnd = 11,
    Auxv = 12,
    ExeFile = 13,
    Map = 14,
    MapSize = 15,
};
struct MemoryBounds {
    uint64_t start_code = 0, end_code = 0;
    uint64_t start_data = 0, end_data = 0;
    uint64_t start_brk = 0, brk = 0, start_stack = 0;
    uint64_t arg_start = 0, arg_end = 0, env_start = 0, env_end = 0;
};
// The AArch64 Linux prctl_mm_map ABI uses an eight-byte user pointer.
struct MmMap {
    MemoryBounds bounds;
    uint64_t auxv = 0;
    uint32_t auxv_size = 0, exe_fd = UINT32_MAX;
};
static_assert(sizeof(MemoryBounds) == 88);
static_assert(sizeof(MmMap) == 104 && offsetof(MmMap, auxv) == 88 &&
              offsetof(MmMap, auxv_size) == 96 && offsetof(MmMap, exe_fd) == 100);
// AArch64 AT_VECTOR_SIZE: two architecture entries, 22 common entries, AT_NULL.
constexpr size_t auxv_words = 2 * (2 + 22 + 1);
struct StringRange {
    uint64_t address, size;
};
struct MemoryState {
    MemoryBounds bounds;
    std::array<uint64_t, auxv_words> saved_auxv{};
    // Embedders can install a non-ELF image with discontiguous argv/envp strings.
    // Native ELF images and explicit PR_SET_MM_MAP updates use bounds instead.
    std::vector<StringRange> argument_ranges, environment_ranges;
};
} // namespace operit::arm64::process
