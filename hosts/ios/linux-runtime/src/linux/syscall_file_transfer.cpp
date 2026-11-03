#include "base/host_io.h"
#include "linux/error_mapping.h"
#include "linux/events/inotify.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/memory_file.h"
#include "linux/files/pipe_buffer.h"
#include "linux/process/limit_enforcement.h"
#include "linux/process/pidfd.h"
#include "linux/signals/signal_file.h"
#include "linux/syscalls.h"
#include <algorithm>
#include <cerrno>
#include <climits>
#include <cstring>
#include <fcntl.h>
#include <poll.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
struct TransferEndpoint {
    std::shared_ptr<FD> fd;
    std::shared_ptr<OpenDescription> description;
    struct stat info{};
    uint64_t offset_pointer = 0;
    int64_t offset = 0;
    bool pipe() const {
        return S_ISFIFO(info.st_mode);
    }
    bool socket() const {
        return S_ISSOCK(info.st_mode);
    }
    bool regular() const {
        return S_ISREG(info.st_mode);
    }
};
struct FileTransfer {
    Memory &memory;
    const ort_arm64_config config;
    TransferEndpoint input, output;
    uint64_t kind, count, flags;
    TaskDeadline deadline;
    bool deadline_set = false;
    std::function<void()> broken_pipe;
    std::shared_ptr<process::ResourceLimits> limits;
    std::function<void()> file_size_signal;

    void buffer(uint64_t address, size_t length, unsigned permissions) {
        if (!memory.accessible(address, length, permissions))
            guest_error(14);
    }
    void offsets() {
        for (auto *endpoint : {&input, &output}) {
            if (endpoint->offset_pointer) {
                buffer(endpoint->offset_pointer, 8, read_perm | write_perm);
                endpoint->offset = int64_t(memory.load(endpoint->offset_pointer, 8));
            } else if (endpoint->regular()) {
                if (endpoint->description->virtual_file)
                    endpoint->offset = endpoint->description->virtual_file->offset;
                else {
                    auto guard = files::memory_file_lock(endpoint->fd->get());
                    endpoint->offset = ::lseek(endpoint->fd->get(), 0, SEEK_CUR);
                    if (endpoint->offset < 0)
                        guest_error(linux_errno(errno));
                }
            }
            if ((endpoint->offset_pointer || endpoint->regular()) && endpoint->offset < 0)
                guest_error(22);
        }
        const uint64_t length = std::min<uint64_t>(count, 0x7ffff000);
        if (uint64_t(input.offset) > uint64_t(INT64_MAX) - length ||
            uint64_t(output.offset) > uint64_t(INT64_MAX) - length)
            guest_error(75);
        if (kind == 285 && input.info.st_dev == output.info.st_dev &&
            input.info.st_ino == output.info.st_ino && length &&
            uint64_t(input.offset) < uint64_t(output.offset) + length &&
            uint64_t(output.offset) < uint64_t(input.offset) + length)
            guest_error(22);
    }
    std::optional<int64_t> waiting(const TransferEndpoint &endpoint, bool writing) {
        if ((endpoint.description->flags & 2048) || (endpoint.pipe() && (flags & 2)))
            return -11;
        if (!deadline_set) {
            deadline_set = true;
            if (endpoint.description->socket) {
                const uint64_t timeout = writing ? endpoint.description->socket->send_timeout_us
                                                 : endpoint.description->socket->receive_timeout_us;
                if (timeout)
                    deadline = TaskClock::now() + std::chrono::microseconds(timeout);
            }
        }
        return {};
    }
    int64_t peek(std::vector<uint8_t> &data) {
        if (input.pipe())
            return input.description->pipe->peek(input.fd->get(), data.data(), data.size());
        if (input.socket()) {
            const ssize_t result = ::recv(input.fd->get(), data.data(), data.size(), MSG_PEEK | MSG_DONTWAIT);
            return result < 0 ? -linux_errno(errno) : result;
        }
        if (input.description->virtual_file)
            return input.description->virtual_file->read(data.data(), data.size(), input.offset, true);
        auto guard = files::memory_file_lock(input.fd->get());
        const ssize_t result = ::pread(input.fd->get(), data.data(), data.size(), input.offset);
        return result < 0 ? -linux_errno(errno) : result;
    }
    int64_t write(const std::vector<uint8_t> &data, size_t count) {
        output.description->require_writable();
        if (output.description->virtual_file)
            return output.description->virtual_file->write(data.data(), count, output.offset,
                                                           bool(output.offset_pointer));
        if (output.description->pipe)
            return output.description->pipe->write(output.fd->get(), data.data(), count);
        if (output.description->standard_stream >= 0 && config.write)
            return config.write(config.io_context, output.description->standard_stream,
                                const_cast<uint8_t *>(data.data()), count);
        // The input is still only peeked. A seal/FSIZE failure cannot consume
        // pipe/socket input, and a legal short write commits only that prefix.
        auto guard = process::guard_file_write(
            *limits, output.fd->get(), count,
            output.offset_pointer ? std::optional<uint64_t>(output.offset) : std::nullopt,
            false, file_size_signal);
        count = size_t(guard.length());
        ssize_t result;
        if (output.socket()) {
            int native_flags = MSG_DONTWAIT;
#ifdef MSG_NOSIGNAL
            native_flags |= MSG_NOSIGNAL;
#endif
            result = ::send(output.fd->get(), data.data(), count, native_flags);
        } else if (output.offset_pointer)
            result = ::pwrite(output.fd->get(), data.data(), count, output.offset);
        else {
            pollfd ready{output.fd->get(), POLLOUT, 0};
            if (::poll(&ready, 1, 0) < 0)
                return -linux_errno(errno);
            if (!ready.revents)
                return -11;
            if (output.pipe()) {
                // Inherited streams may be blocking at the host level. Limit
                // each write to the host's atomic write guarantee.
                const int status = ::fcntl(output.fd->get(), F_GETFL);
                if (status < 0)
                    return -linux_errno(errno);
                if (!(status & O_NONBLOCK)) {
                    const long atomic = ::fpathconf(output.fd->get(), _PC_PIPE_BUF);
                    if (atomic <= 0)
                        return -95;
                    count = std::min<uint64_t>(count, atomic);
                }
            }
            result = host_write(output.fd->get(), data.data(), count);
        }
        return result < 0 ? -linux_errno(errno) : result;
    }
    void commit_input(const std::vector<uint8_t> &data, size_t count) {
        if (kind == 77)
            return;
        if (input.pipe()) {
            const int64_t consumed = input.description->pipe->consume(input.fd->get(), data.data(), count);
            if (consumed != int64_t(count))
                throw Failure("pipe changed outside its guest open description during splice");
        } else if (input.socket()) {
            std::vector<uint8_t> consumed(count);
            const ssize_t result = ::recv(input.fd->get(), consumed.data(), count, MSG_DONTWAIT);
            if (result != int64_t(count) || std::memcmp(data.data(), consumed.data(), count))
                throw Failure("socket changed outside its guest open description during splice");
            if (input.description->socket)
                input.description->socket->received(count, false);
        } else if (!input.offset_pointer) {
            if (input.description->virtual_file) {
                if (!input.description->virtual_file->node->device())
                    input.description->virtual_file->offset = input.offset + count;
            } else {
                auto guard = files::memory_file_lock(input.fd->get());
                if (::lseek(input.fd->get(), input.offset + count, SEEK_SET) < 0)
                    guest_error(linux_errno(errno));
            }
        }
        // Native offset guards have been released before touching guest memory.
        if (input.offset_pointer)
            memory.store(input.offset_pointer, input.offset + count, 8);
    }
    std::optional<int64_t> attempt() {
        offsets();
        size_t capacity = size_t(std::min<uint64_t>(count, max_io));
        if (output.regular() && !output.description->virtual_file) {
            auto guard = process::guard_file_write(
                *limits, output.fd->get(), capacity,
                output.offset_pointer ? std::optional<uint64_t>(output.offset) : std::nullopt,
                false, file_size_signal);
            capacity = size_t(guard.length());
        }
        std::vector<uint8_t> data(capacity);
        const int64_t available = peek(data);
        if (available == -11)
            return waiting(input, false);
        if (available <= 0)
            return available;
        int64_t result;
        try {
            result = write(data, available);
        } catch (const GuestError &error) {
            if (error.number == 32)
                broken_pipe();
            throw;
        }
        if (result == -32)
            broken_pipe();
        if (result > available || result < -4095)
            throw Failure("invalid byte count or errno during guest file transfer");
        if (result == -11)
            return waiting(output, true);
        if (result <= 0)
            return result;
        if (input.description->inotify_open)
            input.description->inotify_open->accessed();
        commit_input(data, result);
        if (output.offset_pointer)
            memory.store(output.offset_pointer, output.offset + result, 8);
        if (output.description->socket)
            output.description->socket->sent(result);
        return result;
    }
};
} // namespace
int64_t Syscalls::dispatch_file_transfer(CpuState &cpu) {
    const auto &a = cpu.x;
    const uint64_t kind = a[8];
    const bool sendfile = kind == 71, tee = kind == 77, copy = kind == 285;
    if (!sendfile && !tee && !copy && kind != 76)
        guest_error(22);
    const uint64_t source_fd = sendfile ? a[1] : a[0];
    const uint64_t target_fd = sendfile ? a[0] : (tee ? a[1] : a[2]);
    auto &source = descriptors_.get(uint32_t(source_fd));
    auto &target = descriptors_.get(uint32_t(target_fd));
    files::require_descriptor_io(source, false);
    files::require_descriptor_io(target, true);
    if (process::pidfd_file(source.description) || process::pidfd_file(target.description) ||
        source.description->inotify || target.description->inotify ||
        signals::SignalFile::find(source.description) || signals::SignalFile::find(target.description))
        guest_error(22); // Neither endpoint may expose an internal readiness pipe.
    target.description->require_writable();
    if (target.description->flags & 1024)
        guest_error(copy ? 9 : 22);
    if (source.description->counter || source.description->poll || source.description->timer ||
        target.description->counter || target.description->poll || target.description->timer)
        guest_error(22);
    const uint64_t flags = sendfile ? 0 : a[tee ? 3 : 5];
    if ((copy && flags) || (!copy && (flags & ~uint64_t{15})))
        guest_error(22);
    auto state = std::make_shared<FileTransfer>(
        FileTransfer{memory_, config_, {}, {}, kind, a[sendfile ? 3 : (tee ? 2 : 4)], flags, {}, false,
                     {}, resource_limits(), {}});
    const uint32_t calling_tid = scheduler_.current().tid;
    state->broken_pipe = [this, calling_tid]() { broken_pipe(calling_tid); };
    state->file_size_signal = [this, calling_tid]() {
        process::send_limit_signal(scheduler_, calling_tid, 25);
    };
    auto initialize = [&](TransferEndpoint &endpoint, const Descriptor &entry, uint64_t pointer) {
        endpoint.description = entry.description;
        endpoint.info = descriptor_stat(entry);
        endpoint.offset_pointer = pointer;
        if (pointer) {
            check_buffer(pointer, 8, read_perm | write_perm);
            endpoint.offset = int64_t(memory_.load(pointer, 8));
            if (endpoint.offset < 0)
                guest_error(22);
            if (endpoint.pipe() || endpoint.socket())
                guest_error(29);
        }
        if (entry.host.get() >= 0) {
            endpoint.fd = std::make_shared<FD>(::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
            check_host(endpoint.fd->get());
        } else
            endpoint.fd = std::make_shared<FD>();
    };
    initialize(state->input, source, sendfile ? a[2] : (tee ? 0 : a[1]));
    initialize(state->output, target, sendfile || tee ? 0 : a[3]);
    if (S_ISDIR(state->input.info.st_mode) || S_ISDIR(state->output.info.st_mode))
        guest_error(copy ? 21 : 22);
    if (sendfile && (!state->input.regular() || source.description->virtual_file))
        guest_error(22);
    if (copy && (!state->input.regular() || !state->output.regular() || source.description->virtual_file ||
                 target.description->virtual_file))
        guest_error(22);
    if ((tee && (!state->input.pipe() || !state->output.pipe())) ||
        (kind == 76 && !state->input.pipe() && !state->output.pipe()))
        guest_error(22);
    if (state->input.pipe() && state->output.pipe() &&
        ((source.description->pipe && source.description->pipe == target.description->pipe) ||
         (state->input.info.st_ino && state->input.info.st_ino == state->output.info.st_ino &&
          state->input.info.st_dev == state->output.info.st_dev)))
        guest_error(22);
    if (state->input.pipe() && !source.description->pipe)
        guest_error(95);
    if (!state->input.pipe() && !state->input.regular() && !state->input.socket() &&
        !source.description->virtual_file)
        guest_error(22);
    if (state->input.socket()) {
        int socket_type = 0;
        socklen_t size = sizeof(socket_type);
        check_host(::getsockopt(state->input.fd->get(), SOL_SOCKET, SO_TYPE, &socket_type, &size));
        if (socket_type != SOCK_STREAM)
            guest_error(22);
    }
    if (!state->count)
        return 0;
    if (auto result = state->attempt())
        return *result;
    scheduler_.block(state->deadline, -11, [state]() { return state->attempt(); });
    return 0;
}
} // namespace operit::arm64
