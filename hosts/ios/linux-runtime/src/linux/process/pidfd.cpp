#include "linux/process/pidfd.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/error_mapping.h"
#include "linux/timers/ready_pipe.h"
#include <algorithm>
#include <cerrno>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <utility>
#include <vector>

namespace operit::arm64::process {
namespace {
struct PidfdNamespace;
}
struct PidfdState {
    enum class Phase { Running, Exited, Reaped } phase = Phase::Running;
    const uint32_t pid;
    std::weak_ptr<PidfdNamespace> space;
    PidfdCredentials credentials;
    bool nproc_check_pending = false;
    std::vector<std::weak_ptr<PidfdFile>> files;
    PidfdState(uint32_t id, const std::shared_ptr<PidfdNamespace> &owner,
               const PidfdCredentials &access)
        : pid(id), space(owner), credentials(access) {}
    void exited() noexcept;
    void reaped() noexcept;
};
namespace {
struct PidfdNamespace {
    std::map<uint32_t, std::shared_ptr<PidfdState>> processes;
};
struct Registry {
    std::mutex mutex;
    std::map<ProcessController *, std::shared_ptr<PidfdNamespace>> spaces;
};
Registry &registry() {
    static Registry instance;
    return instance;
}
// The deleter carries the attachment without changing the shared descriptor
// layout. Release it at the last strong reference, not when epoll's weak owner
// references finally disappear. Ordinary descriptors never have this deleter.
struct PidfdOwner {
    std::shared_ptr<PidfdFile> file;
    void operator()(OpenDescription *description) noexcept {
        delete description;
        file.reset();
    }
};
std::shared_ptr<PidfdState> lookup(Registry &state, ProcessController &controller, uint32_t pid,
                                 int error = 3) {
    auto space = state.spaces.find(&controller);
    if (space == state.spaces.end())
        guest_error(error);
    auto process = space->second->processes.find(pid);
    if (process == space->second->processes.end())
        guest_error(error);
    return process->second;
}
void validate(Registry &state, ProcessController &controller, const std::shared_ptr<PidfdState> &target,
              int error) {
    auto owner = target->space.lock();
    auto space = state.spaces.find(&controller);
    if (!owner || space == state.spaces.end() || space->second != owner ||
        target->phase == PidfdState::Phase::Reaped)
        guest_error(error);
    auto current = owner->processes.find(target->pid);
    if (current == owner->processes.end() || current->second != target)
        guest_error(error);
}
bool signal_permitted(const PidfdCredentials &from, const PidfdCredentials &to, int signal,
                      const signals::ProcessRef &sender, const signals::ProcessRef &target) {
    return from.cap_kill || from.real_uid == to.real_uid || from.real_uid == to.saved_uid ||
           from.effective_uid == to.real_uid || from.effective_uid == to.saved_uid ||
           (signal == signals::continue_signal && sender.session == target.session);
}
bool ptrace_permitted(const PidfdCredentials &from, const PidfdCredentials &to) {
    if (from.cap_sys_ptrace)
        return true;
    return to.dumpable && from.real_uid == to.real_uid && from.real_uid == to.effective_uid &&
           from.real_uid == to.saved_uid && from.real_gid == to.real_gid &&
           from.real_gid == to.effective_gid && from.real_gid == to.saved_gid &&
           !(to.permitted_capabilities & ~from.permitted_capabilities);
}
} // namespace

PidfdFile::PidfdFile(std::shared_ptr<PidfdState> target, FD reader, FD writer)
    : target_(std::move(target)), reader_(std::move(reader)), writer_(std::move(writer)) {}
void PidfdFile::publish_exit() noexcept {
    // Keep our own read reference until the last open description is closed.
    // Even a concurrently closed guest descriptor cannot cause host SIGPIPE.
    if (writer_.get() < 0)
        return;
    const uint8_t ready = 1;
    ssize_t written;
    do {
        written = ::write(writer_.get(), &ready, sizeof(ready));
    } while (written < 0 && errno == EINTR);
    if (written != 1 && !(written < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)))
        writer_ = FD(); // Still wake native waiters if the marker could not be written.
}
void PidfdFile::publish_reap() noexcept {
    writer_ = FD();
}
void PidfdState::exited() noexcept {
    if (phase != Phase::Running)
        return;
    phase = Phase::Exited;
    for (auto &entry : files)
        if (auto file = entry.lock())
            file->publish_exit();
}
void PidfdState::reaped() noexcept {
    if (phase == Phase::Reaped)
        return;
    exited();
    phase = Phase::Reaped;
    for (auto &entry : files)
        if (auto file = entry.lock())
            file->publish_reap();
    files.clear();
}
void pidfd_process_created(ProcessController &controller, uint32_t pid,
                           const PidfdCredentials &credentials) {
    if (!pid || pid > INT32_MAX)
        throw Failure("invalid guest process identity for pidfd");
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    auto &space = state.spaces[&controller];
    if (!space)
        space = std::make_shared<PidfdNamespace>();
    auto fresh = std::make_shared<PidfdState>(pid, space, credentials);
    auto current = space->processes.find(pid);
    if (current != space->processes.end()) {
        // A new incarnation must invalidate every old pidfd before publishing
        // the reused guest number. Never redirect an old pidfd to the new task.
        current->second->reaped();
        current->second = std::move(fresh);
    } else
        space->processes.emplace(pid, std::move(fresh));
}
void pidfd_process_exited(ProcessController &controller, uint32_t pid) noexcept {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    auto space = state.spaces.find(&controller);
    if (space == state.spaces.end())
        return;
    auto process = space->second->processes.find(pid);
    if (process != space->second->processes.end())
        process->second->exited();
}
void pidfd_process_reaped(ProcessController &controller, uint32_t pid) noexcept {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    auto space = state.spaces.find(&controller);
    if (space == state.spaces.end())
        return;
    auto process = space->second->processes.find(pid);
    if (process != space->second->processes.end()) {
        process->second->reaped();
        space->second->processes.erase(process);
    }
}
void pidfd_vm_exit(ProcessController &controller) noexcept {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    auto space = state.spaces.find(&controller);
    if (space == state.spaces.end())
        return;
    for (auto &process : space->second->processes)
        process.second->reaped();
    state.spaces.erase(space);
}
void pidfd_update_credentials(ProcessController &controller, uint32_t pid,
                              const PidfdCredentials &credentials) {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    const auto target = lookup(state, controller, pid);
    if (target->credentials.real_uid != credentials.real_uid)
        target->nproc_check_pending = true;
    target->credentials = credentials;
}
PidfdCredentials pidfd_credentials(ProcessController &controller, uint32_t pid) {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    return lookup(state, controller, pid)->credentials;
}
bool pidfd_nproc_check_pending(ProcessController &controller, uint32_t pid) {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    return lookup(state, controller, pid)->nproc_check_pending;
}
void pidfd_clear_nproc_check(ProcessController &controller, uint32_t pid) {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    lookup(state, controller, pid)->nproc_check_pending = false;
}
uint32_t PidfdFile::guest_pid() const {
    return target_->pid;
}
uint32_t PidfdFile::poll_events(uint32_t requested) const {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    if (target_->phase == PidfdState::Phase::Running)
        return 0;
    return (requested & uint32_t{1 | 64}) | (target_->phase == PidfdState::Phase::Reaped ? 16u : 0u);
}
void PidfdFile::validate(ProcessController &controller, int error) const {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    process::validate(state, controller, target_, error);
}
int64_t PidfdFile::send_signal(ProcessController &controller, uint32_t caller_pid, int signal,
                               const std::optional<signals::Info> &supplied, uint32_t flags) const {
    if (flags || signal < 0 || signal > int(signals::maximum_signal))
        guest_error(22);
    if (supplied && supplied->signal != signal)
        guest_error(22);
    PidfdCredentials from, to;
    {
        auto &state = registry();
        std::lock_guard<std::mutex> guard(state.mutex);
        process::validate(state, controller, target_, 3);
        from = lookup(state, controller, caller_pid)->credentials;
        to = target_->credentials;
    }
    auto hooks = controller.signal_hooks();
    if (!hooks.process)
        guest_error(38);
    const auto sender = hooks.process(caller_pid);
    const auto target = hooks.process(target_->pid);
    if (!signal_permitted(from, to, signal, sender, target))
        guest_error(1);
    if (supplied && (supplied->code >= 0 || supplied->code == -6) && caller_pid != target_->pid)
        guest_error(1);
    const auto info = supplied.value_or(signals::Info::user(signal, caller_pid, from.real_uid));
    // This always queues through the guest signal subsystem. No guest PID is
    // ever passed to kill(2), a host pidfd syscall, or another host PID API.
    return signals::send_process(hooks, target, info);
}
void PidfdFile::check_getfd_access(ProcessController &controller, uint32_t caller_pid) const {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    process::validate(state, controller, target_, 3);
    const auto caller = lookup(state, controller, caller_pid);
    // Same thread group bypasses ptrace checks, as it does for Linux task
    // access. No uid-zero shortcut is allowed for another guest process.
    if (caller != target_ && !ptrace_permitted(caller->credentials, target_->credentials))
        guest_error(1);
    if (target_->phase != PidfdState::Phase::Running)
        guest_error(3); // exit_files has run; there is no target fd table to copy.
}
std::optional<ProcessWaitResult> PidfdFile::wait(ProcessController &controller, ProcessWait request) const {
    validate(controller, 10);
    request.kind = ProcessWait::Kind::Pid;
    request.id = target_->pid;
    // Validation and wait execute on the serialized Machine thread, including
    // scheduler retry callbacks. The captured token is revalidated on every
    // retry; closing/reusing the guest fd or PID cannot retarget this wait.
    return controller.wait(request);
}
int pidfd_open(ProcessController &controller, DescriptorTable &descriptors, int32_t pid,
               uint32_t flags) {
    if (pid <= 0 || (flags & ~pidfd_nonblock))
        guest_error(22);
    std::shared_ptr<PidfdFile> file;
    {
        auto &state = registry();
        std::lock_guard<std::mutex> guard(state.mutex);
        auto target = lookup(state, controller, uint32_t(pid));
        auto pipe = timers::ready_pipe();
        file = std::shared_ptr<PidfdFile>(
            new PidfdFile(target, std::move(pipe.first), std::move(pipe.second)));
        target->files.erase(std::remove_if(target->files.begin(), target->files.end(),
                                          [](const auto &entry) { return entry.expired(); }),
                            target->files.end());
        target->files.push_back(file);
        if (target->phase != PidfdState::Phase::Running)
            file->publish_exit();
        if (target->phase == PidfdState::Phase::Reaped)
            file->publish_reap();
    }
    FD host(::fcntl(file->native_handle(), F_DUPFD_CLOEXEC, 0));
    check_host(host.get());
    // Guest pidfds are always close-on-exec. O_NONBLOCK is an independent
    // open-description flag; the internal readiness pipe is always nonblocking.
    const int open_flags = 2 | int(flags) | 524288;
    auto description = std::shared_ptr<OpenDescription>(new OpenDescription(open_flags), PidfdOwner{file});
    description->scheduler_io = true;
    int fd = descriptors.insert(std::move(host), "", open_flags);
    auto &entry = descriptors.get(fd);
    description->inode = entry.description->inode;
    entry.description = std::move(description);
    return fd;
}
std::shared_ptr<PidfdFile> pidfd_file(const std::shared_ptr<OpenDescription> &description) noexcept {
    auto *owner = std::get_deleter<PidfdOwner>(description);
    return owner ? owner->file : nullptr;
}
std::shared_ptr<PidfdFile> require_pidfd(const Descriptor &descriptor) {
    auto file = pidfd_file(descriptor.description);
    if (!file)
        guest_error(9);
    return file;
}
int pidfd_copy_descriptor(DescriptorTable &destination, const Descriptor &source) {
    const auto description = source.description;
    if (!description)
        guest_error(9);
    // A virtual guest FD legitimately has no host FD, but it must carry its
    // actual virtual open object; manufacturing an empty native FD is invalid.
    if (source.host.get() < 0 && !description->virtual_file && !description->poll &&
        !description->terminal && !description->pipe && !description->timer && !description->counter &&
        description->standard_stream < 0)
        guest_error(9);
    FD host(source.host.get() < 0 ? -1 : ::fcntl(source.host.get(), F_DUPFD_CLOEXEC, 0));
    if (source.host.get() >= 0)
        check_host(host.get());
    // Snapshot before insert: source and destination may be the same table or
    // share CLONE_FILES state. Copying the description preserves file offsets,
    // status flags, sockets, timers, event counters, epoll and pidfd identity.
    const std::string path = source.path;
    int fd = destination.insert(std::move(host), path, description->flags | 524288);
    destination.get(fd).description = description;
    return fd;
}
} // namespace operit::arm64::process
