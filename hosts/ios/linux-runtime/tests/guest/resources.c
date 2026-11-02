#include "linux.h"

struct limit { u64 soft, hard; };
struct time_value { i64 seconds, part; };
struct signal_info { int signal, error, code, padding; unsigned char payload[112]; };
struct signal_event { u64 value; int signal, notify; unsigned char padding[48]; };

static struct limit change_soft(unsigned resource, u64 value, int check) {
    struct limit old = {0, 0}, next;
    CHECK(call6(261, 0, resource, 0, (u64)&old, 0, 0) == 0, check);
    next.soft = value;
    next.hard = old.hard;
    CHECK(call6(261, 0, resource, (u64)&next, 0, 0, 0) == 0, check + 1);
    return old;
}
static void restore(unsigned resource, const struct limit *limit, int check) {
    CHECK(call6(261, 0, resource, (u64)limit, 0, 0, 0) == 0, check);
}
static void consume(int signal, int check) {
    const u64 mask = 1ul << (signal - 1);
    struct signal_info info = {0};
    const struct time_value timeout = {1, 0};
    CHECK(call6(137, (u64)&mask, (u64)&info, (u64)&timeout, 8, 0, 0) == signal, check);
    CHECK(info.signal == signal, check + 1);
}

int guest_main(u64 *stack) {
    if (stack[0] > 1 && equal((const char *)stack[2], "cpu")) {
        const struct limit cpu = {1, 1};
        CHECK(call6(261, 0, 0, (u64)&cpu, 0, 0, 0) == 0, 10);
        volatile u64 checksum = 1;
        for (;;)
            checksum = checksum * 6364136223846793005ul + 1442695040888963407ul;
    }
    const u64 blocked = (1ul << 24) | (1ul << 33);
    CHECK(call6(135, 0, (u64)&blocked, 0, 8, 0, 0) == 0, 11);
    const struct limit size_limit = change_soft(1, 8, 12);
    const int file = call6(56, -100, (u64)"/data/limited-file", 2 | 64 | 512, 0600, 0, 0);
    CHECK(file >= 0, 14);
    CHECK(CALL(64, file, "twelve-bytes", 12) == 8, 15);
    CHECK(CALL(64, file, "x", 1) == -27, 16);
    consume(25, 17);
    CHECK(CALL(57, file, 0, 0) == 0, 19);
    restore(1, &size_limit, 20);
    CHECK(CALL(35, -100, "/data/limited-file", 0) == 0, 21);

    // Linux exempts real UID 0 from NPROC; ordinary users must be charged.
    if (CALL(174, 0, 0, 0) != 0) {
        const struct limit process_limit = change_soft(6, 0, 22);
        CHECK(CALL(220, 17, 0, 0) == -11, 24);
        restore(6, &process_limit, 25);
    }
    const struct limit signal_limit = change_soft(11, 1, 26);
    const int pid = CALL(172, 0, 0, 0);
    struct signal_info queued = {34, 0, -1, 0, {0}};
    *(unsigned *)queued.payload = pid;
    *(unsigned *)(queued.payload + 4) = CALL(174, 0, 0, 0);
    CHECK(CALL(138, pid, 34, &queued) == 0, 28);
    CHECK(CALL(138, pid, 34, &queued) == -11, 29);
    consume(34, 30);

    // Even SIGEV_NONE reserves one signal-queue slot at timer_create.
    const struct signal_event event = {0, 10, 1, {0}};
    int timer = -1;
    CHECK(CALL(107, 1, &event, &timer) == 0 && timer >= 0, 32);
    CHECK(CALL(138, pid, 34, &queued) == -11, 33);
    CHECK(CALL(111, timer, 0, 0) == 0, 34);
    CHECK(CALL(138, pid, 34, &queued) == 0, 35);
    consume(34, 36);
    change_soft(11, 0, 38);
    CHECK(CALL(107, 1, &event, &timer) == -11, 40);
    restore(11, &signal_limit, 41);
    print("resource-filesize-nproc-sigpending-ok\n");
    return 0;
}
