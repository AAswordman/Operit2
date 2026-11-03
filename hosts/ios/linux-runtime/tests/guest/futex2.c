#include "linux.h"
extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);

enum { workers = 3, rounds = 24, private_u32 = 130 };
struct waiter {
    u64 val, uaddr;
    unsigned flags, reserved;
};
static unsigned char stacks[workers][8192] __attribute__((aligned(16)));
static u64 tls_values[workers];
static unsigned tids[workers], words[3], ready, finished, phase;
static unsigned order_word, order_ready, order_done, order_winner;
static unsigned legacy_word, legacy_ready, legacy_done, legacy_winner;

static void wait_for(unsigned *word, unsigned target) {
    for (;;) {
        unsigned seen = __atomic_load_n(word, __ATOMIC_SEQ_CST);
        if (seen >= target)
            return;
        i64 result = CALL(98, word, 128, seen);
        CHECK(result == 0 || result == -11, 10);
    }
}
static void worker(void *arg) {
    (void)arg;
    for (unsigned round = 0; round < rounds; ++round) {
        struct waiter list[2] = {{0, (u64)&words[0], private_u32, 0},
                                 {0, (u64)&words[1], private_u32, 0}};
        __atomic_fetch_add(&ready, 1, __ATOMIC_SEQ_CST);
        CALL(98, &ready, 129, workers);
        i64 result = call6(449, (u64)list, 2, 0, 0, 1, 0);
        CHECK(result == 1 || (round % 3 == 2 && result == -11), 11);
        __atomic_fetch_add(&finished, 1, __ATOMIC_SEQ_CST);
        CALL(98, &finished, 129, workers);
        wait_for(&phase, round + 1);
    }
}
static void join(unsigned index) {
    for (;;) {
        unsigned tid = __atomic_load_n(&tids[index], __ATOMIC_SEQ_CST);
        if (!tid)
            return;
        i64 result = CALL(98, &tids[index], 0, tid);
        CHECK(result == 0 || result == -11, 12);
    }
}
static void readonly_private_anonymous(void) {
    i64 mapping = call6(222, 0, 4096, 3, 0x22, (u64)-1, 0);
    CHECK(mapping > 0, 40);
    *(unsigned *)mapping = 0;
    CHECK(CALL(226, mapping, 4096, 1) == 0, 41);
    CHECK(call6(454, mapping, 0xffffffffu, 1, 2, 0, 0) == -14, 42);
    CHECK(call6(455, mapping, 1, 0xffffffffu, 2, 0, 0) == -14, 43);
    CHECK(call6(454, mapping, 0xffffffffu, 1, private_u32, 0, 0) == 0, 44);
    struct waiter list[2] = {{1, (u64)&words[0], private_u32, 0}, {0, (u64)mapping, 2, 0}};
    CHECK(call6(456, (u64)list, 0, 0, 1, 0, 0) == -14, 45);
    CHECK(CALL(215, mapping, 4096, 0) == 0, 46);
}
static void order_worker(void *arg) {
    unsigned id = (unsigned)(u64)arg;
    __atomic_fetch_add(&order_ready, 1, __ATOMIC_SEQ_CST);
    CALL(98, &order_ready, 129, workers);
    CHECK(call6(455, (u64)&order_word, 0, 0xffffffffu, private_u32, 0, 0) == 0, 47);
    unsigned expected = 0;
    __atomic_compare_exchange_n(&order_winner, &expected, id, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    __atomic_fetch_add(&order_done, 1, __ATOMIC_SEQ_CST);
    CALL(98, &order_done, 129, workers);
}
static void same_key_requeue_order(void) {
    struct waiter list[2] = {{0, (u64)&order_word, private_u32, 0},
                             {0, (u64)&order_word, private_u32, 0}};
    for (unsigned index = 0; index < 2; ++index) {
        CHECK(spawn_thread(order_worker, (void *)(u64)(index + 1), stacks[index] + sizeof(stacks[index]),
                           tls_values + index, tids + index) > 1, 48);
        wait_for(&order_ready, index + 1);
        unsigned attempts = 0;
        for (;;) {
            i64 count = call6(456, (u64)list, 0, 0, 2, 0, 0);
            CHECK(count >= 0 && count <= (i64)index + 1, 49);
            if (count == (i64)index + 1)
                break;
            CHECK(++attempts < 100000, 50);
            CALL(124, 0, 0, 0);
        }
    }
    CHECK(call6(456, (u64)list, 0, 0, 1, 0, 0) == 1, 51);
    CHECK(call6(454, (u64)&order_word, 0xffffffffu, 1, private_u32, 0, 0) == 1, 52);
    wait_for(&order_done, 1);
    CHECK(call6(454, (u64)&order_word, 0xffffffffu, 1, private_u32, 0, 0) == 1, 54);
    join(0);
    join(1);
    CHECK(__atomic_load_n(&order_winner, __ATOMIC_SEQ_CST) == 1, 53);
}
static void legacy_order_worker(void *arg) {
    unsigned id = (unsigned)(u64)arg;
    __atomic_fetch_add(&legacy_ready, 1, __ATOMIC_SEQ_CST);
    CALL(98, &legacy_ready, 129, workers);
    CHECK(CALL(98, &legacy_word, 128, 0) == 0, 60);
    unsigned expected = 0;
    __atomic_compare_exchange_n(&legacy_winner, &expected, id, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    __atomic_fetch_add(&legacy_done, 1, __ATOMIC_SEQ_CST);
    CALL(98, &legacy_done, 129, workers);
}
static void legacy_same_key_requeue_order(unsigned operation) {
    __atomic_store_n(&legacy_word, 0, __ATOMIC_SEQ_CST);
    __atomic_store_n(&legacy_ready, 0, __ATOMIC_SEQ_CST);
    __atomic_store_n(&legacy_done, 0, __ATOMIC_SEQ_CST);
    __atomic_store_n(&legacy_winner, 0, __ATOMIC_SEQ_CST);
    for (unsigned index = 0; index < 2; ++index) {
        CHECK(spawn_thread(legacy_order_worker, (void *)(u64)(index + 1), stacks[index] + sizeof(stacks[index]),
                           tls_values + index, tids + index) > 1, 61);
        wait_for(&legacy_ready, index + 1);
        unsigned attempts = 0;
        for (;;) {
            i64 count = call6(98, (u64)&legacy_word, operation | 128, 0, 2, (u64)&legacy_word, 0);
            CHECK(count >= 0 && count <= (i64)index + 1, 62);
            if (count == (i64)index + 1)
                break;
            CHECK(++attempts < 100000, 63);
            CALL(124, 0, 0, 0);
        }
    }
    if (operation == 4)
        CHECK(call6(98, (u64)&legacy_word, operation | 128, 0, 1, (u64)&legacy_word, 1) == -11, 64);
    CHECK(call6(98, (u64)&legacy_word, operation | 128, 0, 1, (u64)&legacy_word, 0) == 1, 65);
    CHECK(CALL(98, &legacy_word, 129, 1) == 1, 66);
    wait_for(&legacy_done, 1);
    CHECK(CALL(98, &legacy_word, 129, 1) == 1, 67);
    join(0);
    join(1);
    CHECK(__atomic_load_n(&legacy_winner, __ATOMIC_SEQ_CST) == 1, 68);
}
int guest_main(u64 *stack) {
    if (stack[0] > 1 && equal((const char *)stack[2], "readonly")) {
        readonly_private_anonymous();
        print("futex2-readonly-ok\n");
        return 0;
    }
    if (stack[0] > 1 && equal((const char *)stack[2], "same-key")) {
        same_key_requeue_order();
        print("futex2-same-key-ok\n");
        return 0;
    }
    if (stack[0] > 1 && equal((const char *)stack[2], "legacy-requeue")) {
        legacy_same_key_requeue_order(3);
        print("futex-legacy-requeue-ok\n");
        return 0;
    }
    if (stack[0] > 1 && equal((const char *)stack[2], "legacy-cmp-requeue")) {
        legacy_same_key_requeue_order(4);
        print("futex-legacy-cmp-requeue-ok\n");
        return 0;
    }
    readonly_private_anonymous();
    same_key_requeue_order();
    legacy_same_key_requeue_order(3);
    legacy_same_key_requeue_order(4);
    struct waiter invalid = {0, (u64)&words[0], 0, 0};
    i64 past[2] = {0, 0};
    CHECK(call6(449, (u64)&invalid, 1, 0, 0, 1, 0) == -22, 20);
    CHECK(call6(455, (u64)&words[0], 0, 0, private_u32, 0, 1) == -22, 21);
    CHECK(call6(455, (u64)&words[0], 1, 1, private_u32, 0, 1) == -11, 22);
    CHECK(call6(455, (u64)&words[0], 0, 1, private_u32, (u64)past, 1) == -110, 23);
    for (unsigned index = 0; index < workers; ++index)
        CHECK(spawn_thread(worker, 0, stacks[index] + sizeof(stacks[index]), tls_values + index,
                           tids + index) > 1, 24);

    for (unsigned round = 0; round < rounds; ++round) {
        const unsigned target = (round + 1) * workers;
        wait_for(&ready, target);
        if (round % 3 == 0) {
            unsigned attempts = 0;
            while (__atomic_load_n(&finished, __ATOMIC_SEQ_CST) < target) {
                i64 count = call6(454, (u64)&words[1], 0xffffffffu, workers, private_u32, 0, 0);
                CHECK(count >= 0 && count <= workers, 25);
                CHECK(++attempts < 100000, 26);
                CALL(124, 0, 0, 0);
            }
        } else if (round % 3 == 1) {
            struct waiter list[2] = {{0, (u64)&words[1], private_u32, 0},
                                     {0, (u64)&words[2], private_u32, 0}};
            unsigned moved = 0, attempts = 0;
            while (moved < workers) {
                i64 count = call6(456, (u64)list, 0, 0, workers - moved, 0, 0);
                CHECK(count >= 0 && count <= workers - moved, 27);
                moved += (unsigned)count;
                CHECK(++attempts < 100000, 28);
                if (!count)
                    CALL(124, 0, 0, 0);
            }
            CHECK(CALL(98, &words[2], 129, workers) == workers, 29);
        } else {
            __atomic_store_n(&words[0], 1, __ATOMIC_SEQ_CST);
            i64 count = call6(454, (u64)&words[1], 0xffffffffu, workers, private_u32, 0, 0);
            CHECK(count >= 0 && count <= workers, 30);
        }
        wait_for(&finished, target);
        __atomic_store_n(&words[0], 0, __ATOMIC_SEQ_CST);
        __atomic_store_n(&phase, round + 1, __ATOMIC_SEQ_CST);
        CALL(98, &phase, 129, workers);
    }
    for (unsigned index = 0; index < workers; ++index)
        join(index);
    print("futex2-multicore-waitv-wake-requeue-race-ok\n");
    return 0;
}
