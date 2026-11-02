#pragma once
#include "base/support.h"
#include <array>

namespace operit::arm64 {
struct CpuState {
    std::array<uint64_t, 31> x{};
    std::array<std::array<uint8_t, 16>, 32> v{};
    uint64_t sp = 0, pc = 0, tls = 0;
    uint64_t fpcr = 0, fpsr = 0;
    uint8_t nzcv = 0;
    uint64_t exclusive_address = 0, exclusive_epoch = 0;
    unsigned exclusive_size = 0;
    bool exclusive_valid = false;
    uint64_t get(unsigned index, unsigned bits = 64, bool stack = false) const {
        return (index == 31 ? (stack ? sp : 0) : x[index]) & mask(bits);
    }
    void set(unsigned index, uint64_t value, unsigned bits = 64, bool stack = false) {
        value &= mask(bits);
        if (index < 31)
            x[index] = value;
        else if (stack)
            sp = value;
    }
    bool condition(unsigned code) const {
        const bool n = nzcv & 8, z = nzcv & 4, c = nzcv & 2, v = nzcv & 1;
        bool result;
        switch ((code >> 1) & 7) {
        case 0:
            result = z;
            break;
        case 1:
            result = c;
            break;
        case 2:
            result = n;
            break;
        case 3:
            result = v;
            break;
        case 4:
            result = c && !z;
            break;
        case 5:
            result = n == v;
            break;
        case 6:
            result = !z && n == v;
            break;
        default:
            result = true;
        }
        return (code & 1) && code != 15 ? !result : result;
    }
    void logical_flags(uint64_t value, unsigned bits) {
        value &= mask(bits);
        nzcv = ((value >> (bits - 1)) ? 8 : 0) | (value == 0 ? 4 : 0);
    }
    uint64_t add(uint64_t a, uint64_t b, bool carry, unsigned bits, bool flags) {
        a &= mask(bits);
        b &= mask(bits);
        unsigned __int128 wide = static_cast<unsigned __int128>(a) + b + carry;
        uint64_t result = static_cast<uint64_t>(wide) & mask(bits);
        if (flags) {
            logical_flags(result, bits);
            nzcv |= ((wide >> bits) ? 2 : 0) | (((~(a ^ b) & (a ^ result)) >> (bits - 1)) ? 1 : 0);
        }
        return result;
    }
};
} // namespace operit::arm64
