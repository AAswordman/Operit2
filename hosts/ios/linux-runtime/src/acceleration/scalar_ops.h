#pragma once
#include "acceleration/conditions.h"
#include "cpu/aarch64/decoder.h"
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
#include <type_traits>

namespace operit::arm64::scalar {
template <unsigned Bits> using Word = std::conditional_t<Bits == 64, uint64_t, uint32_t>;

template <unsigned Bits> inline uint64_t shift(uint64_t value, unsigned type, unsigned amount) {
    const Word<Bits> input = value;
    switch (type) {
    case 0:
        return Word<Bits>(input << amount);
    case 1:
        return input >> amount;
    case 2:
        return Word<Bits>(static_cast<std::make_signed_t<Word<Bits>>>(input) >> amount);
    default:
        return amount ? Word<Bits>((input >> amount) | (input << (Bits - amount))) : input;
    }
}

template <unsigned Bits, bool Carry = false>
inline uint64_t arithmetic(CpuState &cpu, uint64_t lhs, uint64_t rhs, unsigned flags) {
    const Word<Bits> a = lhs, b = rhs;
    const bool subtract = flags & decoded::subtract;
    if (!(flags & decoded::set_flags)) {
        if constexpr (Carry)
            return Word<Bits>(a + (subtract ? Word<Bits>(~b) : b) + ((cpu.nzcv >> 1) & 1));
        return subtract ? Word<Bits>(a - b) : Word<Bits>(a + b);
    }
    Word<Bits> result;
    uint64_t status;
    if constexpr (Carry) {
        const uint64_t carry = uint64_t(cpu.nzcv & 2) << 28;
        if constexpr (Bits == 64) {
            if (subtract)
                asm("msr nzcv, %2\n sbcs %0, %3, %4\n mrs %1, nzcv"
                    : "=&r"(result), "=r"(status)
                    : "r"(carry), "r"(a), "r"(b)
                    : "cc");
            else
                asm("msr nzcv, %2\n adcs %0, %3, %4\n mrs %1, nzcv"
                    : "=&r"(result), "=r"(status)
                    : "r"(carry), "r"(a), "r"(b)
                    : "cc");
        } else {
            if (subtract)
                asm("msr nzcv, %2\n sbcs %w0, %w3, %w4\n mrs %1, nzcv"
                    : "=&r"(result), "=r"(status)
                    : "r"(carry), "r"(a), "r"(b)
                    : "cc");
            else
                asm("msr nzcv, %2\n adcs %w0, %w3, %w4\n mrs %1, nzcv"
                    : "=&r"(result), "=r"(status)
                    : "r"(carry), "r"(a), "r"(b)
                    : "cc");
        }
    } else if constexpr (Bits == 64) {
        if (subtract)
            asm("subs %0, %2, %3\n mrs %1, nzcv" : "=r"(result), "=r"(status) : "r"(a), "r"(b) : "cc");
        else
            asm("adds %0, %2, %3\n mrs %1, nzcv" : "=r"(result), "=r"(status) : "r"(a), "r"(b) : "cc");
    } else {
        if (subtract)
            asm("subs %w0, %w2, %w3\n mrs %1, nzcv" : "=r"(result), "=r"(status) : "r"(a), "r"(b) : "cc");
        else
            asm("adds %w0, %w2, %w3\n mrs %1, nzcv" : "=r"(result), "=r"(status) : "r"(a), "r"(b) : "cc");
    }
    cpu.nzcv = status >> 28;
    return result;
}

template <unsigned Bits> inline void add_immediate(CpuState &cpu, const Instruction &i) {
    const auto value = arithmetic<Bits>(cpu, cpu.get(i.rn, Bits, true), i.immediate, i.flags);
    cpu.set(i.rd, value, Bits, !(i.flags & decoded::set_flags));
}
template <unsigned Bits> inline void add_register(CpuState &cpu, const Instruction &i) {
    uint64_t rhs;
    if (i.flags & decoded::extended) {
        rhs = cpu.get(i.rm) & i.auxiliary;
        if (i.flags & decoded::signed_extension)
            rhs = sign_extend(rhs, i.immediate);
        rhs <<= i.shift;
    } else
        rhs = shift<Bits>(cpu.get(i.rm, Bits), (i.flags >> decoded::shift_type) & 3, i.shift);
    const bool stack = i.flags & decoded::extended;
    const auto value = arithmetic<Bits>(cpu, cpu.get(i.rn, Bits, stack), rhs, i.flags);
    cpu.set(i.rd, value, Bits, stack && !(i.flags & decoded::set_flags));
}
template <unsigned Bits> inline void add_carry(CpuState &cpu, const Instruction &i) {
    cpu.set(i.rd, arithmetic<Bits, true>(cpu, cpu.get(i.rn, Bits), cpu.get(i.rm, Bits), i.flags), Bits);
}
template <unsigned Bits, bool Immediate> inline void logical(CpuState &cpu, const Instruction &i) {
    uint64_t rhs;
    if constexpr (Immediate)
        rhs = i.immediate;
    else {
        rhs = shift<Bits>(cpu.get(i.rm, Bits), (i.flags >> 2) & 3, i.shift);
        if (i.flags & 16)
            rhs = ~rhs;
    }
    const uint64_t lhs = cpu.get(i.rn, Bits);
    const unsigned op = i.flags & 3;
    const Word<Bits> result = op == 1 ? lhs | rhs : (op == 2 ? lhs ^ rhs : lhs & rhs);
    if (op == 3)
        cpu.nzcv = (result >> (Bits - 1) ? 8 : 0) | (!result ? 4 : 0);
    cpu.set(i.rd, result, 64, Immediate && op != 3);
}
template <unsigned Bits> inline void bitfield(CpuState &cpu, const Instruction &i) {
    const uint64_t source = cpu.get(i.rn, Bits), destination = i.flags == 1 ? cpu.get(i.rd, Bits) : 0;
    const uint64_t bottom = (destination & ~i.immediate) | (shift<Bits>(source, 3, i.shift) & i.immediate);
    const uint64_t top = i.flags == 0 && ((source >> ((i.raw >> 10) & 63)) & 1) ? UINT64_MAX : destination;
    cpu.set(i.rd, (top & ~i.auxiliary) | (bottom & i.auxiliary), Bits);
}
template <unsigned Bits> inline void extract(CpuState &cpu, const Instruction &i) {
    const auto rhs = cpu.get(i.rm, Bits);
    cpu.set(i.rd, i.shift ? (rhs >> i.shift) | (cpu.get(i.rn, Bits) << (Bits - i.shift)) : rhs, Bits);
}
template <unsigned Bits> inline void select(CpuState &cpu, const Instruction &i) {
    auto value = cpu.get(i.rn, Bits);
    if (!condition(cpu, i.shift)) {
        value = cpu.get(i.rm, Bits);
        if (i.flags & 1)
            value = ~value;
        if (i.flags & 2)
            ++value;
    }
    cpu.set(i.rd, value, Bits);
}
template <unsigned Bits> inline void compare(CpuState &cpu, const Instruction &i) {
    if (condition(cpu, i.shift)) {
        const auto rhs = i.flags & 2 ? i.rm : cpu.get(i.rm, Bits);
        arithmetic<Bits>(cpu, cpu.get(i.rn, Bits), rhs,
                         decoded::set_flags | (i.flags & 1 ? decoded::subtract : 0));
    } else
        cpu.nzcv = i.raw & 15;
}
template <unsigned Bits> inline void two_source(CpuState &cpu, const Instruction &i) {
    const auto lhs = cpu.get(i.rn, Bits), rhs = cpu.get(i.rm, Bits);
    uint64_t result;
    if (i.flags == 2)
        result = rhs ? lhs / rhs : 0;
    else if (i.flags == 3) {
        const int64_t a = sign_extend(lhs, Bits), b = sign_extend(rhs, Bits);
        result = !b ? 0 : (a == INT64_MIN && b == -1 ? uint64_t(a) : uint64_t(a / b));
    } else
        result = shift<Bits>(lhs, i.flags - 8, rhs & (Bits - 1));
    cpu.set(i.rd, result, Bits);
}
template <unsigned Bits> inline void one_source(CpuState &cpu, const Instruction &i) {
    const Word<Bits> value = cpu.get(i.rn, Bits);
    Word<Bits> result;
    switch (i.flags) {
    case 0:
        if constexpr (Bits == 64)
            result = __builtin_bitreverse64(value);
        else
            result = __builtin_bitreverse32(value);
        break;
    case 1:
        result =
            ((value & Word<Bits>(0x00ff00ff00ff00ff)) << 8) | ((value >> 8) & Word<Bits>(0x00ff00ff00ff00ff));
        break;
    case 2:
        if constexpr (Bits == 64)
            result = __builtin_bswap32(value) | (uint64_t(__builtin_bswap32(value >> 32)) << 32);
        else
            result = __builtin_bswap32(value);
        break;
    case 3:
        result = __builtin_bswap64(value);
        break;
    default: {
        const Word<Bits> input = i.flags == 5 && (value >> (Bits - 1)) ? ~value : value;
        if constexpr (Bits == 64)
            result = input ? __builtin_clzll(input) : Bits;
        else
            result = input ? __builtin_clz(input) : Bits;
        if (i.flags == 5 && result)
            --result;
        break;
    }
    }
    cpu.set(i.rd, result, Bits);
}
template <unsigned Bits> inline void multiply(CpuState &cpu, const Instruction &i) {
    uint64_t product;
    if ((i.flags & 7) == 1)
        product = uint64_t(int64_t(int32_t(cpu.get(i.rn, 32))) * int64_t(int32_t(cpu.get(i.rm, 32))));
    else if ((i.flags & 7) == 5)
        product = cpu.get(i.rn, 32) * cpu.get(i.rm, 32);
    else
        product = cpu.get(i.rn, Bits) * cpu.get(i.rm, Bits);
    const auto accumulator = cpu.get(i.ra, Bits);
    cpu.set(i.rd, i.flags & 8 ? accumulator - product : accumulator + product, Bits);
}
inline void multiply_high(CpuState &cpu, const Instruction &i) {
    unsigned __int128 product;
    if (i.flags)
        product = static_cast<unsigned __int128>(cpu.get(i.rn)) * cpu.get(i.rm);
    else
        product =
            static_cast<__int128>(static_cast<int64_t>(cpu.get(i.rn))) * static_cast<int64_t>(cpu.get(i.rm));
    cpu.set(i.rd, uint64_t(product >> 64));
}

struct Address {
    uint64_t value, updated;
};
inline Address address(CpuState &cpu, const Instruction &i) {
    const uint64_t base = cpu.get(i.rn, 64, true);
    if ((i.flags & decoded::address_mode) == decoded::register_offset) {
        uint64_t offset = cpu.get(i.rm, i.flags & decoded::offset_word ? 32 : 64);
        if (i.flags & decoded::offset_signed)
            offset = sign_extend(offset, i.flags & decoded::offset_word ? 32 : 64);
        if (i.flags & decoded::offset_scaled)
            offset <<= i.shift;
        return {base + offset, base};
    }
    const uint64_t updated = base + i.immediate;
    return {(i.flags & decoded::address_mode) == decoded::post_index ? base : updated, updated};
}
inline void writeback(CpuState &cpu, const Instruction &i, uint64_t updated) {
    if ((i.flags & decoded::address_mode) >= decoded::post_index)
        cpu.set(i.rn, updated, 64, true);
}
template <unsigned Size> inline void load(CpuState &cpu, Memory &memory, const Instruction &i) {
    const auto location = address(cpu, i);
    uint64_t value = memory.load<Size>(location.value);
    if constexpr (Size < 8) {
        if (i.flags & decoded::signed_load) {
            value = sign_extend(value, Size * 8);
            if (i.flags & decoded::word_result)
                value = uint32_t(value);
        }
    }
    cpu.set(i.rd, value);
    writeback(cpu, i, location.updated);
}
template <unsigned Size> inline void store(CpuState &cpu, Memory &memory, const Instruction &i) {
    const auto location = address(cpu, i);
    memory.store<Size>(location.value, cpu.get(i.rd));
    writeback(cpu, i, location.updated);
}
template <unsigned Size, bool Load> inline void pair(CpuState &cpu, Memory &memory, const Instruction &i) {
    using Value = std::conditional_t<Size == 8, uint64_t, uint32_t>;
    const auto location = address(cpu, i);
    if constexpr (Load) {
        std::array<Value, 2> values;
        memory.copy_out<sizeof(values)>(location.value, values.data());
        uint64_t first = values[0], second = values[1];
        if constexpr (Size == 4) {
            if (i.flags & decoded::signed_load) {
                first = sign_extend(first, 32);
                second = sign_extend(second, 32);
            }
        }
        cpu.set(i.rd, first);
        cpu.set(i.ra, second);
    } else {
        const std::array<Value, 2> values{Value(cpu.get(i.rd)), Value(cpu.get(i.ra))};
        memory.copy_in<sizeof(values)>(location.value, values.data());
    }
    writeback(cpu, i, location.updated);
}
template <unsigned Size>
inline void literal(CpuState &cpu, Memory &memory, const Instruction &i, uint64_t pc) {
    uint64_t value = memory.load<Size>(pc + i.immediate);
    if (i.flags & decoded::signed_load)
        value = sign_extend(value, 32);
    cpu.set(i.rd, value);
}
} // namespace operit::arm64::scalar
