#include "base/host_io.h"
#include "linux/error_mapping.h"
#include "linux/syscalls.h"
#include "linux/terminal/manager.h"
#include "memory/fault.h"
#include <algorithm>
#include <cerrno>
#include <chrono>
#include <fcntl.h>
#include <poll.h>
#include <sys/ioctl.h>

namespace operit::arm64 {
namespace {
constexpr uint32_t baud_mask = 0x100f;
terminal::Attributes read_attributes(Memory &memory, uint64_t address, bool extended) {
    if (!memory.accessible(address, extended ? 44 : 36, read_perm))
        guest_error(14);
    terminal::Attributes result;
    result.input = memory.load(address, 4);
    result.output = memory.load(address + 4, 4);
    result.control = memory.load(address + 8, 4);
    result.local = memory.load(address + 12, 4);
    result.line = memory.load(address + 16, 1);
    memory.copy_out(address + 17, result.characters.data(), result.characters.size());
    if (extended) {
        result.input_speed = memory.load(address + 36, 4);
        result.output_speed = memory.load(address + 40, 4);
    } else {
        result.output_speed = terminal::speed_from_bits(result.control & baud_mask);
        const uint32_t input = (result.control >> 16) & baud_mask;
        result.input_speed = input ? terminal::speed_from_bits(input) : result.output_speed;
    }
    return result;
}
void write_attributes(Memory &memory, uint64_t address, bool extended, const terminal::Attributes &value) {
    if (!memory.accessible(address, extended ? 44 : 36, write_perm))
        guest_error(14);
    memory.store(address, value.input, 4);
    memory.store(address + 4, value.output, 4);
    memory.store(address + 8, value.control, 4);
    memory.store(address + 12, value.local, 4);
    memory.store(address + 16, value.line, 1);
    memory.copy_in(address + 17, value.characters.data(), value.characters.size());
    if (extended) {
        memory.store(address + 36, value.input_speed, 4);
        memory.store(address + 40, value.output_speed, 4);
    }
}
std::shared_ptr<FD> hold(int fd) {
    auto result = std::make_shared<FD>(::fcntl(fd, F_DUPFD_CLOEXEC, 0));
    check_host(result->get());
    return result;
}
struct Break {
    std::shared_ptr<FD> fd;
    bool asserted = false;
    TaskClock::time_point until;
    ~Break() {
        if (asserted)
            ::ioctl(fd->get(), TIOCCBRK);
    }
};
} // namespace
int64_t Syscalls::terminal_ioctl(CpuState &cpu) {
    const auto &a = cpu.x;
    auto &entry = descriptors_.get(uint32_t(a[0]));
    auto description = entry.description;
    auto endpoint = description->terminal;
    if (description->counter || description->poll || description->virtual_file || entry.host.get() < 0 ||
        (!endpoint && !::isatty(entry.host.get())))
        guest_error(25);
    const uint32_t request = uint32_t(a[1]);
    const uint32_t pid = scheduler_.current().pid;
    auto require_endpoint = [&]() -> TerminalEndpoint & {
        if (!endpoint)
            guest_error(25);
        return *endpoint;
    };
    auto store_int = [&](uint32_t value) {
        check_buffer(a[2], 4, write_perm);
        memory_.store(a[2], value, 4);
    };
    auto load_int = [&]() -> uint32_t {
        check_buffer(a[2], 4, read_perm);
        return uint32_t(memory_.load(a[2], 4));
    };
    auto control = [this, endpoint, pid] {
        if (!endpoint)
            return;
        try {
            terminals_->before_control(*endpoint, pid);
        } catch (const GuestError &error) {
            if (error.number == 4)
                interrupt_for_pending_signal();
            throw;
        }
    };
    switch (request) {
    case 0x5401:       // TCGETS
    case 0x802c542a: { // TCGETS2
        const bool extended = request == 0x802c542a;
        auto value = endpoint ? terminals_->attributes(*endpoint) : terminal::attributes(entry.host.get());
        write_attributes(memory_, a[2], extended, value);
        return 0;
    }
    case 0x5402:       // TCSETS
    case 0x5403:       // TCSETSW
    case 0x5404:       // TCSETSF
    case 0x402c542b:   // TCSETS2
    case 0x402c542c:   // TCSETSW2
    case 0x402c542d: { // TCSETSF2
        control();
        bool extended = (request & 0xffff0000) != 0;
        auto value = read_attributes(memory_, a[2], extended);
        const unsigned action = extended ? (request & 0xff) - 0x2b : request - 0x5402;
        auto host = hold(entry.host.get());
        auto attempt = [this, host, endpoint, description, value, action, control]() -> std::optional<int64_t> {
            control();
            if (action && (endpoint ? terminals_->output_queued(*endpoint)
                                    : terminal::output_queued(host->get())))
                return {};
            if (endpoint)
                terminals_->attributes(*endpoint, value, action);
            else
                terminal::set_attributes(host->get(), value, action);
            return 0;
        };
        if (auto result = attempt())
            return *result;
        if (interrupt_for_pending_signal())
            return -4;
        scheduler_.block({}, -4, std::move(attempt));
        return 0;
    }
    case 0x5409:   // TCSBRK: nonzero argument drains without sending a break.
    case 0x5425: { // TCSBRKP: duration is in deciseconds.
        control();
        if (request == 0x5425 && int32_t(a[2]) < 0)
            guest_error(22);
        uint64_t milliseconds =
            request == 0x5409 ? (a[2] ? 0 : 250) : (a[2] ? uint64_t(uint32_t(a[2])) * 100 : 250);
        auto state = std::make_shared<Break>();
        state->fd = hold(entry.host.get());
        auto attempt = [this, state, milliseconds, description, endpoint, control]() -> std::optional<int64_t> {
            control();
            if (!state->asserted) {
                if (endpoint ? terminals_->output_queued(*endpoint)
                             : terminal::output_queued(state->fd->get()))
                    return {};
                if (!milliseconds)
                    return 0;
                check_host(::ioctl(state->fd->get(), TIOCSBRK));
                state->asserted = true;
                state->until = TaskClock::now() + std::chrono::milliseconds(milliseconds);
            }
            if (TaskClock::now() < state->until)
                return {};
            int result = ::ioctl(state->fd->get(), TIOCCBRK);
            check_host(result);
            state->asserted = false;
            return 0;
        };
        if (auto result = attempt())
            return *result;
        if (interrupt_for_pending_signal())
            return -4;
        scheduler_.block({}, -4, std::move(attempt));
        return 0;
    }
    case 0x540a: // TCXONC
        control();
        terminal::flow(entry.host.get(), uint32_t(a[2]));
        return 0;
    case 0x540b: // TCFLSH
        control();
        if (endpoint)
            terminals_->flush(*endpoint, uint32_t(a[2]));
        else
            terminal::flush(entry.host.get(), uint32_t(a[2]));
        return 0;
    case 0x540c: // TIOCEXCL
    case 0x540d: // TIOCNXCL
        if (endpoint)
            terminals_->exclusive(*endpoint, request == 0x540c);
        else
            terminal::exclusive(entry.host.get(), request == 0x540c);
        return 0;
    case 0x540e: // TIOCSCTTY: never acquire or modify the embedding host session.
        terminals_->acquire(require_endpoint(), pid, int32_t(a[2]) == 1);
        return 0;
    case 0x540f: // TIOCGPGRP
        store_int(terminals_->foreground_group(require_endpoint(), pid));
        return 0;
    case 0x5410: // TIOCSPGRP
        try {
            control();
        } catch (const GuestError &error) {
            if (error.number == 5)
                guest_error(25);
            throw;
        }
        terminals_->foreground_group(require_endpoint(), pid, load_int());
        return 0;
    case 0x5411: // TIOCOUTQ
        store_int(terminal::output_queued(entry.host.get()));
        return 0;
    case 0x5412: // TIOCSTI: inject through this guest PTY's master, not host TIOCSTI.
        check_buffer(a[2], 1, read_perm);
        terminals_->inject(require_endpoint(), pid, memory_.load(a[2], 1));
        return 0;
    case 0x5413:   // TIOCGWINSZ
    case 0x5414: { // TIOCSWINSZ
        const bool setting = request == 0x5414;
        check_buffer(a[2], 8, setting ? read_perm : write_perm);
        struct winsize value{};
        if (setting) {
            value.ws_row = memory_.load(a[2], 2);
            value.ws_col = memory_.load(a[2] + 2, 2);
            value.ws_xpixel = memory_.load(a[2] + 4, 2);
            value.ws_ypixel = memory_.load(a[2] + 6, 2);
            if (endpoint)
                terminals_->resize(*endpoint, value);
            else
                check_host(::ioctl(entry.host.get(), TIOCSWINSZ, &value));
        } else {
            if (endpoint)
                value = terminals_->window_size(*endpoint);
            else
                check_host(::ioctl(entry.host.get(), TIOCGWINSZ, &value));
            memory_.store(a[2], value.ws_row, 2);
            memory_.store(a[2] + 2, value.ws_col, 2);
            memory_.store(a[2] + 4, value.ws_xpixel, 2);
            memory_.store(a[2] + 6, value.ws_ypixel, 2);
        }
        return 0;
    }
    case 0x5415: // TIOCMGET
        store_int(terminal::modem_bits(entry.host.get()));
        return 0;
    case 0x5416: // TIOCMBIS
    case 0x5417: // TIOCMBIC
    case 0x5418: // TIOCMSET
        terminal::change_modem_bits(entry.host.get(), load_int(),
                                    request == 0x5418   ? 0
                                    : request == 0x5416 ? 1
                                                        : 2);
        return 0;
    case 0x5420: { // TIOCPKT
        uint32_t value = load_int();
        if (value > 1)
            guest_error(22);
        terminals_->packet(require_endpoint(), value != 0);
        return 0;
    }
    case 0x5422: // TIOCNOTTY
        terminals_->detach(require_endpoint(), pid);
        return 0;
    case 0x5423: // TIOCSETD
        control();
        terminal::line_discipline(entry.host.get(), load_int());
        return 0;
    case 0x5424: // TIOCGETD
        store_int(terminal::line_discipline(entry.host.get()));
        return 0;
    case 0x5427: // TIOCSBRK
    case 0x5428: // TIOCCBRK
        control();
        check_host(::ioctl(entry.host.get(), request == 0x5427 ? TIOCSBRK : TIOCCBRK));
        return 0;
    case 0x5429: // TIOCGSID
        store_int(terminals_->session(require_endpoint(), pid));
        return 0;
    case 0x80045430: // TIOCGPTN
        store_int(terminals_->number(require_endpoint()));
        return 0;
    case 0x40045431: // TIOCSPTLCK
        terminals_->lock(require_endpoint(), load_int() != 0);
        return 0;
    case 0x80045432: // TIOCGDEV
        store_int(terminals_->device(require_endpoint()));
        return 0;
    case 0x40045436: // TIOCSIG takes the Linux signal number as the argument value.
        terminals_->send_signal(require_endpoint(), uint32_t(a[2]));
        return 0;
    case 0x80045438: // TIOCGPKT
        store_int(terminals_->packet(require_endpoint()));
        return 0;
    case 0x80045439: // TIOCGPTLCK
        store_int(terminals_->locked(require_endpoint()));
        return 0;
    case 0x80045440: // TIOCGEXCL
        store_int(terminals_->exclusive(require_endpoint()));
        return 0;
    case 0x5441: { // TIOCGPTPEER
        if (a[2] & ~uint64_t{3 | 256 | 2048 | 524288})
            guest_error(22);
        auto peer = terminals_->open_slave(terminals_->number(require_endpoint()), int(a[2]));
        terminals_->opened(*peer.endpoint, pid, a[2] & 256);
        int fd = descriptors_.insert(std::move(peer.host), peer.path, int(a[2]), 0);
        auto &opened = *descriptors_.get(fd).description;
        opened.terminal = std::move(peer.endpoint);
        opened.scheduler_io = true;
        return fd;
    }
    default:
        guest_error(25);
    }
}
int64_t Syscalls::terminal_io(bool writing, uint64_t fd, uint64_t address, uint64_t length, bool may_block) {
    auto &entry = descriptors_.get(uint32_t(fd));
    auto description = entry.description;
    auto endpoint = description->terminal;
    if (!endpoint)
        guest_error(25);
    if ((description->flags & 3) == (writing ? 0 : 1))
        guest_error(9);
    if (!length)
        return 0;
    check_buffer(address, length, writing ? read_perm : write_perm);
    const size_t capacity = size_t(std::min<uint64_t>(length, max_io));
    auto bytes = std::make_shared<std::vector<uint8_t>>(capacity);
    auto host = hold(entry.host.get());
    uint32_t pid = scheduler_.current().pid;
    auto wait = std::make_shared<TerminalManager::ReadState>();
    auto attempt = [this, host, endpoint, description, bytes, wait, writing, address, length, capacity, pid,
                    may_block]() -> std::optional<int64_t> {
        const bool nonblocking = (description->flags & 2048) || !may_block;
        check_buffer(address, length, writing ? read_perm : write_perm);
        try {
            terminals_->before_io(*endpoint, pid, writing);
        } catch (const GuestError &error) {
            if (error.number == 4)
                interrupt_for_pending_signal();
            throw;
        }
        pollfd event{host->get(), short(writing ? POLLOUT : POLLIN), 0};
        int ready = ::poll(&event, 1, 0);
        if (ready < 0)
            return -linux_errno(errno);
        if (event.revents & POLLNVAL)
            return -9;
        const bool hungup = event.revents & (POLLHUP | POLLERR);
        if (!writing) {
            const auto status = terminals_->read_status(*endpoint, capacity, nonblocking,
                                                        event.revents & POLLIN, hungup, *wait);
            if (status == TerminalManager::ReadStatus::Empty)
                return 0;
            if (status == TerminalManager::ReadStatus::Waiting)
                return nonblocking ? std::optional<int64_t>(-11) : std::nullopt;
        } else if (!ready && !hungup)
            return nonblocking ? std::optional<int64_t>(-11) : std::nullopt;
        int64_t result;
        if (writing) {
            memory_.copy_out(address, bytes->data(), capacity);
            result = host_write(host->get(), bytes->data(), capacity);
        } else
            result = ::read(host->get(), bytes->data(), capacity);
        if (result < 0) {
            int error = linux_errno(errno);
            result = -error;
        }
        if (writing && result > 0)
            terminals_->written_input(*endpoint, bytes->data(), size_t(result));
        if (!writing) {
            result = terminals_->read_result(*endpoint, bytes->data(), result);
            if (result > 0)
                memory_.copy_in(address, bytes->data(), size_t(result));
        }
        if (result == -11 && !nonblocking)
            return {};
        return result;
    };
    if (auto result = attempt())
        return *result;
    if ((description->flags & 2048) || !may_block)
        return -11;
    // Use the same read attempt for ordinary readiness, VTIME and signal-interrupted short reads.
    auto complete_read = [attempt, wait]() -> std::optional<int64_t> {
        wait->interrupted = true;
        try {
            return attempt();
        } catch (const MemoryFault &) {
            return -14;
        }
    };
    if (interrupt_for_pending_signal()) {
        if (!writing)
            if (auto result = complete_read())
                return *result;
        return -4;
    }
    if (!writing) {
        const auto tid = scheduler_.current().tid;
        terminals_->register_read_interruption(tid, std::move(complete_read));
        scheduler_.current().wait_cleanup = [manager = std::weak_ptr<TerminalManager>(terminals_), tid] {
            if (const auto live = manager.lock())
                live->cancel_read(tid);
        };
    }
    scheduler_.block({}, -4, std::move(attempt));
    return 0;
}
} // namespace operit::arm64
