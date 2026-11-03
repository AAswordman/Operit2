#include "linux/ipc/sysv_msg.h"
#include "base/support.h"
#include "memory/address_space.h"
#include "memory/fault.h"
#include "runtime/tasks/scheduler.h"
#include <algorithm>
#include <climits>
#include <ctime>
#include <list>
#include <map>
#include <new>
#include <utility>

namespace operit::arm64::ipc {
namespace {
constexpr uint32_t ipc_creat = 01000, ipc_excl = 02000, ipc_nowait = 04000;
constexpr uint32_t msg_noerror = 010000, msg_except = 020000, msg_copy = 040000;
constexpr int ipc_rmid = 0, ipc_set = 1, ipc_stat = 2, ipc_info = 3;
constexpr int msg_stat = 11, msg_info = 12, msg_stat_any = 13;
constexpr uint32_t index_bits = 15, index_mask = (1u << index_bits) - 1;

// asm-generic ipc64_perm, msqid64_ds and msginfo on little-endian AArch64.
struct GuestIpcPerm {
    int32_t key = 0;
    uint32_t uid = 0, gid = 0, cuid = 0, cgid = 0, mode = 0;
    uint16_t sequence = 0, pad = 0;
    uint64_t unused1 = 0, unused2 = 0;
};
struct GuestMsqidDs {
    GuestIpcPerm perm;
    int64_t stime = 0, rtime = 0, ctime = 0;
    uint64_t cbytes = 0, qnum = 0, qbytes = 0;
    int32_t lspid = 0, lrpid = 0;
    uint64_t unused1 = 0, unused2 = 0;
};
struct GuestMsginfo {
    int32_t pool = 0, map = 0, max = 0, mnb = 0, mni = 0, ssz = 0, tql = 0;
    uint16_t seg = 0, pad = 0;
};
static_assert(sizeof(GuestIpcPerm) == 48 && sizeof(GuestMsqidDs) == 120 && sizeof(GuestMsginfo) == 32,
              "AArch64 System V message ABI");

int64_t now() noexcept { return int64_t(std::time(nullptr)); }

struct Message {
    int64_t type = 0;
    std::vector<uint8_t> text;
};
struct Waiter {
    bool sender = false, queued = false;
    uint32_t tid = 0, pid = 0, flags = 0;
    MessageQueueCredentials credentials;
    Memory *memory = nullptr;
    uint64_t address = 0, size = 0;
    int64_t type = 0;
    Message message;
};
struct Queue {
    int32_t key = 0, id = 0;
    uint32_t uid = 0, gid = 0, cuid = 0, cgid = 0, mode = 0;
    uint16_t sequence = 0;
    uint64_t bytes = 0, max_bytes = 0;
    int64_t stime = 0, rtime = 0, ctime = 0;
    uint32_t lspid = 0, lrpid = 0;
    std::list<Message> messages;
    std::list<std::shared_ptr<Waiter>> senders, receivers;
};

bool member_of(const MessageQueueCredentials &credentials, uint32_t gid) {
    return credentials.fsgid == gid ||
           std::find(credentials.groups.begin(), credentials.groups.end(), gid) != credentials.groups.end();
}
bool allowed(const Queue &queue, const MessageQueueCredentials &credentials, uint32_t mode) {
    const uint32_t requested = (mode | (mode >> 3) | (mode >> 6)) & 7;
    uint32_t granted = queue.mode;
    if (credentials.euid == queue.uid || credentials.euid == queue.cuid)
        granted >>= 6;
    else if (member_of(credentials, queue.gid) || member_of(credentials, queue.cgid))
        granted >>= 3;
    return !(requested & ~granted) || credentials.cap_ipc_owner;
}
void check_access(const Queue &queue, const MessageQueueCredentials &credentials, uint32_t mode) {
    if (!allowed(queue, credentials, mode))
        guest_error(13);
}
void check_control(const Queue &queue, const MessageQueueCredentials &credentials) {
    if (credentials.euid != queue.uid && credentials.euid != queue.cuid && !credentials.cap_sys_admin)
        guest_error(1);
}
bool matches(int64_t message_type, int64_t desired, uint32_t flags) {
    if (!desired)
        return true;
    if (desired > 0)
        return flags & msg_except ? message_type != desired : message_type == desired;
    const int64_t limit = desired == INT64_MIN ? INT64_MAX : -desired;
    return message_type <= limit;
}
bool fits(const Queue &queue, size_t size) {
    return queue.bytes <= queue.max_bytes && size <= queue.max_bytes - queue.bytes &&
           queue.messages.size() < queue.max_bytes;
}
int64_t write_message(Memory &memory, uint64_t address, const Message &message, uint64_t size) {
    const auto count = std::min<uint64_t>(size, message.text.size());
    memory.store(address, uint64_t(message.type), 8);
    if (count)
        memory.copy_in(address + 8, message.text.data(), size_t(count));
    return int64_t(count);
}
int64_t write_result(Memory &memory, uint64_t address, const Message &message, uint64_t size) {
    try {
        return write_message(memory, address, message, size);
    } catch (const GuestError &error) {
        return -int64_t(error.number);
    } catch (const MemoryFault &) {
        return -14;
    }
}
} // namespace

class MessageQueueNamespace {
  public:
    Scheduler &scheduler;
    const MessageQueueLimits limits;
    std::vector<std::shared_ptr<Queue>> slots;
    std::vector<uint16_t> sequences;
    std::vector<bool> used;
    std::map<int32_t, uint32_t> keys;
    uint32_t next_slot = 0;

    MessageQueueNamespace(Scheduler &owner, const MessageQueueLimits &configuration)
        : scheduler(owner), limits(configuration), slots(configuration.max_queues),
          sequences(configuration.max_queues), used(configuration.max_queues) {}

    std::shared_ptr<Queue> lookup(int32_t id, bool index = false) const {
        if (id < 0)
            guest_error(22);
        const uint32_t slot = uint32_t(id) & index_mask;
        if (slot >= slots.size() || !slots[slot] || (!index && slots[slot]->id != id))
            guest_error(22);
        // MSG_STAT's index is an idr index, not a full queue ID.
        if (index && uint32_t(id) != slot)
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

std::shared_ptr<MessageQueueNamespace> make_message_queue_namespace(
    Scheduler &scheduler, const MessageQueueLimits &limits) {
    if (!limits.max_queues || limits.max_queues > index_mask + 1 ||
        limits.max_message_size > INT_MAX || limits.default_queue_bytes > INT_MAX)
        guest_error(22);
    try {
        return std::make_shared<MessageQueueNamespace>(scheduler, limits);
    } catch (const std::bad_alloc &) {
        guest_error(12);
    }
}

namespace {
void detach(const std::shared_ptr<Queue> &queue, const std::shared_ptr<Waiter> &waiter) {
    if (!waiter->queued)
        return;
    waiter->queued = false;
    (waiter->sender ? queue->senders : queue->receivers).remove(waiter);
}
void finish(MessageQueueNamespace &space, const std::shared_ptr<Queue> &queue,
            std::shared_ptr<Waiter> waiter, int64_t result) {
    detach(queue, waiter);
    space.scheduler.wake(waiter->tid, result);
}

void enqueue_wait(const std::shared_ptr<MessageQueueNamespace> &space,
                  const std::shared_ptr<Queue> &queue, const std::shared_ptr<Waiter> &waiter,
                  const std::function<bool()> &has_pending_signal) {
    auto &scheduler = space->scheduler;
    auto &task = scheduler.current();
    if (task.state != Task::State::Runnable || task.wait_cleanup)
        guest_error(22);
    // Linux uses ERESTARTNOHAND: never restart in a user signal handler,
    // including one installed with SA_RESTART, but allow a handlerless restart.
    if (task.signals.restart)
        task.signals.restart->without_handler_only = true;
    if (!has_pending_signal)
        guest_error(22);
    if (has_pending_signal())
        guest_error(4);
    std::weak_ptr<Queue> weak_queue = queue;
    std::weak_ptr<Waiter> weak_waiter = waiter;
    std::function<void()> cleanup = [space, weak_queue, weak_waiter] {
        (void)space; // Retains the IPC namespace until the wait is cancelled.
        auto pending_queue = weak_queue.lock();
        auto pending = weak_waiter.lock();
        if (pending_queue && pending)
            detach(pending_queue, pending);
    };
    auto &waiters = waiter->sender ? queue->senders : queue->receivers;
    waiters.push_back(waiter);
    waiter->queued = true;
    scheduler.block({}, 0);
    task.wait_cleanup = std::move(cleanup);
}

int64_t get_queue(MessageQueueNamespace &space, const MessageQueueCredentials &credentials,
                  int32_t key, uint32_t flags) {
    if (key) {
        const auto found = space.keys.find(key);
        if (found != space.keys.end()) {
            const auto &queue = *space.slots[found->second];
            if ((flags & (ipc_creat | ipc_excl)) == (ipc_creat | ipc_excl))
                guest_error(17);
            check_access(queue, credentials, flags & 0777);
            return queue.id;
        }
        if (!(flags & ipc_creat))
            guest_error(2);
    }
    uint32_t slot = space.next_slot;
    for (uint32_t count = 0; count < space.slots.size(); ++count) {
        if (!space.slots[slot])
            break;
        slot = (slot + 1) % uint32_t(space.slots.size());
    }
    if (space.slots[slot])
        guest_error(28);
    const uint16_t sequence = uint16_t(space.sequences[slot] + (space.used[slot] ? 1 : 0));
    auto queue = std::make_shared<Queue>();
    queue->key = key;
    queue->id = int32_t((uint32_t(sequence) << index_bits) | slot);
    queue->sequence = sequence;
    queue->uid = queue->cuid = credentials.euid;
    queue->gid = queue->cgid = credentials.egid;
    queue->mode = flags & 0777;
    queue->max_bytes = space.limits.default_queue_bytes;
    queue->ctime = now();
    if (key)
        space.keys.emplace(key, slot);
    space.slots[slot] = std::move(queue);
    space.sequences[slot] = sequence;
    space.used[slot] = true;
    space.next_slot = (slot + 1) % uint32_t(space.slots.size());
    return space.slots[slot]->id;
}

// Called only after checking capacity. A matching blocked receiver takes the
// message directly; an undersized receiver gets E2BIG and remains unserved.
void send_message(MessageQueueNamespace &space, const std::shared_ptr<Queue> &queue,
                  Message &message, uint32_t pid) {
    auto it = queue->receivers.begin();
    while (it != queue->receivers.end()) {
        auto receiver = *it++;
        if (!matches(message.type, receiver->type, receiver->flags))
            continue;
        if (receiver->size < message.text.size() && !(receiver->flags & msg_noerror)) {
            finish(space, queue, receiver, -7);
            continue;
        }
        queue->lspid = pid;
        queue->stime = now();
        queue->lrpid = receiver->tid;
        queue->rtime = queue->stime;
        const int64_t result = write_result(*receiver->memory, receiver->address, message, receiver->size);
        finish(space, queue, receiver, result);
        return;
    }
    const size_t bytes = message.text.size();
    queue->messages.emplace_back();
    queue->messages.back() = std::move(message);
    queue->bytes += bytes;
    queue->lspid = pid;
    queue->stime = now();
}

void service_senders(MessageQueueNamespace &space, const std::shared_ptr<Queue> &queue) {
    auto it = queue->senders.begin();
    while (it != queue->senders.end()) {
        auto sender = *it++;
        if (!allowed(*queue, sender->credentials, 0222)) {
            finish(space, queue, sender, -13);
            continue;
        }
        if (!fits(*queue, sender->message.text.size()))
            continue;
        try {
            send_message(space, queue, sender->message, sender->pid);
            finish(space, queue, sender, 0);
        } catch (const std::bad_alloc &) {
            finish(space, queue, sender, -12);
        }
    }
}

int64_t send(const std::shared_ptr<MessageQueueNamespace> &space, Memory &memory,
             const MessageQueueCredentials &credentials, int32_t id, uint64_t address,
             uint64_t size, uint32_t flags, const std::function<bool()> &has_pending_signal) {
    Message message;
    message.type = int64_t(memory.load(address, 8));
    if (id < 0 || size > space->limits.max_message_size || size > INT64_MAX || message.type <= 0)
        guest_error(22);
    message.text.resize(size_t(size));
    if (size)
        memory.copy_out(address + 8, message.text.data(), size_t(size));
    const auto queue = space->lookup(id);
    check_access(*queue, credentials, 0222);
    const auto &task = space->scheduler.current();
    if (fits(*queue, size_t(size))) {
        send_message(*space, queue, message, task.pid);
        return 0;
    }
    if (flags & ipc_nowait)
        guest_error(11);
    auto waiter = std::make_shared<Waiter>();
    waiter->sender = true;
    waiter->tid = task.tid;
    waiter->pid = task.pid;
    waiter->credentials = credentials;
    waiter->message = std::move(message);
    enqueue_wait(space, queue, waiter, has_pending_signal);
    return 0;
}

int64_t receive(const std::shared_ptr<MessageQueueNamespace> &space, Memory &memory,
                const MessageQueueCredentials &credentials, int32_t id, uint64_t address,
                uint64_t size, int64_t type, uint32_t flags,
                const std::function<bool()> &has_pending_signal) {
    if (id < 0 || size > INT64_MAX)
        guest_error(22);
    if (flags & msg_copy) {
        if ((flags & msg_except) || !(flags & ipc_nowait))
            guest_error(22);
        // Linux's checkpoint/restore path first copies this many bytes in.
        std::vector<uint8_t> temporary(size_t(std::min<uint64_t>(size, space->limits.max_message_size)));
        if (!temporary.empty())
            memory.copy_out(address, temporary.data(), temporary.size());
    }
    const auto queue = space->lookup(id);
    check_access(*queue, credentials, 0444);
    auto selected = queue->messages.end();
    uint64_t ordinal = 0;
    for (auto it = queue->messages.begin(); it != queue->messages.end(); ++it) {
        if (flags & msg_copy) {
            if (type >= 0 && uint64_t(type) == ordinal) {
                selected = it;
                break;
            }
            ++ordinal;
        } else if (matches(it->type, type, flags)) {
            if (type >= 0) {
                selected = it;
                break;
            }
            if (selected == queue->messages.end() || it->type < selected->type)
                selected = it;
        }
    }
    if (selected != queue->messages.end()) {
        if (size < selected->text.size() && !(flags & msg_noerror))
            guest_error(7);
        if (flags & msg_copy) {
            if (selected->text.size() > std::min<uint64_t>(size, space->limits.max_message_size))
                guest_error(22);
            return write_message(memory, address, *selected, size);
        }
        Message message = std::move(*selected);
        queue->messages.erase(selected);
        queue->bytes -= message.text.size();
        queue->lrpid = space->scheduler.current().pid;
        queue->rtime = now();
        const int64_t result = write_result(memory, address, message, size);
        service_senders(*space, queue);
        return result;
    }
    if (flags & ipc_nowait)
        guest_error(42);
    auto waiter = std::make_shared<Waiter>();
    const auto &task = space->scheduler.current();
    waiter->tid = task.tid;
    waiter->pid = task.pid;
    waiter->credentials = credentials;
    waiter->memory = &memory;
    waiter->address = address;
    waiter->size = size;
    waiter->type = type;
    waiter->flags = flags;
    enqueue_wait(space, queue, waiter, has_pending_signal);
    return 0;
}

int64_t control(MessageQueueNamespace &space, Memory &memory, const MessageQueueCredentials &credentials,
                int32_t id, int32_t command, uint64_t address) {
    if (id < 0 || command < 0)
        guest_error(22);
    if (command == ipc_info || command == msg_info) {
        GuestMsginfo info;
        info.max = int32_t(space.limits.max_message_size);
        info.mnb = int32_t(space.limits.default_queue_bytes);
        info.mni = int32_t(space.limits.max_queues);
        info.ssz = 16;
        info.seg = 65535;
        if (command == ipc_info) {
            info.pool = 512000;
            info.map = info.tql = 16384;
        } else {
            uint64_t count = 0, bytes = 0;
            for (const auto &queue : space.slots) {
                if (!queue)
                    continue;
                ++info.pool;
                count += queue->messages.size();
                bytes += queue->bytes;
            }
            info.map = int32_t(std::min<uint64_t>(count, INT_MAX));
            info.tql = int32_t(std::min<uint64_t>(bytes, INT_MAX));
        }
        memory.copy_in(address, &info, sizeof(info));
        return space.max_index();
    }
    GuestMsqidDs input;
    if (command == ipc_set)
        memory.copy_out(address, &input, sizeof(input));
    if (command != ipc_set && command != ipc_rmid && command != ipc_stat &&
        command != msg_stat && command != msg_stat_any)
        guest_error(22);
    const auto queue = space.lookup(id, command == msg_stat || command == msg_stat_any);
    if (command == ipc_stat || command == msg_stat || command == msg_stat_any) {
        if (command != msg_stat_any)
            check_access(*queue, credentials, 0444);
        GuestMsqidDs output;
        output.perm.key = queue->key;
        output.perm.uid = queue->uid;
        output.perm.gid = queue->gid;
        output.perm.cuid = queue->cuid;
        output.perm.cgid = queue->cgid;
        output.perm.mode = queue->mode;
        output.perm.sequence = queue->sequence;
        output.stime = queue->stime;
        output.rtime = queue->rtime;
        output.ctime = queue->ctime;
        output.cbytes = queue->bytes;
        output.qnum = queue->messages.size();
        output.qbytes = queue->max_bytes;
        output.lspid = int32_t(queue->lspid);
        output.lrpid = int32_t(queue->lrpid);
        memory.copy_in(address, &output, sizeof(output));
        return command == ipc_stat ? 0 : queue->id;
    }
    check_control(*queue, credentials);
    if (command == ipc_rmid) {
        if (queue->key)
            space.keys.erase(queue->key);
        space.slots[uint32_t(queue->id) & index_mask].reset();
        while (!queue->receivers.empty())
            finish(space, queue, queue->receivers.front(), -43);
        while (!queue->senders.empty())
            finish(space, queue, queue->senders.front(), -43);
        return 0;
    }
    if (input.qbytes > INT_MAX)
        guest_error(22);
    if (input.qbytes > space.limits.default_queue_bytes && !credentials.cap_sys_resource)
        guest_error(1);
    if (input.perm.uid == UINT32_MAX || input.perm.gid == UINT32_MAX)
        guest_error(22);
    queue->uid = input.perm.uid;
    queue->gid = input.perm.gid;
    queue->mode = input.perm.mode & 0777;
    queue->max_bytes = input.qbytes;
    queue->ctime = now();
    auto it = queue->receivers.begin();
    while (it != queue->receivers.end()) {
        auto receiver = *it++;
        if (!allowed(*queue, receiver->credentials, 0444))
            finish(space, queue, receiver, -13);
    }
    service_senders(space, queue);
    return 0;
}
} // namespace

int64_t message_queue_syscall(const std::shared_ptr<MessageQueueNamespace> &space, Memory &memory,
                              const MessageQueueCredentials &credentials, uint64_t number,
                              uint64_t arg0, uint64_t arg1, uint64_t arg2, uint64_t arg3,
                              uint64_t arg4, const std::function<bool()> &has_pending_signal) {
    if (!space)
        guest_error(22);
    try {
        switch (number) {
        case 186:
            return get_queue(*space, credentials, int32_t(arg0), uint32_t(arg1));
        case 187:
            return control(*space, memory, credentials, int32_t(arg0), int32_t(arg1), arg2);
        case 188:
            return receive(space, memory, credentials, int32_t(arg0), arg1, arg2, int64_t(arg3),
                           uint32_t(arg4), has_pending_signal);
        case 189:
            return send(space, memory, credentials, int32_t(arg0), arg1, arg2, uint32_t(arg3),
                        has_pending_signal);
        default:
            guest_error(38);
        }
    } catch (const std::bad_alloc &) {
        guest_error(12);
    }
}
} // namespace operit::arm64::ipc
