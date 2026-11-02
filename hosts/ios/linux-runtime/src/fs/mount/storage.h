#pragma once
#include "base/fd.h"
#include <memory>
#include <string>
#include <sys/types.h>

namespace operit::arm64::mounts {
class TemporaryTree {
    std::string path_;
    FD root_;
    TemporaryTree(std::string path, FD root);

  public:
    ~TemporaryTree();
    TemporaryTree(const TemporaryTree &) = delete;
    TemporaryTree &operator=(const TemporaryTree &) = delete;
    static std::shared_ptr<TemporaryTree> create(mode_t mode);
    int descriptor() const {
        return root_.get();
    }
};
} // namespace operit::arm64::mounts
