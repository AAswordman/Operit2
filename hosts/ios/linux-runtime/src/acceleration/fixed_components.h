#pragma once
#include "acceleration/fixed_manifest.h"
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
#include <vector>

namespace operit::arm64 {
inline constexpr uint64_t fixed_copy_loop_offset = 0x183c0 - 0x18290;
struct ExecutableSnapshot {
    uint64_t address;
    std::vector<uint8_t> bytes;
};
class FixedComponents {
    struct Module {
        uint64_t base, checked_epoch = 0;
        const FixedManifest *manifest = nullptr;
        bool matches = false;
        std::vector<ExecutableSnapshot> executable;
        uint64_t checked_generation = 0;
    };
    bool enabled_;
    uint64_t generation_ = 1, completed_ = 0;
    std::vector<Module> modules_;
    const FixedManifest *manifest(const std::vector<uint8_t> &elf) const;
    void register_verified_image(const FixedManifest &image, uint64_t base,
                                 std::vector<ExecutableSnapshot> executable);

  public:
    uint64_t calls = 0, bytes = 0;
    explicit FixedComponents(bool enabled) : enabled_(enabled) {}
    bool active() const {
        return enabled_ && !modules_.empty();
    }
    uint64_t generation() const {
        return generation_;
    }
    bool contains(uint64_t pc) const {
        for (const auto &module : modules_) {
            if (pc < module.base)
                continue;
            const uint64_t offset = pc - module.base;
            for (const auto &symbol : module.manifest->symbols)
                if (offset == symbol.value ||
                    (symbol.function == FixedFunction::Copy && offset == symbol.value + fixed_copy_loop_offset))
                    return true;
        }
        return false;
    }
    bool recognizes(const std::vector<uint8_t> &elf) const;
    void register_image(const std::vector<uint8_t> &elf, uint64_t base,
                        std::vector<ExecutableSnapshot> executable);
    template <typename SnapshotFactory>
    void register_image_lazy(const std::vector<uint8_t> &elf, uint64_t base, SnapshotFactory &&snapshots) {
        if (const auto *image = manifest(elf))
            register_verified_image(*image, base, snapshots());
    }
    uint64_t completed_instructions() const {
        return completed_;
    }
    uint64_t execute(CpuState &cpu, Memory &memory, uint64_t budget = UINT64_MAX);
};
} // namespace operit::arm64
