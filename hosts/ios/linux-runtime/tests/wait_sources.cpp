#include "linux/files/record_lock.h"
#include "linux/threads/futex_pi.h"
#include "linux/timers/ready_pipe.h"
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <stdexcept>
#include <thread>

using namespace operit::arm64;
namespace {
// Fail on observable scheduler behavior rather than matching implementation text.
void require(bool condition, const char *message) {
    if (!condition) throw std::runtime_error(message);
}
// Native event waits use the coordinator's actual sources and real steady-clock deadline.
std::vector<pollfd> sources(Scheduler &scheduler, int64_t &timeout) {
    const auto deadline = scheduler.wait_deadline();
    timeout = deadline ? std::chrono::duration_cast<std::chrono::nanoseconds>(*deadline - TaskClock::now()).count() : -1;
    return scheduler.wait_descriptors();
}
// Independent VM coordinators must never consume one another's lock notifications.
void disjoint_lock_waits(const char *root) {
    const std::string path = std::string(root) + "/locks";
    FD external(::open(path.c_str(), O_RDWR | O_CREAT | O_CLOEXEC, 0600));
    require(external.get() >= 0, "create shared lock inode");
    struct Vm {
        Memory memory{1 << 20};
        Filesystem filesystem;
        Scheduler scheduler;
        // Use real coordinators and lock registries without invoking an architecture-specific CPU.
        explicit Vm(const char *root) : filesystem(root) {}
        // Retire VM ownership before its memory and open descriptions disappear.
        ~Vm() { files::record_lock_vm_exit(scheduler); }
    } first(root), second(root);
    for (auto *machine : {&first, &second})
        machine->memory.map(0x10000, page_size, read_perm | write_perm);
    Descriptor entries[] = {{FD(::open(path.c_str(), O_RDWR | O_CLOEXEC)), "/locks", 2},
                            {FD(::open(path.c_str(), O_RDWR | O_CLOEXEC)), "/locks", 2}};
    struct GuestLock {
        int16_t type = 1, whence = SEEK_SET;
        uint32_t padding = 0;
        int64_t start, length = 8;
        int32_t pid = 0;
        uint32_t padding2 = 0;
    };
    auto native = [&](int64_t start, bool acquire) {
        struct flock range{};
        range.l_type = acquire ? F_WRLCK : F_UNLCK;
        range.l_whence = SEEK_SET;
        range.l_start = start;
        range.l_len = 8;
        require(::fcntl(external.get(), F_OFD_SETLK, &range) == 0, "change external range lock");
    };
    auto block = [&](Vm &machine, Descriptor &entry, int64_t start) {
        GuestLock lock{1, SEEK_SET, 0, start};
        machine.memory.copy_in(0x10000, &lock, sizeof(lock));
        require(files::record_lock(machine.memory, machine.filesystem, machine.scheduler, entry, 38, 0x10000) == 0 &&
                machine.scheduler.current().state == Task::State::Waiting, "guest lock did not block");
    };
    native(0, true);
    native(16, true);
    block(first, entries[0], 0);
    block(second, entries[1], 16);
    int64_t timeout;
    auto left = sources(first.scheduler, timeout);
    require(timeout == -1, "native lock wait invented a retry deadline");
    auto right = sources(second.scheduler, timeout);
    require(timeout == -1, "second VM lock wait invented a retry deadline");
    require(::poll(left.data(), left.size(), 0) == 0 && ::poll(right.data(), right.size(), 0) == 0,
            "blocked ranges woke without a lock change");
    // Release the second range first: it must progress even while the first stays held.
    native(16, false);
    require(::poll(right.data(), right.size(), 2000) == 1, "disjoint second range lost its producer wake");
    require(second.scheduler.select(false) && second.scheduler.current().cpu.x[0] == 0,
            "second VM failed to accept its range lock");
    left = sources(first.scheduler, timeout);
    require(timeout == -1 && !first.scheduler.select(false), "unreleased first range completed");
    left = sources(first.scheduler, timeout);
    require(::poll(left.data(), left.size(), 0) == 0, "unreleased range spins after another VM's commit");
    native(0, false);
    require(::poll(left.data(), left.size(), 2000) == 1, "first VM lost its independent notification");
    require(first.scheduler.select(false) && first.scheduler.current().cpu.x[0] == 0,
            "first VM failed to accept its range lock");
    // Both events can arrive before either coordinator runs; draining one must preserve the other.
    for (auto *machine : {&first, &second}) {
        auto &entry = entries[machine == &first ? 0 : 1];
        GuestLock unlock{2, SEEK_SET, 0, 0, 0};
        machine->memory.copy_in(0x10000, &unlock, sizeof(unlock));
        require(files::record_lock(machine->memory, machine->filesystem, machine->scheduler, entry, 37, 0x10000) == 0,
                "release guest lock ranges");
    }
    native(0, true);
    native(16, true);
    block(first, entries[0], 0);
    block(second, entries[1], 16);
    left = sources(first.scheduler, timeout);
    right = sources(second.scheduler, timeout);
    native(0, false);
    native(16, false);
    require(::poll(left.data(), left.size(), 2000) == 1 && ::poll(right.data(), right.size(), 2000) == 1,
            "simultaneous range releases lost a notification");
    require(first.scheduler.select(false), "first simultaneous range wait stalled");
    right = sources(second.scheduler, timeout);
    require(::poll(right.data(), right.size(), 0) == 1 && second.scheduler.select(false),
            "one VM consumed another VM's retained completion");
}
} // namespace
// Exercise idle waits, retained producer readiness, deadline expiry and descriptor retirement.
int main() {
    char root[] = "/tmp/operit-wait-sources-XXXXXX";
    try {
        require(::mkdtemp(root), "create VM root");
        auto pipe = timers::ready_pipe();
        Scheduler scheduler;
        scheduler.block({}, 0, [&]() -> std::optional<int64_t> {
            char byte;
            if (::read(pipe.first.get(), &byte, 1) == 1) return 7;
            return {};
        }, {{pipe.first.get(), POLLIN, 0}});
        int64_t timeout = 0;
        auto descriptors = sources(scheduler, timeout);
        require(timeout == -1 && !scheduler.poll_waiting(), "idle VM invented a periodic deadline");
        require(::poll(descriptors.data(), descriptors.size(), 0) == 0, "idle VM invented readiness");
        std::thread producer([&] { const char byte = 1; ::write(pipe.second.get(), &byte, 1); });
        const auto ready = ::poll(descriptors.data(), descriptors.size(), 2000);
        producer.join();
        require(ready == 1, "producer did not wake idle VM");
        require(scheduler.select(false) && scheduler.current().cpu.x[0] == 7, "ready task did not resume");
        const auto deadline = TaskClock::now() + std::chrono::milliseconds(40);
        scheduler.block(deadline, 23);
        descriptors = sources(scheduler, timeout);
        require(timeout > 0 && timeout <= 40000000, "guest deadline used the wrong host clock origin");
        const auto start = TaskClock::now();
        require(::poll(descriptors.data(), descriptors.size(), int((timeout + 999999) / 1000000)) == 0,
                "timer-only VM wait was interrupted by a periodic producer");
        require(TaskClock::now() >= deadline, "deadline expired early");
        require(scheduler.select(false) && scheduler.current().cpu.x[0] == 23, "deadline did not resume guest");
        sources(scheduler, timeout);
        require(timeout == -1 && scheduler.wait_descriptors().empty(),
                "completed wait retained a stale descriptor or timer");
        require(TaskClock::now() - start < std::chrono::seconds(2), "deadline wake stalled");
        timespec wall{};
        require(::clock_gettime(CLOCK_REALTIME, &wall) == 0, "sample native realtime clock");
        const auto nanos = wall.tv_nsec + 40000000;
        std::vector<pollfd> realtime_sources;
        auto realtime_poll = threads::futex_realtime_timeout(
            threads::FutexPiTimeout{wall.tv_sec + nanos / 1000000000, nanos % 1000000000, true}, realtime_sources);
        scheduler.block({}, -110, std::move(realtime_poll), std::move(realtime_sources));
        descriptors = sources(scheduler, timeout);
        require(timeout == -1 && descriptors.size() == 1, "realtime wait lacks a native timer producer");
        require(::poll(descriptors.data(), descriptors.size(), 2000) == 1 && scheduler.select(false) &&
                int64_t(scheduler.current().cpu.x[0]) == -110, "realtime futex deadline did not notify");
        disjoint_lock_waits(root);
        std::filesystem::remove_all(root);
        std::puts("VM waits: indefinite idle, producer notification, accurate deadline, source retirement and independent multi-VM range wakes passed");
        return 0;
    } catch (const std::exception &error) {
        std::filesystem::remove_all(root);
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
}
