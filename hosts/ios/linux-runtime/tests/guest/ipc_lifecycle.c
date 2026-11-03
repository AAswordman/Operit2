#include "linux.h"

struct mq_attr {
    i64 flags, max_messages, message_size, current_messages;
    i64 reserved[4];
};
struct sem_op {
    unsigned short number;
    short operation, flags;
};
struct message {
    i64 type;
    char data[8];
};

int guest_main(u64 *stack) {
    (void)stack;
    const char name[] = "runtime-ipc-lifecycle";
    struct mq_attr attributes = {0, 4, 8, 0, {0, 0, 0, 0}};
    i64 mq = call6(180, (u64)name, 2 | 64 | 128, 0600, (u64)&attributes, 0, 0);
    CHECK(mq >= 0, 10);
    i64 msg = CALL(186, 0, 0600, 0);
    i64 sem = CALL(190, 0, 1, 0600);
    i64 shm = CALL(194, 0, 4096, 0600);
    CHECK(msg >= 0 && sem >= 0 && shm >= 0, 11);
    CHECK(call6(191, (u64)sem, 0, 16, 1, 0, 0) == 0, 12); // SETVAL
    i64 address = CALL(196, shm, 0, 0);
    CHECK(address > 0, 13);
    volatile unsigned *shared = (volatile unsigned *)(u64)address;
    *shared = 0x12345678;

    i64 child = CALL(220, 17, 0, 0); // fork-like clone with SIGCHLD
    CHECK(child >= 0, 14);
    if (!child) {
        struct sem_op decrement = {0, -1, 0};
        struct message sent = {7, {'s', 'y', 's', 'v', '-', 'o', 'k', '!'}};
        const char payload[8] = {'p', 'o', 's', 'i', 'x', 'o', 'k', '!'};
        CHECK(*shared == 0x12345678, 20);
        CHECK(CALL(193, sem, &decrement, 1) == 0, 21);
        *shared = 0x87654321;
        CHECK(call6(189, (u64)msg, (u64)&sent, sizeof(sent.data), 0, 0, 0) == 0, 22);
        CHECK(call6(182, (u64)mq, (u64)payload, sizeof(payload), 7, 0, 0) == 0, 23);
        CHECK(CALL(197, shared, 0, 0) == 0, 24);
        return 0;
    }

    struct message received = {0, {0}};
    CHECK(call6(188, (u64)msg, (u64)&received, sizeof(received.data), 7, 0, 0) == 8, 30);
    CHECK(received.type == 7 && received.data[0] == 's' && received.data[7] == '!', 31);
    char payload[8] = {0};
    unsigned priority = 0;
    CHECK(call6(183, (u64)mq, (u64)payload, sizeof(payload), (u64)&priority, 0, 0) == 8, 32);
    CHECK(payload[0] == 'p' && payload[7] == '!' && priority == 7, 33);
    int status = -1;
    CHECK(call6(260, (u64)child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 34);
    CHECK(*shared == 0x87654321, 35);
    CHECK(call6(191, (u64)sem, 0, 12, 0, 0, 0) == 0, 36); // GETVAL
    CHECK(CALL(195, shm, 0, 0) == 0, 37); // IPC_RMID while still attached
    CHECK(CALL(197, shared, 0, 0) == 0, 38);
    CHECK(call6(191, (u64)sem, 0, 0, 0, 0, 0) == 0, 39);
    CHECK(CALL(187, msg, 0, 0) == 0, 40);
    CHECK(CALL(57, mq, 0, 0) == 0 && CALL(181, name, 0, 0) == 0, 41);
    print("ipc-cross-process-lifecycle-ok\n");
    return 0;
}
