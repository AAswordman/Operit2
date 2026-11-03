#pragma once
#include <array>
#include <cstdint>
#include <functional>
#include <memory>
#include <mutex>
#include <optional>

namespace operit::arm64 {
class Memory;
namespace process {
enum class Resource : uint32_t {
    Cpu = 0,
    FileSize = 1,
    Data = 2,
    Stack = 3,
    Core = 4,
    Rss = 5,
    Processes = 6,
    OpenFiles = 7,
    LockedMemory = 8,
    AddressSpace = 9,
    Locks = 10,
    PendingSignals = 11,
    MessageQueue = 12,
    Nice = 13,
    RealtimePriority = 14,
    RealtimeTime = 15,
};
constexpr uint32_t resource_count = 16;
constexpr uint64_t resource_infinity = UINT64_MAX;
constexpr uint64_t maximum_open_files = 1024 * 1024;

// AArch64 uses two little-endian 64-bit words for both rlimit and rlimit64.
struct ResourceLimit {
    uint64_t soft;
    uint64_t hard;
};
static_assert(sizeof(ResourceLimit) == 16, "Linux rlimit64 ABI");

struct MemoryUsage {
    uint64_t address_space = 0;
    uint64_t data = 0;
    uint64_t stack = 0;
};

class ResourceLimits {
  public:
    using Snapshot = std::array<ResourceLimit, resource_count>;

  private:
    mutable std::mutex mutex_;
    Snapshot limits_;
    explicit ResourceLimits(const Snapshot &limits);

  public:
    ResourceLimits();
    ResourceLimits(const ResourceLimits &) = delete;
    ResourceLimits &operator=(const ResourceLimits &) = delete;
    Snapshot snapshot() const;
    std::shared_ptr<ResourceLimits> fork_copy() const;
    ResourceLimit get(uint32_t resource) const;
    ResourceLimit get(Resource resource) const {
        return get(static_cast<uint32_t>(resource));
    }
    // The returned old pair and the optional replacement are one locked operation.
    // sys_resource is a guest capability, never inferred from the host uid.
    ResourceLimit exchange(uint32_t resource, const std::optional<ResourceLimit> &replacement,
                           bool sys_resource = false);
    void set(Resource resource, ResourceLimit limit, bool sys_resource = false) {
        exchange(static_cast<uint32_t>(resource), limit, sys_resource);
    }
    static bool change_supported(Resource resource);

    // Aggregate user+system time for the whole thread group, excluding children.
    // Returns 0, SIGXCPU(24), or SIGKILL(9). A soft expiry advances rlim_cur
    // by one CPU second under the same mutex used by prlimit64.
    int check_cpu_time(uint64_t user_nanoseconds, uint64_t system_nanoseconds);
    // RTTIME uses microsecond limits against one thread's continuous RT CPU
    // service. Soft expiry advances the shared soft limit by one CPU second;
    // hard expiry wins. Scheduler resets the thread counter on a real wake.
    int check_realtime_time(uint64_t runtime_nanoseconds);
    // Regular-file writes may commit a prefix. Only a nonempty write starting
    // at/above the soft limit fails with EFBIG; the caller then queues SIGXFSZ.
    uint64_t file_write_count(uint64_t offset, uint64_t count) const;
    // For truncate/fallocate size growth; shrinking an existing file remains
    // legal after lowering FSIZE. KEEP_SIZE allocation does not resize a file.
    void check_file_resize(uint64_t current_size, uint64_t requested_size) const;

    // Check before modifying VMAs, including before MAP_FIXED destroys old VMAs.
    // These operate on virtual mapping sizes, independently of the resident cap.
    void check_memory_change(const MemoryUsage &current, const MemoryUsage &removed,
                             const MemoryUsage &added) const;
    void check_mapping(const Memory &memory, uint64_t address, uint64_t length,
                       unsigned permissions, bool shared, bool replace = false,
                       bool stack = false) const;
    void check_protection(const Memory &memory, uint64_t address, uint64_t length,
                          unsigned permissions) const;
    void check_remap(const Memory &memory, uint64_t address, uint64_t old_size,
                     uint64_t new_size, unsigned flags, uint64_t new_address) const;
    // Check before updating brk, including requests that stay in the same page.
    // Translate GuestError(ENOMEM) to the previous brk at the syscall boundary.
    void check_brk(const Memory &memory, uint64_t start_data, uint64_t end_data,
                   uint64_t start_brk, uint64_t old_brk, uint64_t requested) const;
    void check_exec_image(const Memory &memory) const;
    void check_data_size(uint64_t bytes) const;
    uint64_t exec_argument_limit() const;
    void check_exec_arguments(uint64_t bytes) const;
};

MemoryUsage resource_memory_usage(const Memory &memory);
// CLONE_THREAD shares the signal-group limits. CLONE_VM and CLONE_FILES alone do not.
std::shared_ptr<ResourceLimits> inherit_resource_limits(const std::shared_ptr<ResourceLimits> &parent,
                                                       uint64_t clone_flags);

struct LimitCredentials {
    uint32_t real_uid = 0, effective_uid = 0, saved_uid = 0;
    uint32_t real_gid = 0, effective_gid = 0, saved_gid = 0;
    bool sys_resource = false;
};
struct LimitProcess {
    uint32_t pid = 0;
    LimitCredentials credentials;
    std::shared_ptr<ResourceLimits> limits;
};
using LimitProcessLookup = std::function<std::optional<LimitProcess>(uint32_t)>;
void check_limit_access(const LimitProcess &caller, const LimitProcess &target);

// Dispatch 163=getrlimit, 164=setrlimit, 261=prlimit64. Lookup must resolve
// live guest tasks (including a guest tid) to their thread-group limit object.
// Errors use Linux GuestError numbers. No host resource limits are modified.
int64_t resource_limit_syscall(Memory &memory, uint64_t number, uint64_t arg0, uint64_t arg1,
                               uint64_t arg2, uint64_t arg3, const LimitProcess &caller,
                               const LimitProcessLookup &lookup = {});
} // namespace process
} // namespace operit::arm64
