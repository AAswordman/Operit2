#include "cpu/aarch64/simd.h"
#include "cpu/aarch64/vector_narrow.h"
#include "cpu/aarch64/vector_floating.h"
#include "cpu/aarch64/vector_integer.h"
#include "cpu/aarch64/vector_lanes.h"
#include "cpu/aarch64/vector_shift.h"
#include <algorithm>
#include <cstring>

namespace operit::arm64 {
bool execute_simd(CpuState &cpu, uint32_t r) {
    if (execute_vector_narrow(cpu, r) || execute_vector_shift(cpu, r) ||
        execute_vector_arithmetic(cpu, r) || execute_vector_unary(cpu, r) || execute_vector_widen(cpu, r) ||
        execute_vector_reduction(cpu, r) || execute_vector_integer(cpu, r) || execute_vector_floating(cpu, r))
        return true;
    unsigned rd = r & 31, rn = (r >> 5) & 31;
    unsigned count = (r & (1u << 30)) ? 16 : 8;
    std::array<uint8_t, 16> result{};
    const uint32_t move = r & 0xfffffc00;
    if (move == 0x9e660000 || move == 0x1e260000) {
        uint64_t value = 0;
        unsigned size = move == 0x9e660000 ? 8 : 4;
        std::memcpy(&value, cpu.v[rn].data(), size);
        cpu.set(rd, value, size * 8);
        return true;
    }
    if (move == 0x9e670000 || move == 0x1e270000) {
        const uint64_t value = cpu.get(rn);
        std::memcpy(result.data(), &value, move == 0x9e670000 ? 8 : 4);
        cpu.v[rd] = result;
        return true;
    }
    if ((r & 0xff201fe0) == 0x1e201000) { // FMOV scalar immediate (IEEE binary32/binary64)
        const unsigned type = (r >> 22) & 3;
        if (type > 1)
            return false;
        const unsigned width = type ? 64 : 32, exponent_bits = type ? 11 : 8;
        const unsigned fraction_bits = width - exponent_bits - 1;
        const uint64_t immediate = (r >> 13) & 255, repeated = (immediate >> 6) & 1;
        const uint64_t exponent = ((repeated ^ 1) << (exponent_bits - 1)) |
                                  ((repeated ? mask(exponent_bits - 3) : 0) << 2) | ((immediate >> 4) & 3);
        const uint64_t value = ((immediate >> 7) << (width - 1)) | (exponent << fraction_bits) |
                               ((immediate & 15) << (fraction_bits - 4));
        std::memcpy(result.data(), &value, width / 8);
        cpu.v[rd] = result;
        return true;
    }
    if (move == 0x9eae0000) {
        uint64_t value;
        std::memcpy(&value, cpu.v[rn].data() + 8, 8);
        cpu.set(rd, value);
        return true;
    }
    if (move == 0x9eaf0000) {
        const uint64_t value = cpu.get(rn);
        std::memcpy(cpu.v[rd].data() + 8, &value, 8);
        return true;
    }
    const uint32_t copy = r & 0xbfe0fc00;
    if (copy == 0x0e000c00 || copy == 0x0e000400 || copy == 0x0e003c00 || copy == 0x0e002c00 ||
        (r & 0xffe0fc00) == 0x4e001c00) {
        unsigned imm5 = (r >> 16) & 31;
        if (!imm5)
            return false;
        unsigned log_size = __builtin_ctz(imm5), size = 1u << log_size, lane = imm5 >> (log_size + 1);
        if (size > 8 || lane * size + size > 16)
            return false;
        uint64_t value = 0;
        if (copy == 0x0e000c00 || (r & 0xffe0fc00) == 0x4e001c00)
            value = cpu.get(rn);
        else
            std::memcpy(&value, cpu.v[rn].data() + lane * size, size);
        if (copy == 0x0e003c00 || copy == 0x0e002c00) {
            if (copy == 0x0e002c00)
                value = sign_extend(value, size * 8);
            cpu.set(rd, value, count == 16 ? 64 : 32);
            return true;
        }
        if ((r & 0xffe0fc00) == 0x4e001c00) {
            result = cpu.v[rd];
            std::memcpy(result.data() + lane * size, &value, size);
        } else
            for (unsigned j = 0; j < count; j += size)
                std::memcpy(result.data() + j, &value, size);
    } else if ((r & 0xbf20bc00) == 0x0e001800 || (r & 0xbf20bc00) == 0x0e002800 ||
               (r & 0xbf20bc00) == 0x0e003800) { // UZP1/2, TRN1/2, ZIP1/2
        const unsigned size = 1u << ((r >> 22) & 3), lanes = count / size;
        if (lanes < 2)
            return false;
        const unsigned part = (r >> 14) & 1;
        const uint32_t operation = r & 0x3000;
        const auto a = cpu.v[rn], b = cpu.v[(r >> 16) & 31];
        for (unsigned j = 0; j < lanes / 2; ++j) {
            if (operation == 0x1000) {
                std::memcpy(result.data() + j * size, a.data() + (2 * j + part) * size, size);
                std::memcpy(result.data() + (lanes / 2 + j) * size, b.data() + (2 * j + part) * size, size);
            } else {
                const unsigned lane = operation == 0x2000 ? 2 * j + part : part * (lanes / 2) + j;
                std::memcpy(result.data() + 2 * j * size, a.data() + lane * size, size);
                std::memcpy(result.data() + (2 * j + 1) * size, b.data() + lane * size, size);
            }
        }
    } else if ((r & 0xbf208c00) == 0x0e000000) { // TBL / TBX
        const auto indices = cpu.v[(r >> 16) & 31];
        unsigned tables = ((r >> 13) & 3) + 1;
        if (r & (1u << 12))
            std::copy_n(cpu.v[rd].begin(), count, result.begin());
        for (unsigned j = 0; j < count; ++j)
            if (indices[j] < tables * 16)
                result[j] = cpu.v[(rn + indices[j] / 16) % 32][indices[j] % 16];
    } else if ((r & 0x9f20fc00) == 0x0e201c00) {
        unsigned op = (r >> 22) & 3;
        bool u = (r >> 29) & 1;
        auto a = cpu.v[rn], b = cpu.v[(r >> 16) & 31], d = cpu.v[rd];
        for (unsigned j = 0; j < count; ++j) {
            if (!u) {
                if (op == 0)
                    result[j] = a[j] & b[j];
                if (op == 1)
                    result[j] = a[j] & ~b[j];
                if (op == 2)
                    result[j] = a[j] | b[j];
                if (op == 3)
                    result[j] = a[j] | ~b[j];
            } else {
                if (op == 0)
                    result[j] = a[j] ^ b[j];
                if (op == 1)
                    result[j] = (d[j] & a[j]) | (~d[j] & b[j]);
                if (op == 2)
                    result[j] = (d[j] & ~b[j]) | (a[j] & b[j]);
                if (op == 3)
                    result[j] = (d[j] & b[j]) | (a[j] & ~b[j]);
            }
        }
    } else if ((r & 0x9ff80c00) == 0x0f000400) {
        unsigned cmode = (r >> 12) & 15, immediate = ((r >> 5) & 31) | ((r >> 11) & 224);
        bool invert = (r >> 29) & 1;
        uint64_t value;
        unsigned size;
        if (cmode < 8) {
            size = 4;
            value = uint64_t(immediate) << ((cmode / 2) * 8);
        } else if (cmode < 12) {
            size = 2;
            value = uint64_t(immediate) << (((cmode - 8) / 2) * 8);
        } else if (cmode < 14) {
            size = 4;
            unsigned shift = (cmode - 11) * 8;
            value = (uint64_t(immediate) << shift) | mask(shift);
        } else if (cmode == 14) {
            size = invert ? 8 : 1;
            value = immediate;
            if (invert) {
                value = 0;
                for (unsigned j = 0; j < 8; ++j)
                    if (immediate & (1u << j))
                        value |= uint64_t{255} << (j * 8);
            }
        } else { // FMOV (vector immediate): U selects binary64 instead of binary32.
            if (invert && count != 16)
                return false;
            size = invert ? 8 : 4;
            const unsigned exponent_bits = invert ? 11 : 8, fraction_bits = size * 8 - exponent_bits - 1;
            const uint64_t repeated = (immediate >> 6) & 1;
            const uint64_t exponent = ((repeated ^ 1) << (exponent_bits - 1)) |
                                      ((repeated ? mask(exponent_bits - 3) : 0) << 2) |
                                      ((immediate >> 4) & 3);
            value = (uint64_t(immediate >> 7) << (size * 8 - 1)) | (exponent << fraction_bits) |
                    (uint64_t(immediate & 15) << (fraction_bits - 4));
        }
        if (invert && cmode < 14)
            value = ~value;
        for (unsigned j = 0; j < count; j += size)
            std::memcpy(result.data() + j, &value, size);
        if (cmode < 12 && (cmode & 1))
            for (unsigned j = 0; j < count; ++j)
                result[j] = invert ? (cpu.v[rd][j] & result[j]) : (cpu.v[rd][j] | result[j]);
    } else
        return false;
    cpu.v[rd] = result;
    return true;
}
} // namespace operit::arm64
