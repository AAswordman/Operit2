#pragma once
#include <cstdint>

namespace operit::arm64::cache_profile {
// Describe the guest cache model, never the host CPU's cache geometry. Both
// maintenance and ZVA operate on 64-byte lines. CTR reports unknown ERG/CWG,
// PIPT L1 instruction indexing, and requires explicit data/instruction coherence
// operations (IDC/DIC clear). Bit 31 is architecturally RES1.
constexpr unsigned line_log2_words = 4;
constexpr uint64_t line_bytes = uint64_t{4} << line_log2_words;
constexpr uint64_t ctr_el0 = (uint64_t{1} << 31) | (uint64_t{line_log2_words} << 16) |
                             (uint64_t{3} << 14) | line_log2_words;
constexpr uint64_t dczid_el0 = line_log2_words;
constexpr uint32_t dc_zva = 0xd50b7420, ic_ivau = 0xd50b7520;
constexpr uint32_t dc_cvac = 0xd50b7a20, dc_cvau = 0xd50b7b20, dc_civac = 0xd50b7e20;
constexpr bool maintenance(uint32_t raw) {
    const uint32_t operation = raw & 0xffffffe0;
    return operation == dc_zva || operation == ic_ivau || operation == dc_cvac ||
           operation == dc_cvau || operation == dc_civac;
}
} // namespace operit::arm64::cache_profile
