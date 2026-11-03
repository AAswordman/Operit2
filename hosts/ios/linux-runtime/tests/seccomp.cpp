#include "linux/security/seccomp.h"
#include "cpu/aarch64/state.h"
#include "linux/signals/signals.h"
#include "memory/address_space.h"
#include "runtime/tasks/scheduler.h"
#include <array>
#include <cstdio>
#include <cstring>
#include <initializer_list>
#include <pthread.h>
#include <stdexcept>
#include <vector>

using namespace operit::arm64;
using namespace operit::arm64::security;

namespace {
constexpr uint64_t header = 0x10000, code = 0x12000, output = 0x11000;
constexpr uint32_t allow = 0x7fff0000, error = 0x00050000, trap = 0x00030000;
constexpr uint32_t kill_process = 0x80000000;

struct Insn {
    uint16_t op;
    uint8_t jt, jf;
    uint32_t k;
};
Insn statement(uint16_t op, uint32_t k = 0) { return {op, 0, 0, k}; }
Insn jump(uint16_t op, uint32_t k, uint8_t jt, uint8_t jf) { return {op, jt, jf, k}; }

void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}

struct Fixture {
    Memory memory{16 * 1024 * 1024};
    SeccompState state;
    CpuState cpu;
    Fixture() {
        memory.map(header, 0x20000, read_perm | write_perm);
        cpu.x[8] = 172;
        cpu.pc = 0x1234567800101004ull;
    }
    int64_t syscall(uint64_t op, uint64_t flags, uint64_t args) {
        try {
            return seccomp_syscall(state, memory, op, flags, args);
        } catch (const MemoryFault &) {
            return -14;
        } catch (const GuestError &e) {
            return -e.number;
        }
    }
    int64_t prctl(uint64_t op, uint64_t a = 0, uint64_t b = 0, uint64_t c = 0, uint64_t d = 0) {
        try {
            return seccomp_prctl(state, memory, op, a, b, c, d);
        } catch (const MemoryFault &) {
            return -14;
        } catch (const GuestError &e) {
            return -e.number;
        }
    }
    int64_t group_syscall(Scheduler &scheduler, uint64_t flags, uint64_t address = header) {
        try {
            return seccomp_syscall(scheduler.current().seccomp, memory, 1, flags, address, &scheduler);
        } catch (const MemoryFault &) {
            return -14;
        } catch (const GuestError &e) {
            return -e.number;
        }
    }
    void program(const std::vector<Insn> &instructions) {
        memory.store(header, instructions.size(), 2);
        memory.store(header + 8, code, 8);
        for (size_t index = 0; index < instructions.size(); ++index) {
            const auto &i = instructions[index];
            const auto address = code + index * 8;
            memory.store(address, i.op, 2);
            memory.store(address + 2, i.jt, 1);
            memory.store(address + 3, i.jf, 1);
            memory.store(address + 4, i.k, 4);
        }
    }
    int64_t install(const std::vector<Insn> &instructions) {
        program(instructions);
        return syscall(1, 0, header);
    }
    int64_t install(std::initializer_list<Insn> instructions) {
        return install(std::vector<Insn>(instructions));
    }
    void enable() { require(prctl(38, 1) == 0, "set no_new_privs"); }
    SeccompDecision evaluate() const { return seccomp_evaluate(state, cpu); }
};

void privileges_and_control() {
    Fixture f;
    require(f.prctl(39) == 0 && f.prctl(21, 77, 88, 99, 111) == 0, "initial state");
    require(f.prctl(38, 0) == -22 && f.prctl(38, 2) == -22 && f.prctl(38, 1, 1) == -22 &&
            f.prctl(38, 1, 0, 1) == -22 && f.prctl(38, 1, 0, 0, 1) == -22,
            "SET_NO_NEW_PRIVS argument validation");
    require(f.prctl(39, 1) == -22 && f.prctl(39, 0, 1) == -22 && f.prctl(39, 0, 0, 1) == -22 &&
            f.prctl(39, 0, 0, 0, 1) == -22, "GET_NO_NEW_PRIVS argument validation");
    require(f.install({statement(0x06, allow)}) == -13, "filter without no_new_privs");
    require(f.syscall(1, 0, 0) == -14, "invalid filter header");
    f.program({});
    require(f.syscall(1, 0, header) == -22, "invalid length precedes privilege check");
    f.enable();
    require(f.prctl(38, 1) == 0 && f.prctl(38, 0) == -22 && f.prctl(39) == 1,
            "no_new_privs is monotonic");
    for (uint64_t flag : {1u, 2u, 4u, 8u, 32u})
        require(f.syscall(1, flag, header) == -95 && f.prctl(21) == 0, "unsupported flag installed filter");
    require(f.syscall(1, 64, header) == -22 && f.syscall(0, 0, 0) == -95 &&
            f.syscall(0, 0, header) == -22 && f.syscall(999, 0, 0) == -22,
            "unsupported seccomp operations");
    f.program({statement(0x06, allow)});
    f.memory.store(header + 8, UINT64_MAX, 8);
    require(f.syscall(1, 0, header) == -14 && f.prctl(21) == 0, "invalid instruction pointer");
    const uint64_t host_pointer = uint64_t(reinterpret_cast<uintptr_t>(&f));
    f.memory.store(header + 8, host_pointer, 8);
    require(f.syscall(1, 0, header) == -14 && f.prctl(21) == 0, "host pointer must not become guest code");
    f.program({statement(0x06, allow)});
    require(f.prctl(22, 2, header, 9, 10) == 0 && f.prctl(21) == 2,
            "prctl filter installation and ignored extra arguments");

    f.memory.store(output, trap, 4);
    require(f.syscall(2, 0, output) == 0, "GET_ACTION_AVAIL TRAP");
    f.memory.store(output, 0x12340000, 4);
    require(f.syscall(2, 0, output) == -95 && f.syscall(2, 1, output) == -22 &&
            f.syscall(2, 0, 0) == -14, "GET_ACTION_AVAIL invalid action/flags/address");
    require(f.syscall(3, 0, output) == 0 && f.memory.load(output, 2) == 80 &&
            f.memory.load(output + 2, 2) == 24 && f.memory.load(output + 4, 2) == 64,
            "GET_NOTIF_SIZES ABI");
}

std::vector<Insn> deny_number(uint32_t number, uint32_t value) {
    return {statement(0x20, 0), jump(0x15, number, 0, 1), statement(0x06, error | value),
            statement(0x06, allow)};
}

int64_t result_for(const SeccompState &state, uint32_t number) {
    CpuState cpu{};
    cpu.x[8] = number;
    return seccomp_evaluate(state, cpu).value;
}

void tsync_group_and_lifecycle() {
    Fixture f;
    Scheduler scheduler;
    auto &caller = scheduler.current().seccomp;
    const auto unprivileged = scheduler.spawn({}, 0, 0, 0);
    const auto other_process = scheduler.spawn_process({}, 0, 0, 0);
    f.program(deny_number(172, 13));
    require(f.group_syscall(scheduler, 1) == -13 && !scheduler.lookup(unprivileged)->seccomp.mode(),
            "TSYNC permission is checked on the caller");
    require(seccomp_prctl(caller, f.memory, 38, 1, 0, 0, 0) == 0, "enable caller no_new_privs");
    require(f.group_syscall(scheduler, 0) == 0, "install caller base filter");
    const auto inherited = scheduler.spawn({}, 0, 0, 0);
    f.program(deny_number(173, 14));
    require(f.group_syscall(scheduler, 16) == 0, "TSYNC_ESRCH without TSYNC installs locally");
    {
        Scheduler::ScopedTask task(scheduler, unprivileged);
        scheduler.block({}, -110);
    }
    scheduler.lookup(inherited)->stopped = true;
    f.program(deny_number(174, 15));
    require(f.group_syscall(scheduler, 1) == 0, "synchronize ancestor and unfiltered siblings");
    for (auto tid : {scheduler.current_tid(), unprivileged, inherited}) {
        const auto &state = scheduler.lookup(tid)->seccomp;
        require(state.no_new_privileges() && state.mode() == 2 && result_for(state, 172) == -13 &&
                    result_for(state, 173) == -14 && result_for(state, 174) == -15,
                "all live siblings receive the entire filter chain and no_new_privs");
    }
    require(scheduler.lookup(unprivileged)->state == Task::State::Waiting &&
                scheduler.lookup(inherited)->stopped, "TSYNC preserves waiting and stopped states");
    const auto &unrelated = scheduler.lookup(other_process)->seccomp;
    require(!unrelated.no_new_privileges() && unrelated.mode() == 0 && result_for(unrelated, 174) == 0,
            "TSYNC must not affect a different process");

    scheduler.lookup(inherited)->stopped = false;
    uint32_t child;
    {
        Scheduler::ScopedTask task(scheduler, inherited);
        child = scheduler.spawn({}, 0, 0, 0);
    }
    require(scheduler.lookup(child)->seccomp.no_new_privileges() &&
                result_for(scheduler.lookup(child)->seccomp, 173) == -14,
            "a synchronized sibling passes its state to new threads");
    {
        Scheduler::ScopedTask task(scheduler, inherited);
        scheduler.replace_process_image(1, {});
    }
    require(scheduler.lookup(1)->seccomp.no_new_privileges() &&
                result_for(scheduler.lookup(1)->seccomp, 174) == -15 &&
                scheduler.lookup(child)->state == Task::State::Exited &&
                scheduler.lookup(other_process)->seccomp.mode() == 0,
            "non-leader exec retains synchronized state and retires its siblings");
}

void tsync_failure_atomicity() {
    Fixture f;
    Scheduler scheduler;
    const auto compatible = scheduler.spawn({}, 0, 0, 0);
    const auto divergent = scheduler.spawn({}, 0, 0, 0);
    f.program(deny_number(172, 37));
    require(seccomp_prctl(scheduler.current().seccomp, f.memory, 38, 1, 0, 0, 0) == 0 &&
                f.group_syscall(scheduler, 0) == 0, "install caller branch");
    {
        Scheduler::ScopedTask task(scheduler, divergent);
        require(seccomp_prctl(scheduler.current().seccomp, f.memory, 38, 1, 0, 0, 0) == 0 &&
                    f.group_syscall(scheduler, 0) == 0, "install distinct identical branch");
    }
    f.program(deny_number(174, 41));
    require(f.group_syscall(scheduler, 1) == divergent && f.group_syscall(scheduler, 17) == -3,
            "divergent identical filters report TID or ESRCH");
    require(!scheduler.lookup(compatible)->seccomp.no_new_privileges() &&
                scheduler.lookup(compatible)->seccomp.mode() == 0 &&
                result_for(scheduler.current().seccomp, 174) == 0 &&
                result_for(scheduler.lookup(divergent)->seccomp, 174) == 0 &&
                result_for(scheduler.lookup(divergent)->seccomp, 172) == -37,
            "synchronization conflict must not partially update any task");
    require(f.group_syscall(scheduler, 1, 0) == -14, "TSYNC header fault");
    f.program({statement(0xffff), statement(0x06, allow)});
    require(f.group_syscall(scheduler, 1) == -22 &&
                !scheduler.lookup(compatible)->seccomp.no_new_privileges(),
            "invalid filter is rejected before synchronization without updates");
    {
        Scheduler::ScopedTask task(scheduler, divergent);
        scheduler.exit(0, false);
    }
    require(scheduler.lookup(divergent)->state == Task::State::Exited, "divergent task retained at exit");
    f.program(deny_number(174, 41));
    require(f.group_syscall(scheduler, 17) == 0 &&
                scheduler.lookup(compatible)->seccomp.no_new_privileges() &&
                result_for(scheduler.lookup(compatible)->seccomp, 174) == -41,
            "exited divergent tasks must not prevent synchronization");

    Fixture limit;
    Scheduler limited;
    const auto peer = limited.spawn({}, 0, 0, 0);
    require(seccomp_prctl(limited.current().seccomp, limit.memory, 38, 1, 0, 0, 0) == 0,
            "enable path-limit caller");
    const std::vector<Insn> maximum(4096, statement(0x06, allow));
    limit.program(maximum);
    for (unsigned n = 0; n < 7; ++n)
        require(limit.group_syscall(limited, 0) == 0, "build long caller chain");
    require(limit.group_syscall(limited, 1) == -12 && !limited.lookup(peer)->seccomp.no_new_privileges() &&
                limited.lookup(peer)->seccomp.mode() == 0 && limited.current().seccomp.mode() == 2,
            "path-limit failure must leave all siblings unchanged");
}

void reject_bad_programs() {
    Fixture f;
    f.enable();
    const std::vector<std::vector<Insn>> bad = {
        {}, {statement(0x00)}, {statement(0xffff), statement(0x06, allow)},
        {statement(0x20, 1), statement(0x06, allow)},
        {statement(0x20, 64), statement(0x06, allow)},
        {statement(0x20, 0xfffffffc), statement(0x06, allow)},
        {statement(0x28), statement(0x06, allow)}, // H-sized seccomp_data read
        {statement(0x40), statement(0x06, allow)}, // indirect read
        {statement(0x60, 16), statement(0x06, allow)},
        {statement(0x02, 16), statement(0x06, allow)},
        {statement(0x60, 0), statement(0x06, allow)},
        {statement(0x61, 0), statement(0x06, allow)},
        {statement(0x34, 0), statement(0x06, allow)},
        {statement(0x94, 1), statement(0x06, allow)}, // MOD is not in seccomp's opcode set
        {statement(0x64, 32), statement(0x06, allow)},
        {statement(0x74, 0xffffffff), statement(0x06, allow)},
        {statement(0x05, 0xffffffff), statement(0x06, allow)},
        {statement(0x05, 1), statement(0x06, allow)},
        {jump(0x15, 0, 0, 1), statement(0x06, allow)},
        {statement(0x00, 1), jump(0x15, 1, 0, 1), statement(0x02),
         statement(0x60), statement(0x06, allow)}, // one path reads uninitialized scratch
    };
    for (const auto &program : bad) {
        require(f.install(program) == -22, "malformed BPF accepted");
        require(f.state.mode() == 0 && f.evaluate().action == SeccompAction::Allow,
                "failed installation changed state");
    }
    std::vector<Insn> oversized(4097, statement(0x06, allow));
    require(f.install(oversized) == -22, "program over BPF_MAXINSNS");
    require(f.install({statement(0x00, 1), jump(0x15, 1, 0, 1), statement(0x02),
                       statement(0x02), statement(0x60), statement(0x06, allow)}) == 0,
            "initialized scratch on every branch rejected");
}

void abi_and_immutable_snapshot() {
    Fixture f;
    f.enable();
    for (unsigned n = 0; n < 6; ++n)
        f.cpu.x[n] = 0x1020304050607080ull + (uint64_t(n) << 36) + n;
    f.cpu.x[8] = 0xabcdef01000000acull;
    const std::array<uint32_t, 16> expected = {
        172, 0xc00000b7, uint32_t(f.cpu.pc), uint32_t(f.cpu.pc >> 32),
        uint32_t(f.cpu.x[0]), uint32_t(f.cpu.x[0] >> 32), uint32_t(f.cpu.x[1]), uint32_t(f.cpu.x[1] >> 32),
        uint32_t(f.cpu.x[2]), uint32_t(f.cpu.x[2] >> 32), uint32_t(f.cpu.x[3]), uint32_t(f.cpu.x[3] >> 32),
        uint32_t(f.cpu.x[4]), uint32_t(f.cpu.x[4] >> 32), uint32_t(f.cpu.x[5]), uint32_t(f.cpu.x[5] >> 32)};
    for (size_t index = 0; index < expected.size(); ++index) {
        require(f.install({statement(0x20, uint32_t(index * 4)), jump(0x15, expected[index], 0, 1),
                           statement(0x06, error | uint32_t(index + 1)), statement(0x06, allow)}) == 0,
                "install ABI-word check");
        auto result = f.evaluate();
        require(result.action == SeccompAction::Errno && result.value == -int64_t(index + 1),
                "seccomp_data word or equal-precedence ordering");
    }
    Fixture snapshot;
    snapshot.enable();
    require(snapshot.install({statement(0x06, error | 13)}) == 0, "install immutable filter");
    snapshot.memory.store(code + 4, allow, 4);
    require(snapshot.evaluate().value == -13, "guest modified an installed filter");
}

void arithmetic_and_actions() {
    Fixture f;
    f.enable();
    require(f.install({statement(0x00, 18), statement(0x01, 3), statement(0x3c),
                       statement(0x02, 7), statement(0x00, error), statement(0x61, 7),
                       statement(0x0c), statement(0x16)}) == 0, "BPF ALU/scratch install");
    require(f.evaluate().action == SeccompAction::Errno && f.evaluate().value == -6,
            "BPF ALU/scratch/RET A");
    Fixture length;
    length.enable();
    require(length.install({statement(0x80), statement(0x81), jump(0x1d, 0, 0, 1),
                            statement(0x06, error | 64), statement(0x06, allow)}) == 0 &&
            length.evaluate().value == -64, "BPF LEN and comparison with X");
    Fixture divide;
    divide.enable();
    require(divide.install({statement(0x00, 1), statement(0x01, 0), statement(0x3c),
                            statement(0x06, allow)}) == 0 &&
            divide.evaluate().action == SeccompAction::KillThread, "runtime divide by zero must kill");
    Fixture chain;
    chain.enable();
    require(chain.install({statement(0x06, error | 13)}) == 0, "older errno filter");
    require(chain.install({statement(0x06, error | 42)}) == 0 && chain.evaluate().value == -42,
            "newest equal-priority filter wins");
    require(chain.install({statement(0x06, trap | 17)}) == 0 &&
            chain.evaluate().action == SeccompAction::Trap && chain.evaluate().data == 17,
            "trap must dominate errno");
    require(chain.install({statement(0x06, kill_process)}) == 0 &&
            chain.evaluate().action == SeccompAction::KillProcess, "signed KILL_PROCESS precedence");
    Fixture clamped;
    clamped.enable();
    require(clamped.install({statement(0x06, error | 0xffff)}) == 0 && clamped.evaluate().value == -4095,
            "ERRNO cap at MAX_ERRNO");
    Fixture zero_errno;
    zero_errno.enable();
    require(zero_errno.install({statement(0x06, error)}) == 0 &&
            zero_errno.evaluate().action == SeccompAction::Errno && zero_errno.evaluate().value == 0,
            "ERRNO zero must skip the syscall even with a zero return value");
    Fixture logged;
    logged.enable();
    require(logged.install({statement(0x06, 0x7ffc0000)}) == 0 &&
            logged.evaluate().action == SeccompAction::Log, "LOG requires an explicit audit decision");
    for (uint32_t unsupported : {0x7fc00000u, 0x7ff00000u}) {
        Fixture no_handler;
        no_handler.enable();
        require(no_handler.install({statement(0x06, unsupported)}) == 0 &&
                no_handler.evaluate().action == SeccompAction::Errno && no_handler.evaluate().value == -38,
                "listener/tracer absence must deny syscall");
    }
    Fixture unknown;
    unknown.enable();
    require(unknown.install({statement(0x06, 0x12340000)}) == 0 &&
            unknown.evaluate().action == SeccompAction::KillProcess, "unknown action must kill process");
}

void instruction_matrix() {
    struct AluCase { uint16_t opcode; uint32_t a, x, k, expected; };
    const AluCase cases[] = {
        {0x04, 0xffffffff, 0, 2, 1}, {0x0c, 10, 3, 99, 13},
        {0x14, 0, 0, 1, 0xffff}, {0x1c, 10, 3, 0, 7},
        {0x24, 9, 0, 11, 99}, {0x2c, 9, 11, 0, 99},
        {0x34, 100, 0, 3, 33}, {0x3c, 100, 3, 0, 33},
        {0x44, 0x50, 0, 0x0a, 0x5a}, {0x4c, 0x50, 0x0a, 0, 0x5a},
        {0x54, 0x5a, 0, 0x3c, 0x18}, {0x5c, 0x5a, 0x3c, 0, 0x18},
        {0x64, 1, 0, 3, 8}, {0x6c, 1, 35, 0, 8},
        {0x74, 0x80, 0, 3, 16}, {0x7c, 0x80, 35, 0, 16},
        {0x84, 2, 0, 0, 0xfffe},
        {0xa4, 0x5a, 0, 0x3c, 0x66}, {0xac, 0x5a, 0x3c, 0, 0x66},
    };
    for (const auto &test : cases) {
        Fixture f;
        f.enable();
        require(f.install({statement(0x00, test.a), statement(0x01, test.x), statement(test.opcode, test.k),
                           statement(0x54, 0xffff), statement(0x44, error), statement(0x16)}) == 0,
                "ALU matrix installation");
        const auto result = f.evaluate();
        require(result.action == SeccompAction::Errno && result.data == test.expected,
                "ALU matrix result");
    }
    struct BranchCase { uint16_t opcode; uint32_t a, rhs; bool expected; };
    const BranchCase branches[] = {
        {0x15, 3, 3, true}, {0x1d, 3, 4, false},
        {0x25, 0xffffffff, 1, true}, {0x2d, 2, 3, false},
        {0x35, 3, 3, true}, {0x3d, 2, 3, false},
        {0x45, 0x80, 0x80, true}, {0x4d, 0x80, 0x40, false},
    };
    for (const auto &test : branches) {
        Fixture f;
        f.enable();
        require(f.install({statement(0x00, test.a), statement(0x01, test.rhs),
                           jump(test.opcode, test.rhs, 0, 1), statement(0x06, error | 17),
                           statement(0x06, error | 19)}) == 0, "branch matrix installation");
        require(f.evaluate().value == (test.expected ? -17 : -19), "branch matrix result");
    }
    Fixture misc;
    misc.enable();
    require(misc.install({statement(0x00, 42), statement(0x07), statement(0x03, 15),
                          statement(0x00, 0), statement(0x61, 15), statement(0x87),
                          statement(0x05, 1), statement(0x06, allow), statement(0x44, error),
                          statement(0x16)}) == 0 && misc.evaluate().value == -42,
            "TAX, STX, LDX scratch, TXA, JA and RET A");
}

void lifecycle_and_limits() {
    Fixture f;
    const SeccompState preexisting_sibling = f.state;
    f.enable();
    require(!preexisting_sibling.no_new_privileges(), "no_new_privs must not mutate existing siblings");
    require(f.install({statement(0x06, error | 7)}) == 0, "install inherited filter");
    SeccompState child = f.state, sibling = f.state;
    require(f.install({statement(0x06, error | 8)}) == 0 && f.evaluate().value == -8 &&
            seccomp_evaluate(child, f.cpu).value == -7 && seccomp_evaluate(sibling, f.cpu).value == -7,
            "filter updates must not propagate to existing tasks");
    SeccompState grandchild = f.state;
    require(grandchild.no_new_privileges() && grandchild.mode() == 2 &&
            seccomp_evaluate(grandchild, f.cpu).value == -8,
            "clone state must inherit the caller's active filter chain");
    SeccompState exec_survivor = child;
    require(exec_survivor.no_new_privileges() && seccomp_evaluate(exec_survivor, f.cpu).value == -7,
            "exec must retain calling task state");
    Fixture limit;
    limit.enable();
    const std::vector<Insn> max_length(4096, statement(0x06, allow));
    for (unsigned n = 0; n < 7; ++n)
        require(limit.install(max_length) == 0, "valid maximum-sized filter chain");
    require(limit.install(max_length) == -12 && limit.state.mode() == 2 &&
            limit.evaluate().action == SeccompAction::Allow, "filter path limit and atomic failure");
}

void *deep_filter_release_worker(void *) {
    try {
        Fixture f;
        f.enable();
        SeccompState ancestor;
        for (unsigned n = 0; n < 6554; ++n) {
            require(f.install({statement(0x06, allow)}) == 0, "short-filter path limit");
            if (n == 3000)
                ancestor = f.state;
        }
        require(f.install({statement(0x06, allow)}) == -12, "short-filter path limit exceeded");
        f.state = {};
        require(ancestor.mode() == 2 && seccomp_evaluate(ancestor, f.cpu).action == SeccompAction::Allow,
                "releasing a descendant changed a shared filter chain");
        ancestor = {};
        return nullptr;
    } catch (...) {
        return reinterpret_cast<void *>(1);
    }
}

void deep_filter_release() {
    pthread_attr_t attributes;
    require(pthread_attr_init(&attributes) == 0, "pthread attributes");
    require(pthread_attr_setstacksize(&attributes, 512 * 1024) == 0, "small pthread stack");
    pthread_t thread;
    const int created = pthread_create(&thread, &attributes, deep_filter_release_worker, nullptr);
    pthread_attr_destroy(&attributes);
    require(created == 0, "start deep-filter test");
    void *result = nullptr;
    require(pthread_join(thread, &result) == 0 && !result, "deep-filter release");
}

void trap_siginfo() {
    Fixture f;
    f.cpu.x[8] = 0xf0000000abcdef01ull;
    signals::ProcessState process;
    signals::ThreadState thread;
    process.actions->entries[30].handler = 1;
    thread.restart.emplace();
    uint64_t mask = signals::bit(31) | signals::bit(10);
    seccomp_force_trap(process, thread, mask, f.cpu, 123);
    require(!thread.restart && process.actions->entries[30].handler == 0 &&
            !(mask & signals::bit(31)) && (mask & signals::bit(10)), "forced SIGSYS disposition/mask");
    const auto pending = thread.pending.take(UINT64_MAX);
    require(pending && pending->forced && pending->info.signal == 31 && pending->info.code == 1 &&
            pending->info.error == 123, "forced SIGSYS header");
    uint64_t address = 0;
    uint32_t nr = 0, arch = 0;
    std::memcpy(&address, pending->info.payload.data(), 8);
    std::memcpy(&nr, pending->info.payload.data() + 8, 4);
    std::memcpy(&arch, pending->info.payload.data() + 12, 4);
    require(address == f.cpu.pc && nr == 0xabcdef01u && arch == 0xc00000b7,
            "forced SIGSYS call address/number/arch");
}
} // namespace

int main() {
    try {
        privileges_and_control();
        reject_bad_programs();
        abi_and_immutable_snapshot();
        arithmetic_and_actions();
        instruction_matrix();
        lifecycle_and_limits();
        tsync_group_and_lifecycle();
        tsync_failure_atomicity();
        deep_filter_release();
        trap_siginfo();
        std::puts("seccomp verifier, interpreter, ABI, lifecycle and SIGSYS: ok");
        return 0;
    } catch (const std::exception &e) {
        std::fprintf(stderr, "seccomp test failed: %s\n", e.what());
        return 1;
    }
}
