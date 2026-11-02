#ifdef OPERIT_NATIVE
#include <stdlib.h>
#include <unistd.h>
typedef unsigned long u64;
#else
#include "linux.h"
#endif
#include "cases.h"
static u64 input[48], output[46];
static const u64 edges[] = {0,
                            1,
                            ~0UL,
                            0x8000000000000000UL,
                            0x7fffffffffffffffUL,
                            0x8080808080808080UL,
                            0x7f7f7f7f7f7f7f7fUL,
                            0x8000800080008000UL,
                            0x7fff7fff7fff7fffUL,
                            0x8000000080000000UL,
                            0x7fffffff7fffffffUL,
                            0x0102030405060708UL,
                            0xff00ff00ff00ff00UL,
                            0x5555555555555555UL,
                            0xaaaaaaaaaaaaaaaaUL,
                            0x7ff0000000000001UL};
#ifdef OPERIT_NATIVE
int main(int argc, char **argv) {
    if (argc != 2)
        return 2;
    unsigned index = (unsigned)strtoul(argv[1], 0, 10);
#else
int guest_main(u64 *stack) {
    if (stack[0] != 2)
        return 2;
    const char *arg = (const char *)stack[2];
    unsigned index = 0;
    while (*arg >= '0' && *arg <= '9')
        index = index * 10 + (unsigned)(*arg++ - '0');
#endif
    if (index >= sizeof(samples) / sizeof(*samples))
        return 2;
    u64 rng = 0x147a023b965fee11UL;
    for (unsigned test = 0; test < records_per_case[index]; ++test) {
        for (unsigned j = 0; j < 48; ++j) {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            input[j] = test < 16 ? edges[(test + j / 2) % 16] : rng;
        }
        input[12] = (u64)(test % 16) << 28;
        static const u64 fp_controls[] = {0, 1UL << 22, 2UL << 22, 3UL << 22, 1UL << 24, 1UL << 25};
        input[13] = fp_controls[test / 80];
        samples[index](input, output);
#ifdef OPERIT_NATIVE
        if (write(1, output, sizeof(output)) != sizeof(output))
            return 3;
#else
        if (CALL(64, 1, output, sizeof(output)) != sizeof(output))
            return 3;
#endif
    }
    return 0;
}
