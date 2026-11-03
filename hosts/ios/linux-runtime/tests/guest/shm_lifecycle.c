#include "linux.h"

struct ipc_perm {
    int key;
    unsigned uid, gid, cuid, cgid, mode;
    unsigned short sequence, pad;
    u64 unused1, unused2;
};
struct shmid_ds {
    struct ipc_perm permissions;
    u64 size;
    i64 atime, dtime, ctime;
    int creator, last;
    u64 attachments, unused1, unused2;
};

int guest_main(u64 *stack) {
    (void)stack;
    const i64 id = CALL(194, 0, 8192, 0600);
    CHECK(id >= 0, 10);
    struct shmid_ds status = {0};
    CHECK(CALL(195, id, 2, &status) == 0 && status.size == 8192 && status.attachments == 0, 11);
    i64 address = CALL(196, id, 0, 0);
    CHECK(address > 0, 12);
    volatile unsigned *shared = (volatile unsigned *)(u64)address;
    shared[0] = 0x12345678;
    CHECK(CALL(195, id, 2, &status) == 0 && status.attachments == 1, 13);
    CHECK(call6(216, (u64)address, 8192, 16384, 1, 0, 0) == -95, 14);

    i64 child = CALL(220, 17, 0, 0);
    CHECK(child >= 0, 15);
    if (!child) {
        CHECK(shared[0] == 0x12345678, 20);
        CHECK(CALL(226, address + 4096, 4096, 1) == 0, 21);
        shared[0] = 0x87654321;
        CHECK(CALL(197, address, 0, 0) == 0, 22);
        return 0;
    }
    int child_status = -1;
    CHECK(call6(260, (u64)child, (u64)&child_status, 0, 0, 0, 0) == child && child_status == 0, 16);
    CHECK(shared[0] == 0x87654321, 17);
    CHECK(CALL(195, id, 2, &status) == 0 && status.attachments == 1, 18);
    CHECK(CALL(195, id, 0, 0) == 0, 19);
    CHECK(CALL(197, address, 0, 0) == 0, 23);
    CHECK(CALL(195, id, 2, &status) == -22, 24);
    print("sysv-shm-fork-mapping-lifecycle-ok\n");
    return 0;
}
