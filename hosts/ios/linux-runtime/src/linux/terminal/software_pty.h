#pragma once
#include "base/fd.h"
#include "linux/terminal/termios.h"
#include <deque>
#include <mutex>
#include <string>
#include <sys/ioctl.h>

namespace operit::arm64 {
// Socket descriptors carry readiness and bytes; the guest line discipline never
// opens a host terminal device or changes the embedding process's termios.
class SoftwarePty {
    mutable std::mutex mutex_;
    FD master_, slave_;
    terminal::Attributes attributes_;
    winsize size_{};
    std::string line_, input_, output_;
    std::deque<size_t> records_;
    bool literal_ = false, stopped_ = false, packet_ = false;
    unsigned column_ = 0;
    void pump();
    void output(uint8_t byte);
    void echo(uint8_t byte);
    void input(uint8_t byte);
    void commit(bool eof = false);
    void flush_locked(unsigned action);

  public:
    explicit SoftwarePty(const terminal::Attributes &attributes);
    int descriptor(bool master) const;
    int64_t read(bool master, void *bytes, size_t length);
    int64_t write(bool master, const void *bytes, size_t length);
    terminal::Attributes attributes() const;
    void attributes(const terminal::Attributes &value, unsigned action);
    winsize window_size() const;
    void resize(const winsize &size);
    void flush(unsigned action);
    void flow(unsigned action);
    int input_queued(bool master) const;
    int output_queued() const;
    void packet(bool enabled);
    void close_master();
};
} // namespace operit::arm64
