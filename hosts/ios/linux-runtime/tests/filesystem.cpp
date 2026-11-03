#include "fs/filesystem.h"
#include "linux/abi.h"
#include "linux/syscalls.h"
#include <algorithm>
#include <array>
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <sys/ioctl.h>

using namespace operit::arm64;
namespace {
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Temporary {
    std::string path;
    Temporary() {
        char pattern[] = "/tmp/operit-vfs-XXXXXX";
        auto *result = ::mkdtemp(pattern);
        if (!result)
            throw std::runtime_error("mkdtemp");
        path = result;
    }
    ~Temporary() {
        std::filesystem::remove_all(path);
    }
};
struct Fixture {
    Temporary temp;
    std::string root = temp.path + "/root", source = temp.path + "/shared", nested = temp.path + "/nested";
    Memory memory{1 << 20};
    Filesystem fs;
    Scheduler scheduler;
    ort_arm64_config config{};
    std::unique_ptr<Syscalls> sys;
    static const char *prepare(Fixture &f) {
        std::filesystem::create_directory(f.root);
        std::filesystem::create_directory(f.source);
        std::filesystem::create_directory(f.nested);
        std::filesystem::create_directory(f.source + "/inside");
        std::ofstream(f.root + "/secret") << "guest-root";
        std::ofstream(f.source + "/message") << "shared-data";
        std::ofstream(f.nested + "/message") << "nested-data";
        std::filesystem::create_symlink("/secret", f.source + "/absolute");
        std::filesystem::create_symlink("../secret", f.source + "/parent");
        return f.root.c_str();
    }
    Fixture() : fs(prepare(*this)) {
        memory.map(0x10000, 0x10000, read_perm | write_perm);
        fs.bind(source, "/workspace", false);
        fs.bind(nested, "/workspace/inside", true);
        fs.bind(source, "/readonly", true);
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, fs, config, scheduler);
    }
    uint64_t text(const std::string &value, uint64_t address = 0x10000) {
        memory.copy_in(address, value.c_str(), value.size() + 1);
        return address;
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> args) {
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        std::copy(args.begin(), args.end(), cpu.x.begin());
        cpu.x[8] = number;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    int64_t open(const std::string &path, int flags = 0) {
        return call(56, {uint64_t(-100), text(path), uint64_t(flags), 0600});
    }
    std::string read(int fd) {
        int64_t length = call(63, {uint64_t(fd), 0x11000, 100});
        require(length >= 0, "read failed");
        std::string value(size_t(length), '\0');
        memory.copy_out(0x11000, value.data(), value.size());
        return value;
    }
};
void mount_resolution() {
    Fixture f;
    require(f.call(174, {}) == ::getuid() && f.call(175, {}) == ::geteuid() &&
                f.call(176, {}) == ::getgid() && f.call(177, {}) == ::getegid(),
            "identity must match actual host authority");
    const int group_count = ::getgroups(0, nullptr);
    require(group_count >= 0 && f.call(158, {0, 0}) == group_count, "supplementary group count");
    std::vector<gid_t> groups(group_count);
    require(::getgroups(group_count, groups.data()) == group_count, "host supplementary groups");
    require(f.call(158, {uint64_t(group_count), 0x13000}) == group_count, "supplementary group copy");
    for (int index = 0; index < group_count; ++index)
        require(f.memory.load(0x13000 + uint64_t(index) * 4, 4) == groups[index],
                "supplementary group identity");
    require(f.call(158, {UINT64_MAX, 0}) == -22, "negative supplementary group capacity");
    if (group_count)
        require(f.call(158, {uint64_t(group_count), 0x50000}) == -14, "supplementary group EFAULT");
    require(f.call(159, {0, 0}) == -1, "setgroups bypassed guest capabilities");
    int metadata_fd = int(f.open("/workspace/message"));
    struct stat actual{};
    require(::stat((f.source + "/message").c_str(), &actual) == 0, "host ownership reference");
    require(f.call(80, {uint64_t(metadata_fd), 0x13000}) == 0 && f.memory.load(0x13018, 4) == actual.st_uid &&
                f.memory.load(0x1301c, 4) == actual.st_gid,
            "stat ownership differs from real backing file");
    require(f.call(144, {uint64_t(::getgid())}) == 0 && f.call(146, {uint64_t(::getuid())}) == 0,
            "retaining actual credentials");
    require(f.call(146, {uint64_t(::getuid()) + 1}) == -1, "credential changes must not fake privileges");
    require(f.read(int(f.open("/workspace/message"))) == "shared-data", "bind read");
    require(f.read(int(f.open("/workspace/inside/message"))) == "nested-data", "nested mount read");
    require(f.read(int(f.open("/workspace/inside/../message"))) == "shared-data", "nested mount parent");
    require(f.read(int(f.open("/workspace/../secret"))) == "guest-root", "mount parent");
    require(f.read(int(f.open("/workspace/absolute"))) == "guest-root", "absolute symlink in mount");
    require(f.read(int(f.open("/workspace/parent"))) == "guest-root", "relative symlink in mount");
    std::filesystem::rename(f.source, f.source + "-moved");
    require(f.read(int(f.open("/workspace/message"))) == "shared-data",
            "host rename invalidated mount anchor");
}
void mount_writes_and_readonly() {
    Fixture f;
    int fd = int(f.open("/workspace/new", 64 | 2));
    require(fd >= 0, "bind create");
    require((f.call(25, {uint64_t(fd), 3}) & (64 | 128 | 256 | 512 | 524288)) == 0,
            "F_GETFL exposed creation/descriptor flags");
    require(f.call(64, {uint64_t(fd), f.text("persistent"), 10}) == 10, "bind write");
    require(f.call(57, {uint64_t(fd)}) == 0, "bind close");
    std::ifstream input(f.source + "/new");
    std::string output;
    input >> output;
    require(output == "persistent", "host file did not receive guest write");
    require(f.open("/readonly/message", 1) == -30, "readonly open");
    require(f.open("/readonly/new", 64 | 2) == -30, "readonly create");
    require(f.call(34, {uint64_t(-100), f.text("/readonly/sub"), 0700}) == -30, "readonly mkdir");
    require(f.call(35, {uint64_t(-100), f.text("/readonly/message"), 0}) == -30, "readonly unlink");
    f.text("/workspace/message");
    f.text("/secret", 0x10100);
    require(f.call(38, {uint64_t(-100), 0x10000, uint64_t(-100), 0x10100}) == -18, "cross mount rename");
    require(f.call(35, {uint64_t(-100), f.text("/workspace"), 512}) == -16, "unlink mountpoint");
    require(f.open("/workspace/message/") == -20, "trailing slash must require directory");
    require(f.call(34, {999, f.text("/workspace/absolute-dirfd"), 0700}) == 0,
            "absolute path incorrectly used invalid dirfd");
    require(f.call(34, {999, f.text("relative-dirfd"), 0700}) == -9, "relative path ignored invalid dirfd");
}
void descriptor_flags_and_ioctl() {
    Fixture f;
    require(f.call(59, {0x11000, 524288}) == 0, "pipe");
    uint64_t reader = f.memory.load(0x11000, 4), writer = f.memory.load(0x11004, 4);
    auto alias = f.call(23, {reader});
    require(alias >= 0, "dup");
    require(f.call(25, {reader, 1}) == 1 && f.call(25, {uint64_t(alias), 1}) == 0, "dup must clear cloexec");
    f.memory.store(0x12000, 1, 4);
    require(f.call(29, {reader, 0x5421, 0x12000}) == 0, "FIONBIO");
    require(f.call(25, {uint64_t(alias), 3}) & 2048, "status flags not shared");
    require(f.call(63, {reader, 0x12100, 1}) == -11, "nonblock pipe read");
    require(f.call(64, {writer, f.text("abc"), 3}) == 3, "pipe write");
    require(f.call(29, {uint64_t(alias), 0x541b, 0x12000}) == 0 && f.memory.load(0x12000, 4) == 3,
            "FIONREAD");
    require(f.call(29, {reader, 0x5421, 0xdead}) == -14, "ioctl pointer checking");
    require(f.call(29, {999, 0x5421, 0x12000}) == -9, "ioctl invalid fd");
    require(f.call(25, {uint64_t(alias), 4, 0}) == 0 && !(f.call(25, {reader, 3}) & 2048), "F_SETFL sharing");
    auto replacement = f.call(24, {reader, 50, 524288});
    require(replacement == 50 && f.call(25, {50, 1}) == 1, "dup3 target/cloexec");
    require(f.call(24, {reader, reader, 0}) == -22, "dup3 same fd");
    require(f.call(57, {reader}) == 0 && f.read(int(alias)) == "abc", "dup lifetime/offset");
    require(f.call(19, {3, 2048}) >= 0, "eventfd create");
    int event = int(f.scheduler.current().cpu.x[0]);
    int copy = int(f.call(23, {uint64_t(event)}));
    require(copy >= 0 && f.call(63, {uint64_t(copy), 0x12000, 8}) == 8 && f.memory.load(0x12000, 8) == 3,
            "eventfd dup lost virtual state");
    require(f.call(63, {uint64_t(event), 0x12000, 8}) == -11, "eventfd duplicates did not share counter");
    int poll = int(f.call(20, {524288}));
    int poll_copy = int(f.call(25, {uint64_t(poll), 1030, 60}));
    require(poll_copy == 60 && f.call(25, {60, 1}) == 1, "epoll F_DUPFD_CLOEXEC");
    require(f.call(22, {60, 0x12000, 1, 0, 0}) == 0, "epoll alias not usable");
}
void pipe_capacity_and_short_writes() {
    Fixture f;
    require(f.call(59, {0x11000, 2048}) == 0, "capacity pipe2");
    const uint64_t reader = f.memory.load(0x11000, 4), writer = f.memory.load(0x11004, 4);
    const int64_t capacity = f.call(25, {reader, 1032});
    require(capacity >= 4096, "F_GETPIPE_SZ must report a usable Linux pipe capacity");
    require(f.call(25, {writer, 1032}) == capacity, "pipe ends disagree on capacity");
    const int64_t alias = f.call(23, {writer});
    require(alias >= 0 && f.call(25, {uint64_t(alias), 1032}) == capacity,
            "duplicated pipe lost its capacity");
    require(f.call(25, {999, 1032}) == -9, "pipe capacity accepted an invalid descriptor");
    const int64_t regular = f.open("/workspace/message");
    require(regular >= 0 && f.call(25, {uint64_t(regular), 1032}) == -9,
            "pipe capacity accepted a regular file");

    const uint64_t data = 0x20000, length = uint64_t(capacity) + 4096;
    f.memory.map(data, (length + page_size - 1) / page_size * page_size, read_perm | write_perm);
    std::vector<uint8_t> payload(length, 'x');
    f.memory.copy_in(data, payload.data(), payload.size());
    require(f.call(64, {writer, data, uint64_t(capacity)}) == capacity,
            "empty pipe cannot hold its advertised capacity");
    require(f.call(29, {reader, 0x541b, 0x12000}) == 0 &&
                f.memory.load(0x12000, 4) == uint64_t(capacity),
            "FIONREAD disagrees with the full pipe capacity");
    require(f.call(64, {writer, data, 1}) == -11, "full nonblocking pipe must return EAGAIN");
    require(f.call(63, {reader, data, uint64_t(capacity)}) == capacity, "drain full pipe");
    require(f.call(64, {uint64_t(alias), data, length}) == capacity,
            "oversized pipe write must return a short count");
    require(f.call(63, {reader, data, length}) == capacity, "drain short write");
    std::vector<uint8_t> received(capacity);
    f.memory.copy_out(data, received.data(), received.size());
    require(std::all_of(received.begin(), received.end(), [](uint8_t byte) { return byte == 'x'; }),
            "pipe capacity boundary corrupted the payload");
    require(f.call(25, {reader, 1032}) == capacity, "draining changed the pipe capacity");
}
void filesystem_operations_and_directory_views() {
    Fixture f;
    f.text("/workspace/message");
    f.text("/workspace/hard", 0x10100);
    require(f.call(37, {uint64_t(-100), 0x10000, uint64_t(-100), 0x10100, 0}) == 0, "linkat");
    require(std::filesystem::hard_link_count(f.source + "/message") == 2, "linkat host identity");
    f.text("/workspace/message");
    f.text("/workspace/soft", 0x10100);
    require(f.call(36, {0x10000, uint64_t(-100), 0x10100}) == 0, "symlinkat");
    require(f.read(int(f.open("/workspace/soft"))) == "shared-data", "guest absolute symlink");
    require(f.call(53, {uint64_t(-100), f.text("/workspace/message"), 0640}) == 0, "fchmodat");
    struct stat info{};
    require(::stat((f.source + "/message").c_str(), &info) == 0 && (info.st_mode & 0777) == 0640,
            "chmod persistence");
    require(f.call(45, {f.text("/workspace/message"), 3}) == 0, "truncate");
    require(f.read(int(f.open("/workspace/hard"))) == "sha", "hard link data identity");
    f.memory.store(0x12000, 1234567890, 8);
    f.memory.store(0x12008, 123000000, 8);
    f.memory.store(0x12010, 1234567891, 8);
    f.memory.store(0x12018, 456000000, 8);
    require(f.call(88, {uint64_t(-100), f.text("/workspace/message"), 0x12000, 0}) == 0, "utimensat");
    require(::stat((f.source + "/message").c_str(), &info) == 0 && info.st_mtime == 1234567891,
            "timestamp persistence");
    require(f.call(53, {uint64_t(-100), f.text("/readonly/message"), 0600}) == -30, "readonly chmod");
    int directory = int(f.open("/", 16384));
    int64_t size = f.call(61, {uint64_t(directory), 0x13000, 4096});
    require(size > 0, "root getdents");
    bool workspace = false, readonly = false;
    uint64_t last = 0;
    for (int64_t offset = 0; offset < size;) {
        auto name = f.memory.string(0x13000 + offset + 19);
        workspace |= name == "workspace";
        readonly |= name == "readonly";
        last = f.memory.load(0x13000 + offset + 8, 8);
        offset += f.memory.load(0x13000 + offset + 16, 2);
    }
    require(workspace && readonly, "mountpoints absent from directory listing");
    require(f.call(61, {uint64_t(directory), 0x13000, 4096}) == 0, "getdents EOF");
    require(f.call(62, {uint64_t(directory), last, 0}) == int64_t(last) &&
                f.call(61, {uint64_t(directory), 0x13000, 4096}) == 0,
            "mount cookie seek");
    require(f.call(62, {uint64_t(directory), 0, 0}) == 0 &&
                f.call(61, {uint64_t(directory), 0x13000, 4096}) == size,
            "directory rewind");
}
void blocking_io_and_broken_pipe() {
    Fixture f;
    require(f.call(59, {0x11000, 0}) == 0, "blocking pipe");
    uint64_t reader = f.memory.load(0x11000, 4), writer = f.memory.load(0x11004, 4);
    f.scheduler.spawn(f.scheduler.current().cpu, 0x18000, 0, 0);
    require(f.call(63, {reader, 0x12000, 4}) == 0 && f.scheduler.current().state == Task::State::Waiting,
            "read must suspend guest task");
    require(f.scheduler.select() && f.scheduler.current().tid == 2, "blocked read prevented other task");
    require(f.call(64, {writer, f.text("wake"), 4}) == 4, "writer task");
    require(f.scheduler.select() && f.scheduler.current().tid == 1 && f.scheduler.current().cpu.x[0] == 4 &&
                f.memory.load(0x12000, 4) == 0x656b6177,
            "blocked read completion");
    require(f.call(57, {reader}) == 0, "close pipe reader");
    require(f.call(64, {writer, f.text("x"), 1}) == -32, "guest EPIPE must not terminate host");
    int counter = int(f.call(19, {0, 0}));
    require(f.call(63, {uint64_t(counter), 0x12000, 8}) == 0 &&
                f.scheduler.current().state == Task::State::Waiting,
            "eventfd blocking read");
    f.scheduler.select();
    f.memory.store(0x13000, 5, 8);
    require(f.call(64, {uint64_t(counter), 0x13000, 8}) == 8, "eventfd writer");
    f.scheduler.select();
    require(f.scheduler.current().tid == 1 && f.scheduler.current().cpu.x[0] == 8 &&
                f.memory.load(0x12000, 8) == 5,
            "eventfd reader wake");
}

void vector_file_io_and_directory_creation() {
    Fixture f;
    int fd = int(f.open("/workspace/vector", 64 | 2));
    require(fd >= 0 && f.call(64, {uint64_t(fd), f.text("abcdefghij"), 10}) == 10, "vector fixture write");
    require(f.call(62, {uint64_t(fd), 2, 0}) == 2, "vector fixture offset");
    f.text("XYZ12", 0x14000);
    const linux_abi::Iovec vectors[]{{0x14000, 2}, {0x14002, 3}};
    f.memory.copy_in(0x15000, vectors, sizeof(vectors));
    require(f.call(70, {uint64_t(fd), 0x15000, 2, 5, 0}) == 5, "pwritev");
    require(f.call(62, {uint64_t(fd), 0, 1}) == 2, "pwritev changed shared offset");
    require(f.call(69, {uint64_t(fd), 0x15000, 2, 5, 0}) == 5 && f.memory.string(0x14000) == "XYZ12",
            "preadv scatter");
    require(f.call(286, {uint64_t(fd), 0x15000, 2, UINT64_MAX, 0, 0}) == 5 &&
                f.memory.string(0x14000) == "cdeXY" && f.call(62, {uint64_t(fd), 0, 1}) == 7,
            "preadv2 current offset");
    f.text("PQRST", 0x14000);
    require(f.call(287, {uint64_t(fd), 0x15000, 2, UINT64_MAX, 0, 2}) == 5 &&
                f.call(62, {uint64_t(fd), 0, 1}) == 12,
            "pwritev2 current offset and RWF_DSYNC");
    require(f.call(287, {uint64_t(fd), 0x15000, 2, 0, 0, 0x80000000}) == -95,
            "pwritev2 unknown durability flag");
    require(f.call(59, {0x16000, 2048}) == 0, "vector pipe2");
    const uint64_t reader = f.memory.load(0x16000, 4), writer = f.memory.load(0x16004, 4);
    require(f.call(66, {writer, 0x15000, 2}) == 5 && f.read(int(reader)) == "PQRST", "writev pipe contents");
    const linux_abi::Iovec invalid[]{{0x14000, 2}, {0x50000, 3}};
    f.memory.copy_in(0x15000, invalid, sizeof(invalid));
    require(f.call(66, {writer, 0x15000, 2}) == -14 && f.call(63, {reader, 0x17000, 5}) == -11,
            "faulting atomic pipe writev changed the pipe");
    require(f.call(70, {uint64_t(fd), 0x15000, 2, 0, 0}) == 2, "pwritev accessible prefix");
    require(f.call(69, {reader, 0x15000, 2, 0, 0}) == -29, "preadv pipe must return ESPIPE");
    require(f.call(34, {uint64_t(-100), f.text("/workspace/created/"), 0755}) == 0, "mkdir trailing slash");
    require(f.call(34, {uint64_t(-100), f.text("/workspace/created/"), 0755}) == -17,
            "mkdir existing directory");
    require(f.call(34, {uint64_t(-100), f.text("/workspace/missing/child/"), 0755}) == -2,
            "mkdir missing parent");
    f.text("/workspace/missing", 0x14100);
    f.text("/workspace/dangling", 0x14200);
    require(f.call(36, {0x14100, uint64_t(-100), 0x14200}) == 0, "mkdir symlink fixture");
    require(f.call(34, {uint64_t(-100), f.text("/workspace/dangling/"), 0755}) == -17 &&
                !std::filesystem::exists(f.source + "/missing"),
            "mkdir followed a dangling final symlink");
}

void memory_permissions_and_code_epochs() {
    Memory memory(1 << 20);
    memory.map(0x10000, 8192, read_perm | write_perm);
    uint64_t epoch = memory.code_epoch;
    memory.map(0x20000, 4096, read_perm | write_perm);
    memory.protect(0x20000, 4096, read_perm);
    memory.unmap(0x20000, 4096);
    require(memory.code_epoch == epoch, "ordinary data mappings invalidated executable code");
    memory.store(0x10ffc, 0x1122334455667788, 8);
    require(memory.load(0x10ffc, 8) == 0x1122334455667788, "cross-page scalar access");
    memory.protect(0x11000, 4096, read_perm);
    bool fault = false;
    try {
        memory.store(0x10ffc, 0, 8);
    } catch (const Failure &) {
        fault = true;
    }
    require(fault && memory.load(0x10ffc, 8) == 0x1122334455667788,
            "cross-page write permissions/atomic fault");
    memory.protect(0x10000, 4096, read_perm | write_perm | exec_perm);
    require(memory.code_epoch != epoch, "executable protection did not invalidate code");
    epoch = memory.code_epoch;
    memory.store(0x10000, 0, 4);
    require(memory.code_epoch != epoch, "same-page executable write did not invalidate code");
    epoch = memory.code_epoch;
    memory.unmap(0x10000, 4096);
    require(memory.code_epoch != epoch, "executable unmap did not invalidate code");
    fault = false;
    try {
        memory.load(0x10000, 4);
    } catch (const Failure &) {
        fault = true;
    }
    require(fault, "stale translation survived unmap");
}

void modern_open_paths_and_anonymous_files() {
    Fixture f;
    constexpr uint64_t how = 0x14000;
    auto openat2 = [&](uint64_t directory, const std::string &name, uint64_t flags,
                       uint64_t mode, uint64_t resolve) {
        f.memory.store(how, flags, 8);
        f.memory.store(how + 8, mode, 8);
        f.memory.store(how + 16, resolve, 8);
        return f.call(437, {directory, f.text(name), how, 24});
    };
    const auto workspace = f.open("/workspace", 16384);
    require(workspace >= 3, "open directory anchor");
    auto file = openat2(workspace, "message", 0, 0, 8);
    require(file >= 3 && f.read(file) == "shared-data", "openat2 beneath real directory");
    require(openat2(workspace, "../secret", 0, 0, 8) == -18,
            "BENEATH allowed directory escape");
    require(openat2(workspace, "/message", 0, 0, 8) == -18,
            "BENEATH allowed absolute pathname");
    std::ofstream(f.source + "/secret") << "scoped-root";
    file = openat2(workspace, "/absolute", 0, 0, 16);
    require(file >= 3 && f.read(file) == "scoped-root", "IN_ROOT did not scope absolute symlink");
    require(openat2(workspace, "absolute", 0, 0, 4) == -40,
            "NO_SYMLINKS followed a symlink");
    require(openat2(uint64_t(-100), "/workspace/message", 0, 0, 1) == -18,
            "NO_XDEV crossed a bind mount");
    std::filesystem::create_directory(f.source + "/pinned");
    std::ofstream(f.source + "/pinned/message") << "original-inode";
    const auto pinned = f.open("/workspace/pinned", 16384);
    require(pinned >= 3, "pin directory before rename");
    std::filesystem::rename(f.source + "/pinned", f.source + "/renamed");
    std::filesystem::create_directory(f.source + "/pinned");
    std::ofstream(f.source + "/pinned/message") << "replacement-inode";
    file = openat2(pinned, "message", 0, 0, 8);
    require(file >= 3 && f.read(file) == "original-inode",
            "openat2 reopened stale directory path after rename");
    const auto path = openat2(workspace, "message", 2097152, 0, 8);
    require(path >= 3 && f.call(80, {uint64_t(path), 0x16000}) == 0,
            "O_PATH metadata handle");
    require(f.call(63, {uint64_t(path), 0x16000, 0}) == -9,
            "O_PATH allowed even zero-length read");
    require(f.call(25, {uint64_t(path), 4, 0}) == -9,
            "O_PATH allowed F_SETFL");
#ifdef __APPLE__
    // apk must discover that linkable anonymous files are unsupported at open,
    // before downloading an index which it would otherwise fail to publish.
    require(openat2(workspace, ".", 4194304 | 16384 | 2, 0600, 8) == -95,
            "linkable O_TMPFILE must report EOPNOTSUPP on Darwin");
    require(f.open("/workspace", 4194304 | 16384 | 2) == -95,
            "legacy openat must reject linkable O_TMPFILE on Darwin");
    require(f.open("/readonly", 4194304 | 16384 | 2) == -30,
            "O_TMPFILE must preserve read-only mount errors");
#endif
    // O_EXCL prevents subsequent linking, so the unlinked scratch-file variant
    // remains supported on both Darwin and Linux.
    const auto temporary = openat2(workspace, ".", 4194304 | 16384 | 128 | 2, 0600, 8);
    require(temporary >= 3 && f.call(80, {uint64_t(temporary), 0x16000}) == 0 &&
                f.memory.load(0x16000 + 20, 4) == 0,
            "O_TMPFILE did not create an unlinked real file");
    require(f.call(64, {uint64_t(temporary), f.text("anonymous"), 9}) == 9,
            "O_TMPFILE not writable");
    f.memory.store(how + 24, 1, 8);
    require(f.call(437, {uint64_t(workspace), f.text("message"), how, 32}) == -7,
            "nonzero open_how extension accepted");
    require(f.call(437, {uint64_t(workspace), f.text("message"), how, 16}) == -22,
            "undersized open_how accepted");
}

void memfd_seals_and_close_range() {
    Fixture f;
    const auto fd = f.call(279, {f.text("shared-code"), 3});
    require(fd == 3 && f.call(25, {uint64_t(fd), 1, 0}) == 1 &&
                f.call(25, {uint64_t(fd), 1034, 0}) == 0,
            "memfd flags and initial seals");
    require(f.call(64, {uint64_t(fd), f.text("data"), 4}) == 4, "write real memfd contents");
    require(f.call(25, {uint64_t(fd), 1033, 2 | 4}) == 0,
            "install memfd grow/shrink seals");
    require(f.call(46, {uint64_t(fd), 3}) == -1 && f.call(46, {uint64_t(fd), 5}) == -1,
            "memfd size seals did not block truncate");
    require(f.call(64, {uint64_t(fd), f.text("x"), 1}) == -1,
            "memfd grow seal did not block write past EOF");
    require(f.call(68, {uint64_t(fd), f.text("X"), 1, 1}) == 1,
            "memfd grow seal blocked in-place overwrite");
    const auto mapping = f.call(222, {0, 4096, 3, 1, uint64_t(fd), 0});
    require(mapping > 0, "map shared memfd");
    require(f.call(25, {uint64_t(fd), 1033, 8}) == -16,
            "SEAL_WRITE ignored an existing writable shared mapping");
    require(f.call(215, {uint64_t(mapping), 4096}) == 0 &&
                f.call(25, {uint64_t(fd), 1033, 8}) == 0,
            "SEAL_WRITE did not become available after unmap");
    require(f.call(68, {uint64_t(fd), f.text("Y"), 1, 1}) == -1,
            "SEAL_WRITE permitted positioned write");
    const auto alias = f.call(23, {uint64_t(fd)});
    require(alias == 4, "duplicate sealed memfd");
    require(f.call(436, {4, UINT32_MAX, 4}) == 0 && f.call(25, {4, 1, 0}) == 1,
            "close_range CLOEXEC did not retain descriptor");
    require(f.call(436, {4, UINT32_MAX, 0}) == 0 && f.call(25, {4, 1, 0}) == -9 &&
                f.call(25, {3, 1034, 0}) == (2 | 4 | 8),
            "close_range closed wrong interval or lost shared seals");
}

void resource_limits_enforce_guest_allocations() {
    Fixture f;
    constexpr uint64_t limit = 0x12000, previous = 0x12020;
    require(f.call(163, {7, limit}) == 0 && f.memory.load(limit, 8) == 1024,
            "getrlimit open file default");
    const auto hard = f.memory.load(limit + 8, 8);
    f.memory.store(limit, 4, 8);
    f.memory.store(limit + 8, hard, 8);
    require(f.call(261, {0, 7, limit, previous}) == 0 &&
                f.memory.load(previous, 8) == 1024,
            "prlimit did not return previous limit");
    const auto file = f.open("/secret");
    require(file == 3 && f.open("/secret") == -24, "NOFILE did not limit actual open");
    require(f.call(25, {uint64_t(file), 0, 4}) == -22,
            "F_DUPFD accepted minimum outside NOFILE");
    require(f.read(file) == "guest-root", "lower limit invalidated an existing descriptor");
    f.memory.store(limit, 8, 8);
    require(f.call(164, {7, limit}) == 0 && f.open("/secret") == 4,
            "raising NOFILE soft limit did not permit allocation");
    f.memory.store(limit, hard + 1, 8);
    f.memory.store(limit + 8, hard + 1, 8);
    require(f.call(164, {7, limit}) == -1, "unprivileged hard limit increase accepted");
    f.memory.store(limit, 9, 8);
    f.memory.store(limit + 8, 8, 8);
    require(f.call(164, {7, limit}) == -22, "soft limit larger than hard accepted");
    require(f.call(163, {7, 1}) == -14 && f.call(164, {7, 1}) == -14,
            "resource limit pointer validation");
    require(f.call(163, {7, previous}) == 0 && f.memory.load(previous, 8) == 8 &&
                f.memory.load(previous + 8, 8) == hard,
            "failed resource limit mutation changed state");
    f.memory.store(limit, 0x10000, 8);
    f.memory.store(limit + 8, UINT64_MAX, 8);
    require(f.call(164, {9, limit}) == 0, "set address space limit");
    require(f.call(222, {0, 4096, 3, 0x22, UINT64_MAX, 0}) == -12,
            "AS limit did not block actual mmap growth");
    require(f.call(222, {0x10000, 4096, 3, 0x32, UINT64_MAX, 0}) == 0x10000,
            "AS limit blocked equal-size fixed replacement");
}

void verify_robust_owner_death(Fixture &f, bool private_wait) {
    constexpr uint64_t head = 0x14000, entry = 0x14100, futex = entry + 8;
    constexpr uint64_t futex_wait = 0, futex_private_flag = 128;
    f.memory.store(head, entry, 8);
    f.memory.store(head + 8, 8, 8);
    f.memory.store(head + 16, 0, 8);
    f.memory.store(entry, head, 8);
    f.memory.store(futex, 0x80000001, 4);
    require(f.call(99, {head, 24}) == 0, "set robust list");
    require(f.call(100, {0, 0x13000, 0x13008}) == 0 && f.memory.load(0x13000, 8) == head &&
                f.memory.load(0x13008, 8) == 24,
            "get robust list");
    f.scheduler.spawn(f.scheduler.current().cpu, 0x18000, 0, 0);
    require(f.scheduler.select(false) && f.scheduler.current().tid == 2, "robust waiter selection");
    require(f.call(98, {futex, futex_wait | (private_wait ? futex_private_flag : 0), 0x80000001, 0}) == 0 &&
                f.scheduler.current().state == Task::State::Waiting,
            "robust mutex waiter");
    require(f.scheduler.select(false) && f.scheduler.current().tid == 1, "robust owner selection");
    require(f.call(93, {0}) == 0, "owner exit");
    require(f.memory.load(futex, 4) == 0xc0000000, "robust owner-death flag");
    if (private_wait) {
        // Linux uses a shared futex wake during robust-list cleanup, even for a private mutex.
        require(!f.scheduler.select(false), "robust owner exit woke a private-key waiter");
        const auto key = f.scheduler.futex_address(f.memory, futex, true);
        require(f.scheduler.wake_futex(key.address, 1, UINT32_MAX, key.space) == 1,
                "private-key waiter was lost");
    }
    require(f.scheduler.select(false), "robust owner exit left waiter blocked");
    require(f.scheduler.current().tid == 2 && f.scheduler.current().cpu.x[0] == 0,
            "robust owner death/wake");
}

void robust_owner_death_and_scheduler_queries() {
    Fixture f;
    require(f.call(121, {0, 0x13000}) == 0 && f.memory.load(0x13000, 4) == 0, "sched_getparam");
    require(f.call(120, {0}) == 0 && f.call(120, {999}) == -3, "sched_getscheduler");
    require(f.call(123, {0, 128, 0x13000}) == 8 && f.memory.load(0x13000, 8) == 1,
            "actual guest CPU affinity");
    f.memory.store(0x13000, 2, 8);
    require(f.call(122, {0, 8, 0x13000}) == -22, "nonexistent execution CPU accepted");
    verify_robust_owner_death(f, false);
    Fixture private_wait;
    verify_robust_owner_death(private_wait, true);
}

} // namespace
int main() {
    try {
        mount_resolution();
        mount_writes_and_readonly();
        descriptor_flags_and_ioctl();
        pipe_capacity_and_short_writes();
        filesystem_operations_and_directory_views();
        blocking_io_and_broken_pipe();
        vector_file_io_and_directory_creation();
        memory_permissions_and_code_epochs();
        modern_open_paths_and_anonymous_files();
        memfd_seals_and_close_range();
        resource_limits_enforce_guest_allocations();
        robust_owner_death_and_scheduler_queries();
        std::puts("rootfs bind namespace, persistence, readonly, dup and Linux ioctl checks passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "unexpected Linux errno %d\n", error.number);
    }
    return 1;
}
