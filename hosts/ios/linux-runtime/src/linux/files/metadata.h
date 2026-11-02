#pragma once
#include "fs/descriptor_table.h"
#include "fs/filesystem.h"
#include "memory/address_space.h"
#include <array>
#include <sys/stat.h>

namespace operit::arm64::files {
// Linux AArch64 layouts and flags are translated in this module.
void statfs(Memory &memory, uint64_t address, const Path &path);
void statfs(Memory &memory, uint64_t address, const Descriptor &entry);
void statx(Memory &memory, uint64_t address, uint32_t requested, const struct stat &info,
           const Path *path = nullptr, const Descriptor *entry = nullptr);
// operation is the Linux syscall number, 5..16. fd variants use entry; path
// variants use a resolved Path and never reconstruct a host absolute pathname.
int64_t xattr(Memory &memory, uint64_t operation, const std::array<uint64_t, 31> &arguments, const Path *path,
              const Descriptor *entry);
} // namespace operit::arm64::files
