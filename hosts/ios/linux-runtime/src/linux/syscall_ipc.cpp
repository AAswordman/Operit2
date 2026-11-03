#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <cerrno>
#include <unistd.h>

namespace operit::arm64 {
Syscalls::IpcCredentials Syscalls::ipc_credentials() {
    auto *controller = scheduler_.processes();
    if (!controller)
        guest_error(38);
    const auto identity = process::pidfd_credentials(*controller, scheduler_.current().pid);
    CpuState query{};
    query.x[0] = UINT32_MAX;
    query.x[8] = 151;
    const auto fsuid = uint32_t(dispatch_identity(query));
    query.x[8] = 152;
    const auto fsgid = uint32_t(dispatch_identity(query));
    std::vector<gid_t> host_groups;
    for (;;) {
        const int count = ::getgroups(0, nullptr);
        check_host(count);
        if (!count) {
            const int actual = ::getgroups(0, nullptr);
            check_host(actual);
            if (actual)
                continue;
            host_groups.clear();
            break;
        }
        host_groups.resize(count);
        const int actual = ::getgroups(count, host_groups.data());
        if (actual < 0 && errno == EINVAL)
            continue;
        check_host(actual);
        host_groups.resize(actual);
        break;
    }
    return {identity.real_uid, identity.effective_uid, identity.effective_gid, fsuid, fsgid,
            std::vector<uint32_t>(host_groups.begin(), host_groups.end())};
}

void Syscalls::sync_shared_memory_mappings() {
    ipc::shared_memory_mappings_changed(shared_memory_space_, memory_, scheduler_.current().pid);
}

void Syscalls::inherit_shared_memory_space(const Syscalls &parent, bool shared, uint32_t child_pid) {
    shared_memory_space_ = shared ? parent.shared_memory_space_
                                  : ipc::fork_shared_memory_address_space(parent.shared_memory_space_, memory_,
                                                                          child_pid);
}

void Syscalls::release_shared_memory_space(uint32_t pid) noexcept {
    ipc::release_shared_memory_address_space(shared_memory_space_, pid);
}

void Syscalls::commit_ipc_exec() {
    const auto pid = scheduler_.current().pid;
    for (const auto fd : descriptors_.numbers()) {
        const auto &entry = descriptors_.get(fd);
        if (entry.descriptor_flags & 1)
            ipc::posix_mq_close(entry.description, pid);
    }
    ipc::posix_mq_process_exec(posix_mq_namespace_, pid);
}

int64_t Syscalls::dispatch_ipc(CpuState &cpu) {
    const auto identity = ipc_credentials();
    if (cpu.x[8] >= 180 && cpu.x[8] <= 185) {
        auto *controller = scheduler_.processes();
        auto hooks = controller ? controller->signal_hooks() : signals::Hooks{};
        ipc::PosixMqContext context;
        context.credentials = {identity.uid, identity.fsuid, identity.fsgid, identity.groups,
                               false, false, false};
        context.umask = fs_context_->mask;
        context.message_queue_limit = resource_limits()->get(process::Resource::MessageQueue).soft;
        context.interrupt_pending = [this] { return interrupt_for_pending_signal(); };
        context.signal_hooks = controller ? &hooks : nullptr;
        return ipc::posix_mq_syscall(posix_mq_namespace_, memory_, descriptors_, scheduler_, context,
                                      cpu.x[8], cpu.x[0], cpu.x[1], cpu.x[2], cpu.x[3], cpu.x[4]);
    }
    if (cpu.x[8] >= 186 && cpu.x[8] <= 189) {
        ipc::MessageQueueCredentials credentials{identity.euid, identity.egid, identity.fsgid,
                                                 identity.groups, false, false, false};
        return ipc::message_queue_syscall(message_queue_namespace_, memory_, credentials, cpu.x[8],
                                          cpu.x[0], cpu.x[1], cpu.x[2], cpu.x[3], cpu.x[4],
                                          [this] { return interrupt_for_pending_signal(); });
    }
    if (cpu.x[8] >= 190 && cpu.x[8] <= 193) {
        auto &task = scheduler_.current();
        if (!task.semaphore_task)
            task.semaphore_task = ipc::make_semaphore_task();
        ipc::SemaphoreCredentials credentials{identity.euid, identity.egid, identity.fsgid,
                                              identity.groups, false, false};
        return ipc::semaphore_syscall(semaphore_namespace_, *task.semaphore_task, memory_, credentials,
                                      cpu.x[8], cpu.x[0], cpu.x[1], cpu.x[2], cpu.x[3],
                                      [this] { return interrupt_for_pending_signal(); });
    }
    ipc::SharedMemoryCredentials credentials{identity.euid, identity.egid, identity.fsgid, identity.groups,
                                             false, false};
    return ipc::shared_memory_syscall(shared_memory_namespace_, shared_memory_space_, memory_,
                                      *resource_limits(), credentials, scheduler_.current().pid, mmap_next_,
                                      cpu.x[8], cpu.x[0], cpu.x[1], cpu.x[2]);
}
} // namespace operit::arm64
