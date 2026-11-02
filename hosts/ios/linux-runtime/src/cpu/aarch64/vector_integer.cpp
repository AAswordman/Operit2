#include "cpu/aarch64/vector_integer.h"
#include "cpu/aarch64/vector_lanes.h"
#include <cstring>
namespace operit::arm64 {
bool execute_vector_pairwise_widen(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0xbf3ffc00;
    switch (op) {
    case 0x0e202800: // SADDLP
    case 0x2e202800: // UADDLP
    case 0x0e206800: // SADALP
    case 0x2e206800: // UADALP
        break;
    default:
        return false;
    }
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    if (size == 8)
        return false;
    const bool u = r & (1u << 29), accumulate = r & (1u << 14);
    const unsigned count = r & (1u << 30) ? 16 : 8, output_size = size * 2;
    const auto source = cpu.v[(r >> 5) & 31], old = cpu.v[r & 31];
    Vector result{};
    for (unsigned j = 0; j < count; j += output_size) {
        const uint64_t a = vector_lane(source, j, size), b = vector_lane(source, j + size, size);
        const __int128 x = u ? static_cast<__int128>(a) : sign_extend(a, bits);
        const __int128 y = u ? static_cast<__int128>(b) : sign_extend(b, bits);
        // Widen before adding; accumulating forms wrap at the widened width.
        const uint64_t value = uint64_t(x + y) + (accumulate ? vector_lane(old, j, output_size) : 0);
        vector_lane(result, j, output_size, value);
    }
    cpu.v[r & 31] = result;
    return true;
}
bool execute_vector_integer(CpuState &cpu, uint32_t r) {
    if (execute_vector_pairwise_widen(cpu, r))
        return true;
    const unsigned rd = r & 31, rn = (r >> 5) & 31;
    std::array<uint8_t, 16> result{};
    if ((r & 0xfffffc00) == 0x5ef1b800) { // ADDP scalar
        uint64_t a, b;
        std::memcpy(&a, cpu.v[rn].data(), 8);
        std::memcpy(&b, cpu.v[rn].data() + 8, 8);
        const uint64_t sum = a + b;
        std::memcpy(result.data(), &sum, 8);
        cpu.v[rd] = result;
        return true;
    }
    if ((r & 0xffe08400) == 0x6e000400) { // INS (element)
        const unsigned imm5 = (r >> 16) & 31, imm4 = (r >> 11) & 15;
        if (!imm5)
            return false;
        const unsigned log_size = __builtin_ctz(imm5), size = 1u << log_size;
        if (size > 8 || (imm4 & (size - 1)))
            return false;
        const unsigned destination = (imm5 >> (log_size + 1)) * size;
        const unsigned source = (imm4 >> log_size) * size;
        const auto original = cpu.v[rn];
        std::memcpy(cpu.v[rd].data() + destination, original.data() + source, size);
        return true;
    }
    if ((r & 0xffe0fc00) == 0x5e000400) { // DUP (scalar): copy one lane, clear all upper bits.
        const unsigned imm5 = (r >> 16) & 31;
        if (!imm5)
            return false;
        const unsigned log_size = __builtin_ctz(imm5), size = 1u << log_size;
        if (size > 8)
            return false;
        const unsigned source = (imm5 >> (log_size + 1)) * size;
        std::memcpy(result.data(), cpu.v[rn].data() + source, size);
        cpu.v[rd] = result;
        return true;
    }
    if ((r & 0xbfe08400) == 0x2e000000) { // EXT: byte extraction from concatenated operands.
        const unsigned count = r & (1u << 30) ? 16 : 8, offset = (r >> 11) & 15;
        if (offset >= count)
            return false;
        const auto a = cpu.v[rn], b = cpu.v[(r >> 16) & 31];
        for (unsigned j = 0; j < count; ++j)
            result[j] = offset + j < count ? a[offset + j] : b[offset + j - count];
        cpu.v[rd] = result;
        return true;
    }
    return false;
}
} // namespace operit::arm64
