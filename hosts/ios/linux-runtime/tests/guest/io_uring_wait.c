#include "linux.h"

struct sq_offsets {
    unsigned head, tail, mask, entries, flags, dropped, array, reserved;
    u64 user_address;
};
struct cq_offsets {
    unsigned head, tail, mask, entries, overflow, cqes, flags, reserved;
    u64 user_address;
};
struct params {
    unsigned sq_entries, cq_entries, flags, cpu, idle, features, wq_fd, reserved[3];
    struct sq_offsets sq;
    struct cq_offsets cq;
};
struct sqe {
    unsigned char opcode, flags;
    unsigned short priority;
    int fd;
    u64 offset, address;
    unsigned length, operation_flags;
    u64 user_data;
    unsigned short buffer_index, personality;
    unsigned file_index;
    u64 address3, reserved;
};
struct cqe { u64 user_data; int result; unsigned flags; };
struct pollfd { int fd; short events, returned; };
struct epoll_event { unsigned events, padding; u64 data; };
struct timespec64 { i64 seconds, nanoseconds; };

static volatile unsigned *field(u64 base, unsigned offset) {
    return (volatile unsigned *)(base + offset);
}

static i64 open_file(const char *path, unsigned flags) {
    return call6(56, (u64)-100, (u64)path, flags, 0600, 0, 0);
}

static void touch(const char *path) {
    const i64 fd = open_file(path, 1 | 64 | 512);
    CHECK(fd >= 0 && CALL(57, fd, 0, 0) == 0, 10);
}

static int exists(const char *path) {
    const i64 fd = open_file(path, 0);
    if (fd < 0)
        return 0;
    CHECK(CALL(57, fd, 0, 0) == 0, 11);
    return 1;
}

static void sleep_ns(i64 nanoseconds) {
    const struct timespec64 delay = {0, nanoseconds};
    CHECK(CALL(101, &delay, 0, 0) == 0, 12);
}

int guest_main(u64 *stack) {
    CHECK(stack[0] == 2, 13);
    const char *mode = (const char *)stack[2];
    struct params p = {0};
    const i64 ring_fd = CALL(425, 8, &p, 0);
    CHECK(ring_fd >= 0, 14);
    const i64 ring = call6(222, 0, 4096, 3, 1, (u64)ring_fd, 0);
    const i64 sqes = call6(222, 0, 4096, 3, 1, (u64)ring_fd, 0x10000000);
    CHECK(ring > 0 && sqes > 0, 15);
    const i64 file = open_file("/io_uring_gate_target", 2);
    CHECK(file >= 0, 16);
    const i64 epoll = CALL(20, 0, 0, 0);
    struct epoll_event interest = {1, 0, 77};
    CHECK(epoll >= 0 && call6(21, epoll, 1, ring_fd, (u64)&interest, 0, 0) == 0, 17);

    char data[] = "UVWXYZ";
    struct sqe request = {0};
    request.opcode = 23;
    request.fd = (int)file;
    request.address = (u64)data;
    request.length = 6;
    request.user_data = 0x1234;
    ((struct sqe *)(u64)sqes)[0] = request;
    field((u64)ring, p.sq.array)[0] = 0;
    __atomic_store_n(field((u64)ring, p.sq.tail), 1, __ATOMIC_RELEASE);

    const i64 child = call6(220, 17, 0, 0, 0, 0, 0);
    CHECK(child >= 0, 18);
    if (!child) {
        while (!exists("/io_uring_gate_entered") || !exists("/io_uring_wait_started"))
            sleep_ns(1000000);
        sleep_ns(30000000);
        touch("/io_uring_gate_release");
        return 0;
    }
    CHECK(call6(426, ring_fd, 1, 0, 0, 0, 0) == 1, 19);
    touch("/io_uring_submit_returned");
    touch("/io_uring_wait_started");

    if (equal(mode, "enter")) {
        CHECK(call6(426, ring_fd, 0, 1, 1, 0, 0) == 0, 20);
    } else if (equal(mode, "poll")) {
        struct pollfd ready = {(int)ring_fd, 1, 0};
        CHECK(call6(73, (u64)&ready, 1, 0, 0, 0, 0) == 1 && (ready.returned & 1), 21);
    } else if (equal(mode, "epoll")) {
        struct epoll_event ready = {0};
        CHECK(call6(22, epoll, (u64)&ready, 1, (u64)-1, 0, 0) == 1 &&
                  (ready.events & 1) && ready.data == 77, 22);
    } else {
        fail(23);
    }
    CHECK(__atomic_load_n(field((u64)ring, p.cq.tail), __ATOMIC_ACQUIRE) == 1, 24);
    const struct cqe result = *(volatile struct cqe *)((u64)ring + p.cq.cqes);
    CHECK(result.user_data == 0x1234 && result.result == 6 && result.flags == 0, 25);
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 26);
    print("io-uring-blocking-wakeup-ok\n");
    return 0;
}
