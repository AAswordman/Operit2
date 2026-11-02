#include "acceleration/fixed_components.h"
#include "acceleration/musl/copy.h"
#include "acceleration/musl/length.h"
#include "acceleration/musl/move.h"
#include "acceleration/musl/set.h"
#include "base/sha256.h"

namespace operit::arm64 {

const FixedManifest *FixedComponents::manifest(const std::vector<uint8_t> &elf) const {
    if (!enabled_)
        return nullptr;
    const auto digest = sha256(elf.data(), elf.size());
    for (const auto &candidate : fixed_musl_manifests)
        if (digest == candidate.sha256)
            return &candidate;
    return nullptr;
}
bool FixedComponents::recognizes(const std::vector<uint8_t> &elf) const {
    return manifest(elf) != nullptr;
}
void FixedComponents::register_image(const std::vector<uint8_t> &elf, uint64_t base,
                                     std::vector<ExecutableSnapshot> executable) {
    const auto *image = manifest(elf);
    if (!image)
        return;
    // The digest authenticates the complete image. Runtime checks cover every
    // instruction the compiled helpers execute, including memmove's memcpy tail.
    std::vector<ExecutableSnapshot> programs;
    for (const auto &symbol : image->symbols) {
        const uint64_t address = base + symbol.value;
        bool found = false;
        for (const auto &snapshot : executable) {
            if (address < snapshot.address || address - snapshot.address > snapshot.bytes.size())
                continue;
            const size_t offset = address - snapshot.address;
            if (symbol.size > snapshot.bytes.size() - offset)
                continue;
            programs.push_back(
                {address, {snapshot.bytes.begin() + offset, snapshot.bytes.begin() + offset + symbol.size}});
            found = true;
            break;
        }
        if (!found)
            return;
    }
    modules_.push_back(Module{base, 0, image, false, std::move(programs)});
    ++generation_;
}
uint64_t FixedComponents::execute(CpuState &cpu, Memory &memory, uint64_t budget) {
    const Memory::AccessGuard access(memory);
    completed_ = 0;
    if (!enabled_ || !budget)
        return 0;
    for (auto &module : modules_) {
        const FixedSymbol *symbol = nullptr;
        for (const auto &candidate : module.manifest->symbols)
            if (cpu.pc == module.base + candidate.value) {
                symbol = &candidate;
                break;
            }
        if (!symbol)
            continue;
        if (module.checked_epoch != memory.code_epoch) {
            module.matches = true;
            for (const auto &snapshot : module.executable) {
                if (!memory.accessible(snapshot.address, snapshot.bytes.size(), exec_perm)) {
                    module.matches = false;
                    break;
                }
                size_t offset = 0;
                while (offset < snapshot.bytes.size()) {
                    const uint64_t address = snapshot.address + offset;
                    const size_t size = std::min<size_t>(snapshot.bytes.size() - offset,
                                                        page_size - address % page_size);
                    if (std::memcmp(memory.translate(address, exec_perm), snapshot.bytes.data() + offset,
                                    size)) {
                        module.matches = false;
                        break;
                    }
                    offset += size;
                }
                if (!module.matches)
                    break;
            }
            module.checked_epoch = memory.code_epoch;
        }
        if (!module.matches)
            return 0;
        const uint64_t length = cpu.x[2];
        if (symbol->function == FixedFunction::Length) {
            // Check eligibility without changing registers or instruction counts.
            // Invalid strings remain the interpreter's faulting responsibility.
            uint64_t address = cpu.x[0];
            size_t remaining = max_io;
            bool terminated = false;
            while (remaining) {
                const size_t count = std::min<size_t>(remaining, page_size - address % page_size);
                if (!memory.accessible(address, count, read_perm))
                    return 0;
                const uint8_t *data;
                try {
                    data = memory.translate(address, read_perm);
                } catch (const MemoryFault &) {
                    return 0;
                }
                if (std::memchr(data, 0, count)) {
                    terminated = true;
                    break;
                }
                address += count;
                remaining -= count;
            }
            if (!terminated)
                return 0;
        } else {
            if (length > max_io || !memory.accessible(cpu.x[0], length, write_perm))
                return 0;
            if ((symbol->function == FixedFunction::Copy || symbol->function == FixedFunction::Move) &&
                !memory.accessible(cpu.x[1], length, read_perm))
                return 0;
        }

        fixed::FixedExecution execution(cpu, memory, *module.manifest, module.base, budget, completed_);
        switch (symbol->function) {
        case FixedFunction::Copy:
            fixed::copy(execution);
            break;
        case FixedFunction::Move:
            fixed::move(execution);
            break;
        case FixedFunction::Set:
            fixed::set(execution);
            break;
        case FixedFunction::Length:
            fixed::length(execution);
            break;
        }
        if (completed_)
            ++calls;
        if (execution.complete)
            bytes += symbol->function == FixedFunction::Length ? cpu.x[0] : length;
        return completed_;
    }
    return 0;
}
} // namespace operit::arm64
