/* Identical freestanding Linux AArch64 ELF for cross-runtime comparisons.
 * A native macOS build supplies the expected checksums, not speed ratios.
 * All integer overflow is unsigned; floating point stays finite. */
#include <stdint.h>
#include <stddef.h>
#ifdef __APPLE__
#include <time.h>
#include <unistd.h>
static uint64_t now_ns(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (uint64_t)t.tv_sec * 1000000000 + t.tv_nsec;
}
static void output(const char *p, size_t n) { (void)write(1, p, n); }
#else
static long syscall3(long nr, long a, long b, long c) {
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    register long x8 __asm__("x8") = nr;
    __asm__ volatile("svc #0" : "+r"(x0) : "r"(x1), "r"(x2), "r"(x8) : "memory", "cc");
    return x0;
}
static uint64_t now_ns(void) {
    struct { int64_t sec, ns; } t;
    if (syscall3(113, 1, (long)&t, 0)) syscall3(94, 2, 0, 0);
    return (uint64_t)t.sec * 1000000000 + t.ns;
}
static void output(const char *p, size_t n) {
    if (syscall3(64, 1, (long)p, (long)n) != (long)n) syscall3(94, 3, 0, 0);
}
#endif

static volatile uint64_t sink;
static uint64_t data[65536];

__attribute__((noinline)) static uint64_t integer(void) {
    uint64_t a = 1234567890, b = 987654321, c = 0;
    for (unsigned i = 0; i < 10000000; ++i) {
        c += a * b;
        c ^= c >> 7;
        a += c & 255;
        b -= (a << 3) ^ c;
        c += a % (b | 1);
    }
    return c;
}

__attribute__((noinline)) static uint64_t memory(void) {
    for (unsigned i = 0; i < 65536; ++i) data[i] = (uint64_t)i * 2654435761u;
    uint64_t sum = 0;
    uint32_t rng = 12345;
    for (unsigned i = 0; i < 10000000; ++i) {
        rng = rng * 1664525u + 1013904223u;
        unsigned j = (rng >> 8) & 65535;
        sum += data[j];
        data[j] ^= sum;
    }
    return sum;
}

__attribute__((noinline)) static uint64_t floating(void) {
    double a = 0.25, b = 0.75;
    for (unsigned i = 0; i < 10000000; ++i) {
        a = a * 0.9999999 + b * 0.0000001;
        b = b * 0.9999998 + a * 0.0000002;
    }
    union { double d; uint64_t u; } bits = { .d = a + b };
    return bits.u;
}

__attribute__((noinline)) static uint64_t mix(uint64_t x, uint64_t y) {
    return (x ^ (y * 0x9e3779b97f4a7c15ull)) + ((x << 13) | (x >> 51));
}
__attribute__((noinline)) static uint64_t calls(void) {
    uint64_t x = 7;
    for (unsigned i = 0; i < 10000000; ++i) x = mix(x, i);
    return x;
}

static char *decimal(char *p, uint64_t n) {
    char tmp[24]; unsigned k = 0;
    do { tmp[k++] = (char)('0' + n % 10); n /= 10; } while (n);
    while (k) *p++ = tmp[--k];
    return p;
}
static char *text(char *p, const char *s) { while (*s) *p++ = *s++; return p; }
static void run(const char *name, uint64_t (*fn)(void)) {
    sink = fn(); /* Warm this workload inside the same process. */
    uint64_t start = now_ns(), check = fn(), elapsed = now_ns() - start;
    sink = check;
    char buf[160], *p = text(buf, name);
    p = text(p, " ns="); p = decimal(p, elapsed);
    p = text(p, " checksum="); p = decimal(p, check); *p++ = '\n';
    output(buf, (size_t)(p - buf));
}
int main(void) {
    run("integer", integer);
    run("memory", memory);
    run("floating", floating);
    run("calls", calls);
    return 0;
}
#ifndef __APPLE__
__attribute__((noreturn)) void _start(void) {
    syscall3(94, main(), 0, 0);
    __builtin_unreachable();
}
#endif
