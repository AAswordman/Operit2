#include "linux/ipc/sysv_msg.h"
#include "linux/signals/signals.h"
#include "memory/address_space.h"
#include "memory/fault.h"
#include "runtime/tasks/scheduler.h"
#include <climits>
#include <cstdio>
#include <cstring>
#include <stdexcept>
#include <string>

using namespace operit::arm64;
namespace ipc = operit::arm64::ipc;
namespace {
constexpr uint32_t creat = 01000, excl = 02000, nowait = 04000;
constexpr uint32_t noerror = 010000, except = 020000, copy = 040000;
constexpr uint64_t source = 0x10000, destination = 0x12000, stat_buffer = 0x14000;

struct Perm {
    int32_t key;
    uint32_t uid, gid, cuid, cgid, mode;
    uint16_t sequence, pad;
    uint64_t unused1, unused2;
};
struct Stat {
    Perm perm;
    int64_t stime, rtime, ctime;
    uint64_t cbytes, qnum, qbytes;
    int32_t lspid, lrpid;
    uint64_t unused1, unused2;
};
struct Info {
    int32_t pool, map, max, mnb, mni, ssz, tql;
    uint16_t seg, pad;
};
static_assert(sizeof(Stat) == 120 && sizeof(Info) == 32);

void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
struct Fixture {
    Memory memory{16 << 20}, child_memory{16 << 20};
    Scheduler scheduler;
    std::shared_ptr<ipc::MessageQueueNamespace> space;
    ipc::MessageQueueCredentials owner;
    bool pending = false;

    explicit Fixture(ipc::MessageQueueLimits limits = {})
        : space(ipc::make_message_queue_namespace(scheduler, limits)) {
        memory.map(source, 0x8000, read_perm | write_perm);
        child_memory.map(source, 0x8000, read_perm | write_perm);
        owner.euid = 100;
        owner.egid = owner.fsgid = 200;
    }
    uint32_t process() { return scheduler.spawn_process(CpuState{}, 0x18000, 0, 0); }
    uint32_t thread() { return scheduler.spawn(CpuState{}, 0x18000, 0, 0); }
    int64_t call(uint32_t tid, Memory &mem, const ipc::MessageQueueCredentials &cred, uint64_t nr,
                 uint64_t a = 0, uint64_t b = 0, uint64_t c = 0, uint64_t d = 0, uint64_t e = 0) {
        Scheduler::ScopedTask scope(scheduler, tid);
        int64_t result;
        try {
            result = ipc::message_queue_syscall(space, mem, cred, nr, a, b, c, d, e, [this, tid] {
                if (pending) {
                    auto &task = *scheduler.lookup(tid);
                    signals::interrupted(task.signals, task.signal_mask);
                }
                return pending;
            });
        } catch (const GuestError &error) {
            result = -error.number;
        } catch (const MemoryFault &) {
            result = -14;
        }
        scheduler.current().cpu.x[0] = uint64_t(result);
        return result;
    }
    int64_t get(int32_t key, uint32_t flags, const ipc::MessageQueueCredentials *cred = nullptr) {
        return call(1, memory, cred ? *cred : owner, 186, uint32_t(key), flags);
    }
    int64_t ctl(int32_t id, int command, uint64_t address = stat_buffer,
                const ipc::MessageQueueCredentials *cred = nullptr) {
        return call(1, memory, cred ? *cred : owner, 187, uint32_t(id), uint32_t(command), address);
    }
    void put(Memory &mem, int64_t type, const std::string &text, uint64_t address = source) {
        mem.store(address, uint64_t(type), 8);
        if (!text.empty())
            mem.copy_in(address + 8, text.data(), text.size());
    }
    int64_t send(uint32_t tid, Memory &mem, int id, uint64_t size, uint32_t flags = 0,
                 const ipc::MessageQueueCredentials *cred = nullptr, uint64_t address = source) {
        return call(tid, mem, cred ? *cred : owner, 189, uint32_t(id), address, size, flags);
    }
    int64_t recv(uint32_t tid, Memory &mem, int id, uint64_t size, int64_t type = 0, uint32_t flags = 0,
                 const ipc::MessageQueueCredentials *cred = nullptr, uint64_t address = destination) {
        return call(tid, mem, cred ? *cred : owner, 188, uint32_t(id), address, size, uint64_t(type), flags);
    }
    Stat stat(int id) {
        require(ctl(id, 2) == 0, "IPC_STAT");
        Stat value{};
        memory.copy_out(stat_buffer, &value, sizeof(value));
        return value;
    }
    void set(int id, const Stat &value, const ipc::MessageQueueCredentials *cred = nullptr) {
        memory.copy_in(stat_buffer, &value, sizeof(value));
        require(ctl(id, 1, stat_buffer, cred) == 0, "IPC_SET");
    }
    std::string text(Memory &mem, size_t size, uint64_t address = destination) {
        std::string result(size, '\0');
        if (size)
            mem.copy_out(address + 8, result.data(), size);
        return result;
    }
    Task &task(uint32_t tid) { return *scheduler.lookup(tid); }
};

void creation_and_namespaces() {
    Fixture f({2, 8, 16});
    require(f.get(123, 0600) == -2, "missing key");
    int id = int(f.get(123, creat | 0640));
    require(id >= 0 && f.get(123, 0400) == id, "key lookup");
    require(f.get(123, creat | excl | 0600) == -17, "exclusive creation");
    int other = int(f.get(0, 0600));
    require(other >= 0 && other != id, "IPC_PRIVATE must create");
    require(f.get(0, 0600) == -28, "queue limit");
    auto initial = f.stat(id);
    require(initial.perm.key == 123 && initial.perm.uid == 100 && initial.perm.gid == 200 &&
                initial.perm.mode == 0640 && initial.qbytes == 16 && initial.ctime > 0,
            "initial metadata");
    require(f.ctl(id, 0, 0) == 0 && f.get(123, 0) == -2, "remove key");
    int replacement = int(f.get(123, creat | 0600));
    require(replacement >= 0 && replacement != id && f.ctl(id, 2) == -22, "stale ID reused");
    require(f.stat(replacement).perm.sequence != initial.perm.sequence, "sequence did not advance");
    auto original = f.space;
    f.space = ipc::make_message_queue_namespace(f.scheduler, {2, 8, 16});
    require(f.get(123, 0) == -2, "IPC namespaces leaked keys");
    require(f.get(0, 0600) == 0, "isolated namespace ID allocation");
    f.space = original;
    require(f.get(123, 0) == replacement, "namespace restoration");
}

void selection_and_faults() {
    Fixture f;
    int id = int(f.get(0, 0600));
    f.put(f.memory, 5, "five");
    require(f.send(1, f.memory, id, 4) == 0, "send type 5");
    f.put(f.memory, 2, "two");
    require(f.send(1, f.memory, id, 3) == 0, "send type 2");
    f.put(f.memory, 4, "four");
    require(f.send(1, f.memory, id, 4) == 0, "send type 4");
    f.put(f.memory, 2, "second");
    require(f.send(1, f.memory, id, 6) == 0, "send second type 2");
    require(f.recv(1, f.memory, id, 20, -5) == 3 && f.memory.load(destination, 8) == 2 &&
                f.text(f.memory, 3) == "two", "negative type selects lowest and FIFO");
    require(f.recv(1, f.memory, id, 20, 5, except) == 4 && f.memory.load(destination, 8) == 4,
            "MSG_EXCEPT selection");
    require(f.recv(1, f.memory, id, 20, 1, nowait | copy) == 6 && f.text(f.memory, 6) == "second" &&
                f.stat(id).qnum == 2, "MSG_COPY changed queue or wrong index");
    require(f.recv(1, f.memory, id, 1, 0, nowait | copy | noerror) == -22, "MSG_COPY truncation");
    require(f.recv(1, f.memory, id, 20, 0, copy) == -22 &&
                f.recv(1, f.memory, id, 20, 0, nowait | copy | except) == -22,
            "invalid MSG_COPY flags");
    f.memory.protect(0x15000, page_size, write_perm);
    require(f.recv(1, f.memory, id, 4, 0, nowait | copy, nullptr, 0x15000) == -14 &&
                f.stat(id).qnum == 2, "MSG_COPY input must be readable");
    require(f.recv(1, f.memory, id, 2, 0, nowait) == -7 && f.stat(id).qnum == 2,
            "E2BIG consumed message");
    require(f.recv(1, f.memory, id, 2, 0, noerror) == 2 && f.text(f.memory, 2) == "fi",
            "MSG_NOERROR truncation");
    require(f.recv(1, f.memory, id, 20, INT64_MIN) == 6 && f.text(f.memory, 6) == "second",
            "LONG_MIN message type");
    require(f.recv(1, f.memory, id, 20, 0, nowait) == -42, "empty queue ENOMSG");
    f.put(f.memory, 3, "bad");
    require(f.send(1, f.memory, id, 3) == 0, "send before EFAULT");
    require(f.recv(1, f.memory, id, 3, 0, 0, nullptr, 0x90000) == -14 && f.stat(id).qnum == 0,
            "receive copy fault should consume message");
    require(f.send(1, f.memory, id, 3, 0, nullptr, 0x90000) == -14, "send invalid pointer");
    f.put(f.memory, 0, "");
    require(f.send(1, f.memory, id, 0) == -22, "zero message type");
    f.put(f.memory, 1, "");
    require(f.send(1, f.memory, id, UINT64_MAX) == -22, "negative message size");
    require(f.recv(1, f.memory, id, UINT64_MAX) == -22, "negative receive size");
}

void permissions_and_info() {
    Fixture f({4, 12, 16});
    int id = int(f.get(44, creat | 0640));
    ipc::MessageQueueCredentials group;
    group.euid = 101;
    group.egid = group.fsgid = 201;
    group.groups.push_back(200);
    require(f.get(44, 0400, &group) == id && f.get(44, 0200, &group) == -13, "group access");
    f.put(f.memory, 1, "x");
    require(f.send(1, f.memory, id, 1, nowait, &group) == -13, "write permission");
    require(f.recv(1, f.memory, id, 1, 0, nowait, &group) == -42, "group read permission");
    auto stranger = group;
    stranger.groups.clear();
    require(f.get(44, 0400, &stranger) == -13 && f.ctl(id, 2, stat_buffer, &stranger) == -13,
            "other permission");
    require(f.ctl(id, 13, stat_buffer, &stranger) == id, "MSG_STAT_ANY permission bypass");
    require(f.ctl(id, 0, 0, &stranger) == -1, "IPC_RMID owner check");
    stranger.cap_ipc_owner = true;
    require(f.send(1, f.memory, id, 1, nowait, &stranger) == 0, "CAP_IPC_OWNER");
    require(f.ctl(id, 12) >= 0, "MSG_INFO");
    Info info{};
    f.memory.copy_out(stat_buffer, &info, sizeof(info));
    require(info.pool == 1 && info.map == 1 && info.tql == 1 && info.max == 12 && info.mnb == 16 &&
                info.mni == 4, "MSG_INFO counters");
    auto changed = f.stat(id);
    changed.qbytes = 17;
    f.memory.copy_in(stat_buffer, &changed, sizeof(changed));
    require(f.ctl(id, 1) == -1, "increased qbytes without CAP_SYS_RESOURCE");
    auto privileged = f.owner;
    privileged.cap_sys_resource = true;
    f.set(id, changed, &privileged);
    require(f.stat(id).qbytes == 17, "privileged qbytes change");
    changed.perm.uid = UINT32_MAX;
    f.memory.copy_in(stat_buffer, &changed, sizeof(changed));
    require(f.ctl(id, 1, stat_buffer, &privileged) == -22 && f.stat(id).perm.uid == 100,
            "invalid IPC_SET uid must not change permissions");
    stranger.cap_sys_admin = true;
    require(f.ctl(id, 0, 0, &stranger) == 0, "CAP_SYS_ADMIN removal");
}

void blocking_and_lifecycle() {
    Fixture f({8, 8, 4});
    int id = int(f.get(0, 0600));
    const auto child = f.process();
    const auto peer = f.thread();
    f.put(f.memory, 1, "full");
    require(f.send(1, f.memory, id, 4) == 0, "fill queue");
    f.put(f.child_memory, 2, "next");
    require(f.send(child, f.child_memory, id, 4, nowait) == -11, "full queue EAGAIN");
    require(f.send(child, f.child_memory, id, 4) == 0 && f.task(child).state == Task::State::Waiting,
            "sender did not block");
    f.put(f.child_memory, 2, "edit");
    require(f.recv(1, f.memory, id, 4) == 4 && f.text(f.memory, 4) == "full", "receive full message");
    require(f.task(child).state == Task::State::Runnable && int64_t(f.task(child).cpu.x[0]) == 0,
            "sender did not wake");
    require(f.recv(1, f.memory, id, 4) == 4 && f.text(f.memory, 4) == "next",
            "blocked send did not retain original data");
    require(f.recv(child, f.child_memory, id, 4, 7) == 0 && f.task(child).state == Task::State::Waiting,
            "child receiver did not block");
    f.put(f.memory, 7, "hi");
    require(f.send(peer, f.memory, id, 2) == 0 && f.task(child).state == Task::State::Runnable &&
                int64_t(f.task(child).cpu.x[0]) == 2 && f.text(f.child_memory, 2) == "hi" &&
                f.stat(id).qnum == 0, "cross-process direct delivery");
    require(f.recv(child, f.child_memory, id, 1, 8) == 0 && f.recv(peer, f.memory, id, 4, 8) == 0,
            "receiver setup");
    f.put(f.memory, 8, "two");
    require(f.send(1, f.memory, id, 3) == 0 && int64_t(f.task(child).cpu.x[0]) == -7 &&
                int64_t(f.task(peer).cpu.x[0]) == 3 && f.text(f.memory, 3) == "two",
            "E2BIG should skip to next waiting receiver");
    require(f.recv(child, f.child_memory, id, 4) == 0, "interrupt setup");
    f.scheduler.interrupt(child);
    require(int64_t(f.task(child).cpu.x[0]) == -4 && f.task(child).state == Task::State::Runnable,
            "signal interruption");
    f.put(f.memory, 9, "keep");
    require(f.send(1, f.memory, id, 4) == 0 && f.stat(id).qnum == 1, "interrupted waiter remained");
    f.put(f.child_memory, 1, "more");
    require(f.send(child, f.child_memory, id, 4) == 0 && f.recv(peer, f.memory, id, 4, 10) == 0,
            "RMID waiter setup");
    require(f.ctl(id, 0, 0) == 0 && int64_t(f.task(child).cpu.x[0]) == -43 &&
                int64_t(f.task(peer).cpu.x[0]) == -43, "RMID did not wake both kinds of waiter");
    require(f.recv(1, f.memory, id, 4, 0, nowait) == -22, "removed queue still accessible");
}

void cancellation_and_zero_length() {
    Fixture f({4, 4, 2});
    int id = int(f.get(0, 0600));
    const auto child = f.process();
    f.put(f.memory, 1, "");
    require(f.send(1, f.memory, id, 0) == 0 && f.send(1, f.memory, id, 0) == 0 &&
                f.send(1, f.memory, id, 0, nowait) == -11, "zero-length message count limit");
    require(f.recv(1, f.memory, id, 0) == 0 && f.recv(1, f.memory, id, 0) == 0,
            "zero-length receive");
    {
        Scheduler::ScopedTask scope(f.scheduler, child);
        auto &task = f.scheduler.current();
        task.cpu.x[8] = 188;
        signals::remember_syscall(task.signals, task.cpu, true);
    }
    require(f.recv(child, f.child_memory, id, 2) == 0 &&
                f.task(child).signals.restart->without_handler_only,
            "message waits must not restart in a signal handler");
    std::weak_ptr<ipc::MessageQueueNamespace> weak = f.space;
    f.space.reset();
    require(!weak.expired(), "waiting task did not retain namespace");
    f.space = weak.lock();
    f.scheduler.exit_process(child, 0);
    f.put(f.memory, 1, "ok");
    require(f.send(1, f.memory, id, 2) == 0 && f.stat(id).qnum == 1,
            "exited process left a receiver or destroyed queue");
    require(f.recv(1, f.memory, id, 2) == 2, "receive after process exit");
    f.pending = true;
    require(f.recv(1, f.memory, id, 2) == -4 && f.task(1).state == Task::State::Runnable,
            "pending signal before sleep");
}

void permissions_change_while_waiting() {
    Fixture f({4, 4, 4});
    int id = int(f.get(0, 0666));
    auto user = f.owner;
    user.euid = 101;
    const auto child = f.process(), peer = f.thread();
    require(f.recv(child, f.child_memory, id, 4, 2, 0, &user) == 0, "blocked reader");
    f.put(f.memory, 1, "full");
    require(f.send(1, f.memory, id, 4) == 0, "fill before blocked writer");
    f.put(f.memory, 3, "x");
    require(f.send(peer, f.memory, id, 1, 0, &user) == 0, "blocked writer");
    auto changed = f.stat(id);
    changed.perm.mode = 0600;
    f.set(id, changed);
    require(int64_t(f.task(child).cpu.x[0]) == -13 && int64_t(f.task(peer).cpu.x[0]) == -13 &&
                f.task(child).state == Task::State::Runnable && f.task(peer).state == Task::State::Runnable,
            "IPC_SET did not recheck sleeping task permissions");
    require(f.stat(id).qnum == 1, "permission change altered queued messages");
}

void capacity_changes_and_exec() {
    Fixture f({4, 8, 4});
    const int id = int(f.get(0, 0600));
    const auto large = f.process(), small = f.thread();
    f.put(f.memory, 1, "full");
    require(f.send(1, f.memory, id, 4) == 0, "capacity setup");
    f.put(f.child_memory, 2, "long");
    require(f.send(large, f.child_memory, id, 4) == 0, "blocked large send");
    f.put(f.memory, 3, "x");
    require(f.send(small, f.memory, id, 1) == 0, "blocked small send");
    auto changed = f.stat(id);
    changed.qbytes = 2;
    f.set(id, changed);
    require(f.recv(1, f.memory, id, 4) == 4 && f.task(large).state == Task::State::Waiting &&
                f.task(small).state == Task::State::Runnable && f.stat(id).qnum == 1,
            "smaller sender must progress when larger sender does not fit");
    changed = f.stat(id);
    changed.qbytes = 8;
    auto privileged = f.owner;
    privileged.cap_sys_resource = true;
    f.set(id, changed, &privileged);
    require(f.task(large).state == Task::State::Runnable && int64_t(f.task(large).cpu.x[0]) == 0 &&
                f.stat(id).qnum == 2, "increased capacity did not wake sender");
    require(f.recv(1, f.memory, id, 8) == 1 && f.recv(1, f.memory, id, 8) == 4,
            "capacity change message order");
    const auto retired = f.thread();
    require(f.recv(retired, f.memory, id, 4) == 0 && f.task(retired).state == Task::State::Waiting,
            "exec waiter setup");
    f.scheduler.replace_process_image(1, CpuState{});
    require(f.task(retired).state == Task::State::Exited, "exec did not retire waiting peer");
    f.put(f.memory, 1, "ok");
    require(f.send(1, f.memory, id, 2) == 0 && f.stat(id).qnum == 1,
            "exec retained waiter or lost queue");
}
} // namespace

int main() {
    try {
        creation_and_namespaces();
        selection_and_faults();
        permissions_and_info();
        blocking_and_lifecycle();
        cancellation_and_zero_length();
        permissions_change_while_waiting();
        capacity_changes_and_exec();
        std::puts("sysv message queue tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "sysv message queue test failed: %s\n", error.what());
        return 1;
    }
}
