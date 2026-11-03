#include "linux.h"

extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);

enum {
    sigtrap = 5, sigstop = 19, sigchld = 17, eintr = 4, eagain = 11, echild = 10,
    traceme = 0, cont = 7, detach = 17, syscall = 24, setoptions = 0x4200,
    geteventmsg = 0x4201, getregset = 0x4204,
    tracesysgood = 1, tracefork = 2, tracevfork = 4, traceclone = 8,
    traceexec = 0x10, tracevforkdone = 0x20,
    clone_vm = 0x100, clone_ptrace = 0x2000, clone_vfork = 0x4000, clone_untraced = 0x800000,
    wall = 0x40000000, wnohang = 1, f_setfl = 4, o_nonblock = 0x800,
    event_fork = 1, event_vfork = 2, event_clone = 3, event_exec = 4, event_vfork_done = 5,
};

struct iovec64 { u64 base, length; };
struct regs { u64 x[31], sp, pc, pstate; };
static unsigned char thread_stack[16384] __attribute__((aligned(16)));
static u64 thread_tls;
static unsigned thread_tid;
static int thread_pipe[2];

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
    write_all(2, "ptrace follow failure at line ", 30);
    write_all(2, number, length(number));
    write_all(2, ": ", 2);
    write_all(2, condition, length(condition));
    write_all(2, "\n", 1);
    fail(1);
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
static void pipe_pair(int fd[2]) { REQUIRE(CALL(59, fd, 0, 0) == 0); }
static void send_byte(int fd, char c) { write_all(fd, &c, 1); }
static void receive_byte(int fd, char expected) {
    char c = 0;
    read_all(fd, &c, 1);
    REQUIRE(c == expected);
}
static i64 trace(u64 request, i64 tid, u64 address, u64 data) {
    return call6(117, request, (u64)tid, address, data, 0, 0);
}
static i64 wait_pid(i64 tid, int *status, unsigned flags) {
    i64 result;
    do {
        result = call6(260, (u64)tid, (u64)status, flags | wall, 0, 0, 0);
    } while (result == -eintr);
    return result;
}
static void wait_status(i64 tid, int expected) {
    int status = -1;
    i64 result;
    do {
        result = wait_pid(tid, &status, 0);
        if (result != tid || status != ((sigchld << 8) | 0x7f) || status == expected)
            break;
        REQUIRE(trace(cont, tid, 0, 0) == 0);
    } while (1);
    if (result != tid || status != expected) {
        char number[24];
        write_all(2, "wait tid=", 9); decimal(number, (u64)tid); write_all(2, number, length(number));
        write_all(2, " result=", 8); decimal(number, (u64)result); write_all(2, number, length(number));
        write_all(2, " status=", 8); decimal(number, (unsigned)status); write_all(2, number, length(number));
        write_all(2, " expected=", 10); decimal(number, (unsigned)expected); write_all(2, number, length(number));
        write_all(2, "\n", 1);
        failure(__LINE__, "unexpected wait result");
    }
}
static int event_status(int event) { return (event << 16) | (sigtrap << 8) | 0x7f; }
static i64 event_child(i64 parent, int event) {
    u64 child = 0;
    wait_status(parent, event_status(event));
    REQUIRE(trace(geteventmsg, parent, 0, (u64)&child) == 0 && child > 0);
    return (i64)child;
}
static void resume(i64 tid, int how) { REQUIRE(trace(how, tid, 0, 0) == 0); }
static void regs(i64 tid, struct regs *result) {
    struct iovec64 iov = {(u64)result, sizeof(*result)};
    REQUIRE(trace(getregset, tid, 1, (u64)&iov) == 0 && iov.length == sizeof(*result));
}
static void traceme_stop(void) {
    REQUIRE(trace(traceme, 0, 0, 0) == 0);
    i64 self = CALL(172, 0, 0, 0);
    REQUIRE(self > 0 && CALL(129, self, sigstop, 0) == 0);
}
static i64 fork_process(void) { return call6(220, sigchld, 0, 0, 0, 0, 0); }
static void worker(void *arg) {
    send_byte((int)(u64)arg, 'C');
    send_byte(thread_pipe[1], 'T');
}

static void tracee(const char *mode, int marker) {
    traceme_stop();
    if (equal(mode, "follow-thread")) {
        pipe_pair(thread_pipe);
        i64 tid = spawn_thread(worker, (void *)(u64)marker, thread_stack + sizeof(thread_stack),
                               &thread_tls, &thread_tid);
        REQUIRE(tid > 0 && (unsigned)tid == thread_tid);
        receive_byte(thread_pipe[0], 'T');
        send_byte(marker, 'P');
        fail(33);
    }
    i64 child;
    if (equal(mode, "follow-clone3")) {
        u64 args[8] = {0, 0, 0, 0, 0, 0, 0, 0};
        child = call6(435, (u64)args, sizeof(args), 0, 0, 0, 0);
    } else {
        u64 flags = sigchld;
        if (equal(mode, "follow-vfork-late") || equal(mode, "follow-vfork-early"))
            flags |= clone_vm | clone_vfork;
        child = call6(220, flags, 0, 0, 0, 0, 0);
    }
    REQUIRE(child >= 0);
    if (!child) {
        if (equal(mode, "follow-tree")) {
            i64 grandchild = fork_process();
            REQUIRE(grandchild >= 0);
            if (!grandchild) {
                send_byte(marker, 'G');
                fail(31);
            }
            int status = -1;
            REQUIRE(wait_pid(grandchild, &status, 0) == grandchild && status == (31 << 8));
            send_byte(marker, 'C');
            fail(32);
        }
        if (equal(mode, "follow-exec")) {
            const char *args[] = {"/ptrace_follow", "exec-child", 0};
            const char *env[] = {0};
            (void)CALL(221, args[0], args, env);
            failure(__LINE__, "execve returned");
        }
        send_byte(marker, 'C');
        fail(31);
    }
    int status = -1;
    REQUIRE(wait_pid(child, &status, 0) == child && status == ((equal(mode, "follow-tree") ? 32 : 31) << 8));
    send_byte(marker, 'P');
    fail(33);
}

static void follow_process(const char *mode) {
    int marker[2];
    pipe_pair(marker);
    i64 parent = fork_process();
    REQUIRE(parent >= 0);
    if (!parent)
        tracee(mode, marker[1]);
    wait_status(parent, (sigstop << 8) | 0x7f);
    u64 options = tracesysgood | tracefork | tracevfork | traceclone | traceexec | tracevforkdone;
    REQUIRE(trace(setoptions, parent, 0, options) == 0);
    int traced_syscalls = equal(mode, "follow-syscall");
    resume(parent, traced_syscalls ? syscall : cont);
    struct regs before;
    if (traced_syscalls) {
        wait_status(parent, ((sigtrap | 0x80) << 8) | 0x7f);
        regs(parent, &before);
        REQUIRE(before.x[8] == 220 && before.x[0] == sigchld && before.x[7] == 0);
        resume(parent, syscall);
    }
    int event = (equal(mode, "follow-clone3") || equal(mode, "follow-thread")) ? event_clone
                : (equal(mode, "follow-vfork-late") || equal(mode, "follow-vfork-early")) ? event_vfork
                                                                                           : event_fork;
    i64 child = event_child(parent, event);
    if (traced_syscalls) {
        struct regs during;
        regs(parent, &during);
        REQUIRE(during.x[0] == before.x[0]);
    }
    wait_status(child, (sigstop << 8) | 0x7f);
    REQUIRE(CALL(25, marker[0], f_setfl, o_nonblock) == 0);
    char byte = 0;
    REQUIRE(CALL(63, marker[0], &byte, 1) == -eagain);
    REQUIRE(CALL(25, marker[0], f_setfl, 0) == 0);

    if (equal(mode, "follow-vfork-early")) {
        resume(child, cont);
        wait_status(child, 31 << 8);
        resume(parent, cont);
        REQUIRE(event_child(parent, event_vfork_done) == child);
        resume(parent, cont);
    } else if (equal(mode, "follow-vfork-late")) {
        resume(parent, cont);
        int status = -1;
        REQUIRE(wait_pid(parent, &status, wnohang) == 0);
        resume(child, cont);
        REQUIRE(event_child(parent, event_vfork_done) == child);
        wait_status(child, 31 << 8);
        resume(parent, cont);
    } else {
        resume(parent, traced_syscalls ? syscall : cont);
        if (traced_syscalls) {
            wait_status(parent, ((sigtrap | 0x80) << 8) | 0x7f);
            struct regs after;
            regs(parent, &after);
            REQUIRE(after.x[0] == (u64)child && after.x[7] == 1);
            resume(parent, cont);
        }
        resume(child, traced_syscalls ? syscall : cont);
        if (traced_syscalls) {
            wait_status(child, ((sigtrap | 0x80) << 8) | 0x7f);
            struct regs first;
            regs(child, &first);
            REQUIRE(first.x[8] == 64 && first.x[7] == 0);
            resume(child, cont);
        }
        if (equal(mode, "follow-tree")) {
            i64 grandchild = event_child(child, event_fork);
            wait_status(grandchild, (sigstop << 8) | 0x7f);
            resume(child, cont);
            resume(grandchild, cont);
            wait_status(grandchild, 31 << 8);
        } else if (equal(mode, "follow-exec")) {
            REQUIRE(event_child(child, event_exec) == child);
            resume(child, cont);
        }
        wait_status(child, (equal(mode, "follow-thread") ? 0 : equal(mode, "follow-tree") ? 32 : 31) << 8);
    }
    wait_status(parent, 33 << 8);
    if (equal(mode, "follow-tree"))
        receive_byte(marker[0], 'G');
    if (!equal(mode, "follow-exec"))
        receive_byte(marker[0], 'C');
    receive_byte(marker[0], 'P');
}

int guest_main(u64 *stack) {
    const char **argv = (const char **)(stack + 1);
    REQUIRE(stack[0] == 2);
    if (equal(argv[1], "exec-child"))
        return 31;
    follow_process(argv[1]);
    write_all(1, "ptrace-lifecycle-", 17);
    write_all(1, argv[1], length(argv[1]));
    write_all(1, "-ok\n", 4);
    return 0;
}
