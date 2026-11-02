#pragma once
#include "base/fd.h"
#include "linux/error_mapping.h"
#include <fcntl.h>
#include <utility>

namespace operit::arm64::timers {
inline std::pair<FD, FD> ready_pipe() {
    int descriptors[2];
#if defined(__linux__)
    check_host(::pipe2(descriptors, O_NONBLOCK | O_CLOEXEC));
    return {FD(descriptors[0]), FD(descriptors[1])};
#else
    check_host(::pipe(descriptors));
    FD reader(descriptors[0]), writer(descriptors[1]);
    for (int fd : {reader.get(), writer.get()}) {
        check_host(::fcntl(fd, F_SETFD, FD_CLOEXEC));
        check_host(::fcntl(fd, F_SETFL, O_NONBLOCK));
    }
    return {std::move(reader), std::move(writer)};
#endif
}
} // namespace operit::arm64::timers
