#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <array>
#include <climits>
#include <cstring>

namespace operit::arm64 {
namespace {
constexpr uint32_t wait_nohang = 1, wait_untraced = 2, wait_exited = 4, wait_continued = 8;
constexpr uint32_t wait_nowait = 0x1000000, wait_nothread = 0x20000000;
constexpr uint32_t wait_all = 0x40000000, wait_clone = 0x80000000;
constexpr uint64_t maximum_argument_bytes = 1 << 20;
std::vector<std::string> read_strings(Memory &memory, uint64_t address, size_t &bytes) {
    std::vector<std::string> strings;
    if (!address)
        return strings;
    for (size_t j = 0;; ++j) {
        if (j == 4096)
            guest_error(7);
        if (address > address_limit - 8 || !memory.accessible(address, 8, read_perm))
            guest_error(14);
        auto pointer = memory.load(address, 8);
        address += 8;
        if (!pointer)
            break;
        std::string value;
        for (size_t k = 0;; ++k) {
            if (k >= 32 * page_size || bytes >= maximum_argument_bytes)
                guest_error(7);
            if (pointer >= address_limit || !memory.accessible(pointer, 1, read_perm))
                guest_error(14);
            char ch = char(memory.load(pointer++, 1));
            ++bytes;
            if (!ch)
                break;
            value += ch;
        }
        strings.push_back(std::move(value));
    }
    return strings;
}
void store_usage(Memory &memory, uint64_t address, const ProcessWaitResult &result) {
    if (!address)
        return;
    std::array<uint64_t, 18> usage{};
    usage[0] = result.user_microseconds / 1000000;
    usage[1] = result.user_microseconds % 1000000;
    usage[2] = result.system_microseconds / 1000000;
    usage[3] = result.system_microseconds % 1000000;
    usage[4] = result.maximum_rss;
    usage[8] = result.minor_faults;
    usage[9] = result.major_faults;
    usage[16] = result.voluntary_switches;
    usage[17] = result.involuntary_switches;
    memory.copy_in(address, usage.data(), sizeof(usage));
}
void store_wait_info(Memory &memory, uint64_t address, const ProcessWaitResult &result) {
    std::array<uint8_t, 128> bytes{};
    int32_t signal = 17, code, status;
    if (result.status == 0xffff) {
        code = 6;
        status = 18;
    } else if ((result.status & 255) == 0x7f) {
        code = result.trace_stop ? 4 : 5;
        status = (result.status >> 8) & 255;
    } else if (result.status & 127) {
        code = (result.status & 128) ? 3 : 2;
        status = result.status & 127;
    } else {
        code = 1;
        status = (result.status >> 8) & 255;
    }
    std::memcpy(bytes.data(), &signal, 4);
    std::memcpy(bytes.data() + 8, &code, 4);
    std::memcpy(bytes.data() + 16, &result.pid, 4);
    std::memcpy(bytes.data() + 20, &result.uid, 4);
    std::memcpy(bytes.data() + 24, &status, 4);
    uint64_t user = result.user_microseconds / 10000, system = result.system_microseconds / 10000;
    std::memcpy(bytes.data() + 32, &user, 8);
    std::memcpy(bytes.data() + 40, &system, 8);
    memory.copy_in(address, bytes.data(), bytes.size());
}
} // namespace
int64_t Syscalls::dispatch_process(CpuState &cpu) {
    auto *processes = scheduler_.processes();
    if (!processes)
        return unsupported(cpu);
    const auto arguments = cpu.x;
    const auto &a = arguments;
    switch (a[8]) {
    case 117: // ptrace
        return processes->ptrace_request(cpu);
    case 434: // pidfd_open: guest process identity, never a host PID.
        return process::pidfd_open(*processes, descriptors_, int32_t(a[0]), uint32_t(a[1]));
    case 424: { // pidfd_send_signal(pidfd, signal, siginfo, flags)
        if (uint32_t(a[3]) || int32_t(a[1]) < 0 || int32_t(a[1]) > int(signals::maximum_signal))
            guest_error(22);
        auto file = process::require_pidfd(descriptors_.get(uint32_t(a[0])));
        std::optional<signals::Info> info;
        if (a[2]) {
            check_buffer(a[2], sizeof(signals::Info), read_perm);
            info.emplace();
            memory_.copy_out(a[2], &*info, sizeof(*info));
        }
        return file->send_signal(*processes, scheduler_.current().pid, int32_t(a[1]), info,
                                 uint32_t(a[3]));
    }
    case 438: { // pidfd_getfd: PTRACE_MODE_ATTACH_REALCREDS, then import the guest FD.
        if (uint32_t(a[2]))
            guest_error(22);
        auto file = process::require_pidfd(descriptors_.get(uint32_t(a[0])));
        file->check_getfd_access(*processes, scheduler_.current().pid);
        auto *target = processes->process_syscalls(file->guest_pid());
        if (!target)
            guest_error(3);
        return process::pidfd_copy_descriptor(descriptors_, target->descriptors_.get(uint32_t(a[1])));
    }
    case 172:
        return scheduler_.current().pid;
    case 173:
        return processes->parent_pid(scheduler_.current().pid);
    case 154: // setpgid
        if (int32_t(a[0]) < 0 || int32_t(a[1]) < 0)
            guest_error(22);
        processes->set_process_group(uint32_t(a[0]), uint32_t(a[1]));
        return 0;
    case 155: // getpgid
        return processes->process_group(uint32_t(a[0]));
    case 156: // getsid
        return processes->session(uint32_t(a[0]));
    case 157: // setsid
        return processes->create_session();
    case 220: { // AArch64 clone(flags, child_stack, parent_tid, tls, child_tid)
        ProcessClone request;
        request.flags = a[0] & ~uint64_t{255};
        request.exit_signal = a[0] & 255;
        request.stack = a[1];
        request.parent_tid = a[2];
        request.tls = a[3];
        request.child_tid = a[4];
        return processes->clone(cpu, request);
    }
    case 435: { // clone3: versioned Linux struct clone_args, size >= VER0.
        if (a[1] < 64)
            guest_error(22);
        if (a[1] > 4096)
            guest_error(7);
        check_buffer(a[0], a[1], read_perm);
        std::array<uint64_t, 11> fields{};
        const auto known = std::min<uint64_t>(sizeof(fields), a[1]);
        memory_.copy_out(a[0], fields.data(), known);
        for (uint64_t j = known; j < a[1]; ++j)
            if (memory_.load(a[0] + j, 1))
                guest_error(7);
        if (fields[0] & 255)
            guest_error(22);
        if (fields[8] || fields[9])
            guest_error(1); // Restoring explicit TIDs requires CAP_CHECKPOINT_RESTORE.
        if (fields[10])
            guest_error(95);
        if (fields[4] > 64)
            guest_error(22);
        if ((!fields[5]) != (!fields[6]))
            guest_error(22);
        if (fields[6] > address_limit || fields[5] > address_limit - fields[6])
            guest_error(14);
        ProcessClone request;
        request.flags = fields[0];
        request.parent_tid = fields[3];
        request.child_tid = fields[2];
        request.exit_signal = uint32_t(fields[4]);
        request.stack = fields[5] + fields[6];
        request.tls = fields[7];
        return processes->clone(cpu, request);
    }
    case 221:   // execve
    case 281: { // execveat
        std::string executable;
        uint64_t argv, envp;
        if (a[8] == 221) {
            executable = filesystem_.resolve(memory_.string(a[0]), fs_context_->cwd).guest;
            argv = a[1];
            envp = a[2];
        } else {
            constexpr uint64_t at_empty_path = 0x1000, at_symlink_nofollow = 0x100;
            if (a[4] & ~(at_empty_path | at_symlink_nofollow))
                guest_error(22);
            auto name = memory_.string(a[1]);
            if (name.empty()) {
                if (!(a[4] & at_empty_path))
                    guest_error(2);
                const auto &descriptor = descriptors_.get(a[0]);
                if (descriptor.path.empty())
                    guest_error(13);
                executable = filesystem_.resolve(descriptor.path).guest;
            } else {
                auto path = filesystem_.resolve(name, directory_base(a[0]), !(a[4] & at_symlink_nofollow));
                if ((a[4] & at_symlink_nofollow) && S_ISLNK(path.stat().st_mode))
                    guest_error(40);
                executable = path.guest;
            }
            argv = a[2];
            envp = a[3];
        }
        size_t bytes = 0;
        auto args = read_strings(memory_, argv, bytes);
        auto environment = read_strings(memory_, envp, bytes);
        processes->exec(executable, args, environment, fs_context_->cwd);
        return 0;
    }
    case 95:    // waitid(idtype, id, siginfo, options, rusage)
    case 260: { // wait4(pid, status, options, rusage)
        const bool waitid = a[8] == 95;
        const uint64_t output = waitid ? a[2] : a[1];
        const uint64_t usage = waitid ? a[4] : a[3];
        uint32_t options = uint32_t(waitid ? a[3] : a[2]);
        uint32_t allowed =
            wait_nohang | wait_untraced | wait_continued | wait_nothread | wait_all | wait_clone;
        if (waitid)
            allowed |= wait_exited | wait_nowait;
        if (options & ~allowed || (waitid && !(options & (wait_untraced | wait_exited | wait_continued))))
            guest_error(22);
        if (output)
            check_buffer(output, waitid ? 128 : 4, write_perm);
        if (usage)
            check_buffer(usage, 144, write_perm);
        ProcessWait request;
        request.options = waitid ? options : options | wait_exited;
        request.caller_tid = scheduler_.current().tid;
        std::shared_ptr<process::PidfdFile> pidfd;
        bool pidfd_nonblocking = false;
        if (waitid) {
            if (a[0] > 3)
                guest_error(22);
            if (a[0] == 3) { // P_PIDFD: capture the open file, not the numeric FD.
                const auto &entry = descriptors_.get(uint32_t(a[1]));
                pidfd = process::require_pidfd(entry);
                pidfd->validate(*processes, 10);
                pidfd_nonblocking = entry.description->flags & process::pidfd_nonblock;
                request.kind = ProcessWait::Kind::Pid;
                request.id = pidfd->guest_pid();
            } else {
                request.kind = a[0] == 0   ? ProcessWait::Kind::All
                               : a[0] == 1 ? ProcessWait::Kind::Pid
                                           : ProcessWait::Kind::Group;
                request.id = uint32_t(a[1]);
            }
            if (request.kind == ProcessWait::Kind::Pid && !request.id)
                guest_error(22);
            if (request.kind == ProcessWait::Kind::Group && !request.id)
                request.id = processes->process_group(scheduler_.current().pid);
        } else {
            const int64_t pid = int32_t(a[0]);
            if (pid > 0) {
                request.kind = ProcessWait::Kind::Pid;
                request.id = uint32_t(pid);
            } else if (pid == 0 || pid < -1) {
                request.kind = ProcessWait::Kind::Group;
                request.id = pid == 0 ? processes->process_group(scheduler_.current().pid) : uint32_t(-pid);
            }
        }
        auto attempt = [this, processes, request, output, usage, waitid,
                        pidfd]() -> std::optional<int64_t> {
            if (output)
                check_buffer(output, waitid ? 128 : 4, write_perm);
            if (usage)
                check_buffer(usage, 144, write_perm);
            auto result = pidfd ? pidfd->wait(*processes, request) : processes->wait(request);
            if (!result)
                return {};
            if (result->process_reaped)
                process::pidfd_process_reaped(*processes, result->pid);
            if (output) {
                if (waitid)
                    store_wait_info(memory_, output, *result);
                else
                    memory_.store(output, uint32_t(result->status), 4);
            }
            store_usage(memory_, usage, *result);
            return waitid ? 0 : result->pid;
        };
        if (auto result = attempt())
            return *result;
        if (options & wait_nohang) {
            if (waitid && output) {
                std::array<uint8_t, 128> empty{};
                memory_.copy_in(output, empty.data(), empty.size());
            }
            return 0;
        }
        if (pidfd_nonblocking)
            return -11;
        if (interrupt_for_pending_signal())
            return -4;
        scheduler_.block({}, 0, std::move(attempt));
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
