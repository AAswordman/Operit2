#pragma once
#include <unistd.h>
#include <utility>

namespace operit::arm64 {
class FD {
    int value_ = -1;

  public:
    explicit FD(int value = -1) : value_(value) {}
    ~FD() {
        if (value_ >= 0)
            ::close(value_);
    }
    FD(const FD &) = delete;
    FD &operator=(const FD &) = delete;
    FD(FD &&other) noexcept : value_(std::exchange(other.value_, -1)) {}
    FD &operator=(FD &&other) noexcept {
        if (this != &other) {
            if (value_ >= 0)
                ::close(value_);
            value_ = std::exchange(other.value_, -1);
        }
        return *this;
    }
    int get() const {
        return value_;
    }
};

} // namespace operit::arm64
