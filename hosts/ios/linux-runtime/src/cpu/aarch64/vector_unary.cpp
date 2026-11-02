#include "cpu/aarch64/vector_lanes.h"
namespace operit::arm64 {
bool execute_vector_unary(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0xaf3ffc00;
    const bool scalar = r & (1u << 28);
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8,
                   count = scalar           ? size
                           : r & (1u << 30) ? 16
                                            : 8;
    if (scalar && (!(r & (1u << 30)) || (bits != 64 && op != 0x0e207800 && op != 0x2e207800)))
        return false;
    const bool count_leading = op == 0x0e204800 || op == 0x2e204800;
    if (count_leading && (scalar || size == 8))
        return false;
    const auto source = cpu.v[(r >> 5) & 31];
    Vector result{};
    // Byte-wise operations and element reversals have their own size constraints.
    if (scalar &&
        (op == 0x0e205800 || op == 0x2e205800 || op == 0x0e201800 || op == 0x2e200800 || op == 0x0e200800))
        return false;
    if (op == 0x0e205800 || op == 0x2e205800 || op == 0x0e201800 || op == 0x2e200800 || op == 0x0e200800) {
        if ((op == 0x0e205800 && size != 1) || (op == 0x2e205800 && size > 2))
            return false;
        const unsigned group = op == 0x0e201800 ? 2 : op == 0x2e200800 ? 4 : 8;
        if (op != 0x0e205800 && op != 0x2e205800 && size >= group)
            return false;
        for (unsigned j = 0; j < count; ++j) {
            if (op == 0x0e205800)
                result[j] = uint8_t(__builtin_popcount(source[j]));
            else if (op == 0x2e205800) {
                if (size == 1)
                    result[j] = uint8_t(~source[j]);
                else {
                    uint8_t value = source[j], reversed = 0;
                    for (unsigned k = 0; k < 8; ++k) {
                        reversed = uint8_t((reversed << 1) | (value & 1));
                        value >>= 1;
                    }
                    result[j] = reversed;
                }
            } else
                result[j] =
                    source[(j / group) * group + (group / size - 1 - (j % group) / size) * size + j % size];
        }
    } else {
        switch (op) {
        case 0x0e204800: // CLS
        case 0x2e204800: // CLZ
        case 0x0e20b800:
        case 0x2e20b800:
        case 0x0e207800:
        case 0x2e207800:
        case 0x0e208800:
        case 0x2e208800:
        case 0x0e209800:
        case 0x2e209800:
        case 0x0e20a800:
            break;
        default:
            return false;
        }
        if (!scalar && size == 8 && count == 8)
            return false;
        for (unsigned j = 0; j < count; j += size) {
            const __int128 x = sign_extend(vector_lane(source, j, size), bits);
            uint64_t value;
            switch (op) {
            case 0x0e204800:
            case 0x2e204800: {
                const uint64_t original = vector_lane(source, j, size);
                const bool signed_count = op == 0x0e204800;
                const uint64_t normalized = signed_count && x < 0 ? (~original & mask(bits)) : original;
                const unsigned leading =
                    normalized ? unsigned(__builtin_clzll(normalized)) - (64 - bits) : bits;
                value = leading - (signed_count ? 1 : 0);
                break;
            }
            case 0x0e20b800:
                value = uint64_t(x < 0 ? -x : x);
                break;
            case 0x2e20b800:
                value = uint64_t(-x);
                break;
            case 0x0e207800:
                value = vector_saturate(cpu, x < 0 ? -x : x, bits, false);
                break;
            case 0x2e207800:
                value = vector_saturate(cpu, -x, bits, false);
                break;
            case 0x0e208800:
                value = x > 0 ? mask(bits) : 0;
                break;
            case 0x2e208800:
                value = x >= 0 ? mask(bits) : 0;
                break;
            case 0x0e209800:
                value = x == 0 ? mask(bits) : 0;
                break;
            case 0x2e209800:
                value = x <= 0 ? mask(bits) : 0;
                break;
            default:
                value = x < 0 ? mask(bits) : 0;
                break;
            }
            vector_lane(result, j, size, value);
        }
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace operit::arm64
