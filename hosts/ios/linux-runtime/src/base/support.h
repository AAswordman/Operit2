#pragma once
#include "operit/arm64_runtime.h"
#include <cstdint>
#include <cstdio>
#include <stdexcept>
#include <string>

namespace operit::arm64 {
constexpr uint64_t page_size = 4096;
constexpr uint64_t address_limit = uint64_t{1} << 48;
constexpr uint64_t default_memory_limit = uint64_t{256} << 20;
constexpr size_t max_path = 4096;
constexpr size_t max_io = 1 << 20;
constexpr unsigned read_perm = 1, write_perm = 2, exec_perm = 4;

struct Failure : std::runtime_error {
    ort_arm64_status status;
    Failure(const std::string &message, ort_arm64_status s = ORT_ARM64_FAULT)
        : std::runtime_error(message), status(s) {}
};
struct GuestError {
    int number;
};
[[noreturn]] inline void guest_error(int number) {
    throw GuestError{number};
}

inline std::string hex(uint64_t value) {
    char text[32];
    std::snprintf(text, sizeof(text), "0x%llx", static_cast<unsigned long long>(value));
    return text;
}
inline uint64_t align_down(uint64_t value) {
    return value & ~(page_size - 1);
}
inline uint64_t align_up(uint64_t value) {
    if (value > address_limit - page_size)
        throw Failure("address overflow");
    return (value + page_size - 1) & ~(page_size - 1);
}
inline int64_t sign_extend(uint64_t value, unsigned bits) {
    const uint64_t sign = uint64_t{1} << (bits - 1);
    return static_cast<int64_t>((value ^ sign) - sign);
}
inline uint64_t mask(unsigned bits) {
    return bits == 64 ? UINT64_MAX : (uint64_t{1} << bits) - 1;
}
inline uint64_t rotate(uint64_t value, unsigned count, unsigned bits) {
    value &= mask(bits);
    count %= bits;
    return count ? ((value >> count) | (value << (bits - count))) & mask(bits) : value;
}

} // namespace operit::arm64
