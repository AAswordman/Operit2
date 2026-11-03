#include "linux/syscalls.h"
#include "linux/io_uring/io_uring.h"

namespace operit::arm64 {
int64_t Syscalls::dispatch_memory(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (a[8]) {
    case 214: // brk
        return process_brk(a[0]);
    case 215: { // munmap
        if (a[0] % page_size || !a[1] || a[1] > address_limit - page_size)
            guest_error(22);
        const Memory::AccessGuard memory_access(memory_);
        memory_.unmap(a[0], align_up(a[1]));
        sync_shared_memory_mappings();
        return 0;
    }
    case 216: { // mremap
        if (a[1] > address_limit - page_size || a[2] > address_limit - page_size || a[3] > 7)
            guest_error(22);
        const uint64_t old_size = align_up(a[1]), new_size = align_up(a[2]);
        const Memory::AccessGuard memory_access(memory_);
        ipc::shared_memory_check_remap(shared_memory_space_, memory_, a[0], old_size, a[4], new_size, a[3]);
        resource_limits()->check_remap(memory_, a[0], old_size, new_size, unsigned(a[3]), a[4]);
        const uint64_t address =
            memory_.remap(a[0], old_size, new_size, unsigned(a[3]), a[4], mmap_next_);
        if (address != a[0])
            mmap_next_ = address + align_up(a[2]) < address_limit - page_size
                             ? address + align_up(a[2]) + page_size
                             : page_size;
        sync_shared_memory_mappings();
        return address;
    }
    case 222: { // mmap
        constexpr uint64_t map_shared = 1, map_private = 2, map_shared_validate = 3;
        constexpr uint64_t map_fixed = 0x10, map_anonymous = 0x20, map_populate = 0x8000;
        constexpr uint64_t map_nonblock = 0x10000, map_fixed_noreplace = 0x100000;
        // DENYWRITE/EXECUTABLE/STACK are ignored flags in the Linux mmap ABI.
        // NORESERVE selects the lazy allocation already used by this VM.
        constexpr uint64_t supported = 3 | map_fixed | map_anonymous | 0x800 | 0x1000 | 0x4000 |
                                       map_populate | map_nonblock | 0x20000 | map_fixed_noreplace;
        const uint64_t type = a[3] & 0xf;
        if (type != map_shared && type != map_private && type != map_shared_validate)
            guest_error(22);
        if (a[3] & ~supported)
            guest_error(type == map_shared_validate ? 95 : 22);
        if (type == map_shared_validate && a[3] & map_anonymous)
            guest_error(22);
        if (!a[1] || a[2] > 7 || a[5] % page_size)
            guest_error(22);
        if (a[1] >= address_limit - page_size)
            guest_error(12);
        const uint64_t length = align_up(a[1]);
        const Memory::AccessGuard memory_access(memory_);
        uint64_t address = align_down(a[0]);
        const bool fixed = a[3] & (map_fixed | map_fixed_noreplace);
        const bool shared = type != map_private;
        if (fixed && (a[0] % page_size || !Memory::valid_range(address, length)))
            guest_error(22);
        if (a[3] & map_fixed_noreplace && !memory_.free(address, length))
            guest_error(17);
        if (!fixed && (!address || !memory_.free(address, length)))
            address = memory_.find_free(mmap_next_, length);
        const bool replace = a[3] & map_fixed;
        Descriptor *file = nullptr;
        if (!(a[3] & map_anonymous)) {
            file = &descriptors_.get(uint32_t(a[4]));
            if (file->description->flags & 0x200000)
                guest_error(9);
            if ((file->description->flags & 3) == 1)
                guest_error(13);
            if (file->description->virtual_file) {
                if (file->description->virtual_file->node->kind != VirtualNode::Kind::Zero)
                    guest_error(19);
                file = nullptr;
            }
        }
        resource_limits()->check_mapping(memory_, address, length, unsigned(a[2]), shared, replace);
        if (file && file->description->io_uring)
            io_uring::map_ring(file->description->io_uring, memory_, address, length, unsigned(a[2]),
                               a[3], a[5], replace);
        else if (file)
            memory_.map_file(address, length, unsigned(a[2]), file->host.get(),
                             {file->path, a[5], 0, 0, shared}, replace, !file->description->is_read_only(),
                             file->description->inotify_open);
        else
            memory_.map(address, length, unsigned(a[2]), replace, shared);
        if (a[3] & map_populate && !(a[3] & map_nonblock)) {
            try {
                memory_.advise(address, length, a[2] & write_perm ? 23 : 22);
            } catch (const GuestError &) {
                // Linux MAP_POPULATE permits a partially populated mapping.
            }
        }
        sync_shared_memory_mappings();
        if (!fixed)
            mmap_next_ =
                address + length < address_limit - page_size ? address + length + page_size : page_size;
        return address;
    }
    case 226: // mprotect
        if (a[0] % page_size || a[2] > 7 || a[1] >= address_limit - page_size)
            guest_error(22);
        if (a[1]) {
            const uint64_t length = align_up(a[1]);
            const Memory::AccessGuard memory_access(memory_);
            resource_limits()->check_protection(memory_, a[0], length, unsigned(a[2]));
            memory_.protect(a[0], length, unsigned(a[2]));
            sync_shared_memory_mappings();
        }
        return 0;
    case 227: // msync
        if (a[1] > address_limit - page_size || a[2] > 7)
            guest_error(22);
        memory_.sync(a[0], align_up(a[1]), unsigned(a[2]));
        return 0;
    case 232: { // mincore
        if (a[0] % page_size || a[1] > address_limit - page_size)
            guest_error(22);
        const uint64_t length = align_up(a[1]);
        check_buffer(a[2], length / page_size, write_perm);
        for (uint64_t offset = 0; offset < length;) {
            const uint64_t count = std::min<uint64_t>(length - offset, page_size * page_size);
            auto states = memory_.residency(a[0] + offset, count);
            memory_.copy_in(a[2] + offset / page_size, states.data(), states.size());
            offset += count;
        }
        return 0;
    }
    case 233: { // madvise
        if (a[1] > address_limit - page_size)
            guest_error(22);
        const Memory::AccessGuard memory_access(memory_);
        memory_.advise(a[0], align_up(a[1]), int32_t(a[2]));
        sync_shared_memory_mappings();
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
