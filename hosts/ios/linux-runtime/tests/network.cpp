#include "linux/syscalls.h"
#include <array>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <stdexcept>

using namespace operit::arm64;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
struct Fixture {
    std::string root;
    Memory memory{8 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler tasks;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/ort-net-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "temporary root");
        root = created;
        memory.map(0x10000, 0x10000, read_perm | write_perm);
        fs = std::make_unique<Filesystem>(root.c_str());
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, *fs, config, tasks);
    }
    ~Fixture() {
        sys.reset();
        std::filesystem::remove_all(root);
    }
    int64_t call(uint64_t syscall, std::initializer_list<uint64_t> args = {}) {
        auto &cpu = tasks.current().cpu;
        cpu.x.fill(0);
        std::copy(args.begin(), args.end(), cpu.x.begin());
        cpu.x[8] = syscall;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    uint64_t ipv4(uint16_t port = 0, uint64_t at = 0x10000) {
        std::array<uint8_t, 16> bytes{2, 0, uint8_t(port >> 8), uint8_t(port), 127, 0, 0, 1};
        memory.copy_in(at, bytes.data(), bytes.size());
        return at;
    }
    void text(const char *value) {
        memory.copy_in(0x14000, value, std::strlen(value));
    }
};
void socket_abi() {
    Fixture f;
    require(f.call(198, {999, 1, 0}) == -97, "address family errno");
    require(f.call(198, {2, 1 | 0x10000, 0}) == -22, "socket flags validation");
    int server = f.call(198, {2, 1 | 2048 | 524288, 0});
    require(server >= 0, "TCP socket");
    require(f.call(25, {uint64_t(server), 1}) == 1 && f.call(25, {uint64_t(server), 3}) == (2 | 2048),
            "socket guest flags");
    require(f.call(200, {uint64_t(server), f.ipv4(), 16}) == 0, "TCP bind");
    f.memory.store(0x11000, 16, 4);
    require(f.call(204, {uint64_t(server), 0x12000, 0x11000}) == 0 && f.memory.load(0x11000, 4) == 16 &&
                f.memory.load(0x12000, 2) == 2,
            "sockaddr conversion");
    uint16_t port = (f.memory.load(0x12002, 1) << 8) | f.memory.load(0x12003, 1);
    require(port > 0, "actual allocated port");
    require(f.call(201, {uint64_t(server), 4}) == 0, "listen");
    require(f.call(242, {uint64_t(server), 0, 0, 2048}) == -11, "empty nonblocking accept");
    int conflict = f.call(198, {2, 1, 0});
    require(f.call(200, {uint64_t(conflict), f.ipv4(port), 16}) == -98, "EADDRINUSE translation");
    f.call(57, {uint64_t(conflict)});
    int client = f.call(198, {2, 1, 0});
    require(client >= 0, "client socket");
    require(f.call(203, {uint64_t(client), f.ipv4(port), 16}) == 0, "blocking connect");
    if (f.tasks.current().state == Task::State::Waiting)
        f.tasks.select();
    require(f.tasks.current().cpu.x[0] == 0, "connect completion");
    f.memory.store(0x11000, 16, 4);
    int accepted = f.call(242, {uint64_t(server), 0x12000, 0x11000, 2048});
    require(accepted >= 0, "accept4");
    f.text("payload");
    require(f.call(206, {uint64_t(client), 0x14000, 7, 16384, 0, 0}) == 7, "TCP send");
    f.memory.store(0x16000, accepted, 4);
    f.memory.store(0x16004, 1, 2);
    f.memory.store(0x17000, 1, 8);
    f.memory.store(0x17008, 0, 8);
    auto ready = f.call(73, {0x16000, 1, 0x17000, 0, 8});
    if (f.tasks.current().state == Task::State::Waiting) {
        f.tasks.select();
        ready = int64_t(f.tasks.current().cpu.x[0]);
    }
    require(ready == 1, "TCP readiness");
    auto count = f.call(207, {uint64_t(accepted), 0x15000, 7, 0, 0, 0});
    require(count == 7 && f.memory.load(0x15000, 4) == 0x6c796170, "TCP receive");
    f.memory.store(0x11000, 4, 4);
    require(f.call(209, {uint64_t(client), 1, 4, 0x13000, 0x11000}) == 0 && f.memory.load(0x13000, 4) == 0,
            "SO_ERROR zero must remain zero");
    require(f.call(209, {uint64_t(client), 999, 999, 0x13000, 0x11000}) == -92, "ENOPROTOOPT translation");
    require(f.call(210, {uint64_t(client), 7}) == -22, "shutdown validation");
    require(f.sys->unsupported_count == 0, "network syscall dispatch missing");
}
void poll_and_task_waits() {
    Fixture f;
    require(f.call(199, {1, 1, 0, 0x10000}) == 0, "UNIX socketpair");
    auto first = f.memory.load(0x10000, 4), second = f.memory.load(0x10004, 4);
    f.memory.store(0x11000, first, 4);
    f.memory.store(0x11004, 1, 2);
    f.memory.store(0x12000, 0, 8);
    f.memory.store(0x12008, 0, 8);
    require(f.call(73, {0x11000, 1, 0x12000, 0, 8}) == 0 && f.memory.load(0x11006, 2) == 0,
            "ppoll timeout zero");
    f.tasks.spawn(f.tasks.current().cpu, 0x1f000, 0, 0);
    f.memory.store(0x13000, 0x200, 8);
    f.memory.store(0x12000, 1, 8);
    f.memory.store(0x12008, 0, 8);
    require(f.call(73, {0x11000, 1, 0x12000, 0x13000, 8}) == 0 &&
                f.tasks.current().state == Task::State::Waiting,
            "ppoll did not suspend");
    require(f.tasks.current().signal_mask == 0x200, "temporary ppoll mask");
    f.tasks.select();
    require(f.tasks.current().tid == 2, "poll blocked other guest tasks");
    f.text("ready");
    require(f.call(206, {second, 0x14000, 5, 0, 0, 0}) == 5, "cross-task socket wake write");
    f.tasks.select();
    require(f.tasks.current().tid == 1 && f.tasks.current().cpu.x[0] == 1 &&
                f.tasks.current().signal_mask == 0 && f.memory.load(0x11006, 2) == 1,
            "poll readiness/mask restore");
    f.memory.store(0x11000, 999, 4);
    f.memory.store(0x12000, 0, 8);
    f.memory.store(0x12008, 0, 8);
    require(f.call(73, {0x11000, 1, 0x12000, 0, 8}) == 1 && f.memory.load(0x11006, 2) == 32,
            "poll invalid descriptor");
    f.memory.store(0x11000, uint64_t{1} << first, 8);
    require(f.call(72, {first + 1, 0x11000, 0, 0, 0x12000, 0}) == 1 &&
                f.memory.load(0x11000, 8) == (uint64_t{1} << first),
            "pselect descriptor set");
    require(f.call(207, {first, 0x15000, 5, 0, 0, 0}) == 5, "consume wake data");
    require(f.call(207, {first, 0x15000, 5, 64, 0, 0}) == -11, "MSG_DONTWAIT");
    require(f.sys->unsupported_count == 0, "poll syscall dispatch missing");
}
void descriptor_cycles() {
    Fixture f;
    require(f.call(199, {1, 1, 0, 0x10000}) == 0, "rights cycle socketpair");
    uint64_t sender = f.memory.load(0x10000, 4), receiver = f.memory.load(0x10004, 4);
    f.memory.store(0x12000, 20, 8);
    f.memory.store(0x12008, 1, 4);
    f.memory.store(0x1200c, 1, 4);
    f.memory.store(0x12010, receiver, 4);
    f.text("cycle");
    f.memory.store(0x13000, 0x14000, 8);
    f.memory.store(0x13008, 1, 8);
    const uint64_t message[] = {0, 0, 0x13000, 1, 0x12000, 24, 0};
    f.memory.copy_in(0x11000, message, sizeof(message));
    require(f.call(211, {sender, 0x11000, 0}) == 1, "queue socket referring to itself");
    require(f.call(57, {sender}) == 0 && f.call(57, {receiver}) == 0, "close descriptor cycle");
    // ASan/LSan verifies VM teardown releases even cycles with no guest fd left.
}

} // namespace
int main() {
    try {
        socket_abi();
        poll_and_task_waits();
        descriptor_cycles();
        std::puts("socket ABI, TCP, socketpair, network errno, ppoll/pselect and task wake passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
