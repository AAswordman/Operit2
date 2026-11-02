#include "linux/signals/signals.h"
#include "linux/signals/signal_file.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <algorithm>

namespace operit::arm64::signals {
namespace {
void buffer(Memory &memory, uint64_t address, size_t size, unsigned permissions) {
    if (!memory.accessible(address, size, permissions))
        guest_error(14);
}
template <typename T> T read(Memory &memory, uint64_t address) {
    buffer(memory, address, sizeof(T), read_perm);
    T value;
    memory.copy_out(address, &value, sizeof(value));
    return value;
}
template <typename T> void write(Memory &memory, uint64_t address, const T &value) {
    buffer(memory, address, sizeof(T), write_perm);
    memory.copy_in(address, &value, sizeof(value));
}
void validate_signal(int signal, bool zero = false) {
    if (signal < (zero ? 0 : 1) || signal > int(maximum_signal))
        guest_error(22);
}
void size_argument(uint64_t size) {
    if (size != 8)
        guest_error(22);
}
bool permitted(const ProcessRef &sender, const ProcessRef &target, int signal) {
    return sender.effective_uid == 0 || sender.real_uid == target.real_uid ||
           sender.real_uid == target.saved_uid || sender.effective_uid == target.real_uid ||
           sender.effective_uid == target.saved_uid ||
           (signal == continue_signal && sender.session == target.session);
}
std::vector<ProcessRef> processes(Hooks &hooks) {
    if (!hooks.processes)
        guest_error(38);
    return hooks.processes();
}
int64_t kill(Hooks &hooks, const ProcessRef &sender, int32_t pid, int signal) {
    validate_signal(signal, true);
    bool found = false, accepted = false;
    int error = 3;
    for (const auto &target : processes(hooks)) {
        bool match = pid > 0     ? target.pid == uint32_t(pid)
                     : pid == 0  ? target.group == sender.group
                     : pid == -1 ? target.pid != 1 && target.pid != sender.pid
                                 : uint64_t(target.group) == uint64_t(-int64_t(pid));
        if (!match)
            continue;
        found = true;
        if (!permitted(sender, target, signal)) {
            error = 1;
            continue;
        }
        int result = send_process(hooks, target, Info::user(signal, sender.pid, sender.real_uid));
        if (!result)
            accepted = true;
        else
            error = -result;
    }
    return accepted ? 0 : -(found ? error : 3);
}
int64_t thread_kill(Hooks &hooks, const ProcessRef &sender, int32_t pid, int32_t tid, int signal,
                    const std::optional<Info> &queued = {}) {
    validate_signal(signal, true);
    if (tid <= 0 || pid < 0)
        guest_error(22);
    if (queued && (queued->code >= 0 || queued->code == -6) &&
        (!hooks.current || hooks.current().tid != uint32_t(tid)))
        guest_error(1);
    for (const auto &target : processes(hooks)) {
        if (target.zombie || (pid && target.pid != uint32_t(pid)))
            continue;
        auto it = std::find_if(target.threads.begin(), target.threads.end(),
                               [tid](const ThreadRef &thread) { return thread.tid == uint32_t(tid); });
        if (it == target.threads.end())
            continue;
        if (!permitted(sender, target, signal))
            guest_error(1);
        Info info = queued.value_or(Info::user(signal, sender.pid, sender.real_uid, -6));
        info.signal = signal;
        return send_thread(hooks, target, *it, info);
    }
    guest_error(3);
}
Deadline wait_deadline(Memory &memory, uint64_t address) {
    if (!address)
        return {};
    buffer(memory, address, 16, read_perm);
    int64_t seconds = memory.load(address, 8), nanos = memory.load(address + 8, 8);
    if (seconds < 0 || nanos < 0 || nanos >= 1000000000)
        guest_error(22);
    auto now = Clock::now();
    auto maximum = std::chrono::duration_cast<std::chrono::seconds>(Clock::time_point::max() - now).count();
    if (seconds >= maximum)
        return Clock::time_point::max();
    return now + std::chrono::seconds(seconds) + std::chrono::nanoseconds(nanos);
}
void prepare_info(Memory &memory, uint64_t address) {
    buffer(memory, address, sizeof(Info), write_perm);
    size_t remaining = sizeof(Info);
    while (remaining) {
        memory.translate(address, write_perm);
        const auto count = std::min<size_t>(remaining, page_size - address % page_size);
        address += count;
        remaining -= count;
    }
}
Info queued_info(Memory &memory, uint64_t address, int signal) {
    auto info = read<Info>(memory, address);
    info.signal = signal;
    info.padding = 0;
    // si_sys_private is a kernel-only timer token, never a user-supplied one.
    if (info.code == -2)
        std::fill_n(info.payload.begin() + 16, 4, uint8_t{0});
    return info;
}
struct SignalWait {
    ThreadState &state;
    bool active = true;
    SignalWait(ThreadState &thread, uint64_t set) : state(thread) {
        state.wait_set = set;
    }
    void finish() {
        if (active) {
            state.wait_set.reset();
            active = false;
        }
    }
    ~SignalWait() {
        finish();
    }
};
void discard_ignored(const ProcessRef &process, int signal) {
    process.signals->pending.discard(bit(signal));
    for (const auto &thread : process.threads)
        if (thread.signals)
            thread.signals->pending.discard(bit(signal));
}
} // namespace

int64_t dispatch(CpuState &cpu, Memory &memory, Hooks &hooks) {
    if (!hooks.current || !hooks.process)
        guest_error(38);
    const auto a = cpu.x;
    const auto thread = hooks.current();
    const auto process = hooks.process(thread.pid);
    if (!thread.signals || !thread.mask || !process.signals)
        guest_error(3);
    auto &state = *thread.signals;
    auto &mask = *thread.mask;
    auto &signals = *process.signals;
    switch (a[8]) {
    case 129: // kill
        return kill(hooks, process, int32_t(a[0]), int32_t(a[1]));
    case 130: // tkill
        return thread_kill(hooks, process, 0, int32_t(a[0]), int32_t(a[1]));
    case 131: // tgkill
        if (int32_t(a[0]) <= 0)
            guest_error(22);
        return thread_kill(hooks, process, int32_t(a[0]), int32_t(a[1]), int32_t(a[2]));
    case 132: { // sigaltstack
        std::optional<AltStack> requested;
        if (a[0])
            requested = read<AltStack>(memory, a[0]);
        auto old = visible_altstack(state, cpu.sp);
        if (a[1])
            buffer(memory, a[1], sizeof(AltStack), write_perm);
        if (requested)
            change_altstack(state, *requested, cpu.sp);
        if (a[1])
            write(memory, a[1], old);
        return 0;
    }
    case 133: { // rt_sigsuspend
        size_argument(a[1]);
        const auto temporary = read<uint64_t>(memory, a[0]);
        if (!hooks.block)
            guest_error(38);
        begin_temporary_mask(state, mask, temporary);
        remember_syscall(state, cpu, true);
        state.restart->without_handler_only = true;
        state.suspending = true;
        auto poll = [shared = process.signals, thread]() -> std::optional<int64_t> {
            if (!has_deliverable(*shared, *thread.signals, *thread.mask))
                return {};
            interrupted(*thread.signals, *thread.mask);
            return -4;
        };
        try {
            if (auto result = poll())
                return *result;
            hooks.block({}, -4, std::move(poll));
        } catch (...) {
            end_temporary_mask(state, mask);
            state.suspending = false;
            state.restart.reset();
            throw;
        }
        return 0;
    }
    case 134: { // rt_sigaction
        size_argument(a[3]);
        const int signal = int32_t(a[0]);
        validate_signal(signal);
        std::optional<Action> requested;
        if (a[1]) {
            if (signal == kill_signal || signal == stop_signal)
                guest_error(22);
            requested = read<Action>(memory, a[1]);
            constexpr uint64_t supported = sa_no_cldstop | sa_no_cldwait | sa_siginfo | sa_restorer |
                                           sa_onstack | sa_restart | sa_nodefer | sa_resethand;
            requested->flags &= supported;
            requested->mask &= ~unblockable;
        }
        if (a[2])
            write(memory, a[2], signals.actions->entries[signal - 1]);
        if (requested) {
            signals.actions->entries[signal - 1] = *requested;
            if (ignored(signals, signal))
                discard_ignored(process, signal);
        }
        return 0;
    }
    case 135: { // rt_sigprocmask
        size_argument(a[3]);
        std::optional<uint64_t> requested;
        if (a[1]) {
            if (a[0] > 2)
                guest_error(22);
            requested = read<uint64_t>(memory, a[1]) & ~unblockable;
        }
        if (a[2])
            write(memory, a[2], mask);
        if (requested)
            mask = a[0] == 0 ? mask | *requested : a[0] == 1 ? mask & ~*requested : *requested;
        return 0;
    }
    case 136: { // rt_sigpending permits a short sigset, unlike sigprocmask.
        if (a[1] > 8)
            guest_error(22);
        const uint64_t pending = pending_mask(signals, state) & mask;
        buffer(memory, a[0], a[1], write_perm);
        memory.copy_in(a[0], &pending, a[1]);
        return 0;
    }
    case 137: { // rt_sigtimedwait
        size_argument(a[3]);
        const uint64_t set = read<uint64_t>(memory, a[0]) & ~unblockable;
        const auto deadline = wait_deadline(memory, a[2]);
        auto wait = std::make_shared<SignalWait>(state, set);
        auto poll = [&memory, shared = process.signals, thread, set, output = a[1], deadline,
                     wait]() -> std::optional<int64_t> {
            auto &state = *thread.signals;
            if (has_waitable(*shared, state, set)) {
                if (output)
                    prepare_info(memory, output);
                if (auto queued = take_waitable(*shared, state, set)) {
                    if (output)
                        write(memory, output, queued->info);
                    wait->finish();
                    state.restart.reset();
                    return queued->info.signal;
                }
            }
            // A zero timeout is a pending-queue query even when an unrelated
            // signal is deliverable. Consume a matching signal before expiry.
            if (deadline && Clock::now() >= *deadline) {
                wait->finish();
                state.restart.reset();
                return -11;
            }
            if (has_deliverable(*shared, state, *thread.mask)) {
                wait->finish();
                interrupted(state, *thread.mask);
                return -4;
            }
            return {};
        };
        if (auto result = poll())
            return *result;
        if (!hooks.block)
            guest_error(38);
        remember_syscall(state, cpu, false);
        try {
            // The registration also lives in the callback, so destroying a
            // blocked wait after EINTR, EFAULT, exit, or timeout clears wait_set.
            hooks.block(deadline, -11, std::move(poll));
        } catch (...) {
            wait->finish();
            state.restart.reset();
            throw;
        }
        return 0;
    }
    case 138: { // rt_sigqueueinfo
        const int signal = int32_t(a[1]);
        validate_signal(signal, true);
        const int32_t pid = int32_t(a[0]);
        Info info = queued_info(memory, a[2], signal);
        if ((info.code >= 0 || info.code == -6) && thread.tid != uint32_t(pid))
            guest_error(1);
        if (pid <= 0)
            guest_error(3);
        auto target = hooks.process(uint32_t(pid));
        if (!permitted(process, target, signal))
            guest_error(1);
        return send_process(hooks, target, info);
    }
    case 139: // rt_sigreturn
        if (!restore_frame(memory, cpu, state, mask)) {
            force_fault(signals, state, mask, segv_signal, cpu.sp, 1);
            return 0;
        }
        return int64_t(cpu.x[0]);
    case 240: { // rt_tgsigqueueinfo
        if (int32_t(a[0]) <= 0 || int32_t(a[1]) <= 0)
            guest_error(22);
        const int signal = int32_t(a[2]);
        validate_signal(signal, true);
        const Info info = queued_info(memory, a[3], signal);
        return thread_kill(hooks, process, int32_t(a[0]), int32_t(a[1]), signal, info);
    }
    default:
        guest_error(38);
    }
}
} // namespace operit::arm64::signals

namespace operit::arm64 {
int64_t Syscalls::dispatch_signal(CpuState &cpu) {
    if (!scheduler_.processes())
        guest_error(38);
    auto hooks = scheduler_.processes()->signal_hooks();
    if (cpu.x[8] == 74)
        return signals::signalfd4(memory_, descriptors_, hooks, int32_t(cpu.x[0]), cpu.x[1], cpu.x[2],
                                  uint32_t(cpu.x[3]));
    return signals::dispatch(cpu, memory_, hooks);
}
} // namespace operit::arm64

namespace operit::arm64 {
bool Syscalls::interrupt_for_pending_signal() {
    auto *controller = scheduler_.processes();
    if (!controller)
        return false;
    auto &task = scheduler_.current();
    auto hooks = controller->signal_hooks();
    auto process = hooks.process(task.pid);
    if (!process.signals || !signals::has_deliverable(*process.signals, task.signals, task.signal_mask))
        return false;
    signals::interrupted(task.signals, task.signal_mask);
    return true;
}
void Syscalls::broken_pipe(uint32_t tid) {
    auto *controller = scheduler_.processes();
    auto *task = scheduler_.lookup(tid);
    if (!controller || !task)
        return;
    auto hooks = controller->signal_hooks();
    auto process = hooks.process(task->pid);
    for (const auto &thread : process.threads)
        if (thread.tid == tid) {
            signals::send_thread(hooks, process, thread, signals::Info::user(13, process.pid, process.real_uid));
            break;
        }
}
} // namespace operit::arm64
