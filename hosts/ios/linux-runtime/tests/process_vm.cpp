#include "linux/process/pidfd.h"
#include "runtime/machine.h"
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <initializer_list>
#include <stdexcept>
#include <string>
#include <utility>
#include <unistd.h>

using namespace operit::arm64;
namespace {
constexpr uint64_t local_iov = 0x10000, remote_iov = 0x14000;
constexpr uint64_t local_data = 0x20000, remote_data = 0x24000;
constexpr uint64_t mapped_end = 0x30000;
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
struct Root {
    std::string path;
    Root() {
        char pattern[] = "/tmp/operit-process-vm-XXXXXX";
        auto *created = ::mkdtemp(pattern);
        require(created, "mkdtemp process_vm");
        path = created;
    }
    ~Root() { std::filesystem::remove_all(path); }
};
struct Fixture {
    Root root;
    std::unique_ptr<Machine> machine;
    Fixture() {
        ort_arm64_config config{};
        config.root_directory = root.path.c_str();
        config.execution_threads = 2;
        machine = std::make_unique<Machine>(config);
        memory().map(local_iov, mapped_end - local_iov, read_perm | write_perm);
        process::PidfdCredentials credentials;
        credentials.real_uid = credentials.effective_uid = credentials.saved_uid = 1000;
        credentials.real_gid = credentials.effective_gid = credentials.saved_gid = 1000;
        process::pidfd_update_credentials(*machine, 1, credentials);
    }
    Memory &memory(uint32_t pid = 1) { return *machine->find_process(pid)->image.memory; }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> arguments, uint32_t tid = 1) {
        // Exercise the public syscall path at the same serial boundary used by
        // Machine after workers join. Never invoke handle from a CPU worker.
        Scheduler::ScopedTask context(machine->scheduler(), tid);
        auto &task = machine->scheduler().current();
        auto &cpu = task.cpu;
        cpu.x.fill(0);
        std::copy(arguments.begin(), arguments.end(), cpu.x.begin());
        cpu.x[8] = number;
        machine->process_syscalls(task.pid)->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    uint32_t child(uint32_t parent = 1, uint64_t flags = 0) {
        Scheduler::ScopedTask context(machine->scheduler(), parent);
        ProcessClone request;
        request.flags = flags;
        if (flags & 0x10000) {
            request.stack = 0x2f000;
            request.exit_signal = 0;
        }
        return machine->clone(machine->scheduler().current().cpu, request);
    }
    void vectors(uint64_t at, std::initializer_list<std::pair<uint64_t, uint64_t>> values) {
        for (const auto &[base, length] : values) {
            memory().store(at, base, 8);
            memory().store(at + 8, length, 8);
            at += 16;
        }
    }
    void setup(uint64_t local = local_data, uint64_t remote = remote_data, uint64_t length = 8) {
        vectors(local_iov, {{local, length}});
        vectors(remote_iov, {{remote, length}});
    }
    int64_t transfer(bool write, uint64_t target = 1, uint64_t lc = 1, uint64_t rc = 1) {
        return call(write ? 271 : 270, {target, local_iov, lc, remote_iov, rc, 0});
    }
};
void self_and_scatter_gather() {
    Fixture f;
    f.setup();
    f.memory().store(remote_data, 0x8877665544332211ull, 8);
    require(f.transfer(false) == 8 && f.memory().load(local_data, 8) == 0x8877665544332211ull,
            "self process_vm_readv");
    f.memory().store(local_data, 0x123456789abcdef0ull, 8);
    require(f.transfer(true) == 8 && f.memory().load(remote_data, 8) == 0x123456789abcdef0ull,
            "self process_vm_writev");
    f.vectors(local_iov, {{UINT64_MAX, 0}, {local_data, 3}, {local_data + 8, 5}});
    f.vectors(remote_iov, {{remote_data, 2}, {UINT64_MAX, 0}, {remote_data + 2, 6}});
    // Local access_ok checks zero-length bases too; remote zero-length bases
    // are never dereferenced. Use a valid zero-length local base.
    f.memory().store(local_iov, 0, 8);
    f.memory().store(local_data, 0, 8);
    f.memory().store(local_data + 8, 0, 8);
    require(f.transfer(false, 1, 3, 3) == 8 &&
                f.memory().load(local_data, 4) == 0xbcdef0 &&
                f.memory().load(local_data + 8, 8) == 0x123456789aull,
            "mismatched iov lengths/order or zero-length entries");
    f.setup(local_data, remote_data, 4);
    f.memory().store(remote_iov + 8, 8, 8);
    f.memory().store(local_data, UINT64_MAX, 8);
    require(f.transfer(false) == 4 && f.memory().load(local_data + 4, 4) == UINT32_MAX,
            "shorter local total must bound transfer and preserve suffix");
    // Both descriptor arrays must be snapshotted before payload stores.
    f.vectors(local_iov, {{remote_iov + 16, 16}, {local_data, 4}});
    f.vectors(remote_iov, {{remote_data, 16}, {remote_data + 16, 4}});
    f.memory().store(remote_data + 16, 0xaabbccdd, 4);
    require(f.transfer(false, 1, 2, 2) == 20 && f.memory().load(local_data, 4) == 0xaabbccdd,
            "output overwriting remote descriptor array changed the transfer");
}
void validation_and_empty_ordering() {
    Fixture f;
    f.setup();
    require(f.call(270, {0, UINT64_MAX, 0, UINT64_MAX, UINT64_MAX, 0}) == 0,
            "zero local count must bypass remote import and PID lookup");
    require(f.call(271, {0, UINT64_MAX, 0, UINT64_MAX, 0, 1}) == -22,
            "flags must be checked even for an empty transfer");
    f.vectors(local_iov, {{0, 0}});
    require(f.call(270, {UINT32_MAX, local_iov, 1, UINT64_MAX, 1025, 0}) == 0,
            "zero local total must bypass remote validation");
    f.setup();
    require(f.transfer(false, 1, 1025) == -22 && f.transfer(true, 1, 1, 1025) == -22,
            "IOV_MAX bounds");
    require(f.transfer(false, 0) == -3 && f.transfer(true, UINT32_MAX) == -3 &&
                f.transfer(false, 0x7ffffff0) == -3,
            "zero, negative and missing PID are not aliases or host targets");
    require(f.transfer(false, (uint64_t{1} << 32) | 1) == 8, "AArch64 pid_t low-word ABI");
    require(f.call(270, {1, address_limit - 8, 1, remote_iov, 1, 0}) == -14 &&
                f.call(270, {1, local_iov, 1, UINT64_MAX, 1, 0}) == -14,
            "descriptor-array range overflow");
    f.memory().store(local_iov + 8, uint64_t{1} << 63, 8);
    require(f.transfer(false) == -22, "local signed-size overflow");
    f.setup();
    f.memory().store(remote_iov + 8, uint64_t{1} << 63, 8);
    require(f.transfer(true) == -22, "remote signed-size overflow");
    f.setup(address_limit - 1, remote_data, 2);
    require(f.transfer(false) == -14, "local access_ok range overflow");
    f.setup(local_data, UINT64_MAX, 8);
    require(f.transfer(true) == -14, "remote pointer must not wrap or become a host pointer");
    f.vectors(remote_iov, {{UINT64_MAX, 0}});
    require(f.transfer(false, 0) == 0, "empty remote total must bypass PID lookup");
    f.setup();
    f.memory().store(local_iov + 8, 0x80000000, 8);
    require(f.transfer(false) == 8, "MAX_RW_COUNT import must not allocate the requested payload");
    // Linux iovec_from_user rejects each negative ssize_t, but does not sum
    // remote lengths. The capped local iterator bounds the actual transfer.
    f.vectors(remote_iov, {{remote_data, uint64_t(INT64_MAX)}, {UINT64_MAX, uint64_t(INT64_MAX)}});
    f.memory().store(local_iov + 8, 8, 8);
    require(f.transfer(false, 1, 1, 2) == 8, "remote total must not wrap the local transfer budget");
    f.setup();
    f.memory().protect(local_iov, page_size, 0);
    require(f.transfer(true) == -14, "unreadable descriptor array");
}
void partial_transfers_and_memory_permissions() {
    for (const bool write : {false, true}) {
        Fixture f;
        f.setup(local_data, mapped_end - 4);
        f.memory().store(mapped_end - 4, 0x11223344, 4);
        f.memory().store(local_data, 0x8877665544332211ull, 8);
        require(f.transfer(write) == 4, "remote fault inside one iovec lost its copied prefix");
        require(write ? f.memory().load(mapped_end - 4, 4) == 0x44332211
                      : f.memory().load(local_data, 8) == 0x8877665511223344ull,
                "remote short-copy bytes/suffix");
        f.setup(mapped_end - 4, remote_data);
        f.memory().store(remote_data, 0xaabbccddeeff0099ull, 8);
        require(f.transfer(write) == 4, "local fault inside one iovec lost its copied prefix");
        f.setup(local_data, mapped_end);
        require(f.transfer(write) == -14, "first remote byte must report EFAULT");
        f.setup(mapped_end, remote_data);
        require(f.transfer(write) == -14, "first local byte must report EFAULT");
        f.vectors(local_iov, {{local_data, 4}, {local_data + 8, 4}});
        f.vectors(remote_iov, {{remote_data, 4}, {mapped_end, 4}});
        require(f.transfer(write, 1, 2, 2) == 4, "later bad iovec must return earlier bytes");
    }
    Fixture f;
    f.setup();
    f.memory().protect(local_data, page_size, read_perm);
    require(f.transfer(false) == -14, "readv must require writable local destination");
    f.memory().protect(local_data, page_size, write_perm);
    f.memory().store(remote_data, 0x12345678, 8);
    require(f.transfer(false) == 8, "readv must allow write-only local destination");
    require(f.transfer(true) == -14, "writev must require readable local source");
    f.memory().protect(local_data, page_size, read_perm | write_perm);
    f.memory().protect(remote_data, page_size, read_perm);
    require(f.transfer(true) == -14, "writev must not force writes to read-only remote memory");
    f.memory().protect(remote_data, page_size, exec_perm);
    require(f.transfer(false) == -14, "readv must not bypass execute-only remote protection");
    f.memory().protect(remote_data, page_size, write_perm);
    require(f.transfer(true) == 8, "writev must allow write-only remote destination");
    uint64_t host_word = 0x0123456789abcdefull;
    f.setup(local_data, uint64_t(reinterpret_cast<uintptr_t>(&host_word)));
    require(f.transfer(true) == -14 && host_word == 0x0123456789abcdefull,
            "guest pointer escaped into host memory");
}
void cross_process_tid_cow_and_lifetime() {
    Fixture f;
    f.setup();
    f.memory().store(remote_data, 0x1111, 8);
    const auto child = f.child();
    f.memory(child).store(remote_data, 0x2222, 8);
    // The remote descriptor array is deliberately inaccessible in the child;
    // both arrays must still be imported from the calling process.
    f.memory(child).protect(remote_iov, page_size, 0);
    require(f.transfer(false, child) == 8 && f.memory().load(local_data, 8) == 0x2222,
            "cross-process read or caller-owned remote descriptors");
    f.memory().store(local_data, 0x3333, 8);
    require(f.transfer(true, child) == 8 && f.memory(child).load(remote_data, 8) == 0x3333 &&
                f.memory().load(remote_data, 8) == 0x1111,
            "cross-process write bypassed fork COW");
    const auto thread = f.child(child, 0x10f00); // VM|FS|FILES|SIGHAND|THREAD.
    require(f.transfer(false, thread) == 8, "nonleader TID did not resolve its owner address space");
    f.machine->scheduler().stop_process(child);
    require(f.transfer(false, thread) == 8, "stopped live target must remain accessible");
    f.machine->scheduler().continue_process(child);
    f.call(93, {0}, thread);
    require(f.transfer(false, thread) == -3, "retained exited TID must not expose its old memory");
    f.machine->terminate(child, 9);
    require(f.transfer(false, child) == -3, "zombie PID must not expose retained image memory");
    ProcessWait wait;
    wait.kind = ProcessWait::Kind::Pid;
    wait.id = child;
    wait.options = 4;
    wait.caller_tid = 1;
    require(f.machine->wait(wait).has_value() && f.transfer(true, child) == -3,
            "reaped target must remain inaccessible");
}
void realcreds_dumpability_and_capabilities() {
    Fixture f;
    f.setup();
    const auto child = f.child();
    auto caller = process::pidfd_credentials(*f.machine, 1);
    auto target = process::pidfd_credentials(*f.machine, child);
    require(f.call(167, {4, 0}, child) == 0 && f.transfer(false, child) == -1 &&
                f.transfer(true, child) == -1,
            "PR_SET_DUMPABLE must revoke both read and write access");
    require(f.call(167, {4, 1}, child) == 0 && f.transfer(false, child) == 8,
            "restoring dumpability must restore permitted access");
    for (const auto field : {&process::PidfdCredentials::real_uid,
                             &process::PidfdCredentials::effective_uid,
                             &process::PidfdCredentials::saved_uid,
                             &process::PidfdCredentials::real_gid,
                             &process::PidfdCredentials::effective_gid,
                             &process::PidfdCredentials::saved_gid}) {
        auto denied = target;
        denied.*field = 2000;
        process::pidfd_update_credentials(*f.machine, child, denied);
        require(f.transfer(false, child) == -1 && f.transfer(true, child) == -1,
                "REALCREDS must match all three target UIDs and GIDs");
    }
    process::pidfd_update_credentials(*f.machine, child, target);
    caller.effective_uid = caller.saved_uid = 3000;
    caller.effective_gid = caller.saved_gid = 3000;
    process::pidfd_update_credentials(*f.machine, 1, caller);
    require(f.transfer(false, child) == 8, "permission check used caller effective rather than real IDs");
    target.permitted_capabilities = uint64_t{1} << 10;
    process::pidfd_update_credentials(*f.machine, child, target);
    require(f.transfer(false, child) == -1, "target permitted-capability superset must be denied");
    caller.permitted_capabilities = target.permitted_capabilities;
    process::pidfd_update_credentials(*f.machine, 1, caller);
    require(f.transfer(true, child) == 8, "matching permitted capabilities must allow access");
    target.dumpable = false;
    target.saved_gid = 2000;
    process::pidfd_update_credentials(*f.machine, child, target);
    caller.cap_kill = true;
    process::pidfd_update_credentials(*f.machine, 1, caller);
    require(f.transfer(false, child) == -1, "CAP_KILL must not imply CAP_SYS_PTRACE");
    caller.cap_sys_ptrace = true;
    process::pidfd_update_credentials(*f.machine, 1, caller);
    require(f.transfer(false, child) == 8 && f.transfer(true, child) == 8,
            "guest CAP_SYS_PTRACE override");
}
void same_mm_and_unavailable_authority() {
    Fixture f;
    f.setup();
    require(f.call(167, {4, 0}) == 0 && f.transfer(false) == 8, "self access must ignore dumpability");
    const auto thread = f.child(1, 0x10f00);
    require(f.transfer(true, thread) == 8, "same-thread-group introspection must ignore dumpability");
    const auto shared = f.child(1, 0x100); // CLONE_VM, separate thread group.
    require(f.transfer(false, shared) == 8, "mm_access same-mm bypass for CLONE_VM");
    const auto separate = f.child();
    f.machine->scheduler().set_process_controller(nullptr);
    const auto denied = f.transfer(true, separate);
    const auto self = f.transfer(false);
    f.machine->scheduler().set_process_controller(f.machine.get());
    require(denied == -1 && self == 8, "missing controller must fail closed for another process");
    process::pidfd_process_reaped(*f.machine, separate);
    require(f.transfer(false, separate) == -1, "unmodelled live-target credentials must fail closed");
}
} // namespace
int main() {
    try {
        self_and_scatter_gather();
        validation_and_empty_ordering();
        partial_transfers_and_memory_permissions();
        cross_process_tid_cow_and_lifetime();
        realcreds_dumpability_and_capabilities();
        same_mm_and_unavailable_authority();
        std::puts("process_vm ABI, partial transfers, guest identity, permissions and COW passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
