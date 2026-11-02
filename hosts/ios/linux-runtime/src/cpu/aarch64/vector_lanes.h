#pragma once
#include "cpu/aarch64/state.h"
#include <algorithm>
#include <cstring>
namespace operit::arm64 {
using Vector = std::array<uint8_t, 16>;
inline uint64_t vector_lane(const Vector &value, unsigned offset, unsigned size) {
    uint64_t result = 0;
    std::memcpy(&result, value.data() + offset, size);
    return result;
}
inline void vector_lane(Vector &value, unsigned offset, unsigned size, uint64_t element) {
    std::memcpy(value.data() + offset, &element, size);
}
inline uint64_t vector_saturate(CpuState &cpu, __int128 value, unsigned bits, bool unsigned_result) {
    const __int128 minimum = unsigned_result ? 0 : -(static_cast<__int128>(1) << (bits - 1));
    const __int128 maximum =
        unsigned_result ? static_cast<__int128>(mask(bits)) : (static_cast<__int128>(1) << (bits - 1)) - 1;
    if (value < minimum || value > maximum)
        cpu.fpsr |= uint64_t{1} << 27; // cumulative FPSR.QC
    return uint64_t(std::clamp(value, minimum, maximum));
}
bool execute_vector_arithmetic(CpuState &cpu, uint32_t raw);
bool execute_vector_unary(CpuState &cpu, uint32_t raw);
bool execute_vector_widen(CpuState &cpu, uint32_t raw);
bool execute_vector_reduction(CpuState &cpu, uint32_t raw);
} // namespace operit::arm64
