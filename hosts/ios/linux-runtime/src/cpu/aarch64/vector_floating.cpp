#include "cpu/aarch64/vector_floating.h"
#include "cpu/aarch64/floating_point.h"
#include <cstring>
namespace operit::arm64 {
namespace {
#if defined(__aarch64__)
using Bits = unsigned char __attribute__((vector_size(16)));
bool execute_fp_size_conversion(CpuState &cpu, uint32_t r) {
    const uint32_t op = r & 0xaf3ffc00;
    const bool scalar = r & (1u << 28), quad = r & (1u << 30);
    const unsigned size = (r >> 22) & 3;
    const bool widen = op == 0x0e217800, narrow = op == 0x0e216800, odd = op == 0x2e216800;
    if ((!widen && !narrow && !odd) || size > 1 || (odd && size != 1))
        return false;
    if (scalar && (!quad || !odd))
        return false;
    Bits input, result;
    std::memcpy(&input, cpu.v[(r >> 5) & 31].data(), 16);
    std::memcpy(&result, cpu.v[r & 31].data(), 16);
    FloatingPointEnvironment environment(cpu);
    if (scalar)
        asm volatile("fcvtxn %s0, %d1" : "=w"(result) : "w"(input));
    else if (widen) {
        if (size == 1) {
            if (quad)
                asm volatile("fcvtl2 %0.2d, %1.4s" : "=w"(result) : "w"(input));
            else
                asm volatile("fcvtl %0.2d, %1.2s" : "=w"(result) : "w"(input));
        } else if (quad)
            asm volatile("fcvtl2 %0.4s, %1.8h" : "=w"(result) : "w"(input));
        else
            asm volatile("fcvtl %0.4s, %1.4h" : "=w"(result) : "w"(input));
    } else if (odd) {
        if (quad)
            asm volatile("fcvtxn2 %0.4s, %1.2d" : "+w"(result) : "w"(input));
        else
            asm volatile("fcvtxn %0.2s, %1.2d" : "=w"(result) : "w"(input));
    } else if (size == 1) {
        if (quad)
            asm volatile("fcvtn2 %0.4s, %1.2d" : "+w"(result) : "w"(input));
        else
            asm volatile("fcvtn %0.2s, %1.2d" : "=w"(result) : "w"(input));
    } else if (quad)
        asm volatile("fcvtn2 %0.8h, %1.4s" : "+w"(result) : "w"(input));
    else
        asm volatile("fcvtn %0.4h, %1.4s" : "=w"(result) : "w"(input));
    std::memcpy(cpu.v[r & 31].data(), &result, 16);
    return true;
}
bool execute_fp_pairwise(CpuState &cpu, uint32_t r) {
    Bits input, second, result;
    std::memcpy(&input, cpu.v[(r >> 5) & 31].data(), 16);
    std::memcpy(&second, cpu.v[(r >> 16) & 31].data(), 16);
    const bool wide = r & (1u << 22), quad = r & (1u << 30);
#define PAIRWISE(code, instruction)                                                                          \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %d0, %1.2d" : "=w"(result) : "w"(input));                             \
        else                                                                                                 \
            asm volatile(instruction " %s0, %1.2s" : "=w"(result) : "w"(input));                             \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
#define REDUCTION(code, instruction)                                                                         \
    case code: {                                                                                             \
        if (wide)                                                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        asm volatile(instruction " %s0, %1.4s" : "=w"(result) : "w"(input));                                 \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xffbffc00) {
        PAIRWISE(0x7e30d800, "faddp")
        PAIRWISE(0x7e30f800, "fmaxp")
        PAIRWISE(0x7eb0f800, "fminp")
        PAIRWISE(0x7e30c800, "fmaxnmp")
        PAIRWISE(0x7eb0c800, "fminnmp")
        REDUCTION(0x6e30f800, "fmaxv")
        REDUCTION(0x6eb0f800, "fminv")
        REDUCTION(0x6e30c800, "fmaxnmv")
        REDUCTION(0x6eb0c800, "fminnmv")
    default:
        break;
    }
#undef PAIRWISE
#undef REDUCTION
#define VECTOR_PAIRWISE(code, instruction)                                                                   \
    case code: {                                                                                             \
        if (wide && !quad)                                                                                   \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %0.2d, %1.2d, %2.2d" : "=w"(result) : "w"(input), "w"(second));       \
        else if (quad)                                                                                       \
            asm volatile(instruction " %0.4s, %1.4s, %2.4s" : "=w"(result) : "w"(input), "w"(second));       \
        else                                                                                                 \
            asm volatile(instruction " %0.2s, %1.2s, %2.2s" : "=w"(result) : "w"(input), "w"(second));       \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xbfa0fc00) {
        VECTOR_PAIRWISE(0x2e20d400, "faddp")
        VECTOR_PAIRWISE(0x2e20f400, "fmaxp")
        VECTOR_PAIRWISE(0x2ea0f400, "fminp")
        VECTOR_PAIRWISE(0x2e20c400, "fmaxnmp")
        VECTOR_PAIRWISE(0x2ea0c400, "fminnmp")
    default:
        break;
    }
#undef VECTOR_PAIRWISE
#define HALF_SCALAR_PAIRWISE(code, instruction)                                                              \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        asm volatile(".arch_extension fp16\n\t" instruction " %h0, %1.2h"                                    \
                     : "=w"(result) : "w"(input));                                                           \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xfffffc00) {
        HALF_SCALAR_PAIRWISE(0x5e30d800, "faddp")
        HALF_SCALAR_PAIRWISE(0x5e30f800, "fmaxp")
        HALF_SCALAR_PAIRWISE(0x5eb0f800, "fminp")
        HALF_SCALAR_PAIRWISE(0x5e30c800, "fmaxnmp")
        HALF_SCALAR_PAIRWISE(0x5eb0c800, "fminnmp")
    default:
        break;
    }
#undef HALF_SCALAR_PAIRWISE
#define HALF_REDUCTION(code, instruction)                                                                    \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (quad)                                                                                            \
            asm volatile(".arch_extension fp16\n\t" instruction " %h0, %1.8h"                                \
                         : "=w"(result) : "w"(input));                                                       \
        else                                                                                                 \
            asm volatile(".arch_extension fp16\n\t" instruction " %h0, %1.4h"                                \
                         : "=w"(result) : "w"(input));                                                       \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xbffffc00) {
        HALF_REDUCTION(0x0e30f800, "fmaxv")
        HALF_REDUCTION(0x0eb0f800, "fminv")
        HALF_REDUCTION(0x0e30c800, "fmaxnmv")
        HALF_REDUCTION(0x0eb0c800, "fminnmv")
    default:
        break;
    }
#undef HALF_REDUCTION
#define HALF_VECTOR_PAIRWISE(code, instruction)                                                              \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (quad)                                                                                            \
            asm volatile(".arch_extension fp16\n\t" instruction " %0.8h, %1.8h, %2.8h"                       \
                         : "=w"(result) : "w"(input), "w"(second));                                          \
        else                                                                                                 \
            asm volatile(".arch_extension fp16\n\t" instruction " %0.4h, %1.4h, %2.4h"                       \
                         : "=w"(result) : "w"(input), "w"(second));                                          \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xbfe0fc00) {
        HALF_VECTOR_PAIRWISE(0x2e401400, "faddp")
        HALF_VECTOR_PAIRWISE(0x2e403400, "fmaxp")
        HALF_VECTOR_PAIRWISE(0x2ec03400, "fminp")
        HALF_VECTOR_PAIRWISE(0x2e400400, "fmaxnmp")
        HALF_VECTOR_PAIRWISE(0x2ec00400, "fminnmp")
    default:
        return false;
    }
#undef HALF_VECTOR_PAIRWISE
}
bool execute_fp_scalar_simd(CpuState &cpu, uint32_t r) {
    if ((r & 0xdf000000) != 0x5e000000)
        return false;
    Bits result, a, b;
    std::memcpy(&a, cpu.v[(r >> 5) & 31].data(), 16);
    std::memcpy(&b, cpu.v[(r >> 16) & 31].data(), 16);
    const bool wide = r & (1u << 22);
#define SCALAR_BINARY(code, instruction)                                                                     \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %d0, %d1, %d2" : "=w"(result) : "w"(a), "w"(b));                      \
        else                                                                                                 \
            asm volatile(instruction " %s0, %s1, %s2" : "=w"(result) : "w"(a), "w"(b));                      \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xffa0fc00) {
        SCALAR_BINARY(0x5e20dc00, "fmulx")
        SCALAR_BINARY(0x7ea0d400, "fabd")
        SCALAR_BINARY(0x5e20e400, "fcmeq")
        SCALAR_BINARY(0x7e20e400, "fcmge")
        SCALAR_BINARY(0x7ea0e400, "fcmgt")
        SCALAR_BINARY(0x7e20ec00, "facge")
        SCALAR_BINARY(0x7ea0ec00, "facgt")
        SCALAR_BINARY(0x5e20fc00, "frecps")
        SCALAR_BINARY(0x5ea0fc00, "frsqrts")
    default:
        break;
    }
#undef SCALAR_BINARY
#define SCALAR_UNARY(code, instruction)                                                                      \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %d0, %d1" : "=w"(result) : "w"(a));                                   \
        else                                                                                                 \
            asm volatile(instruction " %s0, %s1" : "=w"(result) : "w"(a));                                   \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xffbffc00) {
        SCALAR_UNARY(0x5e21d800, "scvtf")
        SCALAR_UNARY(0x7e21d800, "ucvtf")
        SCALAR_UNARY(0x5e21a800, "fcvtns")
        SCALAR_UNARY(0x7e21a800, "fcvtnu")
        SCALAR_UNARY(0x5ea1a800, "fcvtps")
        SCALAR_UNARY(0x7ea1a800, "fcvtpu")
        SCALAR_UNARY(0x5e21b800, "fcvtms")
        SCALAR_UNARY(0x7e21b800, "fcvtmu")
        SCALAR_UNARY(0x5ea1b800, "fcvtzs")
        SCALAR_UNARY(0x7ea1b800, "fcvtzu")
        SCALAR_UNARY(0x5e21c800, "fcvtas")
        SCALAR_UNARY(0x7e21c800, "fcvtau")
        SCALAR_UNARY(0x5ea1d800, "frecpe")
        SCALAR_UNARY(0x7ea1d800, "frsqrte")
        SCALAR_UNARY(0x5ea1f800, "frecpx")
    default:
        break;
    }
#undef SCALAR_UNARY
#define SCALAR_ZERO(code, instruction)                                                                       \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %d0, %d1, #0.0" : "=w"(result) : "w"(a));                             \
        else                                                                                                 \
            asm volatile(instruction " %s0, %s1, #0.0" : "=w"(result) : "w"(a));                             \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xffbffc00) {
        SCALAR_ZERO(0x5ea0c800, "fcmgt")
        SCALAR_ZERO(0x7ea0c800, "fcmge")
        SCALAR_ZERO(0x5ea0d800, "fcmeq")
        SCALAR_ZERO(0x7ea0d800, "fcmle")
        SCALAR_ZERO(0x5ea0e800, "fcmlt")
    default:
        return false;
    }
#undef SCALAR_ZERO
}
bool execute_fp_half_scalar(CpuState &cpu, uint32_t r) {
    if ((r & 0xdf400000) != 0x5e400000)
        return false;
    Bits a, b, result;
    std::memcpy(&a, cpu.v[(r >> 5) & 31].data(), 16);
    std::memcpy(&b, cpu.v[(r >> 16) & 31].data(), 16);
#define HALF_BINARY(code, instruction)                                                                       \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1, %h2"                                 \
                     : "=w"(result) : "w"(a), "w"(b));                                                       \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xffe0fc00) {
        HALF_BINARY(0x5e401c00, "fmulx")
        HALF_BINARY(0x7ec01400, "fabd")
        HALF_BINARY(0x5e402400, "fcmeq")
        HALF_BINARY(0x7e402400, "fcmge")
        HALF_BINARY(0x7ec02400, "fcmgt")
        HALF_BINARY(0x7e402c00, "facge")
        HALF_BINARY(0x7ec02c00, "facgt")
        HALF_BINARY(0x5e403c00, "frecps")
        HALF_BINARY(0x5ec03c00, "frsqrts")
    default:
        break;
    }
#undef HALF_BINARY
#define HALF_UNARY(code, instruction)                                                                        \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1"                                      \
                     : "=w"(result) : "w"(a));                                                               \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xfffffc00) {
        HALF_UNARY(0x5e79d800, "scvtf")
        HALF_UNARY(0x7e79d800, "ucvtf")
        HALF_UNARY(0x5e79a800, "fcvtns")
        HALF_UNARY(0x7e79a800, "fcvtnu")
        HALF_UNARY(0x5ef9a800, "fcvtps")
        HALF_UNARY(0x7ef9a800, "fcvtpu")
        HALF_UNARY(0x5e79b800, "fcvtms")
        HALF_UNARY(0x7e79b800, "fcvtmu")
        HALF_UNARY(0x5ef9b800, "fcvtzs")
        HALF_UNARY(0x7ef9b800, "fcvtzu")
        HALF_UNARY(0x5e79c800, "fcvtas")
        HALF_UNARY(0x7e79c800, "fcvtau")
        HALF_UNARY(0x5ef9d800, "frecpe")
        HALF_UNARY(0x7ef9d800, "frsqrte")
        HALF_UNARY(0x5ef9f800, "frecpx")
    default:
        break;
    }
#undef HALF_UNARY
#define HALF_ZERO(code, instruction)                                                                         \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1, #0.0"                                \
                     : "=w"(result) : "w"(a));                                                               \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xfffffc00) {
        HALF_ZERO(0x5ef8c800, "fcmgt")
        HALF_ZERO(0x7ef8c800, "fcmge")
        HALF_ZERO(0x5ef8d800, "fcmeq")
        HALF_ZERO(0x7ef8d800, "fcmle")
        HALF_ZERO(0x5ef8e800, "fcmlt")
    default:
        return false;
    }
#undef HALF_ZERO
}
bool execute_fp_half_vector(CpuState &cpu, uint32_t r) {
    if ((r & 0x9f400000) != 0x0e400000)
        return false;
    const bool quad = r & (1u << 30);
    Bits a, b, result;
    std::memcpy(&a, cpu.v[(r >> 5) & 31].data(), 16);
    std::memcpy(&b, cpu.v[(r >> 16) & 31].data(), 16);
    std::memcpy(&result, cpu.v[r & 31].data(), 16);
#define HALF_BINARY(code, instruction)                                                                       \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (quad)                                                                                            \
            asm volatile(".arch_extension fp16\n\t" instruction " %0.8h, %1.8h, %2.8h"                       \
                         : "+w"(result) : "w"(a), "w"(b));                                                   \
        else                                                                                                 \
            asm volatile(".arch_extension fp16\n\t" instruction " %0.4h, %1.4h, %2.4h"                       \
                         : "+w"(result) : "w"(a), "w"(b));                                                   \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xbfe0fc00) {
        HALF_BINARY(0x0e401400, "fadd")
        HALF_BINARY(0x0ec01400, "fsub")
        HALF_BINARY(0x2e401c00, "fmul")
        HALF_BINARY(0x0e401c00, "fmulx")
        HALF_BINARY(0x2e403c00, "fdiv")
        HALF_BINARY(0x0e403400, "fmax")
        HALF_BINARY(0x0ec03400, "fmin")
        HALF_BINARY(0x0e400400, "fmaxnm")
        HALF_BINARY(0x0ec00400, "fminnm")
        HALF_BINARY(0x0e400c00, "fmla")
        HALF_BINARY(0x0ec00c00, "fmls")
        HALF_BINARY(0x2e402400, "fcmge")
        HALF_BINARY(0x2ec02400, "fcmgt")
        HALF_BINARY(0x0e402400, "fcmeq")
        HALF_BINARY(0x2e402c00, "facge")
        HALF_BINARY(0x2ec02c00, "facgt")
        HALF_BINARY(0x2ec01400, "fabd")
        HALF_BINARY(0x0e403c00, "frecps")
        HALF_BINARY(0x0ec03c00, "frsqrts")
    default:
        break;
    }
#undef HALF_BINARY
#define HALF_UNARY(code, instruction, suffix)                                                                \
    case code: {                                                                                             \
        if (!host_supports_half_floating_point())                                                            \
            return false;                                                                                    \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (quad)                                                                                            \
            asm volatile(".arch_extension fp16\n\t" instruction " %0.8h, %1.8h" suffix                       \
                         : "=w"(result) : "w"(a));                                                           \
        else                                                                                                 \
            asm volatile(".arch_extension fp16\n\t" instruction " %0.4h, %1.4h" suffix                       \
                         : "=w"(result) : "w"(a));                                                           \
        std::memcpy(cpu.v[r & 31].data(), &result, 16);                                                      \
        return true;                                                                                         \
    }
    switch (r & 0xbffffc00) {
        HALF_UNARY(0x0ef8f800, "fabs", "")
        HALF_UNARY(0x2ef8f800, "fneg", "")
        HALF_UNARY(0x2ef9f800, "fsqrt", "")
        HALF_UNARY(0x0e798800, "frintn", "")
        HALF_UNARY(0x0ef98800, "frintp", "")
        HALF_UNARY(0x0e799800, "frintm", "")
        HALF_UNARY(0x0ef99800, "frintz", "")
        HALF_UNARY(0x2e798800, "frinta", "")
        HALF_UNARY(0x2e799800, "frintx", "")
        HALF_UNARY(0x2ef99800, "frinti", "")
        HALF_UNARY(0x0e79d800, "scvtf", "")
        HALF_UNARY(0x2e79d800, "ucvtf", "")
        HALF_UNARY(0x0e79a800, "fcvtns", "")
        HALF_UNARY(0x2e79a800, "fcvtnu", "")
        HALF_UNARY(0x0ef9a800, "fcvtps", "")
        HALF_UNARY(0x2ef9a800, "fcvtpu", "")
        HALF_UNARY(0x0e79b800, "fcvtms", "")
        HALF_UNARY(0x2e79b800, "fcvtmu", "")
        HALF_UNARY(0x0ef9b800, "fcvtzs", "")
        HALF_UNARY(0x2ef9b800, "fcvtzu", "")
        HALF_UNARY(0x0e79c800, "fcvtas", "")
        HALF_UNARY(0x2e79c800, "fcvtau", "")
        HALF_UNARY(0x0ef9d800, "frecpe", "")
        HALF_UNARY(0x2ef9d800, "frsqrte", "")
        HALF_UNARY(0x0ef8c800, "fcmgt", ", #0.0")
        HALF_UNARY(0x2ef8c800, "fcmge", ", #0.0")
        HALF_UNARY(0x0ef8d800, "fcmeq", ", #0.0")
        HALF_UNARY(0x2ef8d800, "fcmle", ", #0.0")
        HALF_UNARY(0x0ef8e800, "fcmlt", ", #0.0")
    default:
        return false;
    }
#undef HALF_UNARY
}
#endif
} // namespace

bool execute_vector_floating_conversion(CpuState &cpu, uint32_t raw) {
#if defined(__aarch64__)
    return execute_fp_size_conversion(cpu, raw);
#else
    (void)cpu;
    (void)raw;
    return false;
#endif
}
bool execute_vector_floating_pairwise(CpuState &cpu, uint32_t raw) {
#if defined(__aarch64__)
    return execute_fp_pairwise(cpu, raw);
#else
    (void)cpu;
    (void)raw;
    return false;
#endif
}
bool execute_scalar_floating_simd(CpuState &cpu, uint32_t raw) {
#if defined(__aarch64__)
    return execute_fp_half_scalar(cpu, raw) || execute_fp_scalar_simd(cpu, raw);
#else
    (void)cpu;
    (void)raw;
    return false;
#endif
}
bool execute_vector_floating(CpuState &cpu, uint32_t r) {
#if defined(__aarch64__)
    if (execute_vector_fixed(cpu, r) || execute_vector_floating_indexed(cpu, r) ||
        execute_vector_floating_conversion(cpu, r) || execute_vector_floating_pairwise(cpu, r) ||
        execute_scalar_floating_simd(cpu, r) || execute_fp_half_vector(cpu, r))
        return true;
    // The instruction and every shape are assembled at build time. Guest register
    // contents are data only; host FP state is saved/restored for each operation.
    const unsigned rd = r & 31, rn = (r >> 5) & 31, rm = (r >> 16) & 31;
    const bool wide = r & (1u << 22), quad = r & (1u << 30);
    if (wide && !quad)
        return false;
    Bits result, a, b;
    std::memcpy(&result, cpu.v[rd].data(), 16);
    std::memcpy(&a, cpu.v[rn].data(), 16);
    std::memcpy(&b, cpu.v[rm].data(), 16);
#define BINARY(code, instruction)                                                                            \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %0.2d, %1.2d, %2.2d" : "+w"(result) : "w"(a), "w"(b));                \
        else if (quad)                                                                                       \
            asm volatile(instruction " %0.4s, %1.4s, %2.4s" : "+w"(result) : "w"(a), "w"(b));                \
        else                                                                                                 \
            asm volatile(instruction " %0.2s, %1.2s, %2.2s" : "+w"(result) : "w"(a), "w"(b));                \
        std::memcpy(cpu.v[rd].data(), &result, 16);                                                          \
        return true;                                                                                         \
    }
    switch (r & 0xbfa0fc00) {
        BINARY(0x0e20d400, "fadd")
        BINARY(0x0ea0d400, "fsub")
        BINARY(0x2e20dc00, "fmul")
        BINARY(0x0e20dc00, "fmulx")
        BINARY(0x2e20fc00, "fdiv")
        BINARY(0x0e20f400, "fmax")
        BINARY(0x0ea0f400, "fmin")
        BINARY(0x0e20c400, "fmaxnm")
        BINARY(0x0ea0c400, "fminnm")
        BINARY(0x0e20cc00, "fmla")
        BINARY(0x0ea0cc00, "fmls")
        BINARY(0x2e20e400, "fcmge")
        BINARY(0x2ea0e400, "fcmgt")
        BINARY(0x0e20e400, "fcmeq")
        BINARY(0x2e20ec00, "facge")
        BINARY(0x2ea0ec00, "facgt")
        BINARY(0x2ea0d400, "fabd")
        BINARY(0x0e20fc00, "frecps")
        BINARY(0x0ea0fc00, "frsqrts")
    default:
        break;
    }
#undef BINARY
#define UNARY(code, instruction)                                                                             \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %0.2d, %1.2d" : "=w"(result) : "w"(a));                               \
        else if (quad)                                                                                       \
            asm volatile(instruction " %0.4s, %1.4s" : "=w"(result) : "w"(a));                               \
        else                                                                                                 \
            asm volatile(instruction " %0.2s, %1.2s" : "=w"(result) : "w"(a));                               \
        std::memcpy(cpu.v[rd].data(), &result, 16);                                                          \
        return true;                                                                                         \
    }
    const uint32_t unary_op = r & 0xbfbffc00;
    if (unary_op == 0x0ea1c800 || unary_op == 0x2ea1c800) {
        if (wide)
            return false;
        FloatingPointEnvironment environment(cpu);
        if (unary_op == 0x0ea1c800) {
            if (quad)
                asm volatile("urecpe %0.4s, %1.4s" : "=w"(result) : "w"(a));
            else
                asm volatile("urecpe %0.2s, %1.2s" : "=w"(result) : "w"(a));
        } else if (quad)
            asm volatile("ursqrte %0.4s, %1.4s" : "=w"(result) : "w"(a));
        else
            asm volatile("ursqrte %0.2s, %1.2s" : "=w"(result) : "w"(a));
        std::memcpy(cpu.v[rd].data(), &result, 16);
        return true;
    }
    switch (unary_op) {
        UNARY(0x0ea0f800, "fabs")
        UNARY(0x2ea0f800, "fneg")
        UNARY(0x2ea1f800, "fsqrt")
        UNARY(0x0e218800, "frintn")
        UNARY(0x0ea18800, "frintp")
        UNARY(0x0e219800, "frintm")
        UNARY(0x0ea19800, "frintz")
        UNARY(0x2e218800, "frinta")
        UNARY(0x2e219800, "frintx")
        UNARY(0x2ea19800, "frinti")
        UNARY(0x0e21d800, "scvtf")
        UNARY(0x2e21d800, "ucvtf")
        UNARY(0x0e21a800, "fcvtns")
        UNARY(0x2e21a800, "fcvtnu")
        UNARY(0x0ea1a800, "fcvtps")
        UNARY(0x2ea1a800, "fcvtpu")
        UNARY(0x0e21b800, "fcvtms")
        UNARY(0x2e21b800, "fcvtmu")
        UNARY(0x0e21c800, "fcvtas")
        UNARY(0x2e21c800, "fcvtau")
        UNARY(0x0ea1b800, "fcvtzs")
        UNARY(0x2ea1b800, "fcvtzu")
        UNARY(0x0ea1d800, "frecpe")
        UNARY(0x2ea1d800, "frsqrte")
    default:
        break;
    }
#undef UNARY
#define ZERO(code, instruction)                                                                              \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        if (wide)                                                                                            \
            asm volatile(instruction " %0.2d, %1.2d, #0.0" : "=w"(result) : "w"(a));                         \
        else if (quad)                                                                                       \
            asm volatile(instruction " %0.4s, %1.4s, #0.0" : "=w"(result) : "w"(a));                         \
        else                                                                                                 \
            asm volatile(instruction " %0.2s, %1.2s, #0.0" : "=w"(result) : "w"(a));                         \
        std::memcpy(cpu.v[rd].data(), &result, 16);                                                          \
        return true;                                                                                         \
    }
    switch (r & 0xbfbffc00) {
        ZERO(0x0ea0c800, "fcmgt")
        ZERO(0x2ea0c800, "fcmge")
        ZERO(0x0ea0d800, "fcmeq")
        ZERO(0x2ea0d800, "fcmle")
        ZERO(0x0ea0e800, "fcmlt")
    default:
        return false;
    }
#undef ZERO
#else
    (void)cpu;
    (void)r;
    return false;
#endif
}
} // namespace operit::arm64
