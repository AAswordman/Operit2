#include "linux.h"

struct time_value { i64 seconds, part; };
struct timer_value { struct time_value interval, value; };
struct signal_event { u64 value; int signal, notify; unsigned char padding[48]; };
struct signal_info { int signal, error, code, padding; unsigned char payload[112]; };
_Static_assert(sizeof(struct signal_event) == 64, "sigevent ABI");
_Static_assert(sizeof(struct signal_info) == 128, "siginfo ABI");

static struct signal_info wait_signal(int signal, int check) {
    const u64 mask = 1ul << (signal - 1);
    struct signal_info info = {0};
    struct time_value timeout = {1, 0};
    CHECK(call6(137, (u64)&mask, (u64)&info, (u64)&timeout, 8, 0, 0) == signal, check);
    CHECK(info.signal == signal, check + 1);
    return info;
}

int guest_main(u64 *stack) {
    (void)stack;
    u64 blocked = (1ul << 9) | (1ul << 13) | (1ul << 25) | (1ul << 26);
    CHECK(call6(135, 0, (u64)&blocked, 0, 8, 0, 0) == 0, 10);
    struct signal_event event = {0x12345678abcdef01ul, 10, 0, {0}};
    int id = -1;
    CHECK(CALL(107, 1, &event, &id) == 0 && id >= 0, 11);
    struct timer_value spec = {{0, 0}, {0, 15000000}}, previous = {{1, 1}, {1, 1}};
    CHECK(call6(110, id, 0, (u64)&spec, (u64)&previous, 0, 0) == 0, 12);
    CHECK(previous.interval.seconds == 0 && previous.value.seconds == 0 &&
          previous.interval.part == 0 && previous.value.part == 0, 13);
    struct signal_info info = wait_signal(10, 14);
    CHECK(info.code == -2 && *(u64 *)(info.payload + 8) == event.value, 16);
    CHECK(CALL(109, id, 0, 0) >= 0, 17);
    CHECK(CALL(108, id, &previous, 0) == 0 && previous.value.seconds == 0 &&
          previous.value.part == 0, 18);
    CHECK(CALL(111, id, 0, 0) == 0 && CALL(108, id, &previous, 0) == -22, 19);

    struct timer_value interval = {{0, 0}, {0, 15000}};
    CHECK(CALL(103, 0, &interval, &previous) == 0, 20);
    info = wait_signal(14, 21);
    CHECK(info.code == 128, 23);
    CHECK(CALL(102, 0, &previous, 0) == 0 && previous.value.seconds == 0 &&
          previous.value.part == 0, 24);

    CHECK(CALL(107, 2, &event, &id) == 0, 25);
    spec.value.part = 2000000;
    CHECK(call6(110, id, 0, (u64)&spec, 0, 0, 0) == 0, 26);
    struct time_value start = {0, 0}, now = {0, 0};
    CHECK(CALL(113, 2, &start, 0) == 0, 27);
    volatile u64 checksum = 1;
    do {
        for (unsigned i = 0; i < 1000; ++i)
            checksum = checksum * 6364136223846793005ul + 1442695040888963407ul;
        CHECK(CALL(113, 2, &now, 0) == 0, 28);
    } while ((now.seconds - start.seconds) * 1000000000 + now.part - start.part < 6000000);
    info = wait_signal(10, 29);
    CHECK(info.code == -2 && *(u64 *)(info.payload + 8) == event.value && checksum != 0, 31);
    CHECK(CALL(111, id, 0, 0) == 0, 32);
    print("process-timers-signals-cpu-ok\n");
    return 0;
}
