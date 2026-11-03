#include "linux/error_mapping.h"
#include "linux/events/epoll.h"
#include "linux/events/inotify.h"
#include "linux/files/pipe_buffer.h"
#include "linux/io_uring/io_uring.h"
#include "linux/signals/signal_file.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <fcntl.h>
#include <poll.h>

namespace operit::arm64 {
namespace {
short native_events(uint16_t value) {
    short result = 0;
    if (value & (1 | 64))
        result |= POLLIN;
    if (value & (2 | 128))
        result |= POLLPRI;
    if (value & (4 | 256 | 512))
        result |= POLLOUT;
    return result;
}
uint16_t guest_events(short value, uint16_t requested) {
    uint16_t result = 0;
    if (value & POLLIN)
        result |= requested & (1 | 64);
    if (value & POLLPRI)
        result |= requested & (2 | 128);
    if (value & POLLOUT)
        result |= requested & (4 | 256 | 512);
    if (value & POLLERR)
        result |= 8;
    if (value & POLLHUP)
        result |= 16;
    if (value & POLLNVAL)
        result |= 32;
    return result;
}
struct PollEntry {
    int fd;
    uint16_t events;
    std::shared_ptr<FD> host;
    std::shared_ptr<OpenDescription> description;
};
} // namespace
int64_t Syscalls::dispatch_poll(CpuState &cpu) {
    const auto &a = cpu.x;
    bool select = a[8] == 72;
    if (a[1] > 1024 && !select)
        guest_error(22);
    if (a[0] > 1024 && select)
        guest_error(22);
    uint64_t timeout_address = select ? a[4] : a[2];
    TaskDeadline deadline;
    if (timeout_address) {
        check_buffer(timeout_address, 16, read_perm | write_perm);
        int64_t seconds = memory_.load(timeout_address, 8), nanos = memory_.load(timeout_address + 8, 8);
        if (seconds < 0 || nanos < 0 || nanos >= 1000000000)
            guest_error(22);
        auto maximum =
            std::chrono::duration_cast<std::chrono::seconds>(TaskClock::time_point::max() - TaskClock::now())
                .count();
        if (seconds > maximum - 1)
            deadline = TaskClock::time_point::max();
        else
            deadline = TaskClock::now() + std::chrono::seconds(seconds) + std::chrono::nanoseconds(nanos);
    }
    uint64_t set_size = select ? ((a[0] + 63) / 64) * 8 : 0;
    auto entries = std::make_shared<std::vector<PollEntry>>();
    auto add = [this, entries, select](int fd, uint16_t events) {
        PollEntry item{fd, events, {}, {}};
        if (fd >= 0) {
            try {
                auto &entry = descriptors_.get(fd);
                item.description = entry.description;
                int native = entry.host.get();
                if (item.description->poll)
                    native = item.description->poll->native_handle();
                if (native >= 0) {
                    item.host = std::make_shared<FD>(::fcntl(native, F_DUPFD_CLOEXEC, 0));
                    check_host(item.host->get());
                }
            } catch (const GuestError &error) {
                if (select || error.number != 9)
                    throw;
            }
        }
        entries->push_back(std::move(item));
    };
    if (select) {
        for (unsigned i = 1; i <= 3; ++i)
            if (a[i])
                check_buffer(a[i], set_size, read_perm | write_perm);
        for (uint64_t fd = 0; fd < a[0]; ++fd) {
            uint16_t events = 0;
            for (unsigned i = 1; i <= 3; ++i)
                if (a[i] && (memory_.load(a[i] + (fd / 64) * 8, 8) & (uint64_t{1} << (fd % 64))))
                    events |= i == 1 ? 1 : i == 2 ? 4 : 2;
            if (events)
                add(fd, events);
        }
    } else {
        check_buffer(a[0], a[1] * 8, read_perm | write_perm);
        for (uint64_t i = 0; i < a[1]; ++i)
            add(int32_t(memory_.load(a[0] + i * 8, 4)), memory_.load(a[0] + i * 8 + 4, 2));
    }
    uint64_t mask_pointer = select ? 0 : a[3];
    uint64_t mask_size = select ? 8 : a[4];
    if (select && a[5]) {
        check_buffer(a[5], 16, read_perm);
        mask_pointer = memory_.load(a[5], 8);
        mask_size = memory_.load(a[5] + 8, 8);
    }
    auto &task = scheduler_.current();
    if (mask_pointer) {
        if (mask_size != 8)
            guest_error(22);
        check_buffer(mask_pointer, 8, read_perm);
        signals::begin_temporary_mask(task.signals, task.signal_mask, memory_.load(mask_pointer, 8));
    }
    auto attempt = [this, entries, select, args = a, set_size, deadline,
                    timeout_address]() -> std::optional<int64_t> {
        auto hooks = scheduler_.processes() ? scheduler_.processes()->signal_hooks() : signals::Hooks{};
        InotifyFile::service_all();
        signals::signal_files_safe_point(descriptors_, hooks);
        std::vector<pollfd> native;
        std::vector<std::optional<short>> local_ready;
        for (const auto &item : *entries) {
            if (item.description && item.description->io_uring)
                io_uring::refresh_readiness(item.description->io_uring);
            // A blocked poll retains its description even after another thread
            // closes the guest FD, so refresh these retained files as well.
            auto ready = signals::signal_file_poll(item.description, hooks, native_events(item.events));
            if (item.description && item.description->inotify)
                ready = item.description->inotify->poll_events(native_events(item.events));
            local_ready.push_back(ready);
            native.push_back({item.host ? item.host->get() : -1, native_events(item.events), 0});
        }
        if (::poll(native.data(), native.size(), 0) < 0)
            return -linux_errno(errno);
        std::vector<uint16_t> results(entries->size());
        int ready = 0;
        for (size_t i = 0; i < entries->size(); ++i) {
            const auto &item = (*entries)[i];
            short native_ready = local_ready[i].value_or(native[i].revents);
            if (item.description && item.description->poll)
                native_ready = item.description->poll->poll_events(native_events(item.events));
            if (item.description && item.description->pipe)
                native_ready = item.description->pipe->poll_events(native_events(item.events), native_ready);
            uint16_t flags = guest_events(native_ready, item.events);
            if (item.fd >= 0 && !item.description)
                flags = 32;
            auto mq_ready = ipc::posix_mq_poll(item.description, item.events);
            if (mq_ready)
                flags = uint16_t(*mq_ready);
            else if (item.description && item.description->virtual_file)
                flags = item.events & (1 | 4 | 64 | 256);
            results[i] = flags;
            if (flags)
                ++ready;
        }
        auto now = TaskClock::now();
        if (!ready && (!deadline || now < *deadline))
            return {};
        if (select) {
            for (unsigned i = 1; i <= 3; ++i)
                if (args[i]) {
                    check_buffer(args[i], set_size, write_perm);
                    std::vector<uint8_t> empty(set_size);
                    memory_.copy_in(args[i], empty.data(), empty.size());
                }
            ready = 0;
            for (size_t i = 0; i < entries->size(); ++i) {
                auto &item = (*entries)[i];
                for (unsigned set = 1; set <= 3; ++set) {
                    uint16_t bits = set == 1 ? 1 | 8 | 16 : set == 2 ? 4 | 8 : 2;
                    if (args[set] && (results[i] & bits) &&
                        (item.events & (set == 1   ? 1
                                        : set == 2 ? 4
                                                   : 2))) {
                        uint64_t address = args[set] + (item.fd / 64) * 8;
                        memory_.store(address, memory_.load(address, 8) | (uint64_t{1} << (item.fd % 64)), 8);
                        ++ready;
                    }
                }
            }
        } else {
            check_buffer(args[0], args[1] * 8, write_perm);
            for (size_t i = 0; i < results.size(); ++i)
                memory_.store(args[0] + i * 8 + 6, results[i], 2);
        }
        if (timeout_address && deadline) {
            check_buffer(timeout_address, 16, write_perm);
            int64_t nanos = std::max<int64_t>(
                0, std::chrono::duration_cast<std::chrono::nanoseconds>(*deadline - now).count());
            memory_.store(timeout_address, nanos / 1000000000, 8);
            memory_.store(timeout_address + 8, nanos % 1000000000, 8);
        }
        return ready;
    };
    try {
        if (auto result = attempt()) {
            signals::end_temporary_mask(task.signals, task.signal_mask);
            return *result;
        }
        if (interrupt_for_pending_signal()) {
            signals::end_temporary_mask(task.signals, task.signal_mask);
            return -4;
        }
        scheduler_.block({}, 0, std::move(attempt));
        task.wait_cleanup = [this, tid = task.tid] {
            auto &waiting = scheduler_.find(tid);
            signals::end_temporary_mask(waiting.signals, waiting.signal_mask);
        };
    } catch (...) {
        signals::end_temporary_mask(task.signals, task.signal_mask);
        throw;
    }
    return 0;
}
} // namespace operit::arm64
