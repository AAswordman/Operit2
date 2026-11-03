#include "base/host_io.h"
#include "linux/error_mapping.h"
#include "linux/events/event_counter.h"
#include "linux/events/inotify.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/memory_file.h"
#include "linux/files/pipe_buffer.h"
#include "linux/process/limit_enforcement.h"
#include "linux/process/pidfd.h"
#include "linux/signals/signal_file.h"
#include "linux/syscalls.h"
#include "linux/timers/timer_file.h"
#include <fcntl.h>
#include <poll.h>

namespace operit::arm64 {
int64_t Syscalls::io(bool writing, uint64_t fd, uint64_t address, uint64_t length, uint64_t offset,
                     bool positioned, bool may_block) {
    auto &entry = descriptors_.get(static_cast<uint32_t>(fd));
    files::require_descriptor_io(entry, writing);
    auto description = entry.description;
    if (description->io_uring)
        guest_error(22);
    if (description->inotify) {
        if (writing)
            guest_error(9);
        if (positioned)
            guest_error(29);
    }
    if (process::pidfd_file(description))
        guest_error(22); // The native pipe is a readiness marker, not guest data.
    if (signals::SignalFile::find(description)) {
        auto *controller = scheduler_.processes();
        if (!controller)
            guest_error(38);
        auto hooks = controller->signal_hooks();
        auto result = signals::signal_file_io(memory_, descriptors_, hooks, description, writing,
                                               address, length, positioned, may_block);
        if (!result)
            throw Failure("signalfd disappeared from its retained open description");
        return *result;
    }
    if (description->poll)
        guest_error(22);
    if ((description->timer || description->counter) && positioned)
        guest_error(29);
    if ((description->timer && writing) || ((description->timer || description->counter) && length < 8))
        guest_error(22);
    if (!length && !description->inotify)
        return 0;
    if (description->terminal) {
        check_buffer(address, length, writing ? read_perm : write_perm);
        if (positioned)
            guest_error(29);
        return terminal_io(writing, fd, address, length, may_block);
    }
    const size_t count = static_cast<size_t>(std::min<uint64_t>(length, max_io));
    if (description->virtual_file) {
        check_buffer(address, count, writing ? read_perm : write_perm);
        if (positioned && int64_t(offset) < 0)
            guest_error(22);
        std::vector<uint8_t> data(count);
        if (writing)
            memory_.copy_out(address, data.data(), count);
        size_t result = writing ? description->virtual_file->write(data.data(), count, offset, positioned)
                                : description->virtual_file->read(data.data(), count, offset, positioned);
        if (!writing && result)
            memory_.copy_in(address, data.data(), result);
        return result;
    }
    ort_arm64_io callback = positioned ? nullptr : (writing ? config_.write : config_.read);
    if (description->standard_stream < 0)
        callback = nullptr;
    auto attempt = [this, description, address, count, offset, writing, positioned,
                    limits = resource_limits(), callback,
                    tid = scheduler_.current().tid](int host_fd) -> std::optional<int64_t> {
        if (description->inotify) {
            check_buffer(address, count, write_perm);
            // Resolve every output page before try_read consumes queued records.
            for (size_t prepared = 0; prepared < count;) {
                const uint64_t cursor = address + prepared;
                memory_.translate(cursor, write_perm);
                prepared += std::min<size_t>(count - prepared, page_size - cursor % page_size);
            }
            std::vector<uint8_t> data(count);
            auto used = description->inotify->try_read(data.data(), data.size());
            if (!used)
                return {};
            if (*used)
                memory_.copy_in(address, data.data(), *used);
            return int64_t(*used);
        }
        if (writing)
            description->require_writable();
        int64_t result;
        if (description->timer) {
            check_buffer(address, 8, write_perm);
            try {
                memory_.store(address, description->timer->read(), 8);
                return 8;
            } catch (const GuestError &error) {
                if (error.number == 11)
                    return {};
                throw;
            }
        }
        if (description->counter) {
            check_buffer(address, 8, writing ? read_perm : write_perm);
            try {
                if (writing)
                    description->counter->write(memory_.load(address, 8));
                else
                    memory_.store(address, description->counter->read(host_fd), 8);
                return 8;
            } catch (const GuestError &error) {
                if (error.number == 11)
                    return {};
                throw;
            }
        }
        // Poll inherited pipe/socket streams before entering host I/O. Pipes
        // created by the guest are always native-nonblocking, independently of
        // the Linux-visible status flags.
        if (!callback && !positioned) {
            struct pollfd event{host_fd, short(writing ? POLLOUT : POLLIN), 0};
            int ready = ::poll(&event, 1, 0);
            if (ready < 0)
                return -linux_errno(errno);
            if (!ready)
                return {};
        }
        auto write_guard = [&](size_t requested) {
            return process::guard_file_write(
                *limits, host_fd, requested,
                positioned ? std::optional<uint64_t>(offset) : std::nullopt, std::nullopt,
                [this, tid] { process::send_limit_signal(scheduler_, tid, 25); });
        };
        const bool native = !callback && !(description->pipe && !positioned);
        size_t capacity = count;
        if (writing && native) {
            // Bound the guest copy first, then release the file lock. The
            // final guard below rechecks offsets, rlimit and seals after copying.
            auto guard = write_guard(capacity);
            capacity = size_t(guard.length());
        }
        check_buffer(address, capacity, writing ? read_perm : write_perm);
        std::vector<uint8_t> data(capacity);
        if (writing)
            memory_.copy_out(address, data.data(), capacity);
        if (callback)
            result = callback(config_.io_context, description->standard_stream, data.data(), capacity);
        else if (description->pipe && !positioned) {
            result = writing ? description->pipe->write(host_fd, data.data(), capacity)
                             : description->pipe->read(host_fd, data.data(), capacity);
        } else if (writing) {
            auto guard = write_guard(capacity);
            capacity = size_t(guard.length());
            if (positioned && (description->flags & 1024)) {
                // Linux positioned writes honor O_APPEND but preserve f_pos.
                // Keep memfd offset inspection, append and restoration locked.
                const off_t saved = ::lseek(host_fd, 0, SEEK_CUR);
                if (saved < 0)
                    return -linux_errno(errno);
                result = host_write(host_fd, data.data(), capacity);
                const int error = errno;
                if (::lseek(host_fd, saved, SEEK_SET) < 0)
                    return -linux_errno(errno);
                if (result < 0)
                    result = -linux_errno(error);
            } else {
                result = positioned ? ::pwrite(host_fd, data.data(), capacity, off_t(offset))
                                    : host_write(host_fd, data.data(), capacity);
                if (result < 0)
                    result = -linux_errno(errno);
            }
        } else {
            auto guard = files::memory_file_lock(host_fd);
            result = positioned ? ::pread(host_fd, data.data(), capacity, off_t(offset))
                                : ::read(host_fd, data.data(), capacity);
            if (result < 0)
                result = -linux_errno(errno);
        }
        // No file guard survives into signal delivery, guest copy-back or wait.
        if (result > static_cast<int64_t>(capacity) || result < -4095)
            throw Failure("invalid byte count or errno from host I/O callback");
        if (writing && result == -32)
            broken_pipe(tid);
        if (result == -11)
            return {};
        if (description->socket && result >= 0) {
            if (writing)
                description->socket->sent(result);
            else
                description->socket->received(result, false);
        }
        if (!writing && result > 0 && description->inotify_open)
            description->inotify_open->accessed();
        if (!writing && result > 0)
            memory_.copy_in(address, data.data(), static_cast<size_t>(result));
        return result;
    };
    if (auto result = attempt(entry.host.get()))
        return *result;
    if ((description->flags & 2048) || !may_block)
        return -11;
    // Holding an open description across a blocked syscall matches Linux:
    // close(fd) in another guest thread cannot change the in-flight operation.
    auto host =
        std::make_shared<FD>(entry.host.get() < 0 ? -1 : ::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    if (entry.host.get() >= 0)
        check_host(host->get());
    TaskDeadline deadline;
    if (description->socket) {
        uint64_t timeout =
            writing ? description->socket->send_timeout_us : description->socket->receive_timeout_us;
        if (timeout) {
            deadline = TaskClock::now() + std::chrono::microseconds(timeout);
            if (auto &restart = scheduler_.current().signals.restart)
                restart->restartable = false;
        }
    }
    if (interrupt_for_pending_signal())
        return -4;
    scheduler_.block(deadline, -11, [attempt, host]() { return attempt(host->get()); });
    return 0;
}
} // namespace operit::arm64
