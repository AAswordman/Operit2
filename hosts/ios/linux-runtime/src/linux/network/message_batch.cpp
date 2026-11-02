#include "linux/network/message_batch.h"
#include "memory/fault.h"
#include "runtime/tasks/task.h"
#include <algorithm>
#include <cstring>
#include <deque>
#include <map>
#include <mutex>

namespace operit::arm64::socket_messages {
namespace {
struct CredentialRange {
    uint64_t begin, end;
    Credentials credentials;
};
struct State {
    std::weak_ptr<SocketState> socket;
    bool pass = false;
    int error = 0;
    std::deque<CredentialRange> credentials;
};
std::mutex states_mutex;
std::map<const SocketState *, State> states;
std::mutex waits_mutex;
std::map<const Task *, std::function<int64_t()>> waits;
State &state_for(const std::shared_ptr<SocketState> &socket) {
    for (auto it = states.begin(); it != states.end();) {
        if (it->second.socket.expired())
            it = states.erase(it);
        else
            ++it;
    }
    auto &state = states[socket.get()];
    state.socket = socket;
    while (!state.credentials.empty() && state.credentials.front().end <= socket->received_position)
        state.credentials.pop_front();
    return state;
}
bool equal(const Credentials &one, const Credentials &two) {
    return one.pid == two.pid && one.uid == two.uid && one.gid == two.gid;
}
template <typename Function> void controls(void *buffer, size_t size, Function function) {
    msghdr message{};
    message.msg_control = buffer;
    message.msg_controllen = size;
    auto *begin = static_cast<uint8_t *>(buffer);
    for (auto *item = CMSG_FIRSTHDR(&message); item;) {
        size_t offset = reinterpret_cast<uint8_t *>(item) - begin;
        if (offset > size || size - offset < sizeof(cmsghdr) || item->cmsg_len < CMSG_LEN(0) ||
            item->cmsg_len > size - offset)
            break;
        function(item);
        item = CMSG_NXTHDR(&message, item);
    }
}
} // namespace
void pass_credentials(const std::shared_ptr<SocketState> &socket, bool enabled) {
    std::lock_guard<std::mutex> lock(states_mutex);
    state_for(socket).pass = enabled;
}
bool pass_credentials(const std::shared_ptr<SocketState> &socket) {
    std::lock_guard<std::mutex> lock(states_mutex);
    return state_for(socket).pass;
}
void sent_credentials(const std::shared_ptr<SocketState> &socket, size_t count,
                      const Credentials &credentials) {
    if (socket->domain != 1 || (!count && socket->type == 1))
        return;
    auto peer = socket->peer.lock();
    if (!peer)
        return;
    std::lock_guard<std::mutex> lock(states_mutex);
    auto &ranges = state_for(peer).credentials;
    uint64_t begin = socket->sent_position, end = begin + (socket->type == 1 ? count : 1);
    if (socket->type == 1 && !ranges.empty() && ranges.back().end == begin &&
        equal(ranges.back().credentials, credentials))
        ranges.back().end = end;
    else
        ranges.push_back({begin, end, credentials});
}
std::optional<Credentials> received_credentials(const std::shared_ptr<SocketState> &socket) {
    std::lock_guard<std::mutex> lock(states_mutex);
    auto &state = state_for(socket);
    if (!state.pass || state.credentials.empty() ||
        state.credentials.front().begin > socket->received_position)
        return {};
    return state.credentials.front().credentials;
}
size_t credential_receive_limit(const std::shared_ptr<SocketState> &socket, size_t capacity) {
    if (socket->type != 1)
        return capacity;
    std::lock_guard<std::mutex> lock(states_mutex);
    auto &state = state_for(socket);
    if (!state.pass || state.credentials.empty())
        return capacity;
    const auto &range = state.credentials.front();
    uint64_t boundary = range.begin > socket->received_position ? range.begin : range.end;
    return std::min<uint64_t>(capacity, boundary - socket->received_position);
}
void defer_error(const std::shared_ptr<SocketState> &socket, int error) {
    std::lock_guard<std::mutex> lock(states_mutex);
    state_for(socket).error = error;
}
int take_error(const std::shared_ptr<SocketState> &socket) {
    std::lock_guard<std::mutex> lock(states_mutex);
    auto &state = state_for(socket);
    int error = state.error;
    state.error = 0;
    return error;
}
void arm_message_wait(Task &task, std::function<int64_t()> interrupted) {
    auto *key = &task;
    // Construct the cleanup closure before registering, so an allocation failure
    // cannot leave a callback pointing at a task which never went to sleep.
    std::function<void()> cleanup = [key, previous = task.wait_cleanup] {
        {
            std::lock_guard<std::mutex> lock(waits_mutex);
            waits.erase(key);
        }
        if (previous)
            previous();
    };
    {
        std::lock_guard<std::mutex> lock(waits_mutex);
        waits[key] = std::move(interrupted);
    }
    task.wait_cleanup = std::move(cleanup);
}
void forget_message_wait(Task &task) noexcept {
    std::lock_guard<std::mutex> lock(waits_mutex);
    waits.erase(&task);
}
std::optional<int64_t> interrupt_message_wait(Task &task) {
    if (task.state != Task::State::Waiting)
        return {};
    std::function<int64_t()> interrupted;
    {
        std::lock_guard<std::mutex> lock(waits_mutex);
        auto it = waits.find(&task);
        if (it == waits.end())
            return {};
        interrupted = std::move(it->second);
        waits.erase(it);
    }
    try {
        return interrupted();
    } catch (const GuestError &error) {
        return -error.number;
    } catch (const MemoryFault &) {
        return -14;
    }
}
ReceivedControl::ReceivedControl()
    : storage_((maximum_control + sizeof(std::max_align_t) - 1) / sizeof(std::max_align_t)) {}
void ReceivedControl::received(size_t length) noexcept { used_ = std::min(length, capacity()); }
ReceivedControl::~ReceivedControl() {
    controls(data(), used_, [](cmsghdr *item) {
        if (item->cmsg_level != SOL_SOCKET || item->cmsg_type != SCM_RIGHTS)
            return;
        size_t count = (item->cmsg_len - CMSG_LEN(0)) / sizeof(int);
        for (size_t i = 0; i < count; ++i) {
            int descriptor;
            std::memcpy(&descriptor, CMSG_DATA(item) + i * sizeof(int), sizeof(int));
            if (descriptor >= 0)
                ::close(descriptor);
        }
    });
}
std::vector<FD> ReceivedControl::take_rights() {
    size_t count = 0;
    controls(data(), used_, [&count](cmsghdr *item) {
        if (item->cmsg_level == SOL_SOCKET && item->cmsg_type == SCM_RIGHTS)
            count += (item->cmsg_len - CMSG_LEN(0)) / sizeof(int);
    });
    std::vector<FD> result;
    result.reserve(count);
    controls(data(), used_, [&result](cmsghdr *item) {
        if (item->cmsg_level != SOL_SOCKET || item->cmsg_type != SCM_RIGHTS)
            return;
        size_t count = (item->cmsg_len - CMSG_LEN(0)) / sizeof(int);
        for (size_t i = 0; i < count; ++i) {
            int descriptor, consumed = -1;
            auto *slot = CMSG_DATA(item) + i * sizeof(int);
            std::memcpy(&descriptor, slot, sizeof(int));
            result.emplace_back(descriptor);
            std::memcpy(slot, &consumed, sizeof(int));
        }
    });
    return result;
}
bool ReceivedControl::has_other_control(bool translated_credentials) const {
    bool other = false;
    controls(const_cast<std::max_align_t *>(storage_.data()), used_, [translated_credentials, &other](cmsghdr *item) {
        if (item->cmsg_level == SOL_SOCKET && item->cmsg_type == SCM_RIGHTS)
            return;
#ifdef SCM_CREDENTIALS
        if (translated_credentials && item->cmsg_level == SOL_SOCKET && item->cmsg_type == SCM_CREDENTIALS)
            return;
#else
        (void)translated_credentials;
#endif
        other = true;
    });
    return other;
}
size_t GuestControl::rights_capacity() const {
    size_t remaining = capacity_ - bytes_.size();
    return remaining < control_header_size ? 0 : (remaining - control_header_size) / sizeof(int32_t);
}
void GuestControl::append(int type, const void *payload, size_t length) {
    size_t available = capacity_ - bytes_.size();
    if (available < control_header_size) {
        truncated_ = true;
        return;
    }
    size_t complete = control_header_size + length;
    size_t used = std::min(complete, available);
    size_t offset = bytes_.size();
    // Linux consumes all remaining padding even if it is shorter than CMSG_SPACE.
    bytes_.resize(offset + std::min(control_align(complete), available), 0);
    uint64_t guest_length = used;
    int32_t level = 1;
    std::memcpy(bytes_.data() + offset, &guest_length, 8);
    std::memcpy(bytes_.data() + offset + 8, &level, 4);
    std::memcpy(bytes_.data() + offset + 12, &type, 4);
    if (used > control_header_size)
        std::memcpy(bytes_.data() + offset + control_header_size, payload, used - control_header_size);
    if (used != complete)
        truncated_ = true;
}
} // namespace operit::arm64::socket_messages
