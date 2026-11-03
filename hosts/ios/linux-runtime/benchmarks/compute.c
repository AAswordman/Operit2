#define _POSIX_C_SOURCE 200809L
#include <arm_neon.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static double now(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts))
        abort();
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

static void *(*volatile copy_bytes)(void *, const void *, size_t) = memcpy;

int main(int argc, char **argv) {
    if (argc < 2 || argc > 3 || (argc == 3 && strcmp(argv[1], "fork")))
        return 2;
    const char *mode = argv[1];
    if (!strcmp(mode, "integer")) {
        uint64_t value = 88172645463325252ull;
        const uint64_t count = 100000000;
        const double start = now();
        for (uint64_t i = 0; i < count; ++i) {
            value ^= i;
            value = (value << 13) | (value >> 51);
            value *= 0x9e3779b97f4a7c15ull;
        }
        printf("mode=%s count=%llu seconds=%.6f result=%016llx\n", mode,
               (unsigned long long)count, now() - start, (unsigned long long)value);
    } else if (!strcmp(mode, "sqrt")) {
        const uint64_t count = 20000000;
        double value = 0;
        const double start = now();
        for (uint64_t i = 1; i <= count; ++i)
            value += sqrt((double)i);
        printf("mode=%s count=%llu seconds=%.6f result=%.17g\n", mode,
               (unsigned long long)count, now() - start, value);
    } else if (!strcmp(mode, "fma")) {
        const uint64_t count = 100000000;
        double value = 1;
        const double start = now();
        for (uint64_t i = 0; i < count; ++i) {
            value = fma(value, 1.0000000001, 0.5);
            if (value > 1e18)
                value = 1;
        }
        printf("mode=%s count=%llu seconds=%.6f result=%.17g\n", mode,
               (unsigned long long)count, now() - start, value);
    } else if (!strcmp(mode, "neon")) {
        double a[4096], b[4096];
        for (unsigned i = 0; i < 4096; ++i) {
            a[i] = (double)(i % 128) / 128;
            b[i] = (double)(i % 37) / 32;
        }
        float64x2_t value = vdupq_n_f64(0);
        const unsigned rounds = 20000;
        const double start = now();
        for (unsigned round = 0; round < rounds; ++round)
            for (unsigned i = 0; i < 4096; i += 2)
                value = vfmaq_f64(value, vld1q_f64(a + i), vld1q_f64(b + i));
        const double elapsed = now() - start;
        const double result = vgetq_lane_f64(value, 0) + vgetq_lane_f64(value, 1);
        printf("mode=%s count=%u seconds=%.6f result=%.17g\n", mode, rounds * 4096, elapsed, result);
    } else if (!strcmp(mode, "memcpy")) {
        const size_t size = 64u << 20, rounds = 10;
        unsigned char *source = malloc(size), *destination = malloc(size);
        if (!source || !destination)
            return 3;
        memset(source, 0x5a, size);
        memset(destination, 0, size);
        uint64_t checksum = 0;
        const double start = now();
        for (size_t i = 0; i < rounds; ++i) {
            source[0] = (unsigned char)i;
            copy_bytes(destination, source, size);
            checksum += destination[0] + destination[size - 1];
        }
        const double elapsed = now() - start;
        const int valid = !memcmp(source, destination, size);
        printf("mode=%s bytes=%zu seconds=%.6f result=%llu valid=%d\n", mode, size * rounds,
               elapsed, (unsigned long long)checksum, valid);
        free(source);
        free(destination);
        if (!valid)
            return 4;
    } else if (!strcmp(mode, "fork")) {
        unsigned count = 60;
        if (argc == 3) {
            char *end;
            const unsigned long parsed = strtoul(argv[2], &end, 10);
            if (!*argv[2] || *end || !parsed || parsed > 1000000)
                return 2;
            count = (unsigned)parsed;
        }
        unsigned completed = 0;
        const double start = now();
        for (unsigned i = 0; i < count; ++i) {
            const pid_t child = fork();
            if (child == 0) {
#if defined(__APPLE__)
                execl("/usr/bin/true", "true", (char *)0);
#else
                execl("/bin/true", "true", (char *)0);
#endif
                _exit(127);
            }
            int status;
            if (child > 0 && waitpid(child, &status, 0) == child && WIFEXITED(status) &&
                WEXITSTATUS(status) == 0)
                ++completed;
        }
        printf("mode=%s count=%u seconds=%.6f result=%u\n", mode, count, now() - start, completed);
        if (completed != count)
            return 5;
    } else {
        return 2;
    }
    return 0;
}
