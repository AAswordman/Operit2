#pragma once
#include "linux/terminal/manager.h"

namespace operit::arm64 {
class Console {
    std::shared_ptr<TerminalManager> manager_;
    TerminalOpen master_;

  public:
    Console(std::shared_ptr<TerminalManager> manager, TerminalOpen master) noexcept;
    Console(const Console &) = delete;
    Console &operator=(const Console &) = delete;
    int64_t read(void *bytes, size_t length);
    int64_t write(const void *bytes, size_t length);
    void resize(uint32_t columns, uint32_t rows);
    void signal(int number);
};
} // namespace operit::arm64
