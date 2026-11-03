#include "linux.h"
extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);
static unsigned char __attribute__((aligned(16))) stacks[3][8192];
static u64 tls_values[3];
static unsigned tids[3], condition, mutex_word, ready, completed, flags;
static const unsigned tid_mask = 0x3fffffffu;
static void worker(void *arg) {
    (void)arg;
    const unsigned self = (unsigned)CALL(178, 0, 0, 0);
    __atomic_fetch_add(&ready, 1, __ATOMIC_SEQ_CST);
    CALL(98, &ready, 1 | 128, 3);
    CHECK(call6(98, (u64)&condition, 11 | flags, 0, 0, (u64)&mutex_word, 0) == 0, 10);
    // WAIT_REQUEUE_PI may return zero only after ownership is installed.
    CHECK((__atomic_load_n(&mutex_word, __ATOMIC_SEQ_CST) & tid_mask) == self, 11);
    __atomic_fetch_add(&completed, 1, __ATOMIC_SEQ_CST);
    CHECK(CALL(98, &mutex_word, 7 | flags, 0) == 0, 12);
}
static void join(unsigned index) {
    unsigned tid;
    while ((tid = __atomic_load_n(tids + index, __ATOMIC_SEQ_CST))) {
        i64 result = CALL(98, tids + index, 0, tid);
        CHECK(result == 0 || result == -11, 20);
    }
}
static void run_round(unsigned private_flag, unsigned contended) {
    flags = private_flag;
    condition = mutex_word = ready = completed = 0;
    if (contended)
        CHECK(CALL(98, &mutex_word, 6 | flags, 0) == 0, 30);
    for (unsigned index = 0; index != 3; ++index)
        CHECK(spawn_thread(worker, 0, stacks[index] + sizeof(stacks[index]),
                           tls_values + index, tids + index) > 1, 31);
    for (;;) {
        unsigned observed = __atomic_load_n(&ready, __ATOMIC_SEQ_CST);
        if (observed == 3)
            break;
        i64 result = CALL(98, &ready, 128, observed);
        CHECK(result == 0 || result == -11, 32);
    }
    unsigned transferred = 0;
    while (transferred != 3) {
        // nr_requeue=0 must still migrate one waiter, including a locked target.
        i64 moved = call6(98, (u64)&condition, 12 | flags, 1, 0, (u64)&mutex_word, 0);
        CHECK(moved == 0 || moved == 1, 33);
        transferred += (unsigned)moved;
        if (!moved)
            CHECK(CALL(124, 0, 0, 0) == 0, 34); // Let a ready-but-not-queued task enter WAIT.
    }
    if (contended) {
        CHECK(__atomic_load_n(&completed, __ATOMIC_SEQ_CST) == 0, 35);
        CHECK((__atomic_load_n(&mutex_word, __ATOMIC_SEQ_CST) & tid_mask) ==
                  (unsigned)CALL(178, 0, 0, 0), 36);
        CHECK(CALL(98, &mutex_word, 7 | flags, 0) == 0, 37);
    }
    for (unsigned index = 0; index != 3; ++index)
        join(index);
    CHECK(completed == 3 && mutex_word == 0, 38);
}
int guest_main(u64 *stack) {
    (void)stack;
    run_round(128, 0);
    run_round(128, 1);
    run_round(0, 0);
    run_round(0, 1);
    print("futex-requeue-pi-ownership-handoff-join-ok\n");
    return 0;
}
