#include "linux/events/inotify.h"
#include "linux/syscalls.h"
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <initializer_list>
#include <stdexcept>
#include <vector>

using namespace operit::arm64;
namespace {
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Event {
    int32_t wd;
    uint32_t mask, cookie;
    std::string name;
};
struct Fixture {
    std::string root;
    Memory memory{8 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler scheduler;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/operit-inotify-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        root = created;
        std::filesystem::create_directory(root + "/data");
        std::ofstream(root + "/data/a") << "abcdefgh";
        std::ofstream(root + "/data/b") << "replaced";
        fs = std::make_unique<Filesystem>(root.c_str());
        memory.map(0x10000, 0x20000, read_perm | write_perm);
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, *fs, config, scheduler);
    }
    ~Fixture() {
        sys.reset();
        std::filesystem::remove_all(root);
    }
    uint64_t text(const std::string &value, uint64_t address = 0x10000) {
        memory.copy_in(address, value.c_str(), value.size() + 1);
        return address;
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> args = std::initializer_list<uint64_t>()) {
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        std::copy(args.begin(), args.end(), cpu.x.begin());
        cpu.x[8] = number;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    int instance() {
        const int fd = int(call(26, {inotify::nonblock}));
        require(fd >= 0, "inotify_init1");
        return fd;
    }
    int watch(int fd, const std::string &path, uint32_t mask) {
        const int wd = int(call(27, {uint64_t(fd), text(path), mask}));
        require(wd >= 0, "data-event mask rejected by inotify_add_watch");
        return wd;
    }
    int open(const std::string &path, uint64_t flags = 0) {
        const int fd = int(call(56, {uint64_t(-100), text(path), flags, 0600}));
        require(fd >= 0, "openat");
        return fd;
    }
    void close(int fd) { require(call(57, {uint64_t(fd)}) == 0, "close"); }
    void rename(const std::string &from, const std::string &to) {
        const auto source = text(from), target = text(to, 0x11000);
        require(call(38, {uint64_t(-100), source, uint64_t(-100), target}) == 0, "renameat");
    }
    void unlink(const std::string &path) {
        require(call(35, {uint64_t(-100), text(path), 0}) == 0, "unlinkat");
    }
    std::vector<Event> events(int fd) {
        const int64_t used = call(63, {uint64_t(fd), 0x18000, 0x4000});
        if (used == -11)
            return {};
        require(used > 0, "inotify read");
        std::vector<Event> result;
        for (uint64_t cursor = 0; cursor < uint64_t(used);) {
            require(uint64_t(used) - cursor >= 16, "truncated inotify header");
            const uint64_t address = 0x18000 + cursor;
            const int32_t wd = int32_t(memory.load(address, 4));
            const auto mask = uint32_t(memory.load(address + 4, 4));
            const auto length = uint32_t(memory.load(address + 12, 4));
            require(length <= uint64_t(used) - cursor - 16, "truncated inotify name");
            require(!(mask & inotify::overflow), "unexpected snapshot/bridge overflow");
            result.push_back({wd, mask, uint32_t(memory.load(address + 8, 4)),
                              length ? memory.string(address + 16) : ""});
            cursor += 16 + length;
        }
        return result;
    }
};
size_t count(const std::vector<Event> &events, int wd, uint32_t mask, const std::string &name = "") {
    return std::count_if(events.begin(), events.end(), [&](const Event &event) {
        return event.wd == wd && (event.mask & mask) && event.name == name;
    });
}
constexpr uint32_t data_events = inotify::open | inotify::access | inotify::close_write | inotify::close_nowrite;
void description_lifecycle() {
    Fixture f;
    const int notify = f.instance(), directory = f.watch(notify, "/data", data_events);
    const int inode = f.watch(notify, "/data/a", data_events);
    const int fd = f.open("/data/a");
    auto events = f.events(notify);
    require(count(events, directory, inotify::open, "a") == 1 && count(events, inode, inotify::open) == 1,
            "OPEN must reach the exact dentry and direct inode watch");
    const int duplicate = int(f.call(23, {uint64_t(fd)}));
    require(duplicate >= 0 && f.events(notify).empty(), "dup must not generate OPEN");
    require(f.call(63, {uint64_t(fd), 0x14000, 1}) == 1, "read");
    events = f.events(notify);
    require(count(events, directory, inotify::access, "a") == 1 && count(events, inode, inotify::access) == 1,
            "scalar read did not generate ACCESS");
    require(f.call(67, {uint64_t(fd), 0x14000, 1, 0}) == 1, "pread");
    require(count(f.events(notify), directory, inotify::access, "a") == 1, "pread ACCESS");
    f.memory.store(0x12000, 0x14000, 8);
    f.memory.store(0x12008, 2, 8);
    require(f.call(65, {uint64_t(duplicate), 0x12000, 1}) == 2, "readv");
    require(count(f.events(notify), directory, inotify::access, "a") == 1, "readv ACCESS");
    require(f.call(67, {uint64_t(fd), 0x14000, 1, 4096}) == 0 && f.events(notify).empty(),
            "scalar EOF must not generate ACCESS");
    // Linux vfs_readv reports ACCESS even for successful EOF / empty vectors.
    require(f.call(69, {uint64_t(fd), 0x12000, 1, 4096}) == 0, "preadv EOF");
    require(count(f.events(notify), directory, inotify::access, "a") == 1, "preadv EOF ACCESS");
    require(f.call(65, {uint64_t(fd), 0x12000, 0}) == 0, "empty readv");
    require(count(f.events(notify), directory, inotify::access, "a") == 1, "empty readv ACCESS");
    f.close(fd);
    require(f.events(notify).empty(), "intermediate dup close generated CLOSE");
    f.close(duplicate);
    events = f.events(notify);
    require(count(events, directory, inotify::close_nowrite, "a") == 1 &&
                count(events, inode, inotify::close_nowrite) == 1,
            "final read-only close must generate one CLOSE_NOWRITE");
    const int writer = f.open("/data/a", 2);
    f.events(notify);
    f.close(writer);
    events = f.events(notify);
    require(count(events, directory, inotify::close_write, "a") == 1 &&
                !count(events, directory, inotify::close_nowrite, "a"),
            "CLOSE_WRITE depends on access mode, even without writes");
    const int path_only = f.open("/data/a", 0x200000);
    f.close(path_only);
    require(f.events(notify).empty(), "O_PATH generated data-open events");
}
void hardlinks_and_rename() {
    Fixture f;
    std::filesystem::create_hard_link(f.root + "/data/a", f.root + "/data/alias");
    const int notify = f.instance(), wd = f.watch(notify, "/data", data_events);
    const int fd = f.open("/data/a");
    auto events = f.events(notify);
    require(count(events, wd, inotify::open, "a") == 1 && !count(events, wd, inotify::open, "alias"),
            "inode matching falsely delivered OPEN to a sibling hardlink");
    f.rename("/data/a", "/data/alias"); // Linux hardlink no-op: both names survive.
    f.close(fd);
    events = f.events(notify);
    require(count(events, wd, inotify::close_nowrite, "a") == 1 &&
                !count(events, wd, inotify::close_nowrite, "alias"),
            "same-inode rename incorrectly moved the open dentry");
    const int renamed = f.open("/data/a");
    f.events(notify);
    f.rename("/data/a", "/data/renamed");
    require(f.call(67, {uint64_t(renamed), 0x14000, 1, 0}) == 1, "read after rename");
    events = f.events(notify);
    require(count(events, wd, inotify::access, "renamed") == 1 && !count(events, wd, inotify::access, "a"),
            "ACCESS after rename retained an obsolete filename");
    f.close(renamed);
    require(count(f.events(notify), wd, inotify::close_nowrite, "renamed") == 1,
            "CLOSE after rename retained an obsolete filename");
}
void noop_rename_notifications() {
    Fixture f;
    std::filesystem::create_directory(f.root + "/other");
    std::filesystem::create_hard_link(f.root + "/data/a", f.root + "/data/alias");
    std::filesystem::create_hard_link(f.root + "/data/a", f.root + "/other/alias");
    const int notify = f.instance();
    f.watch(notify, "/data", inotify::all_events);
    f.watch(notify, "/other", inotify::all_events);
    f.watch(notify, "/data/a", inotify::all_events);
    f.rename("/data/a", "/data/alias");
    require(f.events(notify).empty(), "same-inode rename generated events");
    f.rename("/data/a", "/other/alias");
    require(f.events(notify).empty(), "cross-directory same-inode rename generated events");
    f.rename("/data/a", "/data/a");
    require(f.events(notify).empty(), "same-path rename generated events");
    require(f.call(276, {uint64_t(-100), f.text("/data/a"), uint64_t(-100),
                         f.text("/data/alias", 0x11000), 2}) == 0, "same-inode rename exchange");
    require(f.events(notify).empty(), "same-inode rename exchange generated events");
    require(std::filesystem::exists(f.root + "/data/a") &&
                std::filesystem::exists(f.root + "/data/alias") &&
                std::filesystem::exists(f.root + "/other/alias"),
            "no-op rename removed a hardlink");
}
void real_rename_notifications() {
    Fixture f;
    const int notify = f.instance();
    const int parent = f.watch(notify, "/data", inotify::all_events);
    const int inode = f.watch(notify, "/data/a", inotify::all_events);
    f.rename("/data/a", "/data/moved");
    auto events = f.events(notify);
    require(count(events, parent, inotify::moved_from, "a") == 1 &&
                count(events, parent, inotify::moved_to, "moved") == 1 &&
                count(events, inode, inotify::move_self) == 1,
            "real file rename must preserve MOVED_FROM / MOVED_TO / MOVE_SELF");
    f.rename("/data", "/relocated");
    require(count(f.events(notify), parent, inotify::move_self) == 1,
            "real directory rename must preserve MOVE_SELF");
}
uint32_t move_cookie(const std::vector<Event> &events, int from_wd, const std::string &from,
                     int to_wd, const std::string &to) {
    for (const auto &source : events) {
        if (source.wd != from_wd || !(source.mask & inotify::moved_from) ||
            source.name != from || !source.cookie)
            continue;
        for (const auto &target : events)
            if (target.wd == to_wd && (target.mask & inotify::moved_to) &&
                target.name == to && target.cookie == source.cookie)
                return source.cookie;
    }
    throw std::runtime_error("exchange must report both ends with matching nonzero cookies");
}
void exchange_notifications() {
    Fixture f;
    std::filesystem::create_directory(f.root + "/other");
    std::ofstream(f.root + "/other/c") << "other";
    const int notify = f.instance(), second = f.instance();
    const int parent = f.watch(notify, "/data", inotify::all_events);
    const int other = f.watch(notify, "/other", inotify::all_events);
    const int inode_a = f.watch(notify, "/data/a", inotify::all_events);
    const int inode_b = f.watch(notify, "/data/b", inotify::all_events);
    const int second_parent = f.watch(second, "/data", inotify::moved_from | inotify::moved_to);
    const int second_other = f.watch(second, "/other", inotify::moved_from | inotify::moved_to);
    f.rename("/data/a", "/data/c");
    f.events(notify);
    f.events(second);
    auto exchange = [&](const std::string &source, const std::string &target) {
        require(f.call(276, {uint64_t(-100), f.text(source), uint64_t(-100),
                             f.text(target, 0x11000), 2}) == 0, "rename exchange");
    };
    exchange("/data/c", "/data/b");
    auto events = f.events(notify);
    const auto forward = move_cookie(events, parent, "c", parent, "b");
    const auto reverse = move_cookie(events, parent, "b", parent, "c");
    require(forward != reverse, "exchange directions must have distinct cookies");
    require(count(events, inode_a, inotify::move_self) == 1 &&
                count(events, inode_b, inotify::move_self) == 1,
            "both exchanged inode watches must receive MOVE_SELF");
    require(events.size() == 6, "exchange generated extra events or retired a surviving inode");
    events = f.events(second);
    require(move_cookie(events, second_parent, "c", second_parent, "b") !=
                move_cookie(events, second_parent, "b", second_parent, "c") && events.size() == 4,
            "exchange must notify every inotify instance");
    exchange("/data/c", "/other/c");
    events = f.events(notify);
    require(move_cookie(events, parent, "c", other, "c") !=
                move_cookie(events, other, "c", parent, "c"),
            "cross-directory exchange directions must have distinct cookies");
    require(count(events, inode_b, inotify::move_self) == 1 && events.size() == 5,
            "cross-directory exchange must preserve the watched inode");
    events = f.events(second);
    require(move_cookie(events, second_parent, "c", second_other, "c") !=
                move_cookie(events, second_other, "c", second_parent, "c") && events.size() == 4,
            "cross-directory exchange must notify every inotify instance");
}
void snapshot_and_queue_overflow() {
    Fixture f;
#ifdef __APPLE__
    // Mutate the host directly so no guest safe point can observe the
    // intermediate name. Darwin must still report the lost snapshot events.
    InotifyFile snapshot(inotify::nonblock);
    snapshot.add_watch(*f.fs, "/data", inotify::all_events);
    std::ofstream(f.root + "/data/transient") << "transient";
    std::filesystem::remove(f.root + "/data/transient");
    uint32_t record[4]{};
    require(snapshot.read(record, sizeof(record)) == sizeof(record) &&
                int32_t(record[0]) == -1 && record[1] == inotify::overflow &&
                record[2] == 0 && record[3] == 0,
            "transient names lost between censuses must report OVERFLOW");
    require(!snapshot.try_read(record, sizeof(record)), "snapshot OVERFLOW must be consumed once");
#endif
    InotifyLimits limits;
    limits.queued_events = 1;
    InotifyFile bounded(inotify::nonblock, limits);
    const int wd = bounded.add_watch(*f.fs, "/data", inotify::create | inotify::remove);
    std::ofstream(f.root + "/data/queued") << "queued";
    bounded.pump();
    std::filesystem::remove(f.root + "/data/queued");
    bounded.pump();
    uint32_t records[12]{}; // Named CREATE (32 bytes), then OVERFLOW (16 bytes).
    require(bounded.read(records, sizeof(records)) == sizeof(records) &&
                int32_t(records[0]) == wd && records[1] == inotify::create && records[3] == 16 &&
                int32_t(records[8]) == -1 && records[9] == inotify::overflow &&
                records[10] == 0 && records[11] == 0,
            "queue exhaustion must retain CREATE and report OVERFLOW");
    require(!bounded.try_read(records, sizeof(records)), "queue OVERFLOW must be consumed once");
}
void cross_directory_moves() {
    Fixture f;
    std::filesystem::create_directory(f.root + "/other");
    const int notify = f.instance();
    const int source = f.watch(notify, "/data", data_events);
    const int target = f.watch(notify, "/other", data_events);
    const int fd = f.open("/data/a");
    f.events(notify);
    f.rename("/data/a", "/other/moved");
    require(f.call(67, {uint64_t(fd), 0x14000, 1, 0}) == 1, "read after cross-directory rename");
    auto events = f.events(notify);
    require(count(events, target, inotify::access, "moved") == 1 && !count(events, source, inotify::access, "a"),
            "cross-directory ACCESS notified the old parent");
    f.rename("/other", "/relocated");
    require(f.call(67, {uint64_t(fd), 0x14000, 1, 0}) == 1, "read after parent rename");
    require(count(f.events(notify), target, inotify::access, "moved") == 1,
            "renaming the parent invalidated its pinned identity");
    f.close(fd);
    require(count(f.events(notify), target, inotify::close_nowrite, "moved") == 1,
            "cross-directory CLOSE lost its pinned parent");
}
void unlinked_descriptions() {
    Fixture f;
    const int ordinary = f.instance(), excluded = f.instance();
    const int parent = f.watch(ordinary, "/data", data_events);
    const int filtered = f.watch(excluded, "/data", data_events | inotify::exclude_unlinked);
    const int inode = f.watch(ordinary, "/data/a", data_events | inotify::delete_self);
    const int fd = f.open("/data/a");
    f.events(ordinary);
    f.events(excluded);
    f.unlink("/data/a");
    auto events = f.events(ordinary);
    require(!count(events, inode, inotify::delete_self) && !count(events, inode, inotify::ignored),
            "unlinked inode watch retired before the last open description");
    f.events(excluded);
    require(f.call(67, {uint64_t(fd), 0x14000, 1, 0}) == 1, "read unlinked file");
    events = f.events(ordinary);
    require(count(events, parent, inotify::access, "a") == 1 && count(events, inode, inotify::access) == 1,
            "unlinked dentry lost ACCESS without IN_EXCL_UNLINK");
    require(!count(f.events(excluded), filtered, inotify::access, "a"), "IN_EXCL_UNLINK ignored for ACCESS");
    f.close(fd);
    events = f.events(ordinary);
    require(count(events, parent, inotify::close_nowrite, "a") == 1 &&
                count(events, inode, inotify::close_nowrite) == 1 &&
                count(events, inode, inotify::delete_self) == 1 && count(events, inode, inotify::ignored) == 1,
            "final unlinked CLOSE / DELETE_SELF / IGNORED lifecycle incomplete");
    auto position = [&](uint32_t mask) {
        return std::find_if(events.begin(), events.end(), [&](const Event &event) {
            return event.wd == inode && (event.mask & mask);
        });
    };
    require(position(inotify::close_nowrite) < position(inotify::delete_self) &&
                position(inotify::delete_self) < position(inotify::ignored), "unlinked lifecycle event order");
    require(!count(f.events(excluded), filtered, inotify::close_nowrite, "a"), "IN_EXCL_UNLINK ignored for CLOSE");
}
void overwritten_descriptions() {
    Fixture f;
    const int notify = f.instance();
    const int wd = f.watch(notify, "/data", data_events | inotify::exclude_unlinked);
    const int source = f.open("/data/a"), replaced = f.open("/data/b");
    f.events(notify);
    f.rename("/data/a", "/data/b");
    require(f.call(67, {uint64_t(replaced), 0x14000, 1, 0}) == 1, "read overwritten file");
    require(f.events(notify).empty(), "overwritten dentry was treated as the replacement inode");
    require(f.call(67, {uint64_t(source), 0x14000, 1, 0}) == 1, "read replacement file");
    require(count(f.events(notify), wd, inotify::access, "b") == 1, "source dentry did not follow rename replacement");
    f.close(replaced);
    require(f.events(notify).empty(), "overwritten CLOSE ignored IN_EXCL_UNLINK");
    f.close(source);
    require(count(f.events(notify), wd, inotify::close_nowrite, "b") == 1, "replacement CLOSE filename");
}
void directory_and_transfer() {
    Fixture f;
    const int notify = f.instance(), wd = f.watch(notify, "/data", data_events);
    const int directory = f.open("/data", 0x10000);
    auto events = f.events(notify);
    require(count(events, wd, inotify::open) == 1 && (events.front().mask & inotify::is_directory), "directory OPEN / ISDIR");
    require(f.call(61, {uint64_t(directory), 0x14000, 0x2000}) > 0, "getdents64");
    require(count(f.events(notify), wd, inotify::access) == 1, "directory ACCESS");
    require(f.call(61, {uint64_t(directory), 0x14000, 0x2000}) == 0, "getdents64 EOF");
    require(count(f.events(notify), wd, inotify::access) == 1, "directory EOF ACCESS");
    f.close(directory);
    require(count(f.events(notify), wd, inotify::close_nowrite) == 1, "directory CLOSE");
    const int input = f.open("/data/a"), output = f.open("/data/b", 2);
    f.events(notify);
    require(f.call(71, {uint64_t(output), uint64_t(input), 0, 1}) == 1, "sendfile");
    require(count(f.events(notify), wd, inotify::access, "a") == 1, "sendfile ACCESS");
    f.close(input);
    f.close(output);
}
void mapped_reference_and_one_shot() {
    Fixture f;
    const int notify = f.instance(), wd = f.watch(notify, "/data/a", data_events);
    const int fd = f.open("/data/a");
    f.events(notify);
    const int64_t mapping = f.call(222, {0, 8192, read_perm, 2, uint64_t(fd), 0});
    require(mapping > 0, "mmap");
    f.close(fd);
    require(f.events(notify).empty(), "CLOSE generated while mmap retained the data-open");
    require(f.call(215, {uint64_t(mapping), 4096}) == 0, "partial munmap");
    require(f.events(notify).empty(), "partial munmap prematurely generated CLOSE");
    require(f.call(215, {uint64_t(mapping) + 4096, 4096}) == 0, "final munmap");
    require(count(f.events(notify), wd, inotify::close_nowrite) == 1, "final munmap failed to release CLOSE");
    const int once = f.instance(), one = f.watch(once, "/data/a", inotify::open | inotify::one_shot);
    const int second = f.open("/data/a");
    auto events = f.events(once);
    require(events.size() == 2 && events[0].wd == one && events[0].mask == inotify::open &&
                events[1].wd == one && events[1].mask == inotify::ignored, "one-shot OPEN / IGNORED");
    f.close(second);
    require(f.events(once).empty(), "one-shot watch survived its first event");
}
} // namespace
int main() {
    try {
        description_lifecycle();
        hardlinks_and_rename();
        noop_rename_notifications();
        real_rename_notifications();
        exchange_notifications();
        snapshot_and_queue_overflow();
        cross_directory_moves();
        unlinked_descriptions();
        overwritten_descriptions();
        directory_and_transfer();
        mapped_reference_and_one_shot();
        std::puts("guest inotify data-open and dentry lifecycle passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
