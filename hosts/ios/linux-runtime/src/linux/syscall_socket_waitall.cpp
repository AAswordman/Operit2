#include "linux/error_mapping.h"
#include "linux/network/message_batch.h"
#include "linux/syscalls.h"
#include <algorithm>
#include <poll.h>
#include <sys/uio.h>

namespace operit::arm64 {
int64_t Syscalls::receive_waitall(Descriptor &entry,
                                  const std::vector<std::pair<uint64_t, uint64_t>> &vectors,
                                  uint64_t guest_flags, std::function<void()> output) {
    using namespace socket_messages;
    auto state = entry.description->socket;
    if (!state)
        guest_error(88);
    size_t total = 0;
    for (const auto &vector : vectors) {
        if (vector.second > max_io - total)
            guest_error(90);
        total += vector.second;
        check_buffer(vector.first, vector.second, write_perm);
    }
    const int flags = socket_message_flags(uint32_t(guest_flags), false) & ~MSG_WAITALL;
    if (!total) {
        if (output)
            output();
        return 0;
    }
    if (int error = take_error(state))
        return -error;
    auto data = std::make_shared<std::vector<uint8_t>>(total);
    auto progress = std::make_shared<size_t>(0);
    const bool nonblocking = (entry.description->flags & 2048) || (guest_flags & 64);
    auto transfer = [this, data, progress, vectors, flags, guest_flags, nonblocking, output,
                     state](int host) -> std::optional<int64_t> {
        try {
            for (const auto &vector : vectors)
                check_buffer(vector.first, vector.second, write_perm);
            const bool peek = guest_flags & 2;
            const size_t position = peek ? 0 : *progress;
            const size_t capacity = credential_receive_limit(state, data->size() - position);
            iovec vector{data->data() + position, capacity};
            msghdr native{};
            native.msg_iov = &vector;
            native.msg_iovlen = 1;
            ReceivedControl control;
            native.msg_control = control.data();
            native.msg_controllen = control.capacity();
            const ssize_t count = ::recvmsg(host, &native, flags);
            if (count < 0) {
                if (errno == EAGAIN || errno == EWOULDBLOCK) {
                    if (nonblocking && *progress)
                        return *progress;
                    return {};
                }
                return *progress ? int64_t(*progress) : -socket_errno(errno);
            }
            control.received(native.msg_controllen);
            // recv/read discard ancillary data. Keep ownership of every host FD
            // until this attempt exits, and still stop at the ancillary barrier.
            const bool credentials = pass_credentials(state) && received_credentials(state).has_value();
            auto originals = state->received(count, peek);
            const size_t completed = position + size_t(count);
            size_t offset = 0;
            for (const auto &item : vectors) {
                const size_t size = std::min<size_t>(item.second, completed - offset);
                if (size && !(guest_flags & 32))
                    memory_.copy_in(item.first, data->data() + offset, size);
                offset += size;
                if (offset == completed)
                    break;
            }
            *progress = completed;
            if (output)
                output();
            if (!count || *progress == data->size() || nonblocking || native.msg_controllen ||
                !originals.empty() || credentials || (native.msg_flags & (MSG_CTRUNC | MSG_OOB | MSG_EOR | MSG_TRUNC)))
                return *progress;
            if (peek) {
                pollfd event{host, POLLIN, 0};
                if (::poll(&event, 1, 0) < 0)
                    return *progress ? int64_t(*progress) : -socket_errno(errno);
                if (event.revents & POLLHUP)
                    return *progress;
            }
            return {};
        } catch (const GuestError &) {
            if (*progress)
                return *progress;
            throw;
        } catch (const MemoryFault &) {
            if (*progress)
                return *progress;
            throw;
        }
    };
    const int64_t result = socket_wait(entry, false, nonblocking, transfer,
                                      [progress] { return *progress ? int64_t(*progress) : -11; });
    if (scheduler_.current().state == Task::State::Waiting) {
        arm_message_wait(scheduler_.current(), [progress] { return *progress ? int64_t(*progress) : -4; });
        return result;
    }
    return result < 0 && *progress ? int64_t(*progress) : result;
}
} // namespace operit::arm64
