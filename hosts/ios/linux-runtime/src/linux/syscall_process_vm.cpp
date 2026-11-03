#include "linux/syscalls.h"
#include "linux/process/pidfd.h"
#include "memory/fault.h"
#include <algorithm>
#include <array>
#include <limits>
#include <new>
#include <vector>

namespace operit::arm64 {
namespace {
// AArch64 LP64, little-endian iovec. Both descriptor arrays live in the caller.
struct ProcessVmIovec {
    uint64_t base, length;
};
static_assert(sizeof(ProcessVmIovec) == 16 && offsetof(ProcessVmIovec, length) == 8);
constexpr uint64_t process_vm_iov_max = 1024;
// Linux MAX_RW_COUNT for the runtime's 4 KiB guest pages. This is independent
// of max_io: the staging buffer must not impose a one-MiB syscall limit.
constexpr uint64_t process_vm_max_rw = 0x7ffff000;
constexpr uint64_t process_vm_ssize_max = uint64_t(std::numeric_limits<int64_t>::max());

// access_ok tests the user address range, not whether a VMA/page is present.
// In particular, local mapping/protection faults are discovered during copying,
// so an accessible prefix is returned even within a single iovec.
bool process_vm_access_ok(uint64_t address, uint64_t length) {
    return address <= address_limit && length <= address_limit - address;
}
std::vector<ProcessVmIovec> process_vm_import(Memory &memory, uint64_t address,
                                             uint64_t count, bool local) {
    if (count > process_vm_iov_max)
        guest_error(22);
    std::vector<ProcessVmIovec> vectors(size_t(count), ProcessVmIovec{});
    if (!count)
        return vectors;
    const uint64_t bytes = count * sizeof(ProcessVmIovec);
    if (!process_vm_access_ok(address, bytes))
        guest_error(14);
    // Snapshot before any payload store, including when an output overlaps the
    // descriptors. No host pointer is ever obtained from a guest iov_base.
    memory.copy_out(address, vectors.data(), size_t(bytes));
    uint64_t total = 0;
    for (auto &vector : vectors) {
        if (vector.length > process_vm_ssize_max)
            guest_error(22);
        if (!local)
            continue; // Remote ranges are faulted only when reached.
        // import_iovec uses import_ubuf for a single segment: it caps that
        // segment before access_ok, whereas the multi-segment path checks first.
        if (count == 1)
            vector.length = std::min(vector.length, process_vm_max_rw);
        if (!process_vm_access_ok(vector.base, vector.length))
            guest_error(14);
        vector.length = std::min(vector.length, process_vm_max_rw - total);
        total += vector.length;
    }
    return vectors;
}
bool process_vm_ptrace_permitted(const process::PidfdCredentials &from,
                                  const process::PidfdCredentials &to) {
    // Match pidfd_getfd's PTRACE_MODE_ATTACH_REALCREDS rule. These are guest
    // credentials in this controller's namespace; host uid 0 confers no rights.
    if (from.cap_sys_ptrace)
        return true;
    return to.dumpable && from.real_uid == to.real_uid && from.real_uid == to.effective_uid &&
           from.real_uid == to.saved_uid && from.real_gid == to.real_gid &&
           from.real_gid == to.effective_gid && from.real_gid == to.saved_gid &&
           !(to.permitted_capabilities & ~from.permitted_capabilities);
}
int64_t process_vm_copy(Memory &local_memory, Memory &remote_memory,
                        const std::vector<ProcessVmIovec> &local,
                        const std::vector<ProcessVmIovec> &remote, bool writing) {
    std::array<uint8_t, page_size> bytes;
    size_t li = 0, ri = 0;
    uint64_t lo = 0, ro = 0, copied = 0;
    try {
        while (li < local.size() && ri < remote.size()) {
            if (lo == local[li].length) {
                ++li;
                lo = 0;
                continue;
            }
            if (ro == remote[ri].length) {
                ++ri;
                ro = 0;
                continue;
            }
            const uint64_t la = local[li].base + lo, ra = remote[ri].base + ro;
            // Each successful step ends within both pages and the guest user
            // range. A remote base/length overflow can never wrap into low RAM.
            if (la >= address_limit || ra >= address_limit)
                guest_error(14);
            const auto count = std::min({local[li].length - lo, remote[ri].length - ro,
                                         page_size - la % page_size, page_size - ra % page_size});
            if (writing) {
                local_memory.copy_out(la, bytes.data(), size_t(count));
                remote_memory.copy_in(ra, bytes.data(), size_t(count));
            } else {
                remote_memory.copy_out(ra, bytes.data(), size_t(count));
                local_memory.copy_in(la, bytes.data(), size_t(count));
            }
            // Each Memory call has released all its access/page locks here.
            // Never retain a source PageAccess while locking a destination:
            // these spaces can alias COW, shared-file or anonymous pages.
            copied += count;
            lo += count;
            ro += count;
        }
    } catch (const MemoryFault &) {
        return copied ? int64_t(copied) : -14; // Includes file EOF/bus faults.
    } catch (const GuestError &error) {
        return copied ? int64_t(copied) : -int64_t(error.number);
    } catch (const std::bad_alloc &) {
        return copied ? int64_t(copied) : -12;
    }
    return int64_t(copied);
}
} // namespace

int64_t Syscalls::dispatch_process_vm(CpuState &cpu) {
    // ABI/ordering reference: Linux v6.18 mm/process_vm_access.c,
    // lib/iov_iter.c, kernel/fork.c (mm_access/may_access_mm), kernel/ptrace.c
    // and security/commoncap.c; asm-generic/unistd.h assigns 270/271.
    // https://github.com/torvalds/linux/tree/v6.18
    //
    // Coordinator-only, like clone/exec: runtime/parallel.cpp joins every CPU
    // worker before calling handle(), then dispatches syscalls serially. Task
    // membership, image lifetime and VMAs therefore stay stable until return.
    // Do not move this handler into worker execution, block/yield, or call a
    // controller while holding a Memory guard. Credential registry locks are
    // acquired/released before any payload memory access; no new outer lock is
    // needed (and would risk inversion with the memory backing-store locks).
    const auto a = cpu.x;
    if (a[8] != 270 && a[8] != 271)
        return unsupported(cpu);
    if (a[5])
        return -22;
    try {
        const auto local = process_vm_import(memory_, a[1], a[2], true);
        const bool local_empty = std::all_of(local.begin(), local.end(),
                                             [](const auto &v) { return !v.length; });
        if (local_empty)
            return 0; // Linux does not import rvec or look up pid in this case.
        const auto remote = process_vm_import(memory_, a[3], a[4], false);
        if (std::all_of(remote.begin(), remote.end(), [](const auto &v) { return !v.length; }))
            return 0;

        // pid_t consumes the low 32 bits of x0. Unlike Scheduler::find(), pid 0
        // is not an alias for self, and negatives never select process groups.
        const int32_t id = int32_t(a[0]);
        const auto *task = id > 0 ? scheduler_.lookup(uint32_t(id)) : nullptr;
        if (!task || task->state == Task::State::Exited)
            return -3;
        const auto caller_pid = scheduler_.current().pid;
        if (process_anchor_->pid != caller_pid)
            return -1; // An incorrectly bound context cannot supply authority.
        auto *controller = scheduler_.processes();
        Syscalls *target = this;
        if (task->pid != caller_pid) {
            if (!controller)
                return -1; // No guest identity/permission provider: fail closed.
            target = controller->process_syscalls(task->pid);
            if (!target)
                return -3;
            if (&target->scheduler_ != &scheduler_ || target->process_anchor_->pid != task->pid)
                return -1; // Never follow a foreign VM's Syscalls/Memory.
        }
        // Linux mm_access permits an identical mm (including CLONE_VM without
        // CLONE_THREAD); same-thread-group introspection also bypasses ptrace.
        if (task->pid != caller_pid && &target->memory_ != &memory_) {
            process::PidfdCredentials from, to;
            try {
                from = process::pidfd_credentials(*controller, caller_pid);
                to = process::pidfd_credentials(*controller, task->pid);
            } catch (const GuestError &error) {
                if (error.number != 3)
                    throw;
                return -1; // A live target with unmodelled credentials is denied.
            }
            if (!process_vm_ptrace_permitted(from, to))
                return -1;
        }
        return process_vm_copy(memory_, target->memory_, local, remote, a[8] == 271);
    } catch (const GuestError &error) {
        return -int64_t(error.number);
    } catch (const MemoryFault &) {
        return -14;
    } catch (const std::bad_alloc &) {
        return -12;
    }
}
} // namespace operit::arm64
