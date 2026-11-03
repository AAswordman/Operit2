#pragma once
#include <array>
#include <cstddef>
#include <cstdint>

namespace operit::arm64 {
enum class FixedFunction { Copy, Move, Set, Length };
struct FixedSymbol {
    uint64_t value;
    FixedFunction function;
    uint64_t size;
};
using FixedSymbols = std::array<FixedSymbol, 4>;
struct FixedManifest {
    size_t file_size;
    std::array<uint8_t, 32> sha256;
    FixedSymbols symbols;
};
// Both complete ELF images are authenticated independently. Their four helper
// programs have identical instructions after relocating local branches.
inline constexpr FixedSymbols fixed_musl_symbols{{{0x18290, FixedFunction::Copy, 396},
                                                  {0x5e3a4, FixedFunction::Move, 276},
                                                  {0x18420, FixedFunction::Set, 260},
                                                  {0x5ec34, FixedFunction::Length, 92}}};
inline constexpr FixedSymbols compiled_musl_symbols{{{0x18290, FixedFunction::Copy, 396},
                                                     {0x5e398, FixedFunction::Move, 276},
                                                     {0x18420, FixedFunction::Set, 260},
                                                     {0x5ec28, FixedFunction::Length, 92}}};
inline constexpr std::array<FixedManifest, 2> fixed_musl_manifests{{
    // Alpine aarch64 musl 1.2.5-r23, pinned minirootfs.
    {723480, {0xfc, 0x61, 0xb6, 0xe1, 0x1f, 0xd8, 0xc6, 0xbf, 0xee, 0x32, 0x35, 0x86, 0x52, 0x49, 0xcd, 0x78,
      0xee, 0xa3, 0x4a, 0xa3, 0x6d, 0x88, 0x27, 0x7e, 0x60, 0x08, 0x03, 0x80, 0x31, 0x16, 0x09, 0x93},
     fixed_musl_symbols},
    // Alpine aarch64 musl 1.2.5-r24, development packages.
    {723480, {0x65, 0x2a, 0x04, 0xbc, 0xb0, 0x32, 0x67, 0x33, 0xf7, 0x62, 0xec, 0xa7, 0x99, 0x88, 0x7b, 0xbc,
      0x9e, 0x07, 0x0d, 0xf0, 0xa0, 0xd6, 0x66, 0x82, 0x38, 0x76, 0x9f, 0x6d, 0xfc, 0xdc, 0x77, 0xc2},
     compiled_musl_symbols},
}};
} // namespace operit::arm64
