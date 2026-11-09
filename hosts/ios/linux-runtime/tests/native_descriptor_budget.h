#pragma once
#include <algorithm>
#include <fcntl.h>
#include <stdexcept>
#include <sys/resource.h>

// Leave only a small amount of native FD capacity, as in an app that also has
// framework, network and database descriptors. Never change the guest limit.
struct NativeDescriptorBudget {
    // Probe without opening another descriptor, including under a tight limit.
    static size_t count() {
        struct rlimit limit{};
        if (::getrlimit(RLIMIT_NOFILE, &limit) < 0)
            throw std::runtime_error("getrlimit");
        size_t result = 0;
        for (rlim_t fd = 0; fd < std::min<rlim_t>(limit.rlim_cur, 65536); ++fd)
            if (::fcntl(int(fd), F_GETFD) >= 0)
                ++result;
        return result;
    }
    struct rlimit original{};
    explicit NativeDescriptorBudget(rlim_t spare) {
        if (::getrlimit(RLIMIT_NOFILE, &original) < 0)
            throw std::runtime_error("getrlimit");
        rlim_t highest = 2;
        for (rlim_t fd = 0; fd < std::min<rlim_t>(original.rlim_cur, 65536); ++fd)
            if (::fcntl(int(fd), F_GETFD) >= 0)
                highest = fd;
        auto limited = original;
        limited.rlim_cur = std::min(original.rlim_cur, highest + 1 + spare);
        if (::setrlimit(RLIMIT_NOFILE, &limited) < 0)
            throw std::runtime_error("setrlimit");
    }
    ~NativeDescriptorBudget() { ::setrlimit(RLIMIT_NOFILE, &original); }
};
