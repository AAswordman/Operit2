#include "linux/abi.h"
#include "linux/syscalls.h"
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <set>
#include <stdexcept>

using namespace operit::arm64;
namespace {
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Fixture {
    std::string root;
    Memory memory{8 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler scheduler;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/operit-proc-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        root = created;
        std::ofstream(root + "/program") << "program-data";
        fs = std::make_unique<Filesystem>(root.c_str());
        memory.map(0x10000, 0x20000, read_perm | write_perm);
        memory.map(0x40000, 0x3000, read_perm | exec_perm);
        memory.describe(0x40000, 0x3000, {"/program", 0x1000, 0, 123, false});
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, *fs, config, scheduler);
        text("program", 0x15000);
        text("arg", 0x15020);
        text("KEY=value", 0x15040);
        const uint64_t stack[] = {2, 0x15000, 0x15020, 0, 0x15040, 0, 6, 4096, 0, 0};
        memory.copy_in(0x16000, stack, sizeof(stack));
        sys->set_process_image("/program", 0x16000);
    }
    ~Fixture() {
        std::filesystem::remove_all(root);
    }
    uint64_t text(const std::string &value, uint64_t address = 0x10000) {
        memory.copy_in(address, value.c_str(), value.size() + 1);
        return address;
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> args = {}) {
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        std::copy(args.begin(), args.end(), cpu.x.begin());
        cpu.x[8] = number;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    int64_t open(const std::string &name, uint64_t flags = 0) {
        return call(56, {uint64_t(-100), text(name), flags, 0600});
    }
    std::string read(int fd, size_t count = 16384) {
        int64_t size = call(63, {uint64_t(fd), 0x18000, count});
        require(size >= 0, "virtual read");
        std::string result(size, '\0');
        memory.copy_out(0x18000, result.data(), result.size());
        return result;
    }
    std::string contents(const std::string &name) {
        auto fd = open(name);
        require(fd >= 0, "virtual open");
        auto result = read(fd);
        call(57, {uint64_t(fd)});
        return result;
    }
    std::string link(const std::string &name) {
        auto length = call(78, {uint64_t(-100), text(name), 0x18000, 4096});
        require(length >= 0, "readlink");
        std::string result(length, '\0');
        memory.copy_out(0x18000, result.data(), result.size());
        return result;
    }
    std::set<std::string> directory(int fd) {
        int64_t length = call(61, {uint64_t(fd), 0x18000, 16384});
        require(length >= 0, "getdents");
        std::set<std::string> result;
        for (int64_t at = 0; at < length;) {
            result.insert(memory.string(0x18000 + at + 19));
            at += memory.load(0x18000 + at + 16, 2);
        }
        return result;
    }
};
void regular_files() {
    Fixture f;
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::File, "/etc/runtime.conf", 0444);
    node->contents = [] { return "nameserver 192.0.2.1\n"; };
    f.fs->mount_virtual("/etc/runtime.conf", node);
    const auto contents = node->contents();
    int fd = f.open("/etc/runtime.conf");
    require(fd >= 0, "regular virtual file open");
    require(f.call(80, {uint64_t(fd), 0x17000}) == 0, "regular virtual file fstat");
    linux_abi::Stat metadata{};
    f.memory.copy_out(0x17000, &metadata, sizeof(metadata));
    require(metadata.size == int64_t(contents.size()), "regular virtual file size");
    require(f.call(62, {uint64_t(fd), 0, 2}) == int64_t(contents.size()), "regular virtual file SEEK_END");
    require(f.read(fd).empty(), "regular virtual file EOF");
    require(f.call(62, {uint64_t(fd), uint64_t(-4), 2}) == int64_t(contents.size() - 4),
            "regular virtual file negative end offset");
    require(f.read(fd) == contents.substr(contents.size() - 4), "regular virtual file seek/read");
    require(f.call(62, {uint64_t(fd), 0, 0}) == 0 && f.read(fd) == contents, "regular virtual file rewind");
    require(f.call(62, {uint64_t(fd), uint64_t(-1), 0}) == -22, "regular virtual file negative position");
    int sequence = f.open("/proc/self/status");
    require(sequence >= 0 && f.call(62, {uint64_t(sequence), 0, 2}) == -22, "proc sequence file SEEK_END");
    require(f.call(80, {uint64_t(sequence), 0x17000}) == 0, "proc sequence fstat");
    f.memory.copy_out(0x17000, &metadata, sizeof(metadata));
    require(metadata.size == 0, "proc sequence file size");
}
void devices() {
    Fixture f;
    require(!std::filesystem::exists(f.root + "/dev") && !std::filesystem::exists(f.root + "/proc"),
            "virtual mounts modified external rootfs");
    int null = f.open("/dev/null", 2), zero = f.open("/dev/zero", 2), full = f.open("/dev/full", 2);
    require(null >= 0 && zero >= 0 && full >= 0, "devices require mknod/rootfs fixtures");
    require(f.read(null).empty(), "null EOF");
    require(f.call(64, {uint64_t(null), f.text("discard"), 7}) == 7, "null write");
    const linux_abi::Iovec vectors[]{{0x10000, 3}, {0x10003, 4}};
    f.memory.copy_in(0x19000, vectors, sizeof(vectors));
    require(f.call(66, {uint64_t(null), 0x19000, 2}) == 7, "null writev");
    require(f.read(zero, 4096) == std::string(4096, '\0'), "zero read");
    require(f.read(full, 31) == std::string(31, '\0'), "full read");
    require(f.call(64, {uint64_t(full), f.text("x"), 1}) == -28, "full must fail ENOSPC");
    require(f.call(62, {uint64_t(zero), 1234, 0}) == 0, "device seek");
    require(f.call(80, {uint64_t(null), 0x17000}) == 0, "device fstat");
    linux_abi::Stat metadata{};
    f.memory.copy_out(0x17000, &metadata, sizeof(metadata));
    require((metadata.mode & S_IFMT) == S_IFCHR && metadata.rdevice == 259 && metadata.size == 0,
            "Linux null metadata");
    auto random = f.contents("/dev/urandom");
    require(random.size() == 16384 && random != std::string(random.size(), '\0'), "real entropy");
    require(f.open("/dev/zero", 16384) == -20, "O_DIRECTORY on device");
    require(f.open("/dev/null", 64 | 128) == -17, "exclusive creation of device");
    require(f.call(35, {uint64_t(-100), f.text("/dev/null"), 0}) == -30, "virtual device metadata mutation");
    auto mapped = f.call(222, {0, 4096, 3, 2, uint64_t(zero), 0});
    require(mapped > 0 && f.memory.load(mapped, 8) == 0, "zero private mmap");
    f.memory.store(mapped, 0xfeed, 8);
    require(f.call(222, {0, 4096, 3, 2, uint64_t(null), 0}) == -19, "null mmap must fail ENODEV");
    require(f.sys->unsupported_count == 0, "device unsupported syscall");
}
void process_and_memory() {
    Fixture f;
    require(f.link("/proc/self") == "1" && f.link("/proc/thread-self") == "1/task/1", "process links");
    require(f.link("/proc/self/exe") == "/program" && f.link("/proc/self/root") == "/", "guest image links");
    require(f.contents("/proc/self/exe") == "program-data", "exe follows guest root");
    require(f.contents("/proc/self/cmdline") == std::string("program\0arg\0", 12), "cmdline bytes/order");
    require(f.contents("/proc/self/environ") == std::string("KEY=value\0", 10), "environment bytes");
    f.text("NEW", 0x15020);
    require(f.contents("/proc/self/cmdline") == std::string("program\0NEW\0", 12),
            "cmdline must reflect guest memory");
    auto auxv = f.contents("/proc/self/auxv");
    require(auxv.size() == 32 && auxv[0] == 6 && uint8_t(auxv[9]) == 16, "actual initial auxv");
    auto maps = f.contents("/proc/self/maps");
    require(maps.find("000000040000-000000043000 r-xp 00001000 00:00 123  /program") != std::string::npos,
            "maps backing/permissions");
    f.memory.protect(0x41000, 0x1000, read_perm | write_perm);
    maps = f.contents("/proc/self/maps");
    require(maps.find("000000041000-000000042000 rw-p 00002000") != std::string::npos,
            "maps mprotect split offset");
    f.memory.unmap(0x41000, 0x1000);
    require(f.contents("/proc/self/maps").find("000000041000-") == std::string::npos, "maps unmapped range");
    int status = f.open("/proc/self/status");
    auto first = f.read(status);
    require(first.find("Threads:\t1\n") != std::string::npos, "thread count");
    auto child = f.scheduler.spawn(f.scheduler.current().cpu, 0x28000, 0, 0);
    require(f.call(62, {uint64_t(status), 0, 0}) == 0 &&
                f.read(status).find("Threads:\t2\n") != std::string::npos,
            "rewind must regenerate proc state");
    require(f.contents("/proc/self/task/" + std::to_string(child) + "/status").find("Pid:\t2\n") !=
                std::string::npos,
            "task-specific status");
    int tasks = f.open("/proc/self/task", 16384);
    require(f.directory(tasks).count("2") == 1, "live task directory");
    f.scheduler.select();
    require(f.link("/proc/thread-self") == "1/task/2", "thread-self current task");
    f.call(93, {0});
    f.scheduler.select();
    require(f.open("/proc/self/task/2/status") == -2, "exited task still exposed");
    require(f.call(62, {uint64_t(tasks), 0, 0}) == 0 && !f.directory(tasks).count("2"),
            "task enumeration refresh");
    require(f.open("/proc/self/maps", 1) == -13 && f.open("/proc/self/missing", 64) == -30,
            "proc write errors");
    auto root = f.directory(f.open("/"));
    require(root.count("proc") && root.count("dev"), "virtual mounts absent from root getdents");
    require(f.contents("/proc/mounts").find("proc /proc proc rw") != std::string::npos, "actual mount table");
    require(f.sys->unsupported_count == 0, "proc unsupported syscall");
}
void descriptions_and_paths() {
    Fixture f;
    int fd = f.open("/proc/self/cmdline");
    require(f.read(fd, 3) == "pro", "partial virtual read");
    int duplicate = f.call(23, {uint64_t(fd)});
    require(f.read(duplicate, 4) == "gram", "dup did not share virtual offset");
    require(f.call(62, {uint64_t(duplicate), 0, 0}) == 0 && f.read(fd, 7) == "program", "dup seek");
    require(f.call(67, {uint64_t(fd), 0x18000, 3, 0}) == 3 && f.read(fd, 1) == std::string(1, '\0'),
            "pread changed offset");
    // Native AArch64 openat adds O_LARGEFILE, which F_SETFL must preserve.
    constexpr uint64_t large_file = uint64_t{1} << 17;
    require(f.call(25, {uint64_t(fd), 3}) == int64_t(large_file),
            "virtual open missing AArch64 O_LARGEFILE");
    require(f.call(25, {uint64_t(fd), 4, 2048}) == 0 &&
                f.call(25, {uint64_t(duplicate), 3}) == int64_t(large_file | 2048),
            "virtual shared status flags");
    require(f.call(25, {uint64_t(duplicate), 4, 0}) == 0 &&
                f.call(25, {uint64_t(fd), 3}) == int64_t(large_file),
            "virtual shared status flag reset lost immutable flags");
    int directory = f.open("/proc/self", 16384);
    require(f.call(56, {uint64_t(directory), f.text("comm"), 0, 0}) >= 0, "relative virtual openat");
    require(f.call(49, {f.text("/proc/self")}) == 0 && f.link("/proc/self/cwd") == "/proc/1",
            "virtual chdir/cwd");
    require(f.open("/proc/self/../../program") >= 0 && f.open("/dev/../program") >= 0,
            "virtual mount .. traversal");
    int regular = f.open("/program");
    std::string link = "/proc/self/fd/" + std::to_string(regular);
    require(f.link(link) == "/program" && f.contents(link) == "program-data", "fd guest path link");
    require(f.call(79, {uint64_t(-100), f.text(link), 0x17000, 256}) == 0, "fd lstat");
    linux_abi::Stat metadata{};
    f.memory.copy_out(0x17000, &metadata, sizeof(metadata));
    require((metadata.mode & S_IFMT) == S_IFLNK, "fd lstat followed link");
    auto fds = f.directory(f.open("/proc/self/fd", 16384));
    require(fds.count(std::to_string(regular)), "fd directory missing open descriptor");
    f.call(57, {uint64_t(regular)});
    require(f.open(link) == -2, "closed descriptor still exposed");
    int devices = f.open("/dev", 16384);
    require(f.call(61, {uint64_t(devices), 0x18000, 1}) == -22, "tiny getdents accepted");
    require(f.directory(devices).count("urandom"), "failed getdents advanced directory cursor");
}
void epoll_timespec_waits() {
    Fixture f;
    constexpr uint64_t event = 0x17000, timeout = 0x17100, mask = 0x17120;
    constexpr uint64_t output = 0x18000, value = 0x19000;
    const auto epoll = f.call(20, {524288});
    const auto counter = f.call(19, {1, 2048});
    require(epoll >= 3 && counter >= 3, "epoll/eventfd creation");
    f.memory.store(event, 1, 4);
    f.memory.store(event + 8, 0x1122334455667788, 8);
    require(f.call(21, {uint64_t(epoll), 1, uint64_t(counter), event}) == 0, "epoll add counter");
    auto wait = [&](uint64_t pointer = 0x17100, uint64_t mask_pointer = 0, uint64_t size = 8) {
        return f.call(441, {uint64_t(epoll), output, 1, pointer, mask_pointer, size});
    };
    f.memory.store(timeout, uint64_t(-1), 8);
    f.memory.store(timeout + 8, 0, 8);
    require(wait() == -22, "epoll_pwait2 accepted negative seconds with a ready event");
    f.memory.store(timeout, 0, 8);
    f.memory.store(timeout + 8, 1000000000, 8);
    require(wait() == -22, "epoll_pwait2 accepted invalid nanoseconds");
    f.memory.store(timeout + 8, 0, 8);
    f.memory.store(mask, 1u << 11, 8);
    require(wait(0x500000) == -14, "epoll_pwait2 timeout pointer must be checked");
    require(wait(timeout, mask, 4) == -22, "epoll_pwait2 sigset size");
    const uint64_t original_mask = 1u << 9;
    f.scheduler.current().signal_mask = original_mask;
    f.memory.protect(event, 4096, read_perm);
    require(wait(timeout, mask) == 1, "epoll_pwait2 ready event with read-only timespec");
    require(f.memory.load(output, 4) == 1 && f.memory.load(output + 4, 4) == 0 &&
                f.memory.load(output + 8, 8) == 0x1122334455667788,
            "epoll_pwait2 AArch64 event layout or data");
    require(f.scheduler.current().signal_mask == original_mask && f.memory.load(timeout, 8) == 0 &&
                f.memory.load(timeout + 8, 8) == 0,
            "epoll_pwait2 changed timeout or leaked temporary signal mask");
    require(f.call(63, {uint64_t(counter), value, 8}) == 8, "consume eventfd readiness");
    require(wait() == 0 && f.scheduler.current().state == Task::State::Runnable,
            "epoll_pwait2 zero timeout must not block");
    f.memory.protect(event, 4096, read_perm | write_perm);
    const auto sibling = f.scheduler.spawn(f.scheduler.current().cpu, 0, 0, 0);
    require(sibling > 1, "epoll waiter sibling");
    f.memory.store(timeout, 1, 8);
    f.memory.store(timeout + 8, 345678901, 8);
    const auto before = TaskClock::now();
    require(wait(timeout, mask) == 0, "epoll_pwait2 scheduling entry");
    const auto after = TaskClock::now();
    auto &waiting = f.scheduler.current();
    const auto duration = std::chrono::seconds(1) + std::chrono::nanoseconds(345678901);
    require(waiting.state == Task::State::Waiting && waiting.deadline &&
                *waiting.deadline >= before + duration && *waiting.deadline <= after + duration,
            "epoll_pwait2 lost timespec precision");
    require(waiting.signal_mask == (1u << 11), "epoll_pwait2 temporary mask not installed");
    require(f.scheduler.select() && f.scheduler.current().tid == sibling,
            "epoll_pwait2 blocked another runnable guest task");
    f.memory.store(value, 1, 8);
    require(f.call(64, {uint64_t(counter), value, 8}) == 8, "wake epoll waiter");
    require(f.scheduler.select() && f.scheduler.current().tid == 1 &&
                f.scheduler.current().cpu.x[0] == 1 &&
                f.scheduler.current().signal_mask == original_mask,
            "epoll_pwait2 wake did not return the event and restore mask");
    require(f.call(63, {uint64_t(counter), value, 8}) == 8, "consume second event");
    require(wait(0, mask) == 0 && f.scheduler.current().state == Task::State::Waiting &&
                !f.scheduler.current().deadline,
            "epoll_pwait2 null timespec must wait without a deadline");
    require(f.scheduler.select() && f.scheduler.current().tid == sibling, "infinite epoll wait scheduling");
    f.memory.store(value, 1, 8);
    require(f.call(64, {uint64_t(counter), value, 8}) == 8, "wake infinite epoll waiter");
    require(f.scheduler.select() && f.scheduler.current().tid == 1 &&
                f.scheduler.current().cpu.x[0] == 1 &&
                f.scheduler.current().signal_mask == original_mask,
            "epoll_pwait2 infinite wake cleanup");
    require(f.call(63, {uint64_t(counter), value, 8}) == 8, "consume final event");
    f.memory.store(timeout, uint64_t(INT64_MAX), 8);
    f.memory.store(timeout + 8, 999999999, 8);
    require(wait(timeout, mask) == 0 && f.scheduler.current().deadline &&
                *f.scheduler.current().deadline == TaskClock::time_point::max(),
            "epoll_pwait2 huge timeout must saturate without overflowing");
    // Expire via the scheduler to verify the timeout cleanup without sleeping.
    f.scheduler.current().deadline = TaskClock::now();
    require(f.scheduler.select(), "epoll_pwait2 timeout scheduling");
    const auto &expired = f.scheduler.find(1);
    require(expired.state == Task::State::Runnable && expired.cpu.x[0] == 0 &&
                expired.signal_mask == original_mask && !expired.deadline,
            "epoll_pwait2 timeout did not restore the original signal mask");
}
} // namespace
int main() {
    try {
        regular_files();
        devices();
        process_and_memory();
        descriptions_and_paths();
        epoll_timespec_waits();
        std::puts("devfs and dynamic guest procfs passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
