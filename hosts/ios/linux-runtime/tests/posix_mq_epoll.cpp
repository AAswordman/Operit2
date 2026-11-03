#include "fs/descriptor_table.h"
#include "linux/events/epoll.h"
#include "linux/ipc/posix_mq.h"
#include "memory/address_space.h"
#include "runtime/tasks/scheduler.h"
#include <array>
#include <chrono>
#include <cstdio>
#include <memory>
#include <optional>
#include <poll.h>
#include <stdexcept>
#include <string>
#include <unistd.h>

using namespace operit::arm64;
namespace mq = operit::arm64::ipc;
namespace {
constexpr uint32_t in = 1, out = 4, read_normal = 64, write_normal = 256;
constexpr uint32_t oneshot = 1u << 30, edge = 1u << 31;
constexpr uint64_t name_address = 0x10000, attr_address = 0x11000;
constexpr uint64_t data_address = 0x12000, output_address = 0x13000;

void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
void expect(EventPoll &poll, uint32_t events, uint64_t data, const char *message) {
    const auto result = poll.wait(8, 0);
    require(result.size() == 1 && result[0].events == events && result[0].data == data, message);
}
void expect_empty(EventPoll &poll, const char *message) {
    require(poll.wait(8, 0).empty(), message);
}
struct Fixture {
    Memory memory{8 << 20};
    DescriptorTable descriptors;
    Scheduler scheduler;
    std::shared_ptr<mq::PosixMqAccounting> accounting = mq::make_posix_mq_accounting();
    std::shared_ptr<mq::PosixMqNamespace> space = mq::make_posix_mq_namespace(accounting);
    mq::PosixMqContext context;

    Fixture() {
        memory.map(name_address, 0x5000, read_perm | write_perm);
        context.interrupt_pending = [] { return false; };
    }
    int64_t call(uint64_t number, uint64_t a0, uint64_t a1 = 0, uint64_t a2 = 0,
                 uint64_t a3 = 0, uint64_t a4 = 0) {
        Scheduler::ScopedTask scope(scheduler, 1);
        return mq::posix_mq_syscall(space, memory, descriptors, scheduler, context,
                                    number, a0, a1, a2, a3, a4);
    }
    int open(const char *name, uint64_t capacity = 2) {
        memory.copy_in(name_address, name, std::char_traits<char>::length(name) + 1);
        const std::array<uint64_t, 8> attributes{0, capacity, 16};
        memory.copy_in(attr_address, attributes.data(), sizeof(attributes));
        return int(call(180, name_address, 64 | 2 | 2048, 0600, attr_address));
    }
    void send(int fd) {
        memory.store(data_address, 'x', 1);
        require(call(182, fd, data_address, 1) == 0, "send message");
    }
    void receive(int fd) {
        require(call(183, fd, output_address, 16) == 1, "receive message");
    }
    void control(EventPoll &poll, int operation, int fd, uint32_t events, uint64_t data) {
        auto &entry = descriptors.get(fd);
        poll.control(operation, fd, entry.host.get(), entry.description, events, data);
    }
};

void level_and_stale_readiness() {
    Fixture f;
    const int fd = f.open("level");
    EventPoll poll;
    f.control(poll, 1, fd, in | out, 11);
    require(poll.poll_events(POLLIN) == POLLIN, "empty writable queue makes epoll readable");
    require(poll.poll_events(POLLOUT) == 0, "epoll descriptor itself is not writable");
    expect(poll, out, 11, "empty queue writable");
    expect(poll, out, 11, "writable level repeats");
    f.send(fd);
    expect(poll, in | out, 11, "partially full queue");
    f.send(fd);
    expect(poll, in, 11, "full queue not writable");
    f.receive(fd);
    expect(poll, in | out, 11, "full to partially full without marker transition");
    f.receive(fd);
    expect(poll, out, 11, "drained queue");

    f.control(poll, 3, fd, in, 12);
    expect_empty(poll, "modified read interest on empty queue");
    f.send(fd);
    require(poll.poll_events(POLLIN) == POLLIN, "read readiness cached by poll");
    f.receive(fd);
    require(poll.poll_events(POLLIN) == 0, "stale cached readability removed");
    expect_empty(poll, "stale readability must not be delivered");
    f.control(poll, 2, fd, 0, 0);
    expect_empty(poll, "deleted queue watch");
}

void normal_readiness_bits() {
    Fixture f;
    const int fd = f.open("normal-bits");
    EventPoll poll;
    f.control(poll, 1, fd, read_normal | write_normal, 13);
    expect(poll, write_normal, 13, "empty queue WRNORM");
    f.send(fd);
    expect(poll, read_normal | write_normal, 13, "partial queue RDNORM and WRNORM");
    f.send(fd);
    expect(poll, read_normal, 13, "full queue RDNORM only");
    f.control(poll, 3, fd, in | out | read_normal | write_normal, 14);
    expect(poll, in | read_normal, 14, "full queue with standard and normal bits");
    f.receive(fd);
    expect(poll, in | out | read_normal | write_normal, 14,
           "partial queue with standard and normal bits");
    f.receive(fd);
    expect(poll, out | write_normal, 14, "empty queue with standard and normal write bits");
}

void mqueue_edges() {
    Fixture f;
    const int fd = f.open("edge");
    EventPoll poll;
    f.control(poll, 1, fd, in | out | edge, 21);
    expect(poll, out, 21, "edge ADD reports initial writable state");
    expect_empty(poll, "edge ADD reports the initial state only once");
    f.send(fd);
    expect(poll, in | out, 21, "edge enqueue reports current readable and writable state");
    f.send(fd);
    expect(poll, in, 21, "edge enqueue reports a full queue");
    f.receive(fd);
    expect(poll, in | out, 21, "edge dequeue reports full-to-nonfull");
    f.receive(fd);
    expect(poll, out, 21, "edge dequeue reports current empty state");
    expect_empty(poll, "edge state does not repeat without a wakeup");

    f.control(poll, 3, fd, in | edge, 22);
    f.send(fd);
    expect(poll, in, 22, "edge read starts ready");
    f.receive(fd);
    f.send(fd);
    expect(poll, in, 22, "empty-to-readable edge survives an unobserved round trip");
    expect_empty(poll, "round-trip edge delivered once");
    f.receive(fd);
    f.send(fd);
    f.receive(fd);
    expect_empty(poll, "stale edge is discarded when no longer readable");

    f.control(poll, 3, fd, out | edge, 23);
    expect(poll, out, 23, "edge MOD reports current writable state");
    f.send(fd);
    f.send(fd);
    f.receive(fd);
    expect(poll, out, 23, "full-to-writable edge survives an unobserved round trip");
    f.receive(fd);
    expect(poll, out, 23, "dequeue wakes poll waiters even while already writable");
    expect_empty(poll, "writable edge does not repeat without a wakeup");

    const int second = f.open("edge-repeated", 3);
    EventPoll repeated;
    f.control(repeated, 1, second, in | read_normal | edge, 24);
    f.send(second);
    expect(repeated, in | read_normal, 24, "normal read bits on first enqueue");
    f.send(second);
    expect(repeated, in | read_normal, 24, "another enqueue wakes an already readable edge watch");
    f.receive(second);
    expect(repeated, in | read_normal, 24, "dequeue wakes an already readable edge watch");
}

void nested_mqueue_edges() {
    Fixture f;
    const int fd = f.open("nested-edge");
    const int sibling = f.open("nested-edge-sibling");
    auto leaf = std::make_shared<EventPoll>();
    auto middle = std::make_shared<EventPoll>();
    auto leaf_description = std::make_shared<OpenDescription>(0);
    auto middle_description = std::make_shared<OpenDescription>(0);
    leaf_description->poll = leaf;
    middle_description->poll = middle;
    f.control(*leaf, 1, fd, in | edge, 25);
    f.control(*leaf, 1, sibling, out, 26);
    middle->control(1, 210, leaf->native_handle(), leaf_description, in | edge, 27);
    EventPoll root;
    root.control(1, 211, middle->native_handle(), middle_description, in | edge, 28);
    expect(root, in, 28, "initial nested edge from level-ready sibling");
    expect(*middle, in, 27, "middle edge from level-ready sibling");
    expect(*leaf, out, 26, "level-ready sibling");
    expect_empty(root, "level-ready sibling does not repeat the parent edge");

    f.send(fd);
    pollfd native{root.native_handle(), POLLIN, 0};
    require(::poll(&native, 1, 0) == 1 && (native.revents & POLLIN),
            "MQ event immediately marks the nested native handle");
    expect(root, in, 28, "new MQ event reaches edge root despite ready sibling");
    expect(*middle, in, 27, "new MQ event reaches edge middle");
    const auto first = leaf->wait(8, 0);
    require(first.size() == 2, "leaf reports both MQ and level sibling");
    expect_empty(root, "nested event is consumed once");

    f.receive(fd);
    f.send(fd);
    expect(root, in, 28, "unsampled MQ round trip reaches nested edge root");
    expect(*middle, in, 27, "unsampled MQ round trip reaches nested edge middle");
    const auto second = leaf->wait(8, 0);
    require(second.size() == 2, "leaf preserves MQ edge across unsampled round trip");
    expect_empty(root, "nested round trip does not repeat");

    root.control(3, 211, middle->native_handle(), middle_description, in | edge | oneshot, 29);
    expect_empty(root, "MOD of currently unready edge child does not invent readiness");
    f.receive(fd);
    f.send(fd);
    expect(root, in, 29, "nested edge oneshot event");
    f.receive(fd);
    f.send(fd);
    expect_empty(root, "nested oneshot remains disabled");
    root.control(3, 211, middle->native_handle(), middle_description, in | edge | oneshot, 30);
    expect(root, in, 30, "nested oneshot MOD observes pending child readiness");
}

void oneshot_and_duplicates() {
    Fixture f;
    const int fd = f.open("oneshot");
    const int duplicate = f.descriptors.duplicate(fd);
    EventPoll poll;
    f.control(poll, 1, fd, in | out | oneshot, 31);
    expect(poll, out, 31, "initial oneshot");
    f.send(fd);
    expect_empty(poll, "oneshot remains disabled after readiness changes");
    f.control(poll, 3, fd, in | out | oneshot, 32);
    expect(poll, in | out, 32, "oneshot MOD rearm");
    expect_empty(poll, "rearmed oneshot delivered once");
    f.control(poll, 1, duplicate, in | oneshot, 33);
    expect(poll, in, 33, "duplicate descriptor has independent watch");
    expect_empty(poll, "duplicate oneshot disabled");
    f.control(poll, 2, fd, 0, 0);
    f.control(poll, 3, duplicate, in | oneshot, 34);
    expect(poll, in, 34, "duplicate remains after original watch deletion");
    f.descriptors.close(fd);
    f.descriptors.close(duplicate);
    expect_empty(poll, "last open description close removes watches");
}

void nested_readiness() {
    Fixture f;
    const int fd = f.open("nested");
    auto child = std::make_shared<EventPoll>();
    auto child_description = std::make_shared<OpenDescription>(0);
    child_description->poll = child;
    EventPoll parent;
    f.control(*child, 1, fd, in | out, 41);
    parent.control(1, 200, child->native_handle(), child_description, in, 42);
    require(parent.poll_events(POLLIN) == POLLIN, "nested writable readiness is pollable");
    expect(parent, in, 42, "initial nested level readiness");
    expect(parent, in, 42, "nested level repeats while child remains ready");
    f.send(fd);
    expect(parent, in, 42, "parent remains ready for partial child queue");
    f.send(fd);
    expect(parent, in, 42, "parent remains ready for full readable child queue");
    f.receive(fd);
    expect(parent, in, 42, "nested full-to-partial writable readiness");
    expect(*child, in | out, 41, "child level state after parent observation");

    f.control(*child, 3, fd, in | oneshot, 43);
    expect(*child, in, 43, "child oneshot delivered");
    require(parent.poll_events(POLLIN) == 0, "disarmed child makes parent unready");
    f.control(*child, 3, fd, in | oneshot, 44);
    expect(parent, in, 42, "child rearm propagates to parent");
    expect(*child, in, 44, "rearmed child event");
    require(parent.poll_events(POLLIN) == 0, "consumed child clears parent readiness");
}

void deeply_nested_readiness() {
    Fixture f;
    const int fd = f.open("deep-nested");
    auto leaf = std::make_shared<EventPoll>();
    auto middle = std::make_shared<EventPoll>();
    auto leaf_description = std::make_shared<OpenDescription>(0);
    auto middle_description = std::make_shared<OpenDescription>(0);
    leaf_description->poll = leaf;
    middle_description->poll = middle;
    EventPoll root;
    f.control(*leaf, 1, fd, out | oneshot, 45);
    middle->control(1, 201, leaf->native_handle(), leaf_description, in, 46);
    root.control(1, 202, middle->native_handle(), middle_description, in, 47);
    expect(root, in, 47, "three-level writable readiness");
    expect(*middle, in, 46, "middle readiness");
    expect(*leaf, out, 45, "leaf writable oneshot");
    require(root.poll_events(POLLIN) == 0, "consumed leaf clears all ancestors");
    f.send(fd);
    f.send(fd);
    f.control(*leaf, 3, fd, out | oneshot, 45);
    require(root.poll_events(POLLIN) == 0, "full queue stays unready at all levels");
    f.receive(fd);
    expect(root, in, 47, "full-to-partial readiness reaches all ancestors");
    expect(*leaf, out, 45, "leaf full-to-partial oneshot");
    require(root.poll_events(POLLIN) == 0, "ancestors clear after leaf event");
}

void fairness_and_native_watch() {
    Fixture f;
    const int first = f.open("fair-first"), second = f.open("fair-second");
    int raw[2];
    require(::pipe(raw) == 0, "native pipe");
    FD reader(raw[0]), writer(raw[1]);
    auto native = std::make_shared<OpenDescription>(0);
    EventPoll poll;
    f.control(poll, 1, first, out, 51);
    f.control(poll, 1, second, out, 52);
    poll.control(1, 300, reader.get(), native, in, 53);
    const char byte = 1;
    require(::write(writer.get(), &byte, 1) == 1, "write native pipe");
    unsigned seen = 0;
    for (int i = 0; i < 12; ++i) {
        const auto result = poll.wait(1, 0);
        require(result.size() == 1 && result[0].events == (result[0].data == 53 ? in : out),
                "mixed native and MQ readiness");
        require(result[0].data >= 51 && result[0].data <= 53, "unexpected mixed watch");
        seen |= 1u << (result[0].data - 51);
    }
    require(seen == 7, "maxevents fairness across native and MQ watches");
    char consumed;
    require(::read(reader.get(), &consumed, 1) == 1, "drain native pipe");
    poll.control(2, 300, reader.get(), native, 0, 0);
}

void native_edge_and_oneshot() {
    Fixture f;
    const int fd = f.open("native-regression");
    int raw[2];
    require(::pipe(raw) == 0, "native pipe for edge test");
    FD reader(raw[0]), writer(raw[1]);
    auto native = std::make_shared<OpenDescription>(0);
    EventPoll poll;
    f.control(poll, 1, fd, in, 54);
    poll.control(1, 301, reader.get(), native, in | edge | oneshot, 55);
    const char byte = 1;
    require(::write(writer.get(), &byte, 1) == 1, "write native oneshot pipe");
    expect(poll, in, 55, "native edge oneshot");
    expect_empty(poll, "native oneshot disabled");
    require(::write(writer.get(), &byte, 1) == 1, "write disabled native pipe");
    expect_empty(poll, "native oneshot does not fire while disabled");
    poll.control(3, 301, reader.get(), native, in | edge | oneshot, 56);
    expect(poll, in, 56, "native oneshot rearm");
    char bytes[2];
    require(::read(reader.get(), bytes, sizeof(bytes)) == 2, "drain native edge pipe");
    f.send(fd);
    expect(poll, in, 54, "MQ watch still works beside disarmed native watch");
}

void scheduler_rechecks() {
    Fixture f;
    const int fd = f.open("scheduler");
    f.send(fd);
    f.send(fd);
    auto poll = std::make_shared<EventPoll>();
    f.control(*poll, 1, fd, out, 61);
    const uint32_t waiter = f.scheduler.spawn(CpuState{}, 0x14000, 0, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, waiter);
        require(poll->wait(1, 0).empty(), "full queue not initially writable");
        f.scheduler.block({}, 0, [poll]() -> std::optional<int64_t> {
            const auto result = poll->wait(1, 0);
            return result.empty() ? std::optional<int64_t>{} : int64_t(result[0].events);
        });
    }
    f.scheduler.poll_waiting();
    require(f.scheduler.lookup(waiter)->state == Task::State::Waiting, "full queue waiter stays blocked");
    f.receive(fd);
    f.scheduler.poll_waiting();
    require(f.scheduler.lookup(waiter)->state == Task::State::Runnable &&
                f.scheduler.lookup(waiter)->cpu.x[0] == out,
            "scheduler retries wake for writable queue with readable marker unchanged");

    f.control(*poll, 3, fd, in | oneshot, 62);
    expect(*poll, in, 62, "read oneshot before blocking timeout");
    const auto start = std::chrono::steady_clock::now();
    expect_empty(*poll, "nonblocking oneshot check");
    require(poll->wait(1, 20).empty(), "bounded wait without new event");
    const auto elapsed = std::chrono::steady_clock::now() - start;
    require(elapsed >= std::chrono::milliseconds(15) && elapsed < std::chrono::seconds(2),
            "bounded wait timeout");
    f.receive(fd);
    f.control(*poll, 3, fd, in, 63);
    const auto armed_start = std::chrono::steady_clock::now();
    require(poll->wait(1, 20).empty(), "bounded wait for an armed but empty queue");
    const auto armed_elapsed = std::chrono::steady_clock::now() - armed_start;
    require(armed_elapsed >= std::chrono::milliseconds(15) && armed_elapsed < std::chrono::seconds(2),
            "armed queue wait timeout");
}

void blocked_handoffs() {
    Fixture f;
    const int fd = f.open("edge-handoffs", 1);
    f.send(fd);
    EventPoll poll;
    f.control(poll, 1, fd, in | edge, 71);
    expect(poll, in, 71, "full queue starts readable");
    f.descriptors.get(fd).description->flags &= ~2048;

    const uint32_t sender = f.scheduler.spawn(CpuState{}, 0x14000, 0, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, sender);
        require(mq::posix_mq_syscall(f.space, f.memory, f.descriptors, f.scheduler, f.context,
                                    182, fd, data_address, 1) == 0,
                "full-queue sender blocks");
    }
    require(f.scheduler.lookup(sender)->state == Task::State::Waiting, "sender is waiting");
    f.receive(fd);
    require(f.scheduler.lookup(sender)->state == Task::State::Runnable, "sender refilled queue");
    expect_empty(poll, "atomic sender refill does not fabricate a readiness wakeup");
    f.receive(fd);
    expect_empty(poll, "drained queue cannot deliver a stale read event");

    const uint32_t receiver = f.scheduler.spawn(CpuState{}, 0x15000, 0, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, receiver);
        require(mq::posix_mq_syscall(f.space, f.memory, f.descriptors, f.scheduler, f.context,
                                    183, fd, output_address, 16) == 0,
                "empty-queue receiver blocks");
    }
    require(f.scheduler.lookup(receiver)->state == Task::State::Waiting, "receiver is waiting");
    f.send(fd);
    require(f.scheduler.lookup(receiver)->state == Task::State::Runnable, "receiver got direct message");
    expect_empty(poll, "direct delivery to a blocked receiver does not fabricate a queue event");
}

void inherited_edge_watch() {
    Fixture f;
    const int fd = f.open("inherited-edge", 1);
    f.send(fd);
    auto poll = std::make_shared<EventPoll>();
    f.control(*poll, 1, fd, in | edge, 72);
    expect(*poll, in, 72, "parent consumes initial edge");
    DescriptorTable child(f.descriptors);
    auto child_memory = f.memory.fork_clone();
    const uint32_t child_pid = f.scheduler.spawn_process(CpuState{}, 0x18000, 0, 0);
    const uint32_t waiter = f.scheduler.spawn(CpuState{}, 0x17000, 0, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, waiter);
        f.scheduler.block({}, 0, [poll]() -> std::optional<int64_t> {
            auto result = poll->wait(1, 0);
            return result.empty() ? std::optional<int64_t>{} : int64_t(result[0].events);
        });
    }
    f.descriptors.close(fd);
    {
        Scheduler::ScopedTask scope(f.scheduler, child_pid);
        require(mq::posix_mq_syscall(f.space, *child_memory, child, f.scheduler, f.context,
                                    183, fd, output_address, 16) == 1,
                "child drains inherited queue");
        require(mq::posix_mq_syscall(f.space, *child_memory, child, f.scheduler, f.context,
                                    182, fd, data_address, 1) == 0,
                "child refills inherited queue before scheduler retry");
    }
    f.scheduler.poll_waiting();
    require(f.scheduler.lookup(waiter)->state == Task::State::Runnable &&
                f.scheduler.lookup(waiter)->cpu.x[0] == in,
            "unsampled child transitions wake parent after parent descriptor close");
    expect_empty(*poll, "inherited edge is consumed once");
    child.close(fd);
    expect_empty(*poll, "last inherited descriptor close removes the watch");
}
} // namespace

int main() {
    const char *phase = "level_and_stale_readiness";
    try {
        level_and_stale_readiness();
        phase = "normal_readiness_bits";
        normal_readiness_bits();
        phase = "mqueue_edges";
        mqueue_edges();
        phase = "nested_mqueue_edges";
        nested_mqueue_edges();
        phase = "oneshot_and_duplicates";
        oneshot_and_duplicates();
        phase = "nested_readiness";
        nested_readiness();
        phase = "deeply_nested_readiness";
        deeply_nested_readiness();
        phase = "fairness_and_native_watch";
        fairness_and_native_watch();
        phase = "native_edge_and_oneshot";
        native_edge_and_oneshot();
        phase = "scheduler_rechecks";
        scheduler_rechecks();
        phase = "blocked_handoffs";
        blocked_handoffs();
        phase = "inherited_edge_watch";
        inherited_edge_watch();
        std::puts("POSIX mqueue epoll tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "POSIX mqueue epoll test failed in %s: %s\n", phase, error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "POSIX mqueue epoll test failed in %s: guest error %d\n", phase, error.number);
    }
    return 1;
}
