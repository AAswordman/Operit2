#include "runtime/console/console.h"
#include "base/host_io.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <limits>
#include <unistd.h>

namespace operit::arm64 {
Console::Console(std::shared_ptr<TerminalManager> manager, TerminalOpen master) noexcept
    : manager_(std::move(manager)), master_(std::move(master)) {}
int64_t Console::read(void *bytes, size_t length) {
    if (!bytes && length)
        guest_error(14);
    if (!length)
        return 0;
    const auto result = manager_->read(*master_.endpoint, master_.host.get(), bytes, std::min<size_t>(length, 0x7ffff000));
    return manager_->read_result(*master_.endpoint, bytes, result);
}
int64_t Console::write(const void *bytes, size_t length) {
    if (!bytes && length)
        guest_error(14);
    if (!length)
        return 0;
    const auto result = manager_->write(*master_.endpoint, master_.host.get(), bytes, std::min<size_t>(length, 0x7ffff000));
    if (result < 0)
        return result;
    if (result)
        manager_->written_input(*master_.endpoint, bytes, size_t(result));
    return result;
}
void Console::resize(uint32_t columns, uint32_t rows) {
    if (!columns || !rows || columns > std::numeric_limits<unsigned short>::max() ||
        rows > std::numeric_limits<unsigned short>::max())
        guest_error(22);
    winsize size{};
    size.ws_col = static_cast<unsigned short>(columns);
    size.ws_row = static_cast<unsigned short>(rows);
    manager_->resize(*master_.endpoint, size);
}
void Console::signal(int number) {
    if (number < 1 || number > 64)
        guest_error(22);
    manager_->signal_foreground(*master_.endpoint, unsigned(number));
}
} // namespace operit::arm64
