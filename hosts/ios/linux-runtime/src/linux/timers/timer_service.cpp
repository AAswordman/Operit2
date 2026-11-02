#include "linux/timers/timer_service.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/timers/host_events.h"
#include "linux/timers/ready_pipe.h"
#include "linux/timers/timer_state.h"
#include <algorithm>
#include <cerrno>
#include <climits>
#include <ctime>
#include <map>
#include <mutex>
#include <optional>
#include <system_error>
#include <thread>

namespace operit::arm64 {
namespace {
constexpr int64_t second = 1000000000;
int64_t nanoseconds(const TimerTime &time) {
    if (time.seconds < 0 || time.nanoseconds < 0 || time.nanoseconds >= second)
        guest_error(22);
    __int128 value = __int128(time.seconds) * second + time.nanoseconds;
    return value > INT64_MAX ? INT64_MAX : int64_t(value);
}
TimerTime split(int64_t value) {
    return {value / second, value % second};
}
int64_t clock_now(bool realtime) {
    timespec value{};
    check_host(::clock_gettime(realtime ? CLOCK_REALTIME : CLOCK_MONOTONIC, &value));
    __int128 total = __int128(value.tv_sec) * second + value.tv_nsec;
    return total < 0 ? 0 : total > INT64_MAX ? INT64_MAX : int64_t(total);
}
int64_t add_time(int64_t base, __int128 increment) {
    __int128 result = base + increment;
    return result > INT64_MAX ? INT64_MAX : int64_t(result);
}
void readiness(timers::State &state, bool ready) {
    if (state.ready == ready)
        return;
    char byte = 1;
    ssize_t result;
    do {
        result = ready ? ::write(state.writer.get(), &byte, 1) : ::read(state.reader.get(), &byte, 1);
    } while (result < 0 && errno == EINTR);
    if (result != 1)
        guest_error(result < 0 ? linux_errno(errno) : 5);
    state.ready = ready;
}
void update_readiness(timers::State &state) {
    readiness(state, state.expirations || state.cancelled || state.error);
}
void increment(timers::State &state, uint64_t count) {
    state.expirations += std::min<uint64_t>(count, UINT64_MAX - state.expirations);
}
void expire(timers::State &state, int64_t now) {
    if (!state.active || now < state.deadline)
        return;
    state.active = false;
    state.expired = true;
    increment(state, 1);
    update_readiness(state);
}
void restart(timers::State &state, int64_t now) {
    if (!state.expired || !state.interval)
        return;
    // A pending periodic timer is advanced only on access, avoiding wakeups for
    // every interval while its expiration count remains unread.
    uint64_t periods = now < state.deadline ? 0 : uint64_t((now - state.deadline) / state.interval) + 1;
    if (periods)
        increment(state, periods - 1);
    state.deadline = add_time(state.deadline, __int128(periods) * state.interval);
    state.expired = false;
    state.active = state.deadline > now;
}
TimerSpec snapshot(const timers::State &state, int64_t now) {
    int64_t remaining = state.active && state.deadline > now ? state.deadline - now : 0;
    return {split(state.interval), split(remaining)};
}
} // namespace
struct TimerService::Impl {
    std::mutex mutex;
    std::map<timers::State *, std::weak_ptr<timers::State>> timers;
    timers::HostEvents events;
    std::thread worker;
    bool stopping = false;
    int error = 0;

    void check() const {
        if (error)
            guest_error(error);
    }
    void refresh_clock() {
        const bool changed = events.clock_set();
        for (auto it = timers.begin(); it != timers.end();) {
            auto state = it->second.lock();
            if (!state) {
                it = timers.erase(it);
                continue;
            }
            ++it;
            if (!state->cancel_on_set)
                continue;
            if (changed)
                state->cancelled = true;
            if (!events.can_cancel_on_set())
                state->error = 95;
            update_readiness(*state);
        }
    }
    void fail(int number) noexcept {
        std::lock_guard<std::mutex> lock(mutex);
        error = number;
        for (auto &item : timers)
            if (auto state = item.second.lock()) {
                state->error = number;
                if (!state->ready) {
                    const char byte = 1;
                    ssize_t result;
                    do {
                        result = ::write(state->writer.get(), &byte, 1);
                    } while (result < 0 && errno == EINTR);
                    state->ready = result == 1;
                }
            }
    }
    void run() noexcept {
        try {
            std::unique_lock<std::mutex> lock(mutex);
            while (!stopping) {
                refresh_clock();
                const int64_t mono = clock_now(false), wall = clock_now(true);
                std::optional<int64_t> delay;
                bool wall_timer = false;
                for (const auto &item : timers) {
                    auto state = item.second.lock();
                    if (!state)
                        continue;
                    const int64_t now = state->wall_clock ? wall : mono;
                    expire(*state, now);
                    if (!state->active || state->ready)
                        continue;
                    const int64_t remaining = state->deadline - now;
                    if (!delay || remaining < *delay)
                        delay = remaining;
                    wall_timer |= state->wall_clock;
                }
                if (wall_timer && !events.can_cancel_on_set() && (!delay || *delay > 100000000))
                    delay = 100000000;
                lock.unlock();
                events.wait(delay);
                lock.lock();
            }
        } catch (const GuestError &failure) {
            fail(failure.number);
        } catch (...) {
            fail(5);
        }
    }
};
TimerService::TimerService() : impl_(std::make_unique<Impl>()) {
    try {
        impl_->worker = std::thread([state = impl_.get()] { state->run(); });
    } catch (const std::system_error &) {
        guest_error(11);
    }
}
TimerService::~TimerService() {
    {
        std::lock_guard<std::mutex> lock(impl_->mutex);
        impl_->stopping = true;
    }
    impl_->events.wake();
    impl_->worker.join();
}
std::shared_ptr<TimerService> TimerService::acquire() {
    static std::mutex mutex;
    static std::weak_ptr<TimerService> service;
    std::lock_guard<std::mutex> lock(mutex);
    auto result = service.lock();
    if (!result) {
        result = std::shared_ptr<TimerService>(new TimerService());
        service = result;
    }
    return result;
}
std::shared_ptr<timers::State> TimerService::create(int32_t clock_id) {
    auto pipe = timers::ready_pipe();
    auto state = std::make_shared<timers::State>(std::move(pipe.first), std::move(pipe.second), clock_id);
    {
        std::lock_guard<std::mutex> lock(impl_->mutex);
        impl_->check();
        impl_->timers.emplace(state.get(), state);
    }
    impl_->events.wake();
    return state;
}
void TimerService::remove(const std::shared_ptr<timers::State> &state) {
    {
        std::lock_guard<std::mutex> lock(impl_->mutex);
        impl_->timers.erase(state.get());
    }
    impl_->events.wake();
}
TimerSpec TimerService::set_time(timers::State &state, uint32_t flags, const TimerSpec &value) {
    if (flags & ~(TimerFile::absolute | TimerFile::cancel_on_set))
        guest_error(22);
    const int64_t interval = nanoseconds(value.interval), expiration = nanoseconds(value.value);
    const bool wall_clock = state.clock_id == 0 && (flags & TimerFile::absolute);
    const bool cancellable = wall_clock && (flags & TimerFile::cancel_on_set);
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->check();
    impl_->refresh_clock();
    if (cancellable && !impl_->events.can_cancel_on_set())
        guest_error(95);
    const int64_t old_now = clock_now(state.wall_clock);
    expire(state, old_now);
    restart(state, old_now);
    const TimerSpec previous = snapshot(state, old_now);
    const bool cancelled = state.cancelled && cancellable && expiration;
    const int64_t now = clock_now(wall_clock);
    state.wall_clock = wall_clock;
    state.interval = interval;
    state.deadline = flags & TimerFile::absolute ? expiration : add_time(now, expiration);
    state.expirations = 0;
    state.expired = false;
    state.cancelled = false;
    state.cancel_on_set = cancellable;
    state.error = 0;
    state.active = expiration != 0;
    update_readiness(state);
    expire(state, now);
    impl_->events.wake();
    // Linux installs the new timer even when rearming reports a pending clock
    // cancellation. A following settime must not report that same cancellation.
    if (cancelled)
        guest_error(125);
    return previous;
}
TimerSpec TimerService::get_time(timers::State &state) {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->check();
    impl_->refresh_clock();
    if (state.error)
        guest_error(state.error);
    const int64_t now = clock_now(state.wall_clock);
    expire(state, now);
    restart(state, now);
    impl_->events.wake();
    return snapshot(state, now);
}
uint64_t TimerService::read(timers::State &state) {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->check();
    impl_->refresh_clock();
    if (state.error)
        guest_error(state.error);
    const int64_t now = clock_now(state.wall_clock);
    expire(state, now);
    if (state.cancelled) {
        state.cancelled = false;
        state.expirations = 0;
        state.expired = false;
        update_readiness(state);
        impl_->events.wake();
        guest_error(125);
    }
    if (!state.expirations)
        guest_error(11);
    restart(state, now);
    const uint64_t result = state.expirations;
    state.expirations = 0;
    state.expired = false;
    update_readiness(state);
    impl_->events.wake();
    return result;
}
} // namespace operit::arm64
