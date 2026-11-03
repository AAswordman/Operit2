#include "linux/signals/signals.h"
#include "linux/syscalls.h"
#include <atomic>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <future>
#include <stdexcept>
#include <thread>
#include <time.h>
#include <fcntl.h>

using namespace operit::arm64;
namespace {
constexpr uint64_t a = 0x11000, b = 0x11004, c = 0x11008;
constexpr uint64_t array = 0x14000, timeout = 0x16000;
constexpr uint32_t u32_private = 2 | 128, u32_shared = 2;
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Fixture {
    std::string root;
    Memory memory{16 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler scheduler;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/operit-futex2-test-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        root = created;
        fs = std::make_unique<Filesystem>(root.c_str());
        memory.map(0x10000, 0x10000, read_perm | write_perm);
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, *fs, config, scheduler);
    }
    ~Fixture() {
        sys.reset();
        std::filesystem::remove_all(root);
    }
    uint32_t spawn() { return scheduler.spawn(CpuState{}, 0x18000, 0, 0); }
    Task &task(uint32_t tid) { return *scheduler.lookup(tid); }
    int64_t result(uint32_t tid) { return int64_t(task(tid).cpu.x[0]); }
    static void entry(Memory &mem, uint64_t base, unsigned index, uint64_t address, uint64_t value = 0,
                      uint32_t flags = u32_private, uint32_t reserved = 0) {
        const auto item = base + index * 24;
        mem.store(item, value, 8);
        mem.store(item + 8, address, 8);
        mem.store(item + 16, flags, 4);
        mem.store(item + 20, reserved, 4);
    }
    void entry(unsigned index, uint64_t address, uint64_t value = 0, uint32_t flags = u32_private,
               uint32_t reserved = 0) {
        entry(memory, array, index, address, value, flags, reserved);
    }
    int64_t call(uint32_t tid, uint64_t number, std::initializer_list<uint64_t> args,
                 Syscalls *other = nullptr) {
        Scheduler::ScopedTask scope(scheduler, tid);
        auto &current = scheduler.current();
        auto &cpu = current.cpu;
        cpu.x.fill(0);
        cpu.pc = 0x40004;
        cpu.sp = 0x18000;
        unsigned index = 0;
        for (auto value : args)
            cpu.x[index++] = value;
        cpu.x[8] = number;
        signals::remember_syscall(current.signals, cpu, true);
        (other ? other : sys.get())->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    void absolute_time(clockid_t clock, long millis) {
        timespec now{};
        require(::clock_gettime(clock, &now) == 0, "clock_gettime");
        int64_t nanos = now.tv_nsec + millis * 1000000;
        int64_t seconds = now.tv_sec + nanos / 1000000000;
        nanos %= 1000000000;
        memory.store(timeout, seconds, 8);
        memory.store(timeout + 8, nanos, 8);
    }
};
void validation() {
    Fixture f;
    f.entry(0, a);
    f.entry(1, b);
    require(f.call(1, 449, {array, 0, 0, 0, 1}) == -22, "waitv zero count");
    require(f.call(1, 449, {array, 129, 0, 0, 1}) == -22, "waitv count limit");
    require(f.call(1, 449, {0, 1, 0, 0, 1}) == -22, "waitv null array");
    require(f.call(1, 449, {array, 1, 1, 0, 1}) == -22, "waitv top flags");
    require(f.call(1, 449, {0x30000, 1, 0, 0, 1}) == -14, "waitv array fault");
    f.entry(0, a, 0, 0);
    require(f.call(1, 449, {array, 1, 0, 0, 1}) == -22, "waitv non-32 size");
    f.entry(0, a, 0, 2 | 4);
    require(f.call(1, 449, {array, 1, 0, 0, 1}) == -22, "waitv NUMA flag");
    f.entry(0, a, uint64_t{1} << 32);
    require(f.call(1, 449, {array, 1, 0, 0, 1}) == -22, "waitv value width");
    f.entry(0, a, 0, u32_private, 1);
    require(f.call(1, 449, {array, 1, 0, 0, 1}) == -22, "waitv reserved field");
    f.entry(0, a + 1);
    require(f.call(1, 449, {array, 1, 0, 0, 1}) == -22, "waitv alignment");
    f.entry(0, 0x30000);
    require(f.call(1, 449, {array, 1, 0, 0, 1}) == -14, "waitv word fault");
    f.entry(0, a, 1);
    f.memory.store(timeout, 0, 8);
    f.memory.store(timeout + 8, 0, 8);
    require(f.call(1, 449, {array, 1, 0, timeout, 1}) == -11, "waitv comparison before timeout");
    f.entry(0, a);
    require(f.call(1, 449, {array, 1, 0, timeout, 1}) == -110, "waitv past timeout");
    require(f.call(1, 449, {array, 1, 0, timeout, 9}) == -22, "waitv clock ID");
    f.memory.store(timeout + 8, 1000000000, 8);
    require(f.call(1, 449, {array, 1, 0, timeout, 1}) == -22, "waitv invalid timespec");
    require(f.call(1, 454, {a, 0, 1, u32_private}) == -22, "wake empty mask");
    require(f.call(1, 454, {a, uint64_t{1} << 32, 1, u32_private}) == -22, "wake mask width");
    require(f.call(1, 454, {a, 1, 1, 3}) == -22, "wake size");
    require(f.call(1, 454, {a + 1, 1, 0, u32_private}) == -22, "wake alignment with zero count");
    require(f.call(1, 454, {0x30000, 1, 1, u32_private}) == 0, "unmapped private wake");
    require(f.call(1, 454, {0x30000, 1, 1, u32_shared}) == -14, "unmapped shared wake");
    require(f.call(1, 455, {a, uint64_t{1} << 32, 1, u32_private}) == -22, "wait value width");
    require(f.call(1, 455, {a, 0, uint64_t{1} << 32, u32_private}) == -22, "wait mask width");
    require(f.call(1, 455, {a, 0, 0, u32_private}) == -22, "wait empty mask");
    require(f.call(1, 455, {a, 1, 1, u32_private}) == -11, "wait comparison");
    f.entry(0, a);
    f.entry(1, b);
    require(f.call(1, 456, {array, 1, 0, 0}) == -22, "requeue flags");
    require(f.call(1, 456, {array, 0, uint32_t(-1), 0}) == -22, "requeue negative wake");
    require(f.call(1, 456, {array, 0, 0, uint32_t(-1)}) == -22, "requeue negative count");
    f.entry(0, a, 1);
    require(f.call(1, 456, {array, 0, 0, 1}) == -11, "requeue compare");
    f.entry(0, a);
    f.entry(1, b, uint64_t{1} << 32);
    require(f.call(1, 456, {array, 0, 0, 1}) == -22, "requeue destination value width");
}
void wakes_and_requeues() {
    Fixture f;
    const auto first = f.spawn(), second = f.spawn();
    f.entry(0, a);
    f.entry(1, b);
    require(f.call(first, 449, {array, 2, 0, 0, 123}) == 0, "waitv setup");
    require(f.task(first).state == Task::State::Waiting, "waitv did not block");
    require(f.call(1, 98, {b, 1 | 128, 1}) == 1 && f.result(first) == 1, "legacy wake index");
    require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 0, "waitv sibling left queued");
    require(f.call(first, 455, {a, 0, 2, u32_private, 0, 99}) == 0, "futex_wait setup");
    require(f.call(1, 98, {a, 10 | 128, 1, 0, 0, 1}) == 0, "legacy mask mismatch");
    require(f.call(1, 98, {a, 10 | 128, 1, 0, 0, 2}) == 1 && f.result(first) == 0,
            "legacy wake on futex_wait");
    require(f.call(first, 98, {a, 9 | 128, 0, 0, 0, 4}) == 0, "legacy wait setup");
    require(f.call(1, 454, {a, 2, 1, u32_private}) == 0, "new mask mismatch");
    require(f.call(1, 454, {a, 4, 1, u32_private}) == 1, "new wake on legacy wait");
    f.entry(0, a);
    f.entry(1, a);
    f.entry(2, b);
    require(f.call(first, 449, {array, 3}) == 0, "duplicate vector setup");
    require(f.call(1, 454, {a, UINT32_MAX, 2, u32_private}) == 2 && f.result(first) == 1,
            "duplicate vector queue accounting");
    require(f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 0, "duplicate vector cleanup");
    require(f.call(first, 455, {a, 0, 1, u32_private}) == 0, "negative wake setup");
    require(f.call(1, 454, {a, 1, uint32_t(-1), u32_private}) == 1, "negative strict wake behavior");

    f.entry(0, a);
    f.entry(1, b);
    require(f.call(first, 449, {array, 2}) == 0 && f.call(second, 449, {array, 2}) == 0,
            "requeue vector setup");
    f.entry(0, b);
    f.entry(1, c);
    require(f.call(1, 456, {array, 0, 1, 1}) == 2, "new wake and requeue count");
    require(f.result(first) == 1 && f.task(second).state == Task::State::Waiting, "requeue order");
    require(f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 0, "moved source still queued");
    require(f.call(1, 98, {c, 1 | 128, 1}) == 1 && f.result(second) == 1, "moved vector wake index");
    require(f.call(1, 454, {a, UINT32_MAX, 5, u32_private}) == 0, "requeued sibling cleanup");

    f.entry(0, a);
    f.entry(1, b);
    require(f.call(first, 449, {array, 2}) == 0, "legacy requeue vector setup");
    require(f.call(1, 98, {b, 3 | 128, 0, 1, c}) == 1, "legacy requeue vector");
    require(f.call(1, 454, {c, UINT32_MAX, 1, u32_private}) == 1 && f.result(first) == 1,
            "new wake legacy-requeued vector");
    require(f.call(first, 98, {b, 128, 0}) == 0, "legacy requeue source setup");
    f.entry(0, b);
    f.entry(1, c);
    require(f.call(1, 456, {array, 0, 0, 1}) == 1, "new requeue legacy wait");
    require(f.call(1, 98, {c, 1 | 128, 1}) == 1 && f.result(first) == 0, "legacy requeued wake");
}
void cleanup_and_timeouts() {
    Fixture f;
    const auto tid = f.spawn();
    f.entry(0, a);
    f.entry(1, b);
    f.absolute_time(CLOCK_MONOTONIC, 15);
    require(f.call(tid, 449, {array, 2, 0, timeout, 1}) == 0, "monotonic wait setup");
    std::this_thread::sleep_for(std::chrono::milliseconds(25));
    f.scheduler.poll_waiting();
    require(f.result(tid) == -110, "monotonic timeout");
    require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 0 &&
                f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 0, "timeout cleanup");
    f.absolute_time(CLOCK_REALTIME, 15);
    require(f.call(tid, 449, {array, 2, 0, timeout, 0}) == 0, "realtime wait setup");
    std::this_thread::sleep_for(std::chrono::milliseconds(25));
    f.scheduler.poll_waiting();
    require(f.result(tid) == -110, "realtime timeout");
    require(f.call(tid, 449, {array, 2}) == 0, "interrupt setup");
    f.scheduler.interrupt(tid);
    require(f.result(tid) == -4 && f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 0,
            "signal cleanup");
    require(f.call(tid, 449, {array, 2}) == 0, "wake race setup");
    require(f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 1 && f.result(tid) == 1,
            "wake race result");
    f.scheduler.interrupt(tid);
    f.scheduler.poll_waiting();
    require(f.result(tid) == 1, "late signal or timeout overwrote wake");
    require(f.call(tid, 449, {array, 2}) == 0, "discard setup");
    f.scheduler.discard(tid);
    require(f.call(1, 454, {a, UINT32_MAX, 2, u32_private}) == 0 &&
                f.call(1, 454, {b, UINT32_MAX, 2, u32_private}) == 0, "discard cleanup");
}
void pi_interoperation() {
    Fixture f;
    const auto waiter = f.spawn(), pi = f.spawn();
    require(f.call(1, 98, {b, 6 | 128}) == 0, "PI lock setup");
    f.entry(0, a);
    f.entry(1, b, 1);
    require(f.call(waiter, 449, {array, 2}) == 0, "vector against uncontended PI word");
    require(f.call(1, 98, {b, 7 | 128}) == -22, "PI unlock missed secondary vector entry");
    require(f.call(pi, 98, {b, 6 | 128}) == -22, "PI lock missed secondary vector entry");
    require(f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 1 && f.result(waiter) == 1,
            "PI visibility cleanup");
    require(f.call(1, 98, {b, 7 | 128}) == 0, "PI unlock after vector cleanup");

    require(f.call(1, 98, {b, 6 | 128}) == 0 && f.call(pi, 98, {b, 6 | 128}) == 0,
            "contended PI setup");
    f.entry(1, b, f.memory.load(b, 4));
    require(f.call(waiter, 449, {array, 2}) == 0 && f.task(waiter).state == Task::State::Waiting,
            "ordinary vector wait on active PI queue");
    f.scheduler.interrupt(waiter);
    require(f.call(waiter, 455, {b, f.memory.load(b, 4), UINT32_MAX, u32_private}) == 0 &&
                f.task(waiter).state == Task::State::Waiting,
            "ordinary wait on active PI queue");
    f.scheduler.interrupt(waiter);
    f.entry(0, a);
    f.entry(1, b);
    require(f.call(1, 456, {array, 0, 0, 1}) == 0, "requeue to active PI queue with empty source");
    require(f.call(waiter, 455, {a, 0, UINT32_MAX, u32_private}) == 0, "PI target requeue setup");
    require(f.call(1, 456, {array, 0, 0, 1}) == 1 && f.task(waiter).state == Task::State::Waiting,
            "ordinary requeue to active PI queue");
    f.scheduler.interrupt(waiter);
    f.entry(0, b, f.memory.load(b, 4));
    f.entry(1, a);
    require(f.call(1, 456, {array, 0, 0, 0}) == 0, "zero requeue on active PI source");
    f.scheduler.interrupt(pi);
    require(f.call(1, 98, {b, 7 | 128}) == 0, "contended PI cleanup");

    f.memory.store(b, 0, 4);
    f.entry(0, a);
    f.entry(1, b);
    require(f.call(waiter, 449, {array, 2}) == 0, "PI requeue vector setup");
    require(f.call(1, 98, {a, 12 | 128, 1, 0, c, 0}) == -22, "PI requeue accepted vector source");
    f.scheduler.interrupt(waiter);
    require(f.call(pi, 98, {a, 11 | 128, 0, 0, b}) == 0, "PI requeue source setup");
    f.entry(1, b);
    require(f.call(waiter, 449, {array, 2}) == 0, "PI requeue target vector setup");
    require(f.call(1, 98, {a, 12 | 128, 1, 0, b, 0}) == -22,
            "PI requeue missed vector target");
    f.scheduler.interrupt(waiter);
    require(f.call(1, 98, {a, 12 | 128, 1, 0, b, 0}) == 1, "PI requeue after cleanup");
    require(f.call(pi, 98, {b, 7 | 128}) == 0, "PI requeue owner cleanup");
}
void key_regressions() {
    Fixture f;
    const auto tid = f.spawn();
    require(f.call(1, 454, {0, UINT32_MAX, 1, u32_private}) == 0, "private wake at null");
    require(f.call(1, 454, {4, UINT32_MAX, 1, u32_private}) == 0, "private wake below first page");
    require(f.call(1, 455, {0, 0, UINT32_MAX, u32_private}) == -14, "private wait at null faults");
    require(f.call(tid, 455, {a, 0, UINT32_MAX, u32_private}) == 0, "null requeue setup");
    f.entry(0, a);
    f.entry(1, 0);
    require(f.call(1, 456, {array, 0, 0, 1}) == 1, "private requeue to null key");
    require(f.call(1, 454, {0, UINT32_MAX, 1, u32_private}) == 1 && f.result(tid) == 0,
            "private wake at requeued null key");

    f.memory.map(0x30000, 4096, read_perm | write_perm);
    f.memory.store(0x30000, 0, 4);
    f.memory.protect(0x30000, 4096, read_perm);
    require(f.call(1, 454, {0x30000, UINT32_MAX, 1, u32_shared}) == -14,
            "shared wake on readonly private anonymous page");
    require(f.call(1, 455, {0x30000, 1, UINT32_MAX, u32_shared}) == -14,
            "shared wait on readonly private anonymous page");
    require(f.call(1, 454, {0x30000, UINT32_MAX, 1, u32_private}) == 0,
            "private wake on readonly anonymous page");
    f.entry(0, a, 1);
    f.entry(1, 0x30000, 0, u32_shared);
    require(f.call(1, 456, {array, 0, 0, 1}) == -14,
            "readonly anonymous destination key precedes source comparison");

    f.memory.map(0x60000, 4096, read_perm | write_perm, false, true);
    f.memory.store(0x60000, 0, 4);
    f.memory.protect(0x60000, 4096, read_perm);
    require(f.call(tid, 455, {0x60000, 0, UINT32_MAX, u32_shared}) == 0,
            "readonly shared anonymous wait");
    require(f.call(1, 454, {0x60000, UINT32_MAX, 1, u32_shared}) == 1,
            "readonly shared anonymous wake");

    std::ofstream(f.root + "/readonly-shared") << std::string(4096, '\0');
    FD backing = f.fs->resolve("/readonly-shared").open(O_RDWR);
    f.memory.map_file(0x40000, 4096, read_perm | write_perm, backing.get(),
                      {"/readonly-shared", 0, 0, 0, true});
    f.memory.map_file(0x50000, 4096, read_perm, backing.get(),
                      {"/readonly-shared", 0, 0, 0, true});
    require(f.call(tid, 455, {0x50000, 0, UINT32_MAX, u32_shared}) == 0,
            "readonly shared file wait");
    require(f.call(1, 454, {0x40000, UINT32_MAX, 1, u32_shared}) == 1,
            "readonly shared file alias wake");
    f.memory.map_file(0x70000, 4096, read_perm, backing.get(),
                      {"/readonly-shared", 0, 0, 0, false});
    require(f.call(tid, 455, {0x70000, 0, UINT32_MAX, u32_shared}) == 0,
            "readonly private file wait");
    require(f.call(1, 454, {0x70000, UINT32_MAX, 1, u32_shared}) == 1,
            "readonly private file wake");
}
void scheduler_requeue_order() {
    Fixture f;
    const auto first = f.spawn(), second = f.spawn();
    require(f.call(first, 455, {a, 0, UINT32_MAX, u32_private}) == 0 &&
                f.call(second, 455, {a, 0, UINT32_MAX, u32_private}) == 0,
            "same-key requeue setup");
    f.entry(0, a);
    f.entry(1, a);
    require(f.call(1, 456, {array, 0, 0, 1}) == 1, "same-key requeue count");
    require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 1, "same-key wake count");
    require(f.task(first).state == Task::State::Runnable && f.task(second).state == Task::State::Waiting,
            "same-key requeue changed FIFO order");
    require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 1, "same-key cleanup");

    f.entry(0, a);
    f.entry(1, a);
    require(f.call(first, 449, {array, 2}) == 0 &&
                f.call(second, 455, {a, 0, UINT32_MAX, u32_private}) == 0,
            "same-key duplicate vector setup");
    require(f.call(1, 456, {array, 0, 1, 1}) == 2 && f.result(first) == 0,
            "same-key wake and requeue counts vector entries");
    require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 1 && f.result(second) == 0,
            "same-key duplicate vector cleanup");

    require(f.call(first, 455, {a, 0, UINT32_MAX, u32_private}) == 0,
            "same-key mixed PI source setup");
    require(f.call(second, 98, {a, 11 | 128, 0, 0, b}) == 0,
            "same-key mixed PI waiter setup");
    require(f.call(1, 456, {array, 0, 0, 2}) == -22, "same-key PI requeue rejection");
    require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 1 &&
                f.task(first).state == Task::State::Runnable && f.task(second).state == Task::State::Waiting,
            "same-key PI error changed earlier waiter order");
    f.scheduler.interrupt(second);

    std::ofstream(f.root + "/same-key") << std::string(4096, '\0');
    FD backing = f.fs->resolve("/same-key").open(O_RDWR);
    f.memory.map_file(0x30000, 4096, read_perm | write_perm, backing.get(), {"/same-key", 0, 0, 0, true});
    f.memory.map_file(0x40000, 4096, read_perm | write_perm, backing.get(), {"/same-key", 0, 0, 0, true});
    require(f.call(first, 455, {0x30000, 0, UINT32_MAX, u32_shared}) == 0 &&
                f.call(second, 455, {0x30000, 0, UINT32_MAX, u32_shared}) == 0,
            "same shared key setup");
    f.entry(0, 0x30000, 0, u32_shared);
    f.entry(1, 0x40000, 0, u32_shared);
    require(f.call(1, 456, {array, 0, 0, 1}) == 1, "same shared key requeue");
    require(f.call(1, 454, {0x40000, UINT32_MAX, 1, u32_shared}) == 1 &&
                f.task(first).state == Task::State::Runnable && f.task(second).state == Task::State::Waiting,
            "same shared key changed FIFO order");
    require(f.call(1, 454, {0x30000, UINT32_MAX, 1, u32_shared}) == 1, "same shared key cleanup");
}
void legacy_same_key_requeue() {
    for (const unsigned operation : {3u, 4u}) {
        Fixture f;
        const auto first = f.spawn(), second = f.spawn(), third = f.spawn();
        const auto wait = [&](uint32_t tid) { return f.call(tid, 98, {a, 128, 0}); };
        const auto requeue = [&](unsigned wake, unsigned move, unsigned compare = 0) {
            return f.call(1, 98, {a, operation | 128, wake, move, a, compare});
        };
        const auto wake = [&] { return f.call(1, 98, {a, 1 | 128, 1}); };

        require(wait(first) == 0 && wait(second) == 0, "legacy same-key wait setup");
        if (operation == 4)
            require(requeue(0, 1, 1) == -11, "legacy same-key comparison mismatch");
        require(requeue(0, 0) == 0, "legacy same-key zero counts");
        require(requeue(0, 1) == 1, "legacy same-key requeue count");
        require(wake() == 1 && f.task(first).state == Task::State::Runnable &&
                    f.task(second).state == Task::State::Waiting,
                "legacy same-key requeue changed FIFO order");
        require(wake() == 1, "legacy same-key cleanup");

        require(wait(first) == 0 && wait(second) == 0 && wait(third) == 0,
                "legacy same-key wake and requeue setup");
        require(requeue(1, 1) == 2 && f.task(first).state == Task::State::Runnable &&
                    f.task(second).state == Task::State::Waiting,
                "legacy same-key wake and requeue count");
        require(wake() == 1 && f.task(second).state == Task::State::Runnable &&
                    f.task(third).state == Task::State::Waiting,
                "legacy same-key wake and requeue order");
        require(wake() == 1, "legacy same-key wake and requeue cleanup");

        require(wait(first) == 0 && f.call(second, 98, {b, 128, 0}) == 0 && wait(third) == 0,
                "legacy different-key requeue setup");
        require(f.call(1, 98, {a, operation | 128, 0, 1, b, 0}) == 1,
                "legacy different-key requeue count");
        require(f.call(1, 98, {b, 1 | 128, 1}) == 1 &&
                    f.task(second).state == Task::State::Runnable && f.task(first).state == Task::State::Waiting,
                "legacy different-key target order");
        require(f.call(1, 98, {b, 1 | 128, 1}) == 1 &&
                    f.task(first).state == Task::State::Runnable && f.task(third).state == Task::State::Waiting,
                "legacy different-key moved waiter");
        require(wake() == 1, "legacy different-key source cleanup");

        for (const auto tid : {second, third}) {
            f.task(tid).sched_policy = Task::SchedulingPolicy::Fifo;
            f.task(tid).rt_priority = 80;
            f.scheduler.scheduling_changed(tid);
        }
        require(wait(first) == 0 && wait(second) == 0 && wait(third) == 0,
                "legacy realtime same-key setup");
        require(requeue(0, 1) == 1 && wake() == 1 &&
                    f.task(second).state == Task::State::Runnable && f.task(third).state == Task::State::Waiting,
                "legacy realtime same-key order");
        require(wake() == 1 && f.task(third).state == Task::State::Runnable &&
                    f.task(first).state == Task::State::Waiting,
                "legacy realtime same-key priority");
        require(wake() == 1, "legacy realtime same-key cleanup");
        for (const auto tid : {second, third}) {
            f.task(tid).sched_policy = Task::SchedulingPolicy::Other;
            f.task(tid).rt_priority = 0;
            f.scheduler.scheduling_changed(tid);
        }

        f.entry(0, a);
        f.entry(1, a);
        require(f.call(first, 449, {array, 2}) == 0 && wait(second) == 0,
                "legacy same-key duplicate vector setup");
        require(requeue(1, 1) == 2 && f.result(first) == 0 &&
                    f.task(second).state == Task::State::Waiting,
                "legacy same-key duplicate vector count");
        require(wake() == 1, "legacy same-key duplicate vector cleanup");

        require(wait(first) == 0 && f.call(second, 98, {a, 11 | 128, 0, 0, b}) == 0,
                "legacy same-key PI waiter setup");
        require(requeue(1, 1) == -22 && f.task(first).state == Task::State::Runnable &&
                    f.task(second).state == Task::State::Waiting,
                "legacy same-key PI error partial wake");
        f.scheduler.interrupt(second);

        std::ofstream(f.root + "/legacy-same-key") << std::string(4096, '\0');
        FD backing = f.fs->resolve("/legacy-same-key").open(O_RDWR);
        f.memory.map_file(0x30000, 4096, read_perm | write_perm, backing.get(),
                          {"/legacy-same-key", 0, 0, 0, true});
        f.memory.map_file(0x40000, 4096, read_perm | write_perm, backing.get(),
                          {"/legacy-same-key", 0, 0, 0, true});
        require(f.call(first, 98, {0x30000, 0, 0}) == 0 && f.call(second, 98, {0x30000, 0, 0}) == 0,
                "legacy shared alias wait setup");
        require(f.call(1, 98, {0x30000, operation, 0, 1, 0x40000, 0}) == 1,
                "legacy shared alias requeue count");
        require(f.call(1, 98, {0x40000, 1, 1}) == 1 &&
                    f.task(first).state == Task::State::Runnable && f.task(second).state == Task::State::Waiting,
                "legacy shared alias requeue changed FIFO order");
        require(f.call(1, 98, {0x30000, 1, 1}) == 1, "legacy shared alias cleanup");
    }
}
void shared_identity() {
    Fixture f;
    std::ofstream(f.root + "/shared") << std::string(4096, '\0');
    FD backing = f.fs->resolve("/shared").open(O_RDWR);
    f.memory.map_file(0x30000, 4096, read_perm | write_perm, backing.get(), {"/shared", 0, 0, 0, true});
    Memory peer{16 << 20};
    peer.map(0x50000, 4096, read_perm | write_perm);
    peer.map_file(0x40000, 4096, read_perm | write_perm, backing.get(), {"/shared", 0, 0, 0, true});
    ort_arm64_config config{};
    config.root_directory = f.root.c_str();
    Syscalls peer_sys(peer, *f.fs, config, f.scheduler);
    const auto tid = f.scheduler.spawn_process(CpuState{}, 0x18000, 0, 0);
    uint32_t peer_waker;
    {
        Scheduler::ScopedTask scope(f.scheduler, tid);
        peer_waker = f.scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    }
    Fixture::entry(peer, 0x50000, 0, 0x40000, 0, u32_shared);
    require(f.call(tid, 449, {0x50000, 1}, &peer_sys) == 0, "shared vector setup");
    require(f.call(1, 454, {0x30000, UINT32_MAX, 1, u32_shared}) == 1 && f.result(tid) == 0,
            "shared file identity across address spaces");
    Fixture::entry(peer, 0x50000, 0, 0x40000, 0, u32_private);
    require(f.call(tid, 449, {0x50000, 1}, &peer_sys) == 0, "private vector setup");
    require(f.call(1, 454, {0x30000, UINT32_MAX, 1, u32_private}) == 0 &&
                f.call(1, 454, {0x30000, UINT32_MAX, 1, u32_shared}) == 0,
            "private vector escaped its address space");
    require(f.call(peer_waker, 454, {0x40000, UINT32_MAX, 1, u32_private}, &peer_sys) == 1,
            "private peer wake");
}
void concurrent_compare_and_enqueue() {
    Fixture f;
    const auto tid = f.spawn();
    f.entry(0, a);
    f.entry(1, b);
    for (unsigned iteration = 0; iteration < 120; ++iteration) {
        f.memory.store(a, 0, 4);
        std::promise<void> start;
        auto ready = start.get_future();
        std::thread writer([&] {
            ready.wait();
            f.memory.store(a, 1, 4);
        });
        start.set_value();
        const auto result = f.call(tid, 449, {array, 2});
        writer.join();
        require(result == 0 || result == -11, "concurrent compare result");
        if (result == 0) {
            require(f.task(tid).state == Task::State::Waiting, "concurrent wait was not queued");
            require(f.call(1, 454, {b, UINT32_MAX, 1, u32_private}) == 1 && f.result(tid) == 1,
                    "concurrent wait lost wake");
        }
        require(f.call(1, 454, {a, UINT32_MAX, 1, u32_private}) == 0, "concurrent wait left queue entry");
    }
}
} // namespace
int main(int argc, char **argv) {
    try {
        if (argc == 2 && std::strcmp(argv[1], "--key-regressions") == 0) {
            key_regressions();
            return 0;
        }
        if (argc == 2 && std::strcmp(argv[1], "--pi-regressions") == 0) {
            pi_interoperation();
            return 0;
        }
        if (argc == 2 && std::strcmp(argv[1], "--scheduler-regression") == 0) {
            scheduler_requeue_order();
            return 0;
        }
        if (argc == 2 && std::strcmp(argv[1], "--legacy-regression") == 0) {
            legacy_same_key_requeue();
            return 0;
        }
        require(argc == 1, "unknown test argument");
        validation();
        wakes_and_requeues();
        cleanup_and_timeouts();
        pi_interoperation();
        key_regressions();
        scheduler_requeue_order();
        legacy_same_key_requeue();
        shared_identity();
        concurrent_compare_and_enqueue();
        std::puts("futex2 module regressions passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "futex2: %s\n", error.what());
        return 1;
    }
}
