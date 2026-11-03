#include "linux.h"
extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);
static unsigned char stacks[4][16384] __attribute__((aligned(16)));
static u64 tls_values[4], results[4];
static unsigned tids[4], ready, gate, total;
static u64 calculate(unsigned index) {
    u64 value = 0x123456789abcdefULL + index;
    for (unsigned n = 0; n < 100000; ++n) {
        value ^= n;
        value *= 0x100000001b3ULL;
        value = (value << 13) | (value >> 51);
    }
    return value;
}
static void worker(void *argument) {
    const unsigned index = (unsigned)(u64)argument;
    __atomic_fetch_add(&ready, 1, __ATOMIC_SEQ_CST);
    CALL(98, &ready, 129, 1);
    while (!__atomic_load_n(&gate, __ATOMIC_SEQ_CST)) {
        const i64 result = CALL(98, &gate, 128, 0);
        CHECK(result == 0 || result == -11, 10);
    }
    unsigned cpu = 99, node = 99;
    CHECK(CALL(168, &cpu, &node, 0) == 0 && cpu < 4 && node == 0, 11);
    results[index] = calculate(index);
    for (unsigned n = 0; n < 10000; ++n)
        __atomic_fetch_add(&total, 1, __ATOMIC_SEQ_CST);
}
int guest_main(u64 *stack) {
    (void)stack;
    for (unsigned index = 0; index < 4; ++index)
        CHECK(spawn_thread(worker, (void *)(u64)index, stacks[index] + sizeof(stacks[index]),
                           tls_values + index, tids + index) > 1, 20);
    while (__atomic_load_n(&ready, __ATOMIC_SEQ_CST) != 4) {
        const unsigned observed = __atomic_load_n(&ready, __ATOMIC_SEQ_CST);
        if (observed != 4) {
            const i64 result = CALL(98, &ready, 128, observed);
            CHECK(result == 0 || result == -11, 21);
        }
    }
    __atomic_store_n(&gate, 1, __ATOMIC_SEQ_CST);
    CHECK(CALL(98, &gate, 129, 4) >= 0, 22);
    for (unsigned index = 0; index < 4; ++index) {
        unsigned tid;
        while ((tid = __atomic_load_n(tids + index, __ATOMIC_SEQ_CST))) {
            const i64 result = CALL(98, tids + index, 0, tid);
            CHECK(result == 0 || result == -11, 23);
        }
        CHECK(results[index] == calculate(index), 24);
    }
    CHECK(total == 40000, 25);
    print("parallel-compute-atomics-join-ok\n");
    return 0;
}
