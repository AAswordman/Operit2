#include "base/support.h"
#include "linux/terminal/manager.h"
#include "linux/terminal/software_pty.h"
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <poll.h>
#include <stdexcept>
#include <string>

using namespace operit::arm64;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
terminal::Attributes defaults() {
    terminal::Attributes value;
    value.input = 0x500;
    value.output = 5;
    value.control = 0xbf;
    value.local = 0x8a3b;
    value.characters = {3, 28, 127, 21, 4, 0, 1, 0, 17, 19, 26, 0, 18, 15, 23, 22, 0, 0, 0};
    value.input_speed = value.output_speed = 38400;
    return value;
}
void write(SoftwarePty &pty, bool master, const std::string &text) {
    require(pty.write(master, text.data(), text.size()) == int64_t(text.size()), "short terminal write");
}
std::string read(SoftwarePty &pty, bool master) {
    char bytes[8192];
    auto count = pty.read(master, bytes, sizeof(bytes));
    require(count >= 0, "terminal read failed");
    return std::string(bytes, size_t(count));
}
bool ready(SoftwarePty &pty, bool master) {
    pollfd event{pty.descriptor(master), POLLIN, 0};
    require(::poll(&event, 1, 0) >= 0, "terminal poll failed");
    return event.revents & POLLIN;
}
void discipline() {
    SoftwarePty pty(defaults());
    write(pty, true, "ab\x7f" "c\r");
    require(ready(pty, false), "canonical line did not wake poll");
    require(read(pty, false) == "ac\n", "canonical erase or CR translation failed");
    require(read(pty, true) == "ab\b \bc\r\n", "canonical echo failed");
    write(pty, true, "first\nsecond\n");
    require(read(pty, false) == "first\n", "canonical read crossed a line boundary");
    require(read(pty, false) == "second\n", "second canonical line lost");
    pty.flush(1);
    write(pty, true, "pending");
    require(!ready(pty, false), "incomplete canonical input became readable");
    write(pty, true, "\x04");
    require(read(pty, false) == "pending", "VEOF did not commit pending input");
    write(pty, true, "\x04");
    require(ready(pty, false) && read(pty, false).empty(), "empty VEOF did not return EOF");
    write(pty, true, "after\n");
    require(read(pty, false) == "after\n", "VEOF leaked into following input");
    pty.flush(2);
    write(pty, true, "discard\x03");
    require(!ready(pty, false), "VINTR did not flush input");
    require(read(pty, true) == "^C", "VINTR echo or output flush failed");
    write(pty, true, "\x16\x03\n");
    require(read(pty, false) == std::string("\x03\n", 2), "literal-next lost a control character");
    pty.flush(2);
    auto attributes = defaults();
    attributes.input |= 0x4000;
    pty.attributes(attributes, 0);
    write(pty, true, "a\xe4\xb8\xad\x7f\n");
    require(read(pty, false) == "a\n", "UTF-8 erase split a character");
    pty.flush(2);
    attributes.local = attributes.input = attributes.output = 0;
    attributes.characters[6] = 1;
    pty.attributes(attributes, 0);
    // Switching to raw mode must combine queued lines without leaking the
    // readiness token used by a previous empty canonical EOF.
    pty.attributes(defaults(), 0);
    write(pty, true, "one\n\x04two\n");
    pty.attributes(attributes, 0);
    require(pty.input_queued(false) == 8, "FIONREAD exposed a canonical EOF token");
    require(read(pty, false) == "one\ntwo\n", "raw transition split input or exposed an EOF token");
    pty.flush(2);
    const std::string binary("\0\r\n\x03\x7f\xff", 6);
    write(pty, true, binary);
    require(read(pty, false) == binary, "raw input changed bytes");
    require(!ready(pty, true), "raw input was echoed");
    write(pty, false, binary);
    require(read(pty, true) == binary, "raw output changed bytes");
    pty.attributes(defaults(), 0);
    write(pty, true, "\x13");
    write(pty, false, "paused\n");
    require(!ready(pty, true), "XOFF did not stop output");
    write(pty, true, "\x11");
    require(read(pty, true) == "paused\r\n", "XON did not resume output");
    pty.packet(true);
    write(pty, false, "packet");
    require(read(pty, true) == std::string("\0packet", 7), "packet data header missing");
    pty.packet(false);
    pty.close_master();
    require(read(pty, false).empty(), "master close did not hang up slave");
}
void backpressure() {
    auto attributes = defaults();
    attributes.local = attributes.input = attributes.output = 0;
    SoftwarePty pty(attributes);
    std::string data(3 * 1024 * 1024, ' '), received;
    for (size_t i = 0; i < data.size(); ++i)
        data[i] = char(i * 37);
    auto count = pty.write(false, data.data(), data.size());
    require(count > 0 && size_t(count) < data.size(), "terminal output queue is unbounded");
    size_t sent = size_t(count);
    for (unsigned attempts = 0; received.size() < data.size() && attempts < 10000; ++attempts) {
        if (sent < data.size()) {
            count = pty.write(false, data.data() + sent, data.size() - sent);
            require(count > 0 || count == -11, "backpressure returned an unexpected error");
            if (count > 0)
                sent += size_t(count);
        }
        char bytes[8192];
        count = pty.read(true, bytes, sizeof(bytes));
        require(count > 0 || count == -11, "backpressure read failed");
        if (count > 0)
            received.append(bytes, size_t(count));
    }
    require(received == data, "terminal output lost or reordered bytes under backpressure");
    SoftwarePty interrupted(defaults());
    count = interrupted.write(false, data.data(), data.size());
    require(count > 0 && size_t(count) < data.size(), "terminal did not apply backpressure");
    require(interrupted.write(true, "\x03", 1) == 1, "full output queue blocked Ctrl-C");
    require(read(interrupted, true) == "^C", "Ctrl-C did not discard blocked command output");
}
void manager(bool restricted) {
    if (restricted) {
        FD host(::posix_openpt(O_RDWR | O_NOCTTY | O_NONBLOCK));
        require(host.get() < 0 && (errno == EPERM || errno == EACCES),
                "sandbox did not deny native PTY allocation");
    }
    TerminalManager manager;
    TerminalHooks hooks;
    hooks.process = [](uint32_t pid) { return TerminalProcess{pid, pid, pid, std::nullopt}; };
    manager.set_hooks(std::move(hooks));
    auto pair = manager.open_pair();
    auto &master = *pair.first.endpoint;
    auto &slave = *pair.second.endpoint;
    if (restricted)
        require(manager.software(master), "sandbox denial did not select software PTY");
    else
        require(!manager.software(master), "unrestricted host stopped using native PTYs");
    manager.acquire(slave, 100, false);
    const char interrupt = 3;
    require(manager.write(master, pair.first.host.get(), &interrupt, 1) == 1, "master input failed");
    manager.written_input(master, &interrupt, 1);
    auto signals = manager.take_signals();
    require(signals.size() == 1 && signals[0].group == 100 && signals[0].signal == 2,
            "Ctrl-C did not signal the guest foreground group");
    winsize size{};
    size.ws_col = 101;
    size.ws_row = 37;
    manager.resize(master, size);
    require(manager.window_size(slave).ws_col == 101 && manager.window_size(slave).ws_row == 37,
            "terminal resize lost its dimensions");
    signals = manager.take_signals();
    require(signals.size() == 1 && signals[0].signal == 28, "resize did not deliver guest SIGWINCH");
    auto attributes = manager.attributes(slave);
    attributes.input = attributes.output = attributes.local = 0;
    attributes.characters[6] = 3;
    attributes.characters[5] = 0;
    manager.attributes(slave, attributes);
    manager.flush(slave, 2);
    TerminalManager::ReadState wait;
    using Status = TerminalManager::ReadStatus;
    require(manager.write(master, pair.first.host.get(), "ab", 2) == 2, "raw master write failed");
    require(manager.read_status(slave, 64, false, true, false, wait) == Status::Waiting,
            "VMIN returned before enough bytes arrived");
    require(manager.write(master, pair.first.host.get(), "c", 1) == 1, "raw master write failed");
    require(manager.read_status(slave, 64, false, true, false, wait) == Status::Ready,
            "VMIN did not complete when enough bytes arrived");
    char bytes[64];
    require(manager.read(slave, pair.second.host.get(), bytes, sizeof(bytes)) == 3 &&
                std::string(bytes, 3) == "abc", "raw manager read lost bytes");
    attributes.characters[6] = attributes.characters[5] = 0;
    manager.attributes(slave, attributes);
    require(manager.read_status(slave, 64, false, false, false, wait) == Status::Empty,
            "VMIN=VTIME=0 did not return immediately");
    attributes.characters[5] = 1;
    manager.attributes(slave, attributes);
    require(manager.read_status(slave, 64, false, false, false, wait) == Status::Waiting,
            "VTIME did not wait for its deadline");
    wait.deadline = std::chrono::steady_clock::now() - std::chrono::milliseconds(1);
    require(manager.read_status(slave, 64, false, false, false, wait) == Status::Empty,
            "VTIME deadline did not complete an empty read");
    pair.first.endpoint.reset();
    pair.first.host = FD();
    require(manager.read(slave, pair.second.host.get(), bytes, sizeof(bytes)) == 0,
            "slave did not return EOF after master close");
}
} // namespace
int main(int argc, char **argv) {
    try {
        discipline();
        backpressure();
        const bool restricted = argc == 2 && std::string(argv[1]) == "--require-software";
        manager(restricted);
        std::puts("terminal discipline: canonical/raw I/O, echo, EOF, flow, backpressure, signals, resize, VMIN/VTIME passed");
        return 0;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "unexpected guest errno %d\n", error.number);
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    }
    return 1;
}
