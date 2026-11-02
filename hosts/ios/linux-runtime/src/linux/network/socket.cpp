#include "linux/network/socket.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "linux/network/message_batch.h"
#include <arpa/inet.h>
#include <algorithm>
#include <array>
#include <climits>
#include <cerrno>
#include <cstddef>
#include <cstring>
#include <fcntl.h>
#include <ifaddrs.h>
#include <net/if.h>
#include <netinet/tcp.h>
#include <sys/un.h>

namespace operit::arm64 {
int socket_errno(int error) {
    if (error == EWOULDBLOCK)
        return 11;
#ifdef EPROTO
    if (error == EPROTO)
        return 71;
#endif
    return linux_errno(error);
}
namespace {
void check_socket(int result) {
    if (result < 0)
        guest_error(socket_errno(errno));
}
} // namespace
SocketState::~SocketState() {
    if (!abstract_path.empty())
        ::unlink(abstract_path.c_str());
}
void SocketState::abandon_connection() {
    if (auto owner = listener.lock()) {
        auto connected = peer.lock();
        auto &pending = owner->pending_connections;
        pending.erase(std::remove_if(pending.begin(), pending.end(), [&connected](const auto &connection) {
            return connection.socket == connected;
        }), pending.end());
    }
    listener.reset();
    peer.reset();
    peer_credentials.reset();
    peer_name.clear();
}
void SocketState::sent(size_t count, const std::vector<std::shared_ptr<Descriptor>> &rights) {
    if (!count && type == 1)
        return;
    if (!rights.empty())
        if (auto receiver = peer.lock()) {
            auto queued = rights;
            for (auto &descriptor : queued)
                if (descriptor->description && descriptor->description->socket == receiver) {
                    receiver->self_description = descriptor->description;
                    descriptor = std::make_shared<Descriptor>(FD(), *descriptor, false);
                    descriptor->description.reset();
                }
            receiver->pending_rights.push_back({sent_position, std::move(queued)});
        }
    sent_position += type == 1 ? count : 1;
}
std::vector<std::shared_ptr<Descriptor>> SocketState::received(size_t count, bool peek) {
    if (!count && type == 1)
        return {};
    uint64_t end = received_position + (type == 1 ? count : 1);
    std::vector<std::shared_ptr<Descriptor>> result;
    for (const auto &batch : pending_rights)
        if (batch.position >= received_position && batch.position < end)
            for (const auto &descriptor : batch.descriptors) {
                if (descriptor->description)
                    result.push_back(descriptor);
                else {
                    auto restored = std::make_shared<Descriptor>(FD(), *descriptor, false);
                    restored->description = self_description.lock();
                    result.push_back(std::move(restored));
                }
            }
    if (!peek) {
        received_position = end;
        while (!pending_rights.empty() && pending_rights.front().position < end)
            pending_rights.pop_front();
    }
    return result;
}
int socket_family(int guest) {
    switch (guest) {
    case 0:
        return AF_UNSPEC;
    case 1:
        return AF_UNIX;
    case 2:
        return AF_INET;
    case 10:
        return AF_INET6;
    default:
        guest_error(97);
    }
}
int guest_socket_family(int native) {
    switch (native) {
    case AF_UNSPEC:
        return 0;
    case AF_UNIX:
        return 1;
    case AF_INET:
        return 2;
    case AF_INET6:
        return 10;
    default:
        guest_error(97);
    }
}
int socket_type(int guest) {
    switch (guest & 15) {
    case 1:
        return SOCK_STREAM;
    case 2:
        return SOCK_DGRAM;
    case 3:
        return SOCK_RAW;
    case 5:
        return SOCK_SEQPACKET;
    default:
        guest_error(22);
    }
}
void configure_socket(int fd) {
    check_socket(::fcntl(fd, F_SETFD, FD_CLOEXEC));
    int flags = ::fcntl(fd, F_GETFL);
    check_socket(flags);
    check_socket(::fcntl(fd, F_SETFL, flags | O_NONBLOCK));
#ifdef __APPLE__
    int enabled = 1;
    check_socket(::setsockopt(fd, SOL_SOCKET, SO_NOSIGPIPE, &enabled, sizeof(enabled)));
    int low_water = 1;
    check_socket(::setsockopt(fd, SOL_SOCKET, SO_SNDLOWAT, &low_water, sizeof(low_water)));
#endif
}
int socket_message_flags(uint64_t guest, bool sending) {
    constexpr uint64_t supported = 1 | 2 | 4 | 8 | 32 | 64 | 128 | 256 | 16384 | 0x40000000;
    if (guest & ~supported)
        guest_error(95);
    int flags = 0;
    if (guest & 1)
        flags |= MSG_OOB;
    if (guest & 2)
        flags |= MSG_PEEK;
    if (guest & 4)
        flags |= MSG_DONTROUTE;
    if (guest & 64)
        flags |= MSG_DONTWAIT;
    if (guest & 128)
        flags |= MSG_EOR;
    if (guest & 256)
        flags |= MSG_WAITALL;
#ifndef __APPLE__
    if (sending)
        flags |= MSG_NOSIGNAL;
    if (guest & 0x40000000)
        flags |= MSG_CMSG_CLOEXEC;
#endif
    // MSG_CTRUNC/MSG_TRUNC are output flags; datagram MSG_TRUNC input is handled
    // by recvmsg's reported message length in the syscall layer.
    if (sending && (guest & (8 | 32 | 256 | 0x40000000)))
        guest_error(95);
    return flags;
}
NativeAddress socket_address(const std::vector<uint8_t> &guest) {
    if (guest.size() < 2 || guest.size() > 128)
        guest_error(22);
    uint16_t family;
    std::memcpy(&family, guest.data(), 2);
    NativeAddress result;
    int native = socket_family(family);
    if (native == AF_INET) {
        if (guest.size() < 16)
            guest_error(22);
        auto *address = reinterpret_cast<sockaddr_in *>(&result.storage);
        address->sin_family = AF_INET;
        std::memcpy(&address->sin_port, guest.data() + 2, 2);
        std::memcpy(&address->sin_addr, guest.data() + 4, 4);
        result.length = sizeof(*address);
#ifdef __APPLE__
        address->sin_len = result.length;
#endif
    } else if (native == AF_INET6) {
        if (guest.size() < 24)
            guest_error(22);
        auto *address = reinterpret_cast<sockaddr_in6 *>(&result.storage);
        address->sin6_family = AF_INET6;
        std::memcpy(&address->sin6_port, guest.data() + 2, 2);
        std::memcpy(&address->sin6_flowinfo, guest.data() + 4, 4);
        std::memcpy(&address->sin6_addr, guest.data() + 8, 16);
        if (guest.size() >= 28)
            std::memcpy(&address->sin6_scope_id, guest.data() + 24, 4);
        result.length = sizeof(*address);
#ifdef __APPLE__
        address->sin6_len = result.length;
#endif
    } else if (native == AF_UNIX) {
        auto *address = reinterpret_cast<sockaddr_un *>(&result.storage);
        address->sun_family = AF_UNIX;
        if (guest.size() > 2) {
            if (guest[2] == 0)
                guest_error(97); // Abstract namespace needs its own guest namespace broker.
            size_t size = strnlen(reinterpret_cast<const char *>(guest.data() + 2), guest.size() - 2);
            if (size >= sizeof(address->sun_path))
                guest_error(36);
            std::memcpy(address->sun_path, guest.data() + 2, size);
            result.length = offsetof(sockaddr_un, sun_path) + size + 1;
        } else
            result.length = offsetof(sockaddr_un, sun_path);
#ifdef __APPLE__
        address->sun_len = result.length;
#endif
    } else {
        auto *address = reinterpret_cast<sockaddr *>(&result.storage);
        address->sa_family = AF_UNSPEC;
        result.length = sizeof(sockaddr);
#ifdef __APPLE__
        address->sa_len = result.length;
#endif
    }
    return result;
}
std::vector<uint8_t> guest_socket_address(const sockaddr *native, socklen_t length) {
    if (!length)
        return {};
    if (!native || length < offsetof(sockaddr, sa_data))
        guest_error(22);
    uint16_t family = guest_socket_family(native->sa_family);
    std::vector<uint8_t> bytes;
    if (family == 2) {
        if (length < sizeof(sockaddr_in))
            guest_error(22);
        auto *address = reinterpret_cast<const sockaddr_in *>(native);
        bytes.resize(16);
        std::memcpy(bytes.data() + 2, &address->sin_port, 2);
        std::memcpy(bytes.data() + 4, &address->sin_addr, 4);
    } else if (family == 10) {
        if (length < sizeof(sockaddr_in6))
            guest_error(22);
        auto *address = reinterpret_cast<const sockaddr_in6 *>(native);
        bytes.resize(28);
        std::memcpy(bytes.data() + 2, &address->sin6_port, 2);
        std::memcpy(bytes.data() + 4, &address->sin6_flowinfo, 4);
        std::memcpy(bytes.data() + 8, &address->sin6_addr, 16);
        std::memcpy(bytes.data() + 24, &address->sin6_scope_id, 4);
    } else if (family == 1) {
        auto *address = reinterpret_cast<const sockaddr_un *>(native);
        size_t size =
            length > offsetof(sockaddr_un, sun_path)
                ? std::min<size_t>(length - offsetof(sockaddr_un, sun_path), sizeof(address->sun_path))
                : 0;
        bytes.resize(2 + size);
        if (size)
            std::memcpy(bytes.data() + 2, address->sun_path, size);
    } else
        bytes.resize(2);
    std::memcpy(bytes.data(), &family, 2);
    return bytes;
}
SocketOption socket_option(int level, int option) {
    if (level == 1) {
        switch (option) {
        case 2:
            return {SOL_SOCKET, SO_REUSEADDR};
        case 3:
            return {SOL_SOCKET, SO_TYPE};
        case 4:
            return {SOL_SOCKET, SO_ERROR};
        case 5:
            return {SOL_SOCKET, SO_DONTROUTE};
        case 6:
            return {SOL_SOCKET, SO_BROADCAST};
        case 7:
            return {SOL_SOCKET, SO_SNDBUF};
        case 8:
            return {SOL_SOCKET, SO_RCVBUF};
        case 9:
            return {SOL_SOCKET, SO_KEEPALIVE};
        case 10:
            return {SOL_SOCKET, SO_OOBINLINE};
        case 13:
#ifdef __APPLE__
            return {SOL_SOCKET, SO_LINGER_SEC};
#else
            return {SOL_SOCKET, SO_LINGER};
#endif
        case 15:
            return {SOL_SOCKET, SO_REUSEPORT};
        case 18:
            return {SOL_SOCKET, SO_RCVLOWAT};
        case 19:
            return {SOL_SOCKET, SO_SNDLOWAT};
        case 30:
            return {SOL_SOCKET, SO_ACCEPTCONN};
        default:
            guest_error(92);
        }
    }
    if (level == 6) {
        switch (option) {
        case 1:
            return {IPPROTO_TCP, TCP_NODELAY};
        case 2:
            return {IPPROTO_TCP, TCP_MAXSEG};
#ifdef __APPLE__
        case 4:
            return {IPPROTO_TCP, TCP_KEEPALIVE};
#else
        case 4:
            return {IPPROTO_TCP, TCP_KEEPIDLE};
#endif
        case 5:
            return {IPPROTO_TCP, TCP_KEEPINTVL};
        case 6:
            return {IPPROTO_TCP, TCP_KEEPCNT};
        default:
            guest_error(92);
        }
    }
    if (level == 0) {
        switch (option) {
        case 1:
            return {IPPROTO_IP, IP_TOS};
        case 2:
            return {IPPROTO_IP, IP_TTL};
        case 32:
            return {IPPROTO_IP, IP_MULTICAST_IF};
        case 33:
            return {IPPROTO_IP, IP_MULTICAST_TTL};
        case 34:
            return {IPPROTO_IP, IP_MULTICAST_LOOP};
        case 35:
            return {IPPROTO_IP, IP_ADD_MEMBERSHIP};
        case 36:
            return {IPPROTO_IP, IP_DROP_MEMBERSHIP};
        default:
            guest_error(92);
        }
    }
    if (level == 41) {
        switch (option) {
        case 16:
            return {IPPROTO_IPV6, IPV6_UNICAST_HOPS};
        case 17:
            return {IPPROTO_IPV6, IPV6_MULTICAST_IF};
        case 18:
            return {IPPROTO_IPV6, IPV6_MULTICAST_HOPS};
        case 19:
            return {IPPROTO_IPV6, IPV6_MULTICAST_LOOP};
        case 20:
            return {IPPROTO_IPV6, IPV6_JOIN_GROUP};
        case 21:
            return {IPPROTO_IPV6, IPV6_LEAVE_GROUP};
        case 26:
            return {IPPROTO_IPV6, IPV6_V6ONLY};
        case 67:
            return {IPPROTO_IPV6, IPV6_TCLASS};
        default:
            guest_error(92);
        }
    }
    guest_error(92);
}
namespace {
template <class T> T option_value(const std::vector<uint8_t> &bytes) {
    if (bytes.size() < sizeof(T))
        guest_error(22);
    T value{};
    std::memcpy(&value, bytes.data(), sizeof(value));
    return value;
}
template <class T> std::vector<uint8_t> option_bytes(const T &value) {
    std::vector<uint8_t> bytes(sizeof(value));
    std::memcpy(bytes.data(), &value, sizeof(value));
    return bytes;
}
bool timeout_option(int level, int name) {
    return level == 1 && (name == 20 || name == 21 || name == 66 || name == 67);
}
bool receive_timeout_option(int name) {
    return name == 20 || name == 66;
}
void require_option_protocol(const SocketState &state, int level) {
    if ((level == 0 && state.domain != 2 && state.domain != 10) ||
        (level == 41 && state.domain != 10) ||
        (level == 6 && ((state.domain != 2 && state.domain != 10) || state.type != 1)))
        guest_error(92);
}
bool byte_ip_option(int level, int name) {
    return level == 0 && (name == 1 || name == 2 || name == 33 || name == 34);
}
#ifdef __APPLE__
in_addr multicast_interface(const std::vector<uint8_t> &bytes, bool membership) {
    in_addr address{};
    if (bytes.size() >= 8)
        std::memcpy(&address, bytes.data() + 4, 4);
    else if (!membership)
        address = option_value<in_addr>(bytes);
    else
        guest_error(22);
    if (bytes.size() >= 12) {
        int32_t index;
        std::memcpy(&index, bytes.data() + 8, 4);
        if (index) {
            ifaddrs *interfaces = nullptr;
            check_socket(::getifaddrs(&interfaces));
            std::unique_ptr<ifaddrs, decltype(&::freeifaddrs)> owner(interfaces, ::freeifaddrs);
            for (auto *item = interfaces; item; item = item->ifa_next) {
                if (item->ifa_addr && item->ifa_addr->sa_family == AF_INET &&
                    ::if_nametoindex(item->ifa_name) == uint32_t(index))
                    return reinterpret_cast<const sockaddr_in *>(item->ifa_addr)->sin_addr;
            }
            guest_error(99);
        }
    }
    return address;
}
#endif
} // namespace

void set_socket_option(int fd, const std::shared_ptr<SocketState> &socket, int level, int name,
                       const std::vector<uint8_t> &bytes) {
    auto &state = *socket;
    if (level == 1 && name == 16) {
        socket_messages::pass_credentials(socket, option_value<int32_t>(bytes) != 0);
        return;
    }
    if (timeout_option(level, name)) {
        const auto value = option_value<std::array<int64_t, 2>>(bytes);
        if (value[1] < 0 || value[1] >= 1000000)
            guest_error(33);
        auto &timeout = receive_timeout_option(name) ? state.receive_timeout_us : state.send_timeout_us;
        auto &immediate = receive_timeout_option(name) ? state.receive_timeout_immediate
                                                      : state.send_timeout_immediate;
        immediate = value[0] < 0;
        // Zero means no deadline to all socket I/O callers. Negative seconds
        // mean an immediate timeout in Linux, while huge durations saturate.
        if (immediate)
            timeout = 1;
        else if (uint64_t(value[0]) > (uint64_t(INT64_MAX) - uint64_t(value[1])) / 1000000)
            timeout = 0;
        else {
            timeout = uint64_t(value[0]) * 1000000 + uint64_t(value[1]);
            const auto maximum = std::chrono::duration_cast<std::chrono::microseconds>(
                                     std::chrono::steady_clock::time_point::max() -
                                     std::chrono::steady_clock::now()).count();
            // Every existing I/O caller consumes this duration directly, so it
            // must also fit the host steady clock's (usually nanosecond) range.
            if (timeout >= uint64_t(maximum))
                timeout = 0;
        }
        return;
    }
    require_option_protocol(state, level);
    // These are Linux query-only options; Darwin allows setting SO_SNDLOWAT.
    if (level == 1 && (name == 3 || name == 4 || name == 17 || name == 19 || name == 30 ||
                       name == 38 || name == 39))
        guest_error(92);
    const auto option = socket_option(level, name);
    if (level == 1 && name == 13) {
        const auto value = option_value<std::array<int32_t, 2>>(bytes);
        linger previous{};
        socklen_t size = sizeof(previous);
        check_socket(::getsockopt(fd, option.level, option.name, &previous, &size));
        linger native{};
        native.l_onoff = value[0] != 0;
        native.l_linger = native.l_onoff ? value[1] : previous.l_linger;
#ifdef __APPLE__
        if (native.l_onoff && native.l_linger < 0)
            guest_error(95);
#endif
        check_socket(::setsockopt(fd, option.level, option.name, &native, sizeof(native)));
#ifdef __APPLE__
        // Darwin stores linger in a short tick count, even for SO_LINGER_SEC.
        // Reject a duration it truncates rather than report a false success.
        linger actual{};
        size = sizeof(actual);
        check_socket(::getsockopt(fd, option.level, option.name, &actual, &size));
        if (native.l_onoff && actual.l_linger != native.l_linger) {
            check_socket(::setsockopt(fd, option.level, option.name, &previous, sizeof(previous)));
            guest_error(95);
        }
#endif
        return;
    }
    if (level == 0 && (name == 32 || name == 35 || name == 36)) {
#ifdef __APPLE__
        in_addr address = multicast_interface(bytes, name != 32);
        if (name == 32)
            check_socket(::setsockopt(fd, option.level, option.name, &address, sizeof(address)));
        else {
            ip_mreq request{};
            std::memcpy(&request.imr_multiaddr, bytes.data(), 4);
            request.imr_interface = address;
            check_socket(::setsockopt(fd, option.level, option.name, &request, sizeof(request)));
        }
#else
        if (bytes.size() < (name == 32 ? 4u : 8u))
            guest_error(22);
        check_socket(::setsockopt(fd, option.level, option.name, bytes.data(), bytes.size()));
#endif
        return;
    }
    if (level == 41 && (name == 20 || name == 21)) {
        const auto value = option_value<std::array<uint32_t, 5>>(bytes);
        ipv6_mreq request{};
        std::memcpy(&request.ipv6mr_multiaddr, value.data(), 16);
        request.ipv6mr_interface = value[4];
        check_socket(::setsockopt(fd, option.level, option.name, &request, sizeof(request)));
        return;
    }
    int value;
    if (byte_ip_option(level, name) && bytes.size() < 4)
        value = bytes.empty() ? 0 : bytes[0];
    else
        value = option_value<int32_t>(bytes);
    if (level == 1) {
        switch (name) {
        case 2:  // SO_REUSEADDR
        case 5:  // SO_DONTROUTE
        case 6:  // SO_BROADCAST
        case 9:  // SO_KEEPALIVE
        case 10: // SO_OOBINLINE
        case 15: // SO_REUSEPORT
            value = value != 0;
            break;
        case 18: // SO_RCVLOWAT
            value = value < 0 ? INT_MAX : std::max(value, 1);
            break;
        }
    }
    if (level == 6) {
        if ((name == 4 || name == 5) && (value < 1 || value > 32767))
            guest_error(22);
        if (name == 6 && (value < 1 || value > 127))
            guest_error(22);
        if (name == 1)
            value = value != 0;
    }
    if (level == 0) {
        if (name == 1)
            value = uint8_t(value); // Linux retains the low eight TOS bits.
        if (name == 2 && (value < -1 || value == 0 || value > 255))
            guest_error(22);
        if (name == 33) {
            if (value < -1 || value > 255)
                guest_error(22);
            if (value == -1)
                value = 1;
        }
        if (name == 34)
            value = value != 0;
    }
    if (level == 41) {
        if ((name == 16 || name == 18 || name == 67) && (value < -1 || value > 255))
            guest_error(22);
        if (name == 19 && value != 0 && value != 1)
            guest_error(22);
        if (name == 26)
            value = value != 0;
    }
#ifdef __APPLE__
    if (level == 0 && name == 2 && value == -1) {
        FD defaults(::socket(AF_INET, SOCK_DGRAM, 0));
        check_socket(defaults.get());
        socklen_t size = sizeof(value);
        check_socket(::getsockopt(defaults.get(), IPPROTO_IP, IP_TTL, &value, &size));
    }
    if (level == 0 && (name == 33 || name == 34)) {
        const auto native = static_cast<unsigned char>(value);
        check_socket(::setsockopt(fd, option.level, option.name, &native, sizeof(native)));
        return;
    }
    if (level == 1 && (name == 7 || name == 8)) {
        // Linux buffers are hints, clamped to the available kernel limit. BSD
        // rejects oversized hints instead, so retry with a smaller real buffer.
        const int minimum = name == 7 ? 1024 : 128;
        value = std::max(minimum, int(std::min<uint32_t>(uint32_t(value), INT_MAX / 2)));
        while (::setsockopt(fd, option.level, option.name, &value, sizeof(value)) < 0) {
            const int error = errno;
            if (error != ENOBUFS || value == minimum)
                guest_error(socket_errno(error));
            value = std::max(minimum, value / 2);
        }
        return;
    }
#endif
    check_socket(::setsockopt(fd, option.level, option.name, &value, sizeof(value)));
}

std::vector<uint8_t> get_socket_option(int fd, const std::shared_ptr<SocketState> &socket, int level, int name,
                                     size_t capacity) {
    auto &state = *socket;
    if (level == 1 && name == 16)
        return option_bytes(int32_t(socket_messages::pass_credentials(socket)));
    if (level == 1 && name == 17) {
        if (state.peer_credentials)
            return option_bytes(*state.peer_credentials);
        if (state.domain == 1) {
            sockaddr_storage peer{};
            socklen_t length = sizeof(peer);
            if (::getpeername(fd, reinterpret_cast<sockaddr *>(&peer), &length) == 0)
                guest_error(95); // A native peer PID is not a guest process identity.
            if (errno != ENOTCONN && errno != EINVAL)
                guest_error(socket_errno(errno));
        }
        return option_bytes(SocketCredentials{});
    }
    if (level == 1 && name == 4)
        if (const int error = socket_messages::take_error(socket))
            return option_bytes(int32_t(error));
    if (timeout_option(level, name)) {
        const bool immediate = receive_timeout_option(name) ? state.receive_timeout_immediate
                                                            : state.send_timeout_immediate;
        const uint64_t timeout = receive_timeout_option(name) ? state.receive_timeout_us
                                                              : state.send_timeout_us;
        return option_bytes(std::array<int64_t, 2>{immediate ? 0 : int64_t(timeout / 1000000),
                                                   immediate ? 0 : int64_t(timeout % 1000000)});
    }
    require_option_protocol(state, level);
    if (level == 1) {
        if (name == 3)
            return option_bytes(int32_t(state.type));
        if (name == 19)
            return option_bytes(int32_t{1});
        if (name == 38) {
            int32_t protocol = state.protocol;
            if (!protocol && (state.domain == 2 || state.domain == 10))
                protocol = state.type == 1 ? IPPROTO_TCP : state.type == 2 ? IPPROTO_UDP : 0;
            return option_bytes(protocol);
        }
        if (name == 39)
            return option_bytes(int32_t(state.domain));
    }
    const auto option = socket_option(level, name);
    if ((level == 0 && (name == 35 || name == 36)) ||
        (level == 41 && (name == 20 || name == 21)))
        guest_error(92); // Membership operations have no getsockopt form.
    if (level == 1 && name == 13) {
        linger native{};
        socklen_t size = sizeof(native);
        check_socket(::getsockopt(fd, option.level, option.name, &native, &size));
        return option_bytes(std::array<int32_t, 2>{native.l_onoff != 0, native.l_linger});
    }
    if (level == 0 && name == 32) {
        in_addr native{};
        socklen_t size = sizeof(native);
        check_socket(::getsockopt(fd, option.level, option.name, &native, &size));
        return option_bytes(native);
    }
    int value = 0;
    socklen_t size = sizeof(value);
#ifdef __APPLE__
    if (level == 0 && (name == 33 || name == 34)) {
        unsigned char native = 0;
        size = sizeof(native);
        check_socket(::getsockopt(fd, option.level, option.name, &native, &size));
        value = native;
    } else
#endif
        check_socket(::getsockopt(fd, option.level, option.name, &value, &size));
    if (level == 1 && name == 4) {
        value = socket_errno(value);
        if (value && state.connecting) {
            state.connecting = false;
            state.abandon_connection();
        }
    }
    if ((level == 1 && (name == 2 || name == 5 || name == 6 || name == 9 || name == 10 || name == 15 || name == 30)) ||
        (level == 6 && name == 1) || (level == 41 && (name == 19 || name == 26)))
        value = value != 0;
#ifdef __APPLE__
    // Linux reports the doubled buffer size, including its accounting allowance.
    if (level == 1 && (name == 7 || name == 8))
        value = value > INT_MAX / 2 ? INT_MAX : value * 2;
#endif
    if (byte_ip_option(level, name) && capacity && capacity < sizeof(int32_t) && value >= 0 && value <= 255)
        return option_bytes(uint8_t(value));
    return option_bytes(int32_t(value));
}
} // namespace operit::arm64
