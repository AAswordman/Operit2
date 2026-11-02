#include "linux/syscalls.h"
#include "runtime/process/controller.h"
#include <unistd.h>

namespace operit::arm64 {
namespace {
process::LimitCredentials guest_limit_credentials() {
    process::LimitCredentials credentials;
    // Identity-changing syscalls currently only permit retaining this identity,
    // and capget/capset expose an empty guest capability set.
    credentials.real_uid = ::getuid();
    credentials.effective_uid = ::geteuid();
    credentials.saved_uid = ::geteuid();
    credentials.real_gid = ::getgid();
    credentials.effective_gid = ::getegid();
    credentials.saved_gid = ::getegid();
    return credentials;
}
} // namespace
int64_t Syscalls::dispatch_limits(CpuState &cpu) {
    const auto &a = cpu.x;
    const process::LimitProcess caller{process_anchor_->pid, guest_limit_credentials(), resource_limits()};
    const process::LimitProcessLookup lookup = [this](uint32_t pid) -> std::optional<process::LimitProcess> {
        auto *controller = scheduler_.processes();
        Syscalls *target = controller ? controller->process_syscalls(pid) : nullptr;
        if (!target) {
            const auto *thread = scheduler_.lookup(pid);
            if (!thread || thread->state == Task::State::Exited)
                return std::nullopt;
            if (thread->pid == process_anchor_->pid)
                target = this;
            else if (controller)
                target = controller->process_syscalls(thread->pid);
        }
        if (!target)
            return std::nullopt;
        return process::LimitProcess{target->process_anchor_->pid, guest_limit_credentials(),
                                     target->resource_limits()};
    };
    return process::resource_limit_syscall(memory_, a[8], a[0], a[1], a[2], a[3], caller, lookup);
}
} // namespace operit::arm64
