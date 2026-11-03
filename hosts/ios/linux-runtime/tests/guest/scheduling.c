#include "linux.h"
struct scheduling_attributes {
    unsigned size, policy;
    u64 flags;
    int nice;
    unsigned priority;
    u64 runtime, deadline, period;
    unsigned utilization_min, utilization_max;
};
typedef char scheduling_abi_size[(sizeof(struct scheduling_attributes) == 56) ? 1 : -1];
struct future_attributes {
    struct scheduling_attributes value;
    unsigned char extra[16];
};
int guest_main(u64 *stack) {
    (void)stack;
    struct scheduling_attributes attributes = {0};
    CHECK(call6(275, 0, (u64)&attributes, 56, 0, 0, 0) == 0, 10);
    CHECK(attributes.size == 56 && attributes.policy == 0 && attributes.nice == 0 &&
              attributes.flags == 0 && attributes.priority == 0, 11);
    for (unsigned byte = 0; byte != 56; ++byte)
        ((unsigned char *)&attributes)[byte] = 0x5a;
    CHECK(call6(275, 0, (u64)&attributes, 48, 0, 0, 0) == 0 && attributes.size == 48, 12);
    CHECK(attributes.utilization_min == 0x5a5a5a5a && attributes.utilization_max == 0x5a5a5a5a, 13);
    attributes = (struct scheduling_attributes){0, 3, 0, 3, 0, 0, 0, 0, 0, 0};
    CHECK(CALL(274, 0, &attributes, 0) == 0 && CALL(141, 0, 0, 0) == 17, 14);
    attributes.size = 56;
    attributes.nice = -1;
    CHECK(CALL(274, 0, &attributes, 0) == -1 && CALL(141, 0, 0, 0) == 17, 15);
    attributes.policy = 1;
    attributes.priority = 1;
    CHECK(CALL(274, 0, &attributes, 0) == -1 && CALL(120, 0, 0, 0) == 3, 16);
    attributes = (struct scheduling_attributes){56, 0, 128, 4, 0, 0, 0, 0, 0, 0};
    CHECK(CALL(274, 0, &attributes, 0) == -22, 17);
    attributes.flags = 0;
    attributes.runtime = 1;
    CHECK(CALL(274, 0, &attributes, 0) == -95, 18);
    attributes.policy = 6;
    attributes.deadline = attributes.period = 2;
    CHECK(CALL(274, 0, &attributes, 0) == -95, 19);
    attributes = (struct scheduling_attributes){48, 0, 32, 4, 0, 0, 0, 0, 0, 0};
    CHECK(CALL(274, 0, &attributes, 0) == -22, 20);
    attributes.size = 56;
    CHECK(CALL(274, 0, &attributes, 0) == -95, 21);
    struct future_attributes future = {{72, 3, 0, 4, 0, 0, 0, 0, 0, 0}, {0}};
    CHECK(CALL(274, 0, &future, 0) == 0 && CALL(141, 0, 0, 0) == 16, 22);
    future.extra[15] = 1;
    CHECK(CALL(274, 0, &future, 0) == -7 && future.value.size == 56, 23);
    for (unsigned byte = 0; byte != 16; ++byte)
        future.extra[byte] = 0x5a;
    CHECK(call6(275, 0, (u64)&future, 72, 0, 0, 0) == 0 && future.value.size == 56, 24);
    for (unsigned byte = 0; byte != 16; ++byte)
        CHECK(future.extra[byte] == 0x5a, 25);
    attributes = (struct scheduling_attributes){56, 3, 1, 5, 0, 0, 0, 0, 0, 0};
    CHECK(CALL(274, 0, &attributes, 0) == 0, 26);
    attributes.policy = 999;
    attributes.nice = 6;
    attributes.flags = 8;
    CHECK(CALL(274, 0, &attributes, 0) == 0 && CALL(120, 0, 0, 0) == (3 | 0x40000000), 27);
    attributes = (struct scheduling_attributes){56, 0, 17, -20, 99, 1, 0, 0, 0, 0};
    CHECK(CALL(274, 0, &attributes, 0) == 0 && CALL(141, 0, 0, 0) == 14, 28);
    attributes = (struct scheduling_attributes){56, 0, 0, 7, 0, 0, 0, 0, 0, 0};
    CHECK(CALL(274, 0, &attributes, 0) == -1 && CALL(141, 0, 0, 0) == 14, 29);
    CHECK(call6(275, 0, (u64)&attributes, 47, 0, 0, 0) == -22 &&
              call6(275, 0, (u64)&attributes, 56, 1, 0, 0) == -22 &&
              call6(275, 0, 0, 56, 0, 0, 0) == -22, 30);
    i64 interval[2] = {-1, -1};
    CHECK(CALL(127, 0, interval, 0) == 0 && interval[0] == 0 && interval[1] == 0, 31);
    CHECK(CALL(127, -1, interval, 0) == -22 && CALL(127, 0, 0, 0) == -14, 32);
    CHECK(CALL(124, 0, 0, 0) == 0, 33);
    struct { u64 soft, hard; } realtime_limit = {1234, 5678}, old_realtime_limit = {0};
    CHECK(call6(261, 0, 15, (u64)&realtime_limit, (u64)&old_realtime_limit, 0, 0) == 0 &&
              old_realtime_limit.soft == (u64)-1 && old_realtime_limit.hard == (u64)-1, 34);
    CHECK(call6(261, 0, 15, 0, (u64)&old_realtime_limit, 0, 0) == 0 &&
              old_realtime_limit.soft == 1234 && old_realtime_limit.hard == 5678, 35);
    realtime_limit.hard = 5679;
    CHECK(call6(261, 0, 15, (u64)&realtime_limit, 0, 0, 0) == -1, 36);
    print("scheduling-attributes-limits-versioning-ok\n");
    return 0;
}
