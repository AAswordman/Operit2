#pragma once
#include <cstdint>
#include <memory>
#include <mutex>
#include <optional>
#include <string>

namespace operit::arm64 {
class Memory;
class DescriptorTable;
struct Descriptor;
struct OpenDescription;
}

namespace operit::arm64::files {
// Linux ABI values; never pass these to Darwin fcntl.
constexpr uint32_t mfd_cloexec = 0x0001;
constexpr uint32_t mfd_allow_sealing = 0x0002;
constexpr uint32_t memory_file_seal_seal = 0x0001;
constexpr uint32_t memory_file_seal_shrink = 0x0002;
constexpr uint32_t memory_file_seal_grow = 0x0004;
constexpr uint32_t memory_file_seal_write = 0x0008;
constexpr uint32_t memory_file_seal_future_write = 0x0010;

struct MemoryFileState;

// Creates a zero-length O_RDWR | O_LARGEFILE description. Host FDs are always
// CLOEXEC; MFD_CLOEXEC controls only the guest descriptor's FD_CLOEXEC bit.
int memory_file_create(Memory &memory, DescriptorTable &descriptors, uint64_t name, uint32_t flags);
int memory_file_get_seals(const Descriptor &entry);
void memory_file_add_seals(const Descriptor &entry, uint32_t seals);

// Descriptor construction attaches every new OpenDescription automatically.
// dup/fork already share OpenDescription and need no extra attachment.
// The raw host identity also finds the state through dup'd mapping/I/O FDs.
void memory_file_attach(const Descriptor &entry);
void memory_file_description_closed(const OpenDescription *description) noexcept;
std::optional<std::string> memory_file_link(int host_fd);

// Keep the returned guard alive through the native operation. Seal insertion,
// resize and guarded I/O use the same mutex, eliminating check/use races.
// Copy guest buffers BEFORE acquiring an I/O guard, and release it before
// touching guest memory or blocking in the scheduler. Memory mapping paths
// acquire Memory::AccessGuard before acquiring a file seal lock.
class MemoryFileGuard {
    std::shared_ptr<MemoryFileState> state_;
    std::unique_lock<std::recursive_mutex> lock_;
    uint64_t length_ = 0;
    uint64_t offset_ = 0;

    explicit MemoryFileGuard(std::shared_ptr<MemoryFileState> state);
    friend MemoryFileGuard memory_file_lock(int);
    friend MemoryFileGuard memory_file_check_write(int, uint64_t, std::optional<uint64_t>,
                                                    std::optional<bool>);
    friend MemoryFileGuard memory_file_check_truncate(int, uint64_t);
    friend MemoryFileGuard memory_file_check_fallocate(int, uint32_t, uint64_t, uint64_t);

  public:
    MemoryFileGuard(MemoryFileGuard &&) noexcept;
    MemoryFileGuard &operator=(MemoryFileGuard &&) noexcept;
    MemoryFileGuard(const MemoryFileGuard &) = delete;
    MemoryFileGuard &operator=(const MemoryFileGuard &) = delete;
    ~MemoryFileGuard();
    explicit operator bool() const {
        return bool(state_);
    }
    // GROW rejects the first guest-page write chunk which extends past EOF.
    // If earlier chunks fit, return only that completed prefix, ending at a
    // page boundary; never clip the failing chunk to EOF. Apply any FSIZE
    // count reduction before checking seals, as Linux generic_write_checks does.
    uint64_t length() const {
        return length_;
    }
    uint64_t offset() const {
        return offset_;
    }
};

// Also guard read/readv/lseek that change a memfd's shared native file offset,
// so unpositioned writes cannot race the offset used by F_SEAL_GROW checks.
// On ordinary files all of these guards are inert and preserve request sizes.
MemoryFileGuard memory_file_lock(int host_fd);
// With no append override, inspect the native O_APPEND flag while locked.
// RWF_APPEND/RWF_NOAPPEND callers can explicitly force true/false.
MemoryFileGuard memory_file_check_write(int host_fd, uint64_t length,
                                        std::optional<uint64_t> offset = std::nullopt,
                                        std::optional<bool> append = std::nullopt);
MemoryFileGuard memory_file_check_truncate(int host_fd, uint64_t length);
// Protects hole-punch and allocation paths as well as explicit truncation.
MemoryFileGuard memory_file_check_fallocate(int host_fd, uint32_t mode, uint64_t offset,
                                            uint64_t length);

// Attach the token to the mapping's shared MappingObject, not the descriptor
// or individual resident pages. Preserve it across split/coalesce/fork/remap;
// release it only when the last view of that original mapping disappears.
// A failed mmap must release its token. MAP_PRIVATE never blocks write seals.
class MemoryFileMapping {
    std::shared_ptr<MemoryFileState> state_;
    bool shared_, may_write_;
    MemoryFileMapping(std::shared_ptr<MemoryFileState> state, bool shared, bool may_write);
    friend std::shared_ptr<MemoryFileMapping> memory_file_check_mmap(int, bool, unsigned, bool);

  public:
    MemoryFileMapping(const MemoryFileMapping &) = delete;
    MemoryFileMapping &operator=(const MemoryFileMapping &) = delete;
    ~MemoryFileMapping();
    // For MAP_SHARED, clear Region::maximum_permissions & write_perm when
    // false. A mapping predating F_SEAL_FUTURE_WRITE retains its permission.
    bool may_write() const {
        return may_write_;
    }
    void check_protection(unsigned permissions) const;
};

// Reserve before installing the mapping, after ordinary mmap access checks.
// Track writable CAPABILITY even for PROT_READ/PROT_NONE shared mappings: a
// mapping which can subsequently gain PROT_WRITE prevents F_SEAL_WRITE.
// A new shared PROT_WRITE mapping forbidden by a seal fails with EPERM; a new
// read-only shared mapping succeeds with its future write permission removed.
std::shared_ptr<MemoryFileMapping> memory_file_check_mmap(int host_fd, bool shared,
                                                        unsigned permissions,
                                                        bool permit_write = true);
void memory_file_check_mprotect(const std::shared_ptr<MemoryFileMapping> &mapping,
                                unsigned permissions);
} // namespace operit::arm64::files
