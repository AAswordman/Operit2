#include "cpu/aarch64/vector_shift.h"
#include "cpu/aarch64/vector_lanes.h"
#include <cstring>
namespace operit::arm64 {
bool execute_vector_shift(CpuState &cpu, uint32_t r) {
    if ((r & 0x8f800400) != 0x0f000400)
        return false;
    // Scalar SIMD shifts require Q=1. With Q=0 this encoding space includes
    // scalar floating-point multiply-add instructions.
    if ((r & (1u << 28)) && !(r & (1u << 30)))
        return false;
    const unsigned immediate = (r >> 16) & 127;
    if (immediate < 8)
        return false;
    const unsigned bits = 1u << (31 - __builtin_clz(immediate));
    const unsigned size = bits / 8, count = r & (1u << 28) ? 8 : (r & (1u << 30) ? 16 : 8);
    if ((r & (1u << 28)) && bits != 64 && ((r >> 11) & 31) != 12 && ((r >> 11) & 31) != 14)
        return false;
    if (bits > count * 8)
        return false;
    const unsigned operation = (r >> 11) & 31;
    const bool u = r & (1u << 29);
    if (operation == 20 && !(r & (1u << 28))) { // SSHLL/USHLL and their upper-half forms
        if (bits == 64)
            return false;
        const auto source = cpu.v[(r >> 5) & 31];
        std::array<uint8_t, 16> output{};
        const unsigned start = r & (1u << 30) ? 8 : 0;
        for (unsigned lane = 0; lane < 8 / size; ++lane) {
            uint64_t value = 0;
            std::memcpy(&value, source.data() + start + lane * size, size);
            if (!u)
                value = static_cast<uint64_t>(sign_extend(value, bits));
            value <<= immediate - bits;
            std::memcpy(output.data() + lane * size * 2, &value, size * 2);
        }
        cpu.v[r & 31] = output;
        return true;
    }
    if (operation == 14 || (operation == 12 && u)) { // SQSHL/UQSHL/SQSHLU immediate
        const bool scalar = r & (1u << 28);
        if (!scalar && bits == 64 && count == 8)
            return false;
        const auto source = cpu.v[(r >> 5) & 31];
        Vector output{};
        const unsigned shift = immediate - bits;
        for (unsigned j = 0; j < (scalar ? size : count); j += size) {
            const uint64_t input = vector_lane(source, j, size);
            const __int128 value =
                operation == 14 && u ? static_cast<__int128>(input) : sign_extend(input, bits);
            vector_lane(output, j, size,
                        vector_saturate(cpu, value * (static_cast<__int128>(1) << shift), bits, u));
        }
        cpu.v[r & 31] = output;
        return true;
    }
    if (!(r & (1u << 28)) && bits == 64 && count == 8)
        return false;
    if (operation != 0 && operation != 2 && operation != 4 && operation != 6 && operation != 8 &&
        operation != 10)
        return false;
    if (operation == 8 && !u)
        return false;
    const auto source = cpu.v[(r >> 5) & 31], previous = cpu.v[r & 31];
    std::array<uint8_t, 16> result{};
    for (unsigned lane = 0; lane < count; lane += size) {
        uint64_t value = 0, old = 0, output;
        std::memcpy(&value, source.data() + lane, size);
        std::memcpy(&old, previous.data() + lane, size);
        if (operation == 10) {
            unsigned shift = immediate - bits;
            output = (value << shift) | (u ? old & mask(shift) : 0);
        } else {
            unsigned shift = bits * 2 - immediate;
            if (operation == 8) {
                output = (old & ~mask(bits - shift)) |
                         static_cast<uint64_t>(static_cast<unsigned __int128>(value) >> shift);
            } else {
                const bool round = operation == 4 || operation == 6;
                if (u) {
                    unsigned __int128 extended = value;
                    if (round)
                        extended += static_cast<unsigned __int128>(1) << (shift - 1);
                    output = static_cast<uint64_t>(extended >> shift);
                } else {
                    __int128 extended = sign_extend(value, bits);
                    if (round)
                        extended += static_cast<__int128>(1) << (shift - 1);
                    output = static_cast<uint64_t>(extended >> shift);
                }
                if (operation == 2 || operation == 6)
                    output += old;
            }
        }
        std::memcpy(result.data() + lane, &output, size);
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace operit::arm64
