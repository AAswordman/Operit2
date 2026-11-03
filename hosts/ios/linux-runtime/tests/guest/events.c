#include "linux.h"

struct timespec64 {
    i64 seconds, nanoseconds;
};
struct epoll_event64 {
    unsigned events, padding;
    u64 data;
};

int guest_main(u64 *stack) {
    (void)stack;
    const int epoll = CALL(20, 524288, 0, 0);
    const int counter = CALL(19, 1, 2048, 0);
    CHECK(epoll >= 0 && counter >= 0, 10);
    struct epoll_event64 event = {1, 0, 0x1122334455667788UL}, result = {0};
    CHECK(call6(21, epoll, 1, counter, (u64)&event, 0, 0) == 0, 11);
    const i64 page = call6(222, 0, 4096, 3, 34, (u64)-1, 0);
    CHECK(page > 0, 12);
    struct timespec64 *timeout = (void *)(u64)page;
    timeout->seconds = timeout->nanoseconds = 0;
    CHECK(CALL(226, page, 4096, 1) == 0, 13);
    const u64 mask = 1UL << 9;
    u64 before = 0, after = 0;
    CHECK(call6(135, 2, 0, (u64)&before, 8, 0, 0) == 0, 14);
    CHECK(call6(441, epoll, (u64)&result, 1, page, (u64)&mask, 8) == 1, 15);
    CHECK(result.events == 1 && !result.padding && result.data == event.data, 16);
    CHECK(call6(135, 2, 0, (u64)&after, 8, 0, 0) == 0 && after == before, 17);
    u64 value = 0;
    CHECK(CALL(63, counter, &value, 8) == 8 && value == 1, 18);
    CHECK(call6(441, epoll, (u64)&result, 1, page, 0, 8) == 0, 19);
    CHECK(CALL(226, page, 4096, 3) == 0, 20);
    timeout->nanoseconds = 1000000000;
    CHECK(call6(441, epoll, (u64)&result, 1, page, 0, 8) == -22, 21);
    timeout->seconds = -1;
    timeout->nanoseconds = 0;
    CHECK(call6(441, epoll, (u64)&result, 1, page, 0, 8) == -22, 22);
    const int timer = CALL(85, 1, 2048 | 524288, 0);
    CHECK(timer >= 0, 23);
    event.data = 0xabcdef;
    CHECK(call6(21, epoll, 1, timer, (u64)&event, 0, 0) == 0, 24);
    const struct timespec64 expiration[2] = {{0, 0}, {0, 5000000}};
    CHECK(call6(86, timer, 0, (u64)expiration, 0, 0, 0) == 0, 25);
    timeout->seconds = 1;
    timeout->nanoseconds = 234567;
    CHECK(call6(441, epoll, (u64)&result, 1, page, (u64)&mask, 8) == 1, 26);
    CHECK(result.events == 1 && result.data == event.data, 27);
    CHECK(CALL(63, timer, &value, 8) == 8 && value >= 1, 28);
    CHECK(call6(135, 2, 0, (u64)&after, 8, 0, 0) == 0 && after == before, 29);
    CHECK(timeout->seconds == 1 && timeout->nanoseconds == 234567, 30);
    CHECK(CALL(57, timer, 0, 0) == 0 && CALL(57, counter, 0, 0) == 0 &&
              CALL(57, epoll, 0, 0) == 0 && CALL(215, page, 4096, 0) == 0, 31);
    print("epoll-pwait2-timespec-timer-mask-ok\n");
    return 0;
}
