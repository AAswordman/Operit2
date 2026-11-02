#pragma once
#include <cstdint>

namespace operit::arm64::linux_abi {
/* asm-generic Linux layouts. Never memcpy Darwin structs into guest memory. */
struct Timespec {
    int64_t seconds, nanoseconds;
};
struct Iovec {
    uint64_t base, length;
};
struct Stat {
    uint64_t device, inode;
    uint32_t mode, links, uid, gid;
    uint64_t rdevice, pad1;
    int64_t size;
    int32_t block_size, pad2;
    int64_t blocks, atime;
    uint64_t atime_ns;
    int64_t mtime;
    uint64_t mtime_ns;
    int64_t ctime;
    uint64_t ctime_ns;
    uint32_t unused[2];
};
static_assert(sizeof(Timespec) == 16);
static_assert(sizeof(Iovec) == 16);
static_assert(sizeof(Stat) == 128);
} // namespace operit::arm64::linux_abi
