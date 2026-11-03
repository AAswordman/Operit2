#include "linux/debug/ptrace.h"
#include <array>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>
#include <vector>

using namespace operit::arm64;
using namespace operit::arm64::debug;

namespace {
void check(bool condition, const char *message) {
    if (!condition) {
        std::fprintf(stderr, "ptrace regression failed: %s\n", message);
        std::exit(1);
    }
}
struct Iov { uint64_t base, len; };
struct Regs { uint64_t x[31], sp, pc, pstate; };
struct Fpregs { uint8_t v[512]; uint32_t fpsr, fpcr, reserved[2]; };
static_assert(sizeof(Regs) == 272 && sizeof(Fpregs) == 528);

struct Fixture {
    struct Notice { uint32_t tracer, tracee; int status; };
    struct Entry {
        Target target;
        CpuState cpu;
        std::array<uint8_t, 4096> memory{};
        bool stopped = false;
    };
    std::map<uint32_t, Entry> tasks;
    std::vector<Notice> notices;
    std::vector<std::pair<uint32_t, std::optional<signals::Queued>>> signals;
    std::vector<std::pair<uint32_t, signals::Queued>> queued_signals;
    std::vector<uint32_t> released;
    unsigned attaches = 0, force_reads = 0, force_writes = 0;
    bool attach_ok = true, force_ok = true;
    Ptrace ptrace;

    Fixture() : ptrace(hooks()) {
        add(1, 1, 0);
        add(2, 2, 1);
        add(3, 3, 1);
    }
    void add(uint32_t tid, uint32_t tgid, uint32_t parent) {
        auto &entry = tasks[tid];
        entry.target.tid = tid;
        entry.target.tgid = tgid;
        entry.target.parent_tid = parent;
        entry.target.pgrp = 1;
        entry.target.ruid = entry.target.euid = entry.target.suid = 1000;
        entry.target.rgid = entry.target.egid = entry.target.sgid = 1000;
        entry.target.alive = true;
        entry.target.cpu = &entry.cpu;
    }
    Hooks hooks() {
        Hooks result;
        result.target = [this](uint32_t tid) -> std::optional<Target> {
            const auto it = tasks.find(tid);
            return it == tasks.end() ? std::nullopt : std::optional<Target>(it->second.target);
        };
        result.memory = [this](uint32_t tid, uint64_t address, void *bytes, size_t size, Access mode) {
            const auto it = tasks.find(tid);
            if (it == tasks.end() || address > it->second.memory.size() ||
                size > it->second.memory.size() - address)
                return false;
            if (mode == Access::ForceRead) {
                ++force_reads;
                if (!force_ok)
                    return false;
            }
            if (mode == Access::ForceWrite) {
                ++force_writes;
                if (!force_ok)
                    return false;
            }
            auto *remote = it->second.memory.data() + address;
            if (mode == Access::Write || mode == Access::ForceWrite)
                std::memcpy(remote, bytes, size);
            else
                std::memcpy(bytes, remote, size);
            return true;
        };
        result.set_stopped = [this](uint32_t tid, bool stopped) { tasks.at(tid).stopped = stopped; };
        result.attach_stop = [this](uint32_t) { ++attaches; return attach_ok; };
        result.resume_signal = [this](uint32_t tid, std::optional<signals::Queued> queued) {
            signals.emplace_back(tid, std::move(queued));
        };
        result.queue_signal = [this](uint32_t tid, signals::Queued queued) {
            queued_signals.emplace_back(tid, std::move(queued));
        };
        result.notify = [this](uint32_t tracer, uint32_t tracee, int status) {
            notices.push_back({tracer, tracee, status});
        };
        result.release_exit = [this](uint32_t tid) { released.push_back(tid); };
        return result;
    }
    template <typename T> void put(uint32_t tid, uint64_t address, const T &value) {
        std::memcpy(tasks.at(tid).memory.data() + address, &value, sizeof(value));
    }
    template <typename T> T get(uint32_t tid, uint64_t address) {
        T value{};
        std::memcpy(&value, tasks.at(tid).memory.data() + address, sizeof(value));
        return value;
    }
    void signal_stop(uint32_t tid, int number) {
        signals::Queued queued{};
        queued.info.signal = number;
        check(ptrace.signal_delivery_stop(tid, queued), "signal stop");
    }
    WaitResult wait(uint32_t tid, uint32_t options = 0) {
        bool eligible = false;
        auto value = ptrace.wait(1, {WaitRequest::Kind::Tid, tid, options}, eligible);
        check(eligible && value.has_value(), "wait event");
        return *value;
    }
};

void tracing_and_registers() {
    Fixture f;
    check(f.ptrace.request(1, 16, 2, 0, 0) == 0 && f.attaches == 1 && !f.tasks[2].stopped,
          "attach queues SIGSTOP instead of inventing a synchronous stop");
    check(f.ptrace.request(3, 16, 2, 0, 0) == -1, "double attach");
    check(f.ptrace.request(1, 0x4204, 2, 1, 32) == -3, "register access while running");
    f.signal_stop(2, 19);
    const auto observed = f.wait(2, 0x01000000);
    check(observed.status == ((19 << 8) | 0x7f), "attach wait status");
    check(f.wait(2).status == observed.status, "WNOWAIT preserves event");
    bool eligible = false;
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 0}, eligible) && eligible,
          "consumed stop remains attached and waitable");
    check(f.ptrace.request(3, 7, 2, 0, 0) == -3, "foreign tracer denied");

    auto &cpu = f.tasks[2].cpu;
    cpu.x[0] = 0x123456789abcdef0ULL;
    cpu.x[30] = 0x778899;
    cpu.sp = 0x4000;
    cpu.pc = 0x5000;
    cpu.nzcv = 10;
    f.put(1, 32, Iov{128, sizeof(Regs)});
    check(f.ptrace.request(1, 0x4204, 2, 1, 32) == 0, "GETREGSET GPR");
    auto regs = f.get<Regs>(1, 128);
    check(regs.x[0] == cpu.x[0] && regs.x[30] == cpu.x[30] && regs.sp == cpu.sp &&
              regs.pc == cpu.pc && regs.pstate == 0xa0000000,
          "AArch64 register layout");
    regs.x[0] = 42;
    regs.pc += 4;
    regs.pstate = 0x50000000;
    f.put(1, 128, regs);
    check(f.ptrace.request(1, 0x4205, 2, 1, 32) == 0 && cpu.x[0] == 42 && cpu.pc == 0x5004 &&
              cpu.nzcv == 5,
          "SETREGSET GPR");
    regs.pstate = 1;
    f.put(1, 128, regs);
    check(f.ptrace.request(1, 0x4205, 2, 1, 32) == -22 && cpu.pc == 0x5004,
          "invalid pstate leaves registers unchanged");
    f.put(1, 32, Iov{128, 15});
    check(f.ptrace.request(1, 0x4204, 2, 1, 32) == -22, "GPR iovec alignment");
    f.put(1, 32, Iov{128, 8});
    check(f.ptrace.request(1, 0x4204, 2, 1, 32) == 0 && f.get<Iov>(1, 32).len == 8 &&
              f.get<uint64_t>(1, 128) == 42,
          "short GPR transfer");
    cpu.v[3][4] = 0xa7;
    cpu.fpsr = 0x123;
    cpu.fpcr = 0x456;
    f.put(1, 32, Iov{512, sizeof(Fpregs)});
    check(f.ptrace.request(1, 0x4204, 2, 2, 32) == 0, "GETREGSET FPSIMD");
    auto fp = f.get<Fpregs>(1, 512);
    check(fp.v[3 * 16 + 4] == 0xa7 && fp.fpsr == 0x123 && fp.fpcr == 0x456, "FPSIMD layout");
    fp.v[0] = 7;
    f.put(1, 512, fp);
    check(f.ptrace.request(1, 0x4205, 2, 2, 32) == 0 && cpu.v[0][0] == 7, "SETREGSET FPSIMD");
    check(f.ptrace.request(1, 0x4204, 2, 0x999, 32) == -22, "unsupported regset");

    f.put(2, 100, uint64_t{0xffffffffffffffffULL});
    check(f.ptrace.request(1, 2, 2, 100, 64) == 0 && f.get<uint64_t>(1, 64) == UINT64_MAX &&
              f.force_reads == 1,
          "raw PEEK writes word to tracer data pointer");
    check(f.ptrace.request(1, 5, 2, 100, 77) == 0 && f.get<uint64_t>(2, 100) == 77 &&
              f.force_writes == 1,
          "POKE uses force write");
    check(f.ptrace.request(1, 2, 2, 100, 4092) == -14, "PEEK invalid tracer address");
    f.force_ok = false;
    check(f.ptrace.request(1, 5, 2, 100, 1) == -5 && f.get<uint64_t>(2, 100) == 77,
          "failed force write");
    f.force_ok = true;
    check(f.ptrace.request(1, 9, 2, 0, 0) == -5, "unsupported SINGLESTEP");
    check(f.ptrace.request(1, 0x4200, 2, 0, Ptrace::o_tracesysgood) == 0, "TRACESYSGOOD");
    check(f.ptrace.request(1, 24, 2, 0, 0) == 0 && !f.tasks[2].stopped &&
              !f.signals.empty() && !f.signals.back().second,
          "SYSCALL resumes and suppresses attach signal");
    cpu.x[7] = 0x1234;
    check(f.ptrace.syscall_entry(2), "syscall entry stop");
    check(cpu.x[7] == 0, "AArch64 syscall entry exposes x7=0");
    check(f.wait(2).status == (((5 | 0x80) << 8) | 0x7f), "syscall entry status");
    check(f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 1,
          "syscall entry event message");
    check(f.ptrace.request(1, 0x4202, 2, 0, 1024) == 0, "syscall GETSIGINFO");
    const auto syscall_info = f.get<signals::Info>(1, 1024);
    check(syscall_info.signal == 5 && syscall_info.code == (5 | 0x80) &&
              syscall_info.sender_pid() == 2 && syscall_info.sender_uid() == 1000,
          "syscall siginfo reports trap code and tracee identity");
    cpu.x[7] = 0x5678;
    check(f.ptrace.request(1, 24, 2, 0, 10) == 0 && cpu.x[7] == 0x1234 &&
              f.queued_signals.size() == 1 && f.queued_signals.back().second.info.signal == 10 &&
              f.queued_signals.back().second.info.code == 128,
          "syscall resume restores x7 and queues requested signal");
    check(f.ptrace.syscall_exit(2), "syscall exit stop");
    check(cpu.x[7] == 1, "AArch64 syscall exit exposes x7=1");
    check(f.wait(2).status == (((5 | 0x80) << 8) | 0x7f), "syscall exit status");
    check(f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 2,
          "syscall exit event message");
    cpu.x[7] = 0xabcd;
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0 && cpu.x[7] == 0x1234 && !f.ptrace.syscall_entry(2),
          "CONT restores x7 and disables tracing");
    f.signal_stop(2, 10);
    auto original = f.wait(2);
    check(original.status == ((10 << 8) | 0x7f), "ordinary signal stop");
    check(f.ptrace.request(1, 0x4202, 2, 0, 1024) == 0 &&
              f.get<signals::Info>(1, 1024).signal == 10,
          "GETSIGINFO after consuming wait event");
    auto edited_info = f.get<signals::Info>(1, 1024);
    edited_info.error = 123;
    f.put(1, 1024, edited_info);
    check(f.ptrace.request(1, 0x4203, 2, 0, 1024) == 0 &&
              f.ptrace.request(1, 0x4202, 2, 0, 1024) == 0 &&
              f.get<signals::Info>(1, 1024).error == 123,
          "SETSIGINFO updates a pending signal");
    check(f.ptrace.request(1, 17, 2, 0, 12) == 0 && !f.tasks[2].stopped &&
              f.signals.back().second && f.signals.back().second->info.signal == 12,
          "DETACH with replacement signal");
    check(f.ptrace.request(1, 7, 2, 0, 0) == -3, "detached target no longer controlled");
}

void permissions_and_lifecycle() {
    Fixture f;
    f.tasks[2].target.euid = 1001;
    check(f.ptrace.request(1, 16, 2, 0, 0) == -1, "all target user IDs checked");
    f.tasks[2].target.euid = 1000;
    f.tasks[2].target.permitted_capabilities = 4;
    check(f.ptrace.request(1, 16, 2, 0, 0) == -1, "target capabilities checked");
    f.tasks[1].target.cap_sys_ptrace = true;
    f.attach_ok = false;
    check(f.ptrace.request(1, 16, 2, 0, 0) == -95, "failed attach rolls back ownership");
    f.attach_ok = true;
    check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "guest capability permits attach");
    f.signal_stop(2, 19);
    (void)f.wait(2);
    check(f.ptrace.request(1, 0x4200, 2, 0, Ptrace::o_traceexec) == 0, "TRACEEXEC option");
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0, "continue into exec");
    f.ptrace.exec(2, 2);
    check(f.wait(2).status == ((4 << 16) | (5 << 8) | 0x7f), "TRACEEXEC event");
    check(f.ptrace.request(1, 0x4202, 2, 0, 1024) == 0 &&
              f.get<signals::Info>(1, 1024).code == ((4 << 8) | 5),
          "exec event siginfo");
    check(f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 2,
          "exec event message survives wait");
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0, "continue after exec");
    check(f.ptrace.task_exit(2, 42 << 8), "traced exit interception");
    f.tasks[2].target.alive = false;
    auto status = f.wait(2, 0x01000000 | 4);
    check(status.exited && status.status == (42 << 8), "exit WNOWAIT");
    check(f.wait(2, 4).status == status.status, "exit wait after task retirement");
    bool eligible = true;
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 0}, eligible) && !eligible,
          "consumed exit unlinks tracee");

    check(f.ptrace.request(3, 0, 0, 0, 0) == 0, "TRACEME");
    f.signal_stop(3, 10);
    (void)f.wait(3);
    f.ptrace.tracer_exit(1);
    check(!f.tasks[3].stopped && f.signals.back().second && f.signals.back().second->info.signal == 10,
          "tracer exit resumes pending signal");
}

void exit_event_lifecycle() {
    constexpr int exit_status = 42 << 8;
    constexpr int event_status = (6 << 16) | (5 << 8) | 0x7f;
    constexpr uint32_t wnowait = 0x01000000;
    static_assert(Ptrace::o_traceexit == 0x40);
    Fixture f;
    check(!f.ptrace.wants_exit_event(2) && !f.ptrace.exit_event(2, exit_status) && f.notices.empty(),
          "untraced task does not request or emit an exit event");
    check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for exit event");
    signals::Queued queued{};
    queued.info = signals::Info::user(10, 222, 333, -1);
    queued.forced = true;
    check(f.ptrace.signal_delivery_stop(2, queued), "signal stop before exit event");
    check(f.ptrace.request(1, 0x4200, 2, 0, 0x51) == 0 && f.ptrace.wants_exit_event(2),
          "combined TRACESYSGOOD, TRACEEXEC and TRACEEXIT options");
    const auto signal_notices = f.notices.size();
    check(!f.ptrace.exit_event(2, exit_status) && f.notices.size() == signal_notices &&
              f.ptrace.wants_exit_event(2),
          "existing signal stop blocks the event but not the option query");
    check(f.wait(2).status == ((10 << 8) | 0x7f) &&
              f.ptrace.request(1, 0x4202, 2, 0, 1024) == 0 &&
              f.get<signals::Info>(1, 1024).sender_pid() == 222,
          "blocked exit event preserves the original signal stop");
    check(f.ptrace.request(1, 0x4200, 2, 0, 0xd1) == -22 &&
              f.ptrace.request(1, 0x4200, 2, 0, uint64_t{1} << 32) == -22 &&
              f.ptrace.wants_exit_event(2),
          "unknown option bits are rejected without changing options");
    check(f.ptrace.request(1, 24, 2, 0, 10) == 0 && f.signals.size() == 1 &&
              f.signals.back().second && f.signals.back().second->forced &&
              f.signals.back().second->info.signal == 10 &&
              f.signals.back().second->info.sender_pid() == 222 &&
              f.signals.back().second->info.sender_uid() == 333 &&
              f.signals.back().second->info.code == -1,
          "signal resume retains the intercepted signal and forced flag");

    auto &cpu = f.tasks[2].cpu;
    cpu.x[7] = 0x1234;
    check(f.ptrace.syscall_entry(2) && cpu.x[7] == 0, "syscall stop before exit event");
    const auto syscall_notices = f.notices.size();
    check(!f.ptrace.exit_event(2, exit_status) && f.notices.size() == syscall_notices &&
              cpu.x[7] == 0 && f.ptrace.wants_exit_event(2),
          "existing syscall stop is not overwritten by exit event");
    check(f.wait(2).status == (((5 | 0x80) << 8) | 0x7f) &&
              f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 1,
          "TRACESYSGOOD and syscall event message survive rejected options");
    check(f.ptrace.request(1, 24, 2, 0, 0) == 0 && cpu.x[7] == 0x1234,
          "syscall resume restores x7 before exit event");
    check(f.ptrace.syscall_exit(2) && cpu.x[7] == 1 && !f.ptrace.exit_event(2, exit_status),
          "existing syscall exit stop is not overwritten by exit event");
    check(f.wait(2).status == (((5 | 0x80) << 8) | 0x7f) &&
              f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 2 &&
              f.ptrace.request(1, 24, 2, 0, 0) == 0 && cpu.x[7] == 0x1234,
          "syscall exit message and x7 restoration survive the rejected event");
    f.ptrace.exec(2, 2);
    check(f.ptrace.wants_exit_event(2) &&
              f.wait(2).status == ((4 << 16) | (5 << 8) | 0x7f) &&
              f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 2,
          "TRACEEXEC and its event message survive rejected options");
    check(f.ptrace.request(1, 24, 2, 0, 0) == 0, "resume from exec event");

    const auto previous_notices = f.notices.size();
    const auto previous_signals = f.signals.size();
    const auto previous_queued = f.queued_signals.size();
    check(f.ptrace.exit_event(2, exit_status) && f.tasks[2].stopped && f.ptrace.traced(2) &&
              f.ptrace.wants_exit_event(2) && cpu.x[7] == 0x1234,
          "exit event stops a live tracee without changing x7");
    check(f.notices.size() == previous_notices + 1 && f.notices.back().tracer == 1 &&
              f.notices.back().tracee == 2 && f.notices.back().status == event_status &&
              f.released.empty(),
          "exit event notifies the tracer without releasing a final exit");
    check(!f.ptrace.exit_event(2, 9) && f.notices.size() == previous_notices + 1,
          "repeated exit event while stopped is ignored");
    check(f.ptrace.request(3, 0x4201, 2, 0, 32) == -3 &&
              f.ptrace.request(1, 0x4201, 2, 0, 4092) == -14 &&
              f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == exit_status,
          "GETEVENTMSG returns the anticipated encoded status only to the tracer");
    check(f.ptrace.request(1, 0x4202, 2, 0, 1024) == 0 &&
              f.get<signals::Info>(1, 1024).signal == 5 &&
              f.get<signals::Info>(1, 1024).code == ((6 << 8) | 5) &&
              f.get<signals::Info>(1, 1024).sender_pid() == 2,
          "exit event exposes SIGTRAP event siginfo");
    const auto peek = f.wait(2, wnowait | 4);
    check(!peek.exited && peek.status == event_status &&
              f.wait(2, wnowait | 4).status == event_status,
          "WNOWAIT observes the event stop without consuming it");
    const auto observed = f.wait(2, 4);
    check(!observed.exited && observed.status == event_status && f.ptrace.traced(2) &&
              f.ptrace.wants_exit_event(2) && f.released.empty() &&
              f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == exit_status,
          "consuming the stop is not a final exit and preserves the event message");
    bool eligible = false;
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 4}, eligible) && eligible,
          "no final exit is waitable before task_exit");
    cpu.x[7] = 0x5678;
    check(f.ptrace.request(1, 7, 2, 0, 10) == 0 && !f.tasks[2].stopped && cpu.x[7] == 0x5678 &&
              f.signals.size() == previous_signals && f.queued_signals.size() == previous_queued,
          "exit event resume neither alters x7 nor delivers a signal");
    check(f.ptrace.task_exit(2, exit_status) && !f.ptrace.traced(2) &&
              !f.ptrace.wants_exit_event(2) && !f.ptrace.exit_event(2, exit_status),
          "task_exit separately records the final death");
    f.tasks[2].target.alive = false;
    const auto death = f.wait(2, wnowait | 4);
    check(death.exited && death.status == exit_status && f.wait(2, 4).status == exit_status,
          "final exit remains separately waitable after the event");
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 4}, eligible) && !eligible,
          "final exit is unlinked only after it is consumed");
}

void exit_event_disabled() {
    Fixture f;
    check(f.ptrace.request(1, 16, 2, 0, 0) == 0 && !f.ptrace.wants_exit_event(2),
          "TRACEEXIT is disabled by default");
    check(!f.ptrace.exit_event(2, 17 << 8) && !f.tasks[2].stopped && f.notices.empty(),
          "default options do not generate an exit event");
    f.signal_stop(2, 19);
    (void)f.wait(2);
    check(f.ptrace.request(1, 0x4200, 2, 0, 0x51) == 0 && f.ptrace.wants_exit_event(2) &&
              f.ptrace.request(1, 0x4200, 2, 0, 0x11) == 0 && !f.ptrace.wants_exit_event(2),
          "SETOPTIONS replaces and can clear TRACEEXIT");
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0, "resume without TRACEEXIT");
    const auto notices = f.notices.size();
    check(!f.ptrace.exit_event(2, 17 << 8) && !f.tasks[2].stopped &&
              f.ptrace.traced(2) && f.notices.size() == notices,
          "disabled TRACEEXIT generates no stop or notification");
    bool eligible = false;
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 4}, eligible) && eligible,
          "disabled TRACEEXIT leaves no event to wait for");
    check(f.ptrace.task_exit(2, 17 << 8) && f.wait(2, 4).exited,
          "disabled TRACEEXIT still permits normal final exit reporting");
}

void exit_event_signals_and_cleanup() {
    for (const int exit_status : {9, 11 | 0x80}) {
        Fixture f;
        check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for signal exit event");
        f.signal_stop(2, 19);
        (void)f.wait(2);
        check(f.ptrace.request(1, 0x4200, 2, 0, Ptrace::o_traceexit) == 0 &&
                  f.ptrace.request(1, 7, 2, 0, 0) == 0,
              "enable signal exit event");
        const auto previous_signals = f.signals.size();
        f.tasks[2].cpu.x[7] = 0x9876;
        check(f.ptrace.exit_event(2, exit_status) &&
                  f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 &&
                  f.get<uint64_t>(1, 32) == uint64_t(exit_status),
              "GETEVENTMSG retains signal and core-dump wait status bits");
        check(!f.wait(2).exited, "signal exit event is a stop, not a death");
        if (exit_status == 9)
            f.ptrace.fatal_signal(2);
        else
            check(f.ptrace.request(1, 7, 2, 0, 0) == 0, "resume signal exit event");
        check(!f.tasks[2].stopped && f.tasks[2].cpu.x[7] == 0x9876 &&
                  f.signals.size() == previous_signals && f.queued_signals.empty() &&
                  f.ptrace.wants_exit_event(2),
              "resuming or killing an event stop preserves x7 and does not inject signals");
        check(f.ptrace.task_exit(2, exit_status), "record final signal death");
        const auto death = f.wait(2, 4);
        check(death.exited && death.status == exit_status, "final signal death is separately waitable");
    }

    Fixture changed;
    check(changed.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for changed exit status");
    changed.signal_stop(2, 19);
    (void)changed.wait(2);
    check(changed.ptrace.request(1, 0x4200, 2, 0, Ptrace::o_traceexit) == 0 &&
              changed.ptrace.request(1, 7, 2, 0, 0) == 0 &&
              changed.ptrace.exit_event(2, 42 << 8),
          "stop before a later fatal signal");
    check(!changed.wait(2).exited && changed.ptrace.request(1, 0x4201, 2, 0, 32) == 0 &&
              changed.get<uint64_t>(1, 32) == (42 << 8),
          "event reports the originally anticipated exit status");
    changed.ptrace.fatal_signal(2);
    check(changed.ptrace.task_exit(2, 9), "record changed final exit status");
    const auto changed_death = changed.wait(2, 4);
    check(changed_death.exited && changed_death.status == 9,
          "final death status is independent of the earlier event message");

    for (const bool detach : {false, true}) {
        Fixture f;
        check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for exit event cleanup");
        f.signal_stop(2, 19);
        (void)f.wait(2);
        check(f.ptrace.request(1, 0x4200, 2, 0, Ptrace::o_traceexit) == 0 &&
                  f.ptrace.request(1, 7, 2, 0, 0) == 0 && f.ptrace.exit_event(2, 0),
              "stop before detach or tracer death");
        const auto previous_signals = f.signals.size();
        if (detach)
            check(f.ptrace.request(1, 17, 2, 0, 0) == 0, "detach from exit event");
        else
            f.ptrace.tracer_exit(1);
        check(!f.tasks[2].stopped && !f.ptrace.traced(2) && !f.ptrace.wants_exit_event(2) &&
                  f.released.empty() && f.signals.size() == previous_signals &&
                  !f.ptrace.task_exit(2, 0),
              "event cleanup resumes without releasing or fabricating a final exit");
    }
}

void force_access_permissions() {
    Fixture f;
    check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for dumpable check");
    f.signal_stop(2, 19);
    f.tasks[2].target.dumpable = false;
    f.tasks[1].target.cap_sys_ptrace = true;
    check(f.ptrace.request(1, 2, 2, 100, 64) == -5 && f.force_reads == 0,
          "peek denied after target loses dumpability");
    check(f.ptrace.request(1, 5, 2, 100, 77) == -5 && f.force_writes == 0,
          "poke uses attach-time capability rather than later privilege gain");
    f.tasks[2].target.dumpable = true;
    check(f.ptrace.request(1, 5, 2, 100, 77) == 0, "force access after dumpability restored");

    Fixture privileged;
    privileged.tasks[1].target.cap_sys_ptrace = true;
    check(privileged.ptrace.request(1, 16, 2, 0, 0) == 0, "privileged attach");
    privileged.signal_stop(2, 19);
    privileged.tasks[1].target.cap_sys_ptrace = false;
    privileged.tasks[2].target.dumpable = false;
    check(privileged.ptrace.request(1, 5, 2, 100, 77) == 0,
          "force access retains attach-time capability");
}

void thread_wait_exec_and_exit() {
    Fixture f;
    f.add(4, 1, 0);
    check(f.ptrace.request(4, 16, 2, 0, 0) == 0, "non-leader tracer attaches");
    f.signal_stop(2, 19);
    bool eligible = true;
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 0x20000000}, eligible) && !eligible,
          "__WNOTHREAD excludes a sibling tracer");
    check(f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 0x01000000}, eligible).has_value() && eligible,
          "tracer sibling may observe event with WNOWAIT");
    check(!f.ptrace.can_exec(3, 2), "untraced exec cannot replace traced destination TID");
    check(f.ptrace.can_exec(4, 1), "tracer may exec into untraced leader TID");
    f.ptrace.exec(4, 1);
    check(f.ptrace.request(4, 7, 2, 0, 0) == -3, "old tracer TID loses ownership after exec");
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0, "new tracer TID owns tracee after exec");
    check(f.ptrace.task_exit(2, 17 << 8), "tracee death withheld from parent");
    f.tasks[2].target.alive = false;
    f.ptrace.tracer_exit(1);
    check(f.released.size() == 1 && f.released[0] == 2,
          "tracer death releases unconsumed tracee exit");
    check(!f.ptrace.wait(1, {WaitRequest::Kind::Tid, 2, 4}, eligible) && !eligible,
          "released exit no longer belongs to tracer");

    Fixture g;
    g.add(4, 2, 1);
    check(g.ptrace.request(1, 16, 4, 0, 0) == 0, "attach non-leader tracee");
    g.signal_stop(4, 19);
    (void)g.wait(4);
    check(g.ptrace.request(1, 0x4200, 4, 0, Ptrace::o_traceexec) == 0 &&
              g.ptrace.request(1, 7, 4, 0, 0) == 0,
          "continue non-leader into exec");
    check(g.ptrace.can_exec(4, 2), "untraced destination accepts tracee exec");
    g.ptrace.exec(4, 2);
    check(g.wait(2).status == ((4 << 16) | (5 << 8) | 0x7f) &&
              g.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && g.get<uint64_t>(1, 32) == 4,
          "exec rekeys tracee and preserves former TID in event message");
    g.ptrace.fatal_signal(2);
    check(!g.tasks[2].stopped && g.ptrace.request(1, 7, 2, 0, 0) == -3,
          "fatal signal wakes a ptrace-stop");

    Fixture legacy;
    check(legacy.ptrace.request(1, 16, 2, 0, 0) == 0, "attach legacy exec tracee");
    legacy.signal_stop(2, 19);
    (void)legacy.wait(2);
    check(legacy.ptrace.request(1, 7, 2, 0, 0) == 0, "continue into legacy exec");
    legacy.ptrace.exec(2, 2);
    check(!legacy.tasks[2].stopped && legacy.queued_signals.size() == 1 &&
              legacy.queued_signals[0].second.info.signal == 5,
          "legacy exec queues SIGTRAP rather than fabricating an exec event");
    check(legacy.ptrace.signal_delivery_stop(2, legacy.queued_signals[0].second) &&
              legacy.wait(2).status == ((5 << 8) | 0x7f),
          "legacy exec SIGTRAP enters normal signal-delivery stop");
    check(legacy.ptrace.request(1, 7, 2, 0, 5) == 0 &&
              legacy.signals.back().second && legacy.signals.back().second->info.signal == 5,
          "legacy exec SIGTRAP can be delivered to tracee");
}

void interrupted_syscall_stops() {
    Fixture killed;
    check(killed.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for fatal signal");
    killed.signal_stop(2, 19);
    (void)killed.wait(2);
    check(killed.ptrace.request(1, 24, 2, 0, 0) == 0, "trace syscall before fatal signal");
    killed.tasks[2].cpu.x[7] = 0x5555;
    check(killed.ptrace.syscall_entry(2) && killed.tasks[2].cpu.x[7] == 0,
          "fatal signal test enters syscall stop");
    killed.ptrace.fatal_signal(2);
    check(!killed.tasks[2].stopped && killed.tasks[2].cpu.x[7] == 0x5555,
          "fatal signal restores x7 and unstops tracee");

    Fixture orphaned;
    check(orphaned.ptrace.request(1, 16, 2, 0, 0) == 0, "attach for tracer death");
    orphaned.signal_stop(2, 19);
    (void)orphaned.wait(2);
    check(orphaned.ptrace.request(1, 24, 2, 0, 0) == 0, "trace syscall before tracer death");
    orphaned.tasks[2].cpu.x[7] = 0x7777;
    check(orphaned.ptrace.syscall_exit(2) && orphaned.tasks[2].cpu.x[7] == 1,
          "tracer death test enters syscall stop");
    orphaned.ptrace.tracer_exit(1);
    check(!orphaned.tasks[2].stopped && orphaned.tasks[2].cpu.x[7] == 0x7777,
          "tracer death restores x7 and unstops tracee");
}

void follow_clone_events() {
    constexpr uint32_t options = Ptrace::o_tracesysgood | Ptrace::o_tracefork | Ptrace::o_tracevfork |
                                 Ptrace::o_traceclone | Ptrace::o_traceexec | Ptrace::o_tracevforkdone |
                                 Ptrace::o_traceexit;
    struct Case { uint64_t flags; uint32_t exit_signal; int event; };
    const Case cases[] = {{0, 17, 1}, {0, 0, 3}, {0, 10, 3}, {0x4000, 17, 2},
                          {0x4000, 0, 2}, {0x10000, 0, 3}};
    for (const auto &test : cases) {
        Fixture f;
        f.add(4, (test.flags & 0x10000) ? 2 : 4, 2);
        check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "attach clone parent");
        f.signal_stop(2, 19);
        (void)f.wait(2);
        check(f.ptrace.request(1, 0x4200, 2, 0, options) == 0 &&
                  f.ptrace.request(1, 24, 2, 0, 0) == 0,
              "set all follow options and resume parent");
        f.ptrace.clone(2, 4, test.flags, test.exit_signal);
        check(f.ptrace.traced(4) && f.tasks[2].stopped && !f.tasks[4].stopped &&
                  f.queued_signals.size() == 1 && f.queued_signals[0].first == 4 &&
                  f.queued_signals[0].second.info.signal == 19 && !f.ptrace.syscall_entry(4),
              "child inherits tracing but not parent's syscall-resume mode");
        const int expected = (test.event << 16) | (5 << 8) | 0x7f;
        check(f.wait(2, 0x01000000).status == expected && f.wait(2).status == expected &&
                  f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 4,
              "parent fork event, WNOWAIT and child TID message");
        check(f.ptrace.signal_delivery_stop(4, std::move(f.queued_signals[0].second)) &&
                  f.wait(4).status == ((19 << 8) | 0x7f), "new child stops on SIGSTOP");
        check(f.ptrace.request(1, 0x4202, 4, 0, 1024) == 0 &&
                  f.get<signals::Info>(1, 1024).sender_pid() == 0,
              "new child stop carries kernel-generated SIGSTOP information");
        check(f.ptrace.request(1, 24, 4, 0, 0) == 0 && f.ptrace.syscall_entry(4) &&
                  f.wait(4).status == (((5 | 0x80) << 8) | 0x7f),
              "child inherits TRACESYSGOOD and resumes independently");
        check(f.ptrace.request(1, 7, 4, 0, 0) == 0, "resume child for nested fork");
        f.add(5, 5, 4);
        f.ptrace.clone(4, 5, 0, 17);
        check(f.ptrace.traced(5) && f.wait(4).status == ((1 << 16) | (5 << 8) | 0x7f) &&
                  f.ptrace.request(1, 0x4201, 4, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 5,
              "follow options propagate to grandchildren");
        f.ptrace.tracer_exit(1);
        check(!f.ptrace.traced(2) && !f.ptrace.traced(4) && !f.ptrace.traced(5) &&
                  !f.tasks[2].stopped && !f.tasks[4].stopped,
              "tracer death detaches every followed generation");
    }
}

void clone_selection_and_vfork_done() {
    Fixture f;
    f.add(4, 4, 2);
    check(f.ptrace.request(1, 16, 2, 0, 0) == 0, "attach clone selection parent");
    f.signal_stop(2, 19);
    (void)f.wait(2);
    check(f.ptrace.request(1, 0x4200, 2, 0, Ptrace::o_tracefork | Ptrace::o_tracevforkdone) == 0 &&
              f.ptrace.request(1, 7, 2, 0, 0) == 0, "set fork selection");
    f.ptrace.clone(2, 4, 0, 0);
    check(!f.ptrace.traced(4) && !f.tasks[2].stopped, "CLONE does not fall back to TRACEFORK");
    f.ptrace.clone(2, 4, 0x800000, 17);
    check(!f.ptrace.traced(4) && !f.tasks[2].stopped, "CLONE_UNTRACED suppresses fork event");
    f.ptrace.clone(2, 4, 0x802000, 17);
    check(f.ptrace.traced(4) && !f.tasks[2].stopped && f.queued_signals.size() == 1,
          "CLONE_PTRACE inherits even with CLONE_UNTRACED, without parent event");
    check(f.ptrace.signal_delivery_stop(4, std::move(f.queued_signals[0].second)) &&
              f.wait(4).status == ((19 << 8) | 0x7f) &&
              f.ptrace.request(1, 17, 4, 0, 0) == 0 && !f.ptrace.traced(4),
          "detach followed child independently");

    f.ptrace.vfork_done(2, 4);
    check(f.wait(2).status == ((5 << 16) | (5 << 8) | 0x7f) &&
              f.ptrace.request(1, 0x4201, 2, 0, 32) == 0 && f.get<uint64_t>(1, 32) == 4,
          "VFORK_DONE stops parent with child TID");
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0, "resume vfork completion");
    f.ptrace.clone(2, 4, 0, 17);
    check(f.wait(2).status == ((1 << 16) | (5 << 8) | 0x7f), "fork event before deferred completion");
    f.ptrace.vfork_done(2, 4);
    check(f.ptrace.request(1, 7, 2, 0, 0) == 0 && f.tasks[2].stopped &&
              f.wait(2).status == ((5 << 16) | (5 << 8) | 0x7f),
          "completion waits behind existing parent stop");
    f.ptrace.vfork_done(2, 4);
    check(f.ptrace.request(1, 0x4200, 2, 0, 0) == 0 &&
              f.ptrace.request(1, 7, 2, 0, 0) == 0 && !f.tasks[2].stopped,
          "clearing option while stopped suppresses deferred completion");
    f.ptrace.vfork_done(2, 4);
    check(!f.tasks[2].stopped, "disabled VFORK_DONE does not stop parent");
}
} // namespace

int main() {
    tracing_and_registers();
    permissions_and_lifecycle();
    exit_event_lifecycle();
    exit_event_disabled();
    exit_event_signals_and_cleanup();
    force_access_permissions();
    thread_wait_exec_and_exit();
    interrupted_syscall_stops();
    follow_clone_events();
    clone_selection_and_vfork_done();
    std::puts("ptrace state/permissions/registers/memory/lifecycle ok");
}
