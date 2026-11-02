#pragma once
#include "base/fd.h"
#include "linux/network/socket.h"
#include <cstddef>
#include <cstdint>
#include <functional>
#include <optional>
#include <vector>

namespace operit::arm64 { struct Task; }
namespace operit::arm64::socket_messages {
// Linux arm64 uses eight-byte cmsghdr alignment, including on Darwin hosts.
constexpr size_t control_header_size = 16;
constexpr size_t maximum_rights = 253;
constexpr size_t maximum_control = 65536;
constexpr uint64_t wait_for_one = 0x10000;
constexpr size_t control_align(size_t length) { return (length + 7) & ~size_t{7}; }
struct Header {
    uint64_t name;
    uint32_t name_size, padding;
    uint64_t vectors, vector_count, control, control_size;
    uint32_t flags, padding2;
};
struct MultiHeader {
    Header message;
    uint32_t length, padding;
};
static_assert(sizeof(Header) == 56 && sizeof(MultiHeader) == 64);
struct Credentials {
    int32_t pid;
    uint32_t uid, gid;
};
static_assert(sizeof(Credentials) == 12);

// SO_PASSCRED belongs to the guest socket, never to the host process. Socket
// option handlers can use these accessors without changing SocketState's ABI.
void pass_credentials(const std::shared_ptr<SocketState> &socket, bool enabled);
bool pass_credentials(const std::shared_ptr<SocketState> &socket);
// Call before SocketState::sent(), while sent_position still names this send.
void sent_credentials(const std::shared_ptr<SocketState> &socket, size_t count,
                      const Credentials &credentials);
// Call before SocketState::received(). Peeking leaves the record in place.
std::optional<Credentials> received_credentials(const std::shared_ptr<SocketState> &socket);
size_t credential_receive_limit(const std::shared_ptr<SocketState> &socket, size_t capacity);
// recvmmsg preserves a non-EAGAIN socket error occurring after partial success.
void defer_error(const std::shared_ptr<SocketState> &socket, int error);
int take_error(const std::shared_ptr<SocketState> &socket);

// The scheduler's signal interruption path can apply this result before wake().
// Ordinary completion and task retirement remove the callback via wait_cleanup.
void arm_message_wait(Task &task, std::function<int64_t()> interrupted);
void forget_message_wait(Task &task) noexcept;
std::optional<int64_t> interrupt_message_wait(Task &task);

// Own every descriptor in the returned native control buffer immediately after
// recvmsg, before any allocation, fcntl, guest-memory operation or queue lookup.
class ReceivedControl {
    std::vector<std::max_align_t> storage_;
    size_t used_ = 0;

  public:
    ReceivedControl();
    ~ReceivedControl();
    ReceivedControl(const ReceivedControl &) = delete;
    ReceivedControl &operator=(const ReceivedControl &) = delete;
    void *data() { return storage_.data(); }
    size_t capacity() const { return storage_.size() * sizeof(std::max_align_t); }
    void received(size_t length) noexcept;
    std::vector<FD> take_rights();
    bool has_other_control(bool translated_credentials = false) const;
};

class GuestControl {
    size_t capacity_;
    std::vector<uint8_t> bytes_;
    bool truncated_ = false;

  public:
    explicit GuestControl(size_t capacity) : capacity_(capacity) { bytes_.reserve(capacity); }
    size_t rights_capacity() const;
    void append(int type, const void *payload, size_t length);
    void truncate() { truncated_ = true; }
    bool truncated() const { return truncated_; }
    const std::vector<uint8_t> &bytes() const { return bytes_; }
};
} // namespace operit::arm64::socket_messages
