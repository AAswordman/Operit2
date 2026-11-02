#include "cpu/aarch64/vector_narrow.h"
#include "cpu/aarch64/vector_lanes.h"
namespace operit::arm64 {
namespace {
bool execute_narrow_shift(CpuState &cpu, uint32_t r) {
    if ((r & 0x8f800400) != 0x0f000400)
        return false;
    const unsigned operation = (r >> 11) & 31, immediate = (r >> 16) & 127;
    if (operation < 16 || operation > 19 || immediate < 8)
        return false;
    const unsigned bits = 1u << (31 - __builtin_clz(immediate)), size = bits / 8;
    const bool scalar = r & (1u << 28), u = r & (1u << 29), round = operation & 1;
    if (bits == 64 || (scalar && (!(r & (1u << 30)) || (operation < 18 && !u))))
        return false;
    const unsigned shift = bits * 2 - immediate, start = !scalar && (r & (1u << 30)) ? 8 : 0;
    const bool saturating = operation >= 18 || u, unsigned_input = operation >= 18 && u;
    const auto source = cpu.v[(r >> 5) & 31];
    Vector result{};
    if (start)
        std::copy_n(cpu.v[r & 31].begin(), 8, result.begin());
    for (unsigned j = 0; j < (scalar ? size : 8); j += size) {
        const uint64_t input = vector_lane(source, j * 2, size * 2);
        __int128 value =
            unsigned_input || !saturating ? static_cast<__int128>(input) : sign_extend(input, bits * 2);
        if (round)
            value += static_cast<__int128>(1) << (shift - 1);
        value >>= shift;
        const uint64_t output = saturating ? vector_saturate(cpu, value, bits, u) : uint64_t(value);
        vector_lane(result, start + j, size, output);
    }
    cpu.v[r & 31] = result;
    return true;
}
bool execute_narrow_register(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0x8f3ffc00;
    if (op != 0x0e212800 && op != 0x0e214800)
        return false;
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    const bool scalar = r & (1u << 28), q = r & (1u << 30), u = r & (1u << 29);
    if (size == 8 || (scalar && (!q || (op == 0x0e212800 && !u))))
        return false;
    const auto source = cpu.v[(r >> 5) & 31], old = cpu.v[r & 31];
    const unsigned start = !scalar && q ? 8 : 0;
    Vector result{};
    if (start)
        std::copy_n(old.begin(), 8, result.begin());
    for (unsigned j = 0; j < (scalar ? size : 8); j += size) {
        const uint64_t input = vector_lane(source, j * 2, size * 2);
        const __int128 value =
            op == 0x0e214800 && u ? static_cast<__int128>(input) : sign_extend(input, bits * 2);
        const uint64_t output = op == 0x0e214800 || u ? vector_saturate(cpu, value, bits, u) : input;
        vector_lane(result, start + j, size, output);
    }
    cpu.v[r & 31] = result;
    return true;
}
bool execute_narrow_high(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0x9f20fc00;
    if (op != 0x0e204000 && op != 0x0e206000)
        return false;
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    if (size == 8)
        return false;
    const bool q = r & (1u << 30), round = r & (1u << 29);
    const auto a = cpu.v[(r >> 5) & 31], b = cpu.v[(r >> 16) & 31], old = cpu.v[r & 31];
    Vector result{};
    if (q)
        std::copy_n(old.begin(), 8, result.begin());
    for (unsigned j = 0; j < 8; j += size) {
        const uint64_t x = vector_lane(a, j * 2, size * 2), y = vector_lane(b, j * 2, size * 2);
        const uint64_t value = (op == 0x0e204000 ? x + y : x - y) + (round ? uint64_t{1} << (bits - 1) : 0);
        vector_lane(result, (q ? 8 : 0) + j, size, value >> bits);
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace
bool execute_vector_narrow(CpuState &cpu, uint32_t r) {
    return execute_narrow_shift(cpu, r) || execute_narrow_register(cpu, r) || execute_narrow_high(cpu, r);
}
} // namespace operit::arm64
