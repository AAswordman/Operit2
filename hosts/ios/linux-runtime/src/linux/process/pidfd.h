#pragma once
#include "base/fd.h"
#include "linux/signals/signals.h"
#include "runtime/process/controller.h"
#include <cstdint>
#include <memory>
#include <optional>

namespace operit::arm64 {
class DescriptorTable;
struct Descriptor;
struct OpenDescription;
namespace process {
constexpr uint32_t pidfd_nonblock = 2048;

// These are guest credentials. In particular, a host uid of zero does not grant
// either guest capability. REALCREDS is used for pidfd_getfd's ptrace check.
struct PidfdCredentials {
    uint32_t real_uid = 0, effective_uid = 0, saved_uid = 0;
    uint32_t real_gid = 0, effective_gid = 0, saved_gid = 0;
    uint64_t permitted_capabilities = 0;
    bool dumpable = true, cap_kill = false, cap_sys_ptrace = false;
};

// Machine must call created once for each new guest process, including PID 1,
// exited when its whole thread group exits, and reaped as soon as it ceases to
// be waitable (including automatic reaping). Exec must retain this identity.
// Call vm_exit before destroying the controller. These hooks and controller
// operations run on Machine's serialized syscall/lifecycle thread.
void pidfd_process_created(ProcessController &controller, uint32_t pid,
                           const PidfdCredentials &credentials);
void pidfd_process_exited(ProcessController &controller, uint32_t pid) noexcept;
void pidfd_process_reaped(ProcessController &controller, uint32_t pid) noexcept;
void pidfd_vm_exit(ProcessController &controller) noexcept;
void pidfd_update_credentials(ProcessController &controller, uint32_t pid,
                              const PidfdCredentials &credentials);
PidfdCredentials pidfd_credentials(ProcessController &controller, uint32_t pid);
// A deferred NPROC check is armed only by an actual guest real-uid change.
// Lowering a resource limit or updating dumpability alone does not arm it.
// Admission to clone/fork and an exec check that passes clear the marker.
bool pidfd_nproc_check_pending(ProcessController &controller, uint32_t pid);
void pidfd_clear_nproc_check(ProcessController &controller, uint32_t pid);

struct PidfdState;
class PidfdFile {
    friend struct PidfdState;
    friend int pidfd_open(ProcessController &, DescriptorTable &, int32_t, uint32_t);
    std::shared_ptr<PidfdState> target_;
    FD reader_, writer_;
    PidfdFile(std::shared_ptr<PidfdState> target, FD reader, FD writer);
    void publish_exit() noexcept;
    void publish_reap() noexcept;

  public:
    uint32_t guest_pid() const;
    int native_handle() const { return reader_.get(); }
    // Linux POLL/EPOLL bits: IN/RDNORM after exit, plus HUP after reaping.
    // The native handle has the same persistent IN/HUP readiness and can be
    // registered directly with the existing host poll/epoll/kqueue adapters.
    uint32_t poll_events(uint32_t requested) const;
    void validate(ProcessController &controller, int error = 3) const;
    int64_t send_signal(ProcessController &controller, uint32_t caller_pid, int signal,
                        const std::optional<signals::Info> &info, uint32_t flags) const;
    void check_getfd_access(ProcessController &controller, uint32_t caller_pid) const;
    std::optional<ProcessWaitResult> wait(ProcessController &controller, ProcessWait request) const;
};

int pidfd_open(ProcessController &controller, DescriptorTable &descriptors, int32_t pid,
               uint32_t flags);
// The attachment is owned by the OpenDescription's shared_ptr control block,
// so dup, fork, CLONE_FILES, exec, in-VM SCM_RIGHTS and getfd retain the same pidfd.
// No DescriptorTable/OpenDescription layout change or close hook is required.
std::shared_ptr<PidfdFile> pidfd_file(const std::shared_ptr<OpenDescription> &description) noexcept;
std::shared_ptr<PidfdFile> require_pidfd(const Descriptor &descriptor);
// Imports the actual guest open description, including virtual files and epoll;
// native-backed files also receive a real F_DUPFD_CLOEXEC reference. The guest
// result always has FD_CLOEXEC, independently of the source descriptor flags.
int pidfd_copy_descriptor(DescriptorTable &destination, const Descriptor &source);
} // namespace process
} // namespace operit::arm64
