#include "linux/abi.h"
#include "linux/error_mapping.h"
#include "linux/network/message_batch.h"
#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include <algorithm>
#include <chrono>
#include <cstring>
#include <fcntl.h>
#include <poll.h>
#include <sys/uio.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
using namespace socket_messages;
struct MessageData {
    Header header{};
    uint64_t address = 0;
    std::vector<linux_abi::Iovec> vectors;
    std::vector<FD> rights;
    std::vector<std::shared_ptr<Descriptor>> metadata;
    std::vector<uint8_t> control;
    NativeAddress destination;
    Credentials credentials{};
    size_t bytes = 0, progress = 0;
    uint32_t output_flags = 0;
};
struct BatchData {
    uint64_t address = 0, timeout_address = 0;
    uint32_t limit = 1, completed = 0;
    TaskDeadline deadline, element_deadline;
    std::shared_ptr<MessageData> message;
};
bool metadata_cycle(const std::shared_ptr<SocketState> &source,
                    const std::shared_ptr<SocketState> &receiver) {
    std::vector<std::shared_ptr<SocketState>> pending{source};
    std::vector<const SocketState *> visited;
    while (!pending.empty()) {
        auto socket = std::move(pending.back());
        pending.pop_back();
        if (socket == receiver)
            return true;
        if (std::find(visited.begin(), visited.end(), socket.get()) != visited.end())
            continue;
        visited.push_back(socket.get());
        for (const auto &connection : socket->pending_connections)
            pending.push_back(connection.socket);
        for (const auto &batch : socket->pending_rights)
            for (const auto &descriptor : batch.descriptors)
                if (descriptor->description && descriptor->description->socket)
                    pending.push_back(descriptor->description->socket);
    }
    return false;
}
TaskClock::time_point message_deadline(uint64_t seconds, uint64_t nanoseconds) {
    const auto now = TaskClock::now();
    const auto room = std::chrono::duration_cast<std::chrono::nanoseconds>(
                          TaskClock::time_point::max() - now).count();
    if (seconds > uint64_t(room / 1000000000) ||
        (seconds == uint64_t(room / 1000000000) && nanoseconds > uint64_t(room % 1000000000)))
        return TaskClock::time_point::max();
    return now + std::chrono::duration_cast<TaskClock::duration>(
                     std::chrono::nanoseconds(seconds * 1000000000 + nanoseconds));
}
TaskDeadline first_deadline(const BatchData &batch) {
    if (!batch.deadline)
        return batch.element_deadline;
    if (!batch.element_deadline)
        return batch.deadline;
    return std::min(*batch.deadline, *batch.element_deadline);
}
} // namespace
int64_t Syscalls::socket_message(CpuState &cpu) {
    const auto &a = cpu.x;
    const bool sending = a[8] == 211 || a[8] == 269;
    const bool batch = a[8] == 269 || a[8] == 243;
    const uint64_t guest_flags = uint32_t(a[batch ? 3 : 2]);
    auto request = std::make_shared<BatchData>();
    request->address = a[1];
    if (batch)
        request->limit = sending ? std::min<uint32_t>(uint32_t(a[2]), 1024) : uint32_t(a[2]);
    if (batch && !sending && a[4]) {
        check_buffer(a[4], 16, read_perm);
        const int64_t seconds = memory_.load(a[4], 8), nanoseconds = memory_.load(a[4] + 8, 8);
        if (seconds < 0 || nanoseconds < 0 || nanoseconds >= 1000000000)
            guest_error(22);
        request->timeout_address = a[4];
        request->deadline = message_deadline(seconds, nanoseconds);
    }
    auto &entry = descriptors_.get(uint32_t(a[0]));
    auto state = entry.description->socket;
    if (!state)
        guest_error(88);
    if (!request->limit)
        return 0;
    if (sending && (guest_flags & wait_for_one))
        guest_error(95);
    int flags = socket_message_flags(guest_flags & ~(batch ? wait_for_one : 0), sending);
    const bool waitall = !sending && (guest_flags & 256) && state->type == 1;
    if (waitall)
        flags &= ~MSG_WAITALL;
#ifndef __APPLE__
    if (!sending && (guest_flags & 32) && (state->type == 2 || state->type == 5))
        flags |= MSG_TRUNC;
#endif
    const bool nonblocking = (guest_flags & 64) || (entry.description->flags & 2048);
    const uint32_t tid = scheduler_.current().tid;
    const Credentials identity{int32_t(scheduler_.current().pid), uint32_t(::getuid()), uint32_t(::getgid())};
    if (!sending)
        if (int error = take_error(state))
            return -error;
    auto load_message = [this, state, sending, identity](uint64_t address) {
        auto message = std::make_shared<MessageData>();
        message->address = address;
        message->credentials = identity;
        check_buffer(address, sizeof(Header), read_perm);
        memory_.copy_out(address, &message->header, sizeof(Header));
        const auto &header = message->header;
        if (header.vector_count > 1024 || (sending && header.control_size > maximum_control))
            guest_error(90);
        if (header.name && int32_t(header.name_size) < 0)
            guest_error(22);
        check_buffer(header.vectors, header.vector_count * sizeof(linux_abi::Iovec), read_perm);
        message->vectors.reserve(header.vector_count);
        for (size_t index = 0; index < header.vector_count; ++index) {
            linux_abi::Iovec vector{};
            memory_.copy_out(header.vectors + index * sizeof(vector), &vector, sizeof(vector));
            if (vector.length > max_io - message->bytes)
                guest_error(90);
            message->bytes += vector.length;
            check_buffer(vector.base, vector.length, sending ? read_perm : write_perm);
            message->vectors.push_back(vector);
        }
        if (sending && header.name)
            message->destination = read_socket_address(header.name, header.name_size, false);
        if (!sending) {
            check_buffer(address + 40, 12, write_perm);
            if (header.name) {
                check_buffer(address + 8, 4, write_perm);
                check_buffer(header.name, std::min<uint32_t>(header.name_size, 128), write_perm);
            }
            return message;
        }
        check_buffer(header.control, header.control_size, read_perm);
        size_t offset = 0;
        while (header.control_size - offset >= control_header_size) {
            const uint64_t size = memory_.load(header.control + offset, 8);
            const int32_t level = memory_.load(header.control + offset + 8, 4);
            const int32_t type = memory_.load(header.control + offset + 12, 4);
            if (size < control_header_size || size > header.control_size - offset)
                guest_error(22);
            if (level != 1 || (type != 1 && type != 2))
                guest_error(95);
            if (state->domain != 1)
                guest_error(95);
            if (type == 2) {
                if (size != control_header_size + sizeof(Credentials))
                    guest_error(22);
                Credentials credentials{};
                memory_.copy_out(header.control + offset + control_header_size, &credentials,
                                 sizeof(credentials));
                if (credentials.uid == UINT32_MAX || credentials.gid == UINT32_MAX)
                    guest_error(22);
                // Guest capabilities are empty even when the embedding host is root.
                if (credentials.pid != identity.pid ||
                    (credentials.uid != uint32_t(::getuid()) && credentials.uid != uint32_t(::geteuid())) ||
                    (credentials.gid != uint32_t(::getgid()) && credentials.gid != uint32_t(::getegid())))
                    guest_error(1);
                // Host SCM_CREDENTIALS would authenticate a host PID, not this
                // guest process. Only a known guest Unix peer can carry these.
                if (state->peer.expired())
                    guest_error(95);
                message->credentials = credentials;
            } else {
                // Linux scm_fp_copy consumes whole ints; a final fractional int
                // and an empty SCM_RIGHTS record do not invalidate the send.
                const size_t count = (size - control_header_size) / 4;
                if (count > maximum_rights - message->rights.size())
                    guest_error(22);
                for (size_t i = 0; i < count; ++i) {
                    auto &source = descriptors_.get(uint32_t(memory_.load(
                        header.control + offset + control_header_size + i * 4, 4)));
                    const auto &description = *source.description;
                    if (source.host.get() < 0 || description.virtual_file || description.poll ||
                        (state->peer.expired() && (description.counter || description.timer ||
                                                  description.pipe || description.socket || description.terminal ||
                                                  process::pidfd_file(source.description))))
                        guest_error(95);
                    if (description.socket)
                        if (auto peer = state->peer.lock(); peer && description.socket != peer &&
                            metadata_cycle(description.socket, peer))
                            // The host SCM garbage collector cannot collect our
                            // companion guest metadata. Refuse a cycle before
                            // sending, rather than retain host-backed leaves.
                            guest_error(95);
                    FD copy(::fcntl(source.host.get(), F_DUPFD_CLOEXEC, 0));
                    check_host(copy.get());
                    message->rights.push_back(std::move(copy));
                    // The host kernel owns the in-flight FD reference. The
                    // companion guest metadata must not retain a second host FD.
                    message->metadata.push_back(std::make_shared<Descriptor>(FD(), source, false));
                }
            }
            const size_t aligned = control_align(size);
            if (aligned > header.control_size - offset)
                break;
            offset += aligned;
        }
        if (!message->rights.empty()) {
            message->control.resize(CMSG_SPACE(message->rights.size() * sizeof(int)), 0);
            msghdr native{};
            native.msg_control = message->control.data();
            native.msg_controllen = message->control.size();
            auto *control = CMSG_FIRSTHDR(&native);
            control->cmsg_level = SOL_SOCKET;
            control->cmsg_type = SCM_RIGHTS;
            control->cmsg_len = CMSG_LEN(message->rights.size() * sizeof(int));
            for (size_t i = 0; i < message->rights.size(); ++i) {
                const int fd = message->rights[i].get();
                std::memcpy(CMSG_DATA(control) + i * sizeof(int), &fd, sizeof(int));
            }
        }
        return message;
    };
    auto transfer = [this, state, sending, batch, flags, waitall, nonblocking, guest_flags, tid]
                    (int host, const std::shared_ptr<MessageData> &message) -> std::optional<int64_t> {
        const auto &header = message->header;
        const bool peek = !sending && (guest_flags & 2);
        const size_t position = sending || peek ? 0 : message->progress;
        for (const auto &vector : message->vectors)
            check_buffer(vector.base, vector.length, sending ? read_perm : write_perm);
        if (!sending) {
            check_buffer(message->address + 40, 12, write_perm);
            if (header.name) {
                check_buffer(message->address + 8, 4, write_perm);
                check_buffer(header.name, std::min<uint32_t>(header.name_size, 128), write_perm);
            }
        }
        const bool credentials_enabled = !sending && state->domain == 1 && pass_credentials(state);
        auto credentials = credentials_enabled ? received_credentials(state) : std::nullopt;
        if (credentials_enabled && !credentials && (message->bytes || state->type != 1)) {
            pollfd ready{host, POLLIN, 0};
            if (::poll(&ready, 1, 0) < 0)
                return -socket_errno(errno);
            if (!ready.revents)
                return {};
            // A message from an untracked host or guest sender cannot be
            // authenticated as a guest PID. Leave it queued rather than forge it.
            if (state->type != 1)
                return -95;
            uint8_t byte;
            const ssize_t pending = ::recv(host, &byte, 1, MSG_PEEK | MSG_DONTWAIT);
            if (pending > 0)
                return -95;
            if (pending < 0) {
                if (errno == EAGAIN || errno == EWOULDBLOCK)
                    return {};
                return -socket_errno(errno);
            }
        }
        size_t capacity = message->bytes - position;
        if (credentials_enabled)
            capacity = credential_receive_limit(state, capacity);
        std::vector<uint8_t> data(capacity);
        if (sending) {
            size_t offset = 0;
            for (const auto &vector : message->vectors) {
                if (vector.length)
                    memory_.copy_out(vector.base, data.data() + offset, vector.length);
                offset += vector.length;
            }
        }
        iovec vector{data.empty() ? nullptr : data.data(), data.size()};
        sockaddr_storage peer{};
        msghdr native{};
        native.msg_iov = &vector;
        native.msg_iovlen = 1;
        std::optional<ReceivedControl> received_control;
        if (sending) {
            native.msg_name = message->destination.length ? &message->destination.storage : nullptr;
            native.msg_namelen = message->destination.length;
            // A Linux Unix stream needs a payload byte to carry ancillary
            // data. Some BSD hosts otherwise accept a control-only transfer.
            const bool carry_control = !message->control.empty() && (state->type != 1 || message->bytes);
            native.msg_control = carry_control ? message->control.data() : nullptr;
            native.msg_controllen = carry_control ? message->control.size() : 0;
        } else {
            native.msg_name = &peer;
            native.msg_namelen = sizeof(peer);
            received_control.emplace();
            native.msg_control = received_control->data();
            native.msg_controllen = received_control->capacity();
        }
        ssize_t datagram_size = 0;
#ifdef __APPLE__
        if (!sending && (guest_flags & 32) && (state->type == 2 || state->type == 5)) {
            // Darwin does not return the full packet length for MSG_TRUNC.
            // Probe without consuming it; a control-less peek closes its own
            // temporary FD copies in the host kernel.
            std::vector<uint8_t> packet(max_io + 1);
            datagram_size = ::recv(host, packet.data(), packet.size(), MSG_PEEK | MSG_DONTWAIT);
            if (datagram_size < 0) {
                if (errno == EAGAIN || errno == EWOULDBLOCK)
                    return {};
                return -socket_errno(errno);
            }
            if (size_t(datagram_size) == packet.size())
                return -90;
        }
#endif
        int native_flags = flags;
        if (sending && batch && (header.flags & 128))
            native_flags |= MSG_EOR;
        const ssize_t count = sending ? ::sendmsg(host, &native, native_flags)
                                      : ::recvmsg(host, &native, native_flags);
        if (!sending && count >= 0)
            received_control->received(native.msg_controllen);
        if (count < 0) {
            const int error = socket_errno(errno);
            if (error == 11) {
                if (!sending && nonblocking && message->progress)
                    return message->progress;
                return {};
            }
            if (sending && error == 32 && !(guest_flags & 0x4000))
                broken_pipe(tid);
            return message->progress ? int64_t(message->progress) : -error;
        }
        if (sending) {
            sent_credentials(state, count, message->credentials);
            state->sent(count, message->metadata);
            return count;
        }
        auto received = received_control->take_rights();
        // All native FDs are now guarded, including those after a failing fcntl.
        for (auto &descriptor : received)
            check_host(::fcntl(descriptor.get(), F_SETFD, FD_CLOEXEC));
        auto originals = state->received(count, peek);
        const size_t copied = std::min<size_t>(count, data.size());
        size_t skip = position, offset = 0;
        for (const auto &item : message->vectors) {
            const size_t prefix = std::min<size_t>(skip, item.length);
            const size_t size = std::min<size_t>(copied - offset, item.length - prefix);
            if (size && !((guest_flags & 32) && state->type == 1))
                memory_.copy_in(item.base + prefix, data.data() + offset, size);
            skip -= prefix;
            offset += size;
            if (offset == copied)
                break;
        }
        if (header.name) {
            const auto address = socket_address_bytes(reinterpret_cast<sockaddr *>(&peer), native.msg_namelen);
            const size_t size = std::min<size_t>(address.size(), header.name_size);
            check_buffer(header.name, size, write_perm);
            if (size)
                memory_.copy_in(header.name, address.data(), size);
            memory_.store(message->address + 8, address.size(), 4);
        }
        uint32_t output_flags =
            (native.msg_flags & MSG_OOB ? 1 : 0) | (native.msg_flags & MSG_EOR ? 128 : 0) |
            (native.msg_flags & MSG_TRUNC ? 32 : 0) | (native.msg_flags & MSG_CTRUNC ? 8 : 0) |
            (guest_flags & 0x40000000);
        GuestControl output(header.control ? std::min<uint64_t>(header.control_size, maximum_control) : 0);
        if (credentials_enabled && (count || state->type != 1)) {
            if (!credentials)
                guest_error(95);
            output.append(2, &*credentials, sizeof(Credentials));
        }
        if (received_control->has_other_control(credentials_enabled && credentials.has_value()))
            output.truncate(); // Never expose an untranslated host ancillary ABI or PID.
        const size_t wanted = std::min({output.rights_capacity(), received.size(), maximum_rights});
        std::vector<int32_t> installed;
        installed.reserve(wanted);
        try {
            for (size_t i = 0; i < wanted; ++i) {
                const auto original = i < originals.size() ? originals[i] : std::shared_ptr<Descriptor>{};
                const bool translated = original && original->description;
                int status = ::fcntl(received[i].get(), F_GETFL);
                check_host(status);
                int descriptor_flags = (status & O_ACCMODE) == O_RDWR ? 2 :
                                       (status & O_ACCMODE) == O_WRONLY ? 1 : 0;
                if (status & O_APPEND)
                    descriptor_flags |= 1024;
                if (status & O_NONBLOCK)
                    descriptor_flags |= 2048;
                if (guest_flags & 0x40000000)
                    descriptor_flags |= 524288;
                std::shared_ptr<SocketState> imported_socket;
                bool imported_fifo = false;
                if (!translated) {
                    struct stat info{};
                    check_host(::fstat(received[i].get(), &info));
                    if (S_ISSOCK(info.st_mode)) {
                        int type = 0;
                        socklen_t size = sizeof(type);
                        check_host(::getsockopt(received[i].get(), SOL_SOCKET, SO_TYPE, &type, &size));
                        sockaddr_storage address{};
                        size = sizeof(address);
                        check_host(::getsockname(received[i].get(), reinterpret_cast<sockaddr *>(&address), &size));
                        int kind = type == SOCK_STREAM ? 1 : type == SOCK_DGRAM ? 2 :
                                   type == SOCK_SEQPACKET ? 5 : 3;
                        imported_socket = new_socket_state(guest_socket_family(address.ss_family), kind, 0);
                        configure_socket(received[i].get());
                    } else if (S_ISFIFO(info.st_mode)) {
                        check_host(::fcntl(received[i].get(), F_SETFL, status | O_NONBLOCK));
                        imported_fifo = true;
                    }
                }
                int fd;
                try {
                    fd = descriptors_.insert(std::move(received[i]), translated ? original->path : "",
                                             descriptor_flags, 0);
                } catch (const GuestError &error) {
                    if (error.number == 24)
                        break; // Linux installs what fits, then reports MSG_CTRUNC.
                    throw;
                }
                installed.push_back(fd);
                auto &description = descriptors_.get(fd).description;
                if (translated)
                    description = original->description;
                else if (imported_socket) {
                    description->socket = std::move(imported_socket);
                    description->scheduler_io = true;
                } else if (imported_fifo)
                    description->scheduler_io = true;
            }
            if (installed.size() != received.size())
                output.truncate();
            if (!installed.empty())
                output.append(1, installed.data(), installed.size() * sizeof(int32_t));
            if (output.truncated())
                output_flags |= 8;
            const auto &bytes = output.bytes();
            if (!bytes.empty()) {
                check_buffer(header.control, bytes.size(), write_perm);
                memory_.copy_in(header.control, bytes.data(), bytes.size());
            }
            memory_.store(message->address + 40, bytes.size(), 8);
            memory_.store(message->address + 48, output_flags, 4);
        } catch (...) {
            for (int fd : installed)
                descriptors_.close(fd);
            throw;
        }
        message->progress = position + copied;
        message->output_flags = output_flags;
        const bool ancillary = !received.empty() || credentials_enabled || native.msg_controllen || (output_flags & 8);
        if (waitall && count && message->progress < message->bytes && !nonblocking && !ancillary &&
            !(output_flags & (1 | 32 | 128))) {
            if (peek) {
                pollfd event{host, POLLIN, 0};
                if (::poll(&event, 1, 0) < 0)
                    return message->progress ? int64_t(message->progress) : -socket_errno(errno);
                if (event.revents & POLLHUP)
                    return message->progress;
            }
            return {};
        }
        if (waitall)
            return message->progress;
        return std::max(count, datagram_size);
    };
    auto finish = [this, request, batch, sending](int64_t result) -> int64_t {
        if (result > 0) {
            if (auto &restart = scheduler_.current().signals.restart)
                restart->restartable = false;
        }
        if (batch && !sending && result > 0 && request->timeout_address) {
            check_buffer(request->timeout_address, 16, write_perm);
            const auto remaining = std::max<int64_t>(0, std::chrono::duration_cast<std::chrono::nanoseconds>(
                *request->deadline - TaskClock::now()).count());
            memory_.store(request->timeout_address, remaining / 1000000000, 8);
            memory_.store(request->timeout_address + 8, remaining % 1000000000, 8);
        }
        return result;
    };
    auto completed_message = [this, request, batch](int64_t count) {
        if (batch) {
            check_buffer(request->message->address + offsetof(MultiHeader, length), 4, write_perm);
            memory_.store(request->message->address + offsetof(MultiHeader, length), count, 4);
        }
        ++request->completed;
    };
    auto partial = [request, batch, sending, finish, completed_message](int64_t error) -> int64_t {
        if (!sending && request->message && request->message->progress) {
            const auto count = int64_t(request->message->progress);
            completed_message(count);
            request->message.reset();
            request->element_deadline.reset();
            return finish(batch ? request->completed : count);
        }
        return finish(batch && request->completed ? int64_t(request->completed) : error);
    };
    auto attempt = [this, request, state, sending, batch, nonblocking, guest_flags, load_message,
                    transfer, finish, completed_message, partial](int host) -> std::optional<int64_t> {
        while (request->completed < request->limit) {
            std::optional<int64_t> result;
            try {
                if (!request->message) {
                    request->message = load_message(request->address + uint64_t(request->completed) * sizeof(MultiHeader));
                    const uint64_t timeout = sending ? state->send_timeout_us : state->receive_timeout_us;
                    request->element_deadline = timeout ? TaskDeadline(message_deadline(timeout / 1000000,
                                                                                       timeout % 1000000 * 1000)) : TaskDeadline{};
                } else if (auto deadline = first_deadline(*request); deadline && TaskClock::now() >= *deadline)
                    return partial(request->deadline && TaskClock::now() >= *request->deadline ?
                                   (batch ? 0 : -11) : -11);
                result = transfer(host, request->message);
            } catch (const GuestError &error) {
                result = -error.number;
            } catch (const MemoryFault &) {
                result = -14;
            }
            if (!result) {
                // An explicit nonblocking operation still reports EAGAIN when
                // the timeout is zero and no message was received.
                if (nonblocking || (sending ? state->send_timeout_immediate : state->receive_timeout_immediate) ||
                    (batch && !sending && request->completed && (guest_flags & wait_for_one)))
                    return partial(-11);
                if (auto deadline = first_deadline(*request); deadline && TaskClock::now() >= *deadline)
                    return partial(request->deadline && TaskClock::now() >= *request->deadline ?
                                   (batch ? 0 : -11) : -11);
                if (interrupt_for_pending_signal())
                    return partial(-4);
                return {};
            }
            if (*result < 0) {
                if (!sending && request->message && request->message->progress)
                    return partial(*result);
                if (batch && !sending && request->completed && *result != -11)
                    defer_error(state, -*result);
                return finish(batch && request->completed ? int64_t(request->completed) : *result);
            }
            try {
                completed_message(*result);
            } catch (const GuestError &error) {
                return finish(batch && request->completed ? int64_t(request->completed) : -error.number);
            } catch (const MemoryFault &) {
                return finish(batch && request->completed ? int64_t(request->completed) : -14);
            }
            const bool short_send = sending && uint64_t(*result) < request->message->bytes;
            const bool out_of_band = request->message->output_flags & 1;
            request->message.reset();
            request->element_deadline.reset();
            if (!batch)
                return finish(*result);
            if (short_send || out_of_band || (request->deadline && TaskClock::now() >= *request->deadline))
                return finish(request->completed);
        }
        return finish(request->completed);
    };
    if (auto result = attempt(entry.host.get()))
        return *result;
    auto host = std::make_shared<FD>(::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    if (host->get() < 0)
        return partial(-linux_errno(errno));
    if (request->deadline || request->element_deadline)
        if (auto &restart = scheduler_.current().signals.restart)
            restart->restartable = false;
    scheduler_.block(first_deadline(*request), batch ? request->completed : -11,
                     [this, host, request, tid, attempt]() -> std::optional<int64_t> {
        auto result = attempt(host->get());
        if (!result) {
            auto &task = scheduler_.find(tid);
            task.deadline = first_deadline(*request);
            task.timeout_result = request->completed ? int64_t(request->completed) : -11;
        }
        return result;
    });
    arm_message_wait(scheduler_.current(), [this, tid, partial] {
        // Interruption can be initiated while another guest process is current.
        Scheduler::ScopedTask context(scheduler_, tid);
        return partial(-4);
    });
    return 0;
}
} // namespace operit::arm64
