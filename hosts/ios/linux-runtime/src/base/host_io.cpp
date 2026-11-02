#include "base/host_io.h"
#include <cerrno>
#include <fcntl.h>
#include <mutex>
#include <pthread.h>
#include <signal.h>
#include <unistd.h>

namespace operit::arm64 {
ssize_t host_write(int fd, const void *data, size_t count) {
#ifdef __APPLE__
    // Darwin may route a pipe's SIGPIPE to another host thread. Blocking it on
    // the executor alone is insufficient once the resolver starts host workers.
    // Suppress at the open file description and restore its original setting.
    static std::mutex writing;
    std::lock_guard<std::mutex> lock(writing);
    int original = ::fcntl(fd, F_GETNOSIGPIPE);
    if (original < 0)
        return -1;
    if (!original && ::fcntl(fd, F_SETNOSIGPIPE, 1) < 0)
        return -1;
    ssize_t result = ::write(fd, data, count);
    int saved_errno = errno;
    if (!original)
        ::fcntl(fd, F_SETNOSIGPIPE, 0);
    errno = saved_errno;
    return result;
#else
    sigset_t blocked, original, pending;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGPIPE);
    int error = ::pthread_sigmask(SIG_BLOCK, &blocked, &original);
    if (error) {
        errno = error;
        return -1;
    }
    ::sigpending(&pending);
    const bool already_pending = sigismember(&pending, SIGPIPE) == 1;
    ssize_t result = ::write(fd, data, count);
    const int saved_errno = errno;
    if (result < 0 && saved_errno == EPIPE && !already_pending) {
        ::sigpending(&pending);
        if (sigismember(&pending, SIGPIPE) == 1) {
            int signal = 0;
            ::sigwait(&blocked, &signal);
        }
    }
    ::pthread_sigmask(SIG_SETMASK, &original, nullptr);
    errno = saved_errno;
    return result;
#endif
}
} // namespace operit::arm64
