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
struct pollfd {
    int fd;
    short events, returned;
};
struct epoll_event {
    unsigned events, padding;
    u64 data;
};
struct sigaction64 {
    u64 handler, flags, restorer, mask;
};
struct timespec64 {
    i64 seconds, nanoseconds;
};
struct iovec64 {
    u64 base, length;
};

static volatile unsigned signals_seen;

static void on_signal(int signal) {
    if (signal == 10)
        ++signals_seen;
}

static volatile unsigned *field(u64 base, unsigned offset) {
    return (volatile unsigned *)(base + offset);
}
static void submit_result(i64 fd, i64 epoll, u64 ring, struct sqe *entries, const struct params *p,
                          struct sqe request, int expected) {
    unsigned tail = *field(ring, p->sq.tail);
    unsigned index = tail & *field(ring, p->sq.mask);
    entries[index] = request;
    field(ring, p->sq.array)[index] = index;
    __atomic_store_n(field(ring, p->sq.tail), tail + 1, __ATOMIC_RELEASE);
    CHECK(call6(426, (u64)fd, 1, 1, 1, 0, 0) == 1, 20);
    struct pollfd poll = {(int)fd, 1, 0};
    i64 timeout[2] = {0, 0};
    CHECK(call6(73, (u64)&poll, 1, (u64)timeout, 0, 0, 0) == 1 && (poll.returned & 1), 21);
    struct epoll_event ready = {0};
    CHECK(call6(22, (u64)epoll, (u64)&ready, 1, 0, 0, 0) == 1 &&
              (ready.events & 1) && ready.data == 99, 24);
    unsigned head = *field(ring, p->cq.head);
    unsigned count = __atomic_load_n(field(ring, p->cq.tail), __ATOMIC_ACQUIRE);
    CHECK(count - head == 1, 22);
    volatile struct cqe *completions = (volatile struct cqe *)(ring + p->cq.cqes);
    struct cqe completion = completions[head & *field(ring, p->cq.mask)];
    CHECK(completion.user_data == request.user_data && completion.result == expected && !completion.flags, 23);
    __atomic_store_n(field(ring, p->cq.head), head + 1, __ATOMIC_RELEASE);
    CHECK(call6(22, (u64)epoll, (u64)&ready, 1, 0, 0, 0) == 0, 25);
    poll.returned = 0;
    CHECK(call6(73, (u64)&poll, 1, (u64)timeout, 0, 0, 0) == 0, 26);
}

static void submit(i64 fd, i64 epoll, u64 ring, struct sqe *entries, const struct params *p,
                   unsigned opcode, int file, void *buffer, unsigned size, u64 token) {
    struct sqe request = {0};
    request.opcode = opcode;
    request.fd = file;
    request.address = (u64)buffer;
    request.length = size;
    request.user_data = token;
    submit_result(fd, epoll, ring, entries, p, request, (int)size);
}

static void vector_io_and_sync(i64 fd, i64 epoll, u64 ring, struct sqe *entries,
                               const struct params *p, int file) {
    char first[] = "abc", second[] = "DEFGH";
    struct iovec64 vectors[] = {{(u64)first, 3}, {0, 0}, {(u64)second, 5}};
    struct sqe request = {0};
    request.opcode = 2;
    request.fd = file;
    request.address = (u64)vectors;
    request.length = 3;
    request.offset = 10;
    request.user_data = 50;
    CHECK(CALL(62, file, 6, 0) == 6, 50);
    submit_result(fd, epoll, ring, entries, p, request, 8);
    CHECK(CALL(62, file, 0, 1) == 6, 51);
    char actual[8] = {0};
    CHECK(call6(67, file, (u64)actual, sizeof(actual), 10, 0, 0) == 8, 52);
    for (unsigned i = 0; i < 3; ++i)
        CHECK(actual[i] == first[i], 53);
    for (unsigned i = 0; i < 5; ++i)
        CHECK(actual[i + 3] == second[i], 54);

    char left[2] = {0}, right[8] = {'x', 'x', 'x', 'x', 'x', 'x', 'x', 'x'};
    struct iovec64 output[] = {{(u64)left, sizeof(left)}, {(u64)right, sizeof(right)}};
    request.opcode = 1;
    request.address = (u64)output;
    request.length = 2;
    request.user_data = 51;
    submit_result(fd, epoll, ring, entries, p, request, 8);
    CHECK(left[0] == 'a' && left[1] == 'b' && right[0] == 'c' && right[5] == 'H' &&
          right[6] == 'x' && right[7] == 'x', 55);
    CHECK(CALL(62, file, 0, 1) == 6, 56);

    CHECK(CALL(62, file, 10, 0) == 10, 57);
    output[1].length = 3;
    request.offset = (u64)-1;
    request.user_data = 52;
    submit_result(fd, epoll, ring, entries, p, request, 5);
    CHECK(CALL(62, file, 0, 1) == 15, 58);
    CHECK(left[0] == 'a' && left[1] == 'b' && right[0] == 'c' && right[2] == 'E', 59);

    vectors[1].length = 1;
    request.opcode = 2;
    request.address = (u64)vectors;
    request.length = 2;
    request.offset = 20;
    request.user_data = 53;
    submit_result(fd, epoll, ring, entries, p, request, 3);
    CHECK(call6(67, file, (u64)actual, 3, 20, 0, 0) == 3 && actual[0] == 'a' &&
          actual[1] == 'b' && actual[2] == 'c', 60);
    request.opcode = 1;
    request.address = 0;
    request.length = 1;
    request.user_data = 54;
    submit_result(fd, epoll, ring, entries, p, request, -14);
    request.address = (u64)vectors;
    request.length = 1025;
    request.user_data = 55;
    submit_result(fd, epoll, ring, entries, p, request, -22);

    request = (struct sqe){0};
    request.opcode = 3;
    request.fd = file;
    request.user_data = 56;
    submit_result(fd, epoll, ring, entries, p, request, 0);
    request.offset = 10;
    request.length = 8;
    request.operation_flags = 1;
    request.user_data = 57;
    submit_result(fd, epoll, ring, entries, p, request, 0);
    request.operation_flags = 2;
    request.user_data = 58;
    submit_result(fd, epoll, ring, entries, p, request, -22);
    request.operation_flags = 0;
    request.fd = -1;
    request.user_data = 59;
    submit_result(fd, epoll, ring, entries, p, request, -9);

    unsigned char probe[16 + 24 * 8] = {0};
    CHECK(call6(427, fd, 8, (u64)probe, 24, 0, 0) == 0, 61);
    CHECK(probe[16 + 1 * 8 + 2] == 1 && probe[16 + 2 * 8 + 2] == 1 &&
          probe[16 + 3 * 8 + 2] == 1 && probe[16 + 4 * 8 + 2] == 0, 62);
}

static void queue_nop(u64 ring, struct sqe *entries, const struct params *p, u64 token) {
    unsigned tail = *field(ring, p->sq.tail);
    unsigned index = tail & *field(ring, p->sq.mask);
    struct sqe request = {0};
    request.fd = -1;
    request.user_data = token;
    entries[index] = request;
    field(ring, p->sq.array)[index] = index;
    __atomic_store_n(field(ring, p->sq.tail), tail + 1, __ATOMIC_RELEASE);
}

static i64 signal_later(i64 ring_fd) {
    i64 parent = CALL(172, 0, 0, 0);
    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 30);
    if (!child) {
        struct timespec64 delay = {0, 100000000};
        CHECK(CALL(57, ring_fd, 0, 0) == 0, 31);
        CHECK(CALL(101, &delay, 0, 0) == 0, 32);
        CHECK(CALL(129, parent, 10, 0) == 0, 33);
        CALL(93, 0, 0, 0);
        __builtin_unreachable();
    }
    return child;
}

static void wait_signaler(i64 child, unsigned expected) {
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 34);
    CHECK(signals_seen == expected, 35);
}

static void signal_waits(i64 fd, u64 ring, struct sqe *entries, const struct params *p) {
    struct sigaction64 action = {(u64)on_signal, 0x10000000, 0, 0}; // SA_RESTART
    CHECK(call6(134, 10, (u64)&action, 0, 8, 0, 0) == 0, 36);

    queue_nop(ring, entries, p, 41);
    CHECK(call6(426, fd, 1, 0, 0, 0, 0) == 1, 37);
    i64 child = signal_later(fd);
    CHECK(call6(426, fd, 0, 2, 1, 0, 0) == 0, 38);
    wait_signaler(child, 1);
    unsigned head = *field(ring, p->cq.head);
    CHECK(*field(ring, p->cq.tail) - head == 1, 39);
    __atomic_store_n(field(ring, p->cq.head), head + 1, __ATOMIC_RELEASE);

    queue_nop(ring, entries, p, 42);
    child = signal_later(fd);
    CHECK(call6(426, fd, 1, 2, 1, 0, 0) == 1, 40);
    wait_signaler(child, 2);
    head = *field(ring, p->cq.head);
    CHECK(*field(ring, p->cq.tail) - head == 1, 41);
    __atomic_store_n(field(ring, p->cq.head), head + 1, __ATOMIC_RELEASE);

    child = signal_later(fd);
    CHECK(call6(426, fd, 0, 1, 1, 0, 0) == -4, 42);
    wait_signaler(child, 3);
}

int guest_main(u64 *stack) {
    (void)stack;
    struct params p = {0};
    i64 ring_fd = CALL(425, 8, &p, 0);
    CHECK(ring_fd >= 0 && (p.features & 1), 10); // IORING_FEAT_SINGLE_MMAP
    i64 ring = call6(222, 0, 4096, 3, 1, (u64)ring_fd, 0);
    i64 sqes = call6(222, 0, 4096, 3, 1, (u64)ring_fd, 0x10000000);
    CHECK(ring > 0 && sqes > 0, 11);
    i64 duplicate = CALL(23, ring_fd, 0, 0);
    CHECK(duplicate >= 0 && CALL(57, ring_fd, 0, 0) == 0, 17);
    ring_fd = duplicate;
    i64 epoll = CALL(20, 0, 0, 0);
    struct epoll_event interest = {1, 0, 99};
    CHECK(epoll >= 0 && call6(21, (u64)epoll, 1, (u64)ring_fd, (u64)&interest, 0, 0) == 0, 16);
    const char path[] = "/io-uring-lifecycle";
    i64 file = call6(56, (u64)-100, (u64)path, 2 | 64 | 512, 0600, 0, 0);
    CHECK(file >= 0, 12);
    char written[8] = {'r', 'i', 'n', 'g', '-', 'o', 'k', '!'};
    char read[8] = {0};
    submit(ring_fd, epoll, (u64)ring, (struct sqe *)(u64)sqes, &p, 0, -1, 0, 0, 1);
    submit(ring_fd, epoll, (u64)ring, (struct sqe *)(u64)sqes, &p, 23, (int)file, written, 8, 2);
    submit(ring_fd, epoll, (u64)ring, (struct sqe *)(u64)sqes, &p, 22, (int)file, read, 8, 3);
    CHECK(read[0] == 'r' && read[7] == '!', 13);
    vector_io_and_sync(ring_fd, epoll, (u64)ring, (struct sqe *)(u64)sqes, &p, (int)file);
    signal_waits(ring_fd, (u64)ring, (struct sqe *)(u64)sqes, &p);
    CHECK(CALL(57, file, 0, 0) == 0 && CALL(57, ring_fd, 0, 0) == 0 &&
              CALL(57, epoll, 0, 0) == 0, 14);
    CHECK(CALL(215, ring, 4096, 0) == 0 && CALL(215, sqes, 4096, 0) == 0, 15);
    print("io-uring-mmap-submit-poll-file-io-ok\n");
    return 0;
}
