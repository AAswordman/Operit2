#include "linux/signals/frame.h"
#include <algorithm>

namespace operit::arm64::signals {
namespace {
void resume_interrupted(CpuState &cpu, ThreadState &thread, bool handler_restarts, bool handler) {
    if (!thread.restart || !thread.restart->interrupted)
        return;
    const auto &restart = *thread.restart;
    if (restart.restartable && (handler_restarts || restart.always_restart) &&
        (!handler || !restart.without_handler_only) &&
        restart.after_svc >= 4 && int64_t(cpu.x[0]) == -4) {
        std::copy(restart.registers.begin(), restart.registers.end(), cpu.x.begin());
        cpu.pc = restart.after_svc - 4;
    }
}
void finish_mask(ThreadState &thread, uint64_t &mask) {
    if (thread.restore_mask)
        mask = *thread.restore_mask & ~unblockable;
    thread.restore_mask.reset();
    thread.interrupted_mask.reset();
    thread.restart.reset();
    thread.wait_set.reset();
    thread.suspending = false;
}
} // namespace
Delivery deliver(Memory &memory, CpuState &cpu, ProcessState &process, ThreadState &thread, uint64_t &mask,
                 uint64_t trampoline, const std::function<bool(CpuState &)> &before_handler,
                 std::optional<Queued> *tracer_signal,
                 const std::function<bool(const Queued &)> &before_delivery) {
    const uint64_t live_mask = thread.interrupted_mask.value_or(mask) & ~unblockable;
    const uint64_t allowed = ~live_mask;
    unsigned bad_frames = 0;
    for (;;) {
        auto pending = thread.pending.take(bit(kill_signal));
        if (!pending)
            pending = process.pending.take(bit(kill_signal));
        bool resumed = false;
        if (!pending && tracer_signal && *tracer_signal) {
            pending = std::move(*tracer_signal);
            tracer_signal->reset();
            resumed = true;
        }
        if (!pending)
            pending = thread.pending.take(allowed, true);
        if (!pending)
            pending = process.pending.take(allowed, true);
        if (!pending) {
            // A mask wait is still active until the scheduler has interrupted
            // it. Merely observing no pending signal must not undo that mask.
            if (thread.interrupted_mask) {
                resume_interrupted(cpu, thread, true, false);
                finish_mask(thread, mask);
            }
            return {};
        }
        if (!resumed && before_delivery && before_delivery(*pending))
            return {};
        const int signal = pending->info.signal;
        Action action = process.actions->entries.at(signal - 1);
        if (signal == kill_signal || signal == stop_signal)
            action = {};
        if (pending->forced && (action.handler == 1 || (live_mask & bit(signal)))) {
            action = {};
            process.actions->entries[signal - 1] = {};
        }
        if (action.handler == 1)
            continue;
        if (!action.handler) {
            const Effect effect = default_effect(signal);
            if (effect == Effect::None || effect == Effect::Continue)
                continue;
            if (effect == Effect::Stop)
                process.stopped = true;
            return {effect, signal, pending->info};
        }
        if (!(action.flags & sa_restorer) && !trampoline)
            throw Failure("guest signal return trampoline is not installed");
        CpuState saved = cpu;
        if (thread.suspending)
            saved.x[0] = uint64_t(int64_t{-4});
        else
            resume_interrupted(saved, thread, action.flags & sa_restart, true);
        if (before_handler && !before_handler(saved)) {
            if (++bad_frames > 1 || signal == segv_signal)
                return {Effect::Core, segv_signal, Info::fault(segv_signal, saved.pc, 1)};
            force_fault(process, thread, mask, segv_signal, saved.pc, 1);
            continue;
        }
        const uint64_t saved_mask = thread.restore_mask.value_or(mask);
        if (action.flags & sa_resethand)
            process.actions->entries[signal - 1] = {};
        if (!enter_handler(memory, saved, thread, saved_mask, pending->info, action, trampoline)) {
            if (++bad_frames > 1 || signal == segv_signal)
                return {Effect::Core, segv_signal, Info::fault(segv_signal, cpu.sp, 1)};
            force_fault(process, thread, mask, segv_signal, cpu.sp, 1);
            continue;
        }
        cpu = saved;
        mask = (live_mask | action.mask | ((action.flags & sa_nodefer) ? 0 : bit(signal))) & ~unblockable;
        thread.restore_mask.reset();
        thread.interrupted_mask.reset();
        thread.restart.reset();
        thread.wait_set.reset();
        thread.suspending = false;
        return {Effect::Handler, signal, pending->info};
    }
}
} // namespace operit::arm64::signals
