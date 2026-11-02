#ifdef OPERIT_NATIVE
#include <unistd.h>
typedef unsigned long u64;
#else
#include "linux.h"
#endif
extern void exercise(u64 a, u64 b, u64 *out);
static u64 output[104];
static u64 inputs[] = {0,
                       1,
                       2,
                       0x7fffffffffffffffUL,
                       0x8000000000000000UL,
                       0xffffffffffffffffUL,
                       0xffffffffUL,
                       0x0123456789abcdefUL,
                       0xfedcba9876543210UL};
#ifdef OPERIT_NATIVE
int main(void)
#else
int guest_main(u64 *stack)
#endif
{
#ifndef OPERIT_NATIVE
    (void)stack;
#endif
    for (unsigned a = 0; a < sizeof(inputs) / sizeof(*inputs); ++a) {
        for (unsigned b = 0; b < sizeof(inputs) / sizeof(*inputs); ++b) {
            exercise(inputs[a], inputs[b], output);
#ifdef OPERIT_NATIVE
            if (write(1, output, sizeof(output)) != sizeof(output))
                return 1;
#else
            CHECK(CALL(64, 1, output, sizeof(output)) == sizeof(output), 91);
#endif
        }
    }
    return 0;
}
