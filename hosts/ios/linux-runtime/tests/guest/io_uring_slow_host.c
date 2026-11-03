#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/uio.h>
#include <time.h>
#include <unistd.h>

static atomic_int claimed;

static void mark(const char *variable) {
    const char *path = getenv(variable);
    if (!path)
        return;
    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (fd >= 0)
        close(fd);
}

static int gate(int fd, const char *operation) {
    const char *target = getenv("OPERIT_IO_URING_GATE_TARGET");
    const char *selected = getenv("OPERIT_IO_URING_GATE_OPERATION");
    const char *release = getenv("OPERIT_IO_URING_GATE_RELEASE");
    struct stat actual, expected;
    if (!target || !selected || !release || strcmp(selected, operation) ||
        fstat(fd, &actual) || stat(target, &expected) || actual.st_dev != expected.st_dev ||
        actual.st_ino != expected.st_ino)
        return 0;
    int previous = 0;
    if (!atomic_compare_exchange_strong(&claimed, &previous, 1))
        return 0;
    mark("OPERIT_IO_URING_GATE_ENTERED");
    struct timespec start, now, pause = {0, 1000000};
    clock_gettime(CLOCK_MONOTONIC, &start);
    for (;;) {
        if (!access(release, F_OK))
            break;
        nanosleep(&pause, 0);
        clock_gettime(CLOCK_MONOTONIC, &now);
        if ((now.tv_sec - start.tv_sec) * 1000 + (now.tv_nsec - start.tv_nsec) / 1000000 >= 2000) {
            mark("OPERIT_IO_URING_GATE_TIMEOUT");
            errno = ETIMEDOUT;
            return -1;
        }
    }
    if (getenv("OPERIT_IO_URING_GATE_ERROR")) {
        mark("OPERIT_IO_URING_GATE_FINISHED");
        errno = EIO;
        return -1;
    }
    return 1;
}

static ssize_t gated_pread(int fd, void *buffer, size_t count, off_t offset) {
    const int state = gate(fd, "read");
    if (state < 0)
        return -1;
#ifdef __APPLE__
    const ssize_t result = pread(fd, buffer, count, offset);
#else
    ssize_t (*next)(int, void *, size_t, off_t) = dlsym(RTLD_NEXT, "pread");
    const ssize_t result = next(fd, buffer, count, offset);
#endif
    const int error = errno;
    if (state)
        mark("OPERIT_IO_URING_GATE_FINISHED");
    errno = error;
    return result;
}

static ssize_t gated_pwrite(int fd, const void *buffer, size_t count, off_t offset) {
    const int state = gate(fd, "write");
    if (state < 0)
        return -1;
#ifdef __APPLE__
    const ssize_t result = pwrite(fd, buffer, count, offset);
#else
    ssize_t (*next)(int, const void *, size_t, off_t) = dlsym(RTLD_NEXT, "pwrite");
    const ssize_t result = next(fd, buffer, count, offset);
#endif
    const int error = errno;
    if (state)
        mark("OPERIT_IO_URING_GATE_FINISHED");
    errno = error;
    return result;
}

static int gated_fsync(int fd) {
    const int state = gate(fd, "fsync");
    if (state < 0)
        return -1;
#ifdef __APPLE__
    const int result = fsync(fd);
#else
    int (*next)(int) = dlsym(RTLD_NEXT, "fsync");
    const int result = next(fd);
#endif
    const int error = errno;
    if (state)
        mark("OPERIT_IO_URING_GATE_FINISHED");
    errno = error;
    return result;
}

#ifdef __APPLE__
#define INTERPOSE(replacement, original)                                                                     \
    __attribute__((used)) static const struct { const void *replacement; const void *original; }              \
        interpose_##original __attribute__((section("__DATA,__interpose"))) = {                              \
            (const void *)(unsigned long)&replacement, (const void *)(unsigned long)&original}
INTERPOSE(gated_pread, pread);
INTERPOSE(gated_pwrite, pwrite);
INTERPOSE(gated_fsync, fsync);
#else
ssize_t pread(int fd, void *buffer, size_t count, off_t offset) {
    return gated_pread(fd, buffer, count, offset);
}
ssize_t pwrite(int fd, const void *buffer, size_t count, off_t offset) {
    return gated_pwrite(fd, buffer, count, offset);
}
int fsync(int fd) {
    return gated_fsync(fd);
}
ssize_t preadv(int fd, const struct iovec *vectors, int count, off_t offset) {
    const int state = gate(fd, "read");
    if (state < 0)
        return -1;
    ssize_t (*next)(int, const struct iovec *, int, off_t) = dlsym(RTLD_NEXT, "preadv");
    const ssize_t result = next(fd, vectors, count, offset);
    const int error = errno;
    if (state)
        mark("OPERIT_IO_URING_GATE_FINISHED");
    errno = error;
    return result;
}
#endif
