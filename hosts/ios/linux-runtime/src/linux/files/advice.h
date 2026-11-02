#pragma once
#include <cstdint>

namespace operit::arm64 {
class Memory;
class DescriptorTable;
}

namespace operit::arm64::files {
// Linux AArch64 ABI values, independent of host fcntl/posix_fadvise constants.
constexpr int32_t fadv_normal = 0, fadv_random = 1, fadv_sequential = 2;
constexpr int32_t fadv_willneed = 3, fadv_dontneed = 4, fadv_noreuse = 5;
constexpr uint32_t sync_range_wait_before = 1, sync_range_write = 2, sync_range_wait_after = 4;

// Syscalls 213, 223 and 84 respectively. Return 0 or throw GuestError with a
// Linux errno (including fadvise64, unlike libc posix_fadvise's positive errno).
// These calls contain no guest pointers; Memory is retained in the dispatcher
// interface. Native mappings already share the host file's page cache.
int64_t readahead(Memory &memory, DescriptorTable &descriptors, int32_t fd, int64_t offset, uint64_t count);
int64_t fadvise64(Memory &memory, DescriptorTable &descriptors, int32_t fd, int64_t offset, int64_t length,
                  int32_t advice);
int64_t sync_file_range(Memory &memory, DescriptorTable &descriptors, int32_t fd, int64_t offset,
                       int64_t length, uint32_t flags);

// Readahead may warm a bounded prefix, as Linux also limits its readahead window.
// Darwin sync_file_range uses whole-file writeback/waits: stronger and potentially
// more blocking than Linux's range-only operations. No path changes an open offset.
} // namespace operit::arm64::files
