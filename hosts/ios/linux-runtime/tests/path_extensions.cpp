#include "linux/events/inotify.h"
#include "linux/syscalls.h"
#include <algorithm>
#include <atomic>
#include <cerrno>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <filesystem>
#include <fstream>
#include <initializer_list>
#include <iterator>
#include <stdexcept>
#include <thread>
#include <sys/stat.h>
#include <unistd.h>
#include <vector>

// Standalone runtime test executable. The shared dispatcher must route AArch64
// 276 and 452 to dispatch_path before this source is linked and run.
using namespace operit::arm64;
namespace {
constexpr uint64_t cwd = uint64_t(-100);
constexpr uint32_t noreplace = 1, exchange = 2, whiteout = 4;
constexpr uint32_t nofollow = 256, empty_path = 4096;
// Linux AArch64 overrides the asm-generic O_DIRECTORY/O_NOFOLLOW values.
constexpr uint64_t guest_path = 010000000, guest_directory = 0040000, guest_nofollow = 0100000;
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
void require_result(int64_t actual, int64_t expected, const char *message) {
    if (actual != expected)
        throw std::runtime_error(std::string(message) + ": expected " + std::to_string(expected) +
                                 ", got " + std::to_string(actual));
}
struct Event {
    int32_t wd;
    uint32_t mask, cookie;
    std::string name;
};
struct Fixture {
    std::string temporary, root;
    Memory memory{8 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler scheduler;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/operit-path-extensions-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        temporary = created;
        root = temporary + "/root";
        std::filesystem::create_directories(root + "/data");
        std::filesystem::create_directory(root + "/other");
        put("/data/a", "first");
        put("/data/b", "second");
        require(::chmod((root + "/data/a").c_str(), 0600) == 0, "initial chmod");
        fs = std::make_unique<Filesystem>(root.c_str());
        memory.map(0x10000, 0x20000, read_perm | write_perm);
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, *fs, config, scheduler);
    }
    ~Fixture() {
        sys.reset();
        fs.reset();
        // Permission tests may remove directory search bits.
        ::chmod(root.c_str(), 0700);
        ::chmod((root + "/data").c_str(), 0700);
        std::filesystem::remove_all(temporary);
    }
    void put(const std::string &name, const std::string &contents) {
        std::ofstream file(root + name);
        file << contents;
        require(bool(file), "write fixture file");
    }
    std::string contents(const std::string &name) const {
        std::ifstream file(root + name);
        require(bool(file), "read fixture file");
        return std::string(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
    }
    struct stat metadata(const std::string &name) const {
        struct stat info{};
        require(::lstat((root + name).c_str(), &info) == 0, "lstat fixture path");
        return info;
    }
    uint64_t text(const std::string &value, uint64_t address = 0x10000) {
        memory.copy_in(address, value.c_str(), value.size() + 1);
        return address;
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> args = std::initializer_list<uint64_t>{}) {
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        std::copy(args.begin(), args.end(), cpu.x.begin());
        cpu.x[8] = number;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    int64_t rename(const std::string &from, const std::string &to, uint64_t flags = 0,
                   uint64_t fromfd = cwd, uint64_t tofd = cwd) {
        const auto source = text(from), target = text(to, 0x11000);
        return call(276, {fromfd, source, tofd, target, flags});
    }
    int64_t chmod(const std::string &path, uint64_t mode, uint64_t flags = 0, uint64_t fd = cwd) {
        return call(452, {fd, text(path), mode, flags});
    }
    int open(const std::string &path, uint64_t flags = 0) {
        const int fd = int(call(56, {cwd, text(path), flags, 0600}));
        require(fd >= 0, "open guest path");
        return fd;
    }
    int notify() {
        const int fd = int(call(26, {inotify::nonblock}));
        require(fd >= 0, "inotify_init1");
        return fd;
    }
    int watch(int fd, const std::string &path, uint32_t mask) {
        const int wd = int(call(27, {uint64_t(fd), text(path), mask}));
        require(wd >= 0, "inotify_add_watch");
        return wd;
    }
    std::vector<Event> events(int fd) {
        const auto size = call(63, {uint64_t(fd), 0x18000, 0x4000});
        if (size == -11)
            return {};
        require(size > 0, "read inotify events");
        std::vector<Event> result;
        for (uint64_t cursor = 0; cursor < uint64_t(size);) {
            require(uint64_t(size) - cursor >= 16, "inotify header size");
            const auto at = 0x18000 + cursor;
            const auto length = uint32_t(memory.load(at + 12, 4));
            require(length <= uint64_t(size) - cursor - 16, "inotify name size");
            const auto mask = uint32_t(memory.load(at + 4, 4));
            require(!(mask & inotify::overflow), "unexpected inotify overflow");
            result.push_back({int32_t(memory.load(at, 4)), mask,
                              uint32_t(memory.load(at + 8, 4)), length ? memory.string(at + 16) : ""});
            cursor += 16 + length;
        }
        return result;
    }
};
bool has(const std::vector<Event> &events, int wd, uint32_t mask, const std::string &name = "") {
    return std::any_of(events.begin(), events.end(), [&](const Event &event) {
        return event.wd == wd && (event.mask & mask) == mask && event.name == name;
    });
}
void require_move(const std::vector<Event> &events, int wd, const std::string &from,
                  const std::string &to) {
    for (const auto &first : events) {
        if (first.wd != wd || !(first.mask & inotify::moved_from) || first.name != from || !first.cookie)
            continue;
        for (const auto &second : events)
            if (second.wd == wd && (second.mask & inotify::moved_to) && second.name == to &&
                second.cookie == first.cookie)
                return;
    }
    std::string detail = "missing rename event pair with matching nonzero cookie: " + from + " -> " + to;
    for (const auto &event : events)
        detail += " [wd=" + std::to_string(event.wd) + " mask=" + std::to_string(event.mask) +
                  " cookie=" + std::to_string(event.cookie) + " name=" + event.name + "]";
    throw std::runtime_error(detail);
}
void rename_flags_and_errors() {
    Fixture f;
    const auto a = f.metadata("/data/a"), b = f.metadata("/data/b");
    for (const auto flags : {uint64_t(noreplace), uint64_t(exchange | noreplace),
                             uint64_t(exchange | whiteout), uint64_t(8)}) {
        require(f.rename("/data/a", "/data/b", flags) == (flags == noreplace ? -17 : -22),
                "renameat2 flag validation / occupied target");
        require(f.metadata("/data/a").st_ino == a.st_ino && f.metadata("/data/b").st_ino == b.st_ino,
                "failed rename changed namespace");
    }
    require(f.rename("/data/a", "/data/a", noreplace) == -17, "NOREPLACE same path EEXIST");
    require(f.rename("/data/missing", "/data/missing", noreplace) == -2,
            "NOREPLACE same missing path ENOENT");
    struct stat alternate{};
    if (::lstat((f.root + "/data/A").c_str(), &alternate) == 0 && alternate.st_ino == a.st_ino)
        require(f.rename("/data/a", "/data/A", noreplace) == -17,
                "NOREPLACE case-only alias EEXIST on case-insensitive Darwin volumes");
    require(::link((f.root + "/data/a").c_str(), (f.root + "/data/hard").c_str()) == 0, "hardlink");
    require(f.rename("/data/a", "/data/hard", noreplace) == -17, "NOREPLACE same inode EEXIST");
    require(f.rename("/data/a", "/data/white", whiteout) == -95 &&
                f.rename("/data/a", "/data/white", whiteout | noreplace) == -95,
            "unsupported WHITEOUT must not modify source");
    require(f.rename("/data/a", "/data/missing", exchange) == -2, "EXCHANGE requires both entries");
    require(f.rename("/data/missing", "/data/b", exchange) == -2, "EXCHANGE missing source");
    require(f.rename("", "/data/b") == -2, "empty source ENOENT");
    require(f.rename("/data/a", "") == -2, "empty destination ENOENT");
    require(f.rename("a", "b", 0, 99999, 99999) == -9, "relative invalid dirfd EBADF");
    const int ordinary = f.open("/data/a");
    require(f.rename("a", "b", 0, ordinary, ordinary) == -20, "non-directory dirfd ENOTDIR");
    require(f.call(276, {cwd, 0, cwd, f.text("/data/b"), 0}) == -14, "invalid rename path EFAULT");
    require(f.rename("/data/a", "/data/new", uint64_t{1} << 32, 99999, 99999) == 0,
            "absolute paths ignore dirfds; flags are unsigned int");
    require(f.contents("/data/new") == "first", "zero flags preserve ordinary rename semantics");
    require(f.rename("/data/new", "/data/b") == 0 && f.contents("/data/b") == "first",
            "zero flags replace occupied target");
}
void rename_dirfds_and_links() {
    Fixture f;
    const int directory = f.open("/data", guest_path | guest_directory);
    const int other = f.open("/other", guest_directory);
    require(f.rename("/data", "/renamed") == 0, "move directory held by dirfd");
    std::filesystem::create_directory(f.root + "/data");
    f.put("/data/a", "decoy");
    require(f.rename("a", "moved", noreplace, directory, other) == 0, "rename uses pinned dirfd");
    require(f.contents("/other/moved") == "first" && f.contents("/data/a") == "decoy",
            "rename reopened stale dirfd display path");
    require(::symlink("/data/a", (f.root + "/renamed/link").c_str()) == 0, "absolute guest link");
    require(f.rename("link", "link", noreplace, directory, other) == 0, "rename final symlink itself");
    require(S_ISLNK(f.metadata("/other/link").st_mode) && f.contents("/data/a") == "decoy",
            "rename followed final symlink");
    require(f.rename("/other/moved", "/other/link", exchange) == 0, "exchange symlink and file");
    require(S_ISLNK(f.metadata("/other/moved").st_mode) && f.contents("/other/link") == "first",
            "exchange followed final symlink");
    std::filesystem::create_directory(f.root + "/tree");
    f.put("/tree/child", "child");
    require(f.rename("/tree", "/other/link", exchange) == 0, "exchange nonempty directory and file");
    require(f.contents("/tree") == "first" && f.contents("/other/link/child") == "child",
            "mixed-type exchange lost a subtree");
}
void rename_atomicity() {
    Fixture f;
    // Compete with an independent native creator. Exactly one operation may
    // acquire the previously absent name; the loser must retain its own inode.
    for (unsigned index = 0; index < 96; ++index) {
        const auto source = "/data/source-" + std::to_string(index);
        const auto target = "/data/target-" + std::to_string(index);
        f.put(source, "guest");
        std::atomic<bool> start{false};
        int created = -1, error = 0;
        std::thread contender([&] {
            while (!start.load(std::memory_order_acquire))
                std::this_thread::yield();
            created = ::open((f.root + target).c_str(), O_CREAT | O_EXCL | O_WRONLY | O_CLOEXEC, 0600);
            error = errno;
            if (created >= 0) {
                ::write(created, "native", 6);
                ::close(created);
            }
        });
        start.store(true, std::memory_order_release);
        const auto result = f.rename(source, target, noreplace);
        contender.join();
        if (created >= 0) {
            require(result == -17 && f.contents(target) == "native" && f.contents(source) == "guest",
                    "NOREPLACE overwrote a racing creator");
        } else {
            require(error == EEXIST && result == 0 && f.contents(target) == "guest" &&
                        !std::filesystem::exists(f.root + source),
                    "NOREPLACE race did not have exactly one winner");
        }
    }
    const auto a = f.metadata("/data/a"), b = f.metadata("/data/b");
    std::atomic<bool> done{false}, observed{false}, gap{false};
    std::thread observer([&] {
        while (!done.load(std::memory_order_acquire)) {
            for (const auto &name : {"/data/a", "/data/b"}) {
                struct stat info{};
                if (::lstat((f.root + name).c_str(), &info) != 0 ||
                    (info.st_ino != a.st_ino && info.st_ino != b.st_ino))
                    gap.store(true, std::memory_order_release);
            }
            observed.store(true, std::memory_order_release);
        }
    });
    while (!observed.load(std::memory_order_acquire))
        std::this_thread::yield();
    bool success = true;
    for (unsigned index = 0; index < 128; ++index)
        success = (f.rename("/data/a", "/data/b", exchange) == 0) && success;
    done.store(true, std::memory_order_release);
    observer.join();
    require(success && !gap.load(), "EXCHANGE exposed a missing or temporary entry");
    require(f.metadata("/data/a").st_ino == a.st_ino && f.metadata("/data/b").st_ino == b.st_ino,
            "even number of exchanges failed to preserve identities");
}
void chmod_flags_modes_and_fds() {
    Fixture f;
    require(f.chmod("/data/a", 0641) == 0 && (f.metadata("/data/a").st_mode & 07777) == 0641,
            "fchmodat2 ordinary path");
    require(f.chmod("/data/a", (uint64_t{1} << 40) | S_IFDIR | 01640, uint64_t{1} << 32) == 0 &&
                (f.metadata("/data/a").st_mode & 07777) == 01640 && S_ISREG(f.metadata("/data/a").st_mode),
            "chmod mode masks file type/high bits; flags truncate to u32");
    require(f.chmod("/data/a", 0600, 512) == -22, "unknown chmod flag EINVAL");
    require(f.call(452, {99999, 0, 0600, 512}) == -22, "validate flags before pathname/fd");
    require(f.call(452, {cwd, 0, 0600, 0}) == -14, "NULL pathname EFAULT");
    require(f.chmod("", 0600) == -2, "empty path requires AT_EMPTY_PATH");
    require(f.chmod("", 0600, empty_path, 99999) == -9, "empty path invalid fd EBADF");
    require(f.chmod("/missing", 0600) == -2, "chmod missing path ENOENT");
    const int fd = f.open("/data/a");
    require(f.chmod("a", 0600, 0, fd) == -20, "nonempty chmod on ordinary dirfd ENOTDIR");
    require(f.rename("/data/a", "/data/old") == 0, "rename open chmod target");
    f.put("/data/a", "decoy");
    require(f.chmod("", 0620, empty_path | nofollow, fd) == 0 &&
                (f.metadata("/data/old").st_mode & 07777) == 0620,
            "empty path uses original open inode, including read-only FD");
    require(f.call(35, {cwd, f.text("/data/old"), 0}) == 0 && f.chmod("", 0610, empty_path, fd) == 0,
            "empty path works after unlink");
    const int pathfd = f.open("/data/b", guest_path);
    require(f.chmod("", 0604, empty_path, pathfd) == 0 &&
                (f.metadata("/data/b").st_mode & 07777) == 0604,
            "AT_EMPTY_PATH accepts O_PATH descriptor");
    const int directory = f.open("/data", guest_directory);
    require(f.rename("/data", "/moved") == 0, "rename chmod dirfd");
    std::filesystem::create_directory(f.root + "/data");
    f.put("/data/b", "decoy");
    require(f.chmod("b", 0640, 0, directory) == 0 &&
                (f.metadata("/moved/b").st_mode & 07777) == 0640,
            "relative chmod uses pinned dirfd");
    require(f.call(49, {f.text("/moved")}) == 0, "chdir");
    require(f.rename("/moved", "/cwd-renamed") == 0 && f.chmod("", 0750, empty_path) == 0 &&
                (f.metadata("/cwd-renamed").st_mode & 07777) == 0750,
            "AT_EMPTY_PATH with AT_FDCWD uses pinned cwd");
    require(f.chmod("/data/b", 0600, 0, 99999) == 0, "absolute chmod ignores dirfd");
    require(f.chmod("/data/b", 0600, empty_path) == 0, "AT_EMPTY_PATH also accepts nonempty path");
}
void chmod_links_isolation_and_permissions() {
    Fixture f;
    require(::symlink("/data/a", (f.root + "/absolute").c_str()) == 0, "absolute guest symlink");
    require(f.chmod("/absolute", 0640) == 0 && (f.metadata("/data/a").st_mode & 07777) == 0640,
            "chmod absolute symlink resolves inside guest root");
    require(f.chmod("/absolute", 0600, nofollow) == -95 &&
                (f.metadata("/data/a").st_mode & 07777) == 0640,
            "NOFOLLOW cannot chmod symlink or its referent");
    const int linkfd = f.open("/absolute", guest_path | guest_nofollow);
    require_result(f.chmod("", 0600, empty_path, linkfd), -95,
                   "empty O_PATH symlink FD never follows its contents");
    require_result(f.chmod("", 0600, empty_path | nofollow, linkfd), -95,
                   "empty O_PATH symlink FD with NOFOLLOW never follows its contents");
    require(::symlink("/missing", (f.root + "/dangling").c_str()) == 0, "dangling link");
    require(f.chmod("/dangling", 0600) == -2 && f.chmod("/dangling", 0600, nofollow) == -95,
            "dangling link follow/no-follow errors");
    // The same absolute spelling exists outside the guest root and inside it.
    // A host symlink-following fallback would change the external sentinel.
    const std::string sentinel = f.temporary + "/sentinel";
    std::ofstream(sentinel) << "outside";
    require(::chmod(sentinel.c_str(), 0600) == 0, "sentinel mode");
    std::filesystem::create_directories(f.root + f.temporary);
    f.put(sentinel, "inside");
    require(::chmod((f.root + sentinel).c_str(), 0600) == 0, "inside sentinel mode");
    require(::symlink(sentinel.c_str(), (f.root + "/escape").c_str()) == 0, "escape link");
    require(f.chmod("/escape", 0640) == 0 && (f.metadata(sentinel).st_mode & 07777) == 0640,
            "guest absolute target did not resolve inside root");
    struct stat outside{};
    require(::stat(sentinel.c_str(), &outside) == 0 && (outside.st_mode & 07777) == 0600,
            "chmod escaped guest root");
    require(f.chmod("/data/a", 0000) == 0 && f.chmod("/data/a", 0600) == 0,
            "owner chmod must not require file read permission");
    // Guest credentials are the host credentials. Exercise real EACCES without
    // requiring chown/root; root has search override and skips this assertion.
    if (::geteuid() != 0) {
        require(::chmod((f.root + "/data").c_str(), 0000) == 0, "deny directory search");
        const auto result = f.chmod("/data/a", 0644);
        require(::chmod((f.root + "/data").c_str(), 0700) == 0, "restore directory search");
        require(result == -13, "chmod must honor parent search permission");
    }
}
void chmod_path_descriptor_regressions() {
    Fixture f;
    require_result(f.call(56, {cwd, f.text("/data/a"), guest_directory, 0}), -20,
                   "AArch64 O_DIRECTORY rejects regular files");
    require_result(f.call(56, {cwd, f.text("/data/a"), guest_path | guest_directory, 0}), -20,
                   "AArch64 O_PATH|O_DIRECTORY rejects regular files");
    require(::symlink("data/a", (f.root + "/relative").c_str()) == 0, "relative guest symlink");
    require(::symlink("/missing", (f.root + "/dangling").c_str()) == 0, "dangling guest symlink");
    const auto original_mode = f.metadata("/data/a").st_mode & 07777;
    const auto followed_link_mode = f.metadata("/relative").st_mode & 07777;
    require_result(f.call(56, {cwd, f.text("/relative"), guest_nofollow, 0}), -40,
                   "AArch64 O_NOFOLLOW without O_PATH rejects symlink");
    const int followed = f.open("/relative", guest_path);
    require_result(f.chmod("", 0640, empty_path | nofollow, followed), 0,
                   "AT_SYMLINK_NOFOLLOW does not undo following during open");
    require((f.metadata("/data/a").st_mode & 07777) == 0640, "followed O_PATH chmod referent mode");
    require((f.metadata("/relative").st_mode & 07777) == followed_link_mode,
            "followed O_PATH chmod changed symlink mode");
    require_result(f.chmod("/data/a", original_mode), 0, "restore referent mode");
    for (const std::string name : {"/relative", "/dangling"}) {
        const auto link_mode = f.metadata(name).st_mode & 07777;
        const int link = f.open(name, guest_path | guest_nofollow);
        const auto target = name == "/relative" ? "data/a" : "/missing";
        const auto length = f.call(78, {uint64_t(link), f.text(""), 0x17000, 128});
        require_result(length, std::char_traits<char>::length(target),
                       "O_PATH|O_NOFOLLOW FD pins symlink contents");
        std::string contents(size_t(length), '\0');
        f.memory.copy_out(0x17000, contents.data(), contents.size());
        require(contents == target, "empty readlinkat returns pinned symlink contents");
        const auto check_link = [&](int fd) {
            for (const uint32_t flags : {empty_path, empty_path | nofollow})
                require_result(f.chmod("", 0622, flags, fd), -95,
                               "empty O_PATH symlink chmod returns EOPNOTSUPP");
            require((f.metadata("/data/a").st_mode & 07777) == original_mode,
                    "failed symlink descriptor chmod changed referent mode");
        };
        check_link(link);
        require((f.metadata(name).st_mode & 07777) == link_mode,
                "failed symlink descriptor chmod changed link mode");
        const int duplicate = int(f.call(23, {uint64_t(link)}));
        require(duplicate >= 0, "duplicate O_PATH symlink FD");
        require_result(f.rename(name, name + "-moved"), 0, "rename pinned symlink");
        f.put(name, "replacement");
        require_result(f.chmod(name, 0600), 0, "set replacement mode");
        check_link(duplicate);
        require_result(f.call(35, {cwd, f.text(name + "-moved"), 0}), 0, "unlink pinned symlink");
        check_link(link);
        require((f.metadata(name).st_mode & 07777) == 0600,
                "symlink FD chmod reopened replacement pathname");
    }
}
void virtual_and_mount_boundaries() {
    Fixture f;
    require(f.chmod("/dev/null", 0600) == -30, "virtual pathname chmod EROFS");
    const int nullfd = f.open("/dev/null");
    require(f.chmod("", 0600, empty_path, nullfd) == -30, "virtual descriptor chmod EROFS");
    require(f.rename("/dev/null", "/dev/replaced") == -30, "virtual rename EROFS");
    require(f.rename("/data/a", "/dev/new", noreplace) == -30, "virtual target rename EROFS");
    require(f.rename("/dev", "/data/device", exchange) == -16, "virtual mount root EBUSY");
    f.fs->bind(f.root + "/data", "/bound", false);
    f.fs->bind(f.root + "/data", "/readonly", true);
    require(f.rename("/data/a", "/bound/new", noreplace) == -18,
            "bind mount boundary EXDEV even on same native filesystem");
    require(f.rename("/data/a", "/bound/b", exchange) == -18, "EXCHANGE crosses bind mount");
    require(f.rename("/bound", "/other/mount") == -16, "bind mount root EBUSY");
    require(f.rename("/readonly/a", "/readonly/c", noreplace) == -30, "read-only rename EROFS");
    require(f.chmod("/readonly/a", 0600) == -30, "read-only path chmod EROFS");
    const int readonly = f.open("/readonly/a", guest_path);
    require(f.chmod("", 0600, empty_path, readonly) == -30, "read-only FD chmod EROFS");
    require((f.metadata("/data/a").st_mode & 07777) == 0600, "boundary errors mutated underlying inode");
}
void notifications() {
    Fixture f;
    const int notify = f.notify();
    const uint32_t mask = inotify::moved_from | inotify::moved_to | inotify::remove | inotify::attrib |
                          inotify::exclude_unlinked;
    const int directory = f.watch(notify, "/data", mask);
    require(f.rename("/data/a", "/data/b", noreplace) == -17 && f.events(notify).empty(),
            "failed rename emitted mutation events");
    require(f.rename("/data/a", "/data/c", noreplace) == 0, "notified NOREPLACE");
    require_move(f.events(notify), directory, "a", "c");
    require(f.rename("/data/c", "/data/b", exchange) == 0, "notified EXCHANGE");
    const auto swapped = f.events(notify);
    require_move(swapped, directory, "c", "b");
    require_move(swapped, directory, "b", "c");
    require(!has(swapped, directory, inotify::remove, "b") &&
                !has(swapped, directory, inotify::remove, "c"),
            "EXCHANGE falsely deleted a surviving dentry");
    require(f.chmod("/data/b", 0640) == 0 && has(f.events(notify), directory, inotify::attrib, "b"),
            "successful fchmodat2 must emit IN_ATTRIB");
    require(f.chmod("/data/b", 0600, 512) == -22 && f.events(notify).empty(),
            "failed fchmodat2 emitted IN_ATTRIB");
    // Ordinary rename must preserve the existing guest data-open bridge.
    const int access = f.watch(notify, "/data", inotify::access | inotify::close_nowrite | inotify::mask_add);
    const int fd = f.open("/data/b");
    f.events(notify);
    require(f.rename("/data/b", "/data/final") == 0, "rename with open description");
    f.events(notify);
    require(f.call(63, {uint64_t(fd), 0x17000, 1}) == 1, "read renamed open file");
    require(has(f.events(notify), access, inotify::access, "final"), "rename data-open ACCESS spelling");
    require(f.call(57, {uint64_t(fd)}) == 0 &&
                has(f.events(notify), access, inotify::close_nowrite, "final"),
            "rename data-open CLOSE spelling");
    const int left = f.open("/data/c"), right = f.open("/data/final");
    f.events(notify);
    require(f.rename("/data/c", "/data/final", exchange) == 0, "exchange two open descriptions");
    const auto exchanged = f.events(notify);
    require_move(exchanged, directory, "c", "final");
    require_move(exchanged, directory, "final", "c");
    require(f.call(63, {uint64_t(left), 0x17000, 1}) == 1, "read exchanged left description");
    auto accessed = f.events(notify);
    require(has(accessed, access, inotify::access, "final") &&
                !has(accessed, access, inotify::access, "c"),
            "left ACCESS must follow exchanged dentry");
    require(f.call(63, {uint64_t(right), 0x17000, 1}) == 1, "read exchanged right description");
    accessed = f.events(notify);
    require(has(accessed, access, inotify::access, "c") &&
                !has(accessed, access, inotify::access, "final"),
            "right ACCESS must follow exchanged dentry");
    require(f.call(57, {uint64_t(left)}) == 0, "close exchanged left description");
    auto closed = f.events(notify);
    require(has(closed, access, inotify::close_nowrite, "final") &&
                !has(closed, access, inotify::close_nowrite, "c"),
            "left CLOSE must retain exchanged dentry");
    require(f.call(57, {uint64_t(right)}) == 0, "close exchanged right description");
    closed = f.events(notify);
    require(has(closed, access, inotify::close_nowrite, "c") &&
                !has(closed, access, inotify::close_nowrite, "final"),
            "right CLOSE must retain exchanged dentry");
}
void exchange_open_descriptions_across_directories() {
    Fixture f;
    f.put("/other/b", "other");
    const int notify = f.notify();
    const uint32_t mask = inotify::access | inotify::close_nowrite | inotify::exclude_unlinked;
    const int source = f.watch(notify, "/data", mask), target = f.watch(notify, "/other", mask);
    const int left = f.open("/data/a"), right = f.open("/other/b");
    require(f.rename("/data/a", "/other/b", exchange) == 0, "exchange across directory parents");
    f.events(notify);
    require(f.call(63, {uint64_t(left), 0x17000, 1}) == 1, "read moved left inode");
    auto events = f.events(notify);
    require(has(events, target, inotify::access, "b") && !has(events, source, inotify::access, "a"),
            "exchange must update left parent and preserve linked state");
    require(f.call(63, {uint64_t(right), 0x17000, 1}) == 1, "read moved right inode");
    events = f.events(notify);
    require(has(events, source, inotify::access, "a") && !has(events, target, inotify::access, "b"),
            "exchange must update right parent and preserve linked state");
    require(f.call(57, {uint64_t(left)}) == 0, "close left inode in new parent");
    require(has(f.events(notify), target, inotify::close_nowrite, "b"), "cross-parent left CLOSE");
    require(f.call(57, {uint64_t(right)}) == 0, "close right inode in new parent");
    require(has(f.events(notify), source, inotify::close_nowrite, "a"), "cross-parent right CLOSE");

    // Exchanging two hardlinks to one inode is a no-op. Descriptions retain
    // their original dentry rather than having their names swapped.
    require(::link((f.root + "/data/a").c_str(), (f.root + "/other/twin").c_str()) == 0, "hardlink twin");
    const int first = f.open("/data/a"), twin = f.open("/other/twin");
    f.events(notify);
    require(f.rename("/data/a", "/other/twin", exchange) == 0, "exchange same inode");
    f.events(notify);
    require(f.call(63, {uint64_t(first), 0x17000, 1}) == 1, "read no-op original");
    events = f.events(notify);
    require(has(events, source, inotify::access, "a") && !has(events, target, inotify::access, "twin"),
            "same-inode exchange changed original open dentry");
    require(f.call(63, {uint64_t(twin), 0x17000, 1}) == 1, "read no-op twin");
    events = f.events(notify);
    require(has(events, target, inotify::access, "twin") && !has(events, source, inotify::access, "a"),
            "same-inode exchange changed twin open dentry");
    require(f.call(57, {uint64_t(first)}) == 0, "close no-op original");
    require(has(f.events(notify), source, inotify::close_nowrite, "a"), "same-inode original CLOSE");
    require(f.call(57, {uint64_t(twin)}) == 0, "close no-op twin");
    require(has(f.events(notify), target, inotify::close_nowrite, "twin"), "same-inode twin CLOSE");
}
} // namespace
int main() {
    try {
        rename_flags_and_errors();
        rename_dirfds_and_links();
        rename_atomicity();
        chmod_flags_modes_and_fds();
        chmod_links_isolation_and_permissions();
        chmod_path_descriptor_regressions();
        virtual_and_mount_boundaries();
        notifications();
        exchange_open_descriptions_across_directories();
        std::puts("path extension tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "path extension tests failed: %s\n", error.what());
        return 1;
    }
}
