#include "linux.h"

typedef unsigned short u16;
typedef unsigned char u8;
struct instruction { u16 code; u8 jt, jf; unsigned k; };
struct program { u16 length; const struct instruction *filter; };
struct sigaction64 { u64 handler, flags, restorer, mask; };
struct sigsys_info { int signal, error, code, padding; u64 address; unsigned syscall, arch; u8 rest[96]; };
_Static_assert(sizeof(struct program) == 16, "sock_fprog ABI");
_Static_assert(sizeof(struct sigsys_info) == 128, "siginfo ABI");

#define RET_ALLOW 0x7fff0000u
#define RET_ERRNO 0x00050000u
#define RET_TRAP 0x00030000u
#define RET_KILL_PROCESS 0x80000000u
#define RET_KILL_THREAD 0u
#define STMT(op, k) {op, 0, 0, k}
#define JUMP(op, k, jt, jf) {op, jt, jf, k}

extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);
static unsigned char __attribute__((aligned(16))) stacks[3][8192];
static unsigned tids[3];
static u64 tls_values[3];
static unsigned phase;
static i64 thread_values[5];
static unsigned sync_ready, sync_stage;
static i64 sync_values[16];
static volatile unsigned bad_kill_thread;
static volatile struct sigsys_info trapped;

static i64 nnp(void) { return CALL(167, 38, 1, 0); }
static i64 install(struct instruction *filter, u16 size) {
    const struct program program = {size, filter};
    return CALL(277, 1, 0, &program);
}
static i64 install_number_flags(unsigned number, unsigned action, unsigned flags) {
    struct instruction filter[] = {
        STMT(0x20, 0), JUMP(0x15, number, 0, 1), STMT(0x06, action), STMT(0x06, RET_ALLOW)
    };
    const struct program program = {4, filter};
    return CALL(277, 1, flags, &program);
}
static i64 install_number(unsigned number, unsigned action) { return install_number_flags(number, action, 0); }
static i64 prohibited_open(void) {
    return call6(56, (u64)-100, (u64)"/seccomp-escape", 65, 0600, 0, 0);
}
static void wait_thread(unsigned index) {
    while (__atomic_load_n(&tids[index], __ATOMIC_SEQ_CST))
        CALL(124, 0, 0, 0);
}
static i64 fork_process(void) { return call6(220, 17, 0, 0, 0, 0, 0); }
static int wait_child(i64 child) {
    int status = -1;
    CHECK(call6(260, child, (u64)&status, 0, 0, 0, 0) == child, 20);
    return status;
}
static __attribute__((noreturn)) void exit_group(int code) {
    CALL(94, code, 0, 0);
    __builtin_unreachable();
}

static int basic(void) {
    struct instruction rule[] = {
        STMT(0x20, 0), JUMP(0x15, 56, 0, 1), STMT(0x06, RET_ERRNO | 13), STMT(0x06, RET_ALLOW)
    };
    const struct program program = {4, rule};
    CHECK(CALL(167, 39, 0, 0) == 0, 21);
    CHECK(CALL(277, 1, 0, &program) == -13, 22);
    CHECK(call6(167, 38, 1, 0, 0, 1, 0) == -22, 23);
    CHECK(nnp() == 0 && CALL(167, 39, 0, 0) == 1, 24);
    CHECK(CALL(277, 1, 2, &program) == -95 && CALL(277, 1, 8, &program) == -95, 25);
    CHECK(CALL(277, 1, 0, 0) == -14, 26);
    struct instruction arch[] = {
        STMT(0x20, 4), JUMP(0x15, 0xc00000b7u, 1, 0), STMT(0x06, RET_KILL_PROCESS),
        STMT(0x06, RET_ALLOW)
    };
    CHECK(install(arch, 4) == 0 && install(rule, 4) == 0, 27);
    rule[2].k = RET_ALLOW;
    CHECK(prohibited_open() == -13, 28);
    CHECK(install_number(999999, RET_ERRNO | 42) == 0 && CALL(999999, 0, 0, 0) == -42, 29);
    CHECK(CALL(167, 38, 0, 0) == -22 && CALL(167, 39, 0, 0) == 1, 30);
    struct instruction invalid[] = {STMT(0x20, 63), STMT(0x06, RET_ALLOW)};
    CHECK(install(invalid, 2) == -22 && prohibited_open() == -13, 31);
    i64 child = fork_process();
    CHECK(child >= 0, 32);
    if (!child) {
        if (CALL(167, 39, 0, 0) != 1 || prohibited_open() != -13 ||
            install_number(172, RET_ERRNO | 44) != 0 || CALL(172, 0, 0, 0) != -44)
            exit_group(33);
        exit_group(0);
    }
    CHECK(wait_child(child) == 0 && CALL(172, 0, 0, 0) > 0 && prohibited_open() == -13, 34);
    print("seccomp-basic-fork-ok\n");
    return 0;
}

static void thread_before(void *unused) {
    (void)unused;
    while (__atomic_load_n(&phase, __ATOMIC_SEQ_CST) < 1)
        CALL(124, 0, 0, 0);
    thread_values[0] = CALL(173, 0, 0, 0);
    thread_values[1] = CALL(174, 0, 0, 0);
    thread_values[2] = install_number(172, RET_ERRNO | 37);
    thread_values[3] = CALL(172, 0, 0, 0);
    __atomic_store_n(&phase, 2, __ATOMIC_SEQ_CST);
}
static void thread_after(void *unused) {
    (void)unused;
    thread_values[4] = CALL(174, 0, 0, 0);
}
static int thread_test(void) {
    CHECK(nnp() == 0 && install_number(173, RET_ERRNO | 13) == 0, 40);
    CHECK(spawn_thread(thread_before, 0, stacks[0] + sizeof(stacks[0]), tls_values, tids) > 1, 41);
    CHECK(install_number(174, RET_ERRNO | 41) == 0, 42);
    __atomic_store_n(&phase, 1, __ATOMIC_SEQ_CST);
    while (__atomic_load_n(&phase, __ATOMIC_SEQ_CST) < 2)
        CALL(124, 0, 0, 0);
    wait_thread(0);
    CHECK(thread_values[0] == -13 && thread_values[1] >= 0 && thread_values[2] == 0 &&
          thread_values[3] == -37, 43);
    CHECK(CALL(172, 0, 0, 0) > 0 && CALL(173, 0, 0, 0) == -13 && CALL(174, 0, 0, 0) == -41, 44);
    CHECK(spawn_thread(thread_after, 0, stacks[1] + sizeof(stacks[1]), tls_values + 1, tids + 1) > 1, 45);
    wait_thread(1);
    CHECK(thread_values[4] == -41, 46);
    print("seccomp-thread-inheritance-ok\n");
    return 0;
}

static void wait_sync_ready(unsigned mask) {
    while ((__atomic_load_n(&sync_ready, __ATOMIC_SEQ_CST) & mask) != mask)
        CALL(124, 0, 0, 0);
}
static void wait_sync_stage(unsigned stage) {
    while (__atomic_load_n(&sync_stage, __ATOMIC_SEQ_CST) < stage)
        CALL(124, 0, 0, 0);
}
static void sync_ready_bits(unsigned mask) { __atomic_fetch_or(&sync_ready, mask, __ATOMIC_SEQ_CST); }
static void sync_advance(unsigned stage) { __atomic_store_n(&sync_stage, stage, __ATOMIC_SEQ_CST); }

static void sync_before_nnp(void *unused) {
    (void)unused;
    sync_values[0] = CALL(167, 39, 0, 0);
    sync_ready_bits(1);
    wait_sync_stage(1);
    sync_values[1] = CALL(167, 39, 0, 0);
    sync_values[2] = CALL(167, 21, 0, 0);
    sync_ready_bits(2);
    wait_sync_stage(2);
    sync_values[3] = CALL(167, 39, 0, 0);
    sync_values[4] = CALL(172, 0, 0, 0);
    sync_values[5] = CALL(174, 0, 0, 0);
    sync_values[6] = CALL(167, 21, 0, 0);
    for (unsigned i = 0; i < 64; ++i)
        if (CALL(172, 0, 0, 0) != -41 || CALL(174, 0, 0, 0) != -42)
            sync_values[7] = 1;
    sync_ready_bits(4);
}
static void sync_after_base(void *unused) {
    (void)unused;
    sync_values[8] = CALL(172, 0, 0, 0);
    sync_ready_bits(8);
    wait_sync_stage(2);
    sync_values[9] = CALL(167, 39, 0, 0);
    sync_values[10] = CALL(172, 0, 0, 0);
    sync_values[11] = CALL(174, 0, 0, 0);
    sync_values[12] = CALL(167, 21, 0, 0);
    for (unsigned i = 0; i < 64; ++i)
        if (CALL(172, 0, 0, 0) != -41 || CALL(174, 0, 0, 0) != -42)
            sync_values[13] = 1;
    sync_ready_bits(16);
}
static void sync_new_thread(void *unused) {
    (void)unused;
    sync_values[14] = CALL(172, 0, 0, 0);
    sync_values[15] = CALL(174, 0, 0, 0);
}
static int sync_exec_child(void) {
    CHECK(CALL(167, 39, 0, 0) == 1 && CALL(167, 21, 0, 0) == 2 &&
          CALL(172, 0, 0, 0) == -41 && CALL(174, 0, 0, 0) == -42, 100);
    return 0;
}
static int sync_test(void) {
    CHECK(spawn_thread(sync_before_nnp, 0, stacks[0] + sizeof(stacks[0]), tls_values, tids) > 1, 101);
    wait_sync_ready(1);
    CHECK(install_number_flags(172, RET_ERRNO | 41, 1) == -13 && nnp() == 0, 102);
    struct instruction invalid[] = {STMT(0xffff, 0), STMT(0x06, RET_ALLOW)};
    const struct program bad = {2, invalid};
    CHECK(CALL(277, 1, 1, &bad) == -22 && CALL(277, 1, 1, 0) == -14, 103);
    sync_advance(1);
    wait_sync_ready(2);
    CHECK(sync_values[0] == 0 && sync_values[1] == 0 && sync_values[2] == 0, 104);
    CHECK(install_number(172, RET_ERRNO | 41) == 0, 105);
    CHECK(spawn_thread(sync_after_base, 0, stacks[1] + sizeof(stacks[1]), tls_values + 1, tids + 1) > 1, 106);
    wait_sync_ready(8);
    CHECK(sync_values[8] == -41 && install_number_flags(174, RET_ERRNO | 42, 1) == 0, 107);
    sync_advance(2);
    wait_sync_ready(4 | 16);
    wait_thread(0);
    wait_thread(1);
    CHECK(sync_values[3] == 1 && sync_values[4] == -41 && sync_values[5] == -42 &&
          sync_values[6] == 2 && sync_values[7] == 0 && sync_values[9] == 1 &&
          sync_values[10] == -41 && sync_values[11] == -42 && sync_values[12] == 2 &&
          sync_values[13] == 0, 108);
    CHECK(spawn_thread(sync_new_thread, 0, stacks[2] + sizeof(stacks[2]), tls_values + 2, tids + 2) > 1, 109);
    wait_thread(2);
    CHECK(sync_values[14] == -41 && sync_values[15] == -42, 110);
    const i64 child = fork_process();
    CHECK(child >= 0, 111);
    if (!child) {
        if (CALL(167, 39, 0, 0) != 1 || CALL(172, 0, 0, 0) != -41 || CALL(174, 0, 0, 0) != -42)
            exit_group(112);
        const char *args[] = {"/seccomp", "sync-exec-child", 0};
        const char *env[] = {0};
        CALL(221, "/seccomp", args, env);
        exit_group(113);
    }
    CHECK(wait_child(child) == 0, 114);
    print("seccomp-tsync-propagation-ok\n");
    return 0;
}

static void sync_divergent(void *unused) {
    (void)unused;
    sync_values[0] = install_number(172, RET_ERRNO | 37);
    sync_ready_bits(1);
    wait_sync_stage(1);
    sync_values[1] = CALL(174, 0, 0, 0);
    sync_values[2] = CALL(172, 0, 0, 0);
    sync_ready_bits(4);
}
static void sync_compatible(void *unused) {
    (void)unused;
    sync_ready_bits(2);
    wait_sync_stage(1);
    sync_values[3] = CALL(174, 0, 0, 0);
    sync_values[4] = CALL(167, 21, 0, 0);
    sync_ready_bits(8);
    wait_sync_stage(2);
    sync_values[5] = CALL(174, 0, 0, 0);
    sync_values[6] = CALL(172, 0, 0, 0);
    sync_ready_bits(16);
}
static int sync_conflict_test(void) {
    CHECK(nnp() == 0, 120);
    CHECK(spawn_thread(sync_divergent, 0, stacks[0] + sizeof(stacks[0]), tls_values, tids) > 1 &&
          spawn_thread(sync_compatible, 0, stacks[1] + sizeof(stacks[1]), tls_values + 1, tids + 1) > 1, 121);
    CHECK(install_number(172, RET_ERRNO | 37) == 0, 122);
    wait_sync_ready(1 | 2);
    const unsigned conflicting_tid = __atomic_load_n(tids, __ATOMIC_SEQ_CST);
    CHECK(sync_values[0] == 0 && conflicting_tid > 1 &&
          install_number_flags(174, RET_ERRNO | 41, 1) == conflicting_tid &&
          install_number_flags(174, RET_ERRNO | 41, 17) == -3 && CALL(174, 0, 0, 0) >= 0, 123);
    sync_advance(1);
    wait_sync_ready(4 | 8);
    CHECK(sync_values[1] >= 0 && sync_values[2] == -37 && sync_values[3] >= 0 && sync_values[4] == 0, 124);
    wait_thread(0);
    CHECK(install_number_flags(174, RET_ERRNO | 41, 17) == 0, 125);
    sync_advance(2);
    wait_sync_ready(16);
    wait_thread(1);
    CHECK(sync_values[5] == -41 && sync_values[6] == -37, 126);
    print("seccomp-tsync-conflict-ok\n");
    return 0;
}

static void sync_racer(void *argument) {
    const unsigned index = (unsigned)(u64)argument;
    sync_ready_bits(1u << index);
    wait_sync_stage(1);
    sync_values[index] = install_number_flags(index ? 174 : 172, RET_ERRNO | (index ? 42 : 41), 1);
    sync_ready_bits(4u << index);
    wait_sync_stage(2);
    sync_values[2 + index * 2] = CALL(172, 0, 0, 0);
    sync_values[3 + index * 2] = CALL(174, 0, 0, 0);
}
static int sync_race_test(void) {
    CHECK(nnp() == 0, 130);
    CHECK(spawn_thread(sync_racer, (void *)0, stacks[0] + sizeof(stacks[0]), tls_values, tids) > 1 &&
          spawn_thread(sync_racer, (void *)1, stacks[1] + sizeof(stacks[1]), tls_values + 1, tids + 1) > 1,
          131);
    wait_sync_ready(3);
    sync_advance(1);
    wait_sync_ready(12);
    CHECK(sync_values[0] == 0 && sync_values[1] == 0 &&
          CALL(172, 0, 0, 0) == -41 && CALL(174, 0, 0, 0) == -42, 132);
    sync_advance(2);
    wait_thread(0);
    wait_thread(1);
    CHECK(sync_values[2] == -41 && sync_values[3] == -42 &&
          sync_values[4] == -41 && sync_values[5] == -42, 133);
    print("seccomp-tsync-race-ok\n");
    return 0;
}

static int exec_test(void) {
    CHECK(nnp() == 0 && install_number(56, RET_ERRNO | 13) == 0, 50);
    const char *args[] = {"/seccomp", "exec-child", 0};
    const char *env[] = {0};
    CALL(221, "/seccomp", args, env);
    return 51;
}
static int exec_child(void) {
    CHECK(CALL(167, 39, 0, 0) == 1 && CALL(167, 21, 0, 0) == 2 && prohibited_open() == -13, 52);
    print("seccomp-exec-inheritance-ok\n");
    return 0;
}

static void sigsys_handler(int signal, const struct sigsys_info *info, void *context) {
    (void)signal;
    (void)context;
    trapped.signal = info->signal;
    trapped.error = info->error;
    trapped.code = info->code;
    trapped.address = info->address;
    trapped.syscall = info->syscall;
    trapped.arch = info->arch;
}
static int trap_test(void) {
    struct sigaction64 action = {(u64)sigsys_handler, 4, 0, 0};
    CHECK(call6(134, 31, (u64)&action, 0, 8, 0, 0) == 0, 60);
    CHECK(nnp() == 0 && install_number(172, RET_TRAP | 123) == 0, 61);
    register u64 x0 __asm__("x0") = 0x123456789abcdef0ul;
    register u64 x8 __asm__("x8") = 172;
    u64 after;
    __asm__ volatile("adr %1, 1f\nsvc #0\n1:" : "+r"(x0), "=&r"(after) : "r"(x8) : "memory", "cc");
    CHECK(x0 == 0x123456789abcdef0ul, 62);
    CHECK(trapped.signal == 31 && trapped.error == 123 && trapped.code == 1 && trapped.address == after &&
          trapped.syscall == 172 && trapped.arch == 0xc00000b7u, 63);
    print("seccomp-trap-siginfo-ok\n");
    return 0;
}

static void kill_worker(void *unused) {
    (void)unused;
    if (install_number(172, RET_KILL_THREAD) != 0)
        bad_kill_thread = 1;
    else {
        CALL(172, 0, 0, 0);
        bad_kill_thread = 2;
    }
}
static int kill_thread_test(void) {
    const i64 child = fork_process();
    CHECK(child >= 0, 70);
    if (!child) {
        if (nnp() != 0 || spawn_thread(kill_worker, 0, stacks[0] + sizeof(stacks[0]), tls_values, tids) <= 1)
            exit_group(71);
        wait_thread(0);
        exit_group(bad_kill_thread ? 72 : 0);
    }
    CHECK(wait_child(child) == 0, 73);
    print("seccomp-kill-thread-ok\n");
    return 0;
}
static void idle_worker(void *unused) {
    (void)unused;
    __atomic_store_n(&phase, 1, __ATOMIC_SEQ_CST);
    for (;;)
        CALL(124, 0, 0, 0);
}
static int kill_process_test(void) {
    const i64 child = fork_process();
    CHECK(child >= 0, 80);
    if (!child) {
        if (nnp() != 0 || spawn_thread(idle_worker, 0, stacks[0] + sizeof(stacks[0]), tls_values, tids) <= 1)
            exit_group(81);
        while (!__atomic_load_n(&phase, __ATOMIC_SEQ_CST))
            CALL(124, 0, 0, 0);
        if (install_number(172, RET_KILL_PROCESS) != 0)
            exit_group(82);
        CALL(172, 0, 0, 0);
        exit_group(83);
    }
    const int status = wait_child(child);
    CHECK((status & 127) == 31 && (status & 128), 84);
    print("seccomp-kill-process-ok\n");
    return 0;
}
static int exit_test(void) {
    CHECK(nnp() == 0 && install_number(93, RET_ERRNO | 42) == 0, 90);
    if (CALL(93, 123, 0, 0) != -42)
        exit_group(91);
    print("seccomp-exit-blocked-ok\n");
    exit_group(0);
}
int guest_main(u64 *stack) {
    if (stack[0] < 2)
        return 2;
    const char *mode = (const char *)stack[2];
    if (equal(mode, "basic")) return basic();
    if (equal(mode, "threads")) return thread_test();
    if (equal(mode, "sync")) return sync_test();
    if (equal(mode, "sync-exec-child")) return sync_exec_child();
    if (equal(mode, "sync-conflict")) return sync_conflict_test();
    if (equal(mode, "sync-race")) return sync_race_test();
    if (equal(mode, "exec")) return exec_test();
    if (equal(mode, "exec-child")) return exec_child();
    if (equal(mode, "trap")) return trap_test();
    if (equal(mode, "kill-thread")) return kill_thread_test();
    if (equal(mode, "kill-process")) return kill_process_test();
    if (equal(mode, "exit")) return exit_test();
    return 3;
}
