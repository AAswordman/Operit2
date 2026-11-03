#include "linux/ipc/posix_mq.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/signals/signals.h"
#include "linux/timers/ready_pipe.h"
#include "memory/address_space.h"
#include "memory/fault.h"
#include "runtime/tasks/scheduler.h"
#include <algorithm>
#include <array>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <deque>
#include <fcntl.h>
#include <limits>
#include <map>
#include <mutex>
#include <time.h>
#include <unistd.h>

namespace operit::arm64::ipc {
namespace {
constexpr uint32_t create = 64, exclusive = 128, nonblock = 2048, cloexec = 524288;
constexpr uint32_t known_flags = 3 | create | exclusive | nonblock | cloexec;
constexpr uint32_t maximum_priority = 32768;
constexpr uint64_t hard_messages = 65536, hard_message_size = 16 * 1024 * 1024;
constexpr uint64_t mq_attribute_size = 64, sigevent_size = 64;

struct Charge;
struct Queue;
struct OpenFile;
struct Waiter;

struct Registry {
    std::mutex mutex;
    std::map<const OpenDescription *, std::weak_ptr<OpenFile>> entries;
};
Registry &registry() {
    static Registry value;
    return value;
}
} // namespace

class PosixMqAccounting {
  public:
    std::mutex mutex;
    std::map<uint32_t, uint64_t> usage;
};

class PosixMqPollSubscription {
  public:
    explicit PosixMqPollSubscription(std::function<void(uint32_t, bool)> callback)
        : callback(std::move(callback)) {}
    std::function<void(uint32_t, bool)> callback;
};

namespace {
struct Charge {
    std::shared_ptr<PosixMqAccounting> accounting;
    uint32_t uid;
    uint64_t bytes;
    ~Charge() {
        if (!accounting)
            return;
        std::lock_guard<std::mutex> lock(accounting->mutex);
        auto found = accounting->usage.find(uid);
        if (found != accounting->usage.end()) {
            found->second -= bytes;
            if (!found->second)
                accounting->usage.erase(found);
        }
    }
};
struct Message {
    uint32_t priority = 0;
    std::vector<uint8_t> bytes;
};
struct Notification {
    uint32_t owner = 0;
    int32_t method = 0, signal = 0;
    uint64_t value = 0;
    bool after_exec = false;
};
struct Queue {
    uint32_t uid = 0, gid = 0, mode = 0;
    uint64_t max_messages = 0, message_size = 0, count = 0, bytes = 0, sequence = 0;
    std::map<uint32_t, std::deque<Message>, std::greater<uint32_t>> messages;
    std::vector<std::weak_ptr<OpenFile>> files;
    std::vector<std::weak_ptr<PosixMqPollSubscription>> poll_subscriptions;
    std::vector<std::weak_ptr<Waiter>> senders, receivers;
    std::optional<Notification> notification;
    std::shared_ptr<Charge> charge;
};
} // namespace

class PosixMqNamespace {
  public:
    std::shared_ptr<PosixMqAccounting> accounting;
    PosixMqLimits limits;
    std::map<std::string, std::shared_ptr<Queue>> names;
    std::vector<std::weak_ptr<Queue>> queues;
};

namespace {
struct OpenFile {
    std::shared_ptr<Queue> queue;
    std::shared_ptr<OpenDescription> description;
    FD reader, writer;
    bool marked = false;

    explicit OpenFile(std::shared_ptr<Queue> value) : queue(std::move(value)) {
        auto pipes = timers::ready_pipe();
        reader = std::move(pipes.first);
        writer = std::move(pipes.second);
    }
    ~OpenFile() {
        if (description) {
            auto &all = registry();
            std::lock_guard<std::mutex> lock(all.mutex);
            all.entries.erase(description.get());
        }
    }
    void update_marker() noexcept {
        const bool ready = queue->count != 0;
        if (ready == marked)
            return;
        char byte = 1;
        ssize_t result;
        do {
            result = ready ? ::write(writer.get(), &byte, 1) : ::read(reader.get(), &byte, 1);
        } while (result < 0 && errno == EINTR);
        if (result == 1 || (ready && result < 0 && errno == EAGAIN))
            marked = ready;
    }
};

struct AbsoluteTime { int64_t seconds, nanoseconds; };
struct Waiter {
    bool active = true, send = false;
    uint32_t tid = 0;
    int priority = 0;
    uint64_t sequence = 0, address = 0, priority_address = 0;
    std::optional<AbsoluteTime> deadline;
    Memory *memory = nullptr;
    Scheduler *scheduler = nullptr;
    std::shared_ptr<OpenFile> file;
    Message message;
};

std::shared_ptr<OpenFile> find_file(const std::shared_ptr<OpenDescription> &description) {
    if (!description)
        return {};
    auto &all = registry();
    std::lock_guard<std::mutex> lock(all.mutex);
    auto found = all.entries.find(description.get());
    return found == all.entries.end() ? nullptr : found->second.lock();
}
std::shared_ptr<OpenFile> file_for(DescriptorTable &descriptors, uint64_t fd) {
    auto &entry = descriptors.get(uint32_t(fd));
    auto file = find_file(entry.description);
    if (!file)
        guest_error(9);
    return file;
}
void update_markers(Queue &queue) {
    queue.files.erase(std::remove_if(queue.files.begin(), queue.files.end(), [](auto &weak) {
        auto file = weak.lock();
        if (!file)
            return true;
        file->update_marker();
        return false;
    }), queue.files.end());
}
uint32_t readiness(const Queue &queue) {
    return (queue.count ? uint32_t{1 | 64} : 0) |
           (queue.count < queue.max_messages ? uint32_t{4 | 256} : 0);
}
void update_readiness(Queue &queue, bool wake) {
    update_markers(queue);
    const uint32_t ready = readiness(queue);
    queue.poll_subscriptions.erase(
        std::remove_if(queue.poll_subscriptions.begin(), queue.poll_subscriptions.end(),
                       [ready, wake](auto &weak) {
                           auto subscription = weak.lock();
                           if (!subscription)
                               return true;
                           subscription->callback(ready, wake);
                           return false;
                       }),
        queue.poll_subscriptions.end());
}
void check_buffer(Memory &memory, uint64_t address, uint64_t size, unsigned permission) {
    if (size && !memory.accessible(address, size, permission))
        guest_error(14);
}
std::string name_at(Memory &memory, uint64_t address) {
    auto name = memory.string(address);
    if (name.empty())
        guest_error(2);
    if (name.size() > 255)
        guest_error(36);
    if (name == "." || name == ".." || name.find('/') != std::string::npos)
        guest_error(13);
    return name;
}
bool in_group(const PosixMqCredentials &credentials, uint32_t gid) {
    return credentials.fsgid == gid ||
           std::find(credentials.groups.begin(), credentials.groups.end(), gid) != credentials.groups.end();
}
void check_access(const Queue &queue, const PosixMqCredentials &credentials, unsigned requested) {
    if (credentials.cap_dac_override)
        return;
    const unsigned shift = credentials.fsuid == queue.uid ? 6 : in_group(credentials, queue.gid) ? 3 : 0;
    if (((queue.mode >> shift) & requested) != requested)
        guest_error(13);
}
std::shared_ptr<Charge> charge_queue(const std::shared_ptr<PosixMqAccounting> &accounting,
                                     uint32_t uid, uint64_t bytes, uint64_t limit) {
    auto charge = std::shared_ptr<Charge>(new Charge{{}, uid, 0});
    std::lock_guard<std::mutex> lock(accounting->mutex);
    const auto found = accounting->usage.find(uid);
    const uint64_t used = found == accounting->usage.end() ? 0 : found->second;
    if (used > limit || bytes > limit - used)
        guest_error(24);
    accounting->usage[uid] = used + bytes;
    charge->accounting = accounting;
    charge->bytes = bytes;
    return charge;
}
std::optional<AbsoluteTime> read_timeout(Memory &memory, uint64_t address) {
    if (!address)
        return {};
    check_buffer(memory, address, 16, read_perm);
    AbsoluteTime time{int64_t(memory.load(address, 8)), int64_t(memory.load(address + 8, 8))};
    if (time.seconds < 0 || time.nanoseconds < 0 || time.nanoseconds >= 1000000000)
        guest_error(22);
    return time;
}
bool expired(const std::optional<AbsoluteTime> &deadline) {
    if (!deadline)
        return false;
    timespec now{};
    check_host(::clock_gettime(CLOCK_REALTIME, &now));
    return deadline->seconds < now.tv_sec ||
           (deadline->seconds == now.tv_sec && deadline->nanoseconds <= now.tv_nsec);
}
void push_message(Queue &queue, Message message) {
    const auto length = message.bytes.size();
    queue.messages[message.priority].push_back(std::move(message));
    ++queue.count;
    queue.bytes += length;
}
Message pop_message(Queue &queue) {
    auto first = queue.messages.begin();
    Message message = std::move(first->second.front());
    first->second.pop_front();
    if (first->second.empty())
        queue.messages.erase(first);
    --queue.count;
    queue.bytes -= message.bytes.size();
    return message;
}
int64_t copy_message(Memory &memory, uint64_t address, uint64_t priority_address, const Message &message) {
    // Linux consumes the message even if a later copy_to_user fails.
    try {
        if (priority_address)
            memory.store(priority_address, message.priority, 4);
        if (!message.bytes.empty())
            memory.copy_in(address, message.bytes.data(), message.bytes.size());
        return int64_t(message.bytes.size());
    } catch (const MemoryFault &) {
        return -14;
    } catch (const GuestError &error) {
        return -error.number;
    }
}
std::shared_ptr<Waiter> first_waiter(std::vector<std::weak_ptr<Waiter>> &list) {
    std::shared_ptr<Waiter> best;
    for (auto it = list.begin(); it != list.end();) {
        auto waiter = it->lock();
        if (!waiter || !waiter->active) {
            it = list.erase(it);
            continue;
        }
        auto *task = waiter->scheduler->lookup(waiter->tid);
        if (!task || task->state != Task::State::Waiting) {
            waiter->active = false;
            it = list.erase(it);
            continue;
        }
        if (expired(waiter->deadline)) {
            waiter->scheduler->wake(waiter->tid, -110);
            it = list.erase(it);
            continue;
        }
        if (!best || waiter->priority < best->priority ||
            (waiter->priority == best->priority && waiter->sequence < best->sequence))
            best = std::move(waiter);
        ++it;
    }
    return best;
}
bool refill_from_sender(Queue &queue) {
    if (auto sender = first_waiter(queue.senders)) {
        push_message(queue, std::move(sender->message));
        sender->scheduler->wake(sender->tid, 0);
        return true;
    }
    return false;
}
void notify(Queue &queue, Scheduler &scheduler, const PosixMqContext &context) {
    if (!queue.notification)
        return;
    auto notice = *queue.notification;
    queue.notification.reset();
    if (notice.method != 0 || !notice.signal || notice.after_exec)
        return;
    auto *hooks = context.signal_hooks;
    if (!hooks || !hooks->process)
        throw Failure("POSIX mq signal hooks missing on send");
    auto info = signals::Info::user(notice.signal, scheduler.current().pid, context.credentials.uid, -3);
    std::memcpy(info.payload.data() + 8, &notice.value, sizeof(notice.value));
    try {
        auto target = hooks->process(notice.owner);
        (void)signals::send_process(*hooks, target, info);
    } catch (const GuestError &) {
        // The registrant may have exited, or signal admission may have failed.
    }
}
int64_t wait_for(Scheduler &scheduler, const PosixMqContext &context, const std::shared_ptr<OpenFile> &file,
                 std::optional<AbsoluteTime> deadline, bool send, Message message,
                 Memory &memory, uint64_t address, uint64_t priority_address) {
    if (file->description->flags & nonblock)
        return -11;
    if (expired(deadline))
        return -110;
    if (!context.interrupt_pending)
        return -95;
    if (context.interrupt_pending())
        return -4;
    auto &task = scheduler.current();
    if (task.state != Task::State::Runnable)
        throw Failure("POSIX mq wait on a non-runnable task");
    auto waiter = std::make_shared<Waiter>();
    waiter->send = send;
    waiter->tid = task.tid;
    waiter->priority = scheduler.effective_priority(task.tid);
    waiter->sequence = ++file->queue->sequence;
    waiter->deadline = deadline;
    waiter->memory = &memory;
    waiter->scheduler = &scheduler;
    waiter->file = file;
    waiter->message = std::move(message);
    waiter->address = address;
    waiter->priority_address = priority_address;
    auto &list = send ? file->queue->senders : file->queue->receivers;
    list.push_back(waiter);
    scheduler.block({}, -110, [waiter]() -> std::optional<int64_t> {
        if (expired(waiter->deadline))
            return -110;
        return {};
    });
    return 0;
}
std::string status(const Queue &queue) {
    char text[160];
    const auto *notice = queue.notification ? &*queue.notification : nullptr;
    const int size = std::snprintf(text, sizeof(text), "QSIZE:%-10llu NOTIFY:%-5d SIGNO:%-5d NOTIFY_PID:%-6u\n",
                                   static_cast<unsigned long long>(queue.bytes),
                                   notice ? notice->method : 0,
                                   notice && notice->method == 0 ? notice->signal : 0,
                                   notice ? notice->owner : 0);
    return std::string(text, size > 0 ? size_t(size) : 0);
}
int open_queue(PosixMqNamespace &space, Memory &memory, DescriptorTable &descriptors,
               const PosixMqContext &context, uint64_t name_address, uint32_t flags,
               uint32_t mode, uint64_t attribute_address) {
    std::array<uint64_t, 8> attr{};
    if (attribute_address) {
        check_buffer(memory, attribute_address, mq_attribute_size, read_perm);
        memory.copy_out(attribute_address, attr.data(), sizeof(attr));
    }
    const auto name = name_at(memory, name_address);
    if ((flags & ~known_flags) || (flags & 3) == 3)
        guest_error(22);
    auto found = space.names.find(name);
    const bool creating = found == space.names.end();
    if (creating && !(flags & create))
        guest_error(2);
    if (!creating && (flags & (create | exclusive)) == (create | exclusive))
        guest_error(17);
    std::shared_ptr<Queue> queue;
    if (creating) {
        space.queues.erase(std::remove_if(space.queues.begin(), space.queues.end(),
                                         [](const auto &item) { return item.expired(); }), space.queues.end());
        if (space.queues.size() >= space.limits.queues && !context.credentials.cap_sys_resource)
            guest_error(28);
        const int64_t max_messages = attribute_address ? int64_t(attr[1]) :
            std::min(space.limits.default_messages, space.limits.messages);
        const int64_t message_size = attribute_address ? int64_t(attr[2]) :
            std::min(space.limits.default_message_size, space.limits.message_size);
        const uint64_t max_allowed = context.credentials.cap_sys_resource ? hard_messages : space.limits.messages;
        const uint64_t size_allowed = context.credentials.cap_sys_resource ? hard_message_size : space.limits.message_size;
        if (max_messages <= 0 || message_size <= 0 || uint64_t(max_messages) > max_allowed ||
            uint64_t(message_size) > size_allowed)
            guest_error(22);
        queue = std::make_shared<Queue>();
        queue->uid = context.credentials.fsuid;
        queue->gid = context.credentials.fsgid;
        queue->mode = mode & ~context.umask & 0777;
        queue->max_messages = uint64_t(max_messages);
        queue->message_size = uint64_t(message_size);
        // AArch64 Linux msg_msg and posix_msg_tree_node are 48 bytes each.
        const uint64_t bytes = queue->max_messages * (queue->message_size + 48) +
                               std::min<uint64_t>(queue->max_messages, maximum_priority) * 48;
        queue->charge = charge_queue(space.accounting, context.credentials.uid, bytes,
                                     context.message_queue_limit);
    } else {
        queue = found->second;
        const unsigned access = (flags & 3) == 0 ? 4 : (flags & 3) == 1 ? 2 : 6;
        check_access(*queue, context.credentials, access);
    }
    auto file = std::make_shared<OpenFile>(queue);
    FD native(::fcntl(file->reader.get(), F_DUPFD_CLOEXEC, 0));
    check_host(native.get());
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::File, "mqueue:" + name, queue->mode);
    node->sequence = true;
    node->read_range = [weak = std::weak_ptr<Queue>(queue)](uint64_t offset, size_t count) {
        auto value = weak.lock();
        if (!value)
            guest_error(9);
        auto text = status(*value);
        return offset >= text.size() ? std::string{} : text.substr(size_t(offset), count);
    };
    node->write_contents = [](const std::string &) -> size_t { guest_error(22); };
    const int fd = descriptors.insert(std::move(native), "", int(flags | cloexec));
    try {
        auto &entry = descriptors.get(fd);
        file->description = entry.description;
        file->description->virtual_file = std::make_shared<VirtualOpen>(std::move(node));
        file->description->scheduler_io = true;
        {
            auto &all = registry();
            std::lock_guard<std::mutex> lock(all.mutex);
            all.entries.emplace(file->description.get(), file);
        }
        entry.description = std::shared_ptr<OpenDescription>(file, file->description.get());
        queue->files.push_back(file);
        file->update_marker();
        if (creating) {
            space.queues.push_back(queue);
            space.names.emplace(name, queue);
        }
    } catch (...) {
        descriptors.close(fd);
        throw;
    }
    return fd;
}
} // namespace

std::shared_ptr<PosixMqAccounting> make_posix_mq_accounting() {
    return std::make_shared<PosixMqAccounting>();
}
std::shared_ptr<PosixMqNamespace> make_posix_mq_namespace(
    const std::shared_ptr<PosixMqAccounting> &accounting, const PosixMqLimits &limits) {
    if (!accounting || !limits.queues || !limits.messages || !limits.message_size ||
        !limits.default_messages || !limits.default_message_size || limits.messages > hard_messages ||
        limits.message_size > hard_message_size)
        guest_error(22);
    auto space = std::make_shared<PosixMqNamespace>();
    space->accounting = accounting;
    space->limits = limits;
    return space;
}

int64_t posix_mq_syscall(const std::shared_ptr<PosixMqNamespace> &space, Memory &memory,
                         DescriptorTable &descriptors, Scheduler &scheduler,
                         const PosixMqContext &context, uint64_t number, uint64_t arg0,
                         uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4) {
    if (!space)
        guest_error(38);
    switch (number) {
    case 180:
        return open_queue(*space, memory, descriptors, context, arg0, uint32_t(arg1), uint32_t(arg2), arg3);
    case 181: {
        auto name = name_at(memory, arg0);
        auto found = space->names.find(name);
        if (found == space->names.end())
            guest_error(2);
        if (context.credentials.fsuid != found->second->uid && context.credentials.fsuid != 0 &&
            !context.credentials.cap_fowner)
            guest_error(1);
        space->names.erase(found);
        return 0;
    }
    case 182: {
        auto deadline = read_timeout(memory, arg4);
        if (uint32_t(arg3) >= maximum_priority)
            guest_error(22);
        auto file = file_for(descriptors, arg0);
        if ((file->description->flags & 3) == 0)
            guest_error(9);
        auto &queue = *file->queue;
        if (arg2 > queue.message_size)
            guest_error(90);
        check_buffer(memory, arg1, arg2, read_perm);
        Message message;
        message.priority = uint32_t(arg3);
        message.bytes.resize(size_t(arg2));
        if (arg2)
            memory.copy_out(arg1, message.bytes.data(), message.bytes.size());
        if (queue.count >= queue.max_messages)
            return wait_for(scheduler, context, file, deadline, true, std::move(message), memory, 0, 0);
        if (auto receiver = first_waiter(queue.receivers)) {
            auto result = copy_message(*receiver->memory, receiver->address, receiver->priority_address, message);
            receiver->scheduler->wake(receiver->tid, result);
        } else {
            if (!queue.count && queue.notification && queue.notification->method == 0 &&
                queue.notification->signal && !queue.notification->after_exec &&
                (!context.signal_hooks || !context.signal_hooks->process))
                guest_error(95);
            const bool was_empty = queue.count == 0;
            push_message(queue, std::move(message));
            update_readiness(queue, true);
            if (was_empty)
                notify(queue, scheduler, context);
        }
        return 0;
    }
    case 183: {
        auto deadline = read_timeout(memory, arg4);
        auto file = file_for(descriptors, arg0);
        if ((file->description->flags & 3) == 1)
            guest_error(9);
        auto &queue = *file->queue;
        if (arg2 < queue.message_size)
            guest_error(90);
        if (!queue.count)
            return wait_for(scheduler, context, file, deadline, false, {}, memory, arg1, arg3);
        auto message = pop_message(queue);
        const bool refilled = refill_from_sender(queue);
        update_readiness(queue, !refilled);
        return copy_message(memory, arg1, arg3, message);
    }
    case 184: {
        std::array<uint8_t, sigevent_size> event{};
        if (arg1) {
            check_buffer(memory, arg1, event.size(), read_perm);
            memory.copy_out(arg1, event.data(), event.size());
        }
        auto file = file_for(descriptors, arg0);
        auto &queue = *file->queue;
        const uint32_t pid = scheduler.current().pid;
        if (!arg1) {
            if (queue.notification && queue.notification->owner == pid)
                queue.notification.reset();
            return 0;
        }
        Notification notice;
        notice.owner = pid;
        std::memcpy(&notice.value, event.data(), 8);
        std::memcpy(&notice.signal, event.data() + 8, 4);
        std::memcpy(&notice.method, event.data() + 12, 4);
        if (notice.method == 2)
            guest_error(95); // Linux SIGEV_THREAD needs its netlink cookie protocol.
        if (notice.method != 0 && notice.method != 1)
            guest_error(22);
        if (notice.method == 0 && (notice.signal < 0 || notice.signal > 64))
            guest_error(22);
        if (notice.method == 0 && notice.signal &&
            (!context.signal_hooks || !context.signal_hooks->process || !context.signal_hooks->wake))
            guest_error(95);
        if (queue.notification)
            guest_error(16);
        queue.notification = notice;
        return 0;
    }
    case 185: {
        std::array<uint64_t, 8> attr{};
        if (arg1) {
            check_buffer(memory, arg1, mq_attribute_size, read_perm);
            memory.copy_out(arg1, attr.data(), sizeof(attr));
            if (attr[0] & ~uint64_t(nonblock))
                guest_error(22);
        }
        auto file = file_for(descriptors, arg0);
        const auto old_flags = file->description->flags & nonblock;
        if (arg1)
            file->description->flags = (file->description->flags & ~int(nonblock)) | int(attr[0]);
        if (arg2) {
            std::array<uint64_t, 8> old{uint64_t(old_flags), file->queue->max_messages,
                                        file->queue->message_size, file->queue->count};
            memory.copy_in(arg2, old.data(), sizeof(old));
        }
        return 0;
    }
    default:
        guest_error(38);
    }
}

void posix_mq_close(const std::shared_ptr<OpenDescription> &description, uint32_t pid) {
    if (auto file = find_file(description))
        if (file->queue->notification && file->queue->notification->owner == pid)
            file->queue->notification.reset();
}
void posix_mq_process_exec(const std::shared_ptr<PosixMqNamespace> &space, uint32_t pid) {
    if (!space)
        return;
    for (auto &weak : space->queues)
        if (auto queue = weak.lock())
            if (queue->notification && queue->notification->owner == pid)
                queue->notification->after_exec = true;
}
void posix_mq_process_exit(const std::shared_ptr<PosixMqNamespace> &space, uint32_t pid) {
    if (!space)
        return;
    for (auto &weak : space->queues)
        if (auto queue = weak.lock())
            if (queue->notification && queue->notification->owner == pid)
                queue->notification.reset();
}
bool is_posix_mq_file(const std::shared_ptr<OpenDescription> &description) {
    return bool(find_file(description));
}
std::optional<uint32_t> posix_mq_poll(const std::shared_ptr<OpenDescription> &description,
                                      uint32_t requested) {
    auto file = find_file(description);
    if (!file)
        return {};
    return readiness(*file->queue) & requested;
}
std::shared_ptr<PosixMqPollSubscription> posix_mq_subscribe(
    const std::shared_ptr<OpenDescription> &description,
    std::function<void(uint32_t, bool)> callback) {
    auto file = find_file(description);
    if (!file)
        return {};
    auto subscription = std::make_shared<PosixMqPollSubscription>(std::move(callback));
    file->queue->poll_subscriptions.push_back(subscription);
    return subscription;
}
} // namespace operit::arm64::ipc
