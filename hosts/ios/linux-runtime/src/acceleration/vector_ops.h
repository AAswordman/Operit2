#pragma once
#include "acceleration/scalar_ops.h"
#include <arm_neon.h>

namespace operit::arm64::vector {
template <unsigned Size> inline void load(CpuState &cpu, Memory &memory, const Instruction &i) {
    const auto location = scalar::address(cpu, i);
    std::array<uint8_t, 16> value{};
    memory.copy_out<Size>(location.value, value.data());
    cpu.v[i.rd] = value;
    scalar::writeback(cpu, i, location.updated);
}
template <unsigned Size> inline void store(CpuState &cpu, Memory &memory, const Instruction &i) {
    const auto location = scalar::address(cpu, i);
    memory.copy_in<Size>(location.value, cpu.v[i.rd].data());
    scalar::writeback(cpu, i, location.updated);
}
template <unsigned Size, bool Load> inline void pair(CpuState &cpu, Memory &memory, const Instruction &i) {
    const auto location = scalar::address(cpu, i);
    std::array<uint8_t, Size * 2> values;
    if constexpr (Load) {
        memory.copy_out<Size * 2>(location.value, values.data());
        std::array<uint8_t, 16> first{}, second{};
        std::memcpy(first.data(), values.data(), Size);
        std::memcpy(second.data(), values.data() + Size, Size);
        cpu.v[i.rd] = first;
        cpu.v[i.ra] = second;
    } else {
        std::memcpy(values.data(), cpu.v[i.rd].data(), Size);
        std::memcpy(values.data() + Size, cpu.v[i.ra].data(), Size);
        memory.copy_in<Size * 2>(location.value, values.data());
    }
    scalar::writeback(cpu, i, location.updated);
}
template <unsigned Size> inline void to_register(CpuState &cpu, const Instruction &i) {
    uint64_t value = 0;
    std::memcpy(&value, cpu.v[i.rn].data(), Size);
    cpu.set(i.rd, value);
}
template <unsigned Size> inline void from_register(CpuState &cpu, const Instruction &i) {
    const uint64_t value = cpu.get(i.rn);
    std::array<uint8_t, 16> result{};
    std::memcpy(result.data(), &value, Size);
    cpu.v[i.rd] = result;
}
inline void high_to_register(CpuState &cpu, const Instruction &i) {
    uint64_t value;
    std::memcpy(&value, cpu.v[i.rn].data() + 8, 8);
    cpu.set(i.rd, value);
}
inline void from_register_high(CpuState &cpu, const Instruction &i) {
    const uint64_t value = cpu.get(i.rn);
    std::memcpy(cpu.v[i.rd].data() + 8, &value, 8);
}
inline void immediate(CpuState &cpu, const Instruction &i) {
    std::array<uint8_t, 16> result{};
    std::memcpy(result.data(), &i.immediate, sizeof(i.immediate));
    cpu.v[i.rd] = result;
}
inline void logical(CpuState &cpu, const Instruction &i) {
    const auto a = vld1q_u8(cpu.v[i.rn].data()), b = vld1q_u8(cpu.v[i.rm].data());
    uint8x16_t result;
    switch (i.flags) {
    case 0:
        result = vandq_u8(a, b);
        break;
    case 1:
        result = vbicq_u8(a, b);
        break;
    case 2:
        result = vorrq_u8(a, b);
        break;
    case 3:
        result = vornq_u8(a, b);
        break;
    case 4:
        result = veorq_u8(a, b);
        break;
    case 5:
        result = vbslq_u8(vld1q_u8(cpu.v[i.rd].data()), a, b);
        break;
    case 6:
        result = vbslq_u8(b, a, vld1q_u8(cpu.v[i.rd].data()));
        break;
    default:
        result = vbslq_u8(b, vld1q_u8(cpu.v[i.rd].data()), a);
        break;
    }
    if (i.shift == 8)
        result = vcombine_u8(vget_low_u8(result), vdup_n_u8(0));
    vst1q_u8(cpu.v[i.rd].data(), result);
}
} // namespace operit::arm64::vector
