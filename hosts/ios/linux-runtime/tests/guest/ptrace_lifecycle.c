#include "linux.h"

enum {
    sigkill = 9, sigtrap = 5, sigstop = 19, sigchld = 17,
    ptrace_traceme = 0, ptrace_cont = 7, ptrace_attach = 16, ptrace_syscall = 24,
    ptrace_setoptions = 0x4200, ptrace_geteventmsg = 0x4201, ptrace_getregset = 0x4204,
    ptrace_o_tracesysgood = 1, ptrace_o_traceexec = 0x10, ptrace_o_traceexit = 0x40,
    ptrace_event_exec = 4, ptrace_event_exit = 6,
    wnohang = 1, wstopped = 2, wexited = 4, wnowait = 0x01000000,
    cld_exited = 1, cld_killed = 2, cld_trapped = 4,
    eintr = 4, echild = 10, eagain = 11,
    f_setfl = 4, o_nonblock = 0x800,
};

struct wait_info {
    int signal, error, code, padding;
    unsigned pid, uid;
    int status, padding2;
    u64 utime, stime;
    unsigned char unused[80];
};
struct iovec64 { u64 base, length; };
struct user_pt_regs { u64 x[31], sp, pc, pstate; };
_Static_assert(sizeof(struct wait_info) == 128, "AArch64 siginfo size");

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

static void decimal(char *buffer, u64 value) {
    char reverse[24];
    unsigned n = 0, i = 0;
    do {
        reverse[n++] = (char)('0' + value % 10);
        value /= 10;
    } while (value);
    while (n)
        buffer[i++] = reverse[--n];
    buffer[i] = 0;
}

__attribute__((noreturn)) static void failure(unsigned line, const char *condition) {
    char number[24];
    decimal(number, line);
    write_all(2, "ptrace lifecycle failure at line ", 33);
    write_all(2, number, length(number));
    write_all(2, ": ", 2);
    write_all(2, condition, length(condition));
    write_all(2, "\n", 1);
    fail((int)(line % 200 + 1));
    __builtin_unreachable();
}

#define REQUIRE(condition) do { if (!(condition)) failure(__LINE__, #condition); } while (0)

static void read_all(int fd, void *data, u64 size) {
    unsigned char *p = data;
    while (size) {
        i64 n = CALL(63, fd, p, size);
        if (n == -eintr)
            continue;
        REQUIRE(n > 0);
        p += n;
        size -= (u64)n;
    }
}

static void close_fd(int fd) { REQUIRE(CALL(57, fd, 0, 0) == 0); }
static void make_pipe(int fd[2]) { REQUIRE(CALL(59, fd, 0, 0) == 0); }
static i64 fork_process(void) { return call6(220, sigchld, 0, 0, 0, 0, 0); }
static i64 trace(u64 request, i64 tid, u64 address, u64 data) {
    return call6(117, request, (u64)tid, address, data, 0, 0);
}
static i64 wait4_pid(i64 tid, int *status, unsigned options) {
    i64 result;
    do {
        result = call6(260, (u64)tid, (u64)status, options, 0, 0, 0);
    } while (result == -eintr);
    return result;
}
static i64 waitid_pid(i64 tid, struct wait_info *info, unsigned options) {
    i64 result;
    do {
        result = call6(95, 1, (u64)tid, (u64)info, options, 0, 0);
    } while (result == -eintr);
    return result;
}
static void event(i64 tid, unsigned options, int code, int status) {
    struct wait_info info = {0};
    REQUIRE(waitid_pid(tid, &info, options) == 0);
    REQUIRE(info.signal == sigchld && info.code == code && info.pid == (unsigned)tid &&
            info.status == status);
}
static void peek_twice(i64 tid, unsigned kind, int code, int status) {
    event(tid, kind | wnowait, code, status);
    event(tid, kind | wnowait, code, status);
}
static void no_wait_event(i64 tid, unsigned kind) {
    struct wait_info info;
    unsigned char *bytes = (unsigned char *)&info;
    for (unsigned i = 0; i < sizeof(info); ++i)
        bytes[i] = 0xa5;
    REQUIRE(waitid_pid(tid, &info, kind | wnohang) == 0);
    REQUIRE(info.signal == 0 && info.pid == 0);
}
static void wait_status(i64 tid, int expected) {
    int status = -1;
    REQUIRE(wait4_pid(tid, &status, 0) == tid && status == expected);
}
static void send_byte(int fd, char value) { write_all(fd, &value, 1); }
static void receive_byte(int fd, char expected) {
    char value = 0;
    read_all(fd, &value, 1);
    REQUIRE(value == expected);
}
static void traceme_stop(void) {
    REQUIRE(trace(ptrace_traceme, 0, 0, 0) == 0);
    i64 self = CALL(172, 0, 0, 0);
    REQUIRE(self > 0 && CALL(129, self, sigstop, 0) == 0);
}
static void get_regs(i64 tid, struct user_pt_regs *regs) {
    struct iovec64 iov = {(u64)regs, sizeof(*regs)};
    REQUIRE(trace(ptrace_getregset, tid, 1, (u64)&iov) == 0 && iov.length == sizeof(*regs));
}

static void test_waitid(void) {
    i64 child = fork_process();
    REQUIRE(child >= 0);
    if (!child) {
        traceme_stop();
        fail(42);
    }
    peek_twice(child, wstopped, cld_trapped, sigstop);
    wait_status(child, (sigstop << 8) | 0x7f);
    no_wait_event(child, wstopped);
    REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    peek_twice(child, wexited, cld_exited, 42);
    wait_status(child, 42 << 8);
    struct wait_info info = {0};
    REQUIRE(waitid_pid(child, &info, wexited | wnohang) == -echild);
}

/* An ancestor tracer is allowed by the usual Yama ptrace_scope=1 policy. */
static void test_handoff(void) {
    int reports[2], commands[2], gate[2];
    make_pipe(reports);
    make_pipe(commands);
    make_pipe(gate);
    i64 parent = fork_process();
    REQUIRE(parent >= 0);
    if (!parent) {
        close_fd(reports[0]);
        close_fd(commands[1]);
        close_fd(gate[1]);
        i64 child = fork_process();
        REQUIRE(child >= 0);
        if (!child) {
            close_fd(reports[1]);
            close_fd(commands[0]);
            receive_byte(gate[0], 'g');
            fail(37);
        }
        close_fd(gate[0]);
        write_all(reports[1], &child, sizeof(child));
        receive_byte(commands[0], 'b');
        int status = -1;
        REQUIRE(wait4_pid(child, &status, wnohang) == 0);
        send_byte(reports[1], 'b');
        receive_byte(commands[0], 'r');
        peek_twice(child, wexited, cld_exited, 37);
        wait_status(child, 37 << 8);
        REQUIRE(wait4_pid(child, &status, wnohang) == -echild);
        send_byte(reports[1], 'r');
        fail(0);
    }
    close_fd(reports[1]);
    close_fd(commands[0]);
    close_fd(gate[0]);
    i64 child = 0;
    read_all(reports[0], &child, sizeof(child));
    REQUIRE(child > 0 && trace(ptrace_attach, child, 0, 0) == 0);
    wait_status(child, (sigstop << 8) | 0x7f);
    REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    send_byte(gate[1], 'g');
    peek_twice(child, wexited, cld_exited, 37);
    send_byte(commands[1], 'b');
    receive_byte(reports[0], 'b');
    wait_status(child, 37 << 8);
    send_byte(commands[1], 'r');
    receive_byte(reports[0], 'r');
    wait_status(parent, 0);
}

/* The supervisor survives the grandparent tracer and observes the real parent. */
static void test_tracer_exit(void) {
    int parent_reports[2], parent_commands[2], tracer_reports[2], tracer_commands[2], gate[2];
    make_pipe(parent_reports);
    make_pipe(parent_commands);
    make_pipe(tracer_reports);
    make_pipe(tracer_commands);
    make_pipe(gate);
    i64 tracer = fork_process();
    REQUIRE(tracer >= 0);
    if (!tracer) {
        i64 parent = fork_process();
        REQUIRE(parent >= 0);
        if (!parent) {
            close_fd(parent_reports[0]);
            close_fd(parent_commands[1]);
            close_fd(tracer_reports[0]);
            close_fd(tracer_reports[1]);
            close_fd(tracer_commands[0]);
            close_fd(tracer_commands[1]);
            close_fd(gate[1]);
            i64 child = fork_process();
            REQUIRE(child >= 0);
            if (!child) {
                close_fd(parent_reports[1]);
                close_fd(parent_commands[0]);
                receive_byte(gate[0], 'g');
                fail(38);
            }
            close_fd(gate[0]);
            write_all(parent_reports[1], &child, sizeof(child));
            receive_byte(parent_commands[0], 'b');
            int status = -1;
            REQUIRE(wait4_pid(child, &status, wnohang) == 0);
            send_byte(parent_reports[1], 'b');
            receive_byte(parent_commands[0], 'r');
            peek_twice(child, wexited, cld_exited, 38);
            wait_status(child, 38 << 8);
            REQUIRE(wait4_pid(child, &status, wnohang) == -echild);
            send_byte(parent_reports[1], 'r');
            fail(0);
        }
        close_fd(parent_reports[0]);
        close_fd(parent_reports[1]);
        close_fd(parent_commands[0]);
        close_fd(parent_commands[1]);
        close_fd(tracer_reports[0]);
        close_fd(tracer_commands[1]);
        close_fd(gate[0]);
        close_fd(gate[1]);
        i64 child = 0;
        read_all(tracer_commands[0], &child, sizeof(child));
        REQUIRE(child > 0 && trace(ptrace_attach, child, 0, 0) == 0);
        wait_status(child, (sigstop << 8) | 0x7f);
        REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
        send_byte(tracer_reports[1], 'a');
        peek_twice(child, wexited, cld_exited, 38);
        send_byte(tracer_reports[1], 'e');
        receive_byte(tracer_commands[0], 'd');
        fail(0); // Do not consume the tracee's exit.
    }
    close_fd(parent_reports[1]);
    close_fd(parent_commands[0]);
    close_fd(tracer_reports[1]);
    close_fd(tracer_commands[0]);
    close_fd(gate[0]);
    i64 child = 0;
    read_all(parent_reports[0], &child, sizeof(child));
    REQUIRE(child > 0);
    write_all(tracer_commands[1], &child, sizeof(child));
    receive_byte(tracer_reports[0], 'a');
    send_byte(gate[1], 'g');
    receive_byte(tracer_reports[0], 'e');
    send_byte(parent_commands[1], 'b');
    receive_byte(parent_reports[0], 'b');
    send_byte(tracer_commands[1], 'd');
    wait_status(tracer, 0);
    send_byte(parent_commands[1], 'r');
    receive_byte(parent_reports[0], 'r');
}

static void test_sigkill(void) {
    int marker[2];
    make_pipe(marker);
    i64 child = fork_process();
    REQUIRE(child >= 0);
    if (!child) {
        close_fd(marker[0]);
        traceme_stop();
        send_byte(marker[1], 'x');
        fail(99);
    }
    close_fd(marker[1]);
    wait_status(child, (sigstop << 8) | 0x7f);
    REQUIRE(CALL(129, child, sigkill, 0) == 0);
    peek_twice(child, wexited, cld_killed, sigkill);
    wait_status(child, sigkill);
    char byte = 0;
    REQUIRE(CALL(63, marker[0], &byte, 1) == 0);
}

static void test_exit_event(int group, int killed) {
    i64 child = fork_process();
    REQUIRE(child >= 0);
    if (!child) {
        traceme_stop();
        if (group)
            (void)CALL(94, 42, 0, 0);
        fail(42);
    }
    wait_status(child, (sigstop << 8) | 0x7f);
    REQUIRE(trace(ptrace_setoptions, child, 0, ptrace_o_traceexit) == 0);
    if (killed)
        REQUIRE(CALL(129, child, sigkill, 0) == 0);
    else
        REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    wait_status(child, (ptrace_event_exit << 16) | (sigtrap << 8) | 0x7f);
    u64 message = 0;
    const int expected = killed ? sigkill : 42 << 8;
    REQUIRE(trace(ptrace_geteventmsg, child, 0, (u64)&message) == 0 && message == (u64)expected);
    no_wait_event(child, wexited);
    REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    wait_status(child, expected);
}

static void wait_until_sleeping(i64 tid) {
    char path[64] = "/proc/", number[24], text[512];
    decimal(number, (u64)tid);
    unsigned p = 6, i = 0;
    while (number[i])
        path[p++] = number[i++];
    const char suffix[] = "/stat";
    for (i = 0; i < sizeof(suffix); ++i)
        path[p++] = suffix[i];
    for (unsigned attempt = 0; attempt < 10000; ++attempt) {
        i64 fd = call6(56, (u64)-100, (u64)path, 0, 0, 0, 0);
        REQUIRE(fd >= 0);
        i64 n = CALL(63, fd, text, sizeof(text));
        close_fd((int)fd);
        REQUIRE(n > 0);
        i64 end = -1;
        for (i = 0; i < (unsigned)n; ++i)
            if (text[i] == ')')
                end = (i64)i;
        REQUIRE(end >= 0 && end + 2 < n && text[end + 1] == ' ');
        if (text[end + 2] == 'S')
            return;
        REQUIRE(text[end + 2] == 'R' || text[end + 2] == 't' || text[end + 2] == 'T');
        REQUIRE(CALL(124, 0, 0, 0) == 0);
    }
    failure(__LINE__, "tracee did not block in read");
}

static void test_syscall(void) {
    int input[2], marker[2];
    make_pipe(input);
    make_pipe(marker);
    i64 child = fork_process();
    REQUIRE(child >= 0);
    if (!child) {
        close_fd(input[1]);
        close_fd(marker[0]);
        traceme_stop();
        char byte = 0;
        REQUIRE(CALL(63, input[0], &byte, 1) == 1 && byte == 'q');
        send_byte(marker[1], 'm');
        fail(0);
    }
    close_fd(input[0]);
    close_fd(marker[1]);
    wait_status(child, (sigstop << 8) | 0x7f);
    REQUIRE(trace(ptrace_setoptions, child, 0, ptrace_o_tracesysgood) == 0);
    REQUIRE(trace(ptrace_syscall, child, 0, 0) == 0);
    wait_status(child, ((sigtrap | 0x80) << 8) | 0x7f);
    struct user_pt_regs regs;
    get_regs(child, &regs);
    REQUIRE(regs.x[8] == 63 && regs.x[7] == 0 && regs.x[2] == 1);
    REQUIRE(trace(ptrace_syscall, child, 0, 0) == 0);
    wait_until_sleeping(child);
    no_wait_event(child, wstopped);
    send_byte(input[1], 'q');
    wait_status(child, ((sigtrap | 0x80) << 8) | 0x7f);
    get_regs(child, &regs);
    REQUIRE(regs.x[8] == 63 && regs.x[7] == 1 && regs.x[0] == 1);
    REQUIRE(CALL(25, marker[0], f_setfl, o_nonblock) == 0);
    char byte = 0;
    REQUIRE(CALL(63, marker[0], &byte, 1) == -eagain);
    REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    REQUIRE(CALL(25, marker[0], f_setfl, 0) == 0);
    receive_byte(marker[0], 'm');
    wait_status(child, 0);
}

static void test_exec(int traceexec) {
    int marker[2];
    make_pipe(marker);
    char fd_string[24];
    decimal(fd_string, (u64)marker[1]);
    i64 child = fork_process();
    REQUIRE(child >= 0);
    if (!child) {
        close_fd(marker[0]);
        traceme_stop();
        const char *arguments[] = {"/ptrace_lifecycle", "exec-child", fd_string, 0};
        const char *environment[] = {0};
        (void)CALL(221, arguments[0], arguments, environment);
        failure(__LINE__, "execve returned");
    }
    close_fd(marker[1]);
    wait_status(child, (sigstop << 8) | 0x7f);
    if (traceexec)
        REQUIRE(trace(ptrace_setoptions, child, 0, ptrace_o_traceexec) == 0);
    REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    wait_status(child, ((traceexec ? ptrace_event_exec : 0) << 16) | (sigtrap << 8) | 0x7f);
    if (traceexec) {
        u64 message = 0;
        REQUIRE(trace(ptrace_geteventmsg, child, 0, (u64)&message) == 0 && message == (u64)child);
    }
    REQUIRE(CALL(25, marker[0], f_setfl, o_nonblock) == 0);
    char byte = 0;
    REQUIRE(CALL(63, marker[0], &byte, 1) == -eagain);
    REQUIRE(trace(ptrace_cont, child, 0, 0) == 0);
    REQUIRE(CALL(25, marker[0], f_setfl, 0) == 0);
    receive_byte(marker[0], 'e');
    wait_status(child, 36 << 8);
}

int guest_main(u64 *stack) {
    const char **argv = (const char **)(stack + 1);
    REQUIRE(stack[0] >= 2);
    if (equal(argv[1], "exec-child")) {
        REQUIRE(stack[0] == 3);
        unsigned fd = 0;
        for (const char *p = argv[2]; *p; ++p) {
            REQUIRE(*p >= '0' && *p <= '9');
            fd = fd * 10 + (unsigned)(*p - '0');
        }
        send_byte((int)fd, 'e');
        return 36;
    }
    if (equal(argv[1], "waitid"))
        test_waitid();
    else if (equal(argv[1], "handoff"))
        test_handoff();
    else if (equal(argv[1], "tracer-exit"))
        test_tracer_exit();
    else if (equal(argv[1], "sigkill"))
        test_sigkill();
    else if (equal(argv[1], "exit-regular"))
        test_exit_event(0, 0);
    else if (equal(argv[1], "exit-group"))
        test_exit_event(1, 0);
    else if (equal(argv[1], "exit-sigkill"))
        test_exit_event(0, 1);
    else if (equal(argv[1], "syscall"))
        test_syscall();
    else if (equal(argv[1], "exec"))
        test_exec(1);
    else if (equal(argv[1], "exec-legacy"))
        test_exec(0);
    else
        failure(__LINE__, "unknown test mode");
    write_all(1, "ptrace-lifecycle-", 17);
    write_all(1, argv[1], length(argv[1]));
    write_all(1, "-ok\n", 4);
    return 0;
}
