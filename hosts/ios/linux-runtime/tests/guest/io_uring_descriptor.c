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
struct iovec64 { u64 base, length; };
struct poll_entry { int fd; short events, revents; };
struct timespec64 { i64 seconds, nanoseconds; };

int guest_main(u64 *stack) {
    (void)stack;
    struct params p = {0};
    int fd = CALL(425, 1, &p, 0);
    CHECK(fd >= 0, 10);
    i64 ring = call6(222, 0, 4096, 3, 1, fd, 0);
    i64 entries = call6(222, 0, 4096, 3, 1, fd, 0x10000000);
    CHECK(ring > 0 && entries > 0, 11);
    volatile unsigned *tail = (void *)(u64)(ring + p.sq.tail);
    unsigned *array = (void *)(u64)(ring + p.sq.array);
    struct sqe *sqe = (void *)(u64)entries;
    sqe->opcode = 0;
    array[0] = 0;
    __atomic_store_n(tail, 1, __ATOMIC_RELEASE);
    CHECK(call6(426, fd, 1, 0, 0, 0, 0) == 1, 12);
    char byte = 0;
    struct iovec64 vector = {(u64)&byte, 1};
    CHECK(CALL(65, fd, &vector, 1) < 0, 13);
    struct poll_entry pollfd = {fd, 1, 0};
    struct timespec64 timeout = {0, 0};
    CHECK(call6(73, (u64)&pollfd, 1, (u64)&timeout, 0, 8, 0) == 1 && (pollfd.revents & 1), 14);
    CHECK(CALL(57, fd, 0, 0) == 0, 15);
    CHECK(CALL(215, ring, 4096, 0) == 0 && CALL(215, entries, 4096, 0) == 0, 16);
    print("io-uring-internal-descriptor-protected\n");
    return 0;
}
