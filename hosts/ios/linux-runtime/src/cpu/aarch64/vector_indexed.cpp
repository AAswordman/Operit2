#include "cpu/aarch64/vector_indexed.h"
#include "cpu/aarch64/vector_multiply.h"
namespace operit::arm64 {
bool execute_vector_indexed_integer(CpuState &cpu, uint32_t r) {
    // Keep U and every fixed opcode bit in the match; Q and the scalar bit are
    // checked separately below. Floating-point and reserved encodings fall through.
    const uint32_t encoding = r & 0xaf00f400;
    switch (encoding) {
    case 0x2f000000: // MLA
    case 0x2f004000: // MLS
    case 0x0f008000: // MUL
    case 0x0f002000: // SMLAL / SMLAL2
    case 0x2f002000: // UMLAL / UMLAL2
    case 0x0f006000: // SMLSL / SMLSL2
    case 0x2f006000: // UMLSL / UMLSL2
    case 0x0f00a000: // SMULL / SMULL2
    case 0x2f00a000: // UMULL / UMULL2
    case 0x0f003000: // SQDMLAL / SQDMLAL2, scalar SQDMLAL
    case 0x0f007000: // SQDMLSL / SQDMLSL2, scalar SQDMLSL
    case 0x0f00b000: // SQDMULL / SQDMULL2, scalar SQDMULL
    case 0x0f00c000: // SQDMULH, vector/scalar
    case 0x0f00d000: // SQRDMULH, vector/scalar
    case 0x2f00d000: // SQRDMLAH, vector/scalar (FEAT_RDM)
    case 0x2f00f000: // SQRDMLSH, vector/scalar (FEAT_RDM)
        break;
    default:
        return false;
    }
    const unsigned op = (r >> 12) & 15, size = 1u << ((r >> 22) & 3), bits = size * 8;
    const bool u = r & (1u << 29), scalar = r & (1u << 28), q = r & (1u << 30);
    const bool high = op == 12 || op == 13 || op == 15;
    const bool rounding_accumulate = u && (op == 13 || op == 15);
    const bool doubling_long = op == 3 || op == 7 || op == 11;
    const bool same = op == 0 || op == 4 || op == 8 || high;
    if ((size != 2 && size != 4) || (scalar && (!q || (!high && !doubling_long))))
        return false;

    const auto element = VectorIndexedElement::integer(r, size);
    const auto a = cpu.v[(r >> 5) & 31], b = cpu.v[element.reg], old = cpu.v[r & 31];
    const uint64_t bv = vector_lane(b, element.lane * size, size);
    const unsigned start = !scalar && !same && q ? 8 : 0;
    const unsigned lanes = scalar ? 1 : (same && q ? 16 : 8) / size;
    const unsigned output_size = same ? size : size * 2;
    Vector result{};
    for (unsigned j = 0; j < lanes; ++j) {
        const uint64_t av = vector_lane(a, start + j * size, size);
        const uint64_t previous = vector_lane(old, j * output_size, output_size);
        uint64_t value;
        if (rounding_accumulate)
            value = vector_rounding_doubling_accumulate(cpu, av, bv, bits, previous, op == 15);
        else if (high)
            value = vector_doubling_high(cpu, av, bv, bits, op == 13);
        else if (doubling_long)
            value = vector_doubling_long(cpu, av, bv, bits, previous, op != 11, op == 7);
        else {
            // Low products are modulo esize. Long products must extend each
            // operand with the instruction's signedness before multiplying.
            const __int128 x = u ? static_cast<__int128>(av) : sign_extend(av, bits);
            const __int128 y = u ? static_cast<__int128>(bv) : sign_extend(bv, bits);
            const uint64_t product = uint64_t(x * y);
            value = op == 0 || op == 2   ? previous + product
                    : op == 4 || op == 6 ? previous - product
                                         : product;
        }
        vector_lane(result, j * output_size, output_size, value);
    }
    cpu.v[r & 31] = result;
    return true;
}
} // namespace operit::arm64
