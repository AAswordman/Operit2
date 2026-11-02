#pragma once
typedef unsigned long u64;
typedef long i64;
static inline i64 call6(u64 nr, u64 a, u64 b, u64 c, u64 d, u64 e, u64 f) {
    register u64 x0 __asm__("x0") = a, x1 __asm__("x1") = b, x2 __asm__("x2") = c;
    register u64 x3 __asm__("x3") = d, x4 __asm__("x4") = e, x5 __asm__("x5") = f, x8 __asm__("x8") = nr;
    __asm__ volatile("svc #0"
                     : "+r"(x0)
                     : "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5), "r"(x8)
                     : "memory", "cc");
    return (i64)x0;
}
#define CALL(n, a, b, c) call6(n, (u64)(a), (u64)(b), (u64)(c), 0, 0, 0)
static inline void fail(int code) {
    CALL(93, code, 0, 0);
    __builtin_unreachable();
}
#define CHECK(c, n)                                                                                          \
    do {                                                                                                     \
        if (!(c))                                                                                            \
            fail(n);                                                                                         \
    } while (0)
static inline u64 length(const char *p) {
    u64 n = 0;
    while (p[n])
        ++n;
    return n;
}
static inline int equal(const char *a, const char *b) {
    while (*a && *a == *b) {
        ++a;
        ++b;
    }
    return *a == *b;
}
static inline void print(const char *s) {
    CHECK(CALL(64, 1, s, length(s)) == (i64)length(s), 90);
}
