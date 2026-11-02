#pragma once
#include <cstddef>
#include <sys/types.h>

namespace operit::arm64 {
// Deliver EPIPE to the guest without changing the embedding process's global
// SIGPIPE disposition. Only a signal produced by this write is consumed.
ssize_t host_write(int fd, const void *data, size_t count);
} // namespace operit::arm64
