#pragma once
#include <cstddef>
namespace operit::arm64 {
struct BasicBlock;
std::size_t apply_profile_superblocks(BasicBlock &block);
bool is_profile_superblock_entry(const void *entry);
}
