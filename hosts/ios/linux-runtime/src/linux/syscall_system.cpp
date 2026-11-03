#include "base/random.h"
#include "linux/abi.h"
#include "linux/error_mapping.h"
#include "linux/syscalls.h"
#include "linux/system/clocks.h"
#include "runtime/process/controller.h"
#include <cerrno>
#include <climits>
#include <cstring>
#include <fcntl.h>
#include <sched.h>
#include <sys/time.h>
#include <time.h>

namespace operit::arm64 {
namespace {
std::string guest_hostname(Filesystem &filesystem) {
    try {
        auto file = filesystem.open("/etc/hostname", "/", O_RDONLY | O_CLOEXEC | O_NONBLOCK, 0);
        struct stat info{};
        if (::fstat(file.get(), &info) != 0 || !S_ISREG(info.st_mode))
            return "(none)";
        char buffer[65]{};
        ssize_t count;
        do {
            count = ::pread(file.get(), buffer, sizeof(buffer) - 1, 0);
        } while (count < 0 && errno == EINTR);
        if (count <= 0)
            return "(none)";
        std::string hostname(buffer, static_cast<size_t>(count));
        const auto first = hostname.find_first_not_of(" \t\r\n");
        if (first == std::string::npos)
            return "(none)";
        const auto end = hostname.find_first_of(" \t\r\n", first);
        hostname = hostname.substr(first, end == std::string::npos ? end : end - first);
        if (hostname.find('\0') != std::string::npos)
            return "(none)";
        return hostname;
    } catch (const GuestError &) {
        // A missing or inaccessible guest hostname must not make uname fail.
        return "(none)";
    }
}
} // namespace

int64_t Syscalls::dispatch_system(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (a[8]) {
    case 90:
    case 91: { // capget/capset: guest processes have no host kernel capabilities.
        check_buffer(a[0], 8, read_perm);
        const uint32_t version = uint32_t(memory_.load(a[0], 4));
        unsigned words;
        if (version == 0x19980330)
            words = 1;
        else if (version == 0x20071026 || version == 0x20080522)
            words = 2;
        else {
            check_buffer(a[0], 4, write_perm);
            memory_.store(a[0], 0x20080522, 4);
            guest_error(22);
        }
        const int32_t tid = int32_t(memory_.load(a[0] + 4, 4));
        if (tid < 0)
            guest_error(22);
        const auto &target = scheduler_.find(tid ? uint32_t(tid) : scheduler_.current().tid);
        if (a[8] == 91 && target.tid != scheduler_.current().tid)
            guest_error(1);
        check_buffer(a[1], words * 12, a[8] == 90 ? write_perm : read_perm);
        for (unsigned word = 0; word < words * 3; ++word) {
            if (a[8] == 90)
                memory_.store(a[1] + word * 4, 0, 4);
            else if (memory_.load(a[1] + word * 4, 4))
                guest_error(1);
        }
        return 0;
    }
    case 153:
    case 165:
        if (auto *processes = scheduler_.processes())
            return system::dispatch_clocks(cpu, memory_, *processes);
        guest_error(3);
    case 168:
        return system::getcpu(memory_, a[0], a[1],
                              {scheduler_.current().last_cpu, scheduler_.execution_cpus()}, a[2]);
    case 179: {
        std::vector<const Memory *> spaces;
        if (auto *processes = scheduler_.processes()) {
            for (const auto pid : processes->process_ids())
                if (auto *syscalls = processes->process_syscalls(pid))
                    spaces.push_back(&syscalls->memory_);
        } else {
            spaces.push_back(&memory_);
        }
        const auto snapshot = system::snapshot_information(*information_,
            system::memory_information(spaces), system::task_information(scheduler_));
        return system::sysinfo(memory_, a[0], snapshot);
    }
    case 113:
    case 114: { // clock_gettime
        if (auto *processes = scheduler_.processes())
            return system::dispatch_clocks(cpu, memory_, *processes);
        clockid_t id;
        if (a[0] == 0)
            id = CLOCK_REALTIME;
        else if (a[0] == 1 || a[0] == 4 || a[0] == 6 || a[0] == 7)
            id = CLOCK_MONOTONIC;
        else
            guest_error(22);
        check_buffer(a[1], 16, write_perm);
        struct timespec host{};
        check_host(a[8] == 113 ? ::clock_gettime(id, &host) : ::clock_getres(id, &host));
        linux_abi::Timespec value{host.tv_sec, host.tv_nsec};
        memory_.copy_in(a[1], &value, sizeof(value));
        return 0;
    }
    case 158: { // getgroups: permissions follow the actual backing-file identity.
        const int capacity = int32_t(a[0]);
        if (capacity < 0)
            guest_error(22);
        for (;;) {
            const int count = ::getgroups(0, nullptr);
            check_host(count);
            if (!capacity)
                return count;
            if (capacity < count)
                guest_error(22);
            std::vector<gid_t> groups(count);
            const int actual = ::getgroups(count, groups.data());
            if (actual < 0 && errno == EINVAL)
                continue; // The host group list changed between the two calls.
            check_host(actual);
            check_buffer(a[1], uint64_t(actual) * 4, write_perm);
            for (int index = 0; index < actual; ++index)
                memory_.store(a[1] + uint64_t(index) * 4, uint32_t(groups[index]), 4);
            return actual;
        }
    }
    case 159: // setgroups requires CAP_SETGID, absent from the guest capability set.
        guest_error(1);
    case 160: { // uname
        check_buffer(a[0], 65 * 6, write_perm);
        char value[65 * 6]{};
        // Linux/aarch64 describe the guest ABI. No Linux kernel is booted, so
        // do not invent a kernel release or substitute the Darwin host version.
        const std::string fields[] = {"Linux", guest_hostname(filesystem_), "unknown", "unknown",
                                      "aarch64", ""};
        for (unsigned j = 0; j < 6; ++j)
            std::strncpy(value + j * 65, fields[j].c_str(), 64);
        memory_.copy_in(a[0], value, sizeof(value));
        return 0;
    }
    case 166: {
        uint32_t previous = fs_context_->mask;
        fs_context_->mask = uint32_t(a[0]) & 0777;
        return previous;
    }
    case 169: { // gettimeofday
        if (a[1]) {
            check_buffer(a[1], 8, write_perm);
            memory_.store(a[1], 0, 8);
        }
        if (a[0]) {
            check_buffer(a[0], 16, write_perm);
            struct timeval now{};
            check_host(::gettimeofday(&now, nullptr));
            memory_.store(a[0], now.tv_sec, 8);
            memory_.store(a[0] + 8, now.tv_usec, 8);
        }
        return 0;
    }
    case 278: { // getrandom
        if (a[2] & ~uint64_t{3})
            guest_error(22);
        size_t count = std::min<uint64_t>(a[1], max_io);
        check_buffer(a[0], count, write_perm);
        std::vector<uint8_t> data(count);
        random_bytes(data.data(), count);
        memory_.copy_in(a[0], data.data(), count);
        return count;
    }
    default:
        break;
    }
    return unsupported(cpu);
}
} // namespace operit::arm64
