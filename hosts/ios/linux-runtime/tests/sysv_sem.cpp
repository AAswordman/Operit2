#include "linux/ipc/sysv_sem.h"
#include "linux/abi.h"
#include "memory/address_space.h"
#include "runtime/tasks/scheduler.h"
#include <cstdio>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

using namespace operit::arm64;
using namespace operit::arm64::ipc;

namespace {
constexpr uint64_t input = 0x10000, output = 0x11000, timer = 0x12000;
constexpr int create = 01000, exclusive = 02000, nowait = 04000, undo = 010000;
constexpr int rmid = 0, set = 1, stat = 2, info = 3;
constexpr int getpid_cmd = 11, getval = 12, getall = 13, getncnt = 14, getzcnt = 15;
constexpr int setval = 16, setall = 17, sem_stat = 18, sem_info = 19, sem_stat_any = 20;

struct Op { uint16_t index; int16_t value, flags; };
struct Perm {
    int32_t key;
    uint32_t uid, gid, cuid, cgid, mode;
    uint16_t sequence, pad;
    uint64_t unused1, unused2;
};
struct Semid {
    Perm perm;
    int64_t otime, ctime;
    uint64_t count, unused1, unused2;
};
struct Seminfo {
    int32_t semmap, semmni, semmns, semmnu, semmsl, semopm, semume, semusz, semvmx, semaem;
};
static_assert(sizeof(Op) == 6 && sizeof(Semid) == 88 && sizeof(Seminfo) == 40);

void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}

struct Fixture {
    Memory memory{8 << 20};
    Scheduler scheduler;
    std::shared_ptr<SemaphoreNamespace> ns;
    std::map<uint32_t, std::unique_ptr<SemaphoreTask>> handles;
    SemaphoreCredentials credentials{1000, 2000, 2000, {}, false, false};

    explicit Fixture(SemaphoreLimits limits = {}) {
        memory.map(input, 0x8000, read_perm | write_perm);
        ns = make_semaphore_namespace(scheduler, limits);
        handles.emplace(1, make_semaphore_task());
    }
    ~Fixture() {
        for (auto &entry : handles)
            semaphore_task_exit(entry.second, scheduler, entry.first);
    }
    uint32_t spawn(bool share = false, bool process = true, uint32_t parent = 1) {
        Scheduler::ScopedTask context(scheduler, parent);
        const auto id = process ? scheduler.spawn_process(CpuState{}, 0, 0, 0)
                                : scheduler.spawn(CpuState{}, 0, 0, 0);
        handles.emplace(id, clone_semaphore_task(*handles.at(parent), share));
        return id;
    }
    void exit(uint32_t tid) {
        semaphore_task_exit(handles.at(tid), scheduler, tid);
        Scheduler::ScopedTask context(scheduler, tid);
        scheduler.exit(0, false);
    }
    int64_t call(uint32_t tid, uint64_t number, uint64_t a = 0, uint64_t b = 0,
                 uint64_t c = 0, uint64_t d = 0, const std::function<bool()> &interrupt = {}) {
        Scheduler::ScopedTask context(scheduler, tid);
        try {
            return semaphore_syscall(ns, *handles.at(tid), memory, credentials, number, a, b, c, d,
                                     interrupt);
        } catch (const GuestError &error) {
            return -error.number;
        } catch (const MemoryFault &) {
            return -14;
        }
    }
    int64_t get(int key, int n, int flags, uint32_t tid = 1) {
        return call(tid, 190, uint32_t(key), uint32_t(n), uint32_t(flags));
    }
    int64_t ctl(int id, int n, int command, uint64_t arg = 0, uint32_t tid = 1) {
        return call(tid, 191, uint32_t(id), uint32_t(n), uint32_t(command), arg);
    }
    int64_t op(uint32_t tid, int id, const std::vector<Op> &operations, uint64_t timeout = 0,
               bool timed = false, const std::function<bool()> &interrupt = {}) {
        if (!operations.empty())
            memory.copy_in(input, operations.data(), operations.size() * sizeof(Op));
        return call(tid, timed ? 192 : 193, uint32_t(id), input, operations.size(), timeout, interrupt);
    }
    Task &task(uint32_t tid) { return *scheduler.lookup(tid); }
    void timespec(int64_t seconds, int64_t nanos) {
        linux_abi::Timespec value{seconds, nanos};
        memory.copy_in(timer, &value, sizeof(value));
    }
    Semid read_stat(int id) {
        require(ctl(id, 0, stat, output) == 0, "IPC_STAT");
        Semid result{};
        memory.copy_out(output, &result, sizeof(result));
        return result;
    }
};

void creation_and_controls() {
    Fixture f({2, 3, 3, 4});
    require(f.get(77, 0, create | 0600) == -22, "zero-sized creation");
    require(f.get(77, 4, create | 0600) == -22, "SEMMSL");
    require(f.get(77, 1, 0600) == -2, "missing key");
    const int id = int(f.get(77, 2, create | 0640));
    require(id >= 0 && f.get(77, 0, 0) == id, "create/find key");
    require(f.get(77, 1, create | exclusive) == -17, "IPC_EXCL");
    require(f.get(77, 3, 0) == -22, "existing size");
    const auto initial = f.read_stat(id);
    require(initial.perm.key == 77 && initial.perm.uid == 1000 && initial.perm.gid == 2000 &&
                initial.perm.mode == 0640 && initial.count == 2 && !initial.otime && initial.ctime > 0,
            "AArch64 semid64_ds");
    require(f.ctl(id, 0, getval) == 0 && f.ctl(id, 0, getpid_cmd) == 0, "initial values");
    require(f.ctl(id, 0, setval, 5) == 0 && f.ctl(id, 0, getpid_cmd) == 1, "SETVAL/GETPID");
    uint16_t values[2]{6, 7};
    f.memory.copy_in(input, values, sizeof(values));
    require(f.ctl(id, -1, setall, input) == 0, "SETALL ignores semnum");
    require(f.ctl(id, -1, getall, output) == 0, "GETALL ignores semnum");
    uint16_t actual[2]{};
    f.memory.copy_out(output, actual, sizeof(actual));
    require(actual[0] == 6 && actual[1] == 7 && f.ctl(id, 1, getpid_cmd) == 1, "GETALL contents");
    values[0] = 4;
    values[1] = 32768;
    f.memory.copy_in(input, values, sizeof(values));
    require(f.ctl(id, 0, setall, input) == -34 && f.ctl(id, 0, getval) == 6,
            "SETALL range failure must be atomic");
    require(f.ctl(id, 0, setval, uint32_t(-1)) == -34 && f.ctl(id, 2, getval) == -22,
            "control argument validation");
    require(f.ctl(id, 0, getall, 0x500000) == -14 && f.ctl(id, 0, setall, 0x500000) == -14,
            "bad control pointer");
    require(f.ctl(0, 0, info, output) == 0, "IPC_INFO");
    Seminfo limits{};
    f.memory.copy_out(output, &limits, sizeof(limits));
    require(limits.semmni == 2 && limits.semmsl == 3 && limits.semmns == 4 && limits.semopm == 3,
            "IPC_INFO limits");
    require(f.ctl(0, 0, sem_info, output) == 0, "SEM_INFO");
    f.memory.copy_out(output, &limits, sizeof(limits));
    require(limits.semusz == 1 && limits.semaem == 2, "SEM_INFO usage");
    require(f.ctl(id & 32767, 0, sem_stat, output) == id, "SEM_STAT index");
    require(f.get(0, 3, 0) == -28, "SEMMNS");
    const int second = int(f.get(0, 2, 0));
    require(second >= 0 && f.get(0, 1, 0) == -28, "SEMMNI");
    require(f.ctl(id, 0, rmid) == 0 && f.ctl(id, 0, getval) == -22, "IPC_RMID");
    const int replacement = int(f.get(77, 1, create | 0600));
    require(replacement >= 0 && replacement != id && f.ctl(id, 0, getval) == -22, "stale id");
    require(f.ctl(second, 0, rmid) == 0, "remove private set");
}

void permissions_and_namespaces() {
    Fixture f;
    const int id = int(f.get(123, 1, create | 0640));
    auto original = f.credentials;
    f.credentials.euid = 3000;
    f.credentials.egid = f.credentials.fsgid = 3000;
    require(f.get(123, 0, 0400) == -13 && f.ctl(id, 0, getval) == -13,
            "other denied read");
    require(f.ctl(id & 32767, 0, sem_stat_any, output) == id, "SEM_STAT_ANY bypasses read mode");
    require(f.ctl(id, 0, rmid) == -1, "IPC_RMID requires ownership");
    f.credentials.groups.push_back(2000);
    require(f.ctl(id, 0, getval) == 0 && f.ctl(id, 0, setval, 1) == -13,
            "supplementary group read only");
    f.credentials.cap_ipc_owner = true;
    require(f.ctl(id, 0, setval, 1) == 0 && f.ctl(id, 0, rmid) == -1,
            "CAP_IPC_OWNER is not CAP_SYS_ADMIN");
    f.credentials = original;
    Semid request = f.read_stat(id);
    request.perm.uid = 3000;
    request.perm.gid = 3001;
    request.perm.mode = 0600;
    f.memory.copy_in(input, &request, sizeof(request));
    require(f.ctl(id, 0, set, input) == 0, "IPC_SET");
    f.credentials.euid = 3000;
    f.credentials.egid = f.credentials.fsgid = 3001;
    require(f.ctl(id, 0, getval) == 1 && f.ctl(id, 0, rmid) == 0, "new owner");

    const int shared = int(f.get(456, 1, create | 0600));
    auto first = f.ns;
    f.ns = make_semaphore_namespace(f.scheduler);
    require(f.get(456, 0, 0) == -2, "IPC namespace isolation");
    require(f.get(0, 1, 0) == 0, "namespace collision slot setup");
    const int isolated = int(f.get(456, 1, create | 0600));
    require(isolated == shared && f.ctl(isolated, 0, setval, 9) == 0, "namespace id collision setup");
    f.ns = first;
    require(f.ctl(shared, 0, getval) == 0, "namespace state must not collide");
}

void atomic_operations_and_waiters() {
    Fixture f;
    const int id = int(f.get(1, 2, create | 0666));
    const auto peer = f.spawn();
    require(f.ctl(id, 0, setval, 1) == 0, "set atomic initial value");
    require(f.op(peer, id, {{0, -1, 0}, {1, -1, nowait}}) == -11 &&
                f.ctl(id, 0, getval) == 1, "atomic rollback on IPC_NOWAIT");
    const auto complex_result = f.op(peer, id, {{0, -1, 0}, {0, 1, 0}, {1, -1, 0}});
    require(complex_result == 0, "complex wait call");
    require(f.task(peer).state == Task::State::Waiting, "complex waiter state");
    require(f.ctl(id, 0, getval) == 1, "complex wait must not commit prefix");
    require(f.ctl(id, 1, getncnt) == 1, "complex wait second semaphore count");
    require(f.ctl(id, 0, getncnt) == 0, "complex wait first semaphore count");
    require(f.ctl(id, 1, setval, 1) == 0 && f.task(peer).state == Task::State::Runnable &&
                int64_t(f.task(peer).cpu.x[0]) == 0 && f.ctl(id, 0, getval) == 1 &&
                f.ctl(id, 1, getval) == 0 && f.ctl(id, 0, getpid_cmd) == peer &&
                f.ctl(id, 1, getpid_cmd) == peer && f.read_stat(id).otime > 0,
            "complex wake commits atomically");
    require(f.op(peer, id, {{0, 0, 0}}) == 0 && f.ctl(id, 0, getzcnt) == 1,
            "wait-for-zero count");
    require(f.ctl(id, 0, setval, 0) == 0 && f.ctl(id, 0, getzcnt) == 0 &&
                f.task(peer).state == Task::State::Runnable,
            "zero waiter wake");
    require(f.op(peer, id, {{0, -1, 0}}) == 0 && f.ctl(id, 0, getncnt) == 1,
            "wait before removal");
    require(f.ctl(id, 0, rmid) == 0 && int64_t(f.task(peer).cpu.x[0]) == -43 &&
                f.task(peer).state == Task::State::Runnable,
            "IPC_RMID must wake with EIDRM");
}

void timeouts_signals_and_faults() {
    Fixture f({4, 8, 2, 16});
    const int id = int(f.get(9, 1, create | 0666));
    const auto peer = f.spawn();
    require(f.op(peer, id, {}) == -22, "empty operations");
    require(f.op(peer, id, {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}}) == -7, "SEMOPM");
    require(f.op(peer, id, {{1, 0, 0}}) == -27, "EFBIG semaphore index");
    require(f.call(peer, 193, id, 0x500000, 1) == -14, "invalid op pointer");
    f.timespec(0, 1000000000);
    require(f.op(peer, id, {{0, -1, 0}}, timer, true) == -22, "invalid timeout");
    f.timespec(0, 0);
    require(f.op(peer, id, {{0, -1, 0}}, timer, true) == -11, "zero timeout");
    require(f.op(peer, id, {{0, 1, 0}}, timer, true) == 0, "zero timeout when operation is ready");
    require(f.ctl(id, 0, setval, 0) == 0, "reset timeout setup");
    f.timespec(100, 0);
    require(f.op(peer, id, {{0, -1, 0}}, timer, true) == 0, "timed wait setup");
    f.task(peer).deadline = TaskClock::now();
    f.scheduler.poll_waiting();
    require(int64_t(f.task(peer).cpu.x[0]) == -11 && f.ctl(id, 0, getncnt) == 0,
            "timeout returns EAGAIN and dequeues");
    f.task(peer).signals.restart.emplace();
    f.task(peer).signals.restart->restartable = true;
    require(f.op(peer, id, {{0, -1, 0}}) == 0 && !f.task(peer).signals.restart->restartable,
            "semop must not restart on signal");
    f.scheduler.interrupt(peer);
    require(int64_t(f.task(peer).cpu.x[0]) == -4 && f.ctl(id, 0, getncnt) == 0,
            "signal returns EINTR and dequeues");
    require(f.op(peer, id, {{0, -1, 0}}, 0, false, [] { return true; }) == -4 &&
                f.task(peer).state == Task::State::Runnable && f.ctl(id, 0, getncnt) == 0,
            "already pending signal");
    require(f.ctl(id, 0, setval, 32767) == 0 && f.op(peer, id, {{0, 1, 0}}) == -34,
            "SEMVMX range");
}

void undo_lifecycle() {
    Fixture f;
    const int id = int(f.get(7, 1, create | 0666));
    const auto owner = f.spawn();
    const auto child = f.spawn(false, true, owner);
    const auto sharing = f.spawn(true, false, owner);
    const auto waiter = f.spawn();
    require(f.ctl(id, 0, setval, 1) == 0 && f.op(owner, id, {{0, -1, undo}}) == 0,
            "SEM_UNDO acquire");
    require(f.op(waiter, id, {{0, -1, 0}}) == 0 && f.ctl(id, 0, getncnt) == 1,
            "undo waiter setup");
    f.exit(child);
    require(f.ctl(id, 0, getval) == 0 && f.task(waiter).state == Task::State::Waiting,
            "fork must not inherit adjustments");
    f.exit(owner);
    require(f.ctl(id, 0, getval) == 0 && f.task(waiter).state == Task::State::Waiting,
            "CLONE_SYSVSEM defers undo until last sharer");
    f.exit(sharing);
    require(f.task(waiter).state == Task::State::Runnable && f.ctl(id, 0, getval) == 0 &&
                f.ctl(id, 0, getpid_cmd) == waiter,
            "last sharer applies undo and wakes waiter");

    const auto first = f.spawn();
    const auto second = f.spawn();
    require(f.ctl(id, 0, setval, 4) == 0 && f.op(first, id, {{0, -1, undo}}) == 0 &&
                f.op(second, id, {{0, -1, undo}}) == 0 && f.ctl(id, 0, setval, 5) == 0,
            "SETVAL undo clearing setup");
    f.exit(first);
    f.exit(second);
    require(f.ctl(id, 0, getval) == 5, "SETVAL clears all processes' adjustments");
    const auto clamp = f.spawn();
    require(f.op(clamp, id, {{0, 4, undo}}) == 0 && f.op(1, id, {{0, -9, 0}}) == 0,
            "undo clamp setup");
    f.exit(clamp);
    require(f.ctl(id, 0, getval) == 0, "negative exit adjustment clamps at zero");
    const auto removed = f.spawn();
    require(f.ctl(id, 0, setval, 1) == 0 && f.op(removed, id, {{0, -1, undo}}) == 0 &&
                f.ctl(id, 0, rmid) == 0,
            "undo removal setup");
    const int replacement = int(f.get(7, 1, create | 0666));
    f.exit(removed);
    require(f.ctl(replacement, 0, getval) == 0, "removed undo must not touch replacement");
}

void blocked_exit_and_count_changes() {
    Fixture f;
    const int id = int(f.get(0, 2, 0666));
    const auto owner = f.spawn();
    const auto peer = f.spawn();
    require(f.ctl(id, 0, setval, 1) == 0 && f.op(owner, id, {{0, -1, undo}}) == 0 &&
                f.op(owner, id, {{0, -1, 0}}) == 0 && f.op(peer, id, {{0, -1, 0}}) == 0,
            "blocked exiting owner setup");
    f.exit(owner);
    require(f.task(peer).state == Task::State::Runnable && f.ctl(id, 0, getval) == 0 &&
                f.ctl(id, 0, getncnt) == 0,
            "exiting waiter must be removed before applying undo");
    require(f.op(peer, id, {{0, -1, 0}, {1, -1, 0}}) == 0 && f.ctl(id, 0, getncnt) == 1,
            "initial complex blocker");
    require(f.ctl(id, 0, setval, 1) == 0 && f.ctl(id, 0, getncnt) == 0 &&
                f.ctl(id, 1, getncnt) == 1, "complex blocker changes after recheck");
    require(f.ctl(id, 1, setval, 1) == 0 && f.task(peer).state == Task::State::Runnable,
            "complex blocker eventual success");
}

void undo_bounds_and_setall() {
    Fixture f;
    const int id = int(f.get(0, 2, 0666));
    const auto upper = f.spawn();
    require(f.ctl(id, 0, setval, 32767) == 0 && f.op(upper, id, {{0, -32767, undo}}) == 0 &&
                f.op(1, id, {{0, 1, 0}}) == 0,
            "positive undo bound setup");
    require(f.op(upper, id, {{0, -1, undo}}) == -34 && f.ctl(id, 0, getval) == 1,
            "positive undo overflow must roll back");
    f.exit(upper);
    require(f.ctl(id, 0, getval) == 32767 && f.ctl(id, 0, getpid_cmd) == upper,
            "positive exit adjustment clamps and updates sempid");

    const auto lower = f.spawn();
    require(f.ctl(id, 0, setval, 0) == 0 && f.op(lower, id, {{0, 32767, undo}}) == 0 &&
                f.op(1, id, {{0, -32767, 0}}) == 0 && f.op(lower, id, {{0, 1, undo}}) == 0 &&
                f.op(1, id, {{0, -1, 0}}) == 0,
            "negative undo bound setup");
    require(f.op(lower, id, {{0, 1, undo}}) == -34 && f.ctl(id, 0, getval) == 0,
            "negative undo overflow must roll back");
    f.exit(lower);
    require(f.ctl(id, 0, getval) == 0, "negative exit adjustment clamps");

    const auto owner = f.spawn();
    uint16_t initial[2]{2, 3};
    f.memory.copy_in(input, initial, sizeof(initial));
    require(f.ctl(id, 0, setall, input) == 0 &&
                f.op(owner, id, {{0, -1, undo}, {1, -1, undo}}) == 0,
            "SETALL undo setup");
    uint16_t replacement[2]{6, 7};
    f.memory.copy_in(input, replacement, sizeof(replacement));
    require(f.ctl(id, 0, setall, input) == 0, "SETALL replacement");
    f.exit(owner);
    require(f.ctl(id, 0, getval) == 6 && f.ctl(id, 1, getval) == 7,
            "SETALL clears adjustments across the set");
}

void waiter_order_and_error_recheck() {
    Fixture f;
    const int id = int(f.get(0, 2, 0666));
    const auto first = f.spawn();
    const auto second = f.spawn();
    require(f.op(first, id, {{0, -1, 0}}) == 0 && f.op(second, id, {{0, -1, 0}}) == 0 &&
                f.ctl(id, 0, getncnt) == 2,
            "two queued waiters");
    require(f.op(1, id, {{0, 1, 0}}) == 0 && f.task(first).state == Task::State::Runnable &&
                f.task(second).state == Task::State::Waiting && f.ctl(id, 0, getncnt) == 1,
            "FIFO wake of a single eligible waiter");
    require(f.op(1, id, {{0, 1, 0}}) == 0 && f.task(second).state == Task::State::Runnable,
            "second queued waiter wakes");

    require(f.ctl(id, 1, setval, 32767) == 0 &&
                f.op(first, id, {{0, -1, 0}, {1, 1, 0}}) == 0,
            "range recheck wait setup");
    require(f.ctl(id, 0, setval, 1) == 0 && f.task(first).state == Task::State::Runnable &&
                int64_t(f.task(first).cpu.x[0]) == -34 && f.ctl(id, 0, getval) == 1 &&
                f.ctl(id, 1, getval) == 32767,
            "failed queued recheck is atomic and reports ERANGE");
}
} // namespace

int main() {
    try {
        creation_and_controls();
        permissions_and_namespaces();
        atomic_operations_and_waiters();
        timeouts_signals_and_faults();
        undo_lifecycle();
        blocked_exit_and_count_changes();
        undo_bounds_and_setall();
        waiter_order_and_error_recheck();
        std::puts("sysv semaphore tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "sysv semaphore test failed: %s\n", error.what());
        return 1;
    }
}
