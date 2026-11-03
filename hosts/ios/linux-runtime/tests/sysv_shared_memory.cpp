#include "linux/ipc/sysv_shared_memory.h"
#include "linux/process/resource_limits.h"
#include "memory/address_space.h"
#include <array>
#include <condition_variable>
#include <cstdint>
#include <cstdio>
#include <functional>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

using namespace operit::arm64;
using namespace operit::arm64::ipc;

namespace {
constexpr uint32_t ipc_creat = 01000, ipc_excl = 02000;
constexpr uint32_t shm_rdonly = 010000, shm_rnd = 020000, shm_remap = 040000, shm_exec = 0100000;
constexpr uint64_t scratch = 0x10000;

struct GuestIpcPerm {
    int32_t key;
    uint32_t uid, gid, cuid, cgid, mode;
    uint16_t sequence, pad;
    uint64_t unused1, unused2;
};
struct GuestShmidDs {
    GuestIpcPerm perm;
    uint64_t size;
    int64_t atime, dtime, ctime;
    int32_t cpid, lpid;
    uint64_t nattch, unused1, unused2;
};
static_assert(sizeof(GuestShmidDs) == 112, "AArch64 shmid_ds");

void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}

template <class Function> int error_from(Function &&function) {
    try {
        function();
        return 0;
    } catch (const GuestError &error) {
        return error.number;
    } catch (const MemoryFault &) {
        return 14;
    }
}

struct Context {
    std::shared_ptr<SharedMemoryNamespace> ipc_namespace;
    std::shared_ptr<SharedMemoryAddressSpace> space;
    std::shared_ptr<Memory> memory;
    process::ResourceLimits limits;
    SharedMemoryCredentials credentials;
    uint32_t pid;
    uint64_t cursor = 0x1000000000;

    explicit Context(std::shared_ptr<SharedMemoryNamespace> ns = make_shared_memory_namespace(),
                     uint32_t process_id = 1)
        : ipc_namespace(std::move(ns)), space(make_shared_memory_address_space()),
          memory(std::make_shared<Memory>(32 << 20)), pid(process_id) {
        credentials.euid = 1000;
        credentials.egid = credentials.fsgid = 2000;
        memory->map(scratch, page_size * 2, read_perm | write_perm);
    }

    Context(Context &parent, uint32_t process_id, bool clone_vm)
        : ipc_namespace(parent.ipc_namespace),
          memory(clone_vm ? parent.memory : std::shared_ptr<Memory>(parent.memory->fork_clone())),
          credentials(parent.credentials), pid(process_id), cursor(parent.cursor) {
        space = clone_vm ? parent.space : fork_shared_memory_address_space(parent.space, *memory, pid);
    }

    ~Context() {
        release_shared_memory_address_space(space, pid);
    }

    int64_t call(uint64_t number, uint64_t a = 0, uint64_t b = 0, uint64_t c = 0) {
        try {
            return shared_memory_syscall(ipc_namespace, space, *memory, limits, credentials, pid, cursor,
                                         number, a, b, c);
        } catch (const GuestError &error) {
            return -error.number;
        } catch (const MemoryFault &) {
            return -14;
        }
    }
    int64_t get(int32_t key, uint64_t size, uint32_t flags) {
        return call(194, uint32_t(key), size, flags);
    }
    int64_t ctl(int32_t id, int32_t command, uint64_t address = 0) {
        return call(195, uint32_t(id), uint32_t(command), address);
    }
    int64_t attach(int32_t id, uint64_t address = 0, uint32_t flags = 0) {
        return call(196, uint32_t(id), address, flags);
    }
    int64_t detach(uint64_t address) {
        return call(197, address);
    }
    GuestShmidDs stat(int32_t id) {
        require(ctl(id, 2, scratch) == 0, "IPC_STAT failed");
        GuestShmidDs result{};
        memory->copy_out(scratch, &result, sizeof(result));
        return result;
    }
    int64_t set(int32_t id, GuestShmidDs value) {
        memory->copy_in(scratch, &value, sizeof(value));
        return ctl(id, 1, scratch);
    }
    void changed() {
        shared_memory_mappings_changed(space, *memory, pid);
    }
    void unmap(uint64_t address, uint64_t size) {
        const Memory::AccessGuard access(*memory);
        memory->unmap(address, size);
        changed();
    }
    void protect(uint64_t address, uint64_t size, unsigned permissions) {
        const Memory::AccessGuard access(*memory);
        memory->protect(address, size, permissions);
        changed();
    }
    void advise(uint64_t address, uint64_t size, int advice) {
        const Memory::AccessGuard access(*memory);
        memory->advise(address, size, advice);
        changed();
    }
};

void creation_and_namespace_isolation() {
    Context first;
    Context second;
    const int64_t id = first.get(0x12345, 5000, ipc_creat | ipc_excl | 0640);
    require(id >= 0, "keyed creation failed");
    require(first.get(0x12345, 0, 0) == id, "key lookup failed");
    require(first.get(0x12345, 5001, 0) == -22, "oversized existing segment accepted");
    require(first.get(0x12345, 1, ipc_creat | ipc_excl | 0600) == -17, "IPC_EXCL ignored");
    require(second.get(0x12345, 0, 0) == -2, "key leaked between IPC namespaces");
    require(second.ctl(int32_t(id), 2, scratch) == -22, "ID leaked into empty IPC namespace");
    const int64_t other = second.get(0x12345, 5000, ipc_creat | 0600);
    require(other >= 0, "independent namespace create failed");
    const int64_t left = first.attach(int32_t(id));
    const int64_t right = second.attach(int32_t(other));
    require(left > 0 && right > 0, "namespace attachment failed");
    first.memory->store(uint64_t(left), 0x12345678, 4);
    require(second.memory->load(uint64_t(right), 4) == 0, "different namespaces share backing bytes");
    const int64_t alias = first.attach(int32_t(id));
    require(alias > 0, "second attachment to the same segment failed");
    first.memory->store(uint64_t(left) + page_size * 2 - 1, 0x5a, 1);
    require(first.memory->load(uint64_t(alias) + page_size * 2 - 1, 1) == 0x5a,
            "last partial segment page was not shared");
    require(first.detach(uint64_t(alias)) == 0, "second attachment detach failed");
    auto info = first.stat(int32_t(id));
    require(info.perm.key == 0x12345 && info.size == 5000 && info.perm.uid == 1000 && info.perm.gid == 2000 &&
                (info.perm.mode & 0777) == 0640 && info.cpid == 1 && info.lpid == 1 && info.nattch == 1 &&
                info.ctime > 0 && info.atime > 0,
            "IPC_STAT returned incorrect ABI fields");
    require(first.detach(uint64_t(left)) == 0 && second.detach(uint64_t(right)) == 0,
            "namespace detach failed");
    require(first.ctl(int32_t(id), 0) == 0 && second.ctl(int32_t(other), 0) == 0, "namespace removal failed");
    require(first.ctl(int32_t(id), 2, scratch) == -22, "unattached removed ID survived");
    const int64_t private_a = first.get(0, 1, 0600), private_b = first.get(0, 1, 0600);
    require(private_a >= 0 && private_b >= 0 && private_a != private_b, "IPC_PRIVATE reused a segment");
    require(first.ctl(int32_t(private_a), 0) == 0 && first.ctl(int32_t(private_b), 0) == 0,
            "private removal failed");
}

void namespace_change_with_inherited_mapping() {
    auto old_namespace = make_shared_memory_namespace();
    auto new_namespace = make_shared_memory_namespace();
    Context observer(old_namespace, 30), parent(old_namespace, 31);
    const int64_t id = parent.get(321, page_size, ipc_creat | 0600);
    const int64_t address = parent.attach(int32_t(id));
    require(id >= 0 && address > 0, "namespace transition fixture creation failed");
    parent.memory->store(uint64_t(address), 0x13579, 4);
    {
        Context child(parent, 32, false);
        child.ipc_namespace = new_namespace;
        require(child.get(321, 0, 0) == -2 && child.ctl(int32_t(id), 2, scratch) == -22,
                "namespace transition exposed the old key or ID");
        require(child.memory->load(uint64_t(address), 4) == 0x13579 && observer.stat(int32_t(id)).nattch == 2,
                "namespace transition lost the inherited old-namespace attachment");
        require(child.detach(uint64_t(address)) == 0 && observer.stat(int32_t(id)).nattch == 1,
                "detach after namespace change charged the wrong namespace");
    }
    require(parent.ctl(int32_t(id), 0) == 0 && parent.detach(uint64_t(address)) == 0,
            "namespace transition cleanup failed");
    require(observer.ctl(int32_t(id), 2, scratch) == -22,
            "namespace transition retained the removed old segment");
}

void stat_commands_and_permissions() {
    auto ns = make_shared_memory_namespace();
    Context owner(ns, 40), outsider(ns, 41);
    outsider.credentials.euid = 6000;
    outsider.credentials.egid = outsider.credentials.fsgid = 6000;
    const int64_t id = owner.get(88, page_size, ipc_creat | 0000);
    require(id >= 0, "stat fixture creation failed");
    const uint32_t index = uint32_t(id) & 0x7fff;
    require(outsider.ctl(int32_t(id), 2, scratch) == -13 && outsider.ctl(int32_t(index), 13, scratch) == -13,
            "IPC_STAT or SHM_STAT bypassed read permission");
    require(outsider.ctl(int32_t(index), 15, scratch) == id, "SHM_STAT_ANY did not return the real ID");
    GuestShmidDs value{};
    outsider.memory->copy_out(scratch, &value, sizeof(value));
    require(value.perm.key == 88 && value.size == page_size && value.nattch == 0,
            "SHM_STAT_ANY returned incorrect fields");
    require(outsider.ctl(0, 3, scratch) >= 0, "IPC_INFO failed");
    require(outsider.ctl(0, 3, 0) == -14, "IPC_INFO invalid destination did not return EFAULT");
    require(owner.ctl(int32_t(id), 0) == 0, "stat fixture cleanup failed");
}

void removal_and_id_reuse() {
    SharedMemoryLimits limits;
    limits.max_segments = 1;
    auto ns = make_shared_memory_namespace(limits);
    Context process(ns);
    const int64_t id = process.get(42, page_size, ipc_creat | 0600);
    require(id >= 0, "single-slot segment creation failed");
    const int64_t first = process.attach(int32_t(id));
    require(first > 0, "initial attachment failed");
    require(process.ctl(int32_t(id), 0) == 0, "IPC_RMID attached segment failed");
    require(process.get(42, 0, 0) == -2, "IPC_RMID left the key discoverable");
    auto marked = process.stat(int32_t(id));
    require(marked.perm.key == 0 && (marked.perm.mode & 01000) && marked.nattch == 1,
            "IPC_RMID state not visible while attached");
    require(process.get(42, page_size, ipc_creat | 0600) == -28, "marked segment did not consume its slot");
    const int64_t second = process.attach(int32_t(id));
    require(second > 0 && process.stat(int32_t(id)).nattch == 2,
            "attachment to an IPC_RMID-marked live ID failed");
    process.memory->store(uint64_t(first), 0xabcdef, 4);
    require(process.memory->load(uint64_t(second), 4) == 0xabcdef, "removed segment lost shared bytes");
    require(process.detach(uint64_t(first)) == 0 && process.stat(int32_t(id)).nattch == 1,
            "first detach destroyed a still-attached segment");
    require(process.detach(uint64_t(second)) == 0, "last detach failed");
    require(process.ctl(int32_t(id), 2, scratch) == -22, "last detach did not destroy segment");
    const int64_t replacement = process.get(42, page_size, ipc_creat | 0600);
    require(replacement >= 0 && replacement != id, "reused slot did not change the ID sequence");
    require(process.attach(int32_t(id)) == -22, "stale ID accessed the replacement segment");
    require(process.ctl(int32_t(replacement), 0) == 0, "replacement removal failed");
}

void permissions_and_capabilities() {
    auto ns = make_shared_memory_namespace();
    Context owner(ns, 11), group(ns, 12), stranger(ns, 13);
    group.credentials.euid = 1001;
    group.credentials.egid = 3000;
    group.credentials.fsgid = 3001;
    group.credentials.groups = {2000};
    stranger.credentials.euid = 1002;
    stranger.credentials.egid = stranger.credentials.fsgid = 3000;
    const int64_t id = owner.get(77, page_size, ipc_creat | 0640);
    require(id >= 0, "permission fixture creation failed");
    require(group.get(77, 0, 0400) == id && group.get(77, 0, 0200) == -13,
            "shmget group permission check incorrect");
    require(group.ctl(int32_t(id), 2, scratch) == 0 && stranger.ctl(int32_t(id), 2, scratch) == -13,
            "IPC_STAT read permission check incorrect");
    require(stranger.attach(int32_t(id), 0, shm_rdonly) == -13 && group.attach(int32_t(id)) == -13,
            "shmat bypassed read/write permissions");
    const int64_t readonly = group.attach(int32_t(id), 0, shm_rdonly);
    require(readonly > 0, "supplementary group read-only attach failed");
    require(error_from(
                [&] { group.memory->protect(uint64_t(readonly), page_size, read_perm | write_perm); }) == 13,
            "read-only attachment gained write access through mprotect");
    require(group.detach(uint64_t(readonly)) == 0, "read-only detach failed");
    group.credentials.groups.clear();
    group.credentials.fsgid = 2000;
    const int64_t fsgid_address = group.attach(int32_t(id), 0, shm_rdonly);
    require(fsgid_address > 0, "filesystem group permission ignored");
    require(group.detach(uint64_t(fsgid_address)) == 0, "filesystem group detach failed");
    require(stranger.ctl(int32_t(id), 0) == -1, "non-owner removed segment");
    stranger.credentials.euid = 0;
    require(stranger.ctl(int32_t(id), 0) == -1, "UID zero incorrectly implied CAP_SYS_ADMIN");
    stranger.credentials.cap_ipc_owner = true;
    const int64_t privileged = stranger.attach(int32_t(id));
    require(privileged > 0, "CAP_IPC_OWNER did not override access bits");
    require(stranger.ctl(int32_t(id), 0) == -1, "CAP_IPC_OWNER incorrectly implied CAP_SYS_ADMIN");
    require(stranger.detach(uint64_t(privileged)) == 0, "capability attachment detach failed");
    auto values = owner.stat(int32_t(id));
    values.perm.uid = 4321;
    values.perm.gid = 5432;
    values.perm.mode = 0600;
    require(owner.set(int32_t(id), values) == 0, "creator IPC_SET failed");
    auto changed = owner.stat(int32_t(id));
    require(changed.perm.uid == 4321 && changed.perm.gid == 5432 && changed.perm.cuid == 1000 &&
                changed.perm.cgid == 2000 && (changed.perm.mode & 0777) == 0600,
            "IPC_SET changed wrong permission fields");
    require(owner.attach(int32_t(id), 0, shm_exec) == -13,
            "SHM_EXEC ignored the segment's execute permission");
    changed.perm.mode = 0700;
    require(owner.set(int32_t(id), changed) == 0, "could not grant execute permission");
    const int64_t executable = owner.attach(int32_t(id), 0, shm_rdonly | shm_exec);
    require(executable > 0 && owner.memory->accessible(uint64_t(executable), 1, read_perm | exec_perm) &&
                !owner.memory->accessible(uint64_t(executable), 1, write_perm),
            "SHM_EXEC | SHM_RDONLY produced incorrect guest permissions");
    require(owner.detach(uint64_t(executable)) == 0, "executable attachment detach failed");
    require(stranger.ctl(int32_t(id), 0) == -1, "permission change granted unrelated control");
    stranger.credentials.cap_sys_admin = true;
    require(stranger.ctl(int32_t(id), 0) == 0, "CAP_SYS_ADMIN could not remove segment");
}

void split_unmap_and_remap_guards() {
    Context process;
    const int64_t id = process.get(0, page_size * 3, 0600);
    const int64_t address = process.attach(int32_t(id));
    require(id >= 0 && address > 0, "split fixture creation failed");
    const uint64_t base = uint64_t(address);
    process.protect(base + page_size, page_size, read_perm);
    require(process.stat(int32_t(id)).nattch == 3, "mprotect VMA split did not adjust nattch");
    process.protect(base + page_size, page_size, read_perm | write_perm);
    require(process.stat(int32_t(id)).nattch == 1, "mprotect VMA merge did not adjust nattch");
    require(error_from([&] {
                shared_memory_check_remap(process.space, *process.memory, base, page_size, 0, page_size, 1);
            }) == 95,
            "shared attachment mremap was not rejected");
    require(error_from([&] {
                shared_memory_check_remap(process.space, *process.memory, 0x30000, page_size,
                                          base + page_size, page_size, 3);
            }) == 95,
            "fixed mremap onto attachment was not rejected");
    process.unmap(base, page_size);
    require(process.stat(int32_t(id)).nattch == 1, "partial munmap destroyed attachment");
    {
        const Memory::AccessGuard access(*process.memory);
        process.memory->map(base + page_size, page_size, read_perm | write_perm, true);
        process.changed();
    }
    process.memory->store(base + page_size, 0x76543210, 4);
    require(process.ctl(int32_t(id), 0) == 0, "partial attachment IPC_RMID failed");
    require(process.detach(base) == 0, "shmdt failed after leading and middle fragments were replaced");
    require(process.memory->load(base + page_size, 4) == 0x76543210,
            "shmdt removed an unrelated replacement mapping");
    require(!process.memory->accessible(base + 2 * page_size, 1, 0), "shmdt left a trailing fragment");
    require(process.ctl(int32_t(id), 2, scratch) == -22, "final split detach retained removed ID");
    require(process.detach(base) == -22, "second shmdt unexpectedly succeeded");
}

void fixed_mapping_replacement_releases_removed_segment() {
    auto ns = make_shared_memory_namespace();
    Context observer(ns, 50), process(ns, 51);
    const int64_t id = process.get(0, page_size * 2, 0600);
    const int64_t address = process.attach(int32_t(id));
    require(id >= 0 && address > 0 && process.ctl(int32_t(id), 0) == 0,
            "fixed replacement fixture setup failed");
    {
        const Memory::AccessGuard access(*process.memory);
        process.memory->map(uint64_t(address), page_size * 2, read_perm | write_perm, true);
        process.changed();
    }
    process.memory->store(uint64_t(address), 0xabcdef, 4);
    require(observer.ctl(int32_t(id), 2, scratch) == -22,
            "full fixed replacement did not release a removed segment");
    require(process.detach(uint64_t(address)) == -22 &&
                process.memory->load(uint64_t(address), 4) == 0xabcdef,
            "shmdt affected the replacement mapping");
}

void fork_clone_vm_exec_and_exit() {
    auto ns = make_shared_memory_namespace();
    Context observer(ns, 99), parent(ns, 20);
    const int64_t id = parent.get(0, page_size * 2, 0600);
    const int64_t address = parent.attach(int32_t(id));
    require(id >= 0 && address > 0, "fork fixture creation failed");
    const uint64_t base = uint64_t(address);
    parent.memory->store(base, 0x1234, 4);
    {
        Context child(parent, 21, false);
        require(observer.stat(int32_t(id)).nattch == 2, "fork did not increment nattch");
        require(child.memory->load(base, 4) == 0x1234, "fork lost shared bytes");
        child.memory->store(base, 0x5678, 4);
        require(parent.memory->load(base, 4) == 0x5678, "fork attachment was copied instead of shared");
        require(parent.ctl(int32_t(id), 0) == 0, "fork IPC_RMID failed");
        require(child.detach(base) == 0 && observer.stat(int32_t(id)).nattch == 1,
                "child detach affected parent's forked attachment");
    }
    {
        Context peer(parent, 22, true);
        require(observer.stat(int32_t(id)).nattch == 1, "CLONE_VM added a second attachment");
        release_shared_memory_address_space(parent.space, parent.pid);
        parent.space = make_shared_memory_address_space(); // committed exec gets a fresh mm table
        parent.memory = std::make_shared<Memory>(32 << 20);
        parent.memory->map(scratch, page_size * 2, read_perm | write_perm);
        require(!parent.memory->accessible(base, 1, 0) && parent.stat(int32_t(id)).nattch == 1,
                "exec failed to replace Memory while retaining the IPC namespace");
        require(observer.stat(int32_t(id)).nattch == 1 && peer.memory->load(base, 4) == 0x5678,
                "exec detached an address space still held by CLONE_VM");
        release_shared_memory_address_space(peer.space, peer.pid); // final mm owner exits
        require(observer.ctl(int32_t(id), 2, scratch) == -22,
                "last CLONE_VM exit did not release the removed segment");
    }
}

void process_exit_detaches_but_keeps_unremoved_segment() {
    auto ns = make_shared_memory_namespace();
    Context observer(ns, 70);
    int64_t id = -1;
    {
        Context parent(ns, 71);
        id = parent.get(0, page_size, 0600);
        const int64_t address = parent.attach(int32_t(id));
        require(id >= 0 && address > 0, "exit fixture setup failed");
        {
            Context child(parent, 72, false);
            require(observer.stat(int32_t(id)).nattch == 2, "exit fixture fork did not attach");
        }
        auto after_child = observer.stat(int32_t(id));
        require(after_child.nattch == 1 && after_child.lpid == 72 && after_child.dtime > 0,
                "child exit did not detach its inherited mapping");
    }
    auto after_parent = observer.stat(int32_t(id));
    require(after_parent.nattch == 0 && after_parent.lpid == 71,
            "last exit did not detach or incorrectly removed a live segment");
    const int64_t new_address = observer.attach(int32_t(id));
    require(new_address > 0 && observer.detach(uint64_t(new_address)) == 0,
            "unremoved segment could not be reattached after all processes exited");
    require(observer.ctl(int32_t(id), 0) == 0, "exit fixture cleanup failed");
}

void dontfork_fragments() {
    auto ns = make_shared_memory_namespace();
    Context observer(ns, 80), parent(ns, 81);
    const int64_t id = parent.get(0, page_size * 2, 0600);
    const int64_t address = parent.attach(int32_t(id));
    require(id >= 0 && address > 0, "DONTFORK fixture creation failed");
    const uint64_t base = uint64_t(address);
    parent.advise(base, page_size, 10);
    require(observer.stat(int32_t(id)).nattch == 2, "DONTFORK did not split the parent mapping");
    {
        Context child(parent, 82, false);
        require(!child.memory->accessible(base, 1, 0) && child.memory->accessible(base + page_size, 1, 0),
                "fork did not honor DONTFORK fragment");
        require(observer.stat(int32_t(id)).nattch == 3, "fork counted a DONTFORK fragment");
        require(child.detach(base) == 0 && observer.stat(int32_t(id)).nattch == 2,
                "child could not detach surviving fragment by original address");
    }
    require(parent.ctl(int32_t(id), 0) == 0 && parent.detach(base) == 0, "DONTFORK fixture cleanup failed");
    require(observer.ctl(int32_t(id), 2, scratch) == -22, "DONTFORK fixture retained removed segment");
}

void error_paths_and_quotas() {
    SharedMemoryLimits configured;
    configured.max_segments = 2;
    configured.max_segment_size = page_size * 2;
    configured.max_total_pages = 2;
    Context process(make_shared_memory_namespace(configured));
    require(process.get(0, 0, 0600) == -22, "zero-sized new segment accepted");
    require(process.get(0, page_size * 2 + 1, 0600) == -22, "oversized segment accepted");
    require(process.get(10, 1, 0) == -2, "missing key without IPC_CREAT accepted");
    require(process.get(0, page_size, 04000 | 0600) == -95, "unsupported huge pages silently accepted");
    const int64_t id = process.get(0, page_size * 2, 0600);
    require(id >= 0 && process.get(0, 1, 0600) == -28, "namespace page quota not enforced");
    require(process.ctl(int32_t(id), 2, 0) == -14 && process.ctl(int32_t(id), 1, 0) == -14,
            "invalid shmctl buffer did not return EFAULT");
    require(process.stat(int32_t(id)).size == page_size * 2, "failed IPC_SET mutated segment");
    require(process.attach(-1) == -22 && process.detach(1) == -22, "negative ID or unaligned shmdt accepted");
    require(process.attach(int32_t(id), 0x40001) == -22, "unaligned shmat accepted without SHM_RND");
    require(process.attach(int32_t(id), 0x10000) == -22, "shmat replaced an existing mapping");
    require(process.attach(int32_t(id), 0, shm_remap) == -22 &&
                process.attach(int32_t(id), 0x40000, shm_remap) == -95,
            "unsupported SHM_REMAP did not fail explicitly");
    const int64_t rounded = process.attach(int32_t(id), 0x40073, shm_rnd);
    require(rounded == 0x40000, "SHM_RND did not align the requested guest address");
    require(process.detach(uint64_t(rounded)) == 0, "rounded attachment detach failed");
    require(process.ctl(int32_t(id), 11, scratch) == -95 && process.ctl(int32_t(id), 14, scratch) == -95,
            "unsupported lock or SHM_INFO command reported success");
    require(process.ctl(int32_t(id), 0) == 0, "quota fixture cleanup failed");
    require(process.get(0, 1, 0600) >= 0, "removed segment did not release its quota");

    Context limited;
    const int64_t limited_id = limited.get(0, page_size * 2, 0600);
    limited.limits.set(process::Resource::AddressSpace, {page_size * 3, page_size * 3});
    require(limited.attach(int32_t(limited_id)) == -12, "shmat ignored RLIMIT_AS");
    require(limited.stat(int32_t(limited_id)).nattch == 0, "failed shmat changed nattch");
    require(limited.ctl(int32_t(limited_id), 0) == 0, "limited fixture cleanup failed");
}

class Barrier {
    std::mutex mutex_;
    std::condition_variable changed_;
    const unsigned participants_;
    unsigned waiting_ = 0, generation_ = 0;

  public:
    explicit Barrier(unsigned participants) : participants_(participants) {}
    void wait() {
        std::unique_lock<std::mutex> lock(mutex_);
        const unsigned generation = generation_;
        if (++waiting_ == participants_) {
            waiting_ = 0;
            ++generation_;
            changed_.notify_all();
        } else {
            changed_.wait(lock, [&] { return generation_ != generation; });
        }
    }
};

void concurrent_attach_write_remove_detach() {
    constexpr unsigned count = 6;
    auto ns = make_shared_memory_namespace();
    Context owner(ns, 1);
    const int64_t id = owner.get(9876, page_size, ipc_creat | 0600);
    require(id >= 0, "concurrent fixture creation failed");
    Barrier barrier(count);
    std::vector<std::string> failures(count);
    std::vector<std::thread> threads;
    threads.reserve(count);
    for (unsigned index = 0; index < count; ++index) {
        threads.emplace_back([&, index] {
            Context process(ns, index + 10);
            auto check = [&](bool condition, const char *message) {
                if (!condition && failures[index].empty())
                    failures[index] = message;
            };
            auto run = [&](const auto &function) {
                try {
                    function();
                } catch (const std::exception &error) {
                    if (failures[index].empty())
                        failures[index] = error.what();
                } catch (const GuestError &error) {
                    if (failures[index].empty())
                        failures[index] = "unexpected guest errno " + std::to_string(error.number);
                }
            };
            const int64_t address = process.attach(int32_t(id));
            check(address > 0, "concurrent shmat failed");
            barrier.wait();
            run([&] {
                if (address > 0) {
                    process.memory->store(uint64_t(address) + index * 8, uint64_t(index) + 0x100, 8);
                    check(process.stat(int32_t(id)).nattch == count, "concurrent nattch incorrect");
                }
            });
            barrier.wait();
            run([&] {
                if (address > 0)
                    for (unsigned other = 0; other < count; ++other)
                        check(process.memory->load(uint64_t(address) + other * 8, 8) ==
                                  uint64_t(other) + 0x100,
                              "concurrent mappings did not share writes");
            });
            if (!index)
                check(process.ctl(int32_t(id), 0) == 0, "concurrent IPC_RMID failed");
            barrier.wait();
            check(process.get(9876, 0, 0) == -2, "removed concurrent key remained discoverable");
            const int64_t second = process.attach(int32_t(id));
            check(second > 0, "concurrent attach to marked ID failed");
            barrier.wait();
            if (second > 0)
                check(process.detach(uint64_t(second)) == 0, "second concurrent detach failed");
            if (address > 0)
                check(process.detach(uint64_t(address)) == 0, "first concurrent detach failed");
            barrier.wait();
        });
    }
    for (auto &thread : threads)
        thread.join();
    for (const auto &failure : failures)
        require(failure.empty(), failure.c_str());
    require(owner.ctl(int32_t(id), 2, scratch) == -22, "concurrent last detach retained removed ID");
}

void concurrent_create_same_key() {
    constexpr unsigned count = 8;
    auto ns = make_shared_memory_namespace();
    Barrier start(count), created(count);
    std::array<int64_t, count> results{}, observed{};
    std::vector<std::thread> threads;
    for (unsigned index = 0; index < count; ++index) {
        threads.emplace_back([&, index] {
            Context process(ns, index + 100);
            start.wait();
            results[index] = process.get(1234567, page_size, ipc_creat | ipc_excl | 0600);
            created.wait();
            observed[index] = process.get(1234567, 0, 0);
        });
    }
    for (auto &thread : threads)
        thread.join();
    int winners = 0;
    int64_t winner = -1;
    for (const int64_t result : results) {
        if (result >= 0) {
            ++winners;
            winner = result;
        } else {
            require(result == -17, "concurrent IPC_EXCL returned an unexpected error");
        }
    }
    require(winners == 1, "concurrent IPC_EXCL created more or fewer than one segment");
    for (const int64_t result : observed)
        require(result == winner, "concurrent lookup saw inconsistent IDs");
    Context cleaner(ns);
    require(cleaner.ctl(int32_t(winner), 0) == 0, "concurrent creation cleanup failed");
}

void concurrent_remove_versus_first_attach() {
    auto ns = make_shared_memory_namespace();
    Context owner(ns, 200), attaching(ns, 201);
    for (unsigned iteration = 0; iteration < 64; ++iteration) {
        const int64_t id = owner.get(0, page_size, 0600);
        require(id >= 0, "remove/attach race creation failed");
        Barrier start(2);
        int64_t attach_result = -999, remove_result = -999;
        std::thread attach_thread([&] {
            start.wait();
            attach_result = attaching.attach(int32_t(id));
        });
        std::thread remove_thread([&] {
            start.wait();
            remove_result = owner.ctl(int32_t(id), 0);
        });
        attach_thread.join();
        remove_thread.join();
        require(remove_result == 0, "racing IPC_RMID failed");
        require(attach_result > 0 || attach_result == -22, "racing shmat returned an unexpected result");
        if (attach_result > 0)
            require(attaching.detach(uint64_t(attach_result)) == 0, "racing attachment could not detach");
        require(owner.ctl(int32_t(id), 2, scratch) == -22, "racing removal left a live segment");
    }
}
} // namespace

int main() {
    const std::vector<std::pair<const char *, std::function<void()>>> tests = {
        {"creation_and_namespace_isolation", creation_and_namespace_isolation},
        {"namespace_change_with_inherited_mapping", namespace_change_with_inherited_mapping},
        {"stat_commands_and_permissions", stat_commands_and_permissions},
        {"removal_and_id_reuse", removal_and_id_reuse},
        {"permissions_and_capabilities", permissions_and_capabilities},
        {"split_unmap_and_remap_guards", split_unmap_and_remap_guards},
        {"fixed_mapping_replacement_releases_removed_segment",
         fixed_mapping_replacement_releases_removed_segment},
        {"fork_clone_vm_exec_and_exit", fork_clone_vm_exec_and_exit},
        {"process_exit_detaches_but_keeps_unremoved_segment",
         process_exit_detaches_but_keeps_unremoved_segment},
        {"dontfork_fragments", dontfork_fragments},
        {"error_paths_and_quotas", error_paths_and_quotas},
        {"concurrent_attach_write_remove_detach", concurrent_attach_write_remove_detach},
        {"concurrent_create_same_key", concurrent_create_same_key},
        {"concurrent_remove_versus_first_attach", concurrent_remove_versus_first_attach},
    };
    unsigned failed = 0;
    for (const auto &test : tests) {
        try {
            test.second();
            std::printf("PASS %s\n", test.first);
        } catch (const std::exception &error) {
            ++failed;
            std::fprintf(stderr, "FAIL %s: %s\n", test.first, error.what());
        } catch (const GuestError &error) {
            ++failed;
            std::fprintf(stderr, "FAIL %s: unexpected guest errno %d\n", test.first, error.number);
        }
    }
    return failed ? 1 : 0;
}
