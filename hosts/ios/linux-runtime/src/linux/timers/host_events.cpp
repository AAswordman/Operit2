#include "linux/timers/host_events.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/timers/ready_pipe.h"
#include <cerrno>
#include <climits>
#include <ctime>
#if defined(__APPLE__)
#include <mach/host_notify.h>
#include <mach/mach.h>
#include <sys/event.h>
#else
#include <poll.h>
#include <sys/timerfd.h>
#endif

namespace operit::arm64::timers {
namespace {
void drain(int fd) {
    char bytes[64];
    while (true) {
        ssize_t count = ::read(fd, bytes, sizeof(bytes));
        if (count > 0 || (count < 0 && errno == EINTR))
            continue;
        break;
    }
}
#if defined(__APPLE__)
class MachRight {
    mach_port_t port_ = MACH_PORT_NULL;
    mach_port_right_t type_ = MACH_PORT_RIGHT_RECEIVE;

  public:
    ~MachRight() {
        if (port_ != MACH_PORT_NULL)
            ::mach_port_mod_refs(::mach_task_self(), port_, type_, -1);
    }
    mach_port_t get() const {
        return port_;
    }
    bool allocate(mach_port_right_t type) {
        type_ = type;
        return ::mach_port_allocate(::mach_task_self(), type, &port_) == KERN_SUCCESS;
    }
};
#endif
} // namespace
struct HostEvents::Impl {
    FD reader, writer;
#if defined(__APPLE__)
    FD queue;
    MachRight port, port_set;
    mach_port_t host = MACH_PORT_NULL;
    bool calendar_change = false, calendar_set = false;
#else
    FD clock;
    bool calendar_set = false;
#endif

    Impl() {
        auto pipe = ready_pipe();
        reader = std::move(pipe.first);
        writer = std::move(pipe.second);
#if defined(__APPLE__)
        queue = FD(::kqueue());
        check_host(queue.get());
        check_host(::fcntl(queue.get(), F_SETFD, FD_CLOEXEC));
        struct kevent event;
        EV_SET(&event, reader.get(), EVFILT_READ, EV_ADD | EV_CLEAR, 0, 0, nullptr);
        check_host(::kevent(queue.get(), &event, 1, nullptr, 0, nullptr));
        if (!port.allocate(MACH_PORT_RIGHT_RECEIVE) || !port_set.allocate(MACH_PORT_RIGHT_PORT_SET))
            return;
        if (::mach_port_move_member(::mach_task_self(), port.get(), port_set.get()) != KERN_SUCCESS)
            return;
        EV_SET(&event, port_set.get(), EVFILT_MACHPORT, EV_ADD | EV_CLEAR, 0, 0, nullptr);
        if (::kevent(queue.get(), &event, 1, nullptr, 0, nullptr) < 0)
            return;
        host = ::mach_host_self();
        calendar_change = request(HOST_NOTIFY_CALENDAR_CHANGE);
        calendar_set = request(HOST_NOTIFY_CALENDAR_SET);
#else
        clock = FD(::timerfd_create(CLOCK_REALTIME, TFD_NONBLOCK | TFD_CLOEXEC));
        calendar_set = clock.get() >= 0 && arm_clock();
#endif
    }
    ~Impl() {
#if defined(__APPLE__)
        if (host != MACH_PORT_NULL)
            ::mach_port_deallocate(::mach_task_self(), host);
#endif
    }
#if defined(__APPLE__)
    bool request(int flavor) {
        return ::host_request_notification(host, flavor, port.get()) == KERN_SUCCESS;
    }
#else
    bool arm_clock() {
        itimerspec value{};
        value.it_value.tv_sec = INT64_MAX / 1000000000;
        value.it_value.tv_nsec = INT64_MAX % 1000000000;
        return ::timerfd_settime(clock.get(), TFD_TIMER_ABSTIME | TFD_TIMER_CANCEL_ON_SET, &value, nullptr) ==
               0;
    }
#endif
};
HostEvents::HostEvents() : impl_(std::make_unique<Impl>()) {}
HostEvents::~HostEvents() = default;
void HostEvents::wake() noexcept {
    const char byte = 1;
    while (::write(impl_->writer.get(), &byte, 1) < 0 && errno == EINTR) {
    }
}
void HostEvents::wait(std::optional<int64_t> nanoseconds) {
    timespec timeout{};
    if (nanoseconds) {
        timeout.tv_sec = *nanoseconds / 1000000000;
        timeout.tv_nsec = *nanoseconds % 1000000000;
    }
#if defined(__APPLE__)
    struct kevent events[2];
    int count = ::kevent(impl_->queue.get(), nullptr, 0, events, 2, nanoseconds ? &timeout : nullptr);
    if (count < 0 && errno != EINTR)
        guest_error(linux_errno(errno));
    for (int i = 0; i < count; ++i)
        if (events[i].flags & EV_ERROR)
            guest_error(linux_errno(int(events[i].data)));
#else
    pollfd events[] = {{impl_->reader.get(), POLLIN, 0}, {impl_->clock.get(), POLLIN, 0}};
    int count = ::ppoll(events, 2, nanoseconds ? &timeout : nullptr, nullptr);
    if (count < 0 && errno != EINTR)
        guest_error(linux_errno(errno));
    if (count > 0)
        for (const auto &event : events)
            if (event.revents & (POLLERR | POLLNVAL)) {
                // Linux marks a cancelled timerfd readable, without POLLERR.
                guest_error(5);
            }
#endif
    drain(impl_->reader.get());
}
bool HostEvents::clock_set() {
    bool changed = false;
#if defined(__APPLE__)
    if (!impl_->calendar_set && !impl_->calendar_change)
        return false;
    struct Message {
        mach_msg_header_t header;
        char contents[256];
    } message{};
    while (true) {
        kern_return_t result = ::mach_msg(&message.header, MACH_RCV_MSG | MACH_RCV_TIMEOUT, 0,
                                          sizeof(message), impl_->port_set.get(), 0, MACH_PORT_NULL);
        if (result == MACH_RCV_TIMED_OUT)
            break;
        if (result == MACH_RCV_INTERRUPTED)
            continue;
        if (result != KERN_SUCCESS)
            guest_error(5);
        if (message.header.msgh_id == HOST_CALENDAR_SET_REPLYID) {
            changed = true;
            impl_->calendar_set = impl_->request(HOST_NOTIFY_CALENDAR_SET);
        } else if (message.header.msgh_id == HOST_CALENDAR_CHANGED_REPLYID)
            impl_->calendar_change = impl_->request(HOST_NOTIFY_CALENDAR_CHANGE);
        ::mach_msg_destroy(&message.header);
    }
#else
    if (!impl_->calendar_set)
        return false;
    uint64_t expirations;
    while (true) {
        ssize_t count = ::read(impl_->clock.get(), &expirations, sizeof(expirations));
        if (count < 0 && errno == EINTR)
            continue;
        if (count < 0 && errno == EAGAIN)
            break;
        if (count < 0 && errno == ECANCELED)
            changed = true;
        else if (count != ssize_t(sizeof(expirations)))
            guest_error(5);
        impl_->calendar_set = impl_->arm_clock();
        if (!impl_->calendar_set)
            break;
    }
#endif
    return changed;
}
bool HostEvents::can_cancel_on_set() const {
    return impl_->calendar_set;
}
} // namespace operit::arm64::timers
