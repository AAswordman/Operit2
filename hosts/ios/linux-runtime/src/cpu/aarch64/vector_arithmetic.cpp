#include "cpu/aarch64/vector_multiply.h"
namespace operit::arm64 {
namespace {
bool execute_rounding_doubling_accumulate(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0xaf20fc00;
    if (op != 0x2e008400 && op != 0x2e008c00) // SQRDMLAH / SQRDMLSH (FEAT_RDM)
        return false;
    const bool scalar = r & (1u << 28), q = r & (1u << 30);
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    if ((size != 2 && size != 4) || (scalar && !q))
        return false;
    const auto a = cpu.v[(r >> 5) & 31], b = cpu.v[(r >> 16) & 31], old = cpu.v[r & 31];
    const unsigned count = scalar ? size : q ? 16 : 8;
    Vector result{};
    for (unsigned j = 0; j < count; j += size) {
        const uint64_t av = vector_lane(a, j, size), bv = vector_lane(b, j, size);
        const uint64_t value = vector_rounding_doubling_accumulate(cpu, av, bv, bits,
                                                                  vector_lane(old, j, size), op == 0x2e008c00);
        vector_lane(result, j, size, value);
    }
    cpu.v[r & 31] = result;
    return true;
}
bool execute_mixed_saturating_accumulate(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0xaf3ffc00;
    if (op != 0x0e203800 && op != 0x2e203800)
        return false;
    const bool scalar = r & (1u << 28), q = r & (1u << 30), u = r & (1u << 29);
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    if ((!q && scalar) || (!scalar && !q && size == 8))
        return false;
    const auto input = cpu.v[(r >> 5) & 31], old = cpu.v[r & 31];
    Vector result{};
    const unsigned count = scalar ? size : q ? 16 : 8;
    for (unsigned j = 0; j < count; j += size) {
        const uint64_t a = vector_lane(old, j, size), b = vector_lane(input, j, size);
        const __int128 accumulator = u ? static_cast<__int128>(a) : sign_extend(a, bits);
        const __int128 addend = u ? sign_extend(b, bits) : static_cast<__int128>(b);
        vector_lane(result, j, size, vector_saturate(cpu, accumulator + addend, bits, u));
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace
bool execute_vector_arithmetic(CpuState &cpu, uint32_t r) {
    if (execute_rounding_doubling_accumulate(cpu, r) || execute_mixed_saturating_accumulate(cpu, r))
        return true;
    const uint32_t op = r & 0x8f20fc00;
    // Three-same integer operations; U selects signedness or the paired operation.
    switch (op) {
    case 0x0e200400:
    case 0x0e201400:
    case 0x0e202400: // halving add/rounding add/sub
    case 0x0e200c00:
    case 0x0e202c00: // saturating add/sub
    case 0x0e203400:
    case 0x0e203c00: // comparisons
    case 0x0e204c00:
    case 0x0e205c00:
    case 0x0e204400:
    case 0x0e205400: // variable shifts
    case 0x0e206400:
    case 0x0e206c00: // min/max
    case 0x0e207400:
    case 0x0e207c00: // absolute difference/accumulate
    case 0x0e208400:
    case 0x0e208c00: // add/sub, bit test/equality
    case 0x0e209400:
    case 0x0e209c00: // multiply/accumulate
    case 0x0e20b400: // SQDMULH / SQRDMULH
    case 0x0e20a400:
    case 0x0e20ac00:
    case 0x0e20bc00: // pairwise
        break;
    default:
        return false;
    }
    const bool u = r & (1u << 29), pair = op == 0x0e20a400 || op == 0x0e20ac00 || op == 0x0e20bc00;
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    const bool scalar = r & (1u << 28);
    if (scalar && (!(r & (1u << 30)) || (bits != 64 && op != 0x0e200c00 && op != 0x0e202c00 &&
                                         op != 0x0e204c00 && op != 0x0e205c00 && op != 0x0e20b400)))
        return false;
    if (scalar &&
        (op == 0x0e200400 || op == 0x0e201400 || op == 0x0e202400 || op == 0x0e206400 || op == 0x0e206c00 ||
         op == 0x0e207400 || op == 0x0e207c00 || op == 0x0e209400 || op == 0x0e209c00 || pair))
        return false;
    const unsigned count = scalar ? size : r & (1u << 30) ? 16 : 8, lanes = count / size;
    if (bits == 64 && ((!scalar && count == 8) ||
                       (op != 0x0e208400 && op != 0x0e208c00 && op != 0x0e203400 && op != 0x0e203c00 &&
                        op != 0x0e204400 && op != 0x0e205400 && op != 0x0e204c00 && op != 0x0e205c00 &&
                        op != 0x0e200c00 && op != 0x0e202c00 && op != 0x0e20bc00)))
        return false;
    if ((op == 0x0e20bc00 && u) || (op == 0x0e209c00 && u && size != 1) ||
        (op == 0x0e20b400 && size != 2 && size != 4))
        return false;
    const auto a = cpu.v[(r >> 5) & 31], b = cpu.v[(r >> 16) & 31], old = cpu.v[r & 31];
    Vector result{};
    for (unsigned j = 0; j < lanes; ++j) {
        const auto &left = pair && j >= lanes / 2 ? b : a;
        const auto &right = pair ? left : b;
        const unsigned left_index = pair ? (j % (lanes / 2)) * 2 : j;
        const unsigned right_index = pair ? left_index + 1 : j;
        const uint64_t av = vector_lane(left, left_index * size, size),
                       bv = vector_lane(right, right_index * size, size);
        const __int128 x = u ? static_cast<__int128>(av) : sign_extend(av, bits);
        const __int128 y = u ? static_cast<__int128>(bv) : sign_extend(bv, bits);
        const uint64_t previous = vector_lane(old, j * size, size);
        uint64_t value;
        switch (op) {
        case 0x0e200400:
            value = uint64_t((x + y) >> 1);
            break;
        case 0x0e201400:
            value = uint64_t((x + y + 1) >> 1);
            break;
        case 0x0e202400:
            value = uint64_t((x - y) >> 1);
            break;
        case 0x0e200c00:
            value = vector_saturate(cpu, x + y, bits, u);
            break;
        case 0x0e202c00:
            value = vector_saturate(cpu, x - y, bits, u);
            break;
        case 0x0e203400:
            value = x > y ? mask(bits) : 0;
            break;
        case 0x0e203c00:
            value = x >= y ? mask(bits) : 0;
            break;
        case 0x0e206400:
        case 0x0e20a400:
            value = uint64_t(std::max(x, y));
            break;
        case 0x0e206c00:
        case 0x0e20ac00:
            value = uint64_t(std::min(x, y));
            break;
        case 0x0e207400:
        case 0x0e207c00:
            value = uint64_t(x > y ? x - y : y - x) + (op == 0x0e207c00 ? previous : 0);
            break;
        case 0x0e208400:
            value = u ? av - bv : av + bv;
            break;
        case 0x0e208c00:
            value = (u ? av == bv : (av & bv) != 0) ? mask(bits) : 0;
            break;
        case 0x0e209400:
            value = u ? previous - av * bv : previous + av * bv;
            break;
        case 0x0e209c00:
            value = u ? uint64_t(vector_polynomial_product(av, bv, bits)) : av * bv;
            break;
        case 0x0e20b400:
            value = vector_doubling_high(cpu, av, bv, bits, u);
            break;
        case 0x0e20bc00:
            value = av + bv;
            break;
        default: { // shift amount is the signed low byte, not the full element.
            const int shift = int(int8_t(bv));
            const bool saturating = op == 0x0e204c00 || op == 0x0e205c00;
            if (shift >= 0) {
                if (saturating) {
                    const __int128 widened = unsigned(shift) >= bits
                                                 ? (x == 0  ? 0
                                                    : x > 0 ? static_cast<__int128>(1) << bits
                                                            : -(static_cast<__int128>(1) << bits) - 1)
                                                 : x * (static_cast<__int128>(1) << shift);
                    value = vector_saturate(cpu, widened, bits, u);
                } else
                    value = unsigned(shift) >= bits ? 0 : av << shift;
            } else {
                const unsigned amount = unsigned(-shift);
                const bool round = op == 0x0e205400 || op == 0x0e205c00;
                if (amount > bits)
                    value = !u && !round && x < 0 ? mask(bits) : 0;
                else
                    value = uint64_t((x + (round ? static_cast<__int128>(1) << (amount - 1) : 0)) >> amount);
            }
            break;
        }
        }
        vector_lane(result, j * size, size, value);
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace operit::arm64
