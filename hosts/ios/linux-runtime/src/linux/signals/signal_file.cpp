#include "linux/signals/signal_file.h"
#include "base/host_io.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "memory/fault.h"
#include <algorithm>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <poll.h>
#include <set>

namespace operit::arm64::signals {
namespace {
constexpr uint32_t nonblock = 2048, cloexec = 524288;
struct Registry {
    std::mutex mutex;
    std::map<const OpenDescription *, std::weak_ptr<SignalFile>> entries;
};
Registry &registry() {
    static Registry value;
    return value;
}
template <typename T> T payload(const Info &info, size_t offset) {
    T value;
    std::memcpy(&value, info.payload.data() + offset, sizeof(value));
    return value;
}
enum class InfoLayout { Kill, Timer, Poll, Fault, Child, Realtime, Sys };
InfoLayout layout(const Info &info) {
    if (info.code == -2)
        return InfoLayout::Timer;
    if (info.code == -5)
        return InfoLayout::Poll;
    if (info.code < 0)
        return InfoLayout::Realtime;
    if (info.code == 0 || info.code >= 128)
        return InfoLayout::Kill;
    const unsigned limit = info.signal == 4    ? 11
                           : info.signal == 5  ? 6
                           : info.signal == 7  ? 5
                           : info.signal == 8  ? 15
                           : info.signal == 11 ? 10
                           : info.signal == 17 ? 6
                           : info.signal == 29 ? 6
                           : info.signal == 31 ? 2
                                               : 0;
    if (unsigned(info.code) <= limit) {
        if (info.signal == 17)
            return InfoLayout::Child;
        if (info.signal == 29)
            return InfoLayout::Poll;
        if (info.signal == 31)
            return InfoLayout::Sys;
        return InfoLayout::Fault;
    }
    return info.code <= 6 ? InfoLayout::Poll : InfoLayout::Kill;
}
struct Context {
    ThreadRef thread;
    ProcessRef process;
};
Context context(Hooks &hooks) {
    if (!hooks.current || !hooks.process)
        guest_error(38);
    auto thread = hooks.current();
    auto process = hooks.process(thread.pid);
    if (!thread.signals || !thread.mask || !process.signals)
        guest_error(3);
    return {thread, std::move(process)};
}
void prepare_output(Memory &memory, uint64_t address, size_t size) {
    if (!memory.accessible(address, size, write_perm))
        guest_error(14);
    // Resolve COW and file-backed faults before removing anything from a queue.
    while (size) {
        memory.translate(address, write_perm);
        const auto count = std::min<size_t>(size, page_size - address % page_size);
        address += count;
        size -= count;
    }
}
} // namespace

SignalFileInfo signal_file_info(const Info &info) {
    SignalFileInfo result;
    result.signo = info.signal;
    result.error = info.error;
    result.code = info.code;
    switch (layout(info)) {
    case InfoLayout::Timer:
        result.tid = payload<uint32_t>(info, 0);
        result.overrun = payload<uint32_t>(info, 4);
        result.integer = payload<int32_t>(info, 8);
        result.pointer = payload<uint64_t>(info, 8);
        break;
    case InfoLayout::Poll:
        result.band = uint32_t(payload<uint64_t>(info, 0));
        result.fd = payload<int32_t>(info, 8);
        break;
    case InfoLayout::Realtime:
        result.integer = payload<int32_t>(info, 8);
        result.pointer = payload<uint64_t>(info, 8);
        [[fallthrough]];
    case InfoLayout::Kill:
        result.pid = info.sender_pid();
        result.uid = info.sender_uid();
        break;
    case InfoLayout::Child:
        result.pid = info.sender_pid();
        result.uid = info.sender_uid();
        result.status = payload<int32_t>(info, 8);
        result.user_time = payload<uint64_t>(info, 16);
        result.system_time = payload<uint64_t>(info, 24);
        break;
    case InfoLayout::Sys:
        result.call_address = payload<uint64_t>(info, 0);
        result.syscall = payload<int32_t>(info, 8);
        result.architecture = payload<uint32_t>(info, 12);
        break;
    case InfoLayout::Fault:
        result.address = info.fault_address();
        if (info.signal == 7 && (info.code == 4 || info.code == 5))
            result.address_lsb = payload<uint16_t>(info, 8);
        break;
    }
    return result;
}

SignalFile::SignalFile(FD reader, FD writer, uint64_t mask)
    : reader_(std::move(reader)), writer_(std::move(writer)), mask_(mask & ~unblockable) {}
SignalFile::~SignalFile() {
    if (description_) {
        auto &files = registry();
        std::lock_guard<std::mutex> guard(files.mutex);
        files.entries.erase(description_.get());
    }
}
int SignalFile::create(DescriptorTable &descriptors, uint64_t mask, uint32_t flags) {
    int pipes[2];
    check_host(::pipe(pipes));
    FD reader(pipes[0]), writer(pipes[1]);
    for (const int fd : {reader.get(), writer.get()}) {
        check_host(::fcntl(fd, F_SETFD, FD_CLOEXEC));
        check_host(::fcntl(fd, F_SETFL, O_NONBLOCK));
    }
    auto file = std::shared_ptr<SignalFile>(new SignalFile(std::move(reader), std::move(writer), mask));
    FD native(::fcntl(file->reader_.get(), F_DUPFD_CLOEXEC, 0));
    check_host(native.get());
    const int fd = descriptors.insert(std::move(native), "anon_inode:[signalfd]", 2 | int(flags));
    try {
        auto &entry = descriptors.get(fd);
        file->description_ = entry.description;
        file->description_->scheduler_io = true;
        {
            auto &files = registry();
            std::lock_guard<std::mutex> guard(files.mutex);
            files.entries.emplace(file->description_.get(), file);
        }
        entry.description = std::shared_ptr<OpenDescription>(file, file->description_.get());
    } catch (...) {
        descriptors.close(fd);
        throw;
    }
    return fd;
}
std::shared_ptr<SignalFile> SignalFile::find(const std::shared_ptr<OpenDescription> &description) {
    auto &files = registry();
    std::lock_guard<std::mutex> guard(files.mutex);
    auto found = files.entries.find(description.get());
    return found == files.entries.end() ? nullptr : found->second.lock();
}
void SignalFile::set_mask(uint64_t mask) {
    mask_ = mask & ~unblockable;
}
void SignalFile::marker(bool ready) {
    if (ready_ == ready)
        return;
    char byte = 1;
    ssize_t count;
    do {
        count = ready ? host_write(writer_.get(), &byte, 1) : ::read(reader_.get(), &byte, 1);
    } while (count < 0 && errno == EINTR);
    if (count != 1)
        guest_error(count < 0 ? linux_errno(errno) : 5);
    ready_ = ready;
}
bool SignalFile::readable(const ProcessState &process, const ThreadState &thread, uint64_t blocked) const {
    return has_waitable(process, thread, mask_ & blocked);
}
void SignalFile::refresh(const ProcessState &process, const ThreadState &thread, uint64_t blocked) {
    marker(readable(process, thread, blocked));
}
short SignalFile::poll_events(const ProcessState &process, const ThreadState &thread, uint64_t blocked,
                              short requested) {
    refresh(process, thread, blocked);
    return ready_ ? requested & POLLIN : 0;
}
size_t SignalFile::read(ProcessState &process, ThreadState &thread, uint64_t blocked, void *output,
                        size_t count) {
    if (count < sizeof(SignalFileInfo))
        guest_error(22);
    size_t used = 0;
    count = std::min(count, max_io);
    while (count - used >= sizeof(SignalFileInfo)) {
        auto queued = take_waitable(process, thread, mask_ & blocked);
        if (!queued)
            break;
        const auto info = signal_file_info(queued->info);
        std::memcpy(static_cast<uint8_t *>(output) + used, &info, sizeof(info));
        used += sizeof(info);
    }
    refresh(process, thread, blocked);
    return used;
}

int64_t signalfd4(Memory &memory, DescriptorTable &descriptors, Hooks &hooks, int32_t fd,
                  uint64_t mask_address, uint64_t mask_size, uint32_t flags) {
    if (mask_size != sizeof(uint64_t))
        guest_error(22);
    if (!memory.accessible(mask_address, sizeof(uint64_t), read_perm))
        guest_error(14);
    const auto mask = memory.load(mask_address, sizeof(uint64_t)) & ~unblockable;
    if (flags & ~(nonblock | cloexec))
        guest_error(22);
    const auto caller = context(hooks);
    if (fd == -1) {
        fd = SignalFile::create(descriptors, mask, flags);
        try {
            auto file = SignalFile::find(descriptors.get(fd).description);
            file->refresh(*caller.process.signals, *caller.thread.signals, *caller.thread.mask);
        } catch (...) {
            descriptors.close(fd);
            throw;
        }
    } else {
        auto file = SignalFile::find(descriptors.get(uint32_t(fd)).description);
        if (!file)
            guest_error(22);
        file->set_mask(mask);
        file->refresh(*caller.process.signals, *caller.thread.signals, *caller.thread.mask);
    }
    return fd;
}
std::optional<int64_t> signal_file_io(Memory &memory, DescriptorTable &descriptors, Hooks &hooks,
                                     const std::shared_ptr<OpenDescription> &description, bool writing,
                                     uint64_t address, uint64_t count, bool positioned, bool may_block) {
    auto file = SignalFile::find(description);
    if (!file)
        return {};
    if (positioned)
        guest_error(29);
    if (writing || count < sizeof(SignalFileInfo))
        guest_error(22);
    auto caller = context(hooks);
    auto attempt = [file, &memory, &descriptors, hooks, caller, address,
                    count = std::min<uint64_t>(count, max_io)]() mutable -> std::optional<int64_t> {
        auto &process = *caller.process.signals;
        auto &thread = *caller.thread.signals;
        size_t used = 0;
        while (count - used >= sizeof(SignalFileInfo) &&
               file->readable(process, thread, *caller.thread.mask)) {
            SignalFileInfo info;
            try {
                prepare_output(memory, address + used, sizeof(info));
            } catch (const GuestError &) {
                if (!used)
                    throw;
                break;
            } catch (const MemoryFault &) {
                if (!used)
                    throw;
                break;
            }
            if (!file->read(process, thread, *caller.thread.mask, &info, sizeof(info)))
                break;
            memory.copy_in(address + used, &info, sizeof(info));
            used += sizeof(info);
        }
        file->refresh(process, thread, *caller.thread.mask);
        signal_files_safe_point(descriptors, hooks);
        return used ? std::optional<int64_t>(used) : std::nullopt;
    };
    if (auto result = attempt())
        return result;
    if ((description->flags & nonblock) || !may_block)
        return -11;
    if (has_deliverable(*caller.process.signals, *caller.thread.signals, *caller.thread.mask)) {
        interrupted(*caller.thread.signals, *caller.thread.mask);
        return -4;
    }
    if (!hooks.block)
        guest_error(38);
    hooks.block({}, -11, [attempt = std::move(attempt), caller]() mutable -> std::optional<int64_t> {
        if (auto result = attempt())
            return result;
        if (!has_deliverable(*caller.process.signals, *caller.thread.signals, *caller.thread.mask))
            return {};
        interrupted(*caller.thread.signals, *caller.thread.mask);
        return -4;
    });
    return 0;
}
std::optional<short> signal_file_poll(const std::shared_ptr<OpenDescription> &description, Hooks &hooks,
                                     short requested) {
    auto file = SignalFile::find(description);
    if (!file)
        return {};
    const auto caller = context(hooks);
    return file->poll_events(*caller.process.signals, *caller.thread.signals, *caller.thread.mask, requested);
}
void signal_files_safe_point(DescriptorTable &descriptors, Hooks &hooks) {
    std::vector<std::shared_ptr<SignalFile>> files;
    std::set<SignalFile *> seen;
    for (const auto fd : descriptors.numbers())
        if (auto file = SignalFile::find(descriptors.get(fd).description);
            file && seen.insert(file.get()).second)
            files.push_back(std::move(file));
    if (files.empty())
        return;
    const auto caller = context(hooks);
    for (const auto &file : files)
        file->refresh(*caller.process.signals, *caller.thread.signals, *caller.thread.mask);
}
} // namespace operit::arm64::signals
