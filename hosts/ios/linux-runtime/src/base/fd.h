#pragma once
#include <memory>
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

// Guest dup/fork entries share one native open description. Guest descriptor
// flags and table membership are independent and do not require a native dup.
class SharedFD {
    std::shared_ptr<FD> owner_;

  public:
    explicit SharedFD(FD fd) : owner_(std::make_shared<FD>(std::move(fd))) {}
    int get() const {
        return owner_ ? owner_->get() : -1;
    }
};

} // namespace operit::arm64
