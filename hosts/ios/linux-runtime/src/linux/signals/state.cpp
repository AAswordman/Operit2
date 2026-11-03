#include "linux/signals/signals.h"
#include <algorithm>
#include <cstring>
#include <utility>

namespace operit::arm64::signals {
namespace {
constexpr uint64_t stop_set = bit(19) | bit(20) | bit(21) | bit(22);
bool synchronous(const Queued &entry) {
    return entry.forced || (entry.info.code > 0 && (bit(entry.info.signal) &
                                                    (bit(4) | bit(5) | bit(7) | bit(8) | bit(11) | bit(31))));
}
auto select_entry(const std::deque<Queued> &entries, uint64_t allowed, bool synchronous_first,
                  bool include_forced) {
    auto selected = entries.end();
    for (auto it = entries.begin(); it != entries.end(); ++it) {
        if (!include_forced && it->forced)
            continue;
        if (!(allowed & bit(it->info.signal)) && !(synchronous_first && it->forced))
            continue;
        if (selected == entries.end() || (synchronous_first && synchronous(*it) && !synchronous(*selected)) ||
            ((!(synchronous_first && synchronous(*selected)) || synchronous(*it)) &&
             it->info.signal < selected->info.signal))
            selected = it;
    }
    return selected;
}
void discard_process(const ProcessRef &process, uint64_t set) {
    process.signals->pending.discard(set);
    for (const auto &thread : process.threads)
        if (thread.signals)
            thread.signals->pending.discard(set);
}
void prepare_generation(Hooks &hooks, const ProcessRef &process, const Info &info) {
    if (bit(info.signal) & stop_set)
        discard_process(process, bit(continue_signal));
    if (info.signal == continue_signal) {
        if (process.signals->stopped && !hooks.effect)
            guest_error(38);
        discard_process(process, stop_set);
        bool was_stopped = process.signals->stopped;
        process.signals->stopped = false;
        if (was_stopped)
            hooks.effect(process.pid, {Effect::Continue, info.signal, info});
    } else if (info.signal == kill_signal && process.signals->stopped) {
        if (!hooks.effect)
            guest_error(38);
        process.signals->stopped = false;
        hooks.effect(process.pid, {Effect::Continue, info.signal, info});
    }
}
bool retained_when_ignored(const ThreadRef &thread, int signal) {
    return thread.traced || (thread.mask && (*thread.mask & bit(signal))) ||
           (thread.signals && thread.signals->wait_set && (*thread.signals->wait_set & bit(signal)));
}
bool notify(Hooks &hooks, const ProcessRef &process, const ThreadRef &thread, const Info &info, bool force) {
    if (!thread.signals || !thread.mask)
        return false;
    if (thread.signals->wait_set && (*thread.signals->wait_set & bit(info.signal)) && !force) {
        if (hooks.wake)
            hooks.wake(thread.tid, false);
        return true;
    }
    if (force || (!(*thread.mask & bit(info.signal)) &&
                  (thread.traced || !ignored(*process.signals, info.signal)))) {
        if (hooks.wake)
            hooks.wake(thread.tid, true);
        return true;
    }
    return false;
}
} // namespace

Info Info::user(int signal, uint32_t pid, uint32_t uid, int code) {
    Info result;
    result.signal = signal;
    result.code = code;
    std::memcpy(result.payload.data(), &pid, sizeof(pid));
    std::memcpy(result.payload.data() + 4, &uid, sizeof(uid));
    return result;
}
Info Info::fault(int signal, uint64_t address, int code) {
    Info result;
    result.signal = signal;
    result.code = code;
    std::memcpy(result.payload.data(), &address, sizeof(address));
    return result;
}
Info Info::child(uint32_t pid, uint32_t uid, int status, int code) {
    auto result = user(17, pid, uid, code);
    int32_t value = status;
    std::memcpy(result.payload.data() + 8, &value, sizeof(value));
    return result;
}
uint32_t Info::sender_pid() const {
    uint32_t value;
    std::memcpy(&value, payload.data(), sizeof(value));
    return value;
}
uint32_t Info::sender_uid() const {
    uint32_t value;
    std::memcpy(&value, payload.data() + 4, sizeof(value));
    return value;
}
uint64_t Info::fault_address() const {
    uint64_t value;
    std::memcpy(&value, payload.data(), sizeof(value));
    return value;
}
uint64_t Pending::mask() const {
    uint64_t result = 0;
    for (const auto &entry : entries_)
        result |= bit(entry.info.signal);
    return result;
}
void Pending::set_observers(DequeueObserver dequeued, DiscardObserver discarded) {
    dequeued_ = std::move(dequeued);
    discarded_ = std::move(discarded);
}
void Pending::set_charge_source(ChargeSource source) {
    charge_source_ = std::move(source);
}
void Pending::discard(uint64_t set) {
    discard_if([set](const Info &info) { return bool(set & bit(info.signal)); });
}
size_t Pending::discard_if(const std::function<bool(const Info &)> &predicate) {
    // Evaluate once before mutating the queue, including if the predicate throws.
    // Notify only after removal: an observer may rearm a timer and enqueue again.
    std::vector<bool> selected;
    std::vector<Info> discarded;
    selected.reserve(entries_.size());
    for (const auto &entry : entries_) {
        const bool remove = predicate(entry.info);
        selected.push_back(remove);
        if (remove)
            discarded.push_back(entry.info);
    }
    if (discarded.empty())
        return 0;
    size_t index = 0;
    entries_.erase(std::remove_if(entries_.begin(), entries_.end(),
                                  [&](const Queued &) { return bool(selected[index++]); }),
                   entries_.end());
    if (discarded_)
        for (const auto &info : discarded)
            discarded_(info);
    return discarded.size();
}
bool Pending::enqueue(const Info &info, bool forced,
                      std::shared_ptr<process::UidLimitCharge> reserved) {
    if (!bit(info.signal))
        guest_error(22);
    auto replacement = entries_.end();
    if (info.signal < 32) {
        for (auto it = entries_.begin(); it != entries_.end(); ++it) {
            if (it->info.signal == info.signal) {
                if (!forced || it->forced)
                    return true;
                replacement = it;
                break;
            }
        }
    }
    if (info.signal >= 32 && entries_.size() >= 65536)
        return false;
    if (!reserved && charge_source_) {
        try {
            reserved = charge_source_(info, forced);
        } catch (const GuestError &error) {
            if (error.number == 11)
                return false;
            throw;
        }
    }
    if (replacement != entries_.end()) {
        const Info discarded = replacement->info;
        *replacement = {info, forced, std::move(reserved)};
        if (discarded_)
            discarded_(discarded);
    } else {
        entries_.push_back({info, forced, std::move(reserved)});
    }
    return true;
}
std::optional<Queued> Pending::peek(uint64_t allowed, bool synchronous_first,
                                     bool include_forced) const {
    auto it = select_entry(entries_, allowed, synchronous_first, include_forced);
    if (it == entries_.end())
        return {};
    return Queued{it->info, it->forced, {}};
}
std::optional<Queued> Pending::take(uint64_t allowed, bool synchronous_first, bool include_forced) {
    for (;;) {
        auto it = select_entry(entries_, allowed, synchronous_first, include_forced);
        if (it == entries_.end())
            return {};
        Queued result{it->info, it->forced, {}};
        // Release this allocation before the dequeue observer can requeue a
        // notification. A POSIX timer may still own its preallocated charge.
        entries_.erase(entries_.begin() + (it - entries_.cbegin()));
        if (!dequeued_ || dequeued_(result.info))
            return result;
    }
}
Effect default_effect(int signal) {
    switch (signal) {
    case 3:
    case 4:
    case 5:
    case 6:
    case 7:
    case 8:
    case 11:
    case 24:
    case 25:
    case 31:
        return Effect::Core;
    case 17:
    case 23:
    case 28:
        return Effect::None;
    case 18:
        return Effect::Continue;
    case 19:
    case 20:
    case 21:
    case 22:
        return Effect::Stop;
    default:
        return Effect::Terminate;
    }
}
bool ignored(const ProcessState &process, int signal) {
    if (signal == kill_signal || signal == stop_signal)
        return false;
    const auto &action = process.actions->entries.at(signal - 1);
    return action.handler == 1 || (action.handler == 0 && (default_effect(signal) == Effect::None ||
                                                           default_effect(signal) == Effect::Continue));
}
uint64_t pending_mask(const ProcessState &process, const ThreadState &thread) {
    return process.pending.mask() | thread.pending.mask();
}
bool has_waitable(const ProcessState &process, const ThreadState &thread, uint64_t set) {
    set &= ~unblockable;
    return thread.pending.peek(set, false, false).has_value() ||
           process.pending.peek(set, false, false).has_value();
}
std::optional<Queued> take_waitable(ProcessState &process, ThreadState &thread, uint64_t set) {
    set &= ~unblockable;
    auto queued = thread.pending.take(set, false, false);
    return queued ? queued : process.pending.take(set, false, false);
}
bool has_deliverable(const ProcessState &process, const ThreadState &thread, uint64_t mask) {
    const uint64_t allowed = ~(mask & ~unblockable);
    for (const Pending *pending : {&thread.pending, &process.pending}) {
        auto available = pending->mask() & allowed;
        while (available) {
            const int signal = __builtin_ctzll(available) + 1;
            if (!ignored(process, signal))
                return true;
            available &= available - 1;
        }
        if (auto entry = pending->peek(0, true); entry && entry->forced)
            return true;
    }
    return false;
}
std::shared_ptr<ProcessState> fork_process(const ProcessState &parent, bool share_actions) {
    auto result = std::make_shared<ProcessState>();
    result->actions = share_actions ? parent.actions : std::make_shared<ActionTable>(*parent.actions);
    return result;
}
ThreadState fork_thread(const ThreadState &parent, bool shared_memory, bool vfork) {
    ThreadState result;
    if (!shared_memory || vfork)
        result.altstack = parent.altstack;
    return result;
}
void exec_reset(ProcessState &process, ThreadState &thread) {
    process.actions = std::make_shared<ActionTable>(*process.actions);
    for (auto &action : process.actions->entries)
        if (action.handler != 1)
            action = {};
    thread.altstack = {};
    thread.restore_mask.reset();
    thread.interrupted_mask.reset();
    thread.wait_set.reset();
    thread.restart.reset();
    thread.suspending = false;
    process.stopped = false;
}
void remember_syscall(ThreadState &thread, const CpuState &cpu, bool restartable) {
    Restart value;
    std::copy_n(cpu.x.begin(), value.registers.size(), value.registers.begin());
    value.after_svc = cpu.pc;
    value.restartable = restartable;
    switch (cpu.x[8]) {
    case 22:  // epoll_pwait
    case 72:  // pselect6
    case 73:  // ppoll
    case 101: // nanosleep
    case 115: // clock_nanosleep
    case 441: // epoll_pwait2
        value.restartable = false;
        break;
    case 133: // rt_sigsuspend
        value.without_handler_only = true;
        break;
    case 137: // rt_sigtimedwait is never restarted by SA_RESTART.
        value.restartable = false;
        break;
    default:
        break;
    }
    thread.restart = value;
}
void interrupted(ThreadState &thread, uint64_t current_mask) {
    thread.interrupted_mask = current_mask & ~unblockable;
    if (thread.restart)
        thread.restart->interrupted = true;
}
void begin_temporary_mask(ThreadState &thread, uint64_t &mask, uint64_t temporary) {
    if (thread.restore_mask)
        guest_error(22);
    thread.restore_mask = mask;
    mask = temporary & ~unblockable;
}
void end_temporary_mask(ThreadState &thread, uint64_t &mask) {
    if (!thread.restore_mask)
        return;
    mask = *thread.restore_mask & ~unblockable;
    if (!thread.interrupted_mask)
        thread.restore_mask.reset();
}
int send_process(Hooks &hooks, const ProcessRef &target, const Info &info, bool force,
                 std::shared_ptr<process::UidLimitCharge> reserved) {
    if (!target.signals)
        return -3;
    if (target.zombie || !info.signal)
        return 0;
    if (!bit(info.signal))
        return -22;
    prepare_generation(hooks, target, info);
    if (!force && ignored(*target.signals, info.signal) &&
        std::none_of(target.threads.begin(), target.threads.end(),
                     [&](const ThreadRef &thread) { return retained_when_ignored(thread, info.signal); }))
        return 0;
    if (!target.signals->pending.enqueue(info, force, std::move(reserved)))
        return -11;
    // A shared signal belongs to one eligible thread, including a sigwaiter.
    for (const auto &thread : target.threads)
        if (notify(hooks, target, thread, info, force))
            break;
    return 0;
}
int send_thread(Hooks &hooks, const ProcessRef &process, const ThreadRef &target, const Info &info,
                bool force, std::shared_ptr<process::UidLimitCharge> reserved) {
    if (!process.signals || !target.signals)
        return -3;
    if (process.zombie || !info.signal)
        return 0;
    if (!bit(info.signal))
        return -22;
    prepare_generation(hooks, process, info);
    if (!force && ignored(*process.signals, info.signal) && !retained_when_ignored(target, info.signal))
        return 0;
    if (!target.signals->pending.enqueue(info, force, std::move(reserved)))
        return -11;
    notify(hooks, process, target, info, force);
    return 0;
}
void force_fault(ProcessState &process, ThreadState &thread, uint64_t &mask, int signal, uint64_t address,
                 int code) {
    if (!bit(signal))
        guest_error(22);
    auto &action = process.actions->entries[signal - 1];
    if (action.handler == 1 || (mask & bit(signal)))
        action = {};
    mask &= ~bit(signal);
    thread.pending.enqueue(Info::fault(signal, address, code), true);
}
} // namespace operit::arm64::signals
