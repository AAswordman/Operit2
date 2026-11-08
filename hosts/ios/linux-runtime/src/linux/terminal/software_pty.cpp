#include "linux/terminal/software_pty.h"
#include "base/host_io.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <fcntl.h>
#include <sys/socket.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
constexpr size_t queue_limit = 1024 * 1024, line_limit = 4096;
void drain(int fd) {
    char bytes[4096];
    while (::read(fd, bytes, sizeof(bytes)) > 0) {}
}
void send_pending(int fd, std::string &queue) {
    while (!queue.empty()) {
        auto count = host_write(fd, queue.data(), queue.size());
        if (count < 0) {
            if (errno == EINTR)
                continue;
            break;
        }
        if (!count)
            break;
        queue.erase(0, size_t(count));
    }
}
} // namespace
SoftwarePty::SoftwarePty(const terminal::Attributes &attributes) : attributes_(attributes) {
    int pair[2];
    check_host(::socketpair(AF_UNIX, SOCK_STREAM, 0, pair));
    master_ = FD(pair[0]);
    slave_ = FD(pair[1]);
    for (int fd : pair) {
        check_host(::fcntl(fd, F_SETFL, O_NONBLOCK));
        check_host(::fcntl(fd, F_SETFD, FD_CLOEXEC));
#ifdef F_SETNOSIGPIPE
        check_host(::fcntl(fd, F_SETNOSIGPIPE, 1));
#endif
    }
}
int SoftwarePty::descriptor(bool master) const {
    return master ? master_.get() : slave_.get();
}
void SoftwarePty::pump() {
    send_pending(master_.get(), input_);
    if (!stopped_)
        send_pending(slave_.get(), output_);
}
void SoftwarePty::output(uint8_t byte) {
    if (attributes_.output & 1) { // OPOST
        if ((attributes_.output & 2) && byte >= 'a' && byte <= 'z')
            byte -= 'a' - 'A';
        if (byte == '\r' && (attributes_.output & 8)) // OCRNL
            byte = '\n';
        else if (byte == '\r' && !column_ && (attributes_.output & 0x10)) // ONOCR
            return;
        if (byte == '\n' && (attributes_.output & 4)) { // ONLCR
            output_ += '\r';
            column_ = 0;
        }
        if (byte == '\t' && (attributes_.output & 0x1800) == 0x1800) {
            unsigned count = 8 - column_ % 8;
            output_.append(count, ' ');
            column_ += count;
            return;
        }
    }
    output_ += char(byte);
    if (byte == '\r' || (byte == '\n' && (attributes_.output & 0x20)))
        column_ = 0;
    else if (byte == '\b' && column_)
        --column_;
    else if (byte == '\t')
        column_ += 8 - column_ % 8;
    else if (byte >= ' ' && byte != 127)
        ++column_;
}
void SoftwarePty::echo(uint8_t byte) {
    if ((attributes_.local & 0x200) && (byte < 32 || byte == 127) && byte != '\n' && byte != '\t') {
        output('^');
        output(byte == 127 ? '?' : byte + 64);
    } else
        output(byte);
}
void SoftwarePty::commit(bool eof) {
    if (!line_.empty()) {
        records_.push_back(line_.size());
        input_ += line_;
        line_.clear();
    } else if (eof) {
        // A byte wakes poll for an empty VEOF record. read() consumes this token
        // as a zero-length record; it is never exposed to the guest.
        records_.push_back(0);
        input_ += '\0';
    }
}
void SoftwarePty::input(uint8_t byte) {
    const auto &a = attributes_;
    const bool canonical = (a.local & 2) && !(a.local & 0x10000);
    auto special = [&](unsigned slot) { return a.characters[slot] && byte == a.characters[slot]; };
    if (!(a.local & 0x10000)) {
        if (a.input & 0x20) // ISTRIP
            byte &= 0x7f;
        if (!literal_) {
            if (a.input & 0x400) { // IXON
                if (special(9)) { stopped_ = true; return; }
                if (special(8)) { stopped_ = false; return; }
                if (a.input & 0x800)
                    stopped_ = false;
            }
            if ((a.local & 1) && (special(0) || special(1) || special(10))) {
                if (!(a.local & 0x80)) // NOFLSH
                    flush_locked(2);
                if (a.local & 8)
                    echo(byte);
                return; // TerminalManager delivers the guest foreground signal.
            }
            if (byte == '\r') {
                if (a.input & 0x80)
                    return;
                if (a.input & 0x100)
                    byte = '\n';
            } else if (byte == '\n' && (a.input & 0x40))
                byte = '\r';
            if (canonical) {
                auto erase = [&] {
                    if (line_.empty()) return;
                    uint8_t removed = uint8_t(line_.back());
                    line_.pop_back();
                    if (a.input & 0x4000) // IUTF8: erase a whole UTF-8 code point.
                        while ((removed & 0xc0) == 0x80 && !line_.empty()) {
                            removed = uint8_t(line_.back());
                            line_.pop_back();
                        }
                    if (a.local & 8) {
                        if (a.local & 0x10) {
                            output('\b'); output(' '); output('\b');
                        } else
                            echo(byte);
                    }
                };
                if (special(2)) { erase(); return; } // VERASE
                if (special(3)) { // VKILL
                    if ((a.local & 0x818) == 0x818)
                        while (!line_.empty()) erase();
                    else {
                        line_.clear();
                        if (a.local & 8) {
                            echo(byte);
                            if (a.local & 0x20) output('\n');
                        }
                    }
                    return;
                }
                if ((a.local & 0x8000) && special(14)) { // VWERASE
                    while (!line_.empty() && (line_.back() == ' ' || line_.back() == '\t')) erase();
                    while (!line_.empty() && line_.back() != ' ' && line_.back() != '\t') erase();
                    return;
                }
                if ((a.local & 0x8000) && special(15)) { // VLNEXT
                    literal_ = true;
                    if (a.local & 8) { output('^'); output('\b'); }
                    return;
                }
                if (special(4)) { commit(true); return; } // VEOF
            }
        }
    }
    const bool quoted = literal_;
    literal_ = false;
    if (!(a.local & 0x10000) && ((a.local & 8) || (canonical && byte == '\n' && (a.local & 0x40))))
        echo(byte);
    if (canonical) {
        const bool delimiter = !quoted && (byte == '\n' || special(11) || ((a.local & 0x8000) && special(16)));
        if (line_.size() < line_limit - 1 || delimiter)
            line_ += char(byte);
        if (delimiter)
            commit();
    } else {
        input_ += char(byte);
        if (!records_.empty() && records_.back())
            ++records_.back();
        else
            records_.push_back(1);
    }
}
int64_t SoftwarePty::write(bool master, const void *bytes, size_t length) {
    std::lock_guard<std::mutex> lock(mutex_);
    pump();
    const auto *data = static_cast<const uint8_t *>(bytes);
    size_t used = 0;
    // Reserve room for echo/tab/newline expansion before accepting each byte.
    while (used < length) {
        uint8_t byte = data[used];
        if (attributes_.input & 0x20)
            byte &= 0x7f;
        auto special = [&](unsigned slot) {
            return attributes_.characters[slot] && byte == attributes_.characters[slot];
        };
        // Signals and XON must remain usable when queued output/input is full.
        // In particular, Ctrl-C must be able to flush a command's blocked output.
        const bool control = master && !literal_ && !(attributes_.local & 0x10000) &&
            (((attributes_.local & 1) && (special(0) || special(1) || special(10))) ||
             ((attributes_.input & 0x400) && (special(8) || special(9))));
        if (!control && (output_.size() >= queue_limit - 8 ||
            (master && (input_.size() + line_.size() >= queue_limit - line_limit ||
                        records_.size() >= queue_limit / sizeof(size_t)))))
            break;
        if (master)
            input(data[used]);
        else
            output(data[used]);
        ++used;
    }
    pump();
    return used || !length ? int64_t(used) : -11;
}
int64_t SoftwarePty::read(bool master, void *bytes, size_t length) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (!length) return 0;
    pump();
    auto *data = static_cast<uint8_t *>(bytes);
    const size_t header = master && packet_ ? 1 : 0;
    if (header && length == 1) {
        int queued = 0;
        check_host(::ioctl(master_.get(), FIONREAD, &queued));
        if (!queued) return -11;
        data[0] = 0;
        return 1;
    }
    const size_t capacity = length - header;
    int64_t count = 0;
    if (master) {
        count = ::read(master_.get(), data + header, capacity);
        if (count < 0)
            return -linux_errno(errno);
    } else {
        const bool canonical = (attributes_.local & 2) && !(attributes_.local & 0x10000);
        while (size_t(count) < capacity) {
            if (!records_.empty() && !records_.front()) {
                if (canonical && count)
                    break;
                uint8_t token;
                auto result = ::read(slave_.get(), &token, 1);
                if (result <= 0)
                    return count ? count : result < 0 ? -linux_errno(errno) : 0;
                records_.pop_front();
                pump();
                if (canonical)
                    return 0;
                continue; // A queued canonical EOF is not a raw NUL byte.
            }
            size_t wanted = capacity - size_t(count);
            if (!records_.empty())
                wanted = std::min(wanted, records_.front());
            auto result = ::read(slave_.get(), data + count, wanted);
            if (result <= 0)
                return count ? count : result < 0 ? -linux_errno(errno) : 0;
            count += result;
            if (!records_.empty()) {
                records_.front() -= size_t(result);
                if (!records_.front())
                    records_.pop_front();
            }
            pump();
            if (canonical)
                break;
        }
    }
    if (header && count > 0) data[0] = 0; // Linux TIOCPKT_DATA.
    pump();
    return count > 0 ? count + int64_t(header) : count;
}
terminal::Attributes SoftwarePty::attributes() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return attributes_;
}
void SoftwarePty::attributes(const terminal::Attributes &value, unsigned action) {
    if (action > 2 || value.line) guest_error(22);
    auto next = value;
    uint32_t out = value.control & 0x100f, in = (value.control >> 16) & 0x100f;
    next.output_speed = out == 0x1000 ? value.output_speed : terminal::speed_from_bits(out);
    next.input_speed = !in ? next.output_speed : in == 0x1000 ? value.input_speed : terminal::speed_from_bits(in);
    std::lock_guard<std::mutex> lock(mutex_);
    if (action == 2) flush_locked(0);
    if ((attributes_.local ^ next.local) & 0x10002) literal_ = false;
    attributes_ = next;
    if (!(next.local & 2) || (next.local & 0x10000)) commit();
    pump();
}
winsize SoftwarePty::window_size() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return size_;
}
void SoftwarePty::resize(const winsize &size) {
    std::lock_guard<std::mutex> lock(mutex_);
    size_ = size;
}
void SoftwarePty::flush_locked(unsigned action) {
    if (action != 1) {
        drain(slave_.get()); input_.clear(); line_.clear(); records_.clear(); literal_ = false;
    }
    if (action != 0) { drain(master_.get()); output_.clear(); }
}
void SoftwarePty::flush(unsigned action) {
    if (action > 2) guest_error(22);
    std::lock_guard<std::mutex> lock(mutex_);
    flush_locked(action);
}
void SoftwarePty::flow(unsigned action) {
    if (action > 3) guest_error(22);
    std::lock_guard<std::mutex> lock(mutex_);
    if (action < 2) stopped_ = action == 0;
    else if (uint8_t byte = attributes_.characters[action == 2 ? 9 : 8]) output_ += char(byte);
    pump();
}
int SoftwarePty::input_queued(bool master) const {
    std::lock_guard<std::mutex> lock(mutex_);
    int available = 0;
    check_host(::ioctl(descriptor(master), FIONREAD, &available));
    if (master)
        return available;
    // Empty canonical EOF records use readiness tokens, not guest input bytes.
    size_t physical = size_t(available);
    for (size_t record : records_) {
        if (!physical)
            break;
        if (!record)
            --available;
        physical -= std::min(physical, std::max<size_t>(record, 1));
    }
    return available;
}
int SoftwarePty::output_queued() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return int(output_.size());
}
void SoftwarePty::packet(bool enabled) {
    std::lock_guard<std::mutex> lock(mutex_);
    packet_ = enabled;
}
void SoftwarePty::close_master() {
    std::lock_guard<std::mutex> lock(mutex_);
    ::shutdown(master_.get(), SHUT_RDWR);
}
} // namespace operit::arm64
