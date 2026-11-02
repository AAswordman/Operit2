#include "linux.h"
int guest_main(u64 *stack) {
    CHECK(stack[0] >= 1, 1);
    CHECK(((u64)stack & 15) == 0, 2);
    print("hello-arm64\n");
    return 0;
}
