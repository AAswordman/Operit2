#include "linux.h"

struct mq_attr {
    i64 flags, max_messages, message_size, current_messages;
    i64 reserved[4];
};
struct pollfd {
    int fd;
    unsigned short events, revents;
};
struct epoll_event {
    unsigned events, padding;
    u64 data;
};
struct timespec {
    i64 seconds, nanoseconds;
};
struct sigevent {
    u64 value;
    int signal, method;
    char padding[48];
};

static i64 open_queue(const char *name, int flags, struct mq_attr *attributes) {
    return call6(180, (u64)name, flags, 0600, (u64)attributes, 0, 0);
}
static i64 send(i64 fd, const char *message, u64 size, unsigned priority) {
    return call6(182, fd, (u64)message, size, priority, 0, 0);
}
static i64 receive(i64 fd, char *message, unsigned *priority) {
    return call6(183, fd, (u64)message, 8, (u64)priority, 0, 0);
}
static unsigned poll_queue(i64 fd) {
    struct pollfd item = {(int)fd, 1 | 4, 0};
    struct timespec timeout = {0, 0};
    CHECK(call6(73, (u64)&item, 1, (u64)&timeout, 0, 0, 0) == 1, 80);
    return item.revents;
}
static i64 epoll_control(i64 epoll, int operation, i64 fd, struct epoll_event *event) {
    return call6(21, epoll, operation, fd, (u64)event, 0, 0);
}
static i64 epoll_wait_one(i64 epoll, struct epoll_event *event, int timeout) {
    return call6(22, epoll, (u64)event, 1, timeout, 0, 0);
}

static void exercise_epoll(i64 fd) {
    const unsigned input = 1, output = 4, oneshot = 1U << 30, edge = 1U << 31;
    struct epoll_event event = {input | output, 0, 0x1234};
    struct epoll_event got = {0, 0, 0};
    i64 inner = CALL(20, 0, 0, 0);
    i64 outer = CALL(20, 0, 0, 0);
    CHECK(inner >= 0 && outer >= 0, 45);
    CHECK(epoll_control(inner, 1, fd, &event) == 0, 46);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == output && got.data == 0x1234, 47);

    event.events = input | output | edge;
    CHECK(epoll_control(inner, 3, fd, &event) == 0, 48);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == output && got.data == 0x1234, 49);
    CHECK(epoll_wait_one(inner, &got, 0) == 0, 68);
    event.events = input | edge;
    event.data = 0x5678;
    CHECK(epoll_control(outer, 1, inner, &event) == 0, 50);
    CHECK(epoll_wait_one(outer, &got, 0) == 0, 51);

    CHECK(send(fd, "a", 1, 0) == 0, 52);
    CHECK(epoll_wait_one(outer, &got, 0) == 1 && got.events == input && got.data == 0x5678, 69);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == input && got.data == 0x1234, 70);
    CHECK(epoll_wait_one(outer, &got, 0) == 0, 71);
    event.events = output | edge;
    event.data = 0x1234;
    CHECK(epoll_control(inner, 3, fd, &event) == 0, 53);
    CHECK(epoll_wait_one(inner, &got, 0) == 0 && epoll_wait_one(outer, &got, 0) == 0, 55);
    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 56);
    if (!child) {
        char message[8];
        unsigned priority;
        CHECK(receive(fd, message, &priority) == 1 && message[0] == 'a', 57);
        fail(0);
    }
    CHECK(epoll_wait_one(outer, &got, 1000) == 1 && got.events == input && got.data == 0x5678, 58);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == output && got.data == 0x1234, 54);
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 59);

    event.events = input | edge;
    event.data = 0x1234;
    CHECK(epoll_control(inner, 3, fd, &event) == 0, 72);
    CHECK(send(fd, "b", 1, 0) == 0, 73);
    CHECK(epoll_wait_one(outer, &got, 0) == 1 && got.events == input && got.data == 0x5678, 74);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == input && got.data == 0x1234, 75);
    child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 76);
    if (!child) {
        char message[8];
        unsigned priority;
        CHECK(receive(fd, message, &priority) == 1 && message[0] == 'b', 77);
        CHECK(send(fd, "c", 1, 0) == 0, 78);
        fail(0);
    }
    status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 79);
    CHECK(epoll_wait_one(outer, &got, 0) == 1 && got.events == input && got.data == 0x5678, 81);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == input && got.data == 0x1234, 82);
    CHECK(epoll_wait_one(inner, &got, 0) == 0 && epoll_wait_one(outer, &got, 0) == 0, 83);
    char message[8];
    unsigned priority;
    CHECK(receive(fd, message, &priority) == 1 && message[0] == 'c', 84);

    event.events = input | edge | oneshot;
    event.data = 0x9abc;
    CHECK(epoll_control(inner, 3, fd, &event) == 0, 60);
    CHECK(epoll_wait_one(inner, &got, 0) == 0 && send(fd, "d", 1, 0) == 0, 61);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == input && got.data == 0x9abc, 62);
    CHECK(epoll_wait_one(inner, &got, 0) == 0, 63);
    CHECK(receive(fd, message, &priority) == 1 && message[0] == 'd' && send(fd, "e", 1, 0) == 0, 85);
    CHECK(epoll_wait_one(inner, &got, 0) == 0, 86);
    CHECK(epoll_control(inner, 3, fd, &event) == 0, 64);
    CHECK(epoll_wait_one(inner, &got, 0) == 1 && got.events == input && got.data == 0x9abc, 65);
    CHECK(receive(fd, message, &priority) == 1 && message[0] == 'e', 66);
    CHECK(CALL(57, outer, 0, 0) == 0 && CALL(57, inner, 0, 0) == 0, 67);
}

int guest_main(u64 *stack) {
    const char name[] = "runtime-posix-mq-lifecycle";
    struct sigevent notice = {0, 0, 1, {0}}; // SIGEV_NONE
    if (stack[0] > 1) {
        i64 fd = open_queue(name, 2, 0);
        CHECK(fd >= 0, 10);
        CHECK(CALL(184, fd, &notice, 0) == -16, 11); // Registration survived exec.
        CHECK(send(fd, "z", 1, 0) == 0, 12);
        CHECK(CALL(184, fd, &notice, 0) == 0, 13);
        CHECK(CALL(184, fd, 0, 0) == 0, 14);
        return 0;
    }
    struct mq_attr attr = {0, 1, 8, 0, {0, 0, 0, 0}};
    i64 fd = open_queue(name, 2 | 64 | 128, &attr);
    CHECK(fd >= 0 && CALL(25, fd, 1, 0) == 1, 20); // FD_CLOEXEC
    char stat_data[256] = {0};
    CHECK(CALL(80, fd, stat_data, 0) == -95 && CALL(44, fd, stat_data, 0) == -95, 43);
    CHECK(CALL(46, fd, 0, 0) == -95 && CALL(52, fd, 0600, 0) == -95 &&
              CALL(55, fd, -1, -1) == -95, 44);
    exercise_epoll(fd);
    CHECK(poll_queue(fd) == 4, 21);
    CHECK(send(fd, "data", 4, 5) == 0, 22);
    CHECK(poll_queue(fd) == 1, 23);
    char message[8] = {0};
    unsigned priority = 0;
    CHECK(receive(fd, message, &priority) == 4 && message[0] == 'd' && priority == 5, 24);
    CHECK(poll_queue(fd) == 4, 25);

    CHECK(CALL(184, fd, &notice, 0) == 0, 26);
    i64 duplicate = CALL(23, fd, 0, 0);
    CHECK(duplicate >= 0 && CALL(57, duplicate, 0, 0) == 0, 27);
    CHECK(CALL(184, fd, &notice, 0) == 0, 28); // Closing a duplicate deregisters.
    CHECK(CALL(184, fd, 0, 0) == 0, 29);
    duplicate = CALL(23, fd, 0, 0);
    CHECK(duplicate >= 0 && CALL(184, fd, &notice, 0) == 0, 30);
    CHECK(CALL(436, duplicate, duplicate, 0) == 0, 31);
    CHECK(CALL(184, fd, &notice, 0) == 0 && CALL(184, fd, 0, 0) == 0, 32);

    u64 mask = 1UL << 34;
    CHECK(call6(135, 0, (u64)&mask, 0, 8, 0, 0) == 0, 33); // Block SIGRTMIN+1.
    notice.value = 0x1234;
    notice.signal = 35;
    notice.method = 0; // SIGEV_SIGNAL
    CHECK(CALL(184, fd, &notice, 0) == 0 && send(fd, "s", 1, 0) == 0, 34);
    struct timespec immediate = {0, 0};
    char info[128] = {0};
    CHECK(call6(137, (u64)&mask, (u64)info, (u64)&immediate, 8, 0, 0) == 35, 35);
    CHECK(receive(fd, message, &priority) == 1 && message[0] == 's', 36);

    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 37);
    if (!child) {
        notice.method = 1;
        CHECK(CALL(25, fd, 2, 0) == 0 && CALL(184, fd, &notice, 0) == 0, 40);
        const char *arguments[] = {"/posix_mq_lifecycle", "after-exec", 0};
        const char *environment[] = {0};
        CALL(221, arguments[0], arguments, environment);
        fail(41);
    }
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 38);
    CHECK(receive(fd, message, &priority) == 1 && message[0] == 'z', 39);
    CHECK(CALL(181, name, 0, 0) == 0 && CALL(57, fd, 0, 0) == 0, 42);
    print("posix-mq-poll-notify-close-exec-ok\n");
    return 0;
}
