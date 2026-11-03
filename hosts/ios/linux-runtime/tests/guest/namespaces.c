#include "linux.h"

#define CLONE_FS 0x200UL
#define CLONE_FILES 0x400UL
#define CLONE_NEWNS 0x20000UL
#define CLONE_NEWNET 0x40000000UL
#define CLONE_NEWUSER 0x10000000UL

extern i64 namespace_spawn_thread(void (*function)(void), void *stack);
static unsigned char thread_stack[16384] __attribute__((aligned(16)));
static int thread_ready, thread_release, thread_finished, thread_tid = 1, thread_marker;
static i64 thread_fs, thread_files, thread_both, thread_namespace, thread_chdir, thread_close;

static void thread_body(void) {
    CHECK(CALL(96, &thread_tid, 0, 0) > 0, 42);
    __atomic_store_n(&thread_ready, 1, __ATOMIC_SEQ_CST);
    while (!__atomic_load_n(&thread_release, __ATOMIC_SEQ_CST))
        CALL(124, 0, 0, 0);
    thread_fs = CALL(97, CLONE_FS, 0, 0);
    thread_files = CALL(97, CLONE_FILES, 0, 0);
    thread_both = CALL(97, CLONE_FS | CLONE_FILES, 0, 0);
    thread_namespace = CALL(97, CLONE_FS | CLONE_FILES | CLONE_NEWNS, 0, 0);
    thread_chdir = CALL(49, "/child_dir", 0, 0);
    thread_close = CALL(57, thread_marker, 0, 0);
    __atomic_store_n(&thread_finished, 1, __ATOMIC_SEQ_CST);
}
static void send_word(int fd, int value) {
    CHECK(CALL(64, fd, &value, sizeof(value)) == sizeof(value), 40);
}
static int receive_word(int fd) {
    int value = -1;
    CHECK(CALL(63, fd, &value, sizeof(value)) == sizeof(value), 41);
    return value;
}
static int open_marker(void) {
    return (int)CALL(56, -100, "/dev/null", 0);
}
static void cwd_is(const char *expected, int code) {
    char cwd[128];
    CHECK(CALL(17, cwd, sizeof(cwd), 0) > 0 && equal(cwd, expected), code);
}

int guest_main(void) {
    struct { unsigned version, tid; } caps_header = {0x20080522, 0};
    unsigned caps[6] = {1, 1, 1, 1, 1, 1};
    CHECK(CALL(90, &caps_header, caps, 0) == 0, 43);
    for (int i = 0; i < 6; ++i)
        CHECK(caps[i] == 0, 44);
    const i64 mount_result = call6(40, (u64)"tmpfs", (u64)"/child_dir", (u64)"tmpfs", 0, 0, 0);
    const i64 unmount_result = CALL(39, "/proc", 2, 0);
    if (mount_result != -1) print("mount-permission-check-failed\n");
    if (unmount_result != -1) print("umount-permission-check-failed\n");
    CHECK(mount_result == -1, 70);
    CHECK(unmount_result == -1, 71);
    const i64 proc_fd = CALL(56, -100, "/proc/self/status", 0);
    CHECK(proc_fd >= 0 && CALL(57, proc_fd, 0, 0) == 0, 72);
    CHECK(CALL(97, 0, 0, 0) == 0, 1);
    CHECK(CALL(97, (u64)1 << 60, 0, 0) == -22, 2);
    CHECK(CALL(97, CLONE_NEWNS, 0, 0) == -1, 3);
    CHECK(CALL(97, CLONE_NEWUSER, 0, 0) == -1, 4);
    CHECK(CALL(97, CLONE_NEWNET, 0, 0) == -1, 5);
    CHECK(CALL(268, -1, 0, 0) == -9, 6);
    CHECK(CALL(268, 1, 0, 0) == -22, 7);
    CHECK(CALL(56, -100, "/proc/self/ns/mnt", 0) == -2, 45);
    CHECK(call6(220, CLONE_NEWNS | CLONE_FS | 17, 0, 0, 0, 0, 0) == -22, 46);
    CHECK(call6(220, CLONE_NEWNS | 17, 0, 0, 0, 0, 0) == -1, 47);
    CHECK(call6(220, CLONE_NEWUSER | 17, 0, 0, 0, 0, 0) == -1, 48);

    int ready[2], command[2];
    CHECK(CALL(59, ready, 0, 0) == 0 && CALL(59, command, 0, 0) == 0, 8);
    int marker = open_marker();
    CHECK(marker >= 0, 9);
    i64 child = call6(220, CLONE_FS | CLONE_FILES | 17, 0, 0, 0, 0, 0);
    CHECK(child >= 0, 10);
    if (!child) {
        CHECK(CALL(49, "/child_dir", 0, 0) == 0, 11);
        CHECK(CALL(57, marker, 0, 0) == 0, 12);
        send_word(ready[1], 1);
        marker = receive_word(command[0]);
        CHECK(CALL(25, marker, 1, 0) >= 0, 13);
        CHECK(CALL(49, "/child_dir", 0, 0) == 0, 49);
        CHECK(CALL(57, marker, 0, 0) == 0, 50);
        send_word(ready[1], 2);
        marker = receive_word(command[0]);
        CHECK(CALL(97, CLONE_FS, 0, 0) == 0, 14);
        CHECK(CALL(49, "/child_dir", 0, 0) == 0, 16);
        CHECK(CALL(166, 077, 0, 0) == 022, 51);
        CHECK(CALL(57, marker, 0, 0) == 0, 52);
        cwd_is("/child_dir", 17);
        send_word(ready[1], 3);
        marker = receive_word(command[0]);
        CHECK(CALL(97, CLONE_FILES, 0, 0) == 0, 53);
        CHECK(CALL(57, marker, 0, 0) == 0 && CALL(25, marker, 1, 0) == -9, 15);
        CHECK(CALL(97, CLONE_FS | CLONE_FILES, 0, 0) == 0, 54);
        send_word(ready[1], 4);
        return 0;
    }
    CHECK(receive_word(ready[0]) == 1, 18);
    cwd_is("/child_dir", 19);
    CHECK(CALL(25, marker, 1, 0) == -9, 20);
    marker = open_marker();
    CHECK(marker >= 0 && CALL(49, "/", 0, 0) == 0, 21);
    CHECK(CALL(97, CLONE_FS | CLONE_FILES | CLONE_NEWNS, 0, 0) == -1, 55);
    send_word(command[1], marker);
    CHECK(receive_word(ready[0]) == 2, 22);
    cwd_is("/child_dir", 56);
    CHECK(CALL(25, marker, 1, 0) == -9, 57);
    marker = open_marker();
    CHECK(marker >= 0 && CALL(49, "/", 0, 0) == 0, 58);
    send_word(command[1], marker);
    CHECK(receive_word(ready[0]) == 3, 59);
    cwd_is("/", 23);
    CHECK(CALL(166, 022, 0, 0) == 022, 60);
    CHECK(CALL(25, marker, 1, 0) == -9, 61);
    marker = open_marker();
    CHECK(marker >= 0, 62);
    send_word(command[1], marker);
    CHECK(receive_word(ready[0]) == 4, 63);
    CHECK(CALL(25, marker, 1, 0) >= 0, 24);
    int status = -1;
    CHECK(CALL(260, child, &status, 0) == child && status == 0, 25);

    thread_marker = open_marker();
    CHECK(thread_marker >= 0, 64);
    CHECK(namespace_spawn_thread(thread_body, thread_stack + sizeof(thread_stack)) > 0, 26);
    while (!__atomic_load_n(&thread_ready, __ATOMIC_SEQ_CST))
        CALL(124, 0, 0, 0);
    CHECK(CALL(97, CLONE_FS, 0, 0) == -22, 27);
    CHECK(CALL(97, CLONE_FILES, 0, 0) == -22, 28);
    CHECK(CALL(97, CLONE_FS | CLONE_FILES, 0, 0) == -22, 65);
    CHECK(CALL(97, 0, 0, 0) == 0, 29);
    __atomic_store_n(&thread_release, 1, __ATOMIC_SEQ_CST);
    while (!__atomic_load_n(&thread_finished, __ATOMIC_SEQ_CST))
        CALL(124, 0, 0, 0);
    CHECK(thread_fs == -22 && thread_files == -22 && thread_both == -22, 66);
    CHECK(thread_namespace == -1 && thread_chdir == 0 && thread_close == 0, 67);
    cwd_is("/child_dir", 68);
    CHECK(CALL(25, thread_marker, 1, 0) == -9, 69);
    while (__atomic_load_n(&thread_tid, __ATOMIC_SEQ_CST))
        CALL(124, 0, 0, 0);
    CHECK(CALL(97, CLONE_FS | CLONE_FILES, 0, 0) == 0, 30);
    print("namespaces-ok\n");
    return 0;
}
