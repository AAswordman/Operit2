#include "cpu/aarch64/vector_indexed.h"
#include "cpu/aarch64/vector_integer.h"
#include "cpu/aarch64/vector_multiply.h"
namespace operit::arm64 {
namespace {
bool execute_shift_long(CpuState &cpu, uint32_t r) {
    if ((r & 0xbf3ffc00) != 0x2e213800) // SHLL / SHLL2, immediate equals the source element width.
        return false;
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    if (size == 8)
        return false;
    const unsigned start = r & (1u << 30) ? 8 : 0;
    const auto source = cpu.v[(r >> 5) & 31];
    Vector result{};
    for (unsigned j = 0; j < 8 / size; ++j)
        vector_lane(result, j * size * 2, size * 2, vector_lane(source, start + j * size, size) << bits);
    cpu.v[r & 31] = result;
    return true;
}

bool execute_polynomial_long(CpuState &cpu, uint32_t r) {
    if ((r & 0xbf20fc00) != 0x0e20e000) // PMULL / PMULL2; no scalar or unsigned variant.
        return false;
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    if (size != 1 && size != 8)
        return false;
    const unsigned start = r & (1u << 30) ? 8 : 0;
    const auto a = cpu.v[(r >> 5) & 31], b = cpu.v[(r >> 16) & 31];
    Vector result{};
    for (unsigned j = 0; j < 8 / size; ++j) {
        const uint64_t av = vector_lane(a, start + j * size, size);
        const uint64_t bv = vector_lane(b, start + j * size, size);
        const unsigned __int128 product = vector_polynomial_product(av, bv, bits);
        if (size == 8) { // FEAT_PMULL: preserve the entire 64 x 64 polynomial product.
            vector_lane(result, 0, 8, uint64_t(product));
            vector_lane(result, 8, 8, uint64_t(product >> 64));
        } else
            vector_lane(result, j * 2, 2, uint64_t(product));
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace
bool execute_vector_widen(CpuState &cpu, uint32_t r) {
    if (execute_vector_indexed_integer(cpu, r) || execute_vector_pairwise_widen(cpu, r) ||
        execute_shift_long(cpu, r) || execute_polynomial_long(cpu, r))
        return true;
    const uint32_t op = r & 0x8f20fc00;
    switch (op) {
    case 0x0e200000:
    case 0x0e202000: // add/sub long
    case 0x0e201000:
    case 0x0e203000: // add/sub wide
    case 0x0e20c000:
    case 0x0e208000:
    case 0x0e20a000: // multiply long/accumulate/subtract
    case 0x0e207000:
    case 0x0e205000: // absolute difference long/accumulate
    case 0x0e209000:
    case 0x0e20b000:
    case 0x0e20d000: // saturating doubling long/accumulate/subtract
        break;
    default:
        return false;
    }
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8;
    const bool scalar = r & (1u << 28), u = r & (1u << 29), q = r & (1u << 30);
    const bool doubling = op == 0x0e209000 || op == 0x0e20b000 || op == 0x0e20d000;
    if (size == 8 || (doubling && (u || size == 1)) || (scalar && (!q || !doubling)))
        return false;
    const bool wide = op == 0x0e201000 || op == 0x0e203000;
    const unsigned start = !scalar && q ? 8 : 0;
    const auto a = cpu.v[(r >> 5) & 31], b = cpu.v[(r >> 16) & 31], old = cpu.v[r & 31];
    Vector result{};
    for (unsigned j = 0; j < (scalar ? 1 : 8 / size); ++j) {
        const uint64_t av = vector_lane(a, wide ? j * size * 2 : start + j * size, wide ? size * 2 : size);
        const uint64_t bv = vector_lane(b, start + j * size, size);
        const uint64_t previous = vector_lane(old, j * size * 2, size * 2);
        const __int128 x = u ? static_cast<__int128>(av) : sign_extend(av, wide ? bits * 2 : bits);
        const __int128 y = u ? static_cast<__int128>(bv) : sign_extend(bv, bits);
        uint64_t value;
        switch (op) {
        case 0x0e200000:
        case 0x0e201000:
            value = uint64_t(x + y);
            break;
        case 0x0e202000:
        case 0x0e203000:
            value = uint64_t(x - y);
            break;
        case 0x0e20c000:
            value = uint64_t(x * y);
            break;
        case 0x0e208000:
            value = previous + uint64_t(x * y);
            break;
        case 0x0e20a000:
            value = previous - uint64_t(x * y);
            break;
        case 0x0e209000:
        case 0x0e20b000:
        case 0x0e20d000:
            value = vector_doubling_long(cpu, av, bv, bits, previous, op != 0x0e20d000, op == 0x0e20b000);
            break;
        default:
            value = uint64_t(x > y ? x - y : y - x) + (op == 0x0e205000 ? previous : 0);
            break;
        }
        vector_lane(result, j * size * 2, size * 2, value);
    }
    cpu.v[r & 31] = result;
    return true;
}
bool execute_vector_reduction(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0x9f3ffc00;
    if (op != 0x0e31b800 && op != 0x0e30a800 && op != 0x0e31a800 && op != 0x0e303800)
        return false;
    const unsigned size = 1u << ((r >> 22) & 3), bits = size * 8, count = r & (1u << 30) ? 16 : 8;
    const bool u = r & (1u << 29);
    if (size == 8 || (size == 4 && count == 8) || (op == 0x0e31b800 && u))
        return false;
    const auto source = cpu.v[(r >> 5) & 31];
    Vector result{};
    __int128 total = 0;
    for (unsigned j = 0; j < count; j += size) {
        const uint64_t original = vector_lane(source, j, size);
        const __int128 value = u ? static_cast<__int128>(original) : sign_extend(original, bits);
        if (op == 0x0e30a800)
            total = j ? std::max(total, value) : value;
        else if (op == 0x0e31a800)
            total = j ? std::min(total, value) : value;
        else
            total += value;
    }
    vector_lane(result, 0, op == 0x0e303800 ? size * 2 : size, uint64_t(total));
    cpu.v[r & 31] = result;
    return true;
}
} // namespace operit::arm64
