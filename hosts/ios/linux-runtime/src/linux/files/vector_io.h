#pragma once
#include "linux/abi.h"
#include <cstddef>
#include <cstdint>
#include <vector>

namespace operit::arm64 {
class Memory;
namespace files {
struct VectorBuffer {
    std::vector<uint8_t> bytes;
    bool fault = false;
};
class IoVectors {
    std::vector<linux_abi::Iovec> entries_;
    size_t size_ = 0;

  public:
    IoVectors(Memory &memory, uint64_t address, uint64_t count);
    size_t size() const {
        return size_;
    }
    VectorBuffer prepare(Memory &memory, bool writing, size_t capacity) const;
    void scatter(Memory &memory, const void *data, size_t size) const;
};
} // namespace files
} // namespace operit::arm64
