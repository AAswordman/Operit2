#include "linux.h"
static i64 map(u64 address, u64 size, u64 protection, u64 flags) {
    return call6(222, address, size, protection, flags, (u64)-1, 0);
}
static int is_error(i64 value) {
    return value < 0 && value >= -4095;
}
int guest_main(u64 *stack) {
    const char *mode = stack[0] > 1 ? (const char *)stack[2] : "stack";
    if (equal(mode, "stack")) {
        CHECK(stack[0] == 3, 1);
        CHECK(equal((const char *)stack[3], "argument with spaces"), 2);
        CHECK(((u64)stack & 15) == 0, 3);
        u64 *env = stack + stack[0] + 2;
        CHECK(*env != 0, 4);
        for (u64 j = 1; j < stack[0]; ++j)
            CHECK(stack[j + 1] == stack[j] + length((const char *)stack[j]) + 1, 6);
        CHECK(*env == stack[stack[0]] + length((const char *)stack[stack[0]]) + 1, 7);
        while (*env) {
            if (env[1])
                CHECK(env[1] == env[0] + length((const char *)env[0]) + 1, 8);
            ++env;
        }
        u64 *aux = env + 1;
        int page = 0, entry = 0, random = 0, machine = 0, rseq_size = 0, rseq_align = 0;
        while (aux[0]) {
            if (aux[0] == 6)
                page = aux[1] == 4096;
            if (aux[0] == 9)
                entry = aux[1] > 0x100000000UL;
            if (aux[0] == 25) {
                volatile unsigned char *p = (void *)aux[1];
                random = (p[0] | 1);
            }
            if (aux[0] == 15)
                machine = equal((const char *)aux[1], "aarch64");
            if (aux[0] == 27)
                rseq_size = aux[1] == 28;
            if (aux[0] == 28)
                rseq_align = aux[1] == 32;
            aux += 2;
        }
        CHECK(page && entry && random && machine, 5);
        CHECK(rseq_size && rseq_align, 9);
        i64 fd = CALL(56, -100, "/proc/self/auxv", 0);
        CHECK(fd >= 0, 80);
        u64 proc_aux[64];
        i64 bytes = CALL(63, fd, proc_aux, sizeof(proc_aux));
        CHECK(bytes > 0 && bytes % 16 == 0, 81);
        CHECK(CALL(57, fd, 0, 0) == 0, 82);
        rseq_size = rseq_align = 0;
        for (i64 j = 0; j + 1 < bytes / 8; j += 2) {
            if (proc_aux[j] == 27)
                rseq_size = proc_aux[j + 1] == 28;
            if (proc_aux[j] == 28)
                rseq_align = proc_aux[j + 1] == 32;
        }
        CHECK(rseq_size && rseq_align, 83);
    } else if (equal(mode, "exec-stack")) {
        const char *args[] = {"/workload", "stack", "argument with spaces", 0};
        const char *env[] = {"RSEQ_AUXV=1", 0};
        CALL(221, "/workload", args, env);
        fail(84);
    } else if (equal(mode, "memory")) {
        u64 length = 8UL << 30;
        i64 area = map(0, length, 0, 34);
        CHECK(!is_error(area), 10);
        u64 middle = (u64)area + (4UL << 30);
        CHECK(CALL(226, middle, 4096, 3) == 0, 11);
        *(volatile u64 *)middle = 0xabcdef0123456789UL;
        CHECK(*(volatile u64 *)middle == 0xabcdef0123456789UL, 12);
        CHECK(CALL(64, 1, middle + 4090, 16) == -14, 13);
        CHECK(CALL(215, area, length, 0) == 0, 14);
        CHECK(map(0, 4096, 3, 0) == -22, 15);
        CHECK(CALL(226, middle, 4096, 3) == -12, 16);
        u64 before = CALL(214, 0, 0, 0);
        CHECK(CALL(214, before + 8192, 0, 0) == (i64)before + 8192, 17);
        *(volatile u64 *)(before + 4096) = 123;
        CHECK(CALL(214, before, 0, 0) == (i64)before, 18);
    } else if (equal(mode, "files")) {
        i64 fd = call6(56, (u64)-100, (u64) "/created", 66, 0600, 0, 0);
        CHECK(fd >= 0, 20);
        const char *text = "persistent-arm64\n";
        CHECK(CALL(64, fd, text, length(text)) == (i64)length(text), 21);
        CHECK(CALL(62, fd, 0, 0) == 0, 22);
        char data[64] = {0};
        CHECK(CALL(63, fd, data, sizeof(data)) == (i64)length(text), 23);
        CHECK(equal(data, text), 24);
        unsigned char info[128] = {0};
        CHECK(CALL(80, fd, info, 0) == 0, 25);
        CHECK(*(u64 *)(info + 48) == length(text), 26);
        CHECK(CALL(57, fd, 0, 0) == 0, 27);
        fd = CALL(56, -100, "/data/message", 0);
        CHECK(fd >= 0, 28);
        CHECK(CALL(63, fd, data, 63) == 5, 29);
        CHECK(CALL(57, fd, 0, 0) == 0, 30);
        CHECK(CALL(49, "/data", 0, 0) == 0, 31);
        CHECK(CALL(17, data, 64, 0) == 6, 32);
        CHECK(equal(data, "/data"), 33);
        fd = CALL(56, -100, "message", 0);
        CHECK(fd >= 0, 34);
        CHECK(CALL(57, fd, 0, 0) == 0, 35);
        CHECK(CALL(56, -100, "/no-such-file", 0) == -2, 36);
        CHECK(CALL(56, -100, "", 0) == -2, 37);
        CHECK(CALL(56, -100, "/data/message", 16384) == -20, 38);
        CHECK(CALL(56, -100, "/loop", 0) == -40, 39);
        CHECK(CALL(56, -100, "/absolute", 32768) == -40, 40);
        fd = CALL(56, 999, "/data/message", 0);
        CHECK(fd >= 0, 41);
        CHECK(CALL(57, fd, 0, 0) == 0, 42);
    } else if (equal(mode, "paths")) {
        char data[32] = {0};
        i64 fd = CALL(56, -100, "/absolute/message", 0);
        CHECK(fd >= 0, 45);
        CHECK(CALL(63, fd, data, 5) == 5, 46);
        CHECK(data[0] == 'i', 47);
        CALL(57, fd, 0, 0);
        fd = CALL(56, -100, "/escape", 0);
        CHECK(fd >= 0, 48);
        CHECK(CALL(63, fd, data, 5) == 5, 49);
        CHECK(data[0] == 'r', 50);
        CALL(57, fd, 0, 0);
        fd = CALL(56, -100, "/../../secret", 0);
        CHECK(fd >= 0, 51);
        CALL(57, fd, 0, 0);
    } else if (equal(mode, "self-modify")) {
        i64 area = map(0, 4096, 7, 34);
        CHECK(!is_error(area), 55);
        volatile unsigned *code = (void *)(u64)area;
        code[0] = 0xd2800020;
        code[1] = 0xd65f03c0;
        u64 (*function)(void) = (void *)(u64)area;
        CHECK(function() == 1, 56);
        code[0] = 0xd2800040;
        CHECK(function() == 2, 57);
        CHECK(CALL(226, area, 4096, 5) == 0, 58);
        CHECK(function() == 2, 59);
        CHECK(CALL(215, area, 4096, 0) == 0, 60);
    } else if (equal(mode, "bad-pointer")) {
        CHECK(CALL(64, 1, 0, 1) == -14, 61);
        CHECK(CALL(63, 999, 0, 1) == -9, 62);
        CHECK(CALL(999999, 0, 0, 0) == -38, 63);
    } else if (equal(mode, "clock")) {
        i64 time[2] = {-1, -1};
        CHECK(CALL(113, 1, time, 0) == 0, 65);
        CHECK(time[0] >= 0 && time[1] >= 0 && time[1] < 1000000000, 66);
        CHECK(CALL(113, 999, time, 0) == -22, 67);
        unsigned char random[32];
        CHECK(CALL(278, random, sizeof(random), 0) == 32, 68);
        CHECK(CALL(278, random, sizeof(random), 16) == -22, 69);
    } else if (equal(mode, "fault")) {
        i64 area = map(0, 4096, 1, 34);
        CHECK(!is_error(area), 70);
        *(volatile u64 *)(u64)area = 1;
    } else if (equal(mode, "execute-denied")) {
        i64 area = map(0, 4096, 3, 34);
        CHECK(!is_error(area), 71);
        *(unsigned *)(u64)area = 0xd65f03c0;
        ((void (*)(void))(u64)area)();
    } else if (equal(mode, "wait-event")) {
        __asm__ volatile(".inst 0xd503205f");
    } else if (equal(mode, "undefined")) {
        __asm__ volatile(".inst 0x00000000");
    } else if (equal(mode, "loop")) {
        for (;;)
            __asm__ volatile("nop");
    } else if (equal(mode, "benchmark")) {
        volatile u64 result = 1;
        for (u64 j = 0; j < 1000000; ++j)
            result = (result * 1664525 + 1013904223) ^ j;
        CHECK(result != 0, 72);
    } else
        return 79;
    print("workload-ok\n");
    return 0;
}
