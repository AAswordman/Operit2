#include "linux/error_mapping.h"
#include "linux/events/epoll.h"
#include "linux/events/event_counter.h"
#include "linux/events/inotify.h"
#include "linux/files/pipe_buffer.h"
#include "linux/signals/signal_file.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <fcntl.h>
#include <sys/stat.h>
namespace operit::arm64 {
int64_t Syscalls::dispatch_events(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (a[8]) {
    case 19: { // eventfd2: shared counter and native readiness pipe
        if (a[0] > UINT32_MAX || (a[1] & ~uint64_t{1 | 2048 | 524288}))
            guest_error(22);
        int pipe[2];
        check_host(::pipe(pipe));
        FD reader(pipe[0]), writer(pipe[1]);
        check_host(::fcntl(reader.get(), F_SETFD, FD_CLOEXEC));
        check_host(::fcntl(writer.get(), F_SETFD, FD_CLOEXEC));
        check_host(::fcntl(reader.get(), F_SETFL, O_NONBLOCK));
        check_host(::fcntl(writer.get(), F_SETFL, O_NONBLOCK));
        auto counter = std::make_shared<EventCounter>(std::move(writer), a[0], a[1] & 1);
        int fd = descriptors_.insert(std::move(reader), "", 2 | int(a[1] & (2048 | 524288)));
        descriptors_.get(fd).description->counter = std::move(counter);
        descriptors_.get(fd).description->scheduler_io = true;
        return fd;
    }
    case 20: { // epoll_create1
        if (a[0] & ~uint64_t{524288})
            guest_error(22);
        auto poll = std::make_shared<EventPoll>();
        int fd = descriptors_.insert(FD(), "", int(a[0]));
        descriptors_.get(fd).description->poll = std::move(poll);
        return fd;
    }
    case 21: { // epoll_ctl: arm64 epoll_event has 8-byte aligned data (16-byte struct)
        auto &poll = descriptors_.get(uint32_t(a[0]));
        if (!poll.description->poll)
            guest_error(22);
        if (a[0] == a[2])
            guest_error(22);
        auto &target = descriptors_.get(uint32_t(a[2]));
        uint32_t events = 0;
        uint64_t data = 0;
        if (a[1] != 2) {
            check_buffer(a[3], 16, read_perm);
            events = uint32_t(memory_.load(a[3], 4));
            data = memory_.load(a[3] + 8, 8);
        }
        if (target.description->virtual_file && !ipc::is_posix_mq_file(target.description))
            guest_error(1);
        int native = target.host.get();
        if (target.description->poll)
            native = target.description->poll->native_handle();
        else {
            struct stat info{};
            check_host(::fstat(native, &info));
            if (S_ISREG(info.st_mode) || S_ISDIR(info.st_mode))
                guest_error(1);
        }
        poll.description->poll->control(int(a[1]), int(a[2]), native, target.description, events, data);
        return 0;
    }
    case 22:  // epoll_pwait
    case 441: { // epoll_pwait2: keep timespec precision in the guest scheduler.
        auto description = descriptors_.get(uint32_t(a[0])).description;
        if (!description->poll)
            guest_error(22);
        if (!a[2] || a[2] > 4096)
            guest_error(22);
        auto &task = scheduler_.current();
        if (a[4]) {
            if (a[5] != 8)
                guest_error(22);
            check_buffer(a[4], 8, read_perm);
        }
        check_buffer(a[1], a[2] * 16, write_perm);
        // Validate the complete timeout before polling or installing the mask.
        // A null pwait2 timeout waits indefinitely; its timespec is input-only.
        TaskDeadline deadline;
        if (a[8] == 441)
            deadline = thread_deadline(a[3], false, false);
        else {
            const int timeout = int32_t(a[3]);
            if (timeout >= 0)
                deadline = TaskClock::now() + std::chrono::milliseconds(timeout);
        }
        auto read_events = [this, description, address = a[1],
                            maximum = int(a[2])]() -> std::optional<int64_t> {
            // This callback also runs for scheduler retries in the waiting task's
            // scope, after epoll_pwait has installed that task's temporary mask.
            auto hooks = scheduler_.processes() ? scheduler_.processes()->signal_hooks() : signals::Hooks{};
            InotifyFile::service_all();
            signals::signal_files_safe_point(descriptors_, hooks);
            auto events = description->poll->wait(maximum, 0);
            if (events.empty())
                return {};
            check_buffer(address, maximum * 16, write_perm);
            for (size_t j = 0; j < events.size(); ++j) {
                memory_.store(address + j * 16, events[j].events, 4);
                memory_.store(address + j * 16 + 4, 0, 4);
                memory_.store(address + j * 16 + 8, events[j].data, 8);
            }
            return events.size();
        };
        if (a[4])
            signals::begin_temporary_mask(task.signals, task.signal_mask, memory_.load(a[4], 8));
        try {
            if (auto count = read_events()) {
                signals::end_temporary_mask(task.signals, task.signal_mask);
                return *count;
            }
            if (deadline && *deadline <= TaskClock::now()) {
                signals::end_temporary_mask(task.signals, task.signal_mask);
                return 0;
            }
            if (interrupt_for_pending_signal()) {
                signals::end_temporary_mask(task.signals, task.signal_mask);
                return -4;
            }
            scheduler_.block(deadline, 0, std::move(read_events));
            task.wait_cleanup = [this, tid = task.tid] {
                auto &waiting = scheduler_.find(tid);
                signals::end_temporary_mask(waiting.signals, waiting.signal_mask);
            };
        } catch (...) {
            signals::end_temporary_mask(task.signals, task.signal_mask);
            throw;
        }
        return 0;
    }
    case 59: { // pipe2
        if (a[1] & ~uint64_t{2048 | 524288})
            guest_error(22);
        check_buffer(a[0], 8, write_perm);
        int pipe[2];
        check_host(::pipe(pipe));
        FD reader(pipe[0]), writer(pipe[1]);
        check_host(::fcntl(reader.get(), F_SETFD, FD_CLOEXEC));
        check_host(::fcntl(writer.get(), F_SETFD, FD_CLOEXEC));
        // Host pipes stay nonblocking; the guest scheduler implements blocking.
        check_host(::fcntl(reader.get(), F_SETFL, O_NONBLOCK));
        check_host(::fcntl(writer.get(), F_SETFL, O_NONBLOCK));
        auto buffer = files::PipeBuffer::create(reader.get(), writer.get());
        int in = descriptors_.insert(std::move(reader), "", int(a[1]));
        descriptors_.get(in).description->scheduler_io = true;
        descriptors_.get(in).description->pipe = buffer;
        int out;
        try {
            out = descriptors_.insert(std::move(writer), "", int(a[1]) | 1);
            descriptors_.get(out).description->scheduler_io = true;
            descriptors_.get(out).description->pipe = std::move(buffer);
        } catch (...) {
            descriptors_.close(in);
            throw;
        }
        memory_.store(a[0], in, 4);
        memory_.store(a[0] + 4, out, 4);
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
