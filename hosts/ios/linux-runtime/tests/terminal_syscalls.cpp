#include "linux/syscalls.h"
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
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
    Scheduler scheduler;
    std::unique_ptr<Filesystem> filesystem;
    std::unique_ptr<Syscalls> syscalls;
    Fixture() {
        char pattern[] = "/tmp/operit-terminal-XXXXXX";
        const char *created = ::mkdtemp(pattern);
        require(created, "temporary guest root");
        root = created;
        std::ofstream(root + "/message") << "sendfile\n";
        memory.map(0x10000, 0x10000, read_perm | write_perm);
        filesystem = std::make_unique<Filesystem>(root.c_str());
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        syscalls = std::make_unique<Syscalls>(memory, *filesystem, config, scheduler);
        TerminalHooks hooks;
        hooks.process = [](uint32_t pid) { return TerminalProcess{pid, pid, pid, std::nullopt}; };
        syscalls->terminal_manager()->set_hooks(std::move(hooks));
    }
    ~Fixture() {
        syscalls.reset();
        filesystem.reset();
        std::filesystem::remove_all(root);
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> arguments) {
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        std::copy(arguments.begin(), arguments.end(), cpu.x.begin());
        cpu.x[8] = number;
        syscalls->handle(cpu);
        require(scheduler.current().state != Task::State::Waiting, "nonblocking terminal syscall stalled");
        return int64_t(cpu.x[0]);
    }
    uint64_t text(const std::string &value, uint64_t at = 0x10000) {
        memory.copy_in(at, value.c_str(), value.size() + 1);
        return at;
    }
    uint64_t open(const std::string &path, uint32_t flags) {
        auto fd = call(56, {uint64_t(-100), text(path), flags, 0600});
        require(fd >= 0, "guest terminal open failed");
        return uint64_t(fd);
    }
    std::string read(uint64_t fd, bool vector = false) {
        memory.store(0x11000, 0x13000, 8);
        memory.store(0x11008, 64, 8);
        auto count = vector ? call(65, {fd, 0x11000, 1}) : call(63, {fd, 0x13000, 64});
        require(count >= 0, "guest terminal read failed");
        std::string bytes(size_t(count), '\0');
        memory.copy_out(0x13000, bytes.data(), bytes.size());
        return bytes;
    }
    int64_t write(uint64_t fd, const std::string &value, bool vector = false) {
        text(value, 0x12000);
        memory.store(0x11000, 0x12000, 8);
        memory.store(0x11008, value.size(), 8);
        return vector ? call(66, {fd, 0x11000, 1}) : call(64, {fd, 0x12000, value.size()});
    }
};
void exercise(bool restricted) {
    Fixture f;
    auto master = f.open("/dev/ptmx", 2 | 2048 | 256);
    f.memory.store(0x14000, 0, 4);
    require(f.call(29, {master, 0x40045431, 0x14000}) == 0, "unlock guest PTY");
    require(f.call(29, {master, 0x80045430, 0x14000}) == 0, "query guest PTY number");
    auto slave = f.open("/dev/pts/" + std::to_string(f.memory.load(0x14000, 4)), 2 | 2048 | 256);
    require(f.write(master, "a\r") == 2 && f.read(slave) == "a\n" && f.read(master) == "a\r\n",
            "scalar terminal I/O bypassed line discipline");
    require(f.write(slave, "vector\n", true) == 7 && f.read(master, true) == "vector\r\n",
            "vector terminal I/O bypassed output processing");
    require(f.write(master, "first\nsecond\n", true) == 13 && f.read(slave, true) == "first\n" &&
                f.read(slave, true) == "second\n", "readv crossed canonical line boundaries");
    f.read(master);
    require(f.write(master, "\x04") == 1, "canonical EOF input failed");
    require(f.call(29, {slave, 0x541b, 0x14000}) == 0, "TIOCINQ failed");
    // Darwin native PTYs count the EOF record in FIONREAD. The software
    // backend's private wake token must stay invisible to the Linux guest.
    if (restricted)
        require(f.memory.load(0x14000, 4) == 0, "TIOCINQ exposed a private EOF token");
    require(f.read(slave).empty(), "guest canonical EOF read failed");
    require(f.call(29, {slave, 0x540b, 1}) == 0, "flush native EOF echo");
    auto source = f.open("/message", 0);
    auto sent = f.call(71, {slave, source, 0, 9});
    auto sent_output = f.read(master);
    if (sent != 9 || sent_output != "sendfile\r\n") {
        std::fprintf(stderr, "sendfile result=%lld output=", static_cast<long long>(sent));
        for (unsigned char byte : sent_output)
            std::fprintf(stderr, "%02x ", byte);
        std::fputc('\n', stderr);
        require(false, "sendfile bypassed terminal output processing");
    }
    require(f.call(29, {slave, 0x5401, 0x14000}) == 0, "TCGETS failed");
    f.memory.store(0x1400c, 0, 4); // raw input, no echo
    f.memory.store(0x14000, 0, 4);
    require(f.call(29, {slave, 0x5402, 0x14000}) == 0, "TCSETS failed");
    const std::string binary("\0\r\x03", 3);
    require(f.write(master, binary, true) == 3 && f.read(slave, true) == binary,
            "raw vector I/O changed control bytes");
    require(f.call(63, {master, 0x13000, 64}) == -11, "raw input was echoed");
    require(f.call(29, {slave, 0x540a, 0}) == 0, "TCXONC stop failed");
    require(f.write(slave, "paused\n") == 7, "paused terminal write failed");
    require(f.call(63, {master, 0x13000, 64}) == -11, "TCXONC did not stop terminal output");
    require(f.call(29, {slave, 0x540a, 1}) == 0 && f.read(master) == "paused\r\n",
            "TCXONC resume lost terminal output");
    require(f.call(57, {master}) == 0 && f.read(slave).empty(), "guest slave hangup did not return EOF");
}
} // namespace
int main(int argc, char **argv) {
    try {
        exercise(argc == 2 && std::string(argv[1]) == "--require-software");
        std::puts("terminal syscalls: /dev/ptmx, scalar/vector I/O, canonical EOF, sendfile, termios, flow, hangup passed");
        return 0;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "unexpected guest errno %d\n", error.number);
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    }
    return 1;
}
