#pragma once
#include "base/fd.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/timers/ready_pipe.h"
#include <atomic>
#include <cerrno>
#include <cstdint>
#include <fcntl.h>
#include <optional>
#include <pthread.h>
#include <sys/file.h>

namespace operit::arm64::files {
// Only a raw native lock syscall runs on this worker; guest state stays on its coordinator.
class NativeLockWait {
    std::shared_ptr<FD> descriptor_;
    FD reader_, writer_, notify_;
    pthread_t thread_{};
    bool started_ = false, whole_file_;
    int operation_;
    struct flock range_{};
    std::atomic<int64_t> result_{INT64_MIN};

    // Cancellation is enabled solely around the kernel wait, without C++ locks or owned locals.
    static void *execute(void *context) {
        auto &wait = *static_cast<NativeLockWait *>(context);
        int previous;
        ::pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &previous);
        ::pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS, &previous);
        ::pthread_setcancelstate(PTHREAD_CANCEL_ENABLE, &previous);
        const int status = wait.whole_file_ ? ::flock(wait.descriptor_->get(), wait.operation_) :
            ::fcntl(wait.descriptor_->get(), F_OFD_SETLKW, &wait.range_);
        const int error = errno;
        ::pthread_setcancelstate(PTHREAD_CANCEL_DISABLE, &previous);
        wait.result_.store(status == 0 ? 0 : -linux_errno(error), std::memory_order_release);
        const char byte = 1;
        for (int fd : {wait.writer_.get(), wait.notify_.get()})
            if (fd >= 0) while (::write(fd, &byte, 1) < 0 && errno == EINTR) {}
        return nullptr;
    }

  public:
    // A separate readiness marker survives until the coordinator consumes or cancels the request.
    NativeLockWait(std::shared_ptr<FD> descriptor, bool whole_file, int operation,
                   struct flock range = {}, int notify = -1)
        : descriptor_(std::move(descriptor)), whole_file_(whole_file), operation_(operation), range_(range) {
        auto pipe = timers::ready_pipe();
        reader_ = std::move(pipe.first);
        writer_ = std::move(pipe.second);
        if (notify >= 0) {
            notify_ = FD(::fcntl(notify, F_DUPFD_CLOEXEC, 0));
            check_host(notify_.get());
        }
        const int error = ::pthread_create(&thread_, nullptr, execute, this);
        if (error) guest_error(linux_errno(error));
        started_ = true;
    }
    // Join before the owner changes the original lock or closes the VM's descriptors.
    ~NativeLockWait() { stop(); }
    NativeLockWait(const NativeLockWait &) = delete;
    NativeLockWait &operator=(const NativeLockWait &) = delete;
    // The kernel wait is interruptible without a periodic retry thread.
    void stop() {
        if (!started_) return;
        ::pthread_cancel(thread_);
        ::pthread_join(thread_, nullptr);
        started_ = false;
    }
    // The consumer only observes a result after the native syscall has returned.
    std::optional<int64_t> result() const {
        const auto value = result_.load(std::memory_order_acquire);
        return value == INT64_MIN ? std::nullopt : std::optional<int64_t>(value);
    }
    // Borrowed native pipe for Scheduler's ordinary readiness registration.
    int native_handle() const { return reader_.get(); }
};
} // namespace operit::arm64::files
