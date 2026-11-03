#include "linux.h"

struct message {
    i64 type;
    char data[8];
};

int guest_main(u64 *stack) {
    (void)stack;
    const i64 queue = CALL(186, 0, 0600, 0);
    CHECK(queue >= 0, 10);
    struct message received = {0, {0}};
    CHECK(call6(188, queue, (u64)&received, sizeof(received.data), 0, 04000, 0) == -42, 11);

    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 12);
    if (!child) {
        CHECK(call6(188, queue, (u64)&received, sizeof(received.data), 1, 0, 0) == 3, 20);
        CHECK(received.type == 1 && received.data[0] == 'r' && received.data[2] == 'q', 21);
        struct message reply = {2, {'a', 'n', 's', 'w', 'e', 'r', '!', '!'}};
        CHECK(call6(189, queue, (u64)&reply, sizeof(reply.data), 0, 0, 0) == 0, 22);
        return 0;
    }
    struct message request = {1, {'r', 'e', 'q'}};
    CHECK(call6(189, queue, (u64)&request, 3, 0, 0, 0) == 0, 13);
    CHECK(call6(188, queue, (u64)&received, 3, 2, 010000, 0) == 3, 14);
    CHECK(received.type == 2 && received.data[0] == 'a' && received.data[2] == 's', 15);
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 16);
    CHECK(CALL(187, queue, 0, 0) == 0, 17);
    CHECK(call6(188, queue, (u64)&received, 1, 0, 04000, 0) == -22, 18);
    print("sysv-msg-cross-process-lifecycle-ok\n");
    return 0;
}
