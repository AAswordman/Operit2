#pragma once

#include <cstdint>
#include <functional>
#include <memory>
#include <vector>

namespace operit::arm64 {
class Memory;
class Scheduler;

namespace ipc {
class SemaphoreNamespace;
class SemaphoreUndo;
class SemaphoreWaiter;

struct SemaphoreCredentials {
    uint32_t euid = 0, egid = 0, fsgid = 0;
    std::vector<uint32_t> groups;
    bool cap_ipc_owner = false;
    bool cap_sys_admin = false;
};

struct SemaphoreLimits {
    uint32_t max_sets = 32000, max_per_set = 32000, max_operations = 500;
    uint32_t max_semaphores = 1024000000;
};

// One namespace per guest IPC namespace. It is bound to one Scheduler and
// must not outlive it. All operations below, including cleanup, run at the
// scheduler's serial coordinator barrier, never on guest CPU worker threads.
std::shared_ptr<SemaphoreNamespace> make_semaphore_namespace(Scheduler &scheduler,
                                                              const SemaphoreLimits &limits = {});

// One distinct handle per guest task, including threads. Clone with share=true
// exactly for CLONE_SYSVSEM; a normal fork/clone receives an empty undo list.
// Preserve the surviving handle across exec, including non-leader exec.
class SemaphoreTask {
    friend std::unique_ptr<SemaphoreTask> make_semaphore_task();
    friend std::unique_ptr<SemaphoreTask> clone_semaphore_task(const SemaphoreTask &, bool);
    friend void semaphore_task_exit(std::unique_ptr<SemaphoreTask> &, Scheduler &, uint32_t);
    friend int64_t semaphore_syscall(const std::shared_ptr<SemaphoreNamespace> &, SemaphoreTask &,
                                     Memory &, const SemaphoreCredentials &, uint64_t, uint64_t,
                                     uint64_t, uint64_t, uint64_t, const std::function<bool()> &);
    std::shared_ptr<SemaphoreUndo> undo_;
    std::weak_ptr<SemaphoreWaiter> waiting_;
    explicit SemaphoreTask(std::shared_ptr<SemaphoreUndo> undo);

  public:
    ~SemaphoreTask();
    SemaphoreTask(const SemaphoreTask &) = delete;
    SemaphoreTask &operator=(const SemaphoreTask &) = delete;
};

std::unique_ptr<SemaphoreTask> make_semaphore_task();
std::unique_ptr<SemaphoreTask> clone_semaphore_task(const SemaphoreTask &parent, bool share);

// Call once for every exiting task, even during exit_group, fatal signals,
// failed clone rollback, and exec's removal of sibling threads. Call before
// scheduler retirement and before destroying its namespace. The last task
// sharing an undo list applies it and wakes any newly satisfiable waiters.
// On committed CLONE_NEWIPC/unshare, detach the old handle this way and install
// a fresh one; exec itself must not detach the surviving task's undo list.
void semaphore_task_exit(std::unique_ptr<SemaphoreTask> &task, Scheduler &scheduler, uint32_t tid);

// AArch64 syscall numbers 190=semget, 191=semctl, 192=semtimedop, 193=semop.
// Arguments are raw guest x0..x3. Return values are Linux results; errors throw
// GuestError or MemoryFault, handled by the normal syscall dispatcher. A blocked
// call returns 0 after Scheduler::block, like futex. The interrupted callback
// must detect and record an already-deliverable signal.
// The scheduler's normal interrupt/wake/retire paths run wait_cleanup. Guest
// credentials and capabilities must not be substituted with host privileges.
int64_t semaphore_syscall(const std::shared_ptr<SemaphoreNamespace> &ipc_namespace,
                          SemaphoreTask &task, Memory &memory,
                          const SemaphoreCredentials &credentials, uint64_t number,
                          uint64_t arg0, uint64_t arg1, uint64_t arg2, uint64_t arg3 = 0,
                          const std::function<bool()> &interrupted = {});
} // namespace ipc
} // namespace operit::arm64
