#pragma once
#include "fs/filesystem.h"
#include <cstddef>
#include <cstdint>

namespace operit::arm64 {
class Memory;
namespace linux_abi {
struct OpenHow {
    uint64_t flags = 0, mode = 0, resolve = 0;
};
static_assert(sizeof(OpenHow) == 24);
static_assert(offsetof(OpenHow, flags) == 0 && offsetof(OpenHow, mode) == 8 &&
              offsetof(OpenHow, resolve) == 16);
} // namespace linux_abi
namespace files {
constexpr uint32_t openat2_syscall = 437;
namespace arm64_open {
constexpr uint64_t access_mode = 3, create = 64, exclusive = 128, no_ctty = 256;
constexpr uint64_t truncate = 512, append = 1024, nonblock = 2048, data_sync = 4096, async = 8192;
constexpr uint64_t directory = 16384, no_follow = 32768, direct = 65536, large_file = 131072;
constexpr uint64_t no_atime = 262144, close_on_exec = 524288, full_sync = 1048576;
constexpr uint64_t path = 2097152, temporary = 4194304;
constexpr uint64_t known = access_mode | create | exclusive | no_ctty | truncate | append | nonblock |
                           data_sync | async | directory | no_follow | direct | large_file | no_atime |
                           close_on_exec | full_sync | path | temporary;
} // namespace arm64_open

// Decode the versioned guest structure. Nonzero bytes after v0 are E2BIG;
// undersized structures are EINVAL, inaccessible guest bytes fault normally.
linux_abi::OpenHow read_open_how(Memory &memory, uint64_t address, uint64_t size);
void validate_open_how(const linux_abi::OpenHow &how);

struct OpenRequest {
    Path path;
    uint64_t flags = 0; // Linux AArch64 flags, never host O_* bit positions.
    mode_t mode = 0;    // Validated permissions; the caller applies its umask.
    bool path_only() const {
        return flags & arm64_open::path;
    }
    bool temporary() const {
        return flags & arm64_open::temporary;
    }
};
// The syscall caller supplies a pinned directory from Filesystem::directory_anchor.
// Unscoped absolute names ignore that directory, just as the kernel ignores dfd.
// This prepares the open; the caller opens/inserts the returned Path itself.
OpenRequest resolve_openat2(const Filesystem &filesystem, const std::string &name,
                           const Path &directory, const linux_abi::OpenHow &how);
} // namespace files
} // namespace operit::arm64
