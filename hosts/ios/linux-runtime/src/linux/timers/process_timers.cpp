#include "linux/timers/process_timers.h"
#include "linux/error_mapping.h"
#include "linux/process/limit_enforcement.h"
#include <algorithm>
#include <array>
#include <climits>
#include <cstring>
#include <map>
#include <time.h>

namespace operit::arm64 {
namespace {
constexpr uint64_t second = 1000000000;
constexpr uint64_t maximum_time = INT64_MAX;
constexpr int timer_signal_code = -2;
constexpr int kernel_signal_code = 128;
constexpr uint32_t maximum_timers = 65536;
constexpr uint32_t absolute_time = 1;

enum class ClockKind { Realtime, Monotonic, Cpu };
struct TimerClock {
    ClockKind kind = ClockKind::Monotonic;
    uint32_t id = 0;
    bool per_thread = false;
    unsigned sample = 2;
};
struct Setting {
    uint64_t interval = 0, value = 0;
};
struct Expiration {
    bool armed = false, absolute = false;
    uint64_t deadline = 0, interval = 0;
};
struct PosixTimer {
    TimerClock clock;
    Expiration expiry;
    std::shared_ptr<process::UidLimitCharge> signal_charge;
    int32_t id = 0, notify = 0, signal = 14;
    uint32_t target_tid = 0, pending_token = 0;
    uint64_t value = 0, generation = 0, waiting_expirations = 0;
    int overrun = 0;
};
struct Notification {
    int32_t timer_id = 0;
    uint64_t generation = 0;
    int overrun = 0;
};
struct IntervalTimer {
    Expiration expiry;
    bool pending = false;
};

uint64_t add_time(uint64_t first, uint64_t second_value) {
    return first + std::min(second_value, maximum_time - first);
}
uint64_t component(int64_t seconds, int64_t fraction, uint64_t scale) {
    if (seconds < 0 || fraction < 0 || uint64_t(fraction) >= second / scale)
        guest_error(22);
    const auto value = static_cast<unsigned __int128>(seconds) * second + uint64_t(fraction) * scale;
    return value > maximum_time ? maximum_time : uint64_t(value);
}
void buffer(Memory &memory, uint64_t address, uint64_t size, unsigned permissions) {
    if (!address || !memory.accessible(address, size, permissions))
        guest_error(14);
}
Setting read_setting(Memory &memory, uint64_t address, bool microseconds) {
    buffer(memory, address, 32, read_perm);
    const uint64_t scale = microseconds ? 1000 : 1;
    return {component(int64_t(memory.load(address, 8)), int64_t(memory.load(address + 8, 8)), scale),
            component(int64_t(memory.load(address + 16, 8)), int64_t(memory.load(address + 24, 8)), scale)};
}
void write_setting(Memory &memory, uint64_t address, const Setting &setting, bool microseconds) {
    buffer(memory, address, 32, write_perm);
    const uint64_t scale = microseconds ? 1000 : 1;
    memory.store(address, setting.interval / second, 8);
    memory.store(address + 8, setting.interval % second / scale, 8);
    memory.store(address + 16, setting.value / second, 8);
    memory.store(address + 24, setting.value % second / scale, 8);
}
uint64_t wall_time(bool realtime) {
    timespec value{};
    check_host(::clock_gettime(realtime ? CLOCK_REALTIME : CLOCK_MONOTONIC, &value));
    if (value.tv_sec < 0)
        return 0;
    const auto total = static_cast<unsigned __int128>(value.tv_sec) * second + uint64_t(value.tv_nsec);
    return total > maximum_time ? maximum_time : uint64_t(total);
}
int overrun_add(int value, uint64_t increment) {
    return value + int(std::min<uint64_t>(increment, INT_MAX - value));
}
uint64_t advance(Expiration &expiry, uint64_t now) {
    if (!expiry.armed || now < expiry.deadline)
        return 0;
    if (!expiry.interval) {
        expiry.armed = false;
        return 1;
    }
    const uint64_t count = (now - expiry.deadline) / expiry.interval + 1;
    const auto next = static_cast<unsigned __int128>(expiry.deadline) +
                      static_cast<unsigned __int128>(count) * expiry.interval;
    expiry.deadline = next > maximum_time ? maximum_time : uint64_t(next);
    expiry.armed = expiry.deadline > now;
    return count;
}
Setting snapshot(const Expiration &expiry, uint64_t now) {
    return {expiry.interval, expiry.armed && expiry.deadline > now ? expiry.deadline - now : 0};
}
void arm(Expiration &expiry, const Setting &setting, uint64_t now, bool absolute) {
    expiry.interval = setting.interval;
    expiry.absolute = absolute;
    expiry.armed = setting.value != 0;
    expiry.deadline = absolute ? setting.value : add_time(now, setting.value);
}
uint32_t private_token(const signals::Info &info) {
    uint32_t token;
    std::memcpy(&token, info.payload.data() + 16, sizeof(token));
    return token;
}
int32_t notification_id(const signals::Info &info) {
    int32_t id;
    std::memcpy(&id, info.payload.data(), sizeof(id));
    return id;
}
void set_overrun(signals::Info &info, int overrun) {
    const uint32_t zero = 0;
    std::memcpy(info.payload.data() + 4, &overrun, sizeof(overrun));
    std::memcpy(info.payload.data() + 16, &zero, sizeof(zero));
}
signals::Info timer_info(const PosixTimer &timer, uint32_t token, int overrun) {
    signals::Info info;
    info.signal = timer.signal;
    info.code = timer_signal_code;
    std::memcpy(info.payload.data(), &timer.id, sizeof(timer.id));
    std::memcpy(info.payload.data() + 4, &overrun, sizeof(overrun));
    std::memcpy(info.payload.data() + 8, &timer.value, sizeof(timer.value));
    std::memcpy(info.payload.data() + 16, &token, sizeof(token));
    return info;
}
const signals::ThreadRef *find_thread(const signals::ProcessRef &process, uint32_t tid) {
    const auto found = std::find_if(process.threads.begin(), process.threads.end(),
                                    [tid](const auto &thread) { return thread.tid == tid; });
    return found == process.threads.end() ? nullptr : &*found;
}
bool retains_signal(const signals::ProcessRef &process, const signals::ThreadRef *target, int signal) {
    if (!signals::ignored(*process.signals, signal))
        return true;
    const auto retained = [signal](const auto &thread) {
        return (thread.mask && (*thread.mask & signals::bit(signal))) ||
               (thread.signals && thread.signals->wait_set &&
                (*thread.signals->wait_set & signals::bit(signal)));
    };
    return target ? retained(*target) : std::any_of(process.threads.begin(), process.threads.end(), retained);
}
} // namespace

struct ProcessTimers::Impl {
    uint32_t pid;
    uint32_t next_id = 0, next_token = 1;
    std::map<int32_t, PosixTimer> timers;
    std::map<uint32_t, Notification> notifications;
    std::array<IntervalTimer, 3> intervals;

    explicit Impl(uint32_t owner) : pid(owner) {
        if (!pid)
            guest_error(22);
    }
    signals::ProcessRef process(const ProcessTimerHooks &hooks, uint32_t id) const {
        if (!hooks.signals.process)
            throw Failure("process timers require a guest process signal hook");
        return hooks.signals.process(id);
    }
    signals::ProcessRef owner(const ProcessTimerHooks &hooks) const {
        auto result = process(hooks, pid);
        if (result.pid != pid || !result.signals)
            guest_error(3);
        return result;
    }
    std::optional<uint64_t> clock_now(const TimerClock &clock, bool absolute,
                                      const ProcessTimerHooks &hooks) const {
        if (clock.kind != ClockKind::Cpu)
            return wall_time(clock.kind == ClockKind::Realtime && absolute);
        if (!hooks.cpu_usage)
            throw Failure("CPU timers require a guest PID/TID CPU accounting hook");
        const auto usage = hooks.cpu_usage(clock.id, clock.per_thread);
        if (!usage)
            return {};
        const auto value = clock.sample == 0   ? static_cast<unsigned __int128>(usage->user_nanoseconds) +
                                                     usage->system_nanoseconds
                           : clock.sample == 1 ? usage->user_nanoseconds
                                               : usage->runtime_nanoseconds;
        return value > maximum_time ? maximum_time : uint64_t(value);
    }
    TimerClock cpu_clock(unsigned sample) const {
        return {ClockKind::Cpu, pid, false, sample};
    }
    TimerClock resolve_clock(int32_t id, const ProcessTimerHooks &hooks) const {
        if (id == 0)
            return {ClockKind::Realtime};
        if (id == 1)
            return {ClockKind::Monotonic};
        if (id == 4 || id == 5 || id == 6 || id == 7 || id == 8 || id == 9 || id == 11)
            guest_error(95);
        TimerClock result{ClockKind::Cpu};
        if (id == 2) {
            result.id = pid;
        } else if (id == 3) {
            result.per_thread = true;
        } else {
            const auto encoded = uint32_t(id);
            if (id >= 0)
                guest_error(22);
            if ((encoded & 3) == 3)
                guest_error(95);
            result.id = ~encoded >> 3;
            result.per_thread = encoded & 4;
            result.sample = encoded & 3;
        }
        if (result.per_thread) {
            if (!result.id) {
                if (!hooks.signals.current)
                    throw Failure("thread CPU timers require the creating guest TID");
                const auto current = hooks.signals.current();
                if (current.pid != pid || !current.tid)
                    guest_error(22);
                result.id = current.tid;
            }
            const auto target = owner(hooks);
            if (!find_thread(target, result.id))
                guest_error(22);
        } else {
            if (!result.id)
                result.id = pid;
            signals::ProcessRef target;
            try {
                target = process(hooks, result.id);
            } catch (const GuestError &error) {
                if (error.number == 3)
                    guest_error(22);
                throw;
            }
            if (target.pid != result.id || !target.signals)
                guest_error(22);
        }
        if (!clock_now(result, false, hooks))
            guest_error(22);
        return result;
    }
    PosixTimer &require(int32_t id) {
        const auto found = timers.find(id);
        if (found == timers.end())
            guest_error(22);
        return found->second;
    }
    uint32_t token() {
        for (;;) {
            const auto value = next_token++;
            if (value && !notifications.count(value))
                return value;
        }
    }
    void account_expirations(PosixTimer &timer, uint64_t count) {
        if (!timer.pending_token || !count)
            return;
        const auto found = notifications.find(timer.pending_token);
        if (found != notifications.end()) {
            found->second.generation = timer.generation;
            found->second.overrun = overrun_add(found->second.overrun, count);
        }
    }
    void expire(PosixTimer &timer, const signals::ProcessRef &target, ProcessTimerHooks &hooks) {
        if (!timer.expiry.armed && !timer.waiting_expirations)
            return;
        uint64_t count = timer.waiting_expirations;
        if (timer.expiry.armed) {
            const auto now = clock_now(timer.clock, timer.expiry.absolute, hooks);
            if (!now) {
                timer.expiry.armed = false;
                timer.waiting_expirations = 0;
                return;
            }
            const auto expired = advance(timer.expiry, *now);
            count += std::min(expired, UINT64_MAX - count);
        }
        timer.waiting_expirations = 0;
        if (!count || timer.notify == 1)
            return;
        const auto *thread = timer.notify == 4 ? find_thread(target, timer.target_tid) : nullptr;
        if (timer.notify == 4 && (!thread || !thread->signals)) {
            timer.expiry.armed = false;
            return;
        }
        if (timer.pending_token) {
            account_expirations(timer, count);
            return;
        }
        const auto notify = [&](const signals::Info &info) {
            if (thread)
                return signals::send_thread(hooks.signals, target, *thread, info, false,
                                            timer.signal_charge);
            return signals::send_process(hooks.signals, target, info, false, timer.signal_charge);
        };
        auto &pending = thread ? thread->signals->pending : target.signals->pending;
        // A standard signal already pending has priority over a new timer
        // notification; realtime notifications remain one per timer.
        if ((timer.signal < 32 && (pending.mask() & signals::bit(timer.signal))) ||
            !retains_signal(target, thread, timer.signal)) {
            // Even an ignored/coalesced SIGCONT must resume a stopped group.
            const auto info = timer_info(timer, 0, overrun_add(0, count - 1));
            const int result = notify(info);
            if (result < 0 && result != -3 && result != -11)
                guest_error(-result);
            return;
        }
        const auto serial = token();
        const int overrun = overrun_add(0, count - 1);
        notifications.emplace(serial, Notification{timer.id, timer.generation, overrun});
        timer.pending_token = serial;
        const auto info = timer_info(timer, serial, overrun);
        const int result = notify(info);
        if (result < 0) {
            notifications.erase(serial);
            timer.pending_token = 0;
            if (result == -3)
                timer.expiry.armed = false;
            else if (result == -11)
                timer.waiting_expirations = count;
            else
                guest_error(-result);
        }
    }
    void expire_interval(unsigned which, const signals::ProcessRef &target, ProcessTimerHooks &hooks) {
        auto &timer = intervals[which];
        if (!timer.expiry.armed || (which == 0 && timer.pending))
            return;
        const TimerClock clock = which == 0 ? TimerClock{} : cpu_clock(which == 1 ? 1 : 0);
        const auto now = clock_now(clock, false, hooks);
        if (!now) {
            timer.expiry.armed = false;
            return;
        }
        if (*now < timer.expiry.deadline)
            return;
        const int signal = which == 0 ? 14 : which == 1 ? 26 : 27;
        if (which == 0) {
            // Linux reloads ITIMER_REAL when a SIGALRM is dequeued, keeping
            // blocked periodic alarms from generating repeated host wakeups.
            timer.expiry.armed = false;
            timer.pending = retains_signal(target, nullptr, signal);
        } else {
            advance(timer.expiry, *now);
        }
        signals::Info info;
        info.signal = signal;
        info.code = kernel_signal_code;
        const auto result = signals::send_process(hooks.signals, target, info);
        if (result < 0 && result != -11 && result != -3)
            guest_error(-result);
        if (which == 0 && result < 0)
            timer.pending = false;
    }
    void poll(ProcessTimerHooks &hooks) {
        const bool any = std::any_of(timers.begin(), timers.end(),
                                     [](const auto &item) {
                                         return item.second.expiry.armed || item.second.waiting_expirations;
                                     }) ||
                         std::any_of(intervals.begin(), intervals.end(),
                                     [](const auto &timer) { return timer.expiry.armed; });
        if (!any)
            return;
        const auto target = owner(hooks);
        if (target.zombie)
            return;
        for (auto &[id, timer] : timers) {
            (void)id;
            expire(timer, target, hooks);
        }
        for (unsigned which = 0; which < intervals.size(); ++which)
            expire_interval(which, target, hooks);
    }
    Setting posix_setting(PosixTimer &timer, const ProcessTimerHooks &hooks) {
        const auto now = clock_now(timer.clock, timer.expiry.absolute, hooks);
        if (!now) {
            timer.expiry.armed = false;
            return {timer.expiry.interval, 0};
        }
        return snapshot(timer.expiry, *now);
    }
    Setting interval_setting(unsigned which, const ProcessTimerHooks &hooks) {
        auto &timer = intervals[which];
        const auto now = clock_now(which == 0 ? TimerClock{} : cpu_clock(which == 1 ? 1 : 0), false, hooks);
        if (!now) {
            timer.expiry.armed = false;
            return {timer.expiry.interval, 0};
        }
        return snapshot(timer.expiry, *now);
    }
    void create(Memory &memory, int32_t clock_id, uint64_t event_address, uint64_t result_address,
                ProcessTimerHooks &hooks) {
        const auto clock = resolve_clock(clock_id, hooks);
        if (timers.size() >= maximum_timers || next_id > INT_MAX)
            guest_error(11);
        PosixTimer timer;
        timer.clock = clock;
        timer.id = int32_t(next_id);
        timer.value = next_id;
        if (event_address) {
            buffer(memory, event_address, 64, read_perm);
            timer.value = memory.load(event_address, 8);
            timer.signal = int32_t(memory.load(event_address + 8, 4));
            timer.notify = int32_t(memory.load(event_address + 12, 4));
            if (timer.notify != 0 && timer.notify != 1 && timer.notify != 2 && timer.notify != 4)
                guest_error(22);
            if (timer.notify != 1 && !signals::bit(timer.signal))
                guest_error(22);
            if (timer.notify == 4) {
                timer.target_tid = uint32_t(memory.load(event_address + 16, 4));
                const auto target = owner(hooks);
                const auto *thread = find_thread(target, timer.target_tid);
                if (!timer.target_tid || !thread || !thread->signals)
                    guest_error(22);
            }
        }
        buffer(memory, result_address, 4, write_perm);
        if (!hooks.reserve_signal)
            throw Failure("POSIX timers require a guest pending-signal reservation hook");
        timer.signal_charge = hooks.reserve_signal(pid);
        if (!timer.signal_charge || !*timer.signal_charge)
            throw Failure("POSIX timer reservation returned no signal charge");
        const auto id = timer.id;
        auto inserted = timers.emplace(id, std::move(timer)).first;
        try {
            memory.store(result_address, uint32_t(id), 4);
        } catch (...) {
            timers.erase(inserted);
            throw;
        }
        ++next_id;
    }
    int64_t dispatch(CpuState &cpu, Memory &memory, ProcessTimerHooks &hooks) {
        poll(hooks);
        const auto arguments = cpu.x;
        const auto &a = arguments;
        switch (a[8]) {
        case 107:
            create(memory, int32_t(a[0]), a[1], a[2], hooks);
            return 0;
        case 108: {
            auto &timer = require(int32_t(a[0]));
            write_setting(memory, a[1], posix_setting(timer, hooks), false);
            return 0;
        }
        case 109:
            return require(int32_t(a[0])).overrun;
        case 110: {
            if (uint32_t(a[1]) & ~absolute_time)
                guest_error(22);
            const auto setting = read_setting(memory, a[2], false);
            auto &timer = require(int32_t(a[0]));
            const auto previous = posix_setting(timer, hooks);
            const bool absolute = uint32_t(a[1]) & absolute_time;
            const auto now = clock_now(timer.clock, absolute, hooks);
            if (!now)
                guest_error(3);
            ++timer.generation;
            timer.overrun = 0;
            timer.waiting_expirations = 0;
            arm(timer.expiry, setting, *now, absolute);
            if (a[3])
                write_setting(memory, a[3], previous, false);
            expire(timer, owner(hooks), hooks);
            return 0;
        }
        case 111:
            require(int32_t(a[0]));
            timers.erase(int32_t(a[0]));
            return 0;
        case 102:
        case 103: {
            const auto which = uint32_t(a[0]);
            if (which >= intervals.size())
                guest_error(22);
            if (a[8] == 102) {
                write_setting(memory, a[1], interval_setting(which, hooks), true);
                return 0;
            }
            const auto setting = a[1] ? read_setting(memory, a[1], true) : Setting{};
            const auto previous = interval_setting(which, hooks);
            const auto now =
                clock_now(which == 0 ? TimerClock{} : cpu_clock(which == 1 ? 1 : 0), false, hooks);
            if (!now)
                guest_error(3);
            auto &timer = intervals[which];
            arm(timer.expiry, setting, *now, false);
            timer.pending = false;
            if (which == 0 && !setting.value)
                timer.expiry.interval = 0;
            if (a[2])
                write_setting(memory, a[2], previous, true);
            return 0;
        }
        default:
            guest_error(38);
        }
    }
};

ProcessTimers::ProcessTimers(uint32_t pid) : impl_(std::make_unique<Impl>(pid)) {}
ProcessTimers::~ProcessTimers() = default;
ProcessTimers::ProcessTimers(ProcessTimers &&) noexcept = default;
ProcessTimers &ProcessTimers::operator=(ProcessTimers &&) noexcept = default;
int64_t ProcessTimers::dispatch(CpuState &cpu, Memory &memory, ProcessTimerHooks &hooks) {
    return impl_->dispatch(cpu, memory, hooks);
}
void ProcessTimers::poll(ProcessTimerHooks &hooks) {
    impl_->poll(hooks);
}
signals::Deadline ProcessTimers::next_deadline(const ProcessTimerHooks &hooks) const {
    std::optional<uint64_t> delay;
    const auto consider = [&delay](uint64_t remaining) {
        delay = delay ? std::min(*delay, remaining) : remaining;
    };
    std::optional<signals::ProcessRef> target;
    for (const auto &[id, timer] : impl_->timers) {
        (void)id;
        if (!timer.expiry.armed || timer.clock.kind == ClockKind::Cpu || timer.notify == 1 ||
            timer.pending_token)
            continue;
        if (!target)
            target = impl_->owner(hooks);
        const auto *thread = timer.notify == 4 ? find_thread(*target, timer.target_tid) : nullptr;
        if ((timer.notify == 4 && (!thread || !thread->signals)) ||
            (!retains_signal(*target, thread, timer.signal) && timer.signal != signals::continue_signal))
            continue;
        const auto now = impl_->clock_now(timer.clock, timer.expiry.absolute, hooks);
        uint64_t remaining = timer.expiry.deadline > *now ? timer.expiry.deadline - *now : 0;
        // Recheck absolute realtime timers after wall-clock adjustments while idle.
        if (timer.clock.kind == ClockKind::Realtime && timer.expiry.absolute)
            remaining = std::min(remaining, second);
        consider(remaining);
    }
    const auto &real = impl_->intervals[0];
    if (real.expiry.armed && !real.pending) {
        const auto now = wall_time(false);
        consider(real.expiry.deadline > now ? real.expiry.deadline - now : 0);
    }
    if (!delay)
        return {};
    const auto now = signals::Clock::now();
    const auto maximum =
        std::chrono::duration_cast<std::chrono::nanoseconds>(signals::Clock::time_point::max() - now);
    return now + std::chrono::nanoseconds(std::min<uint64_t>(*delay, uint64_t(maximum.count())));
}
bool ProcessTimers::signal_dequeued(signals::Info &info, ProcessTimerHooks &hooks) {
    if (info.signal == 14) {
        auto &real = impl_->intervals[0];
        if (real.pending) {
            real.pending = false;
            if (real.expiry.interval) {
                real.expiry.armed = true;
                advance(real.expiry, wall_time(false));
            }
        }
    }
    if (info.code != timer_signal_code || !private_token(info))
        return true;
    const auto token = private_token(info);
    const auto event = impl_->notifications.find(token);
    if (event == impl_->notifications.end() || event->second.timer_id != notification_id(info))
        return false;
    const auto found = impl_->timers.find(event->second.timer_id);
    if (found != impl_->timers.end() && found->second.pending_token == token) {
        auto &timer = found->second;
        if (timer.expiry.armed) {
            const auto now = impl_->clock_now(timer.clock, timer.expiry.absolute, hooks);
            if (now)
                impl_->account_expirations(timer, advance(timer.expiry, *now));
            else
                timer.expiry.armed = false;
        }
        timer.pending_token = 0;
        if (timer.generation == event->second.generation)
            timer.overrun = event->second.overrun;
    }
    set_overrun(info, event->second.overrun);
    impl_->notifications.erase(event);
    return true;
}
void ProcessTimers::signal_discarded(const signals::Info &info) {
    if (info.signal == 14)
        impl_->intervals[0].pending = false;
    if (info.code != timer_signal_code)
        return;
    const auto token = private_token(info);
    const auto event = impl_->notifications.find(token);
    if (event == impl_->notifications.end() || event->second.timer_id != notification_id(info))
        return;
    const auto found = impl_->timers.find(event->second.timer_id);
    if (found != impl_->timers.end() && found->second.pending_token == token)
        found->second.pending_token = 0;
    impl_->notifications.erase(event);
}
void ProcessTimers::exec_reset() {
    impl_->timers.clear();
    impl_->notifications.clear();
}
void ProcessTimers::clear() {
    exec_reset();
    impl_->intervals = {};
}
} // namespace operit::arm64
