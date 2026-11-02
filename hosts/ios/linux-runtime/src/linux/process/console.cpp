#include "linux/syscalls.h"
#include <limits>

namespace operit::arm64 {
TerminalOpen Syscalls::open_console(uint32_t columns, uint32_t rows) {
    if (!columns || !rows || columns > std::numeric_limits<unsigned short>::max() ||
        rows > std::numeric_limits<unsigned short>::max())
        guest_error(22);
    winsize size{};
    size.ws_col = static_cast<unsigned short>(columns);
    size.ws_row = static_cast<unsigned short>(rows);
    auto pair = terminals_->open_pair(2, nullptr, &size);
    auto &slave = pair.second;
    DescriptorTable descriptors(descriptors_);
    const auto numbers = descriptors.numbers();
    for (int fd : numbers)
        if (fd < 3)
            descriptors.close(uint64_t(fd));
    const auto input = descriptors.insert(std::move(slave.host), slave.path, 2, 0);
    auto &description = *descriptors.get(input).description;
    description.terminal = slave.endpoint;
    description.scheduler_io = true;
    description.standard_stream = -1;
    descriptors.duplicate(input, 1);
    descriptors.duplicate(input, 2);
    terminals_->opened(*slave.endpoint, process_anchor_->pid, false);
    descriptors_ = std::move(descriptors);
    return std::move(pair.first);
}
} // namespace operit::arm64
