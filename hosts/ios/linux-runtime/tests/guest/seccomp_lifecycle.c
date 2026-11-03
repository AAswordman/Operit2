#include "linux.h"

struct filter_instruction {
    unsigned short code;
    unsigned char true_jump, false_jump;
    unsigned value;
};
struct filter_program {
    unsigned short length;
    struct filter_instruction *instructions;
};

static i64 install(struct filter_program *program) {
    return CALL(277, 1, 0, program); // SECCOMP_SET_MODE_FILTER
}

int guest_main(u64 *stack) {
    if (stack[0] > 1) {
        CHECK(CALL(167, 39, 0, 0) == 1, 10); // PR_GET_NO_NEW_PRIVS
        CHECK(CALL(172, 0, 0, 0) == -13, 11); // getpid -> EACCES
        CHECK(CALL(174, 0, 0, 0) == -1, 12); // getuid -> EPERM
        return 0;
    }

    struct filter_instruction deny_pid[] = {
        {0x20, 0, 0, 0},                 // BPF_LD | BPF_W | BPF_ABS: syscall nr
        {0x15, 0, 1, 172},               // BPF_JMP | BPF_JEQ | BPF_K
        {0x06, 0, 0, 0x00050000 | 13},   // SECCOMP_RET_ERRNO | EACCES
        {0x06, 0, 0, 0x7fff0000},        // SECCOMP_RET_ALLOW
    };
    struct filter_instruction deny_uid[] = {
        {0x20, 0, 0, 0},
        {0x15, 0, 1, 174},
        {0x06, 0, 0, 0x00050000 | 1},    // SECCOMP_RET_ERRNO | EPERM
        {0x06, 0, 0, 0x7fff0000},
    };
    struct filter_program first = {4, deny_pid}, second = {4, deny_uid};
    CHECK(install(&first) == -13, 20); // No guest CAP_SYS_ADMIN or NO_NEW_PRIVS.
    CHECK(call6(167, 38, 1, 0, 0, 0, 0) == 0, 21); // PR_SET_NO_NEW_PRIVS
    CHECK(install(&first) == 0 && install(&second) == 0, 22);
    CHECK(CALL(172, 0, 0, 0) == -13 && CALL(174, 0, 0, 0) == -1, 23);

    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 24);
    if (!child) {
        CHECK(CALL(167, 39, 0, 0) == 1, 30);
        CHECK(CALL(172, 0, 0, 0) == -13 && CALL(174, 0, 0, 0) == -1, 31);
        const char *arguments[] = {"/seccomp_lifecycle", "after-exec", 0};
        const char *environment[] = {0};
        CALL(221, arguments[0], arguments, environment);
        fail(32);
    }
    int status = -1;
    CHECK(call6(260, (u64)child, (u64)&status, 0, 0, 0, 0) == child && status == 0, 25);
    print("seccomp-filter-fork-exec-ok\n");
    return 0;
}
