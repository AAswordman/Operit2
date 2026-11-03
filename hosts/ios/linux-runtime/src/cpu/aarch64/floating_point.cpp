#include "cpu/aarch64/floating_point.h"
#include "cpu/aarch64/fixed_conversion.h"
#include "cpu/aarch64/vector_floating.h"
#include <cstring>
#if defined(__aarch64__) && defined(__APPLE__)
#include <sys/sysctl.h>
#elif defined(__aarch64__) && defined(__linux__)
#include <sys/auxv.h>
#endif

namespace operit::arm64 {
uint64_t read_floating_status(const CpuState &cpu) noexcept {
    return cpu.fpsr & floating_status_mask;
}
void write_floating_control(CpuState &cpu, uint64_t value) noexcept {
    cpu.fpcr = value & floating_control_mask;
}
void write_floating_status(CpuState &cpu, uint64_t value) noexcept {
    cpu.fpsr = value & floating_status_mask;
}
bool host_supports_half_floating_point() noexcept {
#if defined(__aarch64__) && defined(__ARM_FEATURE_FP16_SCALAR_ARITHMETIC) &&                                 \
    defined(__ARM_FEATURE_FP16_VECTOR_ARITHMETIC)
    return true;
#elif defined(__aarch64__) && defined(__APPLE__)
    static const bool supported = [] {
        int enabled = 0;
        size_t size = sizeof(enabled);
        if (sysctlbyname("hw.optional.arm.FEAT_FP16", &enabled, &size, nullptr, 0) == 0)
            return enabled != 0;
        size = sizeof(enabled);
        return sysctlbyname("hw.optional.armv8_2_fp16", &enabled, &size, nullptr, 0) == 0 && enabled != 0;
    }();
    return supported;
#elif defined(__aarch64__) && defined(__linux__)
    // Linux HWCAP_FPHP and HWCAP_ASIMDHP. Both scalar and vector helpers use H.
    static const bool supported = (getauxval(16 /* AT_HWCAP */) & (3ul << 9)) == (3ul << 9);
    return supported;
#else
    return false;
#endif
}
#if defined(__aarch64__)
namespace {
using Bits = unsigned char __attribute__((vector_size(16)));
void write_scalar(CpuState &cpu, unsigned rd, const Bits &value, unsigned type) {
    cpu.v[rd].fill(0);
    std::memcpy(cpu.v[rd].data(), &value, type == 3 ? 2 : type == 1 ? 8 : 4);
}
bool size_conversion(CpuState &cpu, uint32_t raw) {
    Bits input, result;
    std::memcpy(&input, cpu.v[(raw >> 5) & 31].data(), 16);
#define SIZE_CONVERSION(code, instruction, type)                                                             \
    case code: {                                                                                             \
        FloatingPointEnvironment environment(cpu);                                                           \
        asm volatile(instruction : "=w"(result) : "w"(input));                                               \
        write_scalar(cpu, raw & 31, result, type);                                                           \
        return true;                                                                                         \
    }
    // H<->S/D FCVT belongs to base ARMv8, independently of FEAT_FP16 arithmetic.
    switch (raw & 0xfffffc00) {
        SIZE_CONVERSION(0x1e624000, "fcvt %s0, %d1", 0)
        SIZE_CONVERSION(0x1e22c000, "fcvt %d0, %s1", 1)
        SIZE_CONVERSION(0x1e23c000, "fcvt %h0, %s1", 3)
        SIZE_CONVERSION(0x1e63c000, "fcvt %h0, %d1", 3)
        SIZE_CONVERSION(0x1ee24000, "fcvt %s0, %h1", 0)
        SIZE_CONVERSION(0x1ee2c000, "fcvt %d0, %h1", 1)
    default:
        return false;
    }
#undef SIZE_CONVERSION
}
bool scalar(CpuState &cpu, uint32_t r, unsigned type) {
    const unsigned rd = r & 31, rn = (r >> 5) & 31, rm = (r >> 16) & 31;
    Bits a, b, result;
    std::memcpy(&a, cpu.v[rn].data(), 16);
    std::memcpy(&b, cpu.v[rm].data(), 16);
    FloatingPointEnvironment environment(cpu);
#define UNARY(instruction)                                                                                   \
    do {                                                                                                     \
        if (type == 3)                                                                                       \
            asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1"                                  \
                         : "=w"(result) : "w"(a));                                                           \
        else if (type == 1)                                                                                  \
            asm volatile(instruction " %d0, %d1" : "=w"(result) : "w"(a));                                   \
        else                                                                                                 \
            asm volatile(instruction " %s0, %s1" : "=w"(result) : "w"(a));                                   \
    } while (false)
#define BINARY(instruction)                                                                                  \
    do {                                                                                                     \
        if (type == 3)                                                                                       \
            asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1, %h2"                             \
                         : "=w"(result) : "w"(a), "w"(b));                                                   \
        else if (type == 1)                                                                                  \
            asm volatile(instruction " %d0, %d1, %d2" : "=w"(result) : "w"(a), "w"(b));                      \
        else                                                                                                 \
            asm volatile(instruction " %s0, %s1, %s2" : "=w"(result) : "w"(a), "w"(b));                      \
    } while (false)
    if ((r & 0xff200c00) == 0x1e200800) {
        switch ((r >> 12) & 15) {
        case 0:
            BINARY("fmul");
            break;
        case 1:
            BINARY("fdiv");
            break;
        case 2:
            BINARY("fadd");
            break;
        case 3:
            BINARY("fsub");
            break;
        case 4:
            BINARY("fmax");
            break;
        case 5:
            BINARY("fmin");
            break;
        case 6:
            BINARY("fmaxnm");
            break;
        case 7:
            BINARY("fminnm");
            break;
        case 8:
            BINARY("fnmul");
            break;
        default:
            return false;
        }
        write_scalar(cpu, rd, result, type);
        return true;
    }
#undef BINARY
    bool unary = true;
    switch (r & 0xff3ffc00) {
    case 0x1e204000:
        UNARY("fmov");
        break;
    case 0x1e20c000:
        UNARY("fabs");
        break;
    case 0x1e214000:
        UNARY("fneg");
        break;
    case 0x1e21c000:
        UNARY("fsqrt");
        break;
    case 0x1e244000:
        UNARY("frintn");
        break;
    case 0x1e24c000:
        UNARY("frintp");
        break;
    case 0x1e254000:
        UNARY("frintm");
        break;
    case 0x1e25c000:
        UNARY("frintz");
        break;
    case 0x1e264000:
        UNARY("frinta");
        break;
    case 0x1e274000:
        UNARY("frintx");
        break;
    case 0x1e27c000:
        UNARY("frinti");
        break;
    default:
        unary = false;
        break;
    }
#undef UNARY
    if (unary) {
        write_scalar(cpu, rd, result, type);
        return true;
    }

    const bool wide = r & (1u << 31);
    uint64_t integer = cpu.get(rn, wide ? 64 : 32);
#define INTEGER_SHAPE(instruction, shape, to_float)                                                          \
    do {                                                                                                     \
        if constexpr (to_float) {                                                                            \
            if (wide)                                                                                        \
                asm volatile(instruction " " shape "0, %x1" : "=w"(result) : "r"(integer));                  \
            else                                                                                             \
                asm volatile(instruction " " shape "0, %w1" : "=w"(result) : "r"(integer));                  \
        } else {                                                                                             \
            if (wide)                                                                                        \
                asm volatile(instruction " %x0, " shape "1" : "=r"(integer) : "w"(a));                       \
            else                                                                                             \
                asm volatile(instruction " %w0, " shape "1" : "=r"(integer) : "w"(a));                       \
        }                                                                                                    \
    } while (false)
#define INTEGER(instruction, to_float)                                                                       \
    do {                                                                                                     \
        if (type == 3)                                                                                       \
            INTEGER_SHAPE(".arch_extension fp16\n\t" instruction, "%h", to_float);                           \
        else if (type == 1)                                                                                  \
            INTEGER_SHAPE(instruction, "%d", to_float);                                                      \
        else                                                                                                 \
            INTEGER_SHAPE(instruction, "%s", to_float);                                                      \
        if constexpr (to_float)                                                                              \
            write_scalar(cpu, rd, result, type);                                                             \
        else                                                                                                 \
            cpu.set(rd, integer, wide ? 64 : 32);                                                            \
        return true;                                                                                         \
    } while (false)
    switch (r & 0x7f3ffc00) {
    case 0x1e220000:
        INTEGER("scvtf", true);
    case 0x1e230000:
        INTEGER("ucvtf", true);
    case 0x1e200000:
        INTEGER("fcvtns", false);
    case 0x1e210000:
        INTEGER("fcvtnu", false);
    case 0x1e240000:
        INTEGER("fcvtas", false);
    case 0x1e250000:
        INTEGER("fcvtau", false);
    case 0x1e280000:
        INTEGER("fcvtps", false);
    case 0x1e290000:
        INTEGER("fcvtpu", false);
    case 0x1e300000:
        INTEGER("fcvtms", false);
    case 0x1e310000:
        INTEGER("fcvtmu", false);
    case 0x1e380000:
        INTEGER("fcvtzs", false);
    case 0x1e390000:
        INTEGER("fcvtzu", false);
    default:
        break;
    }
#undef INTEGER_SHAPE
#undef INTEGER
    const bool conditional_compare = (r & 0xff200c00) == 0x1e200400;
    if ((r & 0xff20fc07) == 0x1e202000 || conditional_compare) {
        // The compare-to-zero encoding reserves Rm == 0.
        if (!conditional_compare && (r & 8) && rm != 0)
            return false;
        if (conditional_compare && !cpu.condition((r >> 12) & 15)) {
            cpu.nzcv = r & 15;
            return true;
        }
        if (!conditional_compare && (r & 8))
            b = Bits{};
        uint64_t flags;
#define COMPARE(instruction)                                                                                 \
        do {                                                                                                 \
            if (type == 3)                                                                                   \
                asm volatile(".arch_extension fp16\n\t" instruction " %h1, %h2\n\tmrs %0, nzcv"              \
                             : "=r"(flags) : "w"(a), "w"(b) : "cc");                                         \
            else if (type == 1)                                                                              \
                asm volatile(instruction " %d1, %d2\n\tmrs %0, nzcv"                                         \
                             : "=r"(flags) : "w"(a), "w"(b) : "cc");                                         \
            else                                                                                             \
                asm volatile(instruction " %s1, %s2\n\tmrs %0, nzcv"                                         \
                             : "=r"(flags) : "w"(a), "w"(b) : "cc");                                         \
        } while (false)
        if (r & 16)
            COMPARE("fcmpe");
        else
            COMPARE("fcmp");
#undef COMPARE
        cpu.nzcv = (flags >> 28) & 15;
        return true;
    }
    if ((r & 0xff200c00) == 0x1e200c00) {
        write_scalar(cpu, rd, cpu.condition((r >> 12) & 15) ? a : b, type);
        return true;
    }
    if ((r & 0xff000000) == 0x1f000000) {
        Bits c;
        std::memcpy(&c, cpu.v[(r >> 10) & 31].data(), 16);
#define FUSED(instruction)                                                                                   \
        do {                                                                                                 \
            if (type == 3)                                                                                   \
                asm volatile(".arch_extension fp16\n\t" instruction " %h0, %h1, %h2, %h3"                    \
                             : "=w"(result) : "w"(a), "w"(b), "w"(c));                                       \
            else if (type == 1)                                                                              \
                asm volatile(instruction " %d0, %d1, %d2, %d3"                                               \
                             : "=w"(result) : "w"(a), "w"(b), "w"(c));                                       \
            else                                                                                             \
                asm volatile(instruction " %s0, %s1, %s2, %s3"                                               \
                             : "=w"(result) : "w"(a), "w"(b), "w"(c));                                       \
        } while (false)
        switch (((r >> 20) & 2) | ((r >> 15) & 1)) {
        case 0:
            FUSED("fmadd");
            break;
        case 1:
            FUSED("fmsub");
            break;
        case 2:
            FUSED("fnmadd");
            break;
        case 3:
            FUSED("fnmsub");
            break;
        }
#undef FUSED
        write_scalar(cpu, rd, result, type);
        return true;
    }
    return false;
}
// FMOV is a bit transfer; even a signalling NaN is moved without IOC or DN/FZ.
bool scalar_move(CpuState &cpu, uint32_t raw, unsigned type) {
    const unsigned rn = (raw >> 5) & 31, rd = raw & 31;
    const uint32_t move = raw & 0xfffffc00;
    unsigned bytes = 0;
    bool from_integer = false, high = false;
    switch (move) {
    case 0x1e260000:
        bytes = 4;
        break;
    case 0x1e270000:
        bytes = 4;
        from_integer = true;
        break;
    case 0x9e660000:
        bytes = 8;
        break;
    case 0x9e670000:
        bytes = 8;
        from_integer = true;
        break;
    case 0x1ee60000:
    case 0x9ee60000:
        bytes = 2;
        break;
    case 0x1ee70000:
    case 0x9ee70000:
        bytes = 2;
        from_integer = true;
        break;
    case 0x9eae0000:
        bytes = 8;
        high = true;
        break;
    case 0x9eaf0000:
        bytes = 8;
        high = true;
        from_integer = true;
        break;
    default:
        break;
    }
    if (bytes) {
        if (from_integer) {
            const uint64_t value = cpu.get(rn);
            if (!high)
                cpu.v[rd].fill(0);
            std::memcpy(cpu.v[rd].data() + (high ? 8 : 0), &value, bytes);
        } else {
            uint64_t value = 0;
            std::memcpy(&value, cpu.v[rn].data() + (high ? 8 : 0), bytes);
            cpu.set(rd, value, raw >> 31 ? 64 : 32);
        }
        return true;
    }
    if ((raw & 0xff201fe0) != 0x1e201000 || type == 2)
        return false;
    const unsigned width = type == 3 ? 16 : type == 1 ? 64 : 32;
    const unsigned exponent_bits = type == 3 ? 5 : type == 1 ? 11 : 8;
    const unsigned fraction_bits = width - exponent_bits - 1;
    const uint64_t immediate = (raw >> 13) & 255, repeated = (immediate >> 6) & 1;
    const uint64_t exponent = ((repeated ^ 1) << (exponent_bits - 1)) |
                              ((repeated ? mask(exponent_bits - 3) : 0) << 2) |
                              ((immediate >> 4) & 3);
    const uint64_t value = ((immediate >> 7) << (width - 1)) | (exponent << fraction_bits) |
                           ((immediate & 15) << (fraction_bits - 4));
    cpu.v[rd].fill(0);
    std::memcpy(cpu.v[rd].data(), &value, width / 8);
    return true;
}
} // namespace
#endif
bool execute_floating_point(CpuState &cpu, uint32_t r) {
    if (execute_fixed_conversion(cpu, r) || execute_scalar_floating_simd(cpu, r))
        return true;
#if defined(__aarch64__)
    if ((r & 0x5f000000) != 0x1e000000 && (r & 0x5f000000) != 0x1f000000)
        return false;
    if (size_conversion(cpu, r))
        return true;
    const unsigned type = (r >> 22) & 3;
    // The high-lane FMOV encodings use ftype == 2; handle their exact words
    // before rejecting that reserved type for arithmetic.
    if (scalar_move(cpu, r, type))
        return true;
    if (type == 2 || (type == 3 && !host_supports_half_floating_point()))
        return false;
    return scalar(cpu, r, type);
#else
    (void)cpu;
    (void)r;
    return false;
#endif
}
} // namespace operit::arm64
