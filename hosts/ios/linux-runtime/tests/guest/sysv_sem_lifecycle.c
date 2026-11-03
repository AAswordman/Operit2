#include "linux.h"

struct sem_op {
    unsigned short number;
    short operation, flags;
};
struct timespec {
    i64 seconds, nanoseconds;
};

static i64 value(i64 id) {
    return call6(191, id, 0, 12, 0, 0, 0); // GETVAL
}
static void wait_child(i64 child, int failure) {
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child && status == 0, failure);
}

int guest_main(u64 *stack) {
    const i64 key = 0x49357;
    const i64 sem = CALL(190, key, stack[0] > 1 ? 0 : 1, stack[0] > 1 ? 0 : 01000 | 0600);
    CHECK(sem >= 0, 10);
    if (stack[0] > 1) {
        CHECK(value(sem) == 1, 11); // Exec must preserve the caller's undo list.
        return 0;
    }
    CHECK(call6(191, sem, 0, 16, 2, 0, 0) == 0, 12); // SETVAL
    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 13);
    if (!child) {
        struct sem_op acquire = {0, -1, 010000}; // SEM_UNDO
        CHECK(CALL(193, sem, &acquire, 1) == 0, 20);
        const char *arguments[] = {"/sysv_sem_lifecycle", "after-exec", 0};
        const char *environment[] = {0};
        CALL(221, arguments[0], arguments, environment);
        fail(21);
    }
    wait_child(child, 14);
    CHECK(value(sem) == 2, 15); // Child exit applies undo after exec.

    child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 16);
    if (!child) {
        struct sem_op acquire = {0, -1, 010000};
        CHECK(CALL(193, sem, &acquire, 1) == 0, 30);
        i64 sharing_child = CALL(220, 0x40000 | 17, 0, 0); // CLONE_SYSVSEM
        CHECK(sharing_child >= 0, 31);
        if (!sharing_child) {
            CHECK(CALL(193, sem, &acquire, 1) == 0, 32);
            return 0;
        }
        wait_child(sharing_child, 33);
        CHECK(value(sem) == 0, 34); // Shared undo waits for the last owner.
        return 0;
    }
    wait_child(child, 17);
    CHECK(value(sem) == 2, 18);
    CHECK(call6(191, sem, 0, 16, 0, 0, 0) == 0, 19);
    struct sem_op no_wait = {0, -1, 04000};
    CHECK(CALL(193, sem, &no_wait, 1) == -11, 22);
    no_wait.flags = 0;
    struct timespec timeout = {0, 0};
    CHECK(call6(192, sem, (u64)&no_wait, 1, (u64)&timeout, 0, 0) == -11, 23);
    CHECK(call6(191, sem, 0, 0, 0, 0, 0) == 0, 24); // IPC_RMID
    print("sysv-sem-undo-fork-clone-exec-ok\n");
    return 0;
}
