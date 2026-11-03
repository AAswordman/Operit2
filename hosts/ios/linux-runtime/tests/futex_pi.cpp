#include "linux/signals/signals.h"
#include "linux/syscalls.h"
#include "linux/threads/futex_pi.h"
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fcntl.h>
#include <fstream>
#include <stdexcept>
#include <time.h>

using namespace operit::arm64;
namespace {
constexpr uint64_t condition = 0x11000, mutex_word = 0x11004, other_mutex = 0x11008, timeout_word = 0x12000;
constexpr uint32_t private_flag = 128;
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Fixture {
    std::string root;
    Memory memory{8 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler scheduler;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/operit-futex-pi-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        root = created;
        fs = std::make_unique<Filesystem>(root.c_str());
        memory.map(0x10000, 0x8000, read_perm | write_perm);
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
    uint32_t word(uint64_t address = mutex_word) { return uint32_t(memory.load(address, 4)); }
    int64_t futex(uint32_t tid, uint64_t address, uint32_t op, uint32_t value = 0,
                   uint64_t timeout_or_count = 0, uint64_t target = 0, uint32_t comparison = 0) {
        Scheduler::ScopedTask scope(scheduler, tid);
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        cpu.pc = 0x40004;
        cpu.sp = 0x18000;
        cpu.x[0] = address;
        cpu.x[1] = op;
        cpu.x[2] = value;
        cpu.x[3] = timeout_or_count;
        cpu.x[4] = target;
        cpu.x[5] = comparison;
        cpu.x[8] = 98;
        signals::remember_syscall(scheduler.current().signals, cpu, true);
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    int64_t wait(uint32_t tid, uint64_t target = mutex_word, uint64_t timeout = 0,
                 uint32_t flags = private_flag) {
        return futex(tid, condition, 11 | flags, 0, timeout, target);
    }
    int64_t requeue(uint32_t count, uint64_t target = mutex_word, uint32_t expected = 0,
                    uint32_t flags = private_flag) {
        return futex(1, condition, 12 | flags, 1, count, target, expected);
    }
    void lock(uint32_t tid, uint64_t target = mutex_word, uint32_t flags = private_flag) {
        require(futex(tid, target, 6 | flags) == 0, "FUTEX_LOCK_PI");
    }
    void unlock(uint32_t tid, uint64_t target = mutex_word, uint32_t flags = private_flag) {
        require(futex(tid, target, 7 | flags) == 0, "FUTEX_UNLOCK_PI");
    }
    void high_priority(uint32_t tid, unsigned priority = 80) {
        task(tid).sched_policy = Task::SchedulingPolicy::Fifo;
        task(tid).rt_priority = priority;
        scheduler.scheduling_changed(tid);
    }
    void future_timeout(bool realtime) {
        timespec now{};
        require(::clock_gettime(realtime ? CLOCK_REALTIME : CLOCK_MONOTONIC, &now) == 0, "clock_gettime");
        memory.store(timeout_word, uint64_t(now.tv_sec) + 60, 8);
        memory.store(timeout_word + 8, uint64_t(now.tv_nsec), 8);
    }
};
void validation() {
    Fixture f;
    require(f.futex(1, condition, 12 | private_flag, 0, 1, mutex_word) == -22, "CMP_REQUEUE_PI accepted nr_wake=0");
    require(f.futex(1, condition, 12 | private_flag, 2, 1, mutex_word) == -22, "CMP_REQUEUE_PI accepted nr_wake=2");
    require(f.futex(1, condition, 12 | private_flag, 1, UINT32_MAX, mutex_word) == -22, "negative requeue count");
    require(f.wait(1, condition) == -22 && f.requeue(0, condition) == -22, "same address accepted");
    require(f.wait(1, mutex_word + 1) == -22 && f.requeue(0, mutex_word + 1) == -22, "unaligned target accepted");
    require(f.wait(1, 0x50000) == -14 && f.requeue(0, 0x50000) == -14, "unmapped target accepted");
    require(f.futex(1, condition, 11 | private_flag, 1, 0, mutex_word) == -11, "WAIT_REQUEUE_PI value comparison");
    require(f.requeue(0, mutex_word, 1) == -11, "CMP_REQUEUE_PI value comparison");
    require(f.requeue(0) == 0 && !f.word(), "empty source changed target");
    f.memory.store(timeout_word, 0, 8);
    f.memory.store(timeout_word + 8, 1000000000, 8);
    require(f.wait(1, mutex_word, timeout_word) == -22, "invalid timeout nanoseconds");
    f.memory.store(timeout_word + 8, 0, 8);
    require(f.wait(1, mutex_word, timeout_word) == -110, "absolute monotonic timeout");
    require(f.wait(1, mutex_word, timeout_word, private_flag | 256) == -110, "absolute realtime timeout");
    f.memory.protect(0x13000, 4096, read_perm);
    require(f.wait(1, 0x13000) == -14, "read-only PI target accepted");
    // The source is read-only; only the destination must be writable.
    f.memory.protect(0x11000, 4096, read_perm);
    require(f.futex(1, condition, 11 | private_flag, 1, 0, 0x10000) == -11, "read-only condition word rejected");
}
void free_target_and_fifo_handoff() {
    Fixture f;
    const auto second = f.spawn(), first = f.spawn(); // Queue order must beat TID order.
    f.task(second).nice = -15; // Non-RT source entries still use one FIFO priority.
    require(f.wait(first) == 0 && f.wait(second) == 0, "wait setup");
    require(f.task(first).state == Task::State::Waiting && !f.word(), "condition wait acquired mutex prematurely");
    require(f.requeue(1) == 2, "free target requeue count");
    require((f.word() & threads::futex_tid_mask) == first && (f.word() & threads::futex_waiters), "top waiter not made owner");
    require(f.task(first).state == Task::State::Runnable && f.task(second).state == Task::State::Waiting,
            "more than the acquiring waiter woke");
    require(f.scheduler.effective_priority(first) == 105, "requeued waiter did not donate priority");
    f.unlock(first);
    require((f.word() & threads::futex_tid_mask) == second && f.task(second).state == Task::State::Runnable &&
                int64_t(f.task(second).cpu.x[0]) == 0, "unlock did not hand off PI ownership");
    require(f.scheduler.effective_priority(first) == 120, "priority donation survived handoff");
    f.unlock(second);
    require(f.word() == 0, "final unlock did not clear the PI word");
}
void contended_target_and_realtime_order() {
    Fixture f;
    const auto ordinary = f.spawn(), realtime = f.spawn();
    f.high_priority(realtime);
    f.lock(1);
    require(f.wait(ordinary) == 0 && f.wait(realtime) == 0, "condition wait setup");
    require(f.scheduler.effective_priority(1) == 120, "condition wait donated before requeue");
    require(f.requeue(0) == 1, "contended nr_requeue=0 must still move the top waiter");
    require(f.task(realtime).state == Task::State::Waiting && !f.task(realtime).futex_requeue_pi &&
                f.task(ordinary).futex_requeue_pi, "source realtime ordering or requeue phase");
    require((f.word() & threads::futex_tid_mask) == 1 && f.scheduler.effective_priority(1) == 19,
            "contended requeue woke a non-owner or failed to boost owner");
    f.unlock(1);
    require((f.word() & threads::futex_tid_mask) == realtime, "realtime waiter not handed ownership");
    f.unlock(realtime);
    require(f.requeue(0) == 1 && (f.word() & threads::futex_tid_mask) == ordinary,
            "remaining source waiter was lost");
    f.unlock(ordinary);
}
void donation_changes_realtime_queue_order() {
    Fixture f;
    const auto donor = f.spawn(), peer = f.spawn();
    f.high_priority(1, 20);
    f.high_priority(peer, 20);
    f.high_priority(donor, 80);
    f.lock(1);
    require(f.wait(donor) == 0 && f.scheduler.select(false) && f.scheduler.current_tid() == 1,
            "PI queue setup did not block the higher-priority condition waiter");
    f.scheduler.yield();
    require(f.scheduler.select(false) && f.scheduler.current_tid() == peer,
            "FIFO peer did not run before PI migration");
    require(f.requeue(0) == 1 && f.scheduler.select(false) && f.scheduler.current_tid() == 1,
            "PI donation did not move the owner ahead of a runnable FIFO peer");
    f.unlock(1);
    require(f.scheduler.effective_priority(1) == 79 && f.scheduler.select(false) &&
                f.scheduler.current_tid() == donor, "PI handoff did not remove the owner's boost");
    f.scheduler.block({}, 0);
    require(f.scheduler.select(false) && f.scheduler.current_tid() == 1,
            "PI deboost did not join the head of the owner's base-priority FIFO queue");
    f.scheduler.wake(donor);
    f.unlock(donor);
}
void pi_boosted_watchdog_service_and_deboost() {
    Fixture f;
    const auto donor = f.spawn(), peer = f.spawn();
    f.high_priority(donor, 80);
    f.high_priority(peer, 60);
    f.lock(1);
    require(f.wait(donor) == 0 && f.scheduler.select(false) && f.scheduler.current_tid() == peer,
            "PI watchdog setup incorrectly donated before migration");
    require(f.requeue(0) == 1 && f.scheduler.select(false) && f.scheduler.current_tid() == 1,
            "PI watchdog owner did not enter the RT class");
    f.scheduler.account_runtime(1, 10'000, true);
    require(f.task(1).sched_policy == Task::SchedulingPolicy::Other &&
                f.task(1).realtime_runtime_nanoseconds == 10'000,
            "ordinary PI owner did not account temporary realtime CPU service");
    f.unlock(1);
    require(f.scheduler.select(false) && f.scheduler.current_tid() == donor &&
                !f.task(1).realtime_runtime_nanoseconds,
            "PI owner retained its realtime watchdog after leaving the RT class");
    f.unlock(donor);
}
void mixed_operations_and_partial_transfer() {
    Fixture f;
    const auto ordinary = f.spawn();
    require(f.futex(ordinary, condition, private_flag) == 0, "ordinary wait setup");
    require(f.requeue(0) == -22 && !f.word(), "ordinary WAIT accepted by CMP_REQUEUE_PI");
    require(f.futex(1, condition, 1 | private_flag, 1) == 1, "ordinary waiter stranded by failed requeue");
    const auto pi = f.spawn();
    require(f.wait(pi) == 0, "PI condition wait setup");
    require(f.futex(1, condition, 1 | private_flag, 1) == -22 && f.task(pi).state == Task::State::Waiting,
            "ordinary WAKE falsely completed a PI condition wait");
    require(f.futex(1, condition, 3 | private_flag, 0, 1, other_mutex) == -22,
            "ordinary REQUEUE accepted a PI condition wait");
    require(f.requeue(0, other_mutex) == -22 && !f.word(other_mutex), "wrong PI target accepted");
    require(f.futex(ordinary, condition, private_flag) == 0, "mixed tail setup");
    require(f.requeue(1) == -22, "mixed tail did not report EINVAL");
    require((f.word() & threads::futex_tid_mask) == pi && f.task(pi).state == Task::State::Runnable &&
                f.task(ordinary).state == Task::State::Waiting,
            "failed mixed requeue rolled back or corrupted the completed prefix");
    f.unlock(pi);
    require(f.futex(1, condition, 1 | private_flag, 1) == 1, "ordinary mixed tail stranded");
}
void timeout_and_signal_phases() {
    Fixture f;
    const auto waiter = f.spawn();
    f.high_priority(waiter);
    f.future_timeout(false);
    require(f.wait(waiter, mutex_word, timeout_word) == 0, "timed source setup");
    f.task(waiter).deadline = TaskClock::now(); // Exercise cleanup without sleeping.
    f.scheduler.poll_waiting();
    require(int64_t(f.task(waiter).cpu.x[0]) == -110 && f.requeue(1) == 0, "source timeout left a queued waiter");
    f.lock(1);
    require(f.wait(waiter, mutex_word, timeout_word) == 0 && f.requeue(0) == 1, "timed target setup");
    require(f.scheduler.effective_priority(1) == 19, "timed waiter donation");
    f.task(waiter).deadline = TaskClock::now();
    f.scheduler.poll_waiting();
    require(int64_t(f.task(waiter).cpu.x[0]) == -110 && f.scheduler.effective_priority(1) == 120,
            "target timeout retained PI donation");
    require(f.wait(waiter) == 0 && f.task(waiter).signals.restart->always_restart, "source restart setup");
    f.scheduler.interrupt(waiter);
    require(int64_t(f.task(waiter).cpu.x[0]) == -4 && f.task(waiter).signals.restart->always_restart,
            "source signal did not preserve unconditional restart");
    require(f.wait(waiter) == 0 && f.requeue(0) == 1, "signal target setup");
    f.scheduler.interrupt(waiter);
    require(int64_t(f.task(waiter).cpu.x[0]) == -11 && !f.task(waiter).signals.restart->restartable &&
                f.scheduler.effective_priority(1) == 120, "signal after requeue must return EAGAIN and deboost");
    f.unlock(1);
    f.future_timeout(true);
    require(f.wait(waiter, mutex_word, timeout_word, private_flag | 256) == 0 && f.task(waiter).poll,
            "realtime source wait lacks an absolute-clock poll");
    f.scheduler.interrupt(waiter);
}
void early_signal_and_cancellation() {
    Fixture f;
    const auto waiter = f.spawn();
    require(f.futex(waiter, condition, 11 | private_flag, 1, 0, mutex_word) == -11 &&
                !f.task(waiter).signals.restart->always_restart, "value mismatch marked as unconditional restart");
    {
        Scheduler::ScopedTask scope(f.scheduler, waiter);
        signals::interrupted(f.task(waiter).signals, f.task(waiter).signal_mask);
        int error = 0;
        try {
            threads::futex_wait_requeue_pi(f.scheduler, f.memory, condition, 0, mutex_word, true,
                                           std::nullopt, [] { return true; });
        } catch (const GuestError &failure) {
            error = failure.number;
        }
        require(error == 4 && f.task(waiter).state == Task::State::Runnable &&
                    f.task(waiter).signals.restart->always_restart && !f.task(waiter).wait_cleanup,
                "already-pending signal queued a wait or lost unconditional restart");
    }
    require(f.requeue(0) == 0, "already-pending signal left a source entry");
    for (bool migrated : {false, true}) {
        if (migrated)
            f.lock(1);
        f.high_priority(waiter);
        require(f.wait(waiter) == 0, "cancel wait setup");
        if (migrated)
            require(f.requeue(0) == 1 && f.scheduler.effective_priority(1) == 19, "cancel target setup");
        f.scheduler.interrupt(waiter, -125); // ECANCELED is not a signal interruption.
        require(int64_t(f.task(waiter).cpu.x[0]) == -125 && !f.task(waiter).wait_cleanup &&
                    f.scheduler.effective_priority(1) == 120 && f.requeue(0) == 0,
                "non-signal cancellation changed errno or retained a waiter/donation");
        if (migrated)
            f.unlock(1);
    }
}
void signal_frame_restart() {
    for (unsigned phase = 0; phase != 3; ++phase) {
        Fixture f;
        const auto waiter = f.spawn();
        if (phase == 2)
            f.lock(1);
        if (phase == 0)
            require(f.futex(waiter, condition, private_flag) == 0, "ordinary signal-frame wait setup");
        else
            require(f.wait(waiter) == 0, "PI signal-frame wait setup");
        if (phase == 2)
            require(f.requeue(0) == 1, "signal-frame requeue setup");
        auto &task = f.task(waiter);
        signals::interrupted(task.signals, task.signal_mask);
        f.scheduler.interrupt(waiter);
        signals::ProcessState process;
        process.actions->entries[9].handler = 0x41000; // Linux SIGUSR1.
        process.actions->entries[9].flags = phase == 2 ? signals::sa_restart : 0;
        require(process.pending.enqueue(signals::Info::user(10, 1, 0)), "signal enqueue");
        const auto result = signals::deliver(f.memory, task.cpu, process, task.signals, task.signal_mask, 0x42000);
        require(result.effect == signals::Effect::Handler, "signal handler frame not entered");
        require(signals::restore_frame(f.memory, task.cpu, task.signals, task.signal_mask), "signal frame restore");
        if (phase == 1)
            require(task.cpu.pc == 0x40000 && task.cpu.x[8] == 98 && task.cpu.x[0] == condition &&
                        task.cpu.x[1] == (11 | private_flag), "WAIT_REQUEUE_PI did not restart without SA_RESTART");
        else
            require(task.cpu.pc == 0x40004 && int64_t(task.cpu.x[0]) == (phase == 2 ? -11 : -4),
                    "ordinary EINTR or post-requeue EAGAIN was incorrectly restarted");
        if (phase == 2)
            f.unlock(1);
    }
}
void priority_chain_and_owner_exit() {
    Fixture f;
    const auto middle = f.spawn(), high = f.spawn();
    f.high_priority(high);
    f.lock(1);
    f.lock(middle, other_mutex);
    f.lock(middle); // Blocks while holding other_mutex.
    require(f.task(middle).state == Task::State::Waiting, "PI chain middle did not block");
    require(f.wait(high, other_mutex) == 0 && f.requeue(0, other_mutex) == 1, "PI chain requeue");
    require(f.scheduler.effective_priority(middle) == 19 && f.scheduler.effective_priority(1) == 19,
            "requeue donation did not traverse the PI chain");
    f.unlock(1);
    require((f.word() & threads::futex_tid_mask) == middle, "PI chain first handoff");
    threads::futex_pi_task_exit(f.scheduler, f.memory, f.task(middle));
    require((f.word(other_mutex) & threads::futex_tid_mask) == high &&
                (f.word(other_mutex) & threads::futex_owner_died) && f.task(high).state == Task::State::Runnable,
            "owner exit did not hand off the requeued PI waiter with OWNER_DIED");
    f.unlock(high, other_mutex);
}
void deadlock_and_waiter_exit() {
    Fixture f;
    const auto waiter = f.spawn();
    f.lock(waiter);
    require(f.wait(waiter) == 0 && f.requeue(0) == -35, "requeue to own mutex did not detect deadlock");
    require(f.task(waiter).futex_requeue_pi && (f.word() & threads::futex_tid_mask) == waiter,
            "deadlock error corrupted pending condition wait");
    f.scheduler.interrupt(waiter);
    f.unlock(waiter);
    f.lock(1);
    f.high_priority(waiter);
    require(f.wait(waiter) == 0 && f.requeue(0) == 1, "exit waiter setup");
    require(f.scheduler.effective_priority(1) == 19, "exit waiter donation setup");
    f.scheduler.discard(waiter);
    require(f.scheduler.effective_priority(1) == 120, "waiter exit retained PI donation");
    f.unlock(1);
    require(!f.word(), "exiting waiter stranded the mutex");
}
void ordinary_source_ordering_and_same_key_requeue() {
    Fixture f;
    const auto later = f.spawn(), earlier = f.spawn(), realtime = f.spawn();
    f.task(later).nice = -15;
    f.high_priority(realtime);
    require(f.futex(earlier, condition, 9 | private_flag, 0, 0, 0, 1) == 0 &&
                f.futex(later, condition, 9 | private_flag, 0, 0, 0, 2) == 0 &&
                f.futex(realtime, condition, 9 | private_flag, 0, 0, 0, 1) == 0,
            "bitset queue setup");
    require(f.futex(1, condition, 10 | private_flag, 1, 0, 0, 2) == 1 &&
                f.task(later).state == Task::State::Runnable &&
                f.task(realtime).state == Task::State::Waiting, "WAKE_BITSET ignored the selected mask");
    require(f.futex(later, condition, private_flag) == 0, "ordinary FIFO tail setup");
    require(f.futex(1, condition, 1 | private_flag, 1) == 1 &&
                f.task(realtime).state == Task::State::Runnable, "ordinary wake ignored realtime priority");
    require(f.futex(1, condition, 3 | private_flag, 0, UINT32_MAX >> 1, condition) == 2,
            "same-key requeue selected a moved task more than once");
    require(f.futex(1, condition, 1 | private_flag, 1) == 1 &&
                f.task(earlier).state == Task::State::Runnable &&
                f.task(later).state == Task::State::Waiting, "non-RT wake used TID or nice instead of FIFO");
    require(f.futex(1, condition, 1 | private_flag, 1) == 1, "ordinary FIFO tail stranded");
}
void shared_process_handoff_and_mapping_cleanup() {
    Fixture f;
    Memory peer_memory{8 << 20};
    std::ofstream(f.root + "/shared-process") << std::string(4096, '\0');
    FD backing = f.fs->resolve("/shared-process").open(O_RDWR);
    f.memory.map_file(0x20000, 4096, read_perm | write_perm, backing.get(),
                      {"/shared-process", 0, 0, 0, true});
    peer_memory.map_file(0x30000, 4096, read_perm | write_perm, backing.get(),
                         {"/shared-process", 0, 0, 0, true});
    const auto peer = f.scheduler.spawn_process(CpuState{}, 0, 0, 0);
    f.high_priority(peer);
    f.lock(1, 0x20004, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, peer);
        require(threads::futex_wait_requeue_pi(f.scheduler, peer_memory, 0x30000, 0, 0x30004, false) == 0,
                "shared process wait");
    }
    require(f.futex(1, 0x20000, 12, 1, 0, 0x20004) == 1 &&
                f.scheduler.effective_priority(1) == 19, "shared process requeue or donation failed");
    f.unlock(1, 0x20004, 0);
    require(f.task(peer).state == Task::State::Runnable &&
                (peer_memory.load(0x30004, 4) & threads::futex_tid_mask) == peer,
            "shared process handoff failed to publish ownership");
    {
        Scheduler::ScopedTask scope(f.scheduler, peer);
        require(threads::futex_pi_unlock(f.scheduler, peer_memory, 0x30004, false) == 0,
                "shared process unlock");
        require(threads::futex_wait_requeue_pi(f.scheduler, peer_memory, 0x30000, 0, 0x30004, true) == 0,
                "private process wait");
    }
    require(f.futex(1, 0x20000, 12 | private_flag, 1, 0, 0x20004) == 0 &&
                f.task(peer).futex_requeue_pi, "private futex crossed address spaces");
    f.scheduler.interrupt(peer);
    f.lock(1, 0x20004, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, peer);
        require(threads::futex_wait_requeue_pi(f.scheduler, peer_memory, 0x30000, 0, 0x30004, false) == 0,
                "mapping cleanup wait");
    }
    require(f.futex(1, 0x20000, 12, 1, 0, 0x20004) == 1, "mapping cleanup requeue");
    peer_memory.unmap(0x30000, 4096);
    f.unlock(1, 0x20004, 0);
    require(f.task(peer).state == Task::State::Runnable && int64_t(f.task(peer).cpu.x[0]) == -14 &&
                f.scheduler.effective_priority(1) == 120 && !f.word(0x20004),
            "inaccessible target retained a waiter, donation or ownership");
}
void shared_keys_and_aliases() {
    Fixture f;
    std::ofstream(f.root + "/shared") << std::string(4096, '\0');
    FD backing = f.fs->resolve("/shared").open(O_RDWR);
    f.memory.map_file(0x20000, 4096, read_perm | write_perm, backing.get(), {"/shared", 0, 0, 0, true});
    f.memory.map_file(0x21000, 4096, read_perm | write_perm, backing.get(), {"/shared", 0, 0, 0, true});
    require(f.futex(1, 0x20000, 11, 0, 0, 0x21000) == -22 &&
                f.futex(1, 0x20000, 12, 1, 0, 0x21000) == -22, "shared aliases treated as distinct futex keys");
    const auto waiter = f.spawn();
    require(f.wait(waiter, 0x20000, 0, 0) == 0 && f.requeue(0, 0x21000, 0, 0) == 1,
            "matching shared target aliases failed to requeue");
    require((f.word(0x20000) & threads::futex_tid_mask) == waiter &&
                (f.word(0x21000) & threads::futex_tid_mask) == waiter, "shared PI ownership not visible through both aliases");
    f.unlock(waiter, 0x20000, 0);
    require(f.futex(waiter, 0x20000, 11 | private_flag, 0, 0, 0x21000) == 0 &&
                f.task(waiter).state == Task::State::Waiting, "private futex aliases incorrectly shared identity");
    f.scheduler.interrupt(waiter);
}
} // namespace
int main() {
    try {
        validation();
        free_target_and_fifo_handoff();
        contended_target_and_realtime_order();
        donation_changes_realtime_queue_order();
        pi_boosted_watchdog_service_and_deboost();
        mixed_operations_and_partial_transfer();
        timeout_and_signal_phases();
        early_signal_and_cancellation();
        signal_frame_restart();
        priority_chain_and_owner_exit();
        deadlock_and_waiter_exit();
        ordinary_source_ordering_and_same_key_requeue();
        shared_process_handoff_and_mapping_cleanup();
        shared_keys_and_aliases();
        std::puts("PI futex requeue, ownership, donation, timeout, signal and exit passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
