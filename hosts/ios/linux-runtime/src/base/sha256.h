#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
namespace operit::arm64 {
std::array<uint8_t, 32> sha256(const void *data, size_t length);
}
