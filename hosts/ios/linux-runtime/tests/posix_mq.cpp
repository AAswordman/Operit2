#include "fs/descriptor_table.h"
#include "linux/ipc/posix_mq.h"
#include "linux/signals/signals.h"
#include "memory/address_space.h"
#include "memory/fault.h"
#include "runtime/tasks/scheduler.h"
#include <array>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <string>
#include <thread>
#include <time.h>

using namespace operit::arm64;
namespace mq = operit::arm64::ipc;
namespace {
constexpr uint64_t name = 0x10000, attr = 0x11000, data = 0x12000, output = 0x13000;
constexpr uint64_t priority = 0x14000, timeout = 0x15000, event = 0x16000;
constexpr uint32_t create = 64, exclusive = 128, nonblock = 2048;

void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
template <typename F> int64_t result(F operation) {
    try {
        return operation();
    } catch (const GuestError &error) {
        return -error.number;
    } catch (const MemoryFault &) {
        return -14;
    }
}
struct Fixture {
    Memory memory{8 << 20};
    DescriptorTable descriptors;
    Scheduler scheduler;
    std::shared_ptr<mq::PosixMqAccounting> accounting = mq::make_posix_mq_accounting();
    std::shared_ptr<mq::PosixMqNamespace> space = mq::make_posix_mq_namespace(accounting);
    mq::PosixMqContext context;
    std::shared_ptr<signals::ProcessState> signal_state = std::make_shared<signals::ProcessState>();
    signals::Hooks hooks;

    Fixture() {
        memory.map(name, 0x9000, read_perm | write_perm);
        context.credentials = {1000, 1000, 100, {}, false, false, false};
        context.interrupt_pending = [] { return false; };
        hooks.process = [this](uint32_t pid) {
            if (pid != 1)
                guest_error(3);
            auto &task = *scheduler.lookup(1);
            signals::ProcessRef process;
            process.pid = 1;
            process.signals = signal_state;
            process.threads.push_back({task.tid, task.pid, &task.cpu, &task.signals, &task.signal_mask});
            return process;
        };
        hooks.wake = [this](uint32_t tid, bool interrupt) {
            if (interrupt)
                scheduler.interrupt(tid);
        };
        context.signal_hooks = &hooks;
    }
    void put_string(uint64_t address, const std::string &value) {
        memory.copy_in(address, value.c_str(), value.size() + 1);
    }
    void attributes(uint64_t max_messages, uint64_t message_size) {
        std::array<uint64_t, 8> values{0, max_messages, message_size};
        memory.copy_in(attr, values.data(), sizeof(values));
    }
    void notification(int method, int signal = 0, uint64_t value = 0) {
        std::array<uint8_t, 64> bytes{};
        std::memcpy(bytes.data(), &value, 8);
        std::memcpy(bytes.data() + 8, &signal, 4);
        std::memcpy(bytes.data() + 12, &method, 4);
        memory.copy_in(event, bytes.data(), bytes.size());
    }
    int64_t call(uint32_t tid, uint64_t number, uint64_t a0, uint64_t a1 = 0,
                 uint64_t a2 = 0, uint64_t a3 = 0, uint64_t a4 = 0) {
        Scheduler::ScopedTask scope(scheduler, tid);
        try {
            return result([&] {
                return mq::posix_mq_syscall(space, memory, descriptors, scheduler, context,
                                            number, a0, a1, a2, a3, a4);
            });
        } catch (const Failure &error) {
            throw std::runtime_error(std::string(error.what()) + " tid=" + std::to_string(tid) +
                                     " syscall=" + std::to_string(number) +
                                     " state=" + std::to_string(int(scheduler.lookup(tid)->state)) +
                                     " cleanup=" + std::to_string(bool(scheduler.lookup(tid)->wait_cleanup)));
        }
    }
    int open(const char *queue_name, uint32_t flags = create | 2, uint32_t mode = 0660,
             uint64_t attribute_address = attr) {
        put_string(name, queue_name);
        return int(call(1, 180, name, flags, mode, attribute_address));
    }
    int64_t send(uint32_t tid, int fd, const std::string &value, uint32_t prio = 0, uint64_t until = 0) {
        if (!value.empty())
            memory.copy_in(data, value.data(), value.size());
        return call(tid, 182, uint32_t(fd), data, value.size(), prio, until);
    }
    int64_t receive(uint32_t tid, int fd, uint64_t address = output, uint64_t prio = priority,
                    uint64_t until = 0, uint64_t length = 16) {
        return call(tid, 183, uint32_t(fd), address, length, prio, until);
    }
    std::string received(size_t length, uint64_t address = output) {
        std::string value(length, '\0');
        if (length)
            memory.copy_out(address, value.data(), length);
        return value;
    }
    uint32_t spawn() { return scheduler.spawn(CpuState{}, 0x18000, 0, 0); }
    Task &task(uint32_t tid) { return *scheduler.lookup(tid); }
    void past() {
        memory.store(timeout, 0, 8);
        memory.store(timeout + 8, 0, 8);
    }
    void future(unsigned milliseconds) {
        timespec now{};
        require(::clock_gettime(CLOCK_REALTIME, &now) == 0, "clock_gettime");
        uint64_t ns = uint64_t(now.tv_nsec) + uint64_t(milliseconds) * 1000000;
        memory.store(timeout, uint64_t(now.tv_sec) + ns / 1000000000, 8);
        memory.store(timeout + 8, ns % 1000000000, 8);
    }
};

void priority_flags_and_readiness() {
    Fixture f;
    f.attributes(3, 16);
    const int fd = f.open("order", create | 2 | nonblock);
    require(fd >= 0 && f.descriptors.get(fd).descriptor_flags == 1, "open and forced CLOEXEC");
    auto description = f.descriptors.get(fd).description;
    require(mq::is_posix_mq_file(description) && !mq::is_posix_mq_file({}), "descriptor type detection");
    require(mq::posix_mq_poll(description, 1 | 4 | 64 | 256) == (4 | 256), "empty readiness");
    require(f.send(1, fd, "a", 2) == 0 && f.send(1, fd, "b", 8) == 0 &&
            f.send(1, fd, "c", 8) == 0, "send priorities");
    require(mq::posix_mq_poll(description, 1 | 4 | 64 | 256) == (1 | 64), "full readiness");
    require(f.send(1, fd, "d") == -11, "nonblocking full");
    require(f.receive(1, fd) == 1 && f.received(1) == "b" && f.memory.load(priority, 4) == 8,
            "highest priority");
    require(f.receive(1, fd) == 1 && f.received(1) == "c", "same-priority FIFO");
    require(f.receive(1, fd) == 1 && f.received(1) == "a" && f.memory.load(priority, 4) == 2,
            "lower priority");
    require(f.receive(1, fd) == -11, "nonblocking empty");
    require(f.send(1, fd, "", 32767) == 0 && f.receive(1, fd) == 0 &&
            f.memory.load(priority, 4) == 32767, "zero length message");
    require(f.send(1, fd, "x", 32768) == -22, "priority limit");
    const int duplicate = f.descriptors.duplicate(fd);
    f.attributes(0, 0);
    f.memory.store(attr, 0, 8);
    require(f.call(1, 185, duplicate, attr, output) == 0, "set attributes");
    require(f.memory.load(output, 8) == nonblock && f.memory.load(output + 8, 8) == 3 &&
            f.memory.load(output + 16, 8) == 16, "old attributes");
    require((description->flags & nonblock) == 0, "shared status flags");
    f.memory.store(attr, 4, 8);
    require(f.call(1, 185, fd, attr, 0) == -22, "invalid attribute flag");
    f.memory.store(attr, nonblock, 8);
    require(f.call(1, 185, fd, attr, 0x80000) == -14 && (description->flags & nonblock),
            "old attribute copy fault after flag update");
}

void namespaces_permissions_and_limits() {
    Fixture f;
    f.attributes(1, 16);
    require(f.open("/libc-name") == -13, "raw syscall must not strip slash");
    const int fd = f.open("shared", create | 2, 0640);
    require(fd >= 0 && f.open("shared", create | exclusive | 2) == -17, "exclusive create");
    f.context.credentials = {2000, 2000, 100, {}, false, false, false};
    require(f.open("shared", 0, 0, 0) >= 0, "group read");
    require(f.open("shared", 1, 0, 0) == -13, "group write denied");
    require(f.call(1, 181, name) == -1, "unlink sticky owner");
    f.context.credentials.groups.push_back(100);
    f.context.credentials.fsgid = 200;
    require(f.open("shared", 0, 0, 0) >= 0, "supplementary group");
    f.context.credentials = {1000, 1000, 100, {}, false, false, false};
    f.put_string(name, "shared");
    require(f.call(1, 181, name) == 0 && f.call(1, 181, name) == -2, "unlink");
    const int recreated = f.open("shared", create | 2, 0600);
    require(recreated >= 0 && f.send(1, fd, "old") == 0, "unlinked queue remains open");
    require(f.send(1, recreated, "new") == 0, "new queue independent");
    require(f.receive(1, fd) == 3 && f.received(3) == "old", "old queue data");
    require(f.receive(1, recreated) == 3 && f.received(3) == "new", "new queue data");
    auto other = mq::make_posix_mq_namespace(f.accounting);
    f.context.message_queue_limit = 1;
    f.put_string(name, "limited");
    require(result([&] { return mq::posix_mq_syscall(other, f.memory, f.descriptors, f.scheduler,
                                                     f.context, 180, name, create | 2, 0600, attr); }) == -24,
            "per-UID message queue resource limit");
    f.context.message_queue_limit = 819200;
    require(result([&] { return mq::posix_mq_syscall(other, f.memory, f.descriptors, f.scheduler,
                                                     f.context, 180, name, 0, 0, 0); }) == -2,
            "IPC namespace isolation");
    f.attributes(0, 16);
    require(f.open("invalid") == -22, "invalid queue dimensions");
    Fixture quota;
    quota.attributes(1, 16);
    quota.context.message_queue_limit = 112;
    const int charged = quota.open("charged");
    require(charged >= 0, "initial queue charge");
    require(quota.call(1, 181, name) == 0 && quota.open("replacement") == -24,
            "unlinked open queue retains charge");
    quota.descriptors.close(charged);
    require(quota.open("replacement") >= 0, "final close releases charge");
}

void blocking_and_timeouts() {
    Fixture f;
    f.attributes(1, 16);
    const int fd = f.open("wait");
    const uint32_t first = f.spawn(), second = f.spawn();
    f.context.interrupt_pending = {};
    require(f.receive(first, fd) == -95, "blocking requires pending-signal hook");
    f.context.interrupt_pending = [] { return true; };
    require(f.receive(first, fd) == -4 && f.task(first).state == Task::State::Runnable,
            "pending signal interrupts before blocking");
    f.context.interrupt_pending = [] { return false; };
    require(f.receive(second, fd, output + 32) == 0 && f.receive(first, fd) == 0, "block readers");
    require(f.task(first).state == Task::State::Waiting && f.task(second).state == Task::State::Waiting,
            "readers waiting");
    require(f.send(1, fd, "first", 4) == 0 && f.task(second).state == Task::State::Runnable &&
            f.task(first).state == Task::State::Waiting && f.received(5, output + 32) == "first",
            "waiter FIFO handoff");
    require(f.send(1, fd, "next", 7) == 0 && f.task(first).state == Task::State::Runnable &&
            int64_t(f.task(first).cpu.x[0]) == 4 && f.received(4) == "next", "second reader handoff");
    require(f.send(1, fd, "full") == 0 && f.send(first, fd, "queued", 9) == 0 &&
            f.task(first).state == Task::State::Waiting, "block sender");
    f.put_string(data, "changed");
    require(f.receive(1, fd) == 4 && f.received(4) == "full" &&
            f.task(first).state == Task::State::Runnable && int64_t(f.task(first).cpu.x[0]) == 0,
            "wake sender on dequeue");
    require(f.receive(1, fd) == 6 && f.received(6) == "queued", "blocked send copied data");
    f.past();
    require(f.receive(1, fd, output, priority, timeout) == -110, "expired absolute timeout");
    require(f.send(1, fd, "ok", 0, timeout) == 0 && f.receive(1, fd, output, priority, timeout) == 2,
            "expired timeout permits immediate operation");
    f.memory.store(timeout + 8, 1000000000, 8);
    require(f.send(1, fd, "bad", 0, timeout) == -22, "invalid timeout on immediate operation");
    f.future(20);
    require(f.receive(first, fd, output, priority, timeout) == 0, "timed wait");
    std::this_thread::sleep_for(std::chrono::milliseconds(30));
    f.scheduler.poll_waiting();
    require(f.task(first).state == Task::State::Runnable && int64_t(f.task(first).cpu.x[0]) == -110,
            "wait timeout wake");
    require(f.receive(second, fd) == 0, "interrupt wait");
    f.scheduler.interrupt(second);
    require(f.task(second).state == Task::State::Runnable && int64_t(f.task(second).cpu.x[0]) == -4,
            "interrupted wait");
    require(f.send(1, fd, "after") == 0 && f.receive(1, fd) == 5, "interrupted waiter removed");
    require(f.receive(first, fd, 0x80000) == 0 && f.send(1, fd, "x") == 0 &&
            int64_t(f.task(first).cpu.x[0]) == -14, "invalid receive buffer at handoff");
    const int temporary = f.descriptors.duplicate(fd);
    require(f.receive(second, temporary) == 0, "wait through duplicate");
    mq::posix_mq_close(f.descriptors.get(temporary).description, 1);
    f.descriptors.close(temporary);
    require(f.send(1, fd, "kept") == 0 && int64_t(f.task(second).cpu.x[0]) == 4 &&
            f.received(4) == "kept", "blocked operation survives close");
    f.task(first).sched_policy = Task::SchedulingPolicy::Fifo;
    f.task(first).rt_priority = 50;
    f.scheduler.scheduling_changed(first);
    require(f.receive(second, fd, output + 32) == 0 && f.receive(first, fd) == 0 &&
            f.send(1, fd, "rt") == 0 && f.task(first).state == Task::State::Runnable &&
            f.task(second).state == Task::State::Waiting, "realtime waiter priority");
    f.scheduler.interrupt(second);
}

void notify_and_lifecycle() {
    Fixture f;
    f.attributes(2, 16);
    const int fd = f.open("notify");
    const auto receiver = f.spawn();
    f.notification(0, 35, 0x123456789abcdef0ull);
    f.task(1).signal_mask = signals::bit(35);
    require(f.call(1, 184, fd, event) == 0 && f.call(1, 184, fd, event) == -16, "exclusive notify");
    require(f.receive(receiver, fd) == 0 && f.send(1, fd, "direct") == 0, "direct handoff");
    require(f.signal_state->pending.empty() && f.call(1, 184, fd, event) == -16,
            "direct handoff must not fire notify");
    require(f.send(1, fd, "queued") == 0, "queue notification send");
    auto signal = f.signal_state->pending.take(signals::bit(35));
    uint64_t value = 0;
    require(bool(signal) && signal->info.code == -3 && signal->info.sender_pid() == 1 &&
            signal->info.sender_uid() == 1000, "SI_MESGQ metadata");
    std::memcpy(&value, signal->info.payload.data() + 8, 8);
    require(value == 0x123456789abcdef0ull, "sigev value");
    require(f.call(1, 184, fd, event) == 0, "one-shot registration removed");
    const int duplicate = f.descriptors.duplicate(fd);
    mq::posix_mq_close(f.descriptors.get(duplicate).description, 1);
    f.descriptors.close(duplicate);
    require(f.call(1, 184, fd, event) == 0, "per-close deregistration");
    mq::posix_mq_process_exec(f.space, 1);
    require(f.receive(1, fd) == 6 && f.send(1, fd, "x") == 0 && f.signal_state->pending.empty(),
            "exec suppresses old notification");
    require(f.call(1, 184, fd, event) == 0, "registration cleared after suppressed trigger");
    f.put_string(name, "notify");
    require(f.call(1, 181, name) == 0, "unlink registered queue");
    mq::posix_mq_process_exit(f.space, 1);
    require(f.call(1, 184, fd, event) == 0, "exit clears unlinked queue registration");
    require(f.call(1, 184, fd, 0) == 0, "cancel notification");
    f.notification(2);
    require(f.call(1, 184, fd, event) == -95, "SIGEV_THREAD must be unsupported");
    f.notification(0, 35);
    f.context.signal_hooks = nullptr;
    require(f.call(1, 184, fd, event) == -95, "signal hooks required");
    f.notification(1);
    require(f.call(1, 184, fd, event) == 0, "SIGEV_NONE");
}

void inherited_descriptors() {
    Fixture f;
    f.attributes(2, 16);
    const int fd = f.open("fork");
    DescriptorTable child(f.descriptors);
    auto child_memory = f.memory.fork_clone();
    const uint32_t pid = f.scheduler.spawn_process(CpuState{}, 0x18000, 0, 0);
    require(child.get(fd).description == f.descriptors.get(fd).description, "fork shares open description");
    require(f.send(1, fd, "parent") == 0, "parent send");
    {
        Scheduler::ScopedTask scope(f.scheduler, pid);
        const auto value = mq::posix_mq_syscall(f.space, *child_memory, child, f.scheduler, f.context,
                                                183, fd, output, 16, priority, 0);
        require(value == 6, "child receive");
    }
    char bytes[6];
    child_memory->copy_out(output, bytes, sizeof(bytes));
    require(std::string(bytes, sizeof(bytes)) == "parent", "forked memory output");
    f.descriptors.close(fd);
    {
        Scheduler::ScopedTask scope(f.scheduler, pid);
        child_memory->copy_in(data, "child", 5);
        require(mq::posix_mq_syscall(f.space, *child_memory, child, f.scheduler, f.context,
                                     182, fd, data, 5, 0, 0) == 0, "child after parent close");
    }
    const int reopened = f.open("fork", 0, 0, 0);
    require(reopened >= 0 && f.receive(1, reopened) == 5 && f.received(5) == "child", "shared namespace");
}
} // namespace

int main() {
    const char *phase = "priority_flags_and_readiness";
    try {
        priority_flags_and_readiness();
        phase = "namespaces_permissions_and_limits";
        namespaces_permissions_and_limits();
        phase = "blocking_and_timeouts";
        blocking_and_timeouts();
        phase = "notify_and_lifecycle";
        notify_and_lifecycle();
        phase = "inherited_descriptors";
        inherited_descriptors();
        std::puts("POSIX message queue tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "POSIX message queue test failed in %s: %s\n", phase, error.what());
        return 1;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "unexpected guest error: %d\n", error.number);
        return 1;
    }
}
