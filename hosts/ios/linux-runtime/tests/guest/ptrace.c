#include "linux.h"

struct iovec64 { u64 base, length; };
struct user_pt_regs { u64 x[31], sp, pc, pstate; };
struct siginfo64 { int signal, error, code, padding; unsigned char payload[112]; };
static volatile u64 marker = 0x1122334455667788UL;
__attribute__((noinline)) static int breakpoint_target(void) { return 1; }
static int (*volatile breakpoint_call)(void) = breakpoint_target;

static i64 trace(u64 request, u64 tid, u64 address, u64 data) {
    return call6(117, request, tid, address, data, 0, 0);
}
static i64 wait_child(u64 tid, int *status) {
    return call6(260, tid, (u64)status, 0, 0, 0, 0);
}
static void get_regs(u64 tid, struct user_pt_regs *regs) {
    struct iovec64 iov = {(u64)regs, sizeof(*regs)};
    CHECK(trace(0x4204, tid, 1, (u64)&iov) == 0 && iov.length == sizeof(*regs), 30);
}
static void set_regs(u64 tid, struct user_pt_regs *regs) {
    struct iovec64 iov = {(u64)regs, sizeof(*regs)};
    CHECK(trace(0x4205, tid, 1, (u64)&iov) == 0 && iov.length == sizeof(*regs), 31);
}
static i64 traced_getpid(u64 *saved_x7, u64 *saved_x8) {
    register u64 x0 __asm__("x0") = 0;
    register u64 x7 __asm__("x7") = 0x12345678;
    register u64 x8 __asm__("x8") = 172;
    __asm__ volatile("svc #0" : "+r"(x0), "+r"(x7), "+r"(x8) : : "memory", "cc");
    *saved_x7 = x7;
    *saved_x8 = x8;
    return (i64)x0;
}

int guest_main(u64 *stack) {
    (void)stack;
    const i64 child = call6(220, 17, 0, 0, 0, 0, 0);
    CHECK(child >= 0, 10);
    if (!child) {
        CHECK(breakpoint_call() == 1, 28);
        CHECK(trace(0, 0, 0, 0) == 0, 11);
        const i64 self = CALL(172, 0, 0, 0);
        CHECK(CALL(129, self, 19, 0) == 0, 12);
        u64 saved_x7 = 0, saved_x8 = 0;
        const i64 result = traced_getpid(&saved_x7, &saved_x8);
        CHECK(result == 0x4242, 13);
        CHECK(saved_x7 == 0x12345678, 35);
        CHECK(saved_x8 == 173, 36);
        CHECK(marker == 0x8877665544332211UL, 14);
        CHECK(breakpoint_call() == 42, 29);
        fail(0);
    }
    int status = 0;
    CHECK(wait_child(child, &status) == child && status == ((19 << 8) | 0x7f), 15);
    struct user_pt_regs regs;
    get_regs(child, &regs);
    const u64 original = regs.x[28];
    regs.x[28] = 0x12345678;
    set_regs(child, &regs);
    get_regs(child, &regs);
    CHECK(regs.x[28] == 0x12345678, 16);
    regs.x[28] = original;
    set_regs(child, &regs);
    u64 word = 0;
    CHECK(trace(2, child, (u64)&marker, (u64)&word) == 0 && word == marker, 17);
    CHECK(trace(5, child, (u64)&marker, 0x8877665544332211UL) == 0, 18);
    CHECK(trace(4, child, (u64)breakpoint_target, 0xd65f03c052800540UL) == 0, 32);
    CHECK(breakpoint_call() == 1, 33);
    CHECK(trace(0x4200, child, 0, 1) == 0, 19);
    CHECK(trace(24, child, 0, 0) == 0, 20);
    CHECK(wait_child(child, &status) == child && status == (((5 | 0x80) << 8) | 0x7f), 21);
    struct siginfo64 info;
    CHECK(trace(0x4202, child, 0, (u64)&info) == 0 && info.signal == 5 && info.code == (5 | 0x80), 34);
    get_regs(child, &regs);
    CHECK(regs.x[8] == 172 && regs.x[7] == 0, 22);
    regs.x[7] = 0x1111;
    regs.x[8] = 173;
    set_regs(child, &regs);
    CHECK(trace(24, child, 0, 0) == 0, 23);
    CHECK(wait_child(child, &status) == child && status == (((5 | 0x80) << 8) | 0x7f), 24);
    get_regs(child, &regs);
    CHECK(regs.x[0] == (u64)child && regs.x[7] == 1 && regs.x[8] == 173, 25);
    regs.x[7] = 0x2222;
    regs.x[0] = 0x4242;
    set_regs(child, &regs);
    CHECK(trace(7, child, 0, 0) == 0, 26);
    CHECK(wait_child(child, &status) == child && status == 0, 27);

    marker = 0x11;
    const i64 attached = call6(220, 17, 0, 0, 0, 0, 0);
    CHECK(attached >= 0, 40);
    if (!attached) {
        while (marker != 0x22)
            (void)CALL(172, 0, 0, 0);
        fail(0);
    }
    CHECK(trace(16, attached, 0, 0) == 0, 41);
    CHECK(wait_child(attached, &status) == attached && status == ((19 << 8) | 0x7f), 42);
    CHECK(trace(5, attached, (u64)&marker, 0x22) == 0, 43);
    CHECK(trace(17, attached, 0, 0) == 0, 44);
    CHECK(wait_child(attached, &status) == attached && status == 0, 45);
    print("guest ptrace traceme/attach/registers/memory/syscall ok\n");
    return 0;
}
