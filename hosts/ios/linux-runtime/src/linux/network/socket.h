#pragma once
#include "base/fd.h"
#include <chrono>
#include <cstdint>
#include <deque>
#include <memory>
#include <optional>
#include <string>
#include <sys/socket.h>
#include <vector>

namespace operit::arm64 {
struct Descriptor;
struct OpenDescription;
struct SocketCredentials {
    int32_t pid = 0;
    uint32_t uid = UINT32_MAX, gid = UINT32_MAX;
};
static_assert(sizeof(SocketCredentials) == 12);
struct SocketState {
    std::weak_ptr<SocketState> peer;
    struct PendingConnection {
        std::string peer_path;
        std::shared_ptr<SocketState> socket;
    };
    std::deque<PendingConnection> pending_connections;
    std::weak_ptr<SocketState> listener;
    SocketCredentials credentials;
    std::optional<SocketCredentials> peer_credentials;
    struct Rights {
        uint64_t position;
        std::vector<std::shared_ptr<Descriptor>> descriptors;
    };
    std::deque<Rights> pending_rights;
    // A queued copy of this receiving socket must not keep its own open
    // description alive after the last externally reachable FD is closed.
    std::weak_ptr<OpenDescription> self_description;
    uint64_t sent_position = 0, received_position = 0;
    void sent(size_t count, const std::vector<std::shared_ptr<Descriptor>> &rights = {});
    std::vector<std::shared_ptr<Descriptor>> received(size_t count, bool peek);
    void abandon_connection();
    int domain, type, protocol;
    std::string abstract_path;
    std::string bound_path;
    bool listening = false;
    ~SocketState();
    uint64_t receive_timeout_us = 0, send_timeout_us = 0;
    bool receive_timeout_immediate = false, send_timeout_immediate = false;
    bool connecting = false;
    std::vector<uint8_t> local_name, peer_name;
    SocketState(int family, int kind, int proto) : domain(family), type(kind), protocol(proto) {}
};
struct NativeAddress {
    sockaddr_storage storage{};
    socklen_t length = 0;
};
int socket_errno(int native_error);
int socket_family(int guest);
int guest_socket_family(int native);
int socket_type(int guest);
int socket_message_flags(uint64_t guest, bool sending);
NativeAddress socket_address(const std::vector<uint8_t> &guest);
std::vector<uint8_t> guest_socket_address(const sockaddr *native, socklen_t length);
void configure_socket(int fd);
struct SocketOption {
    int level, name;
};
SocketOption socket_option(int guest_level, int guest_name);
void set_socket_option(int fd, const std::shared_ptr<SocketState> &state, int guest_level, int guest_name,
                       const std::vector<uint8_t> &value);
std::vector<uint8_t> get_socket_option(int fd, const std::shared_ptr<SocketState> &state,
                                     int guest_level, int guest_name,
                                     size_t capacity);
} // namespace operit::arm64
