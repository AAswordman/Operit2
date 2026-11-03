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
    unsigned sq_entries, cq_entries, flags, sq_thread_cpu, sq_thread_idle, features, wq_fd;
    unsigned reserved[3];
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
struct cqe {
    u64 user_data;
    int result;
    unsigned flags;
};
struct iovec64 { u64 base, length; };
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

static void sleep_short(void) {
    const struct timespec64 delay = {0, 1000000};
    CHECK(CALL(101, &delay, 0, 0) == 0, 12);
}

static void release_gate(int wait_for_finish) {
    while (!exists("/io_uring_gate_entered") || !exists("/io_uring_submit_returned"))
        sleep_short();
    touch("/io_uring_gate_release");
    while (wait_for_finish && !exists("/io_uring_gate_finished"))
        sleep_short();
}

int guest_main(u64 *stack) {
    if (stack[0] == 2 && equal((const char *)stack[2], "exec_helper")) {
        touch("/io_uring_submit_returned");
        int status = -1;
        CHECK(call6(260, (u64)-1, (u64)&status, 0, 0, 0, 0) > 0 && status == 0, 34);
        print("io-uring-background-progress-ok\n");
        return 0;
    }
    CHECK(stack[0] == 3, 13);
    const char *operation = (const char *)stack[2];
    const char *scenario = (const char *)stack[3];
    const int is_read = equal(operation, "read") || equal(operation, "readv");
    const int is_vector = equal(operation, "readv") || equal(operation, "writev");
    const int is_sync = equal(operation, "fsync");
    const int close_file = equal(scenario, "close_fd");
    const int close_ring = equal(scenario, "close_ring");
    const int exec_process = equal(scenario, "exec");
    const int concurrent = equal(scenario, "concurrent");
    const int fail_io = equal(scenario, "error");
    const int unmap_buffer = equal(scenario, "unmap") || equal(scenario, "remap");
    const int remap_buffer = equal(scenario, "remap");

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

    char input[] = "UVWXYZ";
    char output[6] = {0};
    char *data = is_read ? output : input;
    if (unmap_buffer) {
        CHECK(is_read, 39);
        const i64 mapping = call6(222, 0, 4096, 3, 0x22, (u64)-1, 0);
        CHECK(mapping > 0, 40);
        data = (char *)(u64)mapping;
    }
    struct iovec64 vectors[] = {{(u64)data, 3}, {(u64)(data + 3), 3}};
    struct sqe request = {0};
    request.opcode = is_sync ? 3 : is_vector ? (is_read ? 1 : 2) : (is_read ? 22 : 23);
    request.fd = (int)file;
    request.address = is_sync ? 0 : (u64)(is_vector ? (void *)vectors : (void *)data);
    request.length = is_sync ? 0 : is_vector ? 2 : 6;
    request.user_data = 0x1234;
    ((struct sqe *)(u64)sqes)[0] = request;
    field((u64)ring, p.sq.array)[0] = 0;
    if (concurrent) {
        struct sqe nop = {0};
        nop.fd = -1;
        nop.user_data = 0x5678;
        ((struct sqe *)(u64)sqes)[1] = nop;
        field((u64)ring, p.sq.array)[1] = 1;
    }
    __atomic_store_n(field((u64)ring, p.sq.tail), concurrent ? 2 : 1, __ATOMIC_RELEASE);

    const i64 child = call6(220, 17, 0, 0, 0, 0, 0);
    CHECK(child >= 0, 18);
    if (!child) {
        if (close_ring || exec_process) {
            CHECK(CALL(57, file, 0, 0) == 0 && CALL(57, ring_fd, 0, 0) == 0 &&
                      CALL(57, epoll, 0, 0) == 0, 35);
            CHECK(CALL(215, ring, 4096, 0) == 0 && CALL(215, sqes, 4096, 0) == 0, 36);
        }
        release_gate(close_ring || exec_process);
        return 0;
    }

    CHECK(call6(426, ring_fd, concurrent ? 2 : 1, 0, 0, 0, 0) == (concurrent ? 2 : 1), 19);
    if (concurrent) {
        while (__atomic_load_n(field((u64)ring, p.cq.tail), __ATOMIC_ACQUIRE) == 0) {}
        volatile struct cqe *first = (volatile struct cqe *)((u64)ring + p.cq.cqes);
        CHECK(first->user_data == 0x5678 && first->result == 0, 41);
    }
    if (unmap_buffer) {
        CHECK(CALL(215, data, 4096, 0) == 0, 42);
        if (remap_buffer) {
            CHECK(call6(222, (u64)data, 4096, 3, 0x32, (u64)-1, 0) == (i64)(u64)data, 43);
            for (unsigned n = 0; n < 6; ++n)
                data[n] = '.';
        }
    }
    if (close_file) {
        CHECK(CALL(57, file, 0, 0) == 0, 20);
        CHECK(open_file("/io_uring_gate_replacement", 2) == file, 21);
    }
    if (close_ring) {
        CHECK(CALL(57, file, 0, 0) == 0 && CALL(57, ring_fd, 0, 0) == 0 &&
                  CALL(57, epoll, 0, 0) == 0, 22);
        CHECK(CALL(215, ring, 4096, 0) == 0 && CALL(215, sqes, 4096, 0) == 0, 23);
    }
    if (exec_process) {
        CHECK(CALL(57, file, 0, 0) == 0 && CALL(57, epoll, 0, 0) == 0, 37);
        const char *const args[] = {"/io_uring_async", "exec_helper", 0};
        const char *const environment[] = {0};
        CALL(221, args[0], args, environment);
        fail(38);
    }
    touch("/io_uring_submit_returned");

    if (!close_ring) {
        const unsigned wanted = concurrent ? 2 : 1;
        while (__atomic_load_n(field((u64)ring, p.cq.tail), __ATOMIC_ACQUIRE) < wanted) {}
        struct pollfd poll = {(int)ring_fd, 1, 0};
        const i64 timeout[2] = {0, 0};
        CHECK(call6(73, (u64)&poll, 1, (u64)timeout, 0, 0, 0) == 1 && (poll.returned & 1), 24);
        struct epoll_event ready = {0};
        CHECK(call6(22, epoll, (u64)&ready, 1, 0, 0, 0) == 1 &&
                  (ready.events & 1) && ready.data == 77, 25);
        volatile struct cqe *completions = (volatile struct cqe *)((u64)ring + p.cq.cqes);
        int saw_operation = 0, saw_nop = 0;
        for (unsigned n = 0; n < wanted; ++n) {
            const struct cqe cqe = completions[n & *field((u64)ring, p.cq.mask)];
            CHECK(!cqe.flags, 26);
            if (cqe.user_data == 0x1234) {
                CHECK(cqe.result == (fail_io ? -5 : unmap_buffer ? -14 : is_sync ? 0 : 6), 27);
                saw_operation++;
            } else if (cqe.user_data == 0x5678) {
                CHECK(cqe.result == 0, 28);
                saw_nop++;
            } else
                fail(29);
        }
        CHECK(saw_operation == 1 && saw_nop == concurrent, 30);
        __atomic_store_n(field((u64)ring, p.cq.head), wanted, __ATOMIC_RELEASE);
        CHECK(call6(22, epoll, (u64)&ready, 1, 0, 0, 0) == 0, 31);
        if (remap_buffer)
            for (unsigned n = 0; n < 6; ++n)
                CHECK(data[n] == '.', 44);
        if (is_read && !fail_io && !unmap_buffer)
            for (unsigned n = 0; n < 6; ++n)
                CHECK(output[n] == "abcdef"[n], 32);
    }
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 33);
    print("io-uring-background-progress-ok\n");
    return 0;
}
