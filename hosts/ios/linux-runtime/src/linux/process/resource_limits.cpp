#include "linux/process/resource_limits.h"
#include "base/support.h"
#include "memory/address_space.h"
#include <algorithm>
#include <utility>

namespace operit::arm64::process {
namespace {
constexpr uint64_t clone_thread = 0x10000;
uint32_t resource_index(uint32_t resource) {
    if (resource >= resource_count)
        guest_error(22);
    return resource;
}
uint64_t checked_add(uint64_t left, uint64_t right) {
    if (right > UINT64_MAX - left)
        guest_error(12);
    return left + right;
}
void add_usage(MemoryUsage &to, const MemoryUsage &from) {
    to.address_space = checked_add(to.address_space, from.address_space);
    to.data = checked_add(to.data, from.data);
    to.stack = checked_add(to.stack, from.stack);
}
bool is_stack(const Memory::Mapping &mapping) {
    return mapping.backing.path == "[stack]";
}
MemoryUsage mapping_usage(uint64_t length, unsigned permissions, bool shared, bool stack) {
    // Linux data_vm counts private writable VMAs, excluding VM_STACK.
    return {length, !shared && !stack && (permissions & write_perm) ? length : 0,
            stack ? length : 0};
}
MemoryUsage mapping_usage(const Memory::Mapping &mapping, uint64_t length) {
    return mapping_usage(length, mapping.permissions, mapping.backing.shared, is_stack(mapping));
}
MemoryUsage overlap_usage(const std::vector<Memory::Mapping> &mappings, uint64_t address,
                          uint64_t length) {
    MemoryUsage result;
    const uint64_t end = address + length;
    for (const auto &mapping : mappings) {
        const uint64_t begin = std::max(address, mapping.start);
        const uint64_t stop = std::min(end, mapping.end);
        if (begin < stop)
            add_usage(result, mapping_usage(mapping, stop - begin));
    }
    return result;
}
MemoryUsage total_usage(const std::vector<Memory::Mapping> &mappings) {
    MemoryUsage result;
    for (const auto &mapping : mappings)
        add_usage(result, mapping_usage(mapping, mapping.end - mapping.start));
    return result;
}
void check_page_range(uint64_t address, uint64_t length) {
    if (!length || address % page_size || length % page_size ||
        !Memory::valid_range(address, length))
        guest_error(22);
}
void check_growth(uint64_t current, uint64_t removed, uint64_t added, uint64_t limit) {
    if (removed > current)
        throw Failure("resource limit accounting underflow");
    const uint64_t next = checked_add(current - removed, added);
    // Lowering a limit does not destroy mappings or prevent shrinking/replacing
    // them with a mapping of the same charged size.
    if (next > current && next > (limit & ~(page_size - 1)))
        guest_error(12);
}
ResourceLimit read_limit(Memory &memory, uint64_t address) {
    if (!memory.accessible(address, sizeof(ResourceLimit), read_perm))
        guest_error(14);
    return {memory.load(address, 8), memory.load(address + 8, 8)};
}
void write_limit(Memory &memory, uint64_t address, ResourceLimit limit) {
    if (!memory.accessible(address, sizeof(ResourceLimit), write_perm))
        guest_error(14);
    memory.store(address, limit.soft, 8);
    memory.store(address + 8, limit.hard, 8);
}
} // namespace

ResourceLimits::ResourceLimits() {
    limits_.fill({resource_infinity, resource_infinity});
    limits_[static_cast<uint32_t>(Resource::Stack)] = {uint64_t{8} << 20, resource_infinity};
    limits_[static_cast<uint32_t>(Resource::Core)] = {0, resource_infinity};
    limits_[static_cast<uint32_t>(Resource::OpenFiles)] = {1024, maximum_open_files};
    limits_[static_cast<uint32_t>(Resource::LockedMemory)] = {uint64_t{8} << 20, uint64_t{8} << 20};
    limits_[static_cast<uint32_t>(Resource::MessageQueue)] = {819200, 819200};
    limits_[static_cast<uint32_t>(Resource::Nice)] = {0, 0};
    limits_[static_cast<uint32_t>(Resource::RealtimePriority)] = {0, 0};
}
ResourceLimits::ResourceLimits(const Snapshot &limits) : limits_(limits) {}
ResourceLimits::Snapshot ResourceLimits::snapshot() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return limits_;
}
std::shared_ptr<ResourceLimits> ResourceLimits::fork_copy() const {
    return std::shared_ptr<ResourceLimits>(new ResourceLimits(snapshot()));
}
ResourceLimit ResourceLimits::get(uint32_t resource) const {
    resource_index(resource);
    std::lock_guard<std::mutex> lock(mutex_);
    return limits_[resource];
}
bool ResourceLimits::change_supported(Resource resource) {
    switch (resource) {
    case Resource::Cpu:
    case Resource::FileSize:
    case Resource::Processes:
    case Resource::PendingSignals:
    case Resource::Data:
    case Resource::Stack:
    case Resource::OpenFiles:
    case Resource::AddressSpace:
    // Scheduling setters enforce these against the target guest limits.
    case Resource::Nice:
    case Resource::RealtimePriority:
    // Machine enforces continuous realtime CPU service at execution safe points.
    case Resource::RealtimeTime:
    // Linux retains these ABI values but does not enforce RSS or LOCKS.
    case Resource::Rss:
    case Resource::Locks:
        return true;
    default:
        return false;
    }
}
ResourceLimit ResourceLimits::exchange(uint32_t resource,
                                      const std::optional<ResourceLimit> &replacement,
                                      bool sys_resource) {
    resource_index(resource);
    if (replacement && replacement->soft > replacement->hard)
        guest_error(22);
    std::lock_guard<std::mutex> lock(mutex_);
    const ResourceLimit old = limits_[resource];
    if (!replacement)
        return old;
    const auto next = *replacement;
    if (next.hard > old.hard && !sys_resource)
        guest_error(1);
    if (resource == static_cast<uint32_t>(Resource::OpenFiles) && next.hard > maximum_open_files)
        guest_error(1);
    if ((next.soft != old.soft || next.hard != old.hard) &&
        !change_supported(static_cast<Resource>(resource)))
        guest_error(95);
    limits_[resource] = next;
    return old;
}
int ResourceLimits::check_cpu_time(uint64_t user_nanoseconds, uint64_t system_nanoseconds) {
    constexpr uint64_t second = 1000000000;
    // Avoid overflowing when adding two absolute nanosecond counters, or when
    // converting a very large guest limit from seconds to nanoseconds.
    const uint64_t seconds = user_nanoseconds / second + system_nanoseconds / second +
                             (user_nanoseconds % second + system_nanoseconds % second) / second;
    std::lock_guard<std::mutex> lock(mutex_);
    auto &limit = limits_[static_cast<uint32_t>(Resource::Cpu)];
    if (limit.hard != resource_infinity && seconds >= limit.hard)
        return 9;
    if (limit.soft != resource_infinity && seconds >= limit.soft) {
        ++limit.soft;
        return 24;
    }
    return 0;
}
int ResourceLimits::check_realtime_time(uint64_t runtime_nanoseconds) {
    if (!runtime_nanoseconds)
        return 0; // The watchdog has not observed any CPU service yet.
    const uint64_t microseconds = runtime_nanoseconds / 1000;
    std::lock_guard<std::mutex> lock(mutex_);
    auto &limit = limits_[static_cast<uint32_t>(Resource::RealtimeTime)];
    if (limit.hard != resource_infinity && microseconds >= limit.hard)
        return 9;
    if (limit.soft != resource_infinity && microseconds >= limit.soft) {
        // Keep the hard check separate: Linux's soft-limit promotion may pass
        // the hard ceiling. Saturate instead of wrapping an extreme ABI value.
        constexpr uint64_t second = 1000000;
        limit.soft = limit.soft > resource_infinity - second ? resource_infinity : limit.soft + second;
        return 24;
    }
    return 0;
}
uint64_t ResourceLimits::file_write_count(uint64_t offset, uint64_t count) const {
    if (!count)
        return 0;
    const uint64_t limit = get(Resource::FileSize).soft;
    if (limit == resource_infinity)
        return count;
    if (offset >= limit)
        guest_error(27);
    return std::min(count, limit - offset);
}
void ResourceLimits::check_file_resize(uint64_t current_size, uint64_t requested_size) const {
    if (requested_size > current_size && requested_size > get(Resource::FileSize).soft)
        guest_error(27);
}
void ResourceLimits::check_memory_change(const MemoryUsage &current, const MemoryUsage &removed,
                                         const MemoryUsage &added) const {
    const auto values = snapshot();
    check_growth(current.address_space, removed.address_space, added.address_space,
                 values[static_cast<uint32_t>(Resource::AddressSpace)].soft);
    check_growth(current.data, removed.data, added.data,
                 values[static_cast<uint32_t>(Resource::Data)].soft);
    check_growth(current.stack, removed.stack, added.stack,
                 values[static_cast<uint32_t>(Resource::Stack)].soft);
}
void ResourceLimits::check_mapping(const Memory &memory, uint64_t address, uint64_t length,
                                   unsigned permissions, bool shared, bool replace, bool stack) const {
    check_page_range(address, length);
    if (permissions > 7)
        guest_error(22);
    const auto mappings = memory.mappings();
    const auto removed = replace ? overlap_usage(mappings, address, length) : MemoryUsage{};
    check_memory_change(total_usage(mappings), removed,
                        mapping_usage(length, permissions, shared, stack));
}
void ResourceLimits::check_protection(const Memory &memory, uint64_t address, uint64_t length,
                                      unsigned permissions) const {
    if (!length)
        return;
    check_page_range(address, length);
    if (permissions > 7)
        guest_error(22);
    if (!memory.accessible(address, length, 0))
        guest_error(12);
    const auto mappings = memory.mappings();
    MemoryUsage removed, added;
    for (const auto &mapping : mappings) {
        const uint64_t begin = std::max(address, mapping.start);
        const uint64_t end = std::min(address + length, mapping.end);
        if (begin < end) {
            add_usage(removed, mapping_usage(mapping, end - begin));
            add_usage(added, mapping_usage(end - begin, permissions, mapping.backing.shared,
                                          is_stack(mapping)));
        }
    }
    check_memory_change(total_usage(mappings), removed, added);
}
void ResourceLimits::check_remap(const Memory &memory, uint64_t address, uint64_t old_size,
                                 uint64_t new_size, unsigned flags, uint64_t new_address) const {
    constexpr unsigned may_move = 1, fixed = 2, keep_old = 4;
    if (address % page_size || !new_size || old_size % page_size || new_size % page_size ||
        flags & ~(may_move | fixed | keep_old) ||
        ((flags & (fixed | keep_old)) && !(flags & may_move)) ||
        ((flags & keep_old) && old_size != new_size) ||
        !Memory::valid_range(address, std::max<uint64_t>(old_size, 1)) ||
        new_size > address_limit - page_size)
        guest_error(22);
    const auto mappings = memory.mappings();
    const auto source = std::find_if(mappings.begin(), mappings.end(), [&](const Memory::Mapping &m) {
        return m.start <= address && address < m.end;
    });
    if (source == mappings.end() || old_size > source->end - address)
        guest_error(14);
    if (!old_size && (!source->backing.shared || !(flags & may_move)))
        guest_error(22);
    MemoryUsage removed;
    if (flags & fixed) {
        check_page_range(new_address, new_size);
        if (old_size && new_address < address + old_size && address < new_address + new_size)
            guest_error(22);
        removed = overlap_usage(mappings, new_address, new_size);
    }
    if (old_size && !(flags & keep_old))
        add_usage(removed, mapping_usage(*source, old_size));
    check_memory_change(total_usage(mappings), removed, mapping_usage(*source, new_size));
}
void ResourceLimits::check_exec_image(const Memory &memory) const {
    check_memory_change({}, {}, resource_memory_usage(memory));
}
void ResourceLimits::check_data_size(uint64_t bytes) const {
    if (bytes > get(Resource::Data).soft)
        guest_error(12);
}
void ResourceLimits::check_brk(const Memory &memory, uint64_t start_data, uint64_t end_data,
                               uint64_t start_brk, uint64_t old_brk, uint64_t requested) const {
    if (end_data < start_data || old_brk < start_brk || requested < start_brk ||
        requested >= address_limit - page_size || old_brk >= address_limit - page_size)
        guest_error(12);
    if (requested <= old_brk)
        return;
    check_data_size(checked_add(end_data - start_data, requested - start_brk));
    const auto old_end = align_up(old_brk), new_end = align_up(requested);
    if (new_end > old_end)
        check_mapping(memory, old_end, new_end - old_end, read_perm | write_perm, false);
}
uint64_t ResourceLimits::exec_argument_limit() const {
    constexpr uint64_t minimum = 32 * page_size;
    constexpr uint64_t maximum = (uint64_t{8} << 20) / 4 * 3;
    return std::max(minimum, std::min(maximum, get(Resource::Stack).soft / 4));
}
void ResourceLimits::check_exec_arguments(uint64_t bytes) const {
    if (bytes > exec_argument_limit())
        guest_error(7);
}
MemoryUsage resource_memory_usage(const Memory &memory) {
    return total_usage(memory.mappings());
}
std::shared_ptr<ResourceLimits> inherit_resource_limits(const std::shared_ptr<ResourceLimits> &parent,
                                                       uint64_t clone_flags) {
    if (!parent)
        throw Failure("missing guest resource limits");
    return clone_flags & clone_thread ? parent : parent->fork_copy();
}
void check_limit_access(const LimitProcess &caller, const LimitProcess &target) {
    if (!target.limits)
        guest_error(3);
    if (caller.pid == target.pid || caller.credentials.sys_resource)
        return;
    const auto &from = caller.credentials;
    const auto &to = target.credentials;
    if (from.real_uid != to.real_uid || from.real_uid != to.effective_uid ||
        from.real_uid != to.saved_uid || from.real_gid != to.real_gid ||
        from.real_gid != to.effective_gid || from.real_gid != to.saved_gid)
        guest_error(1);
}
int64_t resource_limit_syscall(Memory &memory, uint64_t number, uint64_t arg0, uint64_t arg1,
                               uint64_t arg2, uint64_t arg3, const LimitProcess &caller,
                               const LimitProcessLookup &lookup) {
    uint32_t resource;
    uint64_t new_address = 0, old_address = 0;
    uint32_t pid = 0;
    if (number == 163) {
        resource = uint32_t(arg0);
        old_address = arg1;
    } else if (number == 164) {
        resource = uint32_t(arg0);
        new_address = arg1;
    } else if (number == 261) {
        if (int32_t(arg0) < 0)
            guest_error(22);
        pid = uint32_t(arg0);
        resource = uint32_t(arg1);
        new_address = arg2;
        old_address = arg3;
    } else {
        guest_error(38);
    }
    resource_index(resource);
    std::optional<ResourceLimit> replacement;
    if (new_address || number == 164)
        replacement = read_limit(memory, new_address);
    if (replacement && replacement->soft > replacement->hard)
        guest_error(22);
    LimitProcess target = caller;
    if (pid && pid != caller.pid) {
        const auto found = lookup ? lookup(pid) : std::optional<LimitProcess>{};
        if (!found)
            guest_error(3);
        target = *found;
    }
    check_limit_access(caller, target);
    const auto old = target.limits->exchange(resource, replacement, caller.credentials.sys_resource);
    // Linux applies the new limit before copying the old pair to userspace.
    // A bad old-limit pointer can therefore report EFAULT after a successful set.
    if (old_address || number == 163)
        write_limit(memory, old_address, old);
    return 0;
}
} // namespace operit::arm64::process
