#include "linux/files/vector_io.h"
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
#include "memory/fault.h"
#include <algorithm>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#ifndef __APPLE__
#include <sys/uio.h>
#endif
#include <functional>
#include <poll.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <utility>

namespace operit::arm64::files {
IoVectors::IoVectors(Memory &memory, uint64_t address, uint64_t count) {
    if (count > 1024)
        guest_error(22);
    if (!memory.accessible(address, count * sizeof(linux_abi::Iovec), read_perm))
        guest_error(14);
    std::vector<linux_abi::Iovec> imported(count);
    memory.copy_out(address, imported.data(), imported.size() * sizeof(linux_abi::Iovec));
    entries_.reserve(imported.size());
    for (const auto &entry : imported) {
        if (entry.length > uint64_t(INT64_MAX))
            guest_error(22);
        // access_ok checks the address range, not the presence of every page.
        // Unmapped/protected pages fault only when the operation reaches them.
        if (entry.base > address_limit || entry.length > address_limit - entry.base)
            guest_error(14);
        const size_t length = size_t(std::min<uint64_t>(entry.length, max_io - size_));
        if (length)
            entries_.push_back({entry.base, length});
        size_ += length;
    }
}
VectorBuffer IoVectors::prepare(Memory &memory, bool writing, size_t capacity) const {
    VectorBuffer result;
    result.bytes.resize(std::min(size_, capacity));
    size_t used = 0;
    const unsigned permissions = writing ? read_perm : write_perm;
    for (const auto &entry : entries_) {
        uint64_t address = entry.base;
        size_t remaining = std::min<size_t>(entry.length, result.bytes.size() - used);
        while (remaining) {
            const size_t count = std::min<size_t>(remaining, page_size - address % page_size);
            if (!memory.accessible(address, count, permissions)) {
                result.fault = true;
                result.bytes.resize(used);
                return result;
            }
            try {
                if (writing)
                    memory.copy_out(address, result.bytes.data() + used, count);
                else
                    memory.translate(address,
                                     write_perm); // Resolve COW/file-page faults before consuming input.
            } catch (const MemoryFault &) {
                result.fault = true;
                result.bytes.resize(used);
                return result;
            }
            used += count;
            address += count;
            remaining -= count;
        }
        if (used == result.bytes.size())
            break;
    }
    return result;
}
void IoVectors::scatter(Memory &memory, const void *data, size_t size) const {
    const auto *bytes = static_cast<const uint8_t *>(data);
    for (const auto &entry : entries_) {
        const size_t count = std::min<size_t>(size, entry.length);
        if (count)
            memory.copy_in(entry.base, bytes, count);
        size -= count;
        bytes += count;
        if (!size)
            return;
    }
    if (size)
        throw Failure("vector I/O result exceeds imported buffers");
}
} // namespace operit::arm64::files

namespace operit::arm64 {
namespace {
constexpr uint32_t rwf_dsync = 2, rwf_sync = 4, rwf_append = 16, rwf_noappend = 32, rwf_nosignal = 256;
struct TerminalReadWait {
    size_t previous_available = 0;
    TaskDeadline deadline;
    bool initialized = false;
};
int64_t positioned_write(int fd, const void *data, size_t count, uint64_t offset, bool append) {
    if (!append) {
        const ssize_t result = ::pwrite(fd, data, count, off_t(offset));
        return result < 0 ? -linux_errno(errno) : result;
    }
    // Darwin pwrite ignores O_APPEND, while Linux pwritev appends. The caller
    // holds the memfd guard through append and restoration of the shared offset.
    const off_t saved = ::lseek(fd, 0, SEEK_CUR);
    if (saved < 0)
        return -linux_errno(errno);
    const ssize_t result = host_write(fd, data, count);
    const int error = errno;
    if (::lseek(fd, saved, SEEK_SET) < 0)
        return -linux_errno(errno);
    return result < 0 ? -linux_errno(error) : result;
}
} // namespace
int64_t Syscalls::vector_io(CpuState &cpu) {
    const auto &a = cpu.x;
    const uint64_t number = a[8];
    const bool writing = number == 66 || number == 70 || number == 287;
    const bool version2 = number == 286 || number == 287;
    // Native AArch64 pos_l is a complete unsigned long. pos_h (x4) contributes
    // no bits on this 64-bit ABI; v2 flags still occupy x5.
    const uint64_t offset = a[3];
    const bool positioned = number != 65 && number != 66 && !(version2 && offset == UINT64_MAX);
    const uint32_t flags = version2 ? uint32_t(a[5]) : 0;
    auto &entry = descriptors_.get(uint32_t(a[0]));
    files::require_descriptor_io(entry, writing);
    const auto description = entry.description;
    if (description->inotify) {
        if (writing)
            guest_error(9);
        if (positioned)
            guest_error(29);
    }
    if (process::pidfd_file(description))
        guest_error(22); // Never read/write the pidfd's private readiness pipe.
    auto signal_file = signals::SignalFile::find(description);
    if (signal_file && positioned)
        guest_error(29);
    if (signal_file && writing)
        guest_error(22);
    if (positioned && int64_t(offset) < 0)
        guest_error(22);
    if (positioned && (description->terminal || description->pipe || description->socket ||
                       description->counter || description->timer || description->poll))
        guest_error(29);
    if (description->poll)
        guest_error(22);
    auto vectors = std::make_shared<files::IoVectors>(memory_, a[1], a[2]);
    if (signal_file && vectors->size() < sizeof(signals::SignalFileInfo))
        guest_error(22);
    if (!vectors->size() && !description->inotify) {
#ifndef __APPLE__
        // Linux readv/preadv notify ACCESS even at EOF / with empty vectors.
        // Preserve that native event instead of silently skipping the host
        // syscall or manufacturing a duplicate in the inotify queue.
        if (!writing && entry.host.get() >= 0 && !description->virtual_file &&
            !description->is_anon_inode() && !description->socket && !description->terminal &&
            !description->pipe && description->standard_stream < 0) {
            const auto result = positioned ? ::preadv(entry.host.get(), nullptr, 0, off_t(offset))
                                           : ::readv(entry.host.get(), nullptr, 0);
            if (result < 0)
                return -linux_errno(errno);
        }
#endif
        if (!writing && description->inotify_open)
            description->inotify_open->accessed();
        return 0;
    }
    if ((flags & rwf_append) && (flags & rwf_noappend))
        guest_error(22);
    // Only claim flags whose guarantees are implemented on every backend.
    // In particular, buffered Darwin I/O cannot implement RWF_NOWAIT or atomic
    // storage writes, and changing shared native O_APPEND is not a per-call flag.
    if (flags & ~(rwf_dsync | rwf_sync | rwf_nosignal))
        guest_error(95);
    if (positioned && vectors->size() > uint64_t(INT64_MAX) - offset)
        guest_error(22);
    if ((description->timer && writing) ||
        ((description->timer || description->counter) && vectors->size() < 8))
        guest_error(22);
    struct stat info{};
    if (entry.host.get() >= 0 && !description->virtual_file)
        check_host(::fstat(entry.host.get(), &info));
    if (positioned && (S_ISFIFO(info.st_mode) || S_ISSOCK(info.st_mode)))
        guest_error(29);
    const bool regular = S_ISREG(info.st_mode), fifo = S_ISFIFO(info.st_mode);
    const bool datagram = description->socket && description->socket->type != SOCK_STREAM;
    const bool nonblocking = description->flags & 2048;
    const uint32_t tid = scheduler_.current().tid, pid = scheduler_.current().pid;
    auto terminal_wait = std::make_shared<TerminalReadWait>();
    ort_arm64_io callback =
        !positioned && description->standard_stream >= 0 ? (writing ? config_.write : config_.read) : nullptr;
    // A terminal endpoint must retain its line discipline and guest job control.
    if (description->terminal)
        callback = nullptr;
    if (writing && callback && !description->virtual_file && (flags & (rwf_dsync | rwf_sync)))
        guest_error(95); // The embedding callback exposes no durability operation.
    std::function<std::optional<int64_t>()> signal_read;
    if (signal_file) {
        auto *controller = scheduler_.processes();
        if (!controller)
            guest_error(38);
        auto hooks = controller->signal_hooks();
        const auto reader = hooks.current();
        auto process = hooks.process(reader.pid);
        if (!reader.signals || !reader.mask || !process.signals || process.zombie)
            guest_error(3);
        signal_read = [this, signal_file, vectors, hooks = std::move(hooks), reader,
                       process = std::move(process)]() mutable -> std::optional<int64_t> {
            auto &pending = *process.signals;
            auto &thread = *reader.signals;
            if (signal_file->readable(pending, thread, *reader.mask)) {
                constexpr size_t record = sizeof(signals::SignalFileInfo);
                auto prepared = vectors->prepare(memory_, false, vectors->size() / record * record);
                const size_t capacity = prepared.bytes.size() / record * record;
                if (!capacity)
                    return -14; // Never dequeue a record whose output cannot be written.
                const size_t used = signal_file->read(pending, thread, *reader.mask,
                                                     prepared.bytes.data(), capacity);
                if (used) {
                    vectors->scatter(memory_, prepared.bytes.data(), used);
                    signals::signal_files_safe_point(descriptors_, hooks);
                    return used;
                }
            }
            signal_file->refresh(pending, thread, *reader.mask);
            signals::signal_files_safe_point(descriptors_, hooks);
            if (signals::has_deliverable(pending, thread, *reader.mask)) {
                signals::interrupted(thread, *reader.mask);
                return -4;
            }
            return {};
        };
    }
    auto attempt = [this, description, vectors, terminal_wait, flags, writing, offset, positioned, regular,
                    fifo, datagram, nonblocking, callback, tid, pid,
                    signal_read = std::move(signal_read),
                    limits = resource_limits()](int host_fd) -> std::optional<int64_t> {
        // The native signalfd pipe carries readiness only. Consume signal
        // records through its queue API, preserving the original reader on retry.
        if (signal_read)
            return signal_read();
        if (description->inotify) {
            // Records may span iovecs. Validate the entire aggregate output
            // before consuming anything, then scatter only complete records.
            auto prepared = vectors->prepare(memory_, false, vectors->size());
            if (prepared.fault)
                return -14;
            auto used = description->inotify->try_read(prepared.bytes.data(), prepared.bytes.size());
            if (!used)
                return {};
            if (*used)
                vectors->scatter(memory_, prepared.bytes.data(), *used);
            return int64_t(*used);
        }
        if (writing && !description->virtual_file)
            description->require_writable();
        auto terminal = description->terminal;
        if (terminal)
            terminals_->before_io(*terminal, pid, writing);
        pollfd event{host_fd, short(writing ? POLLOUT : POLLIN), 0};
        if (!description->virtual_file && !description->counter && !description->timer && !callback &&
            !positioned) {
            int ready = ::poll(&event, 1, 0);
            if (ready < 0)
                return -linux_errno(errno);
            if (event.revents & POLLNVAL)
                return -9;
            const bool hungup = event.revents & (POLLHUP | POLLERR);
            if (terminal && !writing && !terminal->master && !nonblocking && !hungup) {
                const auto attributes = terminals_->attributes(*terminal);
                if (!(attributes.local & 2)) {
                    const size_t minimum = std::min<size_t>(vectors->size(), attributes.characters[6]);
                    const unsigned deciseconds = attributes.characters[5];
                    const size_t available = size_t(terminals_->input_queued(*terminal));
                    const auto now = TaskClock::now();
                    if (!terminal_wait->initialized) {
                        terminal_wait->initialized = true;
                        if (!minimum && deciseconds)
                            terminal_wait->deadline = now + std::chrono::milliseconds(deciseconds * 100);
                    }
                    if (minimum && deciseconds) {
                        if (!available)
                            terminal_wait->deadline.reset();
                        else if (available > terminal_wait->previous_available)
                            terminal_wait->deadline = now + std::chrono::milliseconds(deciseconds * 100);
                    }
                    terminal_wait->previous_available = available;
                    const bool expired = terminal_wait->deadline && now >= *terminal_wait->deadline;
                    if (!available) {
                        if ((!minimum && !deciseconds) || expired)
                            return 0;
                        return {};
                    }
                    if (minimum && available < minimum && !expired)
                        return {};
                    ready = 1;
                }
            }
            if (!ready && !hungup)
                return {};
        }
        auto virtual_file = description->virtual_file;
        if (virtual_file) {
            const auto kind = virtual_file->node->kind;
            if (!writing && kind == VirtualNode::Kind::Null)
                return 0;
            if (writing && (kind == VirtualNode::Kind::Null || kind == VirtualNode::Kind::Zero ||
                            kind == VirtualNode::Kind::Full))
                return virtual_file->write(nullptr, vectors->size(), offset, positioned);
        }
        auto write_guard = [&](size_t requested) {
            return process::guard_file_write(
                *limits, host_fd, requested,
                positioned ? std::optional<uint64_t>(offset) : std::nullopt, std::nullopt,
                [this, tid] { process::send_limit_signal(scheduler_, tid, 25); });
        };
        size_t capacity = description->counter || description->timer ? 8 : vectors->size();
        if (writing && regular && !virtual_file && !callback) {
            auto guard = write_guard(capacity);
            capacity = size_t(guard.length());
        }
        // Preparing/scattering guest vectors always occurs outside file guards.
        auto prepared = vectors->prepare(memory_, writing, capacity);
        auto &data = prepared.bytes;
        const size_t accessible = data.size();
        if (prepared.fault && (description->counter || description->timer || (writing && datagram)))
            return -14;
        if (prepared.fault && writing && fifo) {
            // A fault cannot turn an atomic PIPE_BUF write into a shorter
            // successful write. Larger writes may commit complete guest pages.
            data.resize(data.size() / page_size * page_size);
            if (data.empty())
                return -14;
        }
        if (prepared.fault && data.empty() && !(datagram && !writing)) {
            if (writing || callback)
                return -14;
            uint8_t byte;
            if (virtual_file) {
                const uint64_t at = positioned ? offset : virtual_file->offset;
                return virtual_file->read(&byte, 1, at, true) ? -14 : 0;
            }
            if (regular) {
                auto guard = files::memory_file_lock(host_fd);
                const off_t at = positioned ? off_t(offset) : ::lseek(host_fd, 0, SEEK_CUR);
                if (at < 0)
                    return -linux_errno(errno);
                const ssize_t result = ::pread(host_fd, &byte, 1, at);
                return result < 0 ? -linux_errno(errno) : result ? -14 : 0;
            }
            if (description->socket) {
                const ssize_t result = ::recv(host_fd, &byte, 1, MSG_PEEK | MSG_DONTWAIT);
                if (result < 0) {
                    const int error = linux_errno(errno);
                    if (error == 11)
                        return {};
                    return -error;
                }
                return result ? -14 : 0;
            }
            if (fifo || terminal) {
                int available = 0;
                if (::ioctl(host_fd, FIONREAD, &available) < 0)
                    return -linux_errno(errno);
                if (available)
                    return -14;
                if (event.revents & (POLLHUP | POLLERR))
                    return 0;
                return terminal ? std::optional<int64_t>(0) : std::nullopt;
            }
            return -14;
        }
        // A datagram is consumed as one message even if a later guest vector
        // faults. Copy the accessible prefix, report EFAULT, and never send a
        // truncated datagram merely because a later source vector is invalid.
        if (datagram && !writing && prepared.fault)
            data.resize(capacity);
        int64_t result;
        if (virtual_file) {
            result = writing ? virtual_file->write(data.data(), data.size(), offset, positioned)
                             : virtual_file->read(data.data(), data.size(), offset, positioned);
        } else if (description->counter || description->timer) {
            try {
                uint64_t value;
                if (writing) {
                    std::memcpy(&value, data.data(), 8);
                    description->counter->write(value);
                } else {
                    value =
                        description->timer ? description->timer->read() : description->counter->read(host_fd);
                    std::memcpy(data.data(), &value, 8);
                }
                result = 8;
            } catch (const GuestError &error) {
                if (error.number == 11)
                    return {};
                throw;
            }
        } else if (callback) {
            result = callback(config_.io_context, description->standard_stream, data.data(), data.size());
        } else if (terminal) {
            result = writing ? terminals_->write(*terminal, host_fd, data.data(), data.size())
                             : terminals_->read(*terminal, host_fd, data.data(), data.size());
        } else if (description->pipe) {
            result = writing ? description->pipe->write(host_fd, data.data(), data.size())
                             : description->pipe->read(host_fd, data.data(), data.size());
        } else if (writing) {
            auto guard = write_guard(data.size());
            const size_t length = size_t(guard.length());
            result = positioned ? positioned_write(host_fd, data.data(), length, offset,
                                                    regular && (description->flags & 1024))
                                : host_write(host_fd, data.data(), length);
            if (!positioned && result < 0)
                result = -linux_errno(errno);
        } else {
            auto guard = files::memory_file_lock(host_fd);
#ifdef __APPLE__
            result = positioned ? ::pread(host_fd, data.data(), data.size(), off_t(offset))
                                : ::read(host_fd, data.data(), data.size());
#else
            // The guest vectors have already been gathered into this buffer.
            // Keep a native vector syscall so Linux's EOF ACCESS semantics are
            // retained; a scalar pread would suppress that event.
            struct iovec vector{data.data(), data.size()};
            result = positioned ? ::preadv(host_fd, &vector, 1, off_t(offset))
                                : ::readv(host_fd, &vector, 1);
#endif
            if (result < 0)
                result = -linux_errno(errno);
        }
        if (result > int64_t(data.size()) || result < -4095)
            throw Failure("invalid byte count or errno from vector I/O backend");
        if (result == -11)
            return {};
        if (!writing && result >= 0 && description->inotify_open)
            description->inotify_open->accessed();
        if (writing && result == -32 && !(flags & rwf_nosignal))
            broken_pipe(tid);
        if (terminal) {
            if (writing && result > 0)
                terminals_->written_input(*terminal, data.data(), size_t(result));
            if (!writing)
                result = terminals_->read_result(*terminal, data.data(), result);
        }
        if (description->socket && result >= 0) {
            if (writing)
                description->socket->sent(size_t(result));
            else
                description->socket->received(size_t(result), false);
        }
        if (!writing && result > 0) {
            vectors->scatter(memory_, data.data(), std::min<size_t>(size_t(result), accessible));
            if (size_t(result) > accessible)
                return -14;
        }
        if (writing && result > 0 && regular && (flags & (rwf_dsync | rwf_sync))) {
            if (::fsync(host_fd) < 0)
                return -linux_errno(errno);
        }
        return result;
    };
    if (auto result = attempt(entry.host.get()))
        return *result;
    if (nonblocking)
        return -11;
    auto host =
        std::make_shared<FD>(entry.host.get() < 0 ? -1 : ::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    if (entry.host.get() >= 0)
        check_host(host->get());
    TaskDeadline deadline;
    if (description->socket) {
        const uint64_t timeout =
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
