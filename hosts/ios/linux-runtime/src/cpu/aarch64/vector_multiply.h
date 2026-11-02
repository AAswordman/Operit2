#pragma once
#include "cpu/aarch64/vector_lanes.h"
namespace operit::arm64 {
inline unsigned __int128 vector_polynomial_product(uint64_t a, uint64_t b, unsigned bits) {
    unsigned __int128 product = 0;
    for (unsigned bit = 0; bit < bits; ++bit)
        if (b & (uint64_t{1} << bit))
            product ^= static_cast<unsigned __int128>(a) << bit;
    return product;
}
inline __int128 vector_doubling_product(uint64_t a, uint64_t b, unsigned bits) {
    return static_cast<__int128>(sign_extend(a, bits)) * sign_extend(b, bits) * 2;
}
inline uint64_t vector_doubling_high(CpuState &cpu, uint64_t a, uint64_t b, unsigned bits, bool round) {
    const __int128 product = vector_doubling_product(a, b, bits);
    return vector_saturate(cpu, (product + (round ? static_cast<__int128>(1) << (bits - 1) : 0)) >> bits,
                           bits, false);
}
inline uint64_t vector_rounding_doubling_accumulate(CpuState &cpu, uint64_t a, uint64_t b, unsigned bits,
                                                   uint64_t accumulator, bool subtract) {
    const __int128 product = vector_doubling_product(a, b, bits);
    // RDM rounds and saturates the complete sum, including the untruncated product.
    const __int128 previous = static_cast<__int128>(sign_extend(accumulator, bits)) *
                              (static_cast<__int128>(1) << bits);
    const __int128 sum = subtract ? previous - product : previous + product;
    return vector_saturate(cpu, (sum + (static_cast<__int128>(1) << (bits - 1))) >> bits, bits, false);
}
inline uint64_t vector_doubling_long(CpuState &cpu, uint64_t a, uint64_t b, unsigned bits,
                                    uint64_t accumulator, bool accumulate, bool subtract) {
    // SQDMLAL/SQDMLSL saturate the doubled product before adding/subtracting it.
    // Both this saturation and the final accumulation contribute to FPSR.QC.
    const uint64_t saturated = vector_saturate(cpu, vector_doubling_product(a, b, bits), bits * 2, false);
    if (!accumulate)
        return saturated;
    const __int128 previous = sign_extend(accumulator, bits * 2), value = sign_extend(saturated, bits * 2);
    return vector_saturate(cpu, subtract ? previous - value : previous + value, bits * 2, false);
}
struct VectorIndexedElement {
    unsigned reg, lane;
    static VectorIndexedElement integer(uint32_t r, unsigned size) {
        // For halfwords M is a lane bit; for words it is the high register bit.
        // H selects the upper half of Vm independently of the destination's Q bit.
        return {(r >> 16) & (size == 2 ? 15u : 31u), size == 2 ? (((r >> 11) & 1) << 2) | ((r >> 20) & 3)
                                                               : (((r >> 11) & 1) << 1) | ((r >> 21) & 1)};
    }
};
} // namespace operit::arm64
