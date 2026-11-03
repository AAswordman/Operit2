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
    unsigned short ioprio;
    int fd;
    u64 offset, address;
    unsigned length, operation_flags;
    u64 user_data;
    unsigned short buffer_index, personality;
    unsigned file_index;
    u64 address3, reserved;
};
struct cqe { u64 user_data; int result; unsigned flags; };
struct poll_entry { int fd; short events, revents; };
struct timespec64 { i64 seconds, nanoseconds; };

static unsigned load32(volatile unsigned *value) {
    return __atomic_load_n(value, __ATOMIC_ACQUIRE);
}
static void store32(volatile unsigned *value, unsigned next) {
    __atomic_store_n(value, next, __ATOMIC_RELEASE);
}
static void zero(void *pointer, unsigned length) {
    unsigned char *bytes = pointer;
    for (unsigned i = 0; i < length; ++i)
        bytes[i] = 0;
}

int guest_main(u64 *stack) {
    (void)stack;
    struct params p = {0};
    CHECK(CALL(425, 0, &p, 0) == -22, 10);
    p.flags = 2;
    CHECK(CALL(425, 2, &p, 0) == -22, 11);
    zero(&p, sizeof(p));
    p.flags = 8;
    p.cq_entries = 2;
    int fd = CALL(425, 2, &p, 0);
    CHECK(fd >= 0 && p.sq_entries == 2 && p.cq_entries == 2 && (p.features & 1), 12);
    CHECK(CALL(25, fd, 1, 0) == 1, 13);
    int duplicate = CALL(23, fd, 0, 0);
    CHECK(duplicate >= 0 && CALL(57, fd, 0, 0) == 0, 14);
    fd = duplicate;
    unsigned ring_length = p.sq.array + p.sq_entries * 4;
    unsigned cq_length = p.cq.cqes + p.cq_entries * 16;
    if (cq_length > ring_length)
        ring_length = cq_length;
    ring_length = (ring_length + 4095) & ~4095u;
    i64 sq_map = call6(222, 0, ring_length, 3, 1, fd, 0);
    i64 cq_map = call6(222, 0, ring_length, 3, 1, fd, 0x8000000);
    i64 sqes_map = call6(222, 0, 4096, 3, 1, fd, 0x10000000);
    CHECK(sq_map > 0 && cq_map > 0 && sqes_map > 0, 15);
    volatile unsigned *sq_head = (void *)(u64)(sq_map + p.sq.head);
    volatile unsigned *sq_tail = (void *)(u64)(sq_map + p.sq.tail);
    volatile unsigned *array = (void *)(u64)(sq_map + p.sq.array);
    volatile unsigned *cq_head = (void *)(u64)(cq_map + p.cq.head);
    volatile unsigned *cq_tail = (void *)(u64)(cq_map + p.cq.tail);
    struct sqe *sqes = (void *)(u64)sqes_map;
    struct cqe *cqes = (void *)(u64)(cq_map + p.cq.cqes);
    CHECK(*(volatile unsigned *)(u64)(sq_map + p.cq.mask) == 1, 16);
    struct poll_entry pollfd = {fd, 1, 0};
    struct timespec64 timeout = {0, 0};
    CHECK(call6(73, (u64)&pollfd, 1, (u64)&timeout, 0, 8, 0) == 0, 17);
    const int file = call6(56, -100, (u64)"/data/io-uring-probe", 64 | 512 | 2, 0600, 0, 0);
    CHECK(file >= 0, 18);
    char text[] = "ring-data";
    char received[sizeof(text)];
    zero(received, sizeof(received));
    zero(&sqes[0], sizeof(sqes[0]));
    sqes[0].opcode = 23;
    sqes[0].fd = file;
    sqes[0].offset = 4;
    sqes[0].address = (u64)text;
    sqes[0].length = sizeof(text);
    sqes[0].user_data = 0x1234;
    array[0] = 0;
    store32(sq_tail, 1);
    CHECK(call6(426, fd, 1, 1, 1, 0, 0) == 1, 19);
    CHECK(load32(sq_head) == 1 && load32(cq_tail) == 1, 20);
    CHECK(cqes[0].user_data == 0x1234 && cqes[0].result == sizeof(text), 21);
    CHECK(call6(73, (u64)&pollfd, 1, (u64)&timeout, 0, 8, 0) == 1 && (pollfd.revents & 1), 22);
    store32(cq_head, 1);
    pollfd.revents = 0;
    CHECK(call6(73, (u64)&pollfd, 1, (u64)&timeout, 0, 8, 0) == 0, 23);
    zero(&sqes[1], sizeof(sqes[1]));
    sqes[1].opcode = 22;
    sqes[1].fd = file;
    sqes[1].offset = 4;
    sqes[1].address = (u64)received;
    sqes[1].length = sizeof(received);
    sqes[1].user_data = 0x5678;
    array[1] = 1;
    store32(sq_tail, 2);
    CHECK(call6(426, fd, 1, 1, 1, 0, 0) == 1, 24);
    CHECK(cqes[1].user_data == 0x5678 && cqes[1].result == sizeof(received), 25);
    for (unsigned i = 0; i < sizeof(text); ++i)
        CHECK(received[i] == text[i], 26);
    store32(cq_head, 2);
    unsigned char probe[16 + 24 * 8] = {0};
    CHECK(call6(427, fd, 8, (u64)probe, 24, 0, 0) == 0, 27);
    CHECK(probe[1] == 24 && probe[16 + 22 * 8 + 2] == 1 && probe[16 + 23 * 8 + 2] == 1, 28);
    zero(&sqes[0], sizeof(sqes[0]));
    sqes[0].opcode = 99;
    sqes[0].user_data = 9;
    array[0] = 0;
    store32(sq_tail, 3);
    CHECK(call6(426, fd, 1, 0, 0, 0, 0) == 1, 29);
    CHECK(load32(cq_tail) == 3 && cqes[0].user_data == 9 && cqes[0].result == -22, 30);
    store32(cq_head, 3);
    CHECK(CALL(63, fd, received, sizeof(received)) < 0, 31);
    CHECK(CALL(57, fd, 0, 0) == 0, 32);
    CHECK(load32(sq_head) == 3, 33);
    CHECK(CALL(215, sq_map, ring_length, 0) == 0 && CALL(215, cq_map, ring_length, 0) == 0 &&
          CALL(215, sqes_map, 4096, 0) == 0, 34);
    CHECK(CALL(57, file, 0, 0) == 0 && CALL(35, -100, "/data/io-uring-probe", 0) == 0, 35);
    print("io-uring-mmap-file-poll-probe-ok\n");
    return 0;
}
