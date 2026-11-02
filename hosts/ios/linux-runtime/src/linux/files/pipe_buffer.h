#pragma once
#include <cstddef>
#include <cstdint>
#include <deque>
#include <memory>
#include <mutex>

namespace operit::arm64::files {
// A mirror of a guest-created pipe's real kernel byte queue. Every guest pipe
// read/write goes through this object; native FDs retain their actual bytes and
// their native EOF, capacity and poll/epoll readiness. tee can inspect the mirror
// without reading or freeing space in its source pipe.
class PipeBuffer {
    std::mutex mutex_;
    std::deque<uint8_t> bytes_;
    size_t capacity_;
    bool synchronized_ = true;
    explicit PipeBuffer(size_t capacity) : capacity_(capacity) {}
    bool synchronized_from_reader(int fd);

  public:
    static std::shared_ptr<PipeBuffer> create(int reader, int writer);
    size_t capacity() const {
        return capacity_;
    }
    int64_t read(int fd, void *data, size_t count);
    int64_t write(int fd, const void *data, size_t count);
    int64_t peek(int fd, void *data, size_t count);
    int64_t consume(int fd, const void *expected, size_t count);
    // Restrict native write readiness to the logical pipe capacity. Native read
    // readiness never needs synthesis because tee leaves every source byte put.
    short poll_events(short requested, short native_events);
};
} // namespace operit::arm64::files
