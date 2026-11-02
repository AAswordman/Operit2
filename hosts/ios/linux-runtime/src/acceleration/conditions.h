#pragma once
#include "cpu/aarch64/state.h"
#include <array>

namespace operit::arm64::scalar {
inline bool condition(const CpuState &cpu, unsigned code) {
    // Each bit is the result for one architectural NZCV value.
    static constexpr std::array<uint16_t, 16> truth{0xf0f0, 0x0f0f, 0xcccc, 0x3333, 0xff00, 0x00ff,
                                                    0xaaaa, 0x5555, 0x0c0c, 0xf3f3, 0xaa55, 0x55aa,
                                                    0x0a05, 0xf5fa, 0xffff, 0xffff};
    return (truth[code] >> cpu.nzcv) & 1;
}
} // namespace operit::arm64::scalar
