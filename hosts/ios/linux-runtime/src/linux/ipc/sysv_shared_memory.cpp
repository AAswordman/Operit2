#include "linux/ipc/sysv_shared_memory.h"
#include "base/fd.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/process/resource_limits.h"
#include "memory/address_space.h"
#include <algorithm>
#include <cerrno>
#include <climits>
#include <cstdio>
#include <cstdlib>
#include <ctime>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <string>
#include <sys/stat.h>
#include <unistd.h>
#include <utility>
#include <vector>

namespace operit::arm64::ipc {
namespace {
constexpr uint32_t ipc_creat = 01000, ipc_excl = 02000;
constexpr uint32_t shm_hugetlb = 04000, shm_noreserve = 010000;
constexpr uint32_t shm_rdonly = 010000, shm_rnd = 020000, shm_remap = 040000, shm_exec = 0100000;
constexpr uint32_t shm_dest = 01000;
constexpr uint32_t index_bits = 15, index_mask = (1u << index_bits) - 1;

// Linux asm-generic ipc64_perm and shmid64_ds on AArch64.
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
struct GuestShminfo {
    uint64_t max, min, mni, seg, all, unused[4];
};
static_assert(sizeof(GuestIpcPerm) == 48 && sizeof(GuestShmidDs) == 112 &&
              sizeof(GuestShminfo) == 72, "AArch64 System V shared-memory ABI");

int64_t now() noexcept {
    return int64_t(std::time(nullptr));
}

FD create_backing(uint64_t size) {
    const char *configured = std::getenv("TMPDIR");
    std::string path = configured && *configured ? configured : "/tmp";
    if (path.back() != '/')
        path += '/';
    path += "operit-sysv-shm-XXXXXX";
    std::vector<char> name(path.begin(), path.end());
    name.push_back('\0');
    FD result(::mkstemp(name.data()));
    check_host(result.get());
    check_host(::unlink(name.data()));
    check_host(::fcntl(result.get(), F_SETFD, FD_CLOEXEC));
    check_host(::ftruncate(result.get(), off_t(size)));
    return result;
}

struct Segment {
    FD backing;
    uint64_t device = 0, inode = 0;
    uint64_t size = 0, mapped_size = 0, nattch = 0;
    int32_t key = 0, id = 0;
    uint32_t uid = 0, gid = 0, cuid = 0, cgid = 0, mode = 0;
    uint16_t sequence = 0;
    uint32_t cpid = 0, lpid = 0;
    int64_t atime = 0, dtime = 0, ctime = 0;
    bool removed = false;
    std::string path;
};

bool member_of(const SharedMemoryCredentials &credentials, uint32_t gid) {
    return credentials.fsgid == gid ||
           std::find(credentials.groups.begin(), credentials.groups.end(), gid) != credentials.groups.end();
}

void check_access(const Segment &segment, const SharedMemoryCredentials &credentials, uint32_t mode) {
    const uint32_t requested = (mode | (mode >> 3) | (mode >> 6)) & 7;
    uint32_t granted = segment.mode;
    if (credentials.euid == segment.uid || credentials.euid == segment.cuid)
        granted >>= 6;
    else if (member_of(credentials, segment.gid) || member_of(credentials, segment.cgid))
        granted >>= 3;
    if ((requested & ~granted) && !credentials.cap_ipc_owner)
        guest_error(13);
}

void check_control(const Segment &segment, const SharedMemoryCredentials &credentials) {
    if (credentials.euid != segment.uid && credentials.euid != segment.cuid && !credentials.cap_sys_admin)
        guest_error(1);
}

bool intersects(uint64_t first, uint64_t length, uint64_t other, uint64_t other_length) {
    if (!length || !other_length)
        return false;
    return first <= other ? other - first < length : first - other < other_length;
}
} // namespace

class SharedMemoryNamespace {
  public:
    const SharedMemoryLimits limits;
    std::mutex mutex;
    std::vector<std::shared_ptr<Segment>> slots;
    std::vector<uint16_t> sequences;
    std::vector<bool> previously_used;
    std::map<int32_t, uint32_t> keys;
    uint64_t total_pages = 0;
    uint32_t next_slot = 0;

    explicit SharedMemoryNamespace(const SharedMemoryLimits &value)
        : limits(value), slots(value.max_segments), sequences(value.max_segments),
          previously_used(value.max_segments) {}

    std::shared_ptr<Segment> lookup(int32_t id) const {
        if (id < 0)
            guest_error(22);
        const uint32_t slot = uint32_t(id) & index_mask;
        if (slot >= slots.size() || !slots[slot] || slots[slot]->id != id)
            guest_error(22);
        return slots[slot];
    }
    std::shared_ptr<Segment> lookup_index(int32_t index) const {
        if (index < 0)
            guest_error(22);
        const uint32_t slot = uint32_t(index) & index_mask;
        if (slot >= slots.size() || !slots[slot])
            guest_error(22);
        return slots[slot];
    }
    int max_index() const noexcept {
        for (size_t index = slots.size(); index; --index)
            if (slots[index - 1])
                return int(index - 1);
        return 0;
    }
    void destroy_if_unused(const std::shared_ptr<Segment> &segment) noexcept {
        if (!segment->removed || segment->nattch)
            return;
        const uint32_t slot = uint32_t(segment->id) & index_mask;
        if (slot < slots.size() && slots[slot] == segment) {
            total_pages -= segment->mapped_size / page_size;
            slots[slot].reset();
        }
    }
};

namespace {
struct Attachment {
    std::shared_ptr<SharedMemoryNamespace> ipc_namespace;
    std::shared_ptr<Segment> segment;
    uint64_t address = 0, length = 0, fragments = 0;
};

bool matches(const Attachment &attachment, const Memory::Mapping &mapping) {
    return mapping.backing.shared && mapping.backing.device == attachment.segment->device &&
           mapping.backing.inode == attachment.segment->inode && mapping.start >= attachment.address &&
           mapping.end <= attachment.address + attachment.length &&
           mapping.backing.offset == mapping.start - attachment.address;
}

uint64_t fragments(const Attachment &attachment, const std::vector<Memory::Mapping> &mappings) {
    return uint64_t(std::count_if(mappings.begin(), mappings.end(), [&](const Memory::Mapping &mapping) {
        return matches(attachment, mapping);
    }));
}

// Caller holds the owning namespace mutex. Each actual guest VMA opens or
// closes the segment independently when protection/advice splits or joins it.
void change_fragments(Attachment &attachment, uint64_t count, uint32_t pid) noexcept {
    auto &segment = *attachment.segment;
    if (count > attachment.fragments) {
        segment.nattch += count - attachment.fragments;
        segment.atime = now();
        segment.lpid = pid;
    } else if (count < attachment.fragments) {
        segment.nattch -= attachment.fragments - count;
        segment.dtime = now();
        segment.lpid = pid;
    }
    attachment.fragments = count;
    attachment.ipc_namespace->destroy_if_unused(attachment.segment);
}
} // namespace

class SharedMemoryAddressSpace {
  public:
    std::mutex mutex;
    std::vector<Attachment> attachments;
    uint32_t last_pid = 0;

    ~SharedMemoryAddressSpace() {
        for (auto &attachment : attachments) {
            std::lock_guard<std::mutex> lock(attachment.ipc_namespace->mutex);
            change_fragments(attachment, 0, last_pid);
        }
    }
};

std::shared_ptr<SharedMemoryNamespace> make_shared_memory_namespace(const SharedMemoryLimits &limits) {
    if (!limits.max_segments || limits.max_segments > index_mask + 1 || !limits.max_segment_size ||
        limits.max_segment_size >= address_limit - page_size || limits.max_segment_size > uint64_t(INT64_MAX))
        guest_error(22);
    return std::make_shared<SharedMemoryNamespace>(limits);
}

std::shared_ptr<SharedMemoryAddressSpace> make_shared_memory_address_space() {
    return std::make_shared<SharedMemoryAddressSpace>();
}

void shared_memory_mappings_changed(const std::shared_ptr<SharedMemoryAddressSpace> &space,
                                    Memory &memory, uint32_t pid) {
    if (!space)
        guest_error(22);
    const Memory::AccessGuard access(memory);
    const auto mappings = memory.mappings();
    std::lock_guard<std::mutex> lock(space->mutex);
    space->last_pid = pid;
    for (auto it = space->attachments.begin(); it != space->attachments.end();) {
        auto &attachment = *it;
        const uint64_t count = fragments(attachment, mappings);
        {
            std::lock_guard<std::mutex> namespace_lock(attachment.ipc_namespace->mutex);
            change_fragments(attachment, count, pid);
        }
        if (!count)
            it = space->attachments.erase(it);
        else
            ++it;
    }
}

std::shared_ptr<SharedMemoryAddressSpace> fork_shared_memory_address_space(
    const std::shared_ptr<SharedMemoryAddressSpace> &parent, Memory &child_memory, uint32_t child_pid) {
    if (!parent)
        guest_error(22);
    const Memory::AccessGuard access(child_memory);
    const auto mappings = child_memory.mappings();
    auto child = make_shared_memory_address_space();
    child->last_pid = child_pid;
    std::lock_guard<std::mutex> lock(parent->mutex);
    child->attachments.reserve(parent->attachments.size());
    for (const auto &inherited : parent->attachments) {
        const uint64_t count = fragments(inherited, mappings);
        if (!count)
            continue;
        child->attachments.push_back({inherited.ipc_namespace, inherited.segment, inherited.address,
                                      inherited.length, 0});
        auto &attachment = child->attachments.back();
        std::lock_guard<std::mutex> namespace_lock(attachment.ipc_namespace->mutex);
        change_fragments(attachment, count, child_pid);
    }
    return child;
}

void release_shared_memory_address_space(std::shared_ptr<SharedMemoryAddressSpace> &space,
                                         uint32_t pid) noexcept {
    if (!space)
        return;
    {
        std::lock_guard<std::mutex> lock(space->mutex);
        space->last_pid = pid;
    }
    space.reset();
}

void shared_memory_check_remap(const std::shared_ptr<SharedMemoryAddressSpace> &space,
                               Memory &memory, uint64_t old_address, uint64_t old_length,
                               uint64_t new_address, uint64_t new_length, uint64_t flags) {
    if (!space)
        guest_error(22);
    const Memory::AccessGuard access(memory);
    std::lock_guard<std::mutex> lock(space->mutex);
    for (const auto &attachment : space->attachments) {
        if (intersects(old_address, old_length ? old_length : 1, attachment.address, attachment.length) ||
            ((flags & 2) && intersects(new_address, new_length, attachment.address, attachment.length)))
            guest_error(95);
    }
}

namespace {
int64_t shmget(SharedMemoryNamespace &ipc_namespace, const SharedMemoryCredentials &credentials,
               uint32_t pid, int32_t key, uint64_t size, uint32_t flags) {
    if (flags & (shm_hugetlb | shm_noreserve))
        guest_error(95);
    if (flags & ~(0777u | ipc_creat | ipc_excl))
        guest_error(22);
    std::lock_guard<std::mutex> lock(ipc_namespace.mutex);
    if (key) {
        const auto found = ipc_namespace.keys.find(key);
        if (found != ipc_namespace.keys.end()) {
            auto segment = ipc_namespace.slots[found->second];
            if ((flags & (ipc_creat | ipc_excl)) == (ipc_creat | ipc_excl))
                guest_error(17);
            if (size > segment->size)
                guest_error(22);
            check_access(*segment, credentials, flags & 0777);
            return segment->id;
        }
        if (!(flags & ipc_creat))
            guest_error(2);
    }
    if (!size || size > ipc_namespace.limits.max_segment_size)
        guest_error(22);
    const uint64_t mapped_size = align_up(size), pages = mapped_size / page_size;
    if (pages > ipc_namespace.limits.max_total_pages ||
        ipc_namespace.total_pages > ipc_namespace.limits.max_total_pages - pages)
        guest_error(28);
    uint32_t slot = ipc_namespace.next_slot;
    for (uint32_t count = 0; count < ipc_namespace.slots.size(); ++count) {
        if (!ipc_namespace.slots[slot])
            break;
        slot = (slot + 1) % uint32_t(ipc_namespace.slots.size());
    }
    if (ipc_namespace.slots[slot])
        guest_error(28);
    auto segment = std::make_shared<Segment>();
    segment->backing = create_backing(size);
    struct stat info{};
    check_host(::fstat(segment->backing.get(), &info));
    segment->device = uint64_t(info.st_dev);
    segment->inode = uint64_t(info.st_ino);
    segment->size = size;
    segment->mapped_size = mapped_size;
    segment->key = key;
    segment->sequence = uint16_t(ipc_namespace.sequences[slot] +
                                 (ipc_namespace.previously_used[slot] ? 1 : 0));
    segment->id = int32_t((uint32_t(segment->sequence) << index_bits) | slot);
    segment->uid = segment->cuid = credentials.euid;
    segment->gid = segment->cgid = credentials.egid;
    segment->mode = flags & 0777;
    segment->cpid = pid;
    segment->ctime = now();
    char path[32];
    std::snprintf(path, sizeof(path), "/SYSV%08x (deleted)", uint32_t(key));
    segment->path = path;
    if (key)
        ipc_namespace.keys.emplace(key, slot);
    ipc_namespace.slots[slot] = segment;
    ipc_namespace.sequences[slot] = segment->sequence;
    ipc_namespace.previously_used[slot] = true;
    ipc_namespace.next_slot = (slot + 1) % uint32_t(ipc_namespace.slots.size());
    ipc_namespace.total_pages += pages;
    return segment->id;
}

int64_t shmctl(SharedMemoryNamespace &ipc_namespace, Memory &memory,
               const SharedMemoryCredentials &credentials, int32_t id, int32_t command, uint64_t buffer) {
    if (id < 0 || command < 0)
        guest_error(22);
    if (command == 11 || command == 12 || command == 14)
        guest_error(95); // SHM_LOCK/UNLOCK and resident/swap accounting are not emulated.
    if (command != 0 && command != 1 && command != 2 && command != 3 && command != 13 && command != 15)
        guest_error(22);
    GuestShmidDs supplied{};
    if (command == 1)
        memory.copy_out(buffer, &supplied, sizeof(supplied));
    std::lock_guard<std::mutex> lock(ipc_namespace.mutex);
    if (command == 3) {
        const GuestShminfo info{ipc_namespace.limits.max_segment_size, 1, ipc_namespace.limits.max_segments,
                                ipc_namespace.limits.max_segments, ipc_namespace.limits.max_total_pages, {}};
        memory.copy_in(buffer, &info, sizeof(info));
        return ipc_namespace.max_index();
    }
    auto segment = command == 13 || command == 15 ? ipc_namespace.lookup_index(id) : ipc_namespace.lookup(id);
    if (command == 0) {
        check_control(*segment, credentials);
        if (!segment->removed && segment->key) {
            ipc_namespace.keys.erase(segment->key);
            segment->key = 0;
        }
        segment->removed = true;
        segment->mode |= shm_dest;
        ipc_namespace.destroy_if_unused(segment);
        return 0;
    }
    if (command == 1) {
        check_control(*segment, credentials);
        if (supplied.perm.uid == UINT32_MAX || supplied.perm.gid == UINT32_MAX)
            guest_error(22);
        segment->uid = supplied.perm.uid;
        segment->gid = supplied.perm.gid;
        segment->mode = (segment->mode & ~0777u) | (supplied.perm.mode & 0777u);
        segment->ctime = now();
        return 0;
    }
    if (command != 15)
        check_access(*segment, credentials, 0444);
    GuestShmidDs result{};
    result.perm.key = segment->key;
    result.perm.uid = segment->uid;
    result.perm.gid = segment->gid;
    result.perm.cuid = segment->cuid;
    result.perm.cgid = segment->cgid;
    result.perm.mode = segment->mode;
    result.perm.sequence = segment->sequence;
    result.size = segment->size;
    result.atime = segment->atime;
    result.dtime = segment->dtime;
    result.ctime = segment->ctime;
    result.cpid = int32_t(segment->cpid);
    result.lpid = int32_t(segment->lpid);
    result.nattch = segment->nattch;
    memory.copy_in(buffer, &result, sizeof(result));
    return command == 2 ? 0 : segment->id;
}

int64_t shmat(const std::shared_ptr<SharedMemoryNamespace> &ipc_namespace,
              SharedMemoryAddressSpace &space, Memory &memory, const process::ResourceLimits &limits,
              const SharedMemoryCredentials &credentials, uint32_t pid, uint64_t &mmap_next,
              int32_t id, uint64_t address, uint32_t flags) {
    if (id < 0 || flags & ~(shm_rdonly | shm_rnd | shm_remap | shm_exec))
        guest_error(22);
    if (flags & shm_remap) {
        if (!address)
            guest_error(22);
        guest_error(95);
    }
    if (address % page_size) {
        if (!(flags & shm_rnd))
            guest_error(22);
        address = align_down(address);
        if (!address)
            guest_error(22);
    }
    std::lock_guard<std::mutex> space_lock(space.mutex);
    std::lock_guard<std::mutex> namespace_lock(ipc_namespace->mutex);
    auto segment = ipc_namespace->lookup(id);
    const bool readonly = flags & shm_rdonly;
    const unsigned permissions = read_perm | (readonly ? 0 : write_perm) | ((flags & shm_exec) ? exec_perm : 0);
    check_access(*segment, credentials, 0444 | (readonly ? 0 : 0222) | ((flags & shm_exec) ? 0111 : 0));
    const bool selected = !address;
    if (selected)
        address = memory.find_free(mmap_next, segment->mapped_size);
    else if (!Memory::valid_range(address, segment->mapped_size) || !memory.free(address, segment->mapped_size))
        guest_error(22);
    limits.check_mapping(memory, address, segment->mapped_size, permissions, true);
    space.attachments.reserve(space.attachments.size() + 1);
    memory.map_file(address, segment->mapped_size, permissions, segment->backing.get(),
                    {segment->path, 0, 0, 0, true}, false, !readonly);
    space.attachments.push_back({ipc_namespace, segment, address, segment->mapped_size, 0});
    change_fragments(space.attachments.back(), 1, pid);
    space.last_pid = pid;
    if (selected)
        mmap_next = address + segment->mapped_size < address_limit - page_size
                        ? address + segment->mapped_size + page_size
                        : page_size;
    return int64_t(address);
}

int64_t shmdt(SharedMemoryAddressSpace &space, Memory &memory, uint32_t pid, uint64_t address) {
    if (address % page_size)
        guest_error(22);
    const auto mappings = memory.mappings();
    std::lock_guard<std::mutex> lock(space.mutex);
    auto attachment = std::find_if(space.attachments.begin(), space.attachments.end(),
                                   [&](const Attachment &value) { return value.address == address; });
    if (attachment == space.attachments.end())
        guest_error(22);
    bool found = false;
    for (const auto &mapping : mappings) {
        if (matches(*attachment, mapping)) {
            memory.unmap(mapping.start, mapping.end - mapping.start);
            found = true;
        }
    }
    if (!found)
        guest_error(22);
    {
        std::lock_guard<std::mutex> namespace_lock(attachment->ipc_namespace->mutex);
        change_fragments(*attachment, 0, pid);
    }
    space.last_pid = pid;
    space.attachments.erase(attachment);
    return 0;
}
} // namespace

int64_t shared_memory_syscall(const std::shared_ptr<SharedMemoryNamespace> &ipc_namespace,
                              const std::shared_ptr<SharedMemoryAddressSpace> &space,
                              Memory &memory, const process::ResourceLimits &limits,
                              const SharedMemoryCredentials &credentials, uint32_t pid,
                              uint64_t &mmap_next, uint64_t number, uint64_t arg0,
                              uint64_t arg1, uint64_t arg2) {
    if (!ipc_namespace || !space)
        guest_error(22);
    try {
        const Memory::AccessGuard access(memory);
        switch (number) {
        case 194:
            return shmget(*ipc_namespace, credentials, pid, int32_t(arg0), arg1, uint32_t(arg2));
        case 195:
            return shmctl(*ipc_namespace, memory, credentials, int32_t(arg0), int32_t(arg1), arg2);
        case 196:
            return shmat(ipc_namespace, *space, memory, limits, credentials, pid, mmap_next,
                         int32_t(arg0), arg1, uint32_t(arg2));
        case 197:
            return shmdt(*space, memory, pid, arg0);
        default:
            guest_error(38);
        }
    } catch (const MemoryFault &) {
        guest_error(14);
    } catch (const std::bad_alloc &) {
        guest_error(12);
    }
}
} // namespace operit::arm64::ipc
