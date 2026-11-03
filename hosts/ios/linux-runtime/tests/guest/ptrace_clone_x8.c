#include "linux.h"

enum {
    sigchld = 17, sigstop = 19, sigtrap = 5, eintr = 4,
    ptrace_traceme = 0, ptrace_detach = 17, ptrace_syscall = 24,
    ptrace_setoptions = 0x4200, ptrace_getregset = 0x4204, ptrace_setregset = 0x4205,
    ptrace_o_tracesysgood = 1,
    sys_clone = 220, sys_clone3 = 435,
};

static const u64 changed_x8 = 0x1122334455667788UL;

struct iovec64 { u64 base, length; };
struct user_pt_regs { u64 x[31], sp, pc, pstate; };
struct report { u64 kind, x8; i64 result, waited, status; };

static void write_all(int fd, const void *data, u64 size) {
    const unsigned char *p = data;
    while (size) {
        i64 n = CALL(64, fd, p, size);
        if (n == -eintr)
            continue;
        if (n <= 0)
            fail(250);
        p += n;
        size -= (u64)n;
    }
}

static void text(int fd, const char *s) { write_all(fd, s, length(s)); }

static void decimal(int fd, i64 value) {
    char buffer[24];
    unsigned n = 0;
    u64 magnitude = (u64)value;
    if (value < 0) {
        text(fd, "-");
        magnitude = 0 - magnitude;
    }
    do {
        buffer[n++] = (char)('0' + magnitude % 10);
        magnitude /= 10;
    } while (magnitude);
    while (n)
        write_all(fd, &buffer[--n], 1);
}

static void hex(int fd, u64 value) {
    static const char digits[] = "0123456789abcdef";
    char buffer[18] = {'0', 'x'};
    for (unsigned i = 0; i < 16; ++i)
        buffer[2 + i] = digits[(value >> (60 - 4 * i)) & 15];
    write_all(fd, buffer, sizeof(buffer));
}

__attribute__((noreturn)) static void failure(unsigned line, const char *condition) {
    text(2, "ptrace-clone-x8 setup failure at line ");
    decimal(2, line);
    text(2, ": ");
    text(2, condition);
    text(2, "\n");
    fail(1);
    __builtin_unreachable();
}

#define REQUIRE(condition) do { if (!(condition)) failure(__LINE__, #condition); } while (0)

static i64 trace(u64 request, i64 tid, u64 address, u64 data) {
    return call6(117, request, (u64)tid, address, data, 0, 0);
}

static i64 wait_pid(i64 pid, int *status) {
    i64 result;
    do {
        result = call6(260, (u64)pid, (u64)status, 0, 0, 0, 0);
    } while (result == -eintr);
    return result;
}

static void wait_stop(i64 pid, int expected) {
    int status = -1;
    REQUIRE(wait_pid(pid, &status) == pid && status == expected);
}

static void get_regs(i64 pid, struct user_pt_regs *regs) {
    struct iovec64 iov = {(u64)regs, sizeof(*regs)};
    REQUIRE(trace(ptrace_getregset, pid, 1, (u64)&iov) == 0 && iov.length == sizeof(*regs));
}

static void set_regs(i64 pid, struct user_pt_regs *regs) {
    struct iovec64 iov = {(u64)regs, sizeof(*regs)};
    REQUIRE(trace(ptrace_setregset, pid, 1, (u64)&iov) == 0 && iov.length == sizeof(*regs));
}

/* The store must be the first guest instruction after svc in both processes. */
__attribute__((noinline)) static i64 traced_clone(u64 number, u64 arg0, u64 arg1, u64 *saved_x8) {
    register u64 x0 __asm__("x0") = arg0;
    register u64 x1 __asm__("x1") = arg1;
    register u64 x2 __asm__("x2") = 0;
    register u64 x3 __asm__("x3") = 0;
    register u64 x4 __asm__("x4") = 0;
    register u64 x8 __asm__("x8") = number;
    __asm__ volatile("svc #0\n\tstr x8, [%[saved]]"
                     : "+&r"(x0), "+&r"(x8)
                     : "r"(x1), "r"(x2), "r"(x3), "r"(x4), [saved] "r"(saved_x8)
                     : "memory", "cc");
    return (i64)x0;
}

static void tracee(int report_fd, u64 number) {
    REQUIRE(trace(ptrace_traceme, 0, 0, 0) == 0);
    i64 self = CALL(172, 0, 0, 0);
    REQUIRE(self > 0 && CALL(129, self, sigstop, 0) == 0);

    /* clone_args version 0: flags, pidfd, child_tid, parent_tid, exit_signal,
       stack, stack_size, tls. A zero stack gives fork-like process semantics. */
    u64 args[8] = {0, 0, 0, 0, sigchld, 0, 0, 0};
    u64 saved_x8 = 0;
    i64 result = traced_clone(number, number == sys_clone ? sigchld : (u64)args,
                              number == sys_clone ? 0 : sizeof(args), &saved_x8);
    struct report record = {result == 0 ? 0UL : 1UL, saved_x8, result, -1, -1};
    if (result == 0) {
        write_all(report_fd, &record, sizeof(record));
        fail(0);
    }
    if (result > 0) {
        int status = -1;
        record.waited = wait_pid(result, &status);
        record.status = status;
    }
    write_all(report_fd, &record, sizeof(record));
    fail(0);
}

static int read_record(int fd, struct report *record) {
    unsigned char *p = (unsigned char *)record;
    u64 remaining = sizeof(*record);
    while (remaining) {
        i64 n = CALL(63, fd, p, remaining);
        if (n == -eintr)
            continue;
        REQUIRE(n >= 0);
        if (n == 0) {
            REQUIRE(remaining == sizeof(*record));
            return 0;
        }
        p += n;
        remaining -= (u64)n;
    }
    return 1;
}

static void issue(int condition, const char *name, int *failed) {
    if (!condition) {
        text(1, "  FAIL ");
        text(1, name);
        text(1, "\n");
        *failed = 1;
    }
}

int guest_main(u64 *stack) {
    const char **argv = (const char **)(stack + 1);
    REQUIRE(stack[0] == 2);
    u64 number;
    int mutate;
    if (equal(argv[1], "clone-control")) {
        number = sys_clone;
        mutate = 0;
    } else if (equal(argv[1], "clone-mutated")) {
        number = sys_clone;
        mutate = 1;
    } else if (equal(argv[1], "clone3-control")) {
        number = sys_clone3;
        mutate = 0;
    } else if (equal(argv[1], "clone3-mutated")) {
        number = sys_clone3;
        mutate = 1;
    } else {
        failure(__LINE__, "unknown case");
    }

    int pipe_fds[2];
    REQUIRE(CALL(59, pipe_fds, 0, 0) == 0);
    i64 tracee_pid = call6(sys_clone, sigchld, 0, 0, 0, 0, 0);
    REQUIRE(tracee_pid >= 0);
    if (!tracee_pid) {
        REQUIRE(CALL(57, pipe_fds[0], 0, 0) == 0);
        tracee(pipe_fds[1], number);
        __builtin_unreachable();
    }
    REQUIRE(CALL(57, pipe_fds[1], 0, 0) == 0);
    wait_stop(tracee_pid, (sigstop << 8) | 0x7f);
    REQUIRE(trace(ptrace_setoptions, tracee_pid, 0, ptrace_o_tracesysgood) == 0);
    REQUIRE(trace(ptrace_syscall, tracee_pid, 0, 0) == 0);
    wait_stop(tracee_pid, ((sigtrap | 0x80) << 8) | 0x7f);

    struct user_pt_regs regs;
    get_regs(tracee_pid, &regs);
    REQUIRE(regs.x[8] == number && regs.x[7] == 0);
    REQUIRE(number == sys_clone ? regs.x[0] == sigchld && regs.x[1] == 0
                                : regs.x[0] != 0 && regs.x[1] == 64);
    u64 entry_x8 = regs.x[8];
    u64 expected_x8 = mutate ? changed_x8 : number;
    regs.x[8] = expected_x8;
    set_regs(tracee_pid, &regs);
    get_regs(tracee_pid, &regs);
    u64 written_x8 = regs.x[8];

    REQUIRE(trace(ptrace_syscall, tracee_pid, 0, 0) == 0);
    wait_stop(tracee_pid, ((sigtrap | 0x80) << 8) | 0x7f);
    get_regs(tracee_pid, &regs);
    u64 exit_x8 = regs.x[8];
    u64 exit_x7 = regs.x[7];
    i64 clone_result = (i64)regs.x[0];
    REQUIRE(trace(ptrace_detach, tracee_pid, 0, 0) == 0);

    int tracee_status = -1;
    REQUIRE(wait_pid(tracee_pid, &tracee_status) == tracee_pid);
    struct report parent = {0}, child = {0}, record;
    unsigned parents = 0, children = 0, unknown = 0;
    while (read_record(pipe_fds[0], &record)) {
        if (record.kind == 0) {
            child = record;
            ++children;
        } else if (record.kind == 1) {
            parent = record;
            ++parents;
        } else {
            ++unknown;
        }
    }
    REQUIRE(CALL(57, pipe_fds[0], 0, 0) == 0);

    text(1, argv[1]);
    text(1, ": entry="); hex(1, entry_x8);
    text(1, " written="); hex(1, written_x8);
    text(1, " exit="); hex(1, exit_x8);
    text(1, " parent="); hex(1, parent.x8);
    text(1, " child="); hex(1, child.x8);
    text(1, "\n  clone="); decimal(1, clone_result);
    text(1, " parent-result="); decimal(1, parent.result);
    text(1, " child-result="); decimal(1, child.result);
    text(1, " waited="); decimal(1, parent.waited);
    text(1, " child-status="); decimal(1, parent.status);
    text(1, " tracee-status="); decimal(1, tracee_status);
    text(1, " reports="); decimal(1, parents); text(1, "/"); decimal(1, children);
    text(1, "/"); decimal(1, unknown); text(1, "\n");

    int failed = 0;
    issue(written_x8 == expected_x8, "entry write x8", &failed);
    issue(exit_x7 == 1, "syscall exit stop", &failed);
    issue(clone_result > 0, "original clone syscall did not create a child", &failed);
    issue(exit_x8 == expected_x8, "exit-stop x8", &failed);
    issue(tracee_status == 0, "tracee exit status", &failed);
    issue(parents == 1 && children == 1 && unknown == 0, "parent/child reports", &failed);
    issue(parent.result == clone_result && child.result == 0, "parent/child syscall results", &failed);
    issue(parent.waited == clone_result && parent.status == 0, "child wait status", &failed);
    issue(parent.x8 == expected_x8, "parent return x8", &failed);
    issue(child.x8 == expected_x8, "child return x8", &failed);
    if (!failed) {
        text(1, argv[1]);
        text(1, ": PASS\n");
    }
    return failed;
}
