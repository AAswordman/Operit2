#ifdef OPERIT_NATIVE
#include <unistd.h>
typedef unsigned long u64;
#else
#include "linux.h"
#endif
extern void exercise_fp(u64 a, u64 b, u64 *out, u64 fpcr);
static u64 output[276];
static const u64 inputs[] = {0,
                             0x8000000000000000UL,
                             0x3ff0000000000000UL,
                             0xbff8000000000000UL,
                             0x4004000000000000UL,
                             0x7ff0000000000000UL,
                             0xfff0000000000000UL,
                             0x7ff8000000000001UL,
                             0x7ff0000000000001UL,
                             1,
                             0xffffffffffffffffUL,
                             0x0123456789abcdefUL,
                             0x3f800000UL,
                             0x7f800001UL};
static const u64 controls[] = {0, 1UL << 22, 2UL << 22, 3UL << 22, 1UL << 24, 1UL << 25};
#ifdef OPERIT_NATIVE
int main(void)
#else
int guest_main(u64 *stack)
#endif
{
#ifndef OPERIT_NATIVE
    (void)stack;
#endif
    for (unsigned control = 0; control < sizeof(controls) / sizeof(*controls); ++control)
        for (unsigned a = 0; a < sizeof(inputs) / sizeof(*inputs); ++a)
            for (unsigned b = 0; b < sizeof(inputs) / sizeof(*inputs); ++b) {
                exercise_fp(inputs[a], inputs[b], output, controls[control]);
#ifdef OPERIT_NATIVE
                if (write(1, output, sizeof(output)) != sizeof(output))
                    return 1;
#else
                CHECK(CALL(64, 1, output, sizeof(output)) == sizeof(output), 92);
#endif
            }
    return 0;
}
