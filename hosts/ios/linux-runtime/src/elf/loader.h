#pragma once
#include "acceleration/fixed_components.h"
#include "fs/filesystem.h"
#include "memory/address_space.h"

namespace operit::arm64 {
struct Image {
    uint64_t entry = 0, phdr = 0, phent = 0, phnum = 0, end = 0, base = 0;
    std::string interpreter, path;
};
class ElfLoader {
    Memory &memory_;
    Filesystem &filesystem_;
    uint64_t file_limit_;
    FixedComponents &acceleration_;

  public:
    ElfLoader(Memory &memory, Filesystem &filesystem, uint64_t file_limit, FixedComponents &acceleration)
        : memory_(memory), filesystem_(filesystem), file_limit_(file_limit), acceleration_(acceleration) {}
    Image load(const std::string &path, uint64_t pie_base, bool interpreter = false);
};
uint64_t build_initial_stack(Memory &memory, const char *const *argv, const char *const *envp,
                             const Image &main, uint64_t interpreter_base);
} // namespace operit::arm64
