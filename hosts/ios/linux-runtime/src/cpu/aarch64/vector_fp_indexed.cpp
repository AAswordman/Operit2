#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/vector_floating.h"
#include <cstring>
namespace operit::arm64 {
bool execute_vector_floating_indexed(CpuState &cpu, uint32_t r) {
#if defined(__aarch64__)
    if ((r & 0x8f000400) != 0x0f000000)
        return false;
    const unsigned operation = (r >> 12) & 15, size = (r >> 22) & 3;
    const bool scalar = r & (1u << 28), quad = r & (1u << 30), u = r & (1u << 29);
    if ((operation != 1 && operation != 5 && operation != 9) || size == 1 || (u && operation != 9) ||
        (!quad && (scalar || size == 3)) || (size == 3 && (r & (1u << 21))))
        return false;
    if (size == 0 && !host_supports_half_floating_point())
        return false;
    const unsigned bytes = size == 3 ? 8 : size == 2 ? 4 : 2;
    // H:L:M select a half lane; M is a register bit for S/D. In particular,
    // the indexed half source is restricted to V0..V15, not V0..V31.
    const unsigned index = size == 3 ? (r >> 11) & 1
                           : size == 2 ? (((r >> 11) & 1) << 1) | ((r >> 21) & 1)
                                       : (((r >> 11) & 1) << 2) | ((r >> 20) & 3);
    const unsigned rm = (r >> 16) & (size == 0 ? 15 : 31);
    using Bits = unsigned char __attribute__((vector_size(16)));
    Bits result, a, b{};
    std::memcpy(&result, cpu.v[r & 31].data(), 16);
    std::memcpy(&a, cpu.v[(r >> 5) & 31].data(), 16);
    std::memcpy(&b, cpu.v[rm].data() + index * bytes, bytes);
    FloatingPointEnvironment environment(cpu);
#define INDEXED(instruction)                                                                                 \
    do {                                                                                                     \
        if (bytes == 2) {                                                                                    \
            if (scalar)                                                                                      \
                asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1, %2.h[0]"                     \
                             : "+w"(result) : "w"(a), "x"(b));                                               \
            else if (quad)                                                                                   \
                asm volatile(".arch_extension fp16\n\t" instruction " %0.8h, %1.8h, %2.h[0]"                 \
                             : "+w"(result) : "w"(a), "x"(b));                                               \
            else                                                                                             \
                asm volatile(".arch_extension fp16\n\t" instruction " %0.4h, %1.4h, %2.h[0]"                 \
                             : "+w"(result) : "w"(a), "x"(b));                                               \
        } else if (scalar) {                                                                                 \
            if (bytes == 8)                                                                                  \
                asm volatile(instruction " %d0, %d1, %2.d[0]" : "+w"(result) : "w"(a), "w"(b));              \
            else                                                                                             \
                asm volatile(instruction " %s0, %s1, %2.s[0]" : "+w"(result) : "w"(a), "w"(b));              \
        } else if (bytes == 8)                                                                               \
            asm volatile(instruction " %0.2d, %1.2d, %2.d[0]" : "+w"(result) : "w"(a), "w"(b));              \
        else if (quad)                                                                                       \
            asm volatile(instruction " %0.4s, %1.4s, %2.s[0]" : "+w"(result) : "w"(a), "w"(b));              \
        else                                                                                                 \
            asm volatile(instruction " %0.2s, %1.2s, %2.s[0]" : "+w"(result) : "w"(a), "w"(b));              \
    } while (false)
    if (operation == 1)
        INDEXED("fmla");
    else if (operation == 5)
        INDEXED("fmls");
    else if (u)
        INDEXED("fmulx");
    else
        INDEXED("fmul");
#undef INDEXED
    std::memcpy(cpu.v[r & 31].data(), &result, 16);
    return true;
#else
    (void)cpu;
    (void)r;
    return false;
#endif
}
} // namespace operit::arm64
