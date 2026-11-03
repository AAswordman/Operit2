#include "linux/ipc/sysv_sem.h"
#include "base/support.h"
#include "linux/abi.h"
#include "memory/address_space.h"
#include "runtime/tasks/scheduler.h"
#include <algorithm>
#include <chrono>
#include <ctime>
#include <list>
#include <map>
#include <new>
#include <utility>

namespace operit::arm64::ipc {
namespace {
constexpr uint32_t ipc_creat = 01000, ipc_excl = 02000, ipc_nowait = 04000, sem_undo = 010000;
constexpr int semvmx = 32767, semaem = 32767;
constexpr uint32_t index_bits = 15, index_mask = (1u << index_bits) - 1;
constexpr int ipc_rmid = 0, ipc_set = 1, ipc_stat = 2, ipc_info = 3;
constexpr int getpid = 11, getval = 12, getall = 13, getncnt = 14, getzcnt = 15;
constexpr int setval = 16, setall = 17, sem_stat = 18, sem_info = 19, sem_stat_any = 20;

struct GuestIpcPerm {
    int32_t key;
    uint32_t uid, gid, cuid, cgid, mode;
    uint16_t sequence, pad;
    uint64_t unused1, unused2;
};
struct GuestSemidDs {
    GuestIpcPerm perm;
    int64_t otime, ctime;
    uint64_t nsems, unused1, unused2;
};
struct GuestSeminfo {
    int32_t semmap, semmni, semmns, semmnu, semmsl, semopm, semume, semusz, semvmx, semaem;
};
struct Operation {
    uint16_t number;
    int16_t value, flags;
};
static_assert(sizeof(GuestIpcPerm) == 48 && sizeof(GuestSemidDs) == 88 &&
              sizeof(GuestSeminfo) == 40 && sizeof(Operation) == 6,
              "AArch64 System V semaphore ABI");

struct SemaphoreValue {
    int value = 0;
    uint32_t pid = 0;
};
class SemaphoreSet;
struct UndoRecord;
using SetKey = std::weak_ptr<SemaphoreSet>;
using UndoMap = std::map<SetKey, std::shared_ptr<UndoRecord>, std::owner_less<SetKey>>;

int64_t now() noexcept { return int64_t(std::time(nullptr)); }

bool member_of(const SemaphoreCredentials &credentials, uint32_t gid) {
    return credentials.fsgid == gid ||
           std::find(credentials.groups.begin(), credentials.groups.end(), gid) != credentials.groups.end();
}

TaskDeadline deadline_for(const linux_abi::Timespec &timeout) {
    if (timeout.seconds < 0 || timeout.nanoseconds < 0 || timeout.nanoseconds >= 1000000000)
        guest_error(22);
    const auto current = TaskClock::now();
    const auto available = TaskClock::time_point::max() - current;
    const auto seconds = std::chrono::duration_cast<std::chrono::seconds>(available);
    if (timeout.seconds > seconds.count() ||
        (timeout.seconds == seconds.count() &&
         std::chrono::nanoseconds(timeout.nanoseconds) > available - seconds))
        return TaskClock::time_point::max();
    return current + std::chrono::seconds(timeout.seconds) + std::chrono::nanoseconds(timeout.nanoseconds);
}
} // namespace

class SemaphoreUndo {
  public:
    UndoMap records;
};

namespace {
struct UndoRecord {
    std::weak_ptr<SemaphoreUndo> owner;
    std::vector<int> adjustments;
};

class SemaphoreSet {
  public:
    int32_t key = 0, id = 0;
    uint16_t sequence = 0;
    uint32_t uid = 0, gid = 0, cuid = 0, cgid = 0, mode = 0;
    int64_t otime = 0, ctime = 0;
    bool removed = false;
    std::vector<SemaphoreValue> semaphores;
    std::list<std::weak_ptr<UndoRecord>> undos;
    std::list<std::shared_ptr<SemaphoreWaiter>> waiters;
};

void check_access(const SemaphoreSet &set, const SemaphoreCredentials &credentials, uint32_t mode) {
    const uint32_t requested = (mode | (mode >> 3) | (mode >> 6)) & 7;
    uint32_t granted = set.mode;
    if (credentials.euid == set.uid || credentials.euid == set.cuid)
        granted >>= 6;
    else if (member_of(credentials, set.gid) || member_of(credentials, set.cgid))
        granted >>= 3;
    if ((requested & ~granted) && !credentials.cap_ipc_owner)
        guest_error(13);
}

void check_control(const SemaphoreSet &set, const SemaphoreCredentials &credentials) {
    if (credentials.euid != set.uid && credentials.euid != set.cuid && !credentials.cap_sys_admin)
        guest_error(1);
}
} // namespace

class SemaphoreNamespace {
  public:
    Scheduler &scheduler;
    SemaphoreLimits limits;
    std::vector<std::shared_ptr<SemaphoreSet>> slots;
    std::vector<uint16_t> sequences;
    std::vector<bool> used;
    std::map<int32_t, uint32_t> keys;
    uint32_t next_slot = 0, total_semaphores = 0, total_sets = 0;

    SemaphoreNamespace(Scheduler &scheduler_value, const SemaphoreLimits &limits_value)
        : scheduler(scheduler_value), limits(limits_value), slots(limits.max_sets),
          sequences(limits.max_sets), used(limits.max_sets) {}

    std::shared_ptr<SemaphoreSet> lookup(int32_t id, bool index = false) const {
        if (id < 0)
            guest_error(22);
        const uint32_t slot = uint32_t(id) & index_mask;
        if (slot >= slots.size() || !slots[slot] || (!index && slots[slot]->id != id))
            guest_error(22);
        return slots[slot];
    }
    int max_index() const noexcept {
        for (size_t i = slots.size(); i; --i)
            if (slots[i - 1])
                return int(i - 1);
        return 0;
    }
};

class SemaphoreWaiter {
  public:
    uint32_t tid = 0, pid = 0;
    bool queued = false;
    size_t blocking = 0;
    std::weak_ptr<SemaphoreSet> set;
    std::vector<Operation> operations;
    std::shared_ptr<UndoRecord> undo;
    std::vector<int> values, adjustments;
};

namespace {
std::shared_ptr<UndoRecord> find_undo(const std::shared_ptr<SemaphoreUndo> &group,
                                      const std::shared_ptr<SemaphoreSet> &set) {
    const SetKey key = set;
    auto found = group->records.find(key);
    if (found != group->records.end())
        return found->second;
    auto record = std::make_shared<UndoRecord>();
    record->owner = group;
    record->adjustments.resize(set->semaphores.size());
    auto inserted = group->records.emplace(key, record);
    try {
        set->undos.push_back(record);
    } catch (...) {
        group->records.erase(inserted.first);
        throw;
    }
    return record;
}

void erase_waiter(const std::shared_ptr<SemaphoreWaiter> &waiter) {
    if (!waiter->queued)
        return;
    waiter->queued = false;
    if (auto set = waiter->set.lock())
        set->waiters.remove(waiter);
}

// The scratch arrays are allocated before enqueueing, so rechecks, removal,
// and undo wakeups do not need to allocate or touch guest memory.
int attempt(SemaphoreSet &set, SemaphoreWaiter &waiter) {
    for (size_t i = 0; i < set.semaphores.size(); ++i)
        waiter.values[i] = set.semaphores[i].value;
    if (waiter.undo)
        std::copy(waiter.undo->adjustments.begin(), waiter.undo->adjustments.end(), waiter.adjustments.begin());
    for (size_t i = 0; i < waiter.operations.size(); ++i) {
        const auto &op = waiter.operations[i];
        const int value = waiter.values[op.number];
        if ((!op.value && value) || value + op.value < 0) {
            waiter.blocking = i;
            return (uint16_t(op.flags) & ipc_nowait) ? -11 : 1;
        }
        const int next = value + op.value;
        if (next > semvmx)
            return -34;
        if (uint16_t(op.flags) & sem_undo) {
            const int adjustment = waiter.adjustments[op.number] - op.value;
            if (adjustment < -semaem - 1 || adjustment > semaem)
                return -34;
            waiter.adjustments[op.number] = adjustment;
        }
        waiter.values[op.number] = next;
    }
    for (const auto &op : waiter.operations) {
        set.semaphores[op.number].value = waiter.values[op.number];
        set.semaphores[op.number].pid = waiter.pid;
    }
    if (waiter.undo)
        std::copy(waiter.adjustments.begin(), waiter.adjustments.end(), waiter.undo->adjustments.begin());
    set.otime = now();
    return 0;
}

void wake_ready(Scheduler &scheduler, SemaphoreSet &set) {
    bool changed;
    do {
        changed = false;
        for (auto it = set.waiters.begin(); it != set.waiters.end();) {
            auto waiter = *it++;
            const int result = attempt(set, *waiter);
            if (result == 1)
                continue;
            // wake() invokes the task's cleanup, which removes this waiter.
            scheduler.wake(waiter->tid, result);
            if (waiter->queued)
                erase_waiter(waiter);
            if (!result)
                changed = true;
        }
    } while (changed);
}

void clear_undo(SemaphoreSet &set, int number = -1) {
    for (auto it = set.undos.begin(); it != set.undos.end();) {
        if (auto record = it->lock()) {
            if (number < 0)
                std::fill(record->adjustments.begin(), record->adjustments.end(), 0);
            else
                record->adjustments[size_t(number)] = 0;
            ++it;
        } else
            it = set.undos.erase(it);
    }
}

void remove_set(SemaphoreNamespace &ns, const std::shared_ptr<SemaphoreSet> &set) {
    set->removed = true;
    for (const auto &weak : set->undos) {
        if (auto record = weak.lock())
            if (auto group = record->owner.lock())
                group->records.erase(SetKey(set));
    }
    set->undos.clear();
    if (set->key)
        ns.keys.erase(set->key);
    ns.slots[uint32_t(set->id) & index_mask].reset();
    ns.total_semaphores -= uint32_t(set->semaphores.size());
    --ns.total_sets;
    while (!set->waiters.empty()) {
        auto waiter = set->waiters.front();
        ns.scheduler.wake(waiter->tid, -43);
        erase_waiter(waiter);
    }
}

int64_t semget(SemaphoreNamespace &ns, const SemaphoreCredentials &credentials,
               int32_t key, int32_t count, uint32_t flags) {
    if (count < 0 || uint32_t(count) > ns.limits.max_per_set)
        guest_error(22);
    if (key) {
        auto found = ns.keys.find(key);
        if (found != ns.keys.end()) {
            auto set = ns.slots[found->second];
            if ((flags & (ipc_creat | ipc_excl)) == (ipc_creat | ipc_excl))
                guest_error(17);
            if (count && size_t(count) > set->semaphores.size())
                guest_error(22);
            check_access(*set, credentials, flags & 0777);
            return set->id;
        }
        if (!(flags & ipc_creat))
            guest_error(2);
    }
    if (!count)
        guest_error(22);
    if (ns.total_sets >= ns.limits.max_sets || uint32_t(count) > ns.limits.max_semaphores - ns.total_semaphores)
        guest_error(28);
    uint32_t slot = ns.next_slot;
    while (ns.slots[slot])
        slot = (slot + 1) % ns.limits.max_sets;
    const uint16_t sequence = ns.used[slot] ? uint16_t(ns.sequences[slot] + 1) : ns.sequences[slot];
    auto set = std::make_shared<SemaphoreSet>();
    set->semaphores.resize(size_t(count));
    set->key = key;
    set->sequence = sequence;
    set->id = int32_t((uint32_t(sequence) << index_bits) | slot);
    set->uid = set->cuid = credentials.euid;
    set->gid = set->cgid = credentials.egid;
    set->mode = flags & 0777;
    set->ctime = now();
    if (key)
        ns.keys.emplace(key, slot);
    ns.slots[slot] = set;
    ns.used[slot] = true;
    ns.sequences[slot] = sequence;
    ns.next_slot = (slot + 1) % ns.limits.max_sets;
    ns.total_semaphores += uint32_t(count);
    ++ns.total_sets;
    return set->id;
}

int64_t semctl(SemaphoreNamespace &ns, Memory &memory, const SemaphoreCredentials &credentials,
               int32_t id, int32_t number, int32_t command, uint64_t arg, uint32_t pid) {
    if (id < 0)
        guest_error(22);
    if (command == ipc_info || command == sem_info) {
        const int32_t standard = 1024000000;
        GuestSeminfo info{standard, int32_t(ns.limits.max_sets), int32_t(ns.limits.max_semaphores),
                          standard, int32_t(ns.limits.max_per_set), int32_t(ns.limits.max_operations),
                          500, command == sem_info ? int32_t(ns.total_sets) : 20, semvmx,
                          command == sem_info ? int32_t(ns.total_semaphores) : semaem};
        memory.copy_in(arg, &info, sizeof(info));
        return ns.max_index();
    }
    if (command == setval && (int32_t(arg) < 0 || int32_t(arg) > semvmx))
        guest_error(34);
    if (command != ipc_rmid && command != ipc_set && command != ipc_stat && command != sem_stat &&
        command != sem_stat_any && command != getpid && command != getval && command != getall &&
        command != getncnt && command != getzcnt && command != setval && command != setall)
        guest_error(22);
    GuestSemidDs requested{};
    if (command == ipc_set)
        memory.copy_out(arg, &requested, sizeof(requested));
    auto set = ns.lookup(id, command == sem_stat || command == sem_stat_any);
    if (command == ipc_rmid || command == ipc_set) {
        check_control(*set, credentials);
        if (command == ipc_rmid)
            remove_set(ns, set);
        else {
            set->uid = requested.perm.uid;
            set->gid = requested.perm.gid;
            set->mode = requested.perm.mode & 0777;
            set->ctime = now();
        }
        return 0;
    }
    if (command != sem_stat_any)
        check_access(*set, credentials, command == setval || command == setall ? 0222 : 0444);
    if (command == ipc_stat || command == sem_stat || command == sem_stat_any) {
        GuestSemidDs output{};
        output.perm = {set->key, set->uid, set->gid, set->cuid, set->cgid, set->mode,
                       set->sequence, 0, 0, 0};
        output.otime = set->otime;
        output.ctime = set->ctime;
        output.nsems = set->semaphores.size();
        memory.copy_in(arg, &output, sizeof(output));
        return command == ipc_stat ? 0 : set->id;
    }
    if (command == getall || command == setall) {
        std::vector<uint16_t> values(set->semaphores.size());
        if (command == getall) {
            for (size_t i = 0; i < values.size(); ++i)
                values[i] = uint16_t(set->semaphores[i].value);
            memory.copy_in(arg, values.data(), values.size() * sizeof(uint16_t));
        } else {
            memory.copy_out(arg, values.data(), values.size() * sizeof(uint16_t));
            for (const auto value : values)
                if (value > semvmx)
                    guest_error(34);
            for (size_t i = 0; i < values.size(); ++i) {
                set->semaphores[i].value = values[i];
                set->semaphores[i].pid = pid;
            }
            clear_undo(*set);
            set->ctime = now();
            wake_ready(ns.scheduler, *set);
        }
        return 0;
    }
    if (number < 0 || size_t(number) >= set->semaphores.size())
        guest_error(22);
    auto &sem = set->semaphores[size_t(number)];
    switch (command) {
    case getpid: return sem.pid;
    case getval: return sem.value;
    case getncnt:
    case getzcnt: {
        int count = 0;
        for (const auto &waiter : set->waiters) {
            const auto &op = waiter->operations[waiter->blocking];
            if (op.number == number && (command == getzcnt ? op.value == 0 : op.value < 0))
                ++count;
        }
        return count;
    }
    case setval:
        sem.value = int32_t(arg);
        sem.pid = pid;
        clear_undo(*set, number);
        set->ctime = now();
        wake_ready(ns.scheduler, *set);
        return 0;
    default: guest_error(22);
    }
}
} // namespace

SemaphoreTask::SemaphoreTask(std::shared_ptr<SemaphoreUndo> undo) : undo_(std::move(undo)) {}
SemaphoreTask::~SemaphoreTask() = default;

std::shared_ptr<SemaphoreNamespace> make_semaphore_namespace(Scheduler &scheduler,
                                                              const SemaphoreLimits &limits) try {
    if (!limits.max_sets || limits.max_sets > index_mask + 1 || !limits.max_per_set ||
        limits.max_per_set > 65536 || !limits.max_operations || !limits.max_semaphores ||
        limits.max_semaphores > INT32_MAX || limits.max_operations > INT32_MAX)
        guest_error(22);
    return std::make_shared<SemaphoreNamespace>(scheduler, limits);
} catch (const std::bad_alloc &) {
    guest_error(12);
}

std::unique_ptr<SemaphoreTask> make_semaphore_task() {
    return std::unique_ptr<SemaphoreTask>(new SemaphoreTask(std::make_shared<SemaphoreUndo>()));
}

std::unique_ptr<SemaphoreTask> clone_semaphore_task(const SemaphoreTask &parent, bool share) {
    return std::unique_ptr<SemaphoreTask>(new SemaphoreTask(share ? parent.undo_ : std::make_shared<SemaphoreUndo>()));
}

void semaphore_task_exit(std::unique_ptr<SemaphoreTask> &task, Scheduler &scheduler, uint32_t tid) {
    if (!task)
        return;
    if (auto waiter = task->waiting_.lock())
        erase_waiter(waiter);
    const auto *exiting = scheduler.lookup(tid);
    const uint32_t pid = exiting ? exiting->pid : tid;
    if (task->undo_.use_count() == 1) {
        for (auto &entry : task->undo_->records) {
            auto set = entry.first.lock();
            if (!set || set->removed)
                continue;
            for (size_t i = 0; i < set->semaphores.size(); ++i) {
                const int adjustment = entry.second->adjustments[i];
                if (!adjustment)
                    continue;
                auto &sem = set->semaphores[i];
                sem.value = std::clamp(sem.value + adjustment, 0, semvmx);
                sem.pid = pid;
            }
            wake_ready(scheduler, *set);
        }
    }
    task.reset();
}

int64_t semaphore_syscall(const std::shared_ptr<SemaphoreNamespace> &ns, SemaphoreTask &task,
                          Memory &memory, const SemaphoreCredentials &credentials, uint64_t number,
                          uint64_t arg0, uint64_t arg1, uint64_t arg2, uint64_t arg3,
                          const std::function<bool()> &interrupted) try {
    if (!ns || !task.undo_)
        guest_error(22);
    const Memory::AccessGuard access(memory);
    auto &scheduler = ns->scheduler;
    if (number == 190)
        return semget(*ns, credentials, int32_t(arg0), int32_t(arg1), uint32_t(arg2));
    if (number == 191)
        return semctl(*ns, memory, credentials, int32_t(arg0), int32_t(arg1), int32_t(arg2), arg3,
                      scheduler.current().pid);
    if (number != 192 && number != 193)
        guest_error(38);
    const auto id = int32_t(arg0);
    const auto count = uint32_t(arg2);
    if (id < 0 || !count)
        guest_error(22);
    if (count > ns->limits.max_operations)
        guest_error(7);
    TaskDeadline deadline;
    if (number == 192 && arg3) {
        linux_abi::Timespec timeout{};
        memory.copy_out(arg3, &timeout, sizeof(timeout));
        deadline = deadline_for(timeout);
    }
    auto waiter = std::make_shared<SemaphoreWaiter>();
    waiter->operations.resize(count);
    memory.copy_out(arg1, waiter->operations.data(), count * sizeof(Operation));
    auto set = ns->lookup(id);
    bool modifies = false, needs_undo = false;
    for (const auto &op : waiter->operations) {
        if (op.number >= set->semaphores.size())
            guest_error(27);
        modifies |= op.value != 0;
        needs_undo |= (uint16_t(op.flags) & sem_undo) != 0;
    }
    check_access(*set, credentials, modifies ? 0222 : 0444);
    waiter->tid = scheduler.current_tid();
    waiter->pid = scheduler.current().pid;
    waiter->set = set;
    waiter->values.resize(set->semaphores.size());
    if (needs_undo) {
        waiter->undo = find_undo(task.undo_, set);
        waiter->adjustments.resize(set->semaphores.size());
    }
    const int result = attempt(*set, *waiter);
    if (result < 0)
        guest_error(-result);
    if (!result) {
        if (modifies)
            wake_ready(scheduler, *set);
        return 0;
    }
    if (deadline && *deadline <= TaskClock::now())
        guest_error(11);
    auto &current = scheduler.current();
    if (current.state != Task::State::Runnable || current.wait_cleanup || !task.waiting_.expired())
        guest_error(22);
    if (current.signals.restart)
        current.signals.restart->restartable = false;
    if (interrupted && interrupted())
        guest_error(4);
    std::function<void()> cleanup = [ns, waiter] {
        (void)ns;
        erase_waiter(waiter);
    };
    set->waiters.push_back(waiter);
    waiter->queued = true;
    task.waiting_ = waiter;
    scheduler.block(deadline, -11);
    current.wait_cleanup = std::move(cleanup);
    return 0;
} catch (const std::bad_alloc &) {
    guest_error(12);
}
} // namespace operit::arm64::ipc
