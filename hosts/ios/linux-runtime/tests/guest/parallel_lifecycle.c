#include "linux.h"

static void compute(void) {
    volatile u64 value = 1;
    for (unsigned n = 0; n < 100000; ++n)
        value = value * 33 + n;
}
int guest_main(u64 *stack) {
    (void)stack;
    char byte = 1;
    print("parallel-parent-ready\n");
    CHECK(CALL(63, 0, &byte, 1) == 1, 14);
    int ready[2];
    CHECK(CALL(59, ready, 0, 0) == 0, 1);
    const i64 child = CALL(220, 17, 0, 0); // fork: a separate address space.
    CHECK(child >= 0, 2);
    if (!child) {
        CHECK(CALL(57, ready[0], 0, 0) == 0, 3);
        const i64 fd = CALL(56, -100, "/data/message", 0);
        CHECK(fd >= 0, 4);
        const i64 mapping = call6(222, 0, 4096, 1, 2, (u64)fd, 0);
        CHECK(mapping > 0, 5);
        CHECK(*(volatile char *)mapping == 'i', 6);
        CHECK(CALL(57, fd, 0, 0) == 0, 7);
        CHECK(CALL(64, ready[1], &byte, 1) == 1, 8);
        compute();
        // Leave the mapping open for process exit to release.
        return 0;
    }
    CHECK(CALL(57, ready[1], 0, 0) == 0, 9);
    CHECK(CALL(63, ready[0], &byte, 1) == 1, 10);
    CHECK(CALL(57, ready[0], 0, 0) == 0, 11);
    compute();
    int status = -1;
    CHECK(CALL(260, child, &status, 0) == child && status == 0, 12);
    print("parallel-child-reaped\n");
    CHECK(CALL(63, 0, &byte, 1) == 1, 13); // Host keeps the parent blocked.
    return 0;
}
