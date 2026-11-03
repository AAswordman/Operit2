#include "linux.h"
extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);
static unsigned char stacks[4][8192] __attribute__((aligned(16)));
static u64 tls_values[4];
static unsigned tids[4], ready, gate, total;
static void worker(void *arg) {
    unsigned index = (unsigned)(u64)arg;
    u64 tls;
    __asm__ volatile("mrs %0,tpidr_el0" : "=r"(tls));
    CHECK(tls == (u64)&tls_values[index], 10);
    CHECK(CALL(178, 0, 0, 0) == (i64)tids[index], 11);
    __atomic_fetch_add(&ready, 1, __ATOMIC_SEQ_CST);
    CALL(98, &ready, 129, 1);
    while (!__atomic_load_n(&gate, __ATOMIC_SEQ_CST)) {
        i64 result = CALL(98, &gate, 128, 0);
        CHECK(result == 0 || result == -11, 12);
    }
    for (unsigned n = 0; n < 200; ++n)
        __atomic_fetch_add(&total, 1, __ATOMIC_SEQ_CST);
}
int guest_main(u64 *stack) {
    (void)stack;
    for (unsigned j = 0; j < 4; ++j) {
        i64 tid =
            spawn_thread(worker, (void *)(u64)j, stacks[j] + sizeof(stacks[j]), tls_values + j, tids + j);
        CHECK(tid > 1 && tid == (i64)tids[j], 20);
    }
    while (__atomic_load_n(&ready, __ATOMIC_SEQ_CST) != 4) {
        unsigned observed = __atomic_load_n(&ready, __ATOMIC_SEQ_CST);
        if (observed != 4) {
            i64 result = CALL(98, &ready, 128, observed);
            CHECK(result == 0 || result == -11, 21);
        }
    }
    __atomic_store_n(&gate, 1, __ATOMIC_SEQ_CST);
    CHECK(CALL(98, &gate, 129, 4) >= 0, 22);
    for (unsigned j = 0; j < 4; ++j) {
        unsigned tid;
        while ((tid = __atomic_load_n(tids + j, __ATOMIC_SEQ_CST)) != 0) {
            i64 result = CALL(98, tids + j, 0, tid);
            CHECK(result == 0 || result == -11, 23);
        }
    }
    CHECK(total == 800, 24);
    i64 short_wait[2] = {0, 1000000};
    CHECK(call6(98, (u64)&gate, 128, 1, (u64)short_wait, 0, 0) == -110, 25);
    CHECK(CALL(98, (u64)&gate + 1, 129, 1) == -22, 26);
    print("threads-tls-futex-join-ok\n");
    return 0;
}
