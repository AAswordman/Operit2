#include "linux.h"

struct event {
    int watch;
    unsigned mask, cookie, length;
    char name[];
};

static void expect_event(int fd, int watch, unsigned mask, const char *name, int check) {
    unsigned char buffer[4096] __attribute__((aligned(8)));
    const i64 size = CALL(63, fd, buffer, sizeof(buffer));
    CHECK(size >= 16, check);
    int found = 0;
    for (u64 offset = 0; offset + 16 <= (u64)size;) {
        const struct event *event = (const struct event *)(buffer + offset);
        CHECK(event->length <= (u64)size - offset - 16, check + 1);
        CHECK(!(event->mask & 0x4000), check + 1);
        if (event->watch == watch && (event->mask & mask) &&
            (!name || (event->length && equal(event->name, name))))
            found = 1;
        offset += 16 + event->length;
    }
    CHECK(found, check + 2);
}

static void data_lifecycle(void) {
    const int notify = CALL(26, 2048, 0, 0);
    CHECK(notify >= 0, 100);
    const int watch = CALL(27, notify, "/data/inotify-probe", 0x39);
    CHECK(watch >= 0, 101);
    const int opened = call6(56, -100, (u64)"/data/inotify-probe/lifecycle", 64 | 2, 0600, 0, 0);
    CHECK(opened >= 0, 102);
    expect_event(notify, watch, 0x20, "lifecycle", 103);
    CHECK(CALL(64, opened, "hello", 5) == 5, 106);
    const int duplicate = CALL(23, opened, 0, 0);
    CHECK(duplicate >= 0, 107);
    unsigned char buffer[4096];
    CHECK(CALL(63, notify, buffer, sizeof(buffer)) == -11, 108);
    CHECK(call6(67, opened, (u64)buffer, 1, 0, 0, 0) == 1 && buffer[0] == 'h', 109);
    expect_event(notify, watch, 1, "lifecycle", 110);
    // Renaming between hardlinks is a no-op; it must not inject a snapshot
    // overflow or move the dentry retained by the shared open description.
    CHECK(call6(37, -100, (u64)"/data/inotify-probe/lifecycle", -100,
                (u64)"/data/inotify-probe/alias", 0, 0) == 0, 132);
    CHECK(call6(38, -100, (u64)"/data/inotify-probe/lifecycle", -100,
                (u64)"/data/inotify-probe/alias", 0, 0) == 0, 133);
    CHECK(CALL(63, notify, buffer, sizeof(buffer)) == -11, 134);
    CHECK(CALL(35, -100, "/data/inotify-probe/alias", 0) == 0, 135);
    CHECK(CALL(63, notify, buffer, sizeof(buffer)) == -11, 136);
    CHECK(CALL(57, opened, 0, 0) == 0, 113);
    CHECK(CALL(63, notify, buffer, sizeof(buffer)) == -11, 114);
    CHECK(call6(38, -100, (u64)"/data/inotify-probe/lifecycle", -100,
                (u64)"/data/inotify-probe/renamed", 0, 0) == 0, 115);
    CHECK(CALL(57, duplicate, 0, 0) == 0, 116);
    expect_event(notify, watch, 8, "renamed", 117);
    const int reader = call6(56, -100, (u64)"/data/inotify-probe/renamed", 0, 0, 0, 0);
    CHECK(reader >= 0, 120);
    expect_event(notify, watch, 0x20, "renamed", 121);
    CHECK(CALL(57, reader, 0, 0) == 0, 124);
    expect_event(notify, watch, 0x10, "renamed", 125);
    const int path = call6(56, -100, (u64)"/data/inotify-probe/renamed", 0x200000, 0, 0, 0);
    CHECK(path >= 0 && CALL(57, path, 0, 0) == 0, 128);
    CHECK(CALL(63, notify, buffer, sizeof(buffer)) == -11, 129);
    CHECK(CALL(35, -100, "/data/inotify-probe/renamed", 0) == 0, 130);
    CHECK(CALL(57, notify, 0, 0) == 0, 131);
}

int guest_main(u64 *stack) {
    (void)stack;
    const char *directory = "/data/inotify-probe";
    CHECK(CALL(34, -100, directory, 0700) == 0, 10);
    CHECK(CALL(35, -100, directory, 0) == -21, 38);
    const int fd = CALL(26, 2048 | 524288, 0, 0);
    CHECK(fd >= 0, 11);
    u64 statfs[15] = {0};
    CHECK(CALL(44, fd, statfs, 0) == 0 && statfs[0] == 0x09041934, 35);
    CHECK(call6(13, fd, 0, 0, 0, 0, 0) == 0, 36);
    CHECK(call6(10, fd, (u64)"user.probe", 0, 0, 0, 0) == -95, 37);
    const unsigned mask = 0x100 | 0x200 | 0x40 | 0x80;
    const int watch = CALL(27, fd, directory, mask);
    CHECK(watch >= 0, 12);
    unsigned char buffer[4096];
    CHECK(CALL(63, fd, buffer, sizeof(buffer)) == -11, 13);
    CHECK(CALL(64, fd, buffer, 1) == -9, 14);
    CHECK(call6(67, fd, (u64)buffer, 16, 0, 0, 0) == -29, 15);
    const int created = call6(56, -100, (u64)"/data/inotify-probe/first", 64 | 2, 0600, 0, 0);
    CHECK(created >= 0, 16);
    expect_event(fd, watch, 0x100, "first", 17);
    CHECK(CALL(64, created, "hello", 5) == 5 && CALL(57, created, 0, 0) == 0, 20);
    CHECK(call6(38, -100, (u64)"/data/inotify-probe/first", -100,
                (u64)"/data/inotify-probe/second", 0, 0) == 0, 21);
    const i64 size = CALL(63, fd, buffer, sizeof(buffer));
    CHECK(size >= 32, 22);
    unsigned from = 0, to = 0;
    for (u64 offset = 0; offset + 16 <= (u64)size;) {
        const struct event *event = (const struct event *)(buffer + offset);
        CHECK(event->length <= (u64)size - offset - 16, 23);
        CHECK(!(event->mask & 0x4000), 23);
        if (event->watch == watch && (event->mask & 0x40) &&
            event->length && equal(event->name, "first"))
            from = event->cookie;
        if (event->watch == watch && (event->mask & 0x80) &&
            event->length && equal(event->name, "second"))
            to = event->cookie;
        offset += 16 + event->length;
    }
    CHECK(from && to == from, 24);
    CHECK(CALL(35, -100, "/data/inotify-probe/second", 0) == 0, 25);
    expect_event(fd, watch, 0x200, "second", 26);
    CHECK(CALL(28, fd, watch, 0) == 0, 29);
    expect_event(fd, watch, 0x8000, 0, 30);
    CHECK(CALL(57, fd, 0, 0) == 0, 33);
    data_lifecycle();
    CHECK(CALL(35, -100, directory, 0x200) == 0, 34);
    print("inotify-files-rename-cookie-data-lifecycle-ok\n");
    return 0;
}
