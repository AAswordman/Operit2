#include "fs/virtual/procfs.h"
#include "linux/error_mapping.h"
#include "linux/network/resolver.h"
#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <fcntl.h>
#include <iomanip>
#include <sstream>

namespace operit::arm64 {
namespace {
// AArch64 AT_CLKTCK and the guest times() ABI both publish USER_HZ == 100.
constexpr uint64_t proc_nanoseconds_per_tick = 1000000000ull / 100;
std::string process_stat_contents(Scheduler &scheduler, Memory &memory,
                                  const process::MemoryBounds &bounds, uint32_t pid,
                                  uint32_t parent, uint32_t tid, bool whole_group) {
    auto *found = scheduler.lookup(tid);
    if (!found || found->pid != pid)
        guest_error(2);
    auto *controller = scheduler.processes();
    if (!controller)
        guest_error(38);
    const auto usage = controller->proc_stat_accounting(pid, tid, whole_group);
    const auto &task = *found;
    uint64_t mapped = 0;
    for (const auto &region : memory.mappings())
        mapped += region.end - region.start;
    std::ostringstream output;
    std::array<uint64_t, 53> fields{};
    fields[4] = parent;
    fields[5] = controller->process_group(pid);
    fields[6] = controller->session(pid);
    fields[8] = uint64_t(-1);
    fields[10] = usage.self.minor_faults;
    fields[11] = usage.children.minor_faults;
    fields[12] = usage.self.major_faults;
    fields[13] = usage.children.major_faults;
    fields[14] = usage.self.cpu.user_nanoseconds / proc_nanoseconds_per_tick;
    fields[15] = usage.self.cpu.system_nanoseconds / proc_nanoseconds_per_tick;
    fields[16] = usage.children.cpu.user_nanoseconds / proc_nanoseconds_per_tick;
    fields[17] = usage.children.cpu.system_nanoseconds / proc_nanoseconds_per_tick;
    fields[18] = 20;
    fields[20] = std::max<size_t>(1, scheduler.live_tids(pid).size());
    fields[22] = usage.start_boottime_nanoseconds / proc_nanoseconds_per_tick;
    fields[23] = mapped;
    fields[24] = memory.resident() / page_size;
    fields[25] = UINT64_MAX;
    fields[26] = bounds.start_code;
    fields[27] = bounds.end_code;
    fields[28] = bounds.start_stack;
    fields[31] = task.signals.pending.mask() & 0x7fffffff;
    fields[32] = task.signal_mask & 0x7fffffff;
    fields[39] = task.last_cpu;
    fields[45] = bounds.start_data;
    fields[46] = bounds.end_data;
    fields[47] = bounds.start_brk;
    fields[48] = bounds.arg_start;
    fields[49] = bounds.arg_end;
    fields[50] = bounds.env_start;
    fields[51] = bounds.env_end;
    const char state = task.state == Task::State::Exited ? 'Z'
                       : task.stopped ? 'T'
                       : task.state == Task::State::Waiting ? 'S' : 'R';
    output << tid << " (" << task.name << ") " << state;
    for (size_t field = 4; field < fields.size(); ++field) {
        if (field == 8)
            output << " -1";
        else
            output << ' ' << fields[field];
    }
    output << '\n';
    return output.str();
}
} // namespace
void Syscalls::set_process_image(const std::string &path, uint64_t stack) {
    try {
        filesystem_.resolve("/etc/resolv.conf").stat();
    } catch (const GuestError &error) {
        if (error.number != 2)
            throw;
        auto contents = host_resolver_configuration();
        if (!contents.empty()) {
            auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::File, "/etc/resolv.conf", 0444);
            node->contents = [] { return host_resolver_configuration(); };
            filesystem_.mount_virtual("/etc/resolv.conf", std::move(node));
        }
    }
    executable_ = filesystem_.resolve(path).guest;
    scheduler_.current().name = executable_.substr(executable_.find_last_of('/') + 1, 15);
    initialize_process_memory(stack);
}
std::string Syscalls::process_contents(uint32_t tid, const std::string &name) {
    if (name == "mounts")
        return filesystem_.mount_table();
    if (name == "cmdline" || name == "environ" || name == "auxv")
        return process_memory_contents(name);
    if (name == "stat")
        return process_stat_contents(scheduler_, memory_, process_memory_->bounds,
                                     process_anchor_->pid, process_anchor_->parent, tid, true);
    std::ostringstream output;
    const auto &bounds = process_memory_->bounds;
    const uint64_t executable = bounds.end_code > bounds.start_code
                                    ? ((bounds.end_code + page_size - 1) & ~(page_size - 1)) -
                                          align_down(bounds.start_code)
                                    : 0;
    uint64_t mapped = 0, writable = 0;
    auto mappings = memory_.mappings();
    for (const auto &region : mappings) {
        mapped += region.end - region.start;
        if (region.permissions & write_perm)
            writable += region.end - region.start;
    }
    if (name == "meminfo") {
        output << "MemTotal: " << memory_.limit() / 1024
               << " kB\nMemFree: " << (memory_.limit() - memory_.resident()) / 1024
               << " kB\nMemAvailable: " << (memory_.limit() - memory_.resident()) / 1024 << " kB\n";
    } else if (name == "maps") {
        for (const auto &region : mappings) {
            const auto &b = region.backing;
            output << std::hex << std::setfill('0') << std::setw(12) << region.start << '-' << std::setw(12)
                   << region.end << ' ' << ((region.permissions & read_perm) ? 'r' : '-')
                   << ((region.permissions & write_perm) ? 'w' : '-')
                   << ((region.permissions & exec_perm) ? 'x' : '-') << (b.shared ? 's' : 'p') << ' '
                   << std::setw(8) << b.offset << ' ' << std::setw(2) << ((b.device >> 8) & 0xfff) << ':'
                   << std::setw(2) << (b.device & 0xff) << ' ' << std::dec << b.inode;
            auto label = b.path;
            if (label == "[heap]" || label == "[stack]")
                label.clear();
            if (label.empty()) {
                if (region.start <= bounds.brk && region.end >= bounds.start_brk)
                    label = "[heap]";
                else if (region.start <= bounds.start_stack && region.end >= bounds.start_stack)
                    label = "[stack]";
            }
            if (!label.empty()) {
                output << "  ";
                for (char c : label) {
                    if (c == '\n')
                        output << "\\012";
                    else
                        output << c;
                }
            }
            output << '\n';
        }
    } else {
        auto *found = scheduler_.lookup(tid);
        if (!found || found->pid != process_anchor_->pid)
            guest_error(2);
        auto &task = *found;
        if (name == "comm")
            return task.name + "\n";
        if (name == "statm") {
            // These page counts come from this VM's reservations and materialized pages.
            output << mapped / page_size << ' ' << memory_.resident() / page_size << " 0 "
                   << executable / page_size << " 0 " << writable / page_size << " 0\n";
        } else if (name == "status") {
            std::string comm;
            for (char c : task.name) {
                if (c == '\n')
                    comm += "\\n";
                else if (c == '\\')
                    comm += "\\\\";
                else
                    comm += c;
            }
            const auto cpu_count = std::min<uint32_t>(scheduler_.execution_cpus(), 64);
            const uint64_t available = cpu_count == 64 ? UINT64_MAX : (uint64_t(1) << cpu_count) - 1;
            const uint64_t affinity = task.cpu_affinity & available;
            std::ostringstream allowed_mask;
            allowed_mask << std::hex << std::setfill('0');
            if (cpu_count > 32)
                allowed_mask << std::setw(8) << uint32_t(affinity >> 32) << ',';
            allowed_mask << std::setw(8) << uint32_t(affinity);
            std::string allowed_list;
            for (uint32_t cpu = 0; cpu < cpu_count; ++cpu) {
                if (!(affinity & (uint64_t(1) << cpu)))
                    continue;
                const auto first = cpu;
                while (cpu + 1 < cpu_count && (affinity & (uint64_t(1) << (cpu + 1))))
                    ++cpu;
                if (!allowed_list.empty())
                    allowed_list += ',';
                allowed_list += std::to_string(first);
                if (cpu != first)
                    allowed_list += '-' + std::to_string(cpu);
            }
            output << "Name:\t" << comm << "\nState:\t"
                   << (task.state == Task::State::Exited ? "Z (zombie)"
                       : task.stopped ? "T (stopped)"
                       : task.state == Task::State::Waiting ? "S (sleeping)" : "R (running)")
                   << "\nTgid:\t" << process_anchor_->pid << "\nPid:\t" << tid
                   << "\nPPid:\t" << process_anchor_->parent << "\nTracerPid:\t0\nUid:\t"
                   << ::getuid() << '\t'
                   << ::geteuid() << '\t' << ::geteuid() << '\t' << ::geteuid() << "\nGid:\t" << ::getgid()
                   << '\t' << ::getegid() << '\t' << ::getegid() << '\t' << ::getegid()
                   << "\nFDSize:\t1024\nVmSize:\t" << mapped / 1024 << " kB\nVmRSS:\t"
                   << memory_.resident() / 1024 << " kB\nVmData:\t" << writable / 1024 << " kB\nVmExe:\t"
                   << executable / 1024 << " kB\nThreads:\t" << std::max<size_t>(1, scheduler_.live_tids(process_anchor_->pid).size()) << "\nSigBlk:\t"
                   << std::hex << std::setw(16) << std::setfill('0') << task.signal_mask
                   << "\nCpus_allowed:\t" << allowed_mask.str()
                   << "\nCpus_allowed_list:\t" << allowed_list << '\n';
        } else
            guest_error(2);
    }
    return output.str();
}
VirtualNode::Ptr Syscalls::process_fd_link(int number) {
    auto &entry = descriptors_.get(number);
    auto access = process_access();
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::Symlink,
                                              "/proc/" + std::to_string(process_anchor_->pid) + "/fd/" + std::to_string(number), 0700);
    node->target = [access, number] {
        auto &fd = access().descriptors_.get(number);
        if (process::pidfd_file(fd.description))
            return std::string("anon_inode:[pidfd]");
        if (fd.description->inotify)
            return std::string("anon_inode:inotify");
        if (ipc::is_posix_mq_file(fd.description))
            return fd.description->virtual_file->node->identity;
        auto display = fd.display_path();
        if (!display.empty())
            return display;
        struct stat info{};
        if (fd.host.get() >= 0)
            check_host(::fstat(fd.host.get(), &info));
        std::string type = fd.description->poll      ? "anon_inode:[eventpoll]"
                           : fd.description->timer   ? "anon_inode:[timerfd]"
                           : fd.description->counter ? "anon_inode:[eventfd]"
                           : S_ISFIFO(info.st_mode)  ? "pipe:[" + std::to_string(info.st_ino) + "]"
                           : S_ISSOCK(info.st_mode)  ? "socket:[" + std::to_string(info.st_ino) + "]"
                                                     : "hostfd:[" + std::to_string(info.st_ino) + "]";
        return type;
    };
    // Named files are followed through the guest namespace. Anonymous streams
    // retain their underlying endpoint and callback identity when reopened.
    if (entry.path.empty()) {
        node->standard_stream = entry.description->standard_stream;
        node->open_stream = [access, number](int flags) {
            auto &fd = access().descriptors_.get(number);
            if (ipc::is_posix_mq_file(fd.description))
                guest_error(95);
            if (process::pidfd_file(fd.description) || fd.description->poll ||
                fd.description->counter || fd.description->timer || fd.description->inotify)
                guest_error(6);
            if ((flags & O_ACCMODE) != (open_flags(fd.description->flags) & O_ACCMODE))
                guest_error(13);
            int result = fd.host.get() < 0 ? -1 : ::fcntl(fd.host.get(), F_DUPFD_CLOEXEC, 0);
            if (fd.host.get() >= 0)
                check_host(result);
            return FD(result);
        };
        node->stream_stat = [access, number] { auto &context = access(); return context.descriptor_stat(context.descriptors_.get(number)); };
    }
    return node;
}
std::function<Syscalls &()> Syscalls::process_access(uint32_t pid) {
    auto *controller = scheduler_.processes();
    return [controller, anchor = process_anchor_, pid]() -> Syscalls & {
        const uint32_t target = pid ? pid : anchor->pid;
        if (controller) {
            auto *context = controller->process_syscalls(target);
            if (!context)
                guest_error(2);
            return *context;
        }
        if (!anchor->syscalls || target != anchor->pid)
            guest_error(2);
        return *anchor->syscalls;
    };
}
ProcSource Syscalls::process_source(uint32_t pid) {
    auto access = process_access(pid);
    ProcSource source;
    source.processes = [access] {
        auto &context = access();
        auto *controller = context.scheduler_.processes();
        return controller ? controller->process_ids() : std::vector<uint32_t>{context.process_anchor_->pid};
    };
    source.current_pid = [access] { return access().process_anchor_->pid; };
    source.current_tid = [access] { return access().scheduler_.current().tid; };
    source.process = [access](uint32_t target) { return access().process_source(target); };
    source.tasks = [access] {
        auto &context = access();
        auto tasks = context.scheduler_.live_tids(context.process_anchor_->pid);
        if (std::find(tasks.begin(), tasks.end(), context.process_anchor_->pid) == tasks.end())
            tasks.insert(tasks.begin(), context.process_anchor_->pid);
        return tasks;
    };
    source.descriptors = [access] { return access().descriptors_.numbers(); };
    source.stat_contents = [access](uint32_t tid, bool whole_group) {
        auto &context = access();
        return process_stat_contents(context.scheduler_, context.memory_, context.process_memory_->bounds,
                                     context.process_anchor_->pid, context.process_anchor_->parent,
                                     tid, whole_group);
    };
    source.contents = [access](uint32_t tid, const std::string &name) {
        return access().process_contents(tid, name);
    };
    source.read_range = [access](uint32_t tid, const std::string &name, uint64_t offset, size_t count) {
        auto &context = access();
        if (name == "cmdline" || name == "environ" || name == "auxv")
            return context.process_memory_contents(name, offset, count);
        auto contents = context.process_contents(tid, name);
        return offset >= contents.size() ? std::string{} : contents.substr(size_t(offset), count);
    };
    source.link = [access](uint32_t, const std::string &name) {
        auto &context = access();
        return name == "exe" ? context.executable_ : name == "cwd" ? context.fs_context_->cwd : "/";
    };
    source.set_name = [access](uint32_t tid, const std::string &value) {
        auto &context = access();
        auto &task = context.scheduler_.find(tid);
        if (task.pid != context.process_anchor_->pid)
            guest_error(2);
        task.name = value.substr(0, std::min<size_t>(15, value.find('\0')));
        return value.size();
    };
    source.descriptor_link = [access](int fd) { return access().process_fd_link(fd); };
    source.descriptor_info = [access](int fd) {
        auto &context = access();
        auto &entry = context.descriptors_.get(fd);
        auto d = entry.description;
        off_t position = d->virtual_file        ? d->virtual_file->offset
                         : entry.host.get() < 0 ? 0
                                                : ::lseek(entry.host.get(), 0, SEEK_CUR);
        if (position < 0)
            position = 0;
        std::ostringstream text;
        text << "pos:\t" << position << "\nflags:\t0" << std::oct
             << (d->flags | (entry.descriptor_flags ? 524288 : 0)) << "\nino:\t" << std::dec
             << context.descriptor_stat(entry).st_ino << '\n';
        if (auto pidfd = process::pidfd_file(d)) {
            // Exit preserves the guest PID until reaping publishes POLLHUP.
            constexpr uint32_t poll_hangup = 0x10;
            const int64_t pid = (pidfd->poll_events(0) & poll_hangup) ? -1 : int64_t(pidfd->guest_pid());
            text << "Pid:\t" << pid << "\nNSpid:\t" << pid << '\n';
        }
        return text.str();
    };
    return source;
}
void Syscalls::mount_process_filesystems() {
    filesystem_.mount_virtual("/dev", device_filesystem(terminals_));
    filesystem_.mount_virtual("/proc", process_filesystem(process_source()));
}
} // namespace operit::arm64
