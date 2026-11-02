#include "cpu/aarch64/fixed_conversion.h"
#include "cpu/aarch64/floating_point.h"
#include <array>
#include <cstring>
#include <utility>

namespace operit::arm64 {
#if defined(__aarch64__)
namespace {
using Bits = unsigned char __attribute__((vector_size(16)));
// The immediate is a build-time constant in every helper. No guest instruction
// bytes become executable code, including for FP16 and the #64 boundary.
template <unsigned Fraction>
void convert(CpuState &cpu, uint32_t raw, unsigned operation, unsigned type) {
    const unsigned rn = (raw >> 5) & 31, rd = raw & 31;
    const bool wide = raw >> 31;
    uint64_t integer = cpu.get(rn, wide ? 64 : 32);
    Bits real{};
    if (operation >= 24)
        std::memcpy(&real, cpu.v[rn].data(), 16);
#define TO_FLOAT_SHAPE(instruction, shape)                                                                   \
    do {                                                                                                     \
        if (wide)                                                                                            \
            asm volatile(instruction " " shape "0, %x1, #%c2"                                                \
                         : "=w"(real) : "r"(integer), "i"(Fraction));                                        \
        else if constexpr (Fraction <= 32)                                                                   \
            asm volatile(instruction " " shape "0, %w1, #%c2"                                                \
                         : "=w"(real) : "r"(integer), "i"(Fraction));                                        \
    } while (false)
#define TO_FLOAT(instruction)                                                                                \
    do {                                                                                                     \
        if (type == 3)                                                                                       \
            TO_FLOAT_SHAPE(".arch_extension fp16\n\t" instruction, "%h");                                    \
        else if (type == 1)                                                                                  \
            TO_FLOAT_SHAPE(instruction, "%d");                                                               \
        else                                                                                                 \
            TO_FLOAT_SHAPE(instruction, "%s");                                                               \
    } while (false)
#define TO_INTEGER_SHAPE(instruction, shape)                                                                 \
    do {                                                                                                     \
        if (wide)                                                                                            \
            asm volatile(instruction " %x0, " shape "1, #%c2"                                                \
                         : "=r"(integer) : "w"(real), "i"(Fraction));                                        \
        else if constexpr (Fraction <= 32)                                                                   \
            asm volatile(instruction " %w0, " shape "1, #%c2"                                                \
                         : "=r"(integer) : "w"(real), "i"(Fraction));                                        \
    } while (false)
#define TO_INTEGER(instruction)                                                                              \
    do {                                                                                                     \
        if (type == 3)                                                                                       \
            TO_INTEGER_SHAPE(".arch_extension fp16\n\t" instruction, "%h");                                  \
        else if (type == 1)                                                                                  \
            TO_INTEGER_SHAPE(instruction, "%d");                                                             \
        else                                                                                                 \
            TO_INTEGER_SHAPE(instruction, "%s");                                                             \
    } while (false)
    switch (operation) {
    case 2:
        TO_FLOAT("scvtf");
        break;
    case 3:
        TO_FLOAT("ucvtf");
        break;
    case 24:
        TO_INTEGER("fcvtzs");
        break;
    case 25:
        TO_INTEGER("fcvtzu");
        break;
    }
#undef TO_FLOAT_SHAPE
#undef TO_FLOAT
#undef TO_INTEGER_SHAPE
#undef TO_INTEGER
    if (operation < 4) {
        // A scalar SIMD register write clears every bit above H/S/D.
        cpu.v[rd].fill(0);
        const unsigned bytes = type == 3 ? 2 : type == 1 ? 8 : 4;
        std::memcpy(cpu.v[rd].data(), &real, bytes);
    } else
        cpu.set(rd, integer, wide ? 64 : 32);
}
template <size_t... Fractions> constexpr auto conversions(std::index_sequence<Fractions...>) {
    return std::array{&convert<Fractions + 1>...};
}
constexpr auto fixed_conversions = conversions(std::make_index_sequence<64>{});
} // namespace
#endif
bool execute_fixed_conversion(CpuState &cpu, uint32_t raw) {
#if defined(__aarch64__)
    // sf and ftype vary; S == 0 and every fixed opcode bit must still match.
    if ((raw & 0x7f200000) != 0x1e000000)
        return false;
    const unsigned operation = (raw >> 16) & 31, type = (raw >> 22) & 3;
    if ((operation != 2 && operation != 3 && operation != 24 && operation != 25) || type == 2 ||
        (type == 3 && !host_supports_half_floating_point()))
        return false;
    const bool wide = raw >> 31;
    const unsigned fraction = 64 - ((raw >> 10) & 63);
    if (!wide && fraction > 32)
        return false;
    FloatingPointEnvironment environment(cpu);
    fixed_conversions[fraction - 1](cpu, raw, operation, type);
    return true;
#else
    (void)cpu;
    (void)raw;
    return false;
#endif
}
} // namespace operit::arm64
