#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/vector_floating.h"
#include <array>
#include <cstring>
#include <utility>

namespace operit::arm64 {
#if defined(__aarch64__)
namespace {
using Bits = unsigned char __attribute__((vector_size(16)));
template <unsigned Fraction> void convert_fixed(CpuState &cpu, uint32_t raw, unsigned bits, bool to_integer) {
    Bits input, result;
    std::memcpy(&input, cpu.v[(raw >> 5) & 31].data(), 16);
    const bool scalar = raw & (1u << 28), quad = raw & (1u << 30), u = raw & (1u << 29);
#define CONVERT(instruction)                                                                                 \
    do {                                                                                                     \
        if (bits == 64) {                                                                                    \
            if (scalar)                                                                                      \
                asm volatile(instruction " %d0, %d1, #%c2" : "=w"(result) : "w"(input), "i"(Fraction));      \
            else                                                                                             \
                asm volatile(instruction " %0.2d, %1.2d, #%c2" : "=w"(result) : "w"(input), "i"(Fraction));  \
        } else if (bits == 32) {                                                                             \
            if constexpr (Fraction <= 32) {                                                                  \
                if (scalar)                                                                                  \
                    asm volatile(instruction " %s0, %s1, #%c2" : "=w"(result) : "w"(input), "i"(Fraction));  \
                else if (quad)                                                                               \
                    asm volatile(instruction " %0.4s, %1.4s, #%c2" : "=w"(result) : "w"(input), "i"(Fraction)); \
                else                                                                                         \
                    asm volatile(instruction " %0.2s, %1.2s, #%c2" : "=w"(result) : "w"(input), "i"(Fraction)); \
            }                                                                                                \
        } else if constexpr (Fraction <= 16) {                                                               \
            if (scalar)                                                                                      \
                asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1, #%c2"                        \
                             : "=w"(result) : "w"(input), "i"(Fraction));                                    \
            else if (quad)                                                                                   \
                asm volatile(".arch_extension fp16\n\t" instruction " %0.8h, %1.8h, #%c2"                    \
                             : "=w"(result) : "w"(input), "i"(Fraction));                                    \
            else                                                                                             \
                asm volatile(".arch_extension fp16\n\t" instruction " %0.4h, %1.4h, #%c2"                    \
                             : "=w"(result) : "w"(input), "i"(Fraction));                                    \
        }                                                                                                    \
    } while (false)
    if (to_integer) {
        if (u)
            CONVERT("fcvtzu");
        else
            CONVERT("fcvtzs");
    } else if (u)
        CONVERT("ucvtf");
    else
        CONVERT("scvtf");
#undef CONVERT
    std::memcpy(cpu.v[raw & 31].data(), &result, 16);
}
template <size_t... Fractions> constexpr auto conversions(std::index_sequence<Fractions...>) {
    return std::array{&convert_fixed<Fractions + 1>...};
}
constexpr auto fixed_conversions = conversions(std::make_index_sequence<64>{});
} // namespace
#endif
bool execute_vector_fixed(CpuState &cpu, uint32_t raw) {
#if defined(__aarch64__)
    if ((raw & 0x8f800400) != 0x0f000400)
        return false;
    const unsigned operation = (raw >> 11) & 31, immediate = (raw >> 16) & 127;
    if ((operation != 28 && operation != 31) || immediate < 16)
        return false;
    const unsigned bits = immediate >= 64 ? 64 : immediate >= 32 ? 32 : 16;
    const unsigned fraction = bits * 2 - immediate;
    const bool scalar = raw & (1u << 28), quad = raw & (1u << 30);
    if ((!quad && (scalar || bits == 64)) || (bits == 16 && !host_supports_half_floating_point()))
        return false;
    FloatingPointEnvironment environment(cpu);
    fixed_conversions[fraction - 1](cpu, raw, bits, operation == 31);
    return true;
#else
    (void)cpu;
    (void)raw;
    return false;
#endif
}
} // namespace operit::arm64
