#include "runtime/machine.h"
#include <new>

namespace operit::arm64 {
namespace {
template <class Operation>
int64_t console_operation(Machine &machine, const char *name, Operation &&operation) {
    try {
        const auto result = operation();
        if (result < 0)
            machine.error = std::string(name) + " failed with Linux errno " + std::to_string(-result);
        else
            machine.error.clear();
        return result;
    } catch (const GuestError &failure) {
        machine.error = std::string(name) + " failed with Linux errno " + std::to_string(failure.number);
        return -failure.number;
    } catch (const std::bad_alloc &failure) {
        machine.error = failure.what();
        return -12;
    } catch (const std::exception &failure) {
        machine.error = failure.what();
        return -5;
    } catch (...) {
        machine.error = std::string(name) + " failed";
        return -5;
    }
}
} // namespace
int Machine::terminal_open(uint32_t columns, uint32_t rows) {
    const auto result = console_operation(*this, "terminal open", [&]() -> int64_t {
        if (!loaded_ || started_ || main_exited_ || failed_)
            guest_error(22);
        if (console_)
            guest_error(16);
        auto &syscalls = *processes_.at(1)->image.syscalls;
        auto master = syscalls.open_console(columns, rows);
        console_.emplace(syscalls.terminal_manager(), std::move(master));
        return 0;
    });
    return result < 0 ? -1 : 0;
}
int64_t Machine::terminal_read(void *bytes, size_t length) {
    return console_operation(*this, "terminal read", [&]() -> int64_t {
        if (!console_)
            guest_error(9);
        return console_->read(bytes, length);
    });
}
int64_t Machine::terminal_write(const void *bytes, size_t length) {
    return console_operation(*this, "terminal write", [&]() -> int64_t {
        if (!console_)
            guest_error(9);
        return console_->write(bytes, length);
    });
}
int Machine::terminal_resize(uint32_t columns, uint32_t rows) {
    const auto result = console_operation(*this, "terminal resize", [&]() -> int64_t {
        if (!console_)
            guest_error(9);
        console_->resize(columns, rows);
        return 0;
    });
    return result < 0 ? -1 : 0;
}
int Machine::terminal_signal(int number) {
    const auto result = console_operation(*this, "terminal signal", [&]() -> int64_t {
        if (!console_)
            guest_error(9);
        console_->signal(number);
        return 0;
    });
    return result < 0 ? -1 : 0;
}
int Machine::terminal_close() {
    const auto result = console_operation(*this, "terminal close", [&]() -> int64_t {
        if (!console_)
            guest_error(9);
        if (!main_exited_)
            terminate(1, 9);
        console_.reset();
        return 0;
    });
    return result < 0 ? -1 : 0;
}
} // namespace operit::arm64
