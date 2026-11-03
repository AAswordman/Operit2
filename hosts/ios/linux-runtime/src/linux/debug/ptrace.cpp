#include "linux/debug/ptrace.h"
#include "memory/fault.h"
#include <algorithm>
#include <array>
#include <cstring>
#include <utility>

namespace operit::arm64::debug {
namespace {
constexpr uint64_t traceme = 0, peektext = 1, peekdata = 2, poketext = 4, pokedata = 5;
constexpr uint64_t cont = 7, attach = 16, detach = 17, syscall = 24;
constexpr uint64_t setoptions = 0x4200, geteventmsg = 0x4201, getsiginfo = 0x4202;
constexpr uint64_t setsiginfo = 0x4203, getregset = 0x4204, setregset = 0x4205;
constexpr uint32_t wexited = 4, wnowait = 0x01000000, wnothread = 0x20000000;
constexpr int sigtrap = 5, sigkill = 9, sigstop = 19, sigchld = 17;
constexpr uint64_t clone_ptrace = 0x2000, clone_vfork = 0x4000, clone_untraced = 0x800000;
constexpr uint64_t nt_prstatus = 1, nt_prfpreg = 2;

struct Iovec { uint64_t base, length; };
struct Gpr { uint64_t x[31], sp, pc, pstate; };
struct Fpr {
    std::array<std::array<uint8_t, 16>, 32> v;
    uint32_t fpsr, fpcr, reserved[2];
};
static_assert(sizeof(Iovec) == 16 && sizeof(Gpr) == 272 && sizeof(Fpr) == 528);

bool permitted(const Target &from, const Target &to) {
    return from.cap_sys_ptrace ||
           (to.dumpable && from.ruid == to.ruid && from.ruid == to.euid && from.ruid == to.suid &&
            from.rgid == to.rgid && from.rgid == to.egid && from.rgid == to.sgid &&
            !(to.permitted_capabilities & ~from.permitted_capabilities));
}

Gpr get_gpr(const CpuState &cpu) {
    Gpr regs{};
    std::copy(cpu.x.begin(), cpu.x.end(), regs.x);
    regs.sp = cpu.sp;
    regs.pc = cpu.pc;
    regs.pstate = uint64_t(cpu.nzcv & 15) << 28;
    return regs;
}
Fpr get_fpr(const CpuState &cpu) {
    Fpr regs{};
    regs.v = cpu.v;
    regs.fpsr = uint32_t(cpu.fpsr);
    regs.fpcr = uint32_t(cpu.fpcr);
    return regs;
}
} // namespace

Ptrace::Ptrace(Hooks hooks) : hooks_(std::move(hooks)) {}

bool Ptrace::traced(uint32_t tid) const noexcept {
    const auto found = tracees_.find(tid);
    return found != tracees_.end() && !found->second.dead;
}

bool Ptrace::wants_exit_event(uint32_t tid) const noexcept {
    const auto found = tracees_.find(tid);
    return found != tracees_.end() && !found->second.dead && (found->second.options & o_traceexit);
}

std::optional<Target> Ptrace::live(uint32_t tid) const {
    auto target = hooks_.target ? hooks_.target(tid) : std::nullopt;
    if (!target || !target->alive || !target->cpu)
        return {};
    return target;
}

Ptrace::Tracee *Ptrace::owned(uint32_t caller_tid, uint32_t tid, bool require_stop) {
    const auto it = tracees_.find(tid);
    if (it == tracees_.end() || it->second.tracer_tid != caller_tid || it->second.dead ||
        (require_stop && !it->second.stop))
        return nullptr;
    return &it->second;
}

void Ptrace::stop(uint32_t tid, Event event) {
    auto &tracee = tracees_.at(tid);
    if (const auto target = live(tid))
        tracee.uid = target->ruid;
    event.info = event.kind == Stop::Signal && tracee.pending_signal
                     ? tracee.pending_signal->info
                     : signals::Info::user(sigtrap, tid, tracee.uid, event.status >> 8);
    tracee.stop = event;
    hooks_.set_stopped(tid, true);
    hooks_.notify(tracee.tracer_tid, tid, event.status);
}

void Ptrace::unstop(uint32_t tid, Tracee &tracee) {
    if (tracee.saved_syscall_x7) {
        if (const auto target = live(tid))
            target->cpu->x[7] = *tracee.saved_syscall_x7;
        tracee.saved_syscall_x7.reset();
    }
    if (tracee.saved_syscall_result) {
        if (const auto target = live(tid))
            target->cpu->x[0] = *tracee.saved_syscall_result;
        tracee.saved_syscall_result.reset();
    }
    tracee.event_syscall_x0.reset();
    tracee.stop.reset();
    hooks_.set_stopped(tid, false);
}

int64_t Ptrace::regset(uint32_t caller_tid, const Target &target, bool write,
                       uint64_t type, uint64_t iovec) {
    Iovec iov{};
    if (!hooks_.memory(caller_tid, iovec, &iov, sizeof(iov), Access::Read))
        return -14;
    if (type != nt_prstatus && type != nt_prfpreg)
        return -22;
    const size_t size = type == nt_prstatus ? sizeof(Gpr) : sizeof(Fpr);
    const size_t alignment = type == nt_prstatus ? sizeof(uint64_t) : sizeof(uint32_t);
    if (iov.length % alignment)
        return -22;
    const size_t count = size_t(std::min<uint64_t>(iov.length, size));
    std::array<uint8_t, sizeof(Fpr)> bytes{};
    if (type == nt_prstatus) {
        const auto regs = get_gpr(*target.cpu);
        std::memcpy(bytes.data(), &regs, sizeof(regs));
    } else {
        const auto regs = get_fpr(*target.cpu);
        std::memcpy(bytes.data(), &regs, sizeof(regs));
    }
    if (count && !hooks_.memory(caller_tid, iov.base, bytes.data(), count,
                                write ? Access::Read : Access::Write))
        return -14;
    if (write) {
        if (type == nt_prstatus) {
            Gpr regs{};
            std::memcpy(&regs, bytes.data(), sizeof(regs));
            // The runtime only models the condition flags, not EL0 PSTATE
            // extensions. Reject values we cannot faithfully represent.
            if (regs.pstate & ~(uint64_t{15} << 28))
                return -22;
            std::copy(std::begin(regs.x), std::end(regs.x), target.cpu->x.begin());
            target.cpu->sp = regs.sp;
            target.cpu->pc = regs.pc;
            target.cpu->nzcv = uint8_t(regs.pstate >> 28);
            target.cpu->exclusive_valid = false;
        } else {
            Fpr regs{};
            std::memcpy(&regs, bytes.data(), sizeof(regs));
            target.cpu->v = regs.v;
            target.cpu->fpsr = regs.fpsr;
            target.cpu->fpcr = regs.fpcr;
        }
    }
    iov.length = count;
    // Linux updates only iov_len after the transfer, not the whole iovec.
    return hooks_.memory(caller_tid, iovec + offsetof(Iovec, length), &iov.length,
                         sizeof(iov.length), Access::Write) ? 0 : -14;
}

int64_t Ptrace::request(uint32_t caller_tid, uint64_t request, int64_t pid,
                        uint64_t addr, uint64_t data) {
    if (!hooks_.target || !hooks_.memory || !hooks_.set_stopped || !hooks_.resume_signal ||
        !hooks_.queue_signal || !hooks_.notify || !hooks_.release_exit)
        return -38;
    try {
        const auto caller = live(caller_tid);
        if (!caller)
            return -3;
        if (request == traceme) {
            if (tracees_.count(caller_tid))
                return -1;
            const auto parent = live(caller->parent_tid);
            if (!parent || parent->tgid == caller->tgid)
                return -1;
            if (!parent->cap_sys_ptrace &&
                (caller->permitted_capabilities & ~parent->permitted_capabilities))
                return -1;
            Tracee tracee;
            tracee.tracer_tid = parent->tid;
            tracee.tracer_tgid = parent->tgid;
            tracee.tgid = caller->tgid;
            tracee.pgrp = caller->pgrp;
            tracee.uid = caller->ruid;
            tracee.force_capable = caller->cap_sys_ptrace;
            tracees_.emplace(caller_tid, tracee);
            return 0;
        }
        if (pid <= 0 || uint64_t(pid) > UINT32_MAX)
            return -3;
        const auto tid = uint32_t(pid);
        const auto target = live(tid);
        if (!target)
            return -3;
        if (request == attach) {
            if (caller->tgid == target->tgid || tracees_.count(tid) || !permitted(*caller, *target))
                return -1;
            if (!hooks_.attach_stop)
                return -95;
            Tracee tracee;
            tracee.tracer_tid = caller_tid;
            tracee.tracer_tgid = caller->tgid;
            tracee.tgid = target->tgid;
            tracee.pgrp = target->pgrp;
            tracee.uid = target->ruid;
            tracee.force_capable = caller->cap_sys_ptrace;
            tracees_.emplace(tid, tracee);
            if (!hooks_.attach_stop(tid)) {
                tracees_.erase(tid);
                return -95;
            }
            return 0;
        }
        auto *tracee = owned(caller_tid, tid, true);
        if (!tracee)
            return -3;
        switch (request) {
        case setoptions:
            if (data & ~(uint64_t{o_tracesysgood | o_tracefork | o_tracevfork | o_traceclone |
                                  o_traceexec | o_tracevforkdone | o_traceexit}))
                return -22;
            tracee->options = uint32_t(data);
            return 0;
        case getregset:
        case setregset:
            return regset(caller_tid, *target, request == setregset, addr, data);
        case peektext:
        case peekdata: {
            if (!target->dumpable && !tracee->force_capable)
                return -5;
            uint64_t word = 0;
            if (!hooks_.memory(tid, addr, &word, sizeof(word), Access::ForceRead))
                return -5;
            return hooks_.memory(caller_tid, data, &word, sizeof(word), Access::Write) ? 0 : -14;
        }
        case poketext:
        case pokedata:
            if (!target->dumpable && !tracee->force_capable)
                return -5;
            return hooks_.memory(tid, addr, &data, sizeof(data), Access::ForceWrite) ? 0 : -5;
        case getsiginfo:
            return hooks_.memory(caller_tid, data, &tracee->stop->info,
                                 sizeof(signals::Info), Access::Write) ? 0 : -14;
        case setsiginfo: {
            signals::Info info{};
            if (!hooks_.memory(caller_tid, data, &info, sizeof(info), Access::Read))
                return -14;
            tracee->stop->info = info;
            if (tracee->pending_signal)
                tracee->pending_signal->info = info;
            return 0;
        }
        case geteventmsg: {
            auto message = tracee->stop->message;
            return hooks_.memory(caller_tid, data, &message, sizeof(message), Access::Write) ? 0 : -14;
        }
        case cont:
        case syscall:
        case detach: {
            if (data > signals::maximum_signal)
                return -5;
            if (tracee->pending_signal && !hooks_.resume_signal)
                return -95;
            auto pending = std::move(tracee->pending_signal);
            tracee->pending_signal.reset();
            tracee->syscall_trace = request == syscall;
            if (pending) {
                if (!data) {
                    pending.reset();
                } else if (data != uint64_t(pending->info.signal)) {
                    pending->info = signals::Info::user(int(data), caller->tgid, caller->ruid);
                    pending->forced = false;
                }
                hooks_.resume_signal(tid, std::move(pending));
            } else if (data && (tracee->stop->kind == Stop::Entry || tracee->stop->kind == Stop::Exit)) {
                signals::Queued queued{};
                queued.info = signals::Info::user(int(data), 0, 0, 128);
                hooks_.queue_signal(tid, std::move(queued));
            }
            const auto pending_vfork_done = tracee->pending_vfork_done;
            tracee->pending_vfork_done.reset();
            unstop(tid, *tracee);
            if (request == detach) {
                tracees_.erase(tid);
            } else if (pending_vfork_done) {
                if (tracee->options & o_tracevforkdone)
                    stop_vfork_done(tid, *pending_vfork_done);
                else
                    tracee->vfork_syscall_x0.reset();
            }
            return 0;
        }
        default:
            // Includes SEIZE, INTERRUPT, SINGLESTEP, hardware debug regsets,
            // and unsupported architecture requests.
            return -5;
        }
    } catch (const MemoryFault &) {
        return -14;
    } catch (const GuestError &error) {
        return -error.number;
    }
}

std::optional<WaitResult> Ptrace::wait(uint32_t caller_tid, const WaitRequest &request,
                                       bool &eligible) {
    eligible = false;
    const auto caller = live(caller_tid);
    if (!caller)
        return {};
    for (auto it = tracees_.begin(); it != tracees_.end(); ++it) {
        const auto tid = it->first;
        auto &tracee = it->second;
        if (const auto target = live(tid)) {
            tracee.pgrp = target->pgrp;
            tracee.uid = target->ruid;
        }
        if (tracee.tracer_tgid != caller->tgid ||
            ((request.options & wnothread) && tracee.tracer_tid != caller_tid) ||
            (request.kind == WaitRequest::Kind::Tid && request.id != tid) ||
            (request.kind == WaitRequest::Kind::Group && request.id != tracee.pgrp))
            continue;
        eligible = true;
        WaitResult result{tid, tracee.uid, 0, tracee.dead};
        if (tracee.dead && (request.options & wexited)) {
            result.status = tracee.exit_status;
            if (!(request.options & wnowait))
                tracees_.erase(it);
            return result;
        }
        if (!tracee.stop || tracee.stop->reported)
            continue;
        result.status = tracee.stop->status;
        if (!(request.options & wnowait))
            tracee.stop->reported = true;
        return result;
    }
    return {};
}

bool Ptrace::syscall_entry(uint32_t tid) {
    auto it = tracees_.find(tid);
    if (it == tracees_.end() || it->second.dead || it->second.stop || !it->second.syscall_trace)
        return false;
    const auto target = live(tid);
    if (!target)
        return false;
    it->second.saved_syscall_x7 = target->cpu->x[7];
    target->cpu->x[7] = 0;
    const int signal = sigtrap | ((it->second.options & o_tracesysgood) ? 0x80 : 0);
    stop(tid, {Stop::Entry, (signal << 8) | 0x7f, 1, false});
    return true;
}

bool Ptrace::syscall_exit(uint32_t tid) {
    auto it = tracees_.find(tid);
    if (it == tracees_.end() || it->second.dead || it->second.stop || !it->second.syscall_trace)
        return false;
    const auto target = live(tid);
    if (!target)
        return false;
    it->second.saved_syscall_x7 = target->cpu->x[7];
    target->cpu->x[7] = 1;
    const int signal = sigtrap | ((it->second.options & o_tracesysgood) ? 0x80 : 0);
    stop(tid, {Stop::Exit, (signal << 8) | 0x7f, 2, false});
    return true;
}

bool Ptrace::signal_delivery_stop(uint32_t tid, signals::Queued queued) {
    auto it = tracees_.find(tid);
    if (it == tracees_.end() || it->second.dead || it->second.stop || queued.info.signal == sigkill)
        return false;
    const int signal = queued.info.signal;
    it->second.pending_signal = std::move(queued);
    stop(tid, {Stop::Signal, (signal << 8) | 0x7f, 0, false});
    return true;
}

void Ptrace::fatal_signal(uint32_t tid) {
    auto it = tracees_.find(tid);
    if (it == tracees_.end() || !it->second.stop)
        return;
    it->second.pending_signal.reset();
    it->second.pending_vfork_done.reset();
    it->second.vfork_syscall_x0.reset();
    unstop(tid, it->second);
}

void Ptrace::clone(uint32_t parent_tid, uint32_t child_tid, uint64_t flags, uint32_t exit_signal) {
    const auto it = tracees_.find(parent_tid);
    if (it == tracees_.end() || it->second.dead)
        return;
    auto &parent = it->second;
    if (flags & clone_vfork)
        if (const auto target = live(parent_tid))
            parent.vfork_syscall_x0 = target->cpu->x[0];
    const int event = (flags & clone_vfork) ? 2 : exit_signal != sigchld ? 3 : 1;
    const bool report = !(flags & clone_untraced) && (parent.options & (1u << event));
    if (!report && !(flags & clone_ptrace))
        return;
    const auto child = live(child_tid);
    if (!child || tracees_.count(child_tid))
        throw Failure("invalid ptrace child during clone");
    Tracee tracee;
    tracee.tracer_tid = parent.tracer_tid;
    tracee.tracer_tgid = parent.tracer_tgid;
    tracee.tgid = child->tgid;
    tracee.pgrp = child->pgrp;
    tracee.uid = child->ruid;
    tracee.options = parent.options;
    tracee.force_capable = parent.force_capable;
    tracees_.emplace(child_tid, std::move(tracee));
    signals::Queued queued{};
    queued.info = signals::Info::user(sigstop, 0, 0);
    hooks_.queue_signal(child_tid, std::move(queued));
    if (report) {
        if (const auto target = live(parent_tid))
            parent.event_syscall_x0 = target->cpu->x[0];
        stop(parent_tid, {Stop::Fork, (event << 16) | (sigtrap << 8) | 0x7f, child_tid, false});
    }
}

void Ptrace::preserve_event_syscall_result(uint32_t tid) {
    const auto it = tracees_.find(tid);
    if (it == tracees_.end() || !it->second.event_syscall_x0)
        return;
    if (const auto target = live(tid)) {
        it->second.saved_syscall_result = target->cpu->x[0];
        target->cpu->x[0] = *it->second.event_syscall_x0;
    }
}

void Ptrace::stop_vfork_done(uint32_t tid, uint32_t child_tid) {
    auto &parent = tracees_.at(tid);
    parent.event_syscall_x0 = parent.vfork_syscall_x0;
    parent.vfork_syscall_x0.reset();
    preserve_event_syscall_result(tid);
    stop(tid, {Stop::VforkDone, (5 << 16) | (sigtrap << 8) | 0x7f, child_tid, false});
}

void Ptrace::vfork_done(uint32_t parent_tid, uint32_t child_tid) {
    const auto it = tracees_.find(parent_tid);
    if (it == tracees_.end() || it->second.dead)
        return;
    auto &parent = it->second;
    if (parent.stop) {
        parent.pending_vfork_done = child_tid;
    } else if (parent.options & o_tracevforkdone) {
        stop_vfork_done(parent_tid, child_tid);
    } else {
        parent.vfork_syscall_x0.reset();
    }
}

bool Ptrace::can_exec(uint32_t old_tid, uint32_t new_tid) const {
    return old_tid == new_tid || !tracees_.count(new_tid);
}

void Ptrace::exec(uint32_t old_tid, uint32_t new_tid) {
    if (!can_exec(old_tid, new_tid))
        throw Failure("unsupported ptrace state during non-leader exec");
    if (old_tid != new_tid)
        for (auto &[tid, tracee] : tracees_) {
            (void)tid;
            if (tracee.tracer_tid == old_tid)
                tracee.tracer_tid = new_tid;
        }
    auto it = tracees_.find(old_tid);
    if (it == tracees_.end())
        return;
    if (old_tid != new_tid) {
        Tracee moved = std::move(it->second);
        tracees_.erase(it);
        it = tracees_.emplace(new_tid, std::move(moved)).first;
    }
    if (const auto target = live(new_tid)) {
        it->second.tgid = target->tgid;
        it->second.uid = target->ruid;
    }
    if (it->second.options & o_traceexec) {
        stop(new_tid, {Stop::Exec, (4 << 16) | (sigtrap << 8) | 0x7f, old_tid, false});
    } else {
        signals::Queued queued{};
        queued.info = signals::Info::user(sigtrap, it->second.tgid, it->second.uid);
        hooks_.queue_signal(new_tid, std::move(queued));
    }
}

bool Ptrace::exit_event(uint32_t tid, int encoded_status) {
    const auto it = tracees_.find(tid);
    if (it == tracees_.end() || it->second.dead || it->second.stop ||
        !(it->second.options & o_traceexit))
        return false;
    stop(tid, {Stop::EventExit, (6 << 16) | (sigtrap << 8) | 0x7f,
               uint32_t(encoded_status), false});
    return true;
}

bool Ptrace::task_exit(uint32_t tid, int status) {
    auto it = tracees_.find(tid);
    if (it == tracees_.end())
        return false;
    auto &tracee = it->second;
    if (tracee.stop)
        unstop(tid, tracee);
    tracee.pending_signal.reset();
    tracee.dead = true;
    tracee.exit_status = status;
    hooks_.notify(tracee.tracer_tid, tid, status);
    return true;
}

void Ptrace::tracer_exit(uint32_t tid) {
    for (auto it = tracees_.begin(); it != tracees_.end();) {
        auto &tracee = it->second;
        if (tracee.tracer_tid != tid) {
            ++it;
            continue;
        }
        if (tracee.dead)
            hooks_.release_exit(it->first);
        if (tracee.stop && !tracee.dead) {
            if (tracee.pending_signal && hooks_.resume_signal)
                hooks_.resume_signal(it->first, std::move(tracee.pending_signal));
            unstop(it->first, tracee);
        }
        it = tracees_.erase(it);
    }
}

} // namespace operit::arm64::debug
