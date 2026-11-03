#include "linux/events/epoll.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "linux/io_uring/io_uring.h"
#include "linux/ipc/posix_mq.h"
#ifndef __APPLE__
#include "linux/timers/ready_pipe.h"
#endif
#include <algorithm>
#include <cerrno>
#include <chrono>
#include <fcntl.h>
#include <functional>
#include <mutex>
#include <poll.h>
#ifdef __APPLE__
#include <sys/event.h>
#include <time.h>
#else
#include <sys/epoll.h>
#endif
namespace operit::arm64 {
namespace {
constexpr uint32_t input = 1;
constexpr uint32_t priority = 2;
constexpr uint32_t output = 4;
constexpr uint32_t error = 8;
constexpr uint32_t hangup = 16;
constexpr uint32_t read_normal = 64;
constexpr uint32_t write_normal = 256;
constexpr uint32_t read_hangup = 8192;
constexpr uint32_t one_shot = 1u << 30;
constexpr uint32_t edge_triggered = 1u << 31;
constexpr unsigned maximum_links = 4;
constexpr uint32_t mqueue_events = input | output | read_normal | write_normal;
struct Subscriptions {
    std::mutex mutex;
    std::map<uintptr_t, std::set<std::pair<EventPoll *, uint64_t>>> descriptions;
};
Subscriptions &subscriptions() {
    static Subscriptions instance;
    return instance;
}
} // namespace
EventPoll::EventPoll() {
#ifdef __APPLE__
    queue_ = FD(::kqueue());
#else
    queue_ = FD(::epoll_create1(EPOLL_CLOEXEC));
#endif
    check_host(queue_.get());
    check_host(::fcntl(queue_.get(), F_SETFD, FD_CLOEXEC));
#ifdef __APPLE__
    struct kevent event;
    EV_SET(&event, 0, EVFILT_USER, EV_ADD | EV_DISABLE, 0, 0, nullptr);
    check_host(::kevent(queue_.get(), &event, 1, nullptr, 0, nullptr));
#endif
}
EventPoll::~EventPoll() {
    // Incoming registrations must disappear with the final guest OFD, even
    // though their private native duplicates still refer to this queue.
    while (!parents_.empty()) {
        const auto parent = *parents_.begin();
        try {
            parent.first->remove(parent.second);
        } catch (...) {
            parent.first->erase(parent.second);
        }
    }
    while (!watches_.empty()) {
        const uint64_t token = watches_.begin()->first;
        try {
            remove(token);
        } catch (...) {
            erase(token);
        }
    }
}
void EventPoll::description_closed(const OpenDescription *description) noexcept {
    std::set<std::pair<EventPoll *, uint64_t>> closing;
    auto &registry = subscriptions();
    {
        std::lock_guard<std::mutex> guard(registry.mutex);
        auto found = registry.descriptions.find(reinterpret_cast<uintptr_t>(description));
        if (found == registry.descriptions.end())
            return;
        closing.swap(found->second);
        registry.descriptions.erase(found);
    }
    for (const auto &watch : closing) {
        try {
            watch.first->remove(watch.second);
        } catch (...) {
            watch.first->erase(watch.second);
        }
    }
}
void EventPoll::validate_nested(EventPoll &child) const {
    if (&child == this)
        guest_error(22);
    std::map<const EventPoll *, unsigned> descendants;
    std::function<unsigned(const EventPoll *)> below = [&](const EventPoll *node) {
        if (node == this)
            guest_error(40);
        auto known = descendants.find(node);
        if (known != descendants.end())
            return known->second;
        unsigned depth = 0;
        for (const auto &entry : node->watches_) {
            const auto &watch = entry.second;
            if (watch.nested && !watch.owner.expired())
                depth = std::max(depth, 1 + below(watch.nested));
        }
        descendants.emplace(node, depth);
        return depth;
    };
    std::map<const EventPoll *, unsigned> ancestors;
    std::function<unsigned(const EventPoll *)> above = [&](const EventPoll *node) {
        auto known = ancestors.find(node);
        if (known != ancestors.end())
            return known->second;
        unsigned depth = 0;
        for (const auto &parent : node->parents_) {
            auto watch = parent.first->watches_.find(parent.second);
            if (watch != parent.first->watches_.end() && !watch->second.owner.expired())
                depth = std::max(depth, 1 + above(parent.first));
        }
        ancestors.emplace(node, depth);
        return depth;
    };
    if (above(this) + 1 + below(&child) > maximum_links)
        guest_error(40);
}
void EventPoll::refresh_mqueue(uint64_t token, Watch &watch) {
    if (!watch.armed)
        return;
    const uint32_t ready = watch.mq_ready & watch.events & mqueue_events;
    auto pending = pending_.find(token);
    if (pending != pending_.end()) {
        if (ready)
            pending->second = ready;
        else
            pending_.erase(pending);
    } else if (ready && !(watch.events & edge_triggered))
        pending_[token] = ready;
}
void EventPoll::mqueue_changed(uint64_t token, uint32_t ready, bool wake) {
    auto found = watches_.find(token);
    if (found == watches_.end())
        return;
    auto &watch = found->second;
    watch.mq_ready = ready;
    if (!wake || !watch.armed || !(watch.events & mqueue_events))
        return;
    // Linux mqueue wakes poll waiters on each enqueue/dequeue, even when the
    // readiness mask remains unchanged. The current mask is checked on scan.
    pending_[token] = watch.events & mqueue_events;
#ifdef __APPLE__
    ++revision_;
    marker(true, true);
    notify_parents();
#else
    mq_marker(true);
#endif
}
void EventPoll::collect_queues(std::set<EventPoll *> &queues) {
    if (!queues.insert(this).second)
        return;
    for (auto &entry : watches_) {
        auto &watch = entry.second;
        if (watch.nested && watch.armed && (watch.events & input))
            watch.nested->collect_queues(queues);
    }
}
#ifdef __APPLE__
void EventPoll::change(Watch &watch, uint64_t token) {
    if (watch.nested || watch.mqueue)
        return;
    const int access = ::fcntl(watch.host.get(), F_GETFL);
    check_host(access);
    for (unsigned index = 0; index < 3; ++index) {
        const int16_t filter = index == 0 ? EVFILT_READ : index == 1 ? EVFILT_WRITE : EVFILT_EXCEPT;
        // Linux always reports ERR/HUP, including watches with no requested
        // readiness bits. Monitor both supported directions for those events.
        const bool interested = index == 0   ? (access & O_ACCMODE) != O_WRONLY
                                : index == 1 ? (access & O_ACCMODE) != O_RDONLY
                                             : bool(watch.events & priority);
        const uint16_t flags =
            interested ? EV_ADD | EV_CLEAR | (watch.armed ? EV_ENABLE : EV_DISABLE) : EV_DELETE;
        struct kevent event;
        EV_SET(&event, watch.host.get(), filter, flags, index == 2 ? NOTE_OOB : 0, 0,
               reinterpret_cast<void *>(uintptr_t(token)));
        const int result = ::kevent(queue_.get(), &event, 1, nullptr, 0, nullptr);
        if (result < 0 && (flags & EV_DELETE) && errno == ENOENT)
            continue;
        check_host(result);
    }
}
void EventPoll::marker(bool enabled, bool notify) {
    if (enabled == marker_enabled_ && !(enabled && notify))
        return;
    struct kevent event;
    EV_SET(&event, 0, EVFILT_USER, enabled ? EV_ENABLE : EV_DISABLE, enabled ? NOTE_TRIGGER : 0, 0, nullptr);
    check_host(::kevent(queue_.get(), &event, 1, nullptr, 0, nullptr));
    marker_enabled_ = enabled;
}
void EventPoll::notify_parents() {
    for (const auto &parent : parents_) {
        auto found = parent.first->watches_.find(parent.second);
        if (found == parent.first->watches_.end())
            continue;
        auto &watch = found->second;
        if (!watch.armed || watch.owner.expired() || !(watch.events & input))
            continue;
        parent.first->pending_[parent.second] = input;
        watch.child_ready = true;
        watch.child_revision = revision_;
        ++parent.first->revision_;
        parent.first->marker(true, true);
        parent.first->notify_parents();
    }
}
uint32_t EventPoll::ready_mask(const Watch &watch, uint32_t events) const {
    if (watch.nested)
        return watch.nested->pending_.empty() ? 0 : events & input;
    if (watch.mqueue)
        return events & ipc::posix_mq_poll(watch.owner.lock(), watch.events & mqueue_events).value_or(0);
    short requested = 0;
    if (watch.events & (input | read_hangup))
        requested |= POLLIN;
    if (watch.events & output)
        requested |= POLLOUT;
    if (watch.events & priority)
        requested |= POLLPRI;
    pollfd descriptor{watch.host.get(), requested, 0};
    check_host(::poll(&descriptor, 1, 0));
    uint32_t ready = events & read_hangup;
    if (descriptor.revents & POLLIN)
        ready |= input;
    if (descriptor.revents & POLLOUT)
        ready |= output;
    if (descriptor.revents & POLLPRI)
        ready |= priority;
    if (descriptor.revents & POLLERR)
        ready |= error;
    if (descriptor.revents & POLLHUP)
        ready |= hangup;
    return events & ready;
}
void EventPoll::refresh(std::set<EventPoll *> &refreshed) {
    if (!refreshed.insert(this).second)
        return;
    prune();
    for (auto &entry : watches_) {
        if (auto owner = entry.second.owner.lock(); owner && owner->io_uring)
            io_uring::refresh_readiness(owner->io_uring);
        if (entry.second.nested && entry.second.armed && (entry.second.events & input))
            entry.second.nested->refresh(refreshed);
    }

    // Children finish before this thread's scratch buffer is reused. EV_CLEAR
    // lets new native events wake an ET parent while another LT child stays
    // ready; the user filter keeps cached events pollable.
    marker(false);
    thread_local std::vector<struct kevent> events;
    events.resize(std::max<size_t>(1, watches_.size() * 3));
    struct timespec immediate{};
    const int count = ::kevent(queue_.get(), nullptr, 0, events.data(), int(events.size()), &immediate);
    check_host(count);
    bool notified = false;
    for (int j = 0; j < count; ++j) {
        const uint64_t token = reinterpret_cast<uintptr_t>(events[j].udata);
        auto found = watches_.find(token);
        if (found == watches_.end() || !found->second.armed || found->second.nested || found->second.mqueue)
            continue;
        auto &watch = found->second;
        uint32_t mask = events[j].filter == EVFILT_READ    ? (watch.events & input)
                        : events[j].filter == EVFILT_WRITE ? (watch.events & output)
                                                           : (watch.events & priority);
        if (events[j].flags & EV_ERROR)
            mask |= error;
        if (events[j].flags & EV_EOF) {
            pollfd descriptor{watch.host.get(), 0, 0};
            check_host(::poll(&descriptor, 1, 0));
            if (descriptor.revents & POLLHUP)
                mask |= hangup;
            if (descriptor.revents & POLLERR)
                mask |= error;
            if (events[j].filter == EVFILT_READ)
                mask |= watch.events & read_hangup;
        }
        if (mask) {
            pending_[token] |= mask;
            notified = true;
        }
    }
    for (auto &[token, watch] : watches_)
        if (watch.mqueue)
            refresh_mqueue(token, watch);
    for (auto &[token, watch] : watches_) {
        if (!watch.nested || !watch.armed || !(watch.events & input))
            continue;
        const bool ready = !watch.nested->pending_.empty();
        if (ready && (!(watch.events & edge_triggered) || !watch.child_ready ||
                      watch.child_revision != watch.nested->revision_)) {
            pending_[token] |= input;
            if (!watch.child_ready || watch.child_revision != watch.nested->revision_)
                notified = true;
        }
        watch.child_ready = ready;
        watch.child_revision = watch.nested->revision_;
    }
    for (auto it = pending_.begin(); it != pending_.end();) {
        auto found = watches_.find(it->first);
        const uint32_t ready =
            found == watches_.end() || !found->second.armed ? 0 : ready_mask(found->second, it->second);
        if (ready) {
            it->second = ready;
            ++it;
        } else
            it = pending_.erase(it);
    }
    if (notified)
        ++revision_;
    marker(!pending_.empty());
}
void EventPoll::await_events(int timeout) {
    std::set<EventPoll *> queues;
    collect_queues(queues);
    std::vector<pollfd> descriptors;
    descriptors.reserve(queues.size());
    try {
        for (auto *queue : queues) {
            // A cached child event already consumed by an ET parent is not
            // a new wakeup. Wait only for actual native notifications.
            queue->marker(false);
            descriptors.push_back({queue->native_handle(), POLLIN, 0});
        }
        check_host(::poll(descriptors.data(), descriptors.size(), timeout));
    } catch (...) {
        for (auto *queue : queues) {
            try {
                queue->marker(!queue->pending_.empty());
            } catch (...) {
            }
        }
        throw;
    }
    for (auto *queue : queues)
        queue->marker(!queue->pending_.empty());
}
#else
void EventPoll::ensure_mq_marker() {
    if (mq_reader_.get() >= 0)
        return;
    // Publish cached MQ events into the native epoll, including to parents.
    auto pipe = timers::ready_pipe();
    struct epoll_event event{};
    event.events = EPOLLIN;
    event.data.u64 = 0;
    check_host(::epoll_ctl(queue_.get(), EPOLL_CTL_ADD, pipe.first.get(), &event));
    mq_reader_ = std::move(pipe.first);
    mq_writer_ = std::move(pipe.second);
}
void EventPoll::mq_marker(bool notify) {
    if (mq_reader_.get() < 0)
        return;
    const bool enabled = !pending_.empty();
    if (mq_marker_enabled_ && (!enabled || notify)) {
        char bytes[128];
        ssize_t count;
        do {
            count = ::read(mq_reader_.get(), bytes, sizeof(bytes));
        } while (count > 0 || (count < 0 && errno == EINTR));
        if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK)
            check_host(int(count));
    }
    if (enabled && (!mq_marker_enabled_ || notify)) {
        const char byte = 1;
        ssize_t count;
        do {
            count = ::write(mq_writer_.get(), &byte, 1);
        } while (count < 0 && errno == EINTR);
        if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK)
            check_host(int(count));
    }
    mq_marker_enabled_ = enabled;
}
void EventPoll::refresh(std::set<EventPoll *> &refreshed) {
    if (!refreshed.insert(this).second)
        return;
    prune();
    for (auto &[token, watch] : watches_) {
        if (auto owner = watch.owner.lock(); owner && owner->io_uring)
            io_uring::refresh_readiness(owner->io_uring);
        if (watch.nested && watch.armed && (watch.events & input))
            watch.nested->refresh(refreshed);
        if (watch.mqueue)
            refresh_mqueue(token, watch);
    }
    mq_marker();
}
#endif
void EventPoll::erase(uint64_t token) noexcept {
    auto found = watches_.find(token);
    if (found == watches_.end())
        return;
    if (found->second.nested)
        found->second.nested->parents_.erase({this, token});
    {
        auto &registry = subscriptions();
        std::lock_guard<std::mutex> guard(registry.mutex);
        auto target = registry.descriptions.find(found->second.key.second);
        if (target != registry.descriptions.end()) {
            target->second.erase({this, token});
            if (target->second.empty())
                registry.descriptions.erase(target);
        }
    }
    keys_.erase(found->second.key);
    pending_.erase(token);
    watches_.erase(found);
}
void EventPoll::remove(uint64_t token) {
    auto found = watches_.find(token);
    if (found == watches_.end())
        return;
#ifndef __APPLE__
    if (!found->second.mqueue && ::epoll_ctl(queue_.get(), EPOLL_CTL_DEL, found->second.host.get(), nullptr) < 0 &&
        errno != ENOENT && errno != EBADF)
        guest_error(linux_errno(errno));
#endif
    // The weak owner deliberately does not retain a guest open description.
    erase(token);
#ifdef __APPLE__
    marker(!pending_.empty());
#else
    mq_marker();
#endif
}
void EventPoll::prune() {
    for (auto it = watches_.begin(); it != watches_.end();) {
        const uint64_t token = it->first;
        const bool expired = it->second.owner.expired();
        ++it;
        if (expired)
            remove(token);
    }
}
void EventPoll::control(int operation, int guest_fd, int host_fd,
                        const std::shared_ptr<OpenDescription> &owner, uint32_t events, uint64_t data) {
    if (operation < 1 || operation > 3)
        guest_error(22);
    const bool mqueue = ipc::is_posix_mq_file(owner);
    if (operation != 2 && (events & ~uint32_t{input | priority | output | error | hangup | read_hangup |
                                              one_shot | edge_triggered |
                                              (mqueue ? read_normal | write_normal : 0)}))
        guest_error(22);
    auto *nested = owner->poll.get();
    if (nested == this)
        guest_error(22);
    prune();
    const Key key{guest_fd, reinterpret_cast<uintptr_t>(owner.get())};
    auto existing = keys_.find(key);
    if (operation == 1 && existing != keys_.end())
        guest_error(17);
    if (operation != 1 && existing == keys_.end())
        guest_error(2);
    if (operation == 2) {
        remove(existing->second);
        return;
    }
    if (operation == 1 && nested)
        validate_nested(*nested);
    const uint64_t token = operation == 1 ? next_token_++ : existing->second;
    if (operation == 1) {
        FD duplicate(::fcntl(host_fd, F_DUPFD_CLOEXEC, 0));
        check_host(duplicate.get());
        watches_.emplace(token, Watch{std::move(duplicate), owner, key, nested, events, data});
        try {
            watches_.at(token).mqueue = mqueue;
#ifndef __APPLE__
            if (watches_.at(token).mqueue)
                ensure_mq_marker();
#endif
            keys_.emplace(key, token);
            if (nested)
                nested->parents_.emplace(this, token);
            if (mqueue) {
                watches_.at(token).mq_subscription = ipc::posix_mq_subscribe(
                    owner, [this, token](uint32_t ready, bool wake) { mqueue_changed(token, ready, wake); });
                if (!watches_.at(token).mq_subscription)
                    guest_error(9);
            }
            auto &registry = subscriptions();
            std::lock_guard<std::mutex> guard(registry.mutex);
            registry.descriptions[key.second].emplace(this, token);
        } catch (...) {
            erase(token);
            throw;
        }
    }
    auto &watch = watches_.at(token);
    const uint32_t old_events = watch.events;
    const uint64_t old_data = watch.data;
    const bool old_armed = watch.armed;
    watch.events = events;
    watch.data = data;
    watch.armed = true;
    try {
#ifdef __APPLE__
        change(watch, token);
#else
        if (!watch.mqueue) {
            struct epoll_event event{};
            event.events = events;
            event.data.u64 = token;
            check_host(::epoll_ctl(queue_.get(), operation, watch.host.get(), &event));
        }
#endif
    } catch (...) {
        if (operation == 1)
            erase(token);
        else {
            watch.events = old_events;
            watch.data = old_data;
            watch.armed = old_armed;
#ifdef __APPLE__
            try {
                change(watch, token);
            } catch (...) {
            }
#endif
        }
        throw;
    }
    pending_.erase(token);
    if (watch.mqueue) {
        watch.mq_ready = ipc::posix_mq_poll(owner, mqueue_events).value_or(0);
        const uint32_t ready = watch.mq_ready & watch.events & mqueue_events;
        if (ready)
            pending_[token] = ready;
    }
#ifdef __APPLE__
    watch.child_ready = false;
    watch.child_revision = 0;
    if (watch.nested && watch.armed && (watch.events & input)) {
        std::set<EventPoll *> refreshed;
        watch.nested->refresh(refreshed);
        if (!watch.nested->pending_.empty()) {
            pending_[token] = input;
            watch.child_ready = true;
            watch.child_revision = watch.nested->revision_;
        }
    }
    if (pending_.find(token) != pending_.end()) {
        ++revision_;
        marker(true, true);
        notify_parents();
    } else
        marker(!pending_.empty());
#else
    mq_marker(watch.mqueue && pending_.find(token) != pending_.end());
#endif
}
short EventPoll::poll_events(short requested) {
    std::set<EventPoll *> refreshed;
    refresh(refreshed);
#ifdef __APPLE__
    return pending_.empty() ? 0 : requested & POLLIN;
#else
    pollfd descriptor{queue_.get(), requested, 0};
    check_host(::poll(&descriptor, 1, 0));
    return descriptor.revents;
#endif
}
std::vector<PollReady> EventPoll::wait(int maximum, int timeout) {
    if (maximum <= 0)
        guest_error(22);
    prune();
    std::vector<PollReady> result;
#ifdef __APPLE__
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(std::max(0, timeout));
    for (;;) {
        std::set<EventPoll *> refreshed;
        refresh(refreshed);
        if (!pending_.empty() || !timeout)
            break;
        int remaining = -1;
        if (timeout > 0) {
            const auto nanos = deadline - std::chrono::steady_clock::now();
            if (nanos <= std::chrono::steady_clock::duration::zero())
                break;
            remaining = int(std::chrono::duration_cast<std::chrono::milliseconds>(nanos).count());
            remaining = std::max(1, remaining);
        }
        await_events(remaining);
    }
    auto next = pending_.upper_bound(last_ready_);
    size_t remaining = pending_.size();
    while (remaining-- && !pending_.empty() && result.size() < size_t(maximum)) {
        if (next == pending_.end())
            next = pending_.begin();
        const auto item = *next++;
        auto &watch = watches_.at(item.first);
        last_ready_ = item.first;
        result.push_back({item.second, watch.data});
        if (watch.events & (edge_triggered | one_shot))
            pending_.erase(item.first);
        if (watch.events & one_shot) {
            watch.armed = false;
            change(watch, item.first);
        }
    }
    marker(!pending_.empty());
#else
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(std::max(0, timeout));
    std::vector<struct epoll_event> events(maximum);
    for (;;) {
        std::set<EventPoll *> refreshed;
        refresh(refreshed);
        int remaining = timeout;
        if (timeout > 0) {
            const auto nanos = deadline - std::chrono::steady_clock::now();
            remaining = nanos <= std::chrono::steady_clock::duration::zero()
                            ? 0
                            : std::max(1, int(std::chrono::duration_cast<std::chrono::milliseconds>(nanos).count()));
        }
        const int count = ::epoll_wait(queue_.get(), events.data(), maximum, remaining);
        check_host(count);
        bool mqueue_ready = false;
        for (int j = 0; j < count; ++j) {
            const uint64_t token = events[j].data.u64;
            if (!token) {
                mqueue_ready = true;
                continue;
            }
            auto found = watches_.find(token);
            if (found == watches_.end())
                continue;
            result.push_back({events[j].events, found->second.data});
            if (found->second.events & one_shot)
                found->second.armed = false;
        }
        if (mqueue_ready) {
            auto next = pending_.upper_bound(last_ready_);
            size_t pending_count = pending_.size();
            while (pending_count-- && !pending_.empty() && result.size() < size_t(maximum)) {
                if (next == pending_.end())
                    next = pending_.begin();
                const auto item = *next++;
                auto &watch = watches_.at(item.first);
                last_ready_ = item.first;
                result.push_back({item.second, watch.data});
                if (watch.events & (edge_triggered | one_shot))
                    pending_.erase(item.first);
                if (watch.events & one_shot)
                    watch.armed = false;
            }
            mq_marker();
        }
        if (!result.empty() || timeout == 0 ||
            (timeout > 0 && std::chrono::steady_clock::now() >= deadline))
            break;
    }
#endif
    return result;
}
} // namespace operit::arm64
