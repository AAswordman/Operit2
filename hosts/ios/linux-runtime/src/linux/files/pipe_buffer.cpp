#include "linux/files/pipe_buffer.h"
#include "base/host_io.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <poll.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <unistd.h>
#include <vector>

namespace operit::arm64::files {
std::shared_ptr<PipeBuffer> PipeBuffer::create(int reader, int writer) {
    struct stat source{}, destination{};
    check_host(::fstat(reader, &source));
    check_host(::fstat(writer, &destination));
    if (!S_ISFIFO(source.st_mode) || !S_ISFIFO(destination.st_mode))
        guest_error(22);
#ifdef __APPLE__
    // Darwin can reduce a native pipe to SMALL_PIPE_SIZE. Advertising no more
    // than its minimum capacity preserves Linux atomic writes up to 4096 even
    // though Darwin's public PIPE_BUF guarantee is only 512 bytes.
    const long host_page = ::sysconf(_SC_PAGESIZE);
    if (host_page < 4096)
        guest_error(95);
    const size_t capacity = std::min<size_t>(16 * 1024, host_page);
#else
    const int native_capacity = ::fcntl(writer, F_GETPIPE_SZ);
    check_host(native_capacity);
    const size_t capacity = native_capacity;
#endif
    int unread = 0;
    check_host(::ioctl(reader, FIONREAD, &unread));
    if (unread)
        guest_error(22); // Register the empty pair at pipe2 creation.
    return std::shared_ptr<PipeBuffer>(new PipeBuffer(capacity));
}
bool PipeBuffer::synchronized_from_reader(int fd) {
    if (!synchronized_)
        return false;
    int unread = 0;
    if (::ioctl(fd, FIONREAD, &unread) < 0 || unread < 0 || size_t(unread) != bytes_.size())
        synchronized_ = false;
    return synchronized_;
}
int64_t PipeBuffer::read(int fd, void *data, size_t count) {
    std::lock_guard<std::mutex> guard(mutex_);
    if (!count)
        return 0;
    synchronized_from_reader(fd);
    const ssize_t result = ::read(fd, data, count);
    if (result < 0)
        return -linux_errno(errno);
    if (synchronized_) {
        const auto *bytes = static_cast<const uint8_t *>(data);
        for (ssize_t index = 0; index < result; ++index) {
            if (bytes_.empty() || bytes_.front() != bytes[index]) {
                synchronized_ = false;
                bytes_.clear();
                break;
            }
            bytes_.pop_front();
        }
    }
    return result;
}
int64_t PipeBuffer::write(int fd, const void *data, size_t count) {
    std::lock_guard<std::mutex> guard(mutex_);
    if (!count)
        return 0;
    // Darwin reports FIONREAD on the write endpoint independently of the
    // reader's queued bytes. Only read-side operations may validate the mirror;
    // retaining a read endpoint here would also suppress a real EPIPE.
    if (!synchronized_)
        return -95;
    const size_t space = bytes_.size() >= capacity_ ? 0 : capacity_ - bytes_.size();
    if (!space || (count <= 4096 && count > space)) {
        // A closed read end takes precedence over a full pipe.
        pollfd ready{fd, POLLOUT, 0};
        if (::poll(&ready, 1, 0) < 0)
            return -linux_errno(errno);
        return ready.revents & (POLLERR | POLLHUP) ? -32 : -11;
    }
    const size_t size = std::min(count, space);
    const ssize_t result = host_write(fd, data, size);
    if (result < 0)
        return -linux_errno(errno);
    const auto *bytes = static_cast<const uint8_t *>(data);
    bytes_.insert(bytes_.end(), bytes, bytes + result);
    return result;
}
int64_t PipeBuffer::peek(int fd, void *data, size_t count) {
    std::lock_guard<std::mutex> guard(mutex_);
    if (!synchronized_from_reader(fd))
        return -95;
    count = std::min(count, bytes_.size());
    if (!count) {
        pollfd ready{fd, POLLIN, 0};
        if (::poll(&ready, 1, 0) < 0)
            return -linux_errno(errno);
        return ready.revents & POLLHUP ? 0 : -11;
    }
    std::copy_n(bytes_.begin(), count, static_cast<uint8_t *>(data));
    return count;
}
int64_t PipeBuffer::consume(int fd, const void *expected, size_t count) {
    std::lock_guard<std::mutex> guard(mutex_);
    if (!synchronized_from_reader(fd) || count > bytes_.size())
        return -95;
    std::vector<uint8_t> bytes(count);
    const ssize_t result = ::read(fd, bytes.data(), count);
    if (result < 0)
        return -linux_errno(errno);
    if (size_t(result) != count || std::memcmp(bytes.data(), expected, count)) {
        synchronized_ = false;
        bytes_.clear();
        return -5;
    }
    bytes_.erase(bytes_.begin(), bytes_.begin() + count);
    return result;
}
short PipeBuffer::poll_events(short requested, short native_events) {
    std::lock_guard<std::mutex> guard(mutex_);
    const size_t space = bytes_.size() >= capacity_ ? 0 : capacity_ - bytes_.size();
    if (space < 4096)
        native_events &= ~(POLLOUT | POLLWRNORM);
    return native_events & (requested | POLLERR | POLLHUP | POLLNVAL);
}
} // namespace operit::arm64::files
