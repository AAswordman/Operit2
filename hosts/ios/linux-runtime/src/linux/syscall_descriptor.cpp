#include "linux/error_mapping.h"
#include "linux/events/inotify.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/memory_file.h"
#include "linux/files/pipe_buffer.h"
#include "linux/files/record_lock.h"
#include "linux/syscalls.h"
#include <fcntl.h>
#include <sys/ioctl.h>

namespace operit::arm64 {
int64_t Syscalls::descriptor_control(CpuState &cpu) {
    const auto &a = cpu.x;
    if (a[8] == 279) // memfd_create; central dispatch case 279 must route here.
        return files::memory_file_create(memory_, descriptors_, a[0], uint32_t(a[1]));
    auto &entry = descriptors_.get(uint32_t(a[0]));
    files::check_path_fcntl(entry, a[1]);
    auto &description = *entry.description;
    switch (a[1]) {
    case 0:    // F_DUPFD
    case 1030: // F_DUPFD_CLOEXEC
        return descriptors_.duplicate_minimum(uint32_t(a[0]), a[2], a[1] == 1030);
    case 1: // F_GETFD: descriptor flags are not shared by dup.
        return entry.descriptor_flags;
    case 2: // F_SETFD
        entry.descriptor_flags = a[2] & 1;
        return 0;
    case 3: // F_GETFL: status flags belong to the open file description.
        return description.flags;
    case 4: {                                      // F_SETFL
        constexpr int mutable_flags = 1024 | 2048; // O_APPEND, O_NONBLOCK
        if (a[2] & (8192 | 65536 | 262144))        // O_ASYNC, O_DIRECT, O_NOATIME
            guest_error(22);
        // Inotify's native pipe is a permanently nonblocking marker. Its
        // Linux status flags live only in the shared open description.
        if (!description.counter && !description.poll && !description.inotify &&
            !description.virtual_file) {
            auto seal_guard = files::memory_file_lock(entry.host.get());
            int flags = ::fcntl(entry.host.get(), F_GETFL);
            check_host(flags);
            flags = (flags & ~(O_APPEND | O_NONBLOCK)) | ((a[2] & 1024) ? O_APPEND : 0) |
                    ((description.scheduler_io || (a[2] & 2048)) ? O_NONBLOCK : 0);
            check_host(::fcntl(entry.host.get(), F_SETFL, flags));
        }
        description.flags = (description.flags & ~mutable_flags) | (int(a[2]) & mutable_flags);
        return 0;
    }
    case 1032: // F_GETPIPE_SZ
        // Bash uses this value before writing a here-document into an empty
        // pipe. Its fallback can exceed our native-backed pipe's capacity.
        if (!description.pipe)
            guest_error(9);
        return description.pipe->capacity();
    case 1033: // F_ADD_SEALS
        files::memory_file_add_seals(entry, uint32_t(a[2]));
        return 0;
    case 1034: // F_GET_SEALS
        return files::memory_file_get_seals(entry);
    case 5:  // F_GETLK
    case 6:  // F_SETLK
    case 7:  // F_SETLKW
    case 36: // F_OFD_GETLK
    case 37: // F_OFD_SETLK
    case 38: // F_OFD_SETLKW
        if (ipc::is_posix_mq_file(entry.description))
            guest_error(95);
        return files::record_lock(memory_, filesystem_, scheduler_, entry, a[1], a[2]);
    default:
        return unsupported(cpu);
    }
}
int64_t Syscalls::descriptor_ioctl(CpuState &cpu) {
    const auto &a = cpu.x;
    auto &entry = descriptors_.get(uint32_t(a[0]));
    files::require_data_descriptor(entry);
    auto &description = *entry.description;
    // Translate the Linux request and payload explicitly. Darwin request numbers
    // and termios layouts are different and must never cross this boundary.
    switch (uint32_t(a[1])) {
    case 0x5421: { // Linux FIONBIO
        check_buffer(a[2], 4, read_perm);
        bool enabled = memory_.load(a[2], 4) != 0;
        // Inotify's native pipe is a permanently nonblocking marker. Its
        // Linux status flags live only in the shared open description.
        if (!description.counter && !description.poll && !description.inotify &&
            !description.virtual_file) {
            auto seal_guard = files::memory_file_lock(entry.host.get());
            int flags = ::fcntl(entry.host.get(), F_GETFL);
            check_host(flags);
            check_host(
                ::fcntl(entry.host.get(), F_SETFL,
                        (description.scheduler_io || enabled) ? flags | O_NONBLOCK : flags & ~O_NONBLOCK));
        }
        description.flags = (description.flags & ~2048) | (enabled ? 2048 : 0);
        return 0;
    }
    case 0x541b: { // Linux FIONREAD / TIOCINQ
        check_buffer(a[2], 4, write_perm);
        if (description.inotify) {
            const uint32_t available = description.inotify->available();
            memory_.store(a[2], available, 4);
            return 0;
        }
        if (description.counter || description.poll || description.virtual_file)
            guest_error(25);
        int available = 0;
        if (description.socket && description.socket->type == 2) {
            std::vector<uint8_t> packet(max_io);
            ssize_t count = ::recv(entry.host.get(), packet.data(), packet.size(), MSG_PEEK | MSG_DONTWAIT);
            if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK)
                guest_error(linux_errno(errno));
            available = count < 0 ? 0 : count;
        } else
            check_host(::ioctl(entry.host.get(), FIONREAD, &available));
        memory_.store(a[2], uint32_t(available), 4);
        return 0;
    }
    case 0x5450: // Linux FIONCLEX
    case 0x5451: // Linux FIOCLEX
        entry.descriptor_flags = uint32_t(a[1]) == 0x5451;
        return 0;
    default:
        return terminal_ioctl(cpu);
    }
}
} // namespace operit::arm64
