#include "linux.h"

extern long spawn_thread(void (*fn)(void *), void *arg, void *stack, void *tls, unsigned *tid);

enum {
    sigchld = 17, sigstop = 19, sigtrap = 5,
    eintr = 4, eagain = 11, echild = 10,
    ptrace_cont = 7, ptrace_attach = 16, ptrace_detach = 17,
    ptrace_setoptions = 0x4200, ptrace_geteventmsg = 0x4201,
    ptrace_o_traceexit = 0x40, ptrace_event_exit = 6,
    wall = 0x40000000, wnohang = 1,
    exit_code = 42, exit_status = exit_code << 8,
    exit_stop = (ptrace_event_exit << 16) | (sigtrap << 8) | 0x7f,
};

static unsigned char stacks[2][16384] __attribute__((aligned(16)));
static u64 tls_values[2];
static unsigned tids[2], ready, gate;

static void write_all(int fd, const void *data, u64 size) {
    const unsigned char *p = data;
    while (size) {
        i64 n = CALL(64, fd, p, size);
        if (n == -eintr)
            continue;
        if (n <= 0) {
            CALL(94, 251, 0, 0);
            __builtin_unreachable();
        }
        p += n;
        size -= (u64)n;
    }
}

static void text(int fd, const char *s) { write_all(fd, s, length(s)); }

static void number(int fd, i64 value) {
    char digits[24];
    unsigned count = 0;
    u64 magnitude = (u64)value;
    if (value < 0) {
        text(fd, "-");
        magnitude = 0 - magnitude;
    }
    do {
        digits[count++] = (char)('0' + magnitude % 10);
        magnitude /= 10;
    } while (magnitude);
    while (count)
        write_all(fd, &digits[--count], 1);
}

__attribute__((noreturn)) static void failure(unsigned line, const char *expression,
                                              i64 actual, i64 expected) {
    text(2, "ptrace-threads-exit line ");
    number(2, line);
    text(2, ": ");
    text(2, expression);
    text(2, "; actual=");
    number(2, actual);
    text(2, " expected=");
    number(2, expected);
    text(2, "\n");
    CALL(94, 1, 0, 0);
    __builtin_unreachable();
}

#define REQUIRE(condition) do { if (!(condition)) failure(__LINE__, #condition, 0, 1); } while (0)
#define EXPECT(actual, expected) do { \
    i64 actual_ = (i64)(actual), expected_ = (i64)(expected); \
    if (actual_ != expected_) failure(__LINE__, #actual, actual_, expected_); \
} while (0)

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

static i64 trace(u64 request, i64 tid, u64 data) {
    return call6(117, request, (u64)tid, 0, data, 0, 0);
}

static i64 wait_tid(i64 tid, int *status, unsigned options) {
    i64 result;
    do {
        result = call6(260, (u64)tid, (u64)status, options | wall, 0, 0, 0);
    } while (result == -eintr);
    return result;
}

static void wait_status(i64 tid, int expected) {
    int status = -1;
    EXPECT(wait_tid(tid, &status, 0), tid);
    EXPECT(status, expected);
}

static void worker(void *arg) {
    unsigned index = (unsigned)(u64)arg;
    EXPECT(CALL(178, 0, 0, 0), __atomic_load_n(&tids[index], __ATOMIC_SEQ_CST));
    __atomic_fetch_add(&ready, 1, __ATOMIC_SEQ_CST);
    CALL(98, &ready, 129, 1);
    for (;;) {
        i64 result = CALL(98, &gate, 128, 0);
        REQUIRE(result == 0 || result == -eintr || result == -eagain);
    }
}

static void target(int report_fd, int command_fd) {
    for (unsigned index = 0; index < 2; ++index) {
        i64 tid = spawn_thread(worker, (void *)(u64)index, stacks[index] + sizeof(stacks[index]),
                               tls_values + index, tids + index);
        REQUIRE(tid > 0);
        EXPECT(tid, __atomic_load_n(&tids[index], __ATOMIC_SEQ_CST));
    }
    while (__atomic_load_n(&ready, __ATOMIC_SEQ_CST) != 2) {
        unsigned observed = __atomic_load_n(&ready, __ATOMIC_SEQ_CST);
        if (observed != 2) {
            i64 result = CALL(98, &ready, 128, observed);
            REQUIRE(result == 0 || result == -eintr || result == -eagain);
        }
    }
    write_all(report_fd, tids, sizeof(tids));
    char command = 0;
    read_all(command_fd, &command, 1);
    EXPECT(command, 'x');
    CALL(94, exit_code, 0, 0);
    failure(__LINE__, "exit_group returned", 0, 1);
}

static void check_held(i64 tid, int expected_status) {
    int status = -1;
    u64 message = ~0UL;
    EXPECT(wait_tid(tid, &status, wnohang), 0);
    EXPECT(trace(ptrace_geteventmsg, tid, (u64)&message), 0);
    EXPECT(message, expected_status);
}

static void wait_individual(i64 tid, int expected_status) {
    struct timespec { i64 seconds, nanoseconds; } start, now, delay = {0, 10000000};
    EXPECT(CALL(113, 1, &start, 0), 0);
    for (;;) {
        int status = -1;
        i64 result = wait_tid(tid, &status, wnohang);
        if (result == tid) {
            EXPECT(status, expected_status);
            return;
        }
        EXPECT(result, 0);
        EXPECT(CALL(113, 1, &now, 0), 0);
        if (now.seconds - start.seconds >= 3)
            failure(__LINE__, "first thread exits while second is still stopped", result, tid);
        i64 slept = CALL(101, &delay, 0, 0);
        REQUIRE(slept == 0 || slept == -eintr);
    }
}

static void trace_to_exit_stops(i64 child, int report_fd, int command_fd, unsigned target_tids[2],
                                int trace_leader, int fatal) {
    const int expected_status = fatal ? 9 : exit_status;
    read_all(report_fd, target_tids, 2 * sizeof(*target_tids));
    REQUIRE(target_tids[0] && target_tids[1] && target_tids[0] != target_tids[1] &&
            target_tids[0] != (unsigned)child && target_tids[1] != (unsigned)child);
    text(1, "leader="); number(1, child);
    text(1, " workers="); number(1, target_tids[0]); text(1, ",");
    number(1, target_tids[1]); text(1, "\n");

    for (unsigned index = 0; index < 2U + (unsigned)trace_leader; ++index) {
        i64 tid = index == 2 ? child : target_tids[index];
        EXPECT(trace(ptrace_attach, tid, 0), 0);
        wait_status(tid, (sigstop << 8) | 0x7f);
        EXPECT(trace(ptrace_setoptions, tid, ptrace_o_traceexit), 0);
        EXPECT(trace(ptrace_cont, tid, 0), 0);
    }
    text(1, "both workers attached with TRACEEXIT\n");
    if (fatal)
        EXPECT(CALL(129, child, 9, 0), 0);
    else
        write_all(command_fd, "x", 1);
    for (unsigned index = 0; index < 2U + (unsigned)trace_leader; ++index) {
        i64 tid = index == 2 ? child : target_tids[index];
        u64 message = ~0UL;
        wait_status(tid, exit_stop);
        EXPECT(trace(ptrace_geteventmsg, tid, (u64)&message), 0);
        EXPECT(message, expected_status);
        check_held(tid, expected_status);
        text(1, "TRACEEXIT tid="); number(1, tid);
        text(1, " message="); number(1, (i64)message); text(1, "\n");
    }
}

static i64 fork_process(void) { return call6(220, sigchld, 0, 0, 0, 0, 0); }

static void expect_byte(int fd, char expected) {
    char value = 0;
    read_all(fd, &value, 1);
    EXPECT(value, expected);
}

/* The tracer is an ancestor of the target but not its real parent. */
static void test_tracer_death(void) {
    int ids[2], reports[2], commands[2], parent_in[2], parent_out[2], tracer_in[2], tracer_out[2];
    EXPECT(CALL(59, ids, 0, 0), 0);
    EXPECT(CALL(59, reports, 0, 0), 0);
    EXPECT(CALL(59, commands, 0, 0), 0);
    EXPECT(CALL(59, parent_in, 0, 0), 0);
    EXPECT(CALL(59, parent_out, 0, 0), 0);
    EXPECT(CALL(59, tracer_in, 0, 0), 0);
    EXPECT(CALL(59, tracer_out, 0, 0), 0);
    i64 tracer = fork_process();
    REQUIRE(tracer >= 0);
    if (!tracer) {
        i64 parent = fork_process();
        REQUIRE(parent >= 0);
        if (!parent) {
            i64 child = fork_process();
            REQUIRE(child >= 0);
            if (!child) {
                target(reports[1], commands[0]);
                __builtin_unreachable();
            }
            write_all(ids[1], &child, sizeof(child));
            expect_byte(parent_in[0], 'b');
            int status = -1;
            EXPECT(wait_tid(child, &status, wnohang), 0);
            write_all(parent_out[1], "b", 1);
            expect_byte(parent_in[0], 'r');
            wait_status(child, exit_status);
            EXPECT(wait_tid(child, &status, wnohang), -echild);
            write_all(parent_out[1], "r", 1);
            CALL(94, 0, 0, 0);
            __builtin_unreachable();
        }
        i64 child = 0;
        unsigned target_tids[2] = {0, 0};
        read_all(ids[0], &child, sizeof(child));
        REQUIRE(child > 0);
        trace_to_exit_stops(child, reports[0], commands[1], target_tids, 0, 0);
        write_all(tracer_out[1], "e", 1);
        expect_byte(tracer_in[0], 'd');
        CALL(94, 0, 0, 0);
        __builtin_unreachable();
    }
    expect_byte(tracer_out[0], 'e');
    write_all(parent_in[1], "b", 1);
    expect_byte(parent_out[0], 'b');
    text(1, "real parent cannot reap while tracer holds both stops\n");
    write_all(tracer_in[1], "d", 1);
    wait_status(tracer, 0);
    write_all(parent_in[1], "r", 1);
    expect_byte(parent_out[0], 'r');
    text(1, "real parent reaped target after tracer death\ntracer-death: PASS\n");
}

int guest_main(u64 *stack) {
    const char **argv = (const char **)(stack + 1);
    REQUIRE(stack[0] == 2);
    if (equal(argv[1], "tracer-death")) {
        test_tracer_death();
        return 0;
    }
    int fatal = equal(argv[1], "fatal") || equal(argv[1], "fatal-leader");
    int trace_leader = equal(argv[1], "leader") || equal(argv[1], "fatal-leader");
    int individual = equal(argv[1], "individual") || trace_leader || fatal;
    int detach = equal(argv[1], "detach");
    const int expected_status = fatal ? 9 : exit_status;
    REQUIRE(individual || detach || equal(argv[1], "group"));

    int reports[2], commands[2];
    EXPECT(CALL(59, reports, 0, 0), 0);
    EXPECT(CALL(59, commands, 0, 0), 0);
    i64 child = fork_process();
    REQUIRE(child >= 0);
    if (!child) {
        EXPECT(CALL(57, reports[0], 0, 0), 0);
        EXPECT(CALL(57, commands[1], 0, 0), 0);
        target(reports[1], commands[0]);
        __builtin_unreachable();
    }
    EXPECT(CALL(57, reports[1], 0, 0), 0);
    EXPECT(CALL(57, commands[0], 0, 0), 0);
    unsigned target_tids[2] = {0, 0};
    trace_to_exit_stops(child, reports[0], commands[1], target_tids, trace_leader, fatal);
    int status = -1;
    EXPECT(wait_tid(child, &status, wnohang), 0);
    if (trace_leader) {
        EXPECT(trace(ptrace_cont, child, 0), 0);
        EXPECT(wait_tid(child, &status, wnohang), 0);
        text(1, "leader continued; workers still held\n");
    }
    EXPECT(trace(detach ? ptrace_detach : ptrace_cont, target_tids[0], 0), 0);
    check_held(target_tids[1], expected_status);
    EXPECT(wait_tid(child, &status, wnohang), 0);
    if (detach) {
        EXPECT(wait_tid(target_tids[0], &status, wnohang), -echild);
        text(1, "first detached; second still held\n");
    } else {
        text(1, "first continued; second still held\n");
    }
    if (individual) {
        wait_individual(target_tids[0], expected_status);
        check_held(target_tids[1], expected_status);
        EXPECT(wait_tid(child, &status, wnohang), 0);
        text(1, "first exited while second held\n");
    }
    EXPECT(trace(ptrace_cont, target_tids[1], 0), 0);
    if (!individual && !detach)
        wait_status(target_tids[0], expected_status);
    wait_status(target_tids[1], expected_status);
    wait_status(child, expected_status);
    EXPECT(wait_tid(target_tids[0], &status, wnohang), -echild);
    EXPECT(wait_tid(target_tids[1], &status, wnohang), -echild);
    EXPECT(wait_tid(child, &status, wnohang), -echild);
    text(1, argv[1]);
    text(1, ": PASS\n");
    return 0;
}
