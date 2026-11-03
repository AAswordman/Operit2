#include "linux/process/limit_enforcement.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "runtime/process/controller.h"
#include "runtime/tasks/scheduler.h"
#include <cerrno>
#include <exception>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <sys/stat.h>
#include <unistd.h>
#include <utility>

namespace operit::arm64::process {
struct UidLimitState {
    std::mutex mutex;
    std::map<uint32_t, UidLimitUsage> users;
};
UidLimitCharge::UidLimitCharge(std::shared_ptr<UidLimitState> state, uint32_t uid, Kind kind)
    : state_(std::move(state)), uid_(uid), kind_(kind) {}
UidLimitCharge::UidLimitCharge(UidLimitCharge &&other) noexcept
    : state_(std::move(other.state_)), uid_(other.uid_), kind_(other.kind_) {}
UidLimitCharge &UidLimitCharge::operator=(UidLimitCharge &&other) noexcept {
    if (this != &other) {
        reset();
        state_ = std::move(other.state_);
        uid_ = other.uid_;
        kind_ = other.kind_;
    }
    return *this;
}
UidLimitCharge::~UidLimitCharge() { reset(); }
void UidLimitCharge::reset() noexcept {
    auto state = std::move(state_);
    if (!state)
        return;
    std::lock_guard<std::mutex> lock(state->mutex);
    const auto found = state->users.find(uid_);
    if (found == state->users.end())
        std::terminate();
    auto &count = kind_ == Kind::Task ? found->second.tasks : found->second.pending_signals;
    if (!count)
        std::terminate();
    --count;
    if (!found->second.tasks && !found->second.pending_signals)
        state->users.erase(found);
}
UidLimitAccounting::UidLimitAccounting() : state_(std::make_shared<UidLimitState>()) {}
UidLimitUsage UidLimitAccounting::usage(uint32_t uid) const {
    std::lock_guard<std::mutex> lock(state_->mutex);
    const auto found = state_->users.find(uid);
    return found == state_->users.end() ? UidLimitUsage{} : found->second;
}
UidLimitCharge UidLimitAccounting::reserve(uint32_t uid, UidLimitCharge::Kind kind,
                                          uint64_t limit, bool override_limit) const {
    std::lock_guard<std::mutex> lock(state_->mutex);
    auto found = state_->users.find(uid);
    const uint64_t current = found == state_->users.end() ? 0 :
        (kind == UidLimitCharge::Kind::Task ? found->second.tasks : found->second.pending_signals);
    if (current == UINT64_MAX || (!override_limit && current >= limit))
        guest_error(11);
    if (found == state_->users.end())
        found = state_->users.emplace(uid, UidLimitUsage{}).first;
    auto &count = kind == UidLimitCharge::Kind::Task ? found->second.tasks : found->second.pending_signals;
    ++count;
    return UidLimitCharge(state_, uid, kind);
}
UidLimitCharge UidLimitAccounting::track_task(uint32_t uid) const {
    return reserve(uid, UidLimitCharge::Kind::Task, resource_infinity, true);
}
UidLimitCharge UidLimitAccounting::reserve_task(uint32_t uid, const ResourceLimits &limits,
                                             bool privileged) const {
    return reserve(uid, UidLimitCharge::Kind::Task, limits.get(Resource::Processes).soft,
                   uid == 0 || privileged);
}
void UidLimitAccounting::reassign_task(UidLimitCharge &charge, uint32_t uid) const {
    if (charge.state_ != state_ || charge.kind_ != UidLimitCharge::Kind::Task)
        throw Failure("task resource charge belongs to another accounting domain");
    if (charge.uid_ == uid)
        return;
    std::lock_guard<std::mutex> lock(state_->mutex);
    const auto old = state_->users.find(charge.uid_);
    if (old == state_->users.end() || !old->second.tasks)
        throw Failure("missing task resource charge");
    auto next = state_->users.find(uid);
    if (next != state_->users.end() && next->second.tasks == UINT64_MAX)
        guest_error(11);
    if (next == state_->users.end())
        next = state_->users.emplace(uid, UidLimitUsage{}).first;
    ++next->second.tasks;
    --old->second.tasks;
    if (!old->second.tasks && !old->second.pending_signals)
        state_->users.erase(old);
    charge.uid_ = uid;
}
bool UidLimitAccounting::task_limit_exceeded(uint32_t uid, const ResourceLimits &limits,
                                             bool privileged) const {
    return uid != 0 && !privileged && usage(uid).tasks > limits.get(Resource::Processes).soft;
}
UidLimitCharge UidLimitAccounting::reserve_signal(uint32_t uid, const ResourceLimits &limits,
                                                bool override_limit) const {
    return reserve(uid, UidLimitCharge::Kind::Signal, limits.get(Resource::PendingSignals).soft,
                   override_limit);
}
int send_limit_signal(Scheduler &scheduler, uint32_t tid, int signal, bool process_wide) {
    auto *task = scheduler.lookup(tid);
    auto *controller = scheduler.processes();
    if (!controller || !task || task->state == Task::State::Exited)
        return -3;
    try {
        auto hooks = controller->signal_hooks();
        auto target = hooks.process(task->pid);
        const auto info = signals::Info::user(signal, 0, 0, 128); // SI_KERNEL
        if (process_wide)
            return signals::send_process(hooks, target, info);
        for (const auto &thread : target.threads)
            if (thread.tid == tid)
                return signals::send_thread(hooks, target, thread, info);
        return -3;
    } catch (const GuestError &error) {
        return -error.number;
    }
}
int enforce_cpu_limit(Scheduler &scheduler, uint32_t tid, ResourceLimits &limits,
                      uint64_t user_nanoseconds, uint64_t system_nanoseconds) {
    const int signal = limits.check_cpu_time(user_nanoseconds, system_nanoseconds);
    if (!signal)
        return 0;
    const int status = send_limit_signal(scheduler, tid, signal, true);
    return status < 0 ? status : signal;
}
int enforce_realtime_limit(Scheduler &scheduler, uint32_t tid, ResourceLimits &limits,
                           uint64_t runtime_nanoseconds) {
    const int signal = limits.check_realtime_time(runtime_nanoseconds);
    if (!signal)
        return 0;
    const int status = send_limit_signal(scheduler, tid, signal, true);
    return status < 0 ? status : signal;
}
files::MemoryFileGuard guard_file_write(const ResourceLimits &limits, int host_fd, uint64_t count,
                                        std::optional<uint64_t> offset, std::optional<bool> append,
                                        const std::function<void()> &file_size_signal) {
    try {
        auto position_guard = files::memory_file_lock(host_fd);
        if (count) {
            struct stat info{};
            check_host(::fstat(host_fd, &info));
            if (S_ISREG(info.st_mode)) {
                const int flags = ::fcntl(host_fd, F_GETFL);
                check_host(flags);
                uint64_t at;
                if (append.value_or(bool(flags & O_APPEND))) {
                    if (info.st_size < 0)
                        guest_error(22);
                    at = uint64_t(info.st_size);
                } else if (offset) {
                    if (*offset > uint64_t(INT64_MAX))
                        guest_error(22);
                    at = *offset;
                } else {
                    const off_t position = ::lseek(host_fd, 0, SEEK_CUR);
                    if (position < 0)
                        guest_error(linux_errno(errno));
                    at = uint64_t(position);
                }
                count = limits.file_write_count(at, count);
            }
        }
        return files::memory_file_check_write(host_fd, count, offset, append);
    } catch (const GuestError &error) {
        // All native-file locks have unwound before touching signal/scheduler
        // state. A successful prefix never queues SIGXFSZ.
        if (error.number == 27 && file_size_signal)
            file_size_signal();
        throw;
    }
}
} // namespace operit::arm64::process
