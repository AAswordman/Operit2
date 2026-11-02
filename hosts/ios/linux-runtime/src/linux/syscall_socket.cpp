#include "linux/error_mapping.h"
#include "base/random.h"
#include "linux/network/message_batch.h"
#include "linux/network/socket.h"
#include "linux/syscalls.h"
#include <arpa/inet.h>
#include <array>
#include <cerrno>
#include <fcntl.h>
#include <poll.h>
#include <sys/ioctl.h>
#include <sys/un.h>

namespace operit::arm64 {
namespace {
void check_socket(int result) {
    if (result < 0)
        guest_error(socket_errno(errno));
}
std::string unix_path(const sockaddr *address, socklen_t length) {
    if (address->sa_family != AF_UNIX || length <= offsetof(sockaddr_un, sun_path))
        return {};
    const auto *value = reinterpret_cast<const sockaddr_un *>(address);
    const size_t available = std::min<size_t>(length - offsetof(sockaddr_un, sun_path), sizeof(value->sun_path));
    return std::string(value->sun_path, strnlen(value->sun_path, available));
}
SocketCredentials socket_credentials(uint32_t pid) {
    return {int32_t(pid), uint32_t(::geteuid()), uint32_t(::getegid())};
}
} // namespace
NativeAddress Syscalls::read_socket_address(uint64_t address, uint64_t length, bool binding) {
    length = uint32_t(length);
    if (length > 128 || length < 2)
        guest_error(22);
    check_buffer(address, length, read_perm);
    std::vector<uint8_t> bytes(length);
    memory_.copy_out(address, bytes.data(), bytes.size());
    uint16_t family;
    std::memcpy(&family, bytes.data(), 2);
    if (family == 1 && length > 2) {
        std::string host;
        if (bytes[2] == 0) {
            host = network_->unix.abstract(std::string(bytes.begin() + 3, bytes.end()));
        } else {
            auto end = std::find(bytes.begin() + 2, bytes.end(), uint8_t{0});
            std::string name(bytes.begin() + 2, end);
            auto path = filesystem_.resolve(name, fs_context_->cwd, false);
            if (binding)
                path.require_writable();
            if (path.virtual_path)
                guest_error(30);
            host = network_->unix.pathname(path);
        }
        network_->address_names[host] = bytes;
        bytes.resize(2 + host.size() + 1);
        std::memcpy(bytes.data() + 2, host.c_str(), host.size() + 1);
    }
    return socket_address(bytes);
}
std::vector<uint8_t> Syscalls::socket_address_bytes(const sockaddr *address, socklen_t length) {
    if (!length)
        return {};
    if (address->sa_family == AF_UNIX) {
        auto *value = reinterpret_cast<const sockaddr_un *>(address);
        const size_t available = length > offsetof(sockaddr_un, sun_path)
                                     ? std::min<size_t>(length - offsetof(sockaddr_un, sun_path),
                                                        sizeof(value->sun_path))
                                     : 0;
        std::string name(value->sun_path, strnlen(value->sun_path, available));
        auto found = network_->address_names.find(name);
        if (found != network_->address_names.end())
            return found->second;
        if (!name.empty())
            guest_error(2); // Never return a host pathname to a guest.
    }
    return guest_socket_address(address, length);
}
void Syscalls::write_socket_address(uint64_t address, uint64_t size_pointer,
                                    const std::vector<uint8_t> &bytes) {
    check_buffer(size_pointer, 4, read_perm | write_perm);
    int32_t capacity = int32_t(memory_.load(size_pointer, 4));
    if (capacity < 0)
        guest_error(22);
    size_t size = std::min<size_t>(uint32_t(capacity), bytes.size());
    if (size) {
        check_buffer(address, size, write_perm);
        memory_.copy_in(address, bytes.data(), size);
    }
    memory_.store(size_pointer, bytes.size(), 4);
}
int64_t Syscalls::socket_wait(Descriptor &entry, bool writing, bool nonblocking,
                              std::function<std::optional<int64_t>(int)> attempt,
                              std::function<int64_t()> timeout_value) {
    if (auto result = attempt(entry.host.get()))
        return *result;
    if (nonblocking || (entry.description->flags & 2048))
        return -11;
    auto state = entry.description->socket;
    const int64_t expired = timeout_value ? timeout_value() : -11;
    if (writing ? state->send_timeout_immediate : state->receive_timeout_immediate)
        return expired;
    auto host = std::make_shared<FD>(::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    check_socket(host->get());
    uint64_t timeout = writing ? state->send_timeout_us : state->receive_timeout_us;
    TaskDeadline deadline;
    if (timeout) {
        const auto now = TaskClock::now();
        const auto remaining =
            std::chrono::duration_cast<std::chrono::microseconds>(TaskClock::time_point::max() - now).count();
        deadline = timeout >= uint64_t(remaining) ? TaskClock::time_point::max()
                                                : now + std::chrono::microseconds(int64_t(timeout));
        if (auto &restart = scheduler_.current().signals.restart)
            restart->restartable = false;
    }
    auto tid = scheduler_.current().tid;
    if (interrupt_for_pending_signal())
        return -4;
    scheduler_.block(deadline, expired, [this, host, tid, attempt = std::move(attempt), timeout_value] {
        auto result = attempt(host->get());
        if (!result && timeout_value)
            scheduler_.find(tid).timeout_result = timeout_value();
        return result;
    });
    return 0;
}
int64_t Syscalls::dispatch_socket(CpuState &cpu) {
    const auto &a = cpu.x;
    if (a[8] == 198 || a[8] == 199) {
        if (uint32_t(a[1]) & ~uint32_t{15 | 2048 | 524288})
            guest_error(22);
        int family = socket_family(int32_t(a[0])), type = socket_type(int32_t(a[1]));
        if (int32_t(a[2]) < 0)
            guest_error(22);
        if ((family == AF_INET || family == AF_INET6) &&
            ((type == SOCK_STREAM && int32_t(a[2]) == IPPROTO_UDP) ||
             (type == SOCK_DGRAM && int32_t(a[2]) == IPPROTO_TCP)))
            guest_error(93);
        auto install = [this, a, family, type](FD host) {
            if (family == AF_INET6 && (type == SOCK_STREAM || type == SOCK_DGRAM)) {
                int only_v6 = 0;
                check_socket(::setsockopt(host.get(), IPPROTO_IPV6, IPV6_V6ONLY, &only_v6, sizeof(only_v6)));
            }
            configure_socket(host.get());
            int fd = descriptors_.insert(std::move(host), "", 2 | int(a[1] & (2048 | 524288)), 0);
            auto &d = *descriptors_.get(fd).description;
            d.socket = new_socket_state(int(a[0]), int(a[1] & 15), int(a[2]));
            d.socket->credentials = socket_credentials(scheduler_.current().pid);
            d.scheduler_io = true;
            return fd;
        };
        if (a[8] == 198) {
            FD host(::socket(family, type, int32_t(a[2])));
            check_socket(host.get());
            return install(std::move(host));
        }
        check_buffer(a[3], 8, write_perm);
        int pair[2];
        check_socket(::socketpair(family, type, int32_t(a[2]), pair));
        FD first(pair[0]), second(pair[1]);
        int one = install(std::move(first));
        int two;
        try {
            two = install(std::move(second));
        } catch (...) {
            descriptors_.close(one);
            throw;
        }
        descriptors_.get(one).description->socket->peer = descriptors_.get(two).description->socket;
        descriptors_.get(two).description->socket->peer = descriptors_.get(one).description->socket;
        if (family == AF_UNIX) {
            auto first_state = descriptors_.get(one).description->socket;
            auto second_state = descriptors_.get(two).description->socket;
            first_state->peer_credentials = second_state->credentials;
            second_state->peer_credentials = first_state->credentials;
        }
        memory_.store(a[3], one, 4);
        memory_.store(a[3] + 4, two, 4);
        return 0;
    }
    auto &entry = descriptors_.get(uint32_t(a[0]));
    auto state = entry.description->socket;
    if (!state)
        guest_error(88);
    switch (a[8]) {
    case 200: { // bind
        auto address = read_socket_address(a[1], a[2], true);
        check_socket(::bind(entry.host.get(), reinterpret_cast<sockaddr *>(&address.storage), address.length));
        if (state->domain == 1) {
            state->bound_path = unix_path(reinterpret_cast<sockaddr *>(&address.storage), address.length);
            state->local_name = socket_address_bytes(reinterpret_cast<sockaddr *>(&address.storage), address.length);
        }
        if (a[2] > 2 && memory_.load(a[1], 2) == 1 && memory_.load(a[1] + 2, 1) == 0)
            state->abstract_path = reinterpret_cast<sockaddr_un *>(&address.storage)->sun_path;
        return 0;
    }
    case 201:
        check_socket(::listen(entry.host.get(), int32_t(a[1]) < 0 ? SOMAXCONN : int32_t(a[1])));
        if (!state->listening)
            state->credentials = socket_credentials(scheduler_.current().pid);
        state->listening = true;
        return 0;
    case 202:
    case 242: { // accept/accept4
        uint32_t flags = a[8] == 242 ? uint32_t(a[3]) : 0;
        if (flags & ~uint32_t{2048 | 524288})
            guest_error(22);
        if (a[1]) {
            check_buffer(a[2], 4, read_perm | write_perm);
            if (int32_t(memory_.load(a[2], 4)) < 0)
                guest_error(22);
        }
        auto accept = [this, state, address = a[1], length = a[2],
                       flags](int host) -> std::optional<int64_t> {
            sockaddr_storage peer{};
            socklen_t size = sizeof(peer);
            FD accepted(::accept(host, reinterpret_cast<sockaddr *>(&peer), &size));
            if (accepted.get() < 0) {
                if (errno == EAGAIN || errno == EWOULDBLOCK)
                    return {};
                return -socket_errno(errno);
            }
            configure_socket(accepted.get());
            std::shared_ptr<SocketState> accepted_state;
            if (state->domain == 1) {
                const auto path = unix_path(reinterpret_cast<sockaddr *>(&peer), size);
                auto &pending = state->pending_connections;
                auto found = std::find_if(pending.begin(), pending.end(), [&path](const auto &connection) {
                    return !path.empty() && connection.peer_path == path;
                });
                if (found != pending.end()) {
                    accepted_state = std::move(found->socket);
                    pending.erase(found);
                    if (auto client = accepted_state->peer.lock())
                        client->listener.reset();
                }
            }
            if (!accepted_state) {
                accepted_state = new_socket_state(state->domain, state->type, state->protocol);
                accepted_state->credentials = state->credentials;
                socket_messages::pass_credentials(accepted_state, socket_messages::pass_credentials(state));
            }
            if (address)
                write_socket_address(address, length,
                                     socket_address_bytes(reinterpret_cast<sockaddr *>(&peer), size));
            int fd = descriptors_.insert(std::move(accepted), "", 2 | int(flags), 0);
            auto &description = *descriptors_.get(fd).description;
            description.scheduler_io = true;
            description.socket = std::move(accepted_state);
            description.socket->receive_timeout_us = state->receive_timeout_us;
            description.socket->send_timeout_us = state->send_timeout_us;
            description.socket->receive_timeout_immediate = state->receive_timeout_immediate;
            description.socket->send_timeout_immediate = state->send_timeout_immediate;
            return fd;
        };
        return socket_wait(entry, false, false, accept);
    }
    case 203: { // connect: host sockets always remain nonblocking.
        auto address = read_socket_address(a[1], a[2], false);
        std::shared_ptr<SocketState> listener;
        if (state->domain == 1 && (state->type == 1 || state->type == 5) &&
            !state->connecting && state->peer.expired()) {
            const auto path = unix_path(reinterpret_cast<sockaddr *>(&address.storage), address.length);
            for (const auto &weak : network_->sockets)
                if (auto candidate = weak.lock(); candidate && candidate->listening &&
                    candidate->domain == 1 && candidate->type == state->type &&
                    !path.empty() && candidate->bound_path == path) {
                    listener = std::move(candidate);
                    break;
                }
            if (listener) {
                sockaddr_storage local{};
                socklen_t size = sizeof(local);
                check_socket(::getsockname(entry.host.get(), reinterpret_cast<sockaddr *>(&local), &size));
                state->bound_path = unix_path(reinterpret_cast<sockaddr *>(&local), size);
                if (state->bound_path.empty()) {
                    // Match accept to this exact guest connector. An unnamed
                    // native client would be indistinguishable from an external
                    // host connection interleaved in the listener's backlog.
                    bool bound = false;
                    for (unsigned attempt = 0; attempt < 64 && !bound; ++attempt) {
                        std::array<uint8_t, 16> random{};
                        random_bytes(random.data(), random.size());
                        const std::string identity("\0ort-peer", 9);
                        const auto key = identity + std::string(reinterpret_cast<char *>(random.data()), random.size());
                        auto native_path = network_->unix.abstract(key);
                        std::vector<uint8_t> visible{1, 0};
                        std::vector<uint8_t> bytes(2 + native_path.size() + 1, 0);
                        bytes[0] = 1;
                        std::memcpy(bytes.data() + 2, native_path.c_str(), native_path.size() + 1);
                        const auto native = socket_address(bytes);
                        auto cleanup_path = native_path;
                        network_->address_names.emplace(native_path, visible);
                        if (::bind(entry.host.get(), reinterpret_cast<const sockaddr *>(&native.storage), native.length) < 0) {
                            if (errno == EADDRINUSE)
                                continue;
                            guest_error(socket_errno(errno));
                        }
                        state->abstract_path = std::move(cleanup_path);
                        state->bound_path = std::move(native_path);
                        state->local_name = std::move(visible);
                        bound = true;
                    }
                    if (!bound)
                        guest_error(98);
                }
            }
        }
        if (config_.trace_syscalls) {
            char text[INET6_ADDRSTRLEN]{};
            const auto *native = reinterpret_cast<const sockaddr *>(&address.storage);
            const void *source = nullptr;
            uint16_t port = 0;
            if (native->sa_family == AF_INET) {
                const auto *v4 = reinterpret_cast<const sockaddr_in *>(native);
                source = &v4->sin_addr;
                port = ntohs(v4->sin_port);
            } else if (native->sa_family == AF_INET6) {
                const auto *v6 = reinterpret_cast<const sockaddr_in6 *>(native);
                source = &v6->sin6_addr;
                port = ntohs(v6->sin6_port);
            }
            if (source && ::inet_ntop(native->sa_family, source, text, sizeof(text)))
                std::fprintf(stderr, "socket connect fd=%d address=%s port=%u\n", int32_t(a[0]), text, port);
        }
        int result;
#ifdef __APPLE__
        if (address.storage.ss_family == AF_UNSPEC && state->type == 2 &&
            (state->domain == 2 || state->domain == 10)) {
            result = ::disconnectx(entry.host.get(), SAE_ASSOCID_ANY, SAE_CONNID_ANY);
            if (result < 0 && errno == ENOTCONN)
                result = 0;
        } else
#endif
            result = ::connect(entry.host.get(), reinterpret_cast<sockaddr *>(&address.storage), address.length);
        const int error = result < 0 ? socket_errno(errno) : 0;
        if (listener && (!error || error == 115 || error == 114)) {
            auto accepted = new_socket_state(state->domain, state->type, state->protocol);
            accepted->credentials = listener->credentials;
            accepted->peer_credentials = socket_credentials(scheduler_.current().pid);
            accepted->peer = state;
            accepted->local_name = listener->local_name;
            accepted->peer_name = state->local_name;
            socket_messages::pass_credentials(accepted, socket_messages::pass_credentials(listener));
            listener->pending_connections.push_back({state->bound_path, accepted});
            state->credentials = *accepted->peer_credentials;
            state->peer_credentials = listener->credentials;
            state->peer = accepted;
            state->peer_name = listener->local_name;
            state->listener = listener;
        }
        if (!error || (error == 106 && state->connecting)) {
            state->connecting = false;
            return 0;
        }
        if (error != 115 && error != 114) {
            if (state->connecting)
                state->abandon_connection();
            state->connecting = false;
            return -error;
        }
        state->connecting = true;
        if (entry.description->flags & 2048)
            return -error;
        auto pending = [state](int host) -> std::optional<int64_t> {
            pollfd ready{host, POLLOUT, 0};
            int count = ::poll(&ready, 1, 0);
            if (count < 0)
                return -socket_errno(errno);
            if (!count)
                return {};
            if (ready.revents & POLLNVAL)
                return -9;
            if (!(ready.revents & (POLLOUT | POLLERR | POLLHUP)))
                return {};
            int error = 0;
            socklen_t size = sizeof(error);
            if (::getsockopt(host, SOL_SOCKET, SO_ERROR, &error, &size) < 0)
                return -socket_errno(errno);
            if (error) {
                state->connecting = false;
                state->abandon_connection();
                return -socket_errno(error);
            }
            // SO_ERROR == 0 by itself does not establish that connect finished.
            sockaddr_storage peer{};
            size = sizeof(peer);
            if (::getpeername(host, reinterpret_cast<sockaddr *>(&peer), &size) < 0)
                return -socket_errno(errno);
            state->connecting = false;
            return 0;
        };
        return socket_wait(entry, true, false, pending, [] { return int64_t{-115}; });
    }
    case 204:
    case 205: { // getsockname/getpeername
        sockaddr_storage address{};
        socklen_t length = sizeof(address);
        check_socket(a[8] == 204
                       ? ::getsockname(entry.host.get(), reinterpret_cast<sockaddr *>(&address), &length)
                       : ::getpeername(entry.host.get(), reinterpret_cast<sockaddr *>(&address), &length));
        write_socket_address(a[1], a[2],
                             state->domain == 1 && a[8] == 205 && !state->peer_name.empty()
                                 ? state->peer_name
                                 : socket_address_bytes(reinterpret_cast<sockaddr *>(&address), length));
        return 0;
    }
    case 206:
    case 207: { // sendto/recvfrom
        bool sending = a[8] == 206;
        int flags = socket_message_flags(a[3], sending);
        size_t count = std::min<uint64_t>(a[2], max_io);
        check_buffer(a[1], count, sending ? read_perm : write_perm);
        if (!sending && (a[3] & 256) && state->type == 1) {
            if (a[4]) {
                check_buffer(a[5], 4, write_perm);
                memory_.store(a[5], 0, 4);
            }
            return receive_waitall(entry, {{a[1], count}}, a[3]);
        }
        NativeAddress destination;
        if (sending && a[4])
            destination = read_socket_address(a[4], a[5], false);
        if (!sending && a[4])
            check_buffer(a[5], 4, read_perm | write_perm);
        auto transfer = [this, sending, address = a[1], count, flags, destination, output = a[4],
                         length = a[5], guest_flags = a[3], state,
                         credentials = socket_messages::Credentials{int32_t(scheduler_.current().pid),
                                                                    uint32_t(::getuid()), uint32_t(::getgid())},
                         tid = scheduler_.current().tid](int host) -> std::optional<int64_t> {
            if (!sending)
                if (const int error = socket_messages::take_error(state))
                    return -error;
            check_buffer(address, count, sending ? read_perm : write_perm);
            const size_t capacity = sending ? count : socket_messages::credential_receive_limit(state, count);
            std::vector<uint8_t> bytes(capacity);
            sockaddr_storage peer{};
            socklen_t peer_size = sizeof(peer);
            int datagram_size = 0;
            if (!sending && (guest_flags & 32) && (state->type == 2 || state->type == 5)) {
                pollfd event{host, POLLIN, 0};
                int ready = ::poll(&event, 1, 0);
                if (ready < 0)
                    return -socket_errno(errno);
                if (!ready)
                    return {};
                std::vector<uint8_t> packet(max_io);
                ssize_t pending = ::recv(host, packet.data(), packet.size(), MSG_PEEK | MSG_DONTWAIT);
                if (pending < 0) {
                    if (errno == EAGAIN || errno == EWOULDBLOCK)
                        return {};
                    return -socket_errno(errno);
                }
                datagram_size = pending;
            }
            ssize_t result;
            if (sending) {
                memory_.copy_out(address, bytes.data(), count);
                result = ::sendto(
                    host, bytes.data(), count, flags,
                    destination.length ? reinterpret_cast<const sockaddr *>(&destination.storage) : nullptr,
                    destination.length);
            } else
                result = ::recvfrom(host, bytes.data(), capacity, flags, reinterpret_cast<sockaddr *>(&peer),
                                    &peer_size);
            if (result < 0) {
                int error = socket_errno(errno);
                if (error == 11)
                    return {};
                if (sending && error == 108)
                    error = 32;
                if (sending && error == 32 && !(guest_flags & 0x4000))
                    broken_pipe(tid);
                return -error;
            }
            if (sending) {
                socket_messages::sent_credentials(state, result, credentials);
                state->sent(result);
            } else {
                state->received(result, guest_flags & 2);
                if (!(state->type == 1 && (guest_flags & 32)))
                    memory_.copy_in(address, bytes.data(), std::min<size_t>(result, capacity));
                if (output)
                    write_socket_address(
                        output, length, socket_address_bytes(reinterpret_cast<sockaddr *>(&peer), peer_size));
                if ((guest_flags & 32) && (state->type == 2 || state->type == 5))
                    result = std::max<ssize_t>(result, datagram_size);
            }
            return result;
        };
        return socket_wait(entry, sending, a[3] & 64, transfer);
    }
    case 208:
    case 209: { // setsockopt/getsockopt: never pass a guest payload ABI to Darwin.
        const bool setting = a[8] == 208;
        const int level = int32_t(a[1]), name = int32_t(a[2]);
        const bool timeout = level == 1 && (name == 20 || name == 21 || name == 66 || name == 67);
        if (setting) {
            const int32_t length = int32_t(a[4]);
            if (length < 0)
                guest_error(22);
            if (level == 1 && (name == 3 || name == 4 || name == 17 || name == 19 || name == 30 ||
                               name == 38 || name == 39))
                guest_error(92);
            if (!timeout && !(level == 1 && name == 16))
                socket_option(level, name); // Reject unknown options before copying an optval.
            size_t size = timeout ? 16 : level == 1 && name == 13 ? 8 : 4;
            if (level == 0 && (name == 1 || name == 2 || name == 33 || name == 34) && length < 4)
                size = length ? 1 : 0;
            if (level == 0 && (name == 32 || name == 35 || name == 36))
                size = length >= 12 ? 12 : length >= 8 ? 8 : name == 32 ? 4 : 8;
            if (level == 41 && (name == 20 || name == 21))
                size = 20;
            if (uint32_t(length) < size)
                guest_error(22);
            std::vector<uint8_t> value(size);
            if (size) {
                check_buffer(a[3], size, read_perm);
                memory_.copy_out(a[3], value.data(), size);
            }
            set_socket_option(entry.host.get(), state, level, name, value);
        } else {
            check_buffer(a[4], 4, read_perm | write_perm);
            const int32_t capacity = int32_t(memory_.load(a[4], 4));
            if (capacity < 0)
                guest_error(22);
            const auto value = get_socket_option(entry.host.get(), state, level, name, uint32_t(capacity));
            const size_t size = std::min<size_t>(uint32_t(capacity), value.size());
            if (size) {
                check_buffer(a[3], size, write_perm);
                memory_.copy_in(a[3], value.data(), size);
            }
            memory_.store(a[4], size, 4);
        }
        return 0;
    }
    case 210: { // shutdown
        const int how = int32_t(a[1]);
        if (how < 0 || how > 2)
            guest_error(22);
        const int native = how == 0 ? SHUT_RD : how == 1 ? SHUT_WR : SHUT_RDWR;
        check_socket(::shutdown(entry.host.get(), native));
        return 0;
    }
    case 211:
    case 212:
        return socket_message(cpu);
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
