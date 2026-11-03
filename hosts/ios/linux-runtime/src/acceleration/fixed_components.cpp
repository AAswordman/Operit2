#include "acceleration/fixed_components.h"
#include "acceleration/musl/copy.h"
#include "acceleration/musl/length.h"
#include "acceleration/musl/move.h"
#include "acceleration/musl/set.h"
#include "acceleration/scalar_ops.h"
#include "base/sha256.h"

namespace operit::arm64 {

const FixedManifest *FixedComponents::manifest(const std::vector<uint8_t> &elf) const {
    if (!enabled_)
        return nullptr;
    bool candidate_size = false;
    for (const auto &candidate : fixed_musl_manifests)
        candidate_size |= candidate.file_size == elf.size();
    if (!candidate_size)
        return nullptr;
    const auto digest = sha256(elf.data(), elf.size());
    for (const auto &candidate : fixed_musl_manifests)
        if (candidate.file_size == elf.size() && digest == candidate.sha256)
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
    register_verified_image(*image, base, std::move(executable));
}
void FixedComponents::register_verified_image(const FixedManifest &image, uint64_t base,
                                               std::vector<ExecutableSnapshot> executable) {
    // The digest authenticates the complete image. Runtime checks cover every
    // instruction the compiled helpers execute, including memmove's memcpy tail.
    std::vector<ExecutableSnapshot> programs;
    for (const auto &symbol : image.symbols) {
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
    modules_.push_back(Module{base, 0, &image, false, std::move(programs)});
    ++generation_;
}
uint64_t FixedComponents::execute(CpuState &cpu, Memory &memory, uint64_t budget) {
    completed_ = 0;
    if (!enabled_ || !budget)
        return 0;
    for (auto &module : modules_) {
        const FixedSymbol *symbol = nullptr;
        bool copy_loop = false;
        for (const auto &candidate : module.manifest->symbols) {
            if (cpu.pc == module.base + candidate.value) {
                symbol = &candidate;
                break;
            }
            if (candidate.function == FixedFunction::Copy &&
                cpu.pc == module.base + candidate.value + fixed_copy_loop_offset) {
                symbol = &candidate;
                copy_loop = true;
                break;
            }
        }
        if (!symbol)
            continue;
        // Hot bounded copies need only their code/data pages, not a stop of
        // every guest CPU. Cold pages, COW, large spans and strlen retain the
        // original exclusive path. Lock every authenticated helper symbol:
        // memmove can tail-call memcpy and self-modifying destinations must
        // share the same physical lock as their executable alias.
        Memory::PreparedAccess prepared(memory);
        const uint64_t length = cpu.x[2];
        const auto within_page = [length](uint64_t address) {
            return !length || length <= page_size - address % page_size;
        };
        const bool reads_source = symbol->function == FixedFunction::Copy ||
                                  symbol->function == FixedFunction::Move;
        uint64_t helper_budget = budget;
        uint64_t copy_source = 0, copy_destination = 0, copy_bytes = 0;
        if (copy_loop) {
            // The body writes and reads 64 bytes in ten guest instructions.
            // Restrict each resumed slice to already resident single pages;
            // a boundary, COW or fault falls back to the ordinary interpreter.
            copy_source = cpu.x[1] + 16;
            copy_destination = cpu.x[3] + 16;
            const uint64_t iterations = std::min({budget / 10,
                (page_size - copy_source % page_size) / 64,
                (page_size - copy_destination % page_size) / 64});
            if (!iterations)
                return 0;
            copy_bytes = iterations * 64;
            helper_budget = iterations * 10;
        }
        // The existing cross-page copy API uses a raw-span writer guard.
        // Keep it on the exclusive fallback rather than upgrading while the
        // prepared reader/page locks are held.
        bool ready = copy_loop || (symbol->function != FixedFunction::Length && length <= max_io &&
                                   within_page(cpu.x[0]) && (!reads_source || within_page(cpu.x[1])));
        if (ready) {
            for (const auto &snapshot : module.executable) {
                if (!prepared.add(snapshot.address, snapshot.bytes.size(), exec_perm)) {
                    ready = false;
                    break;
                }
            }
        }
        if (ready)
            ready = prepared.add(copy_loop ? copy_destination : cpu.x[0], copy_loop ? copy_bytes : length,
                                 write_perm);
        if (ready && reads_source)
            ready = prepared.add(copy_loop ? copy_source : cpu.x[1], copy_loop ? copy_bytes : length,
                                 read_perm);
        if (ready)
            ready = prepared.lock();
        std::optional<Memory::AccessGuard> exclusive;
        if (!ready) {
            prepared.cancel();
            if (copy_loop)
                return 0;
            exclusive.emplace(memory);
        }
        if (module.checked_epoch != memory.code_epoch) {
            const uint64_t checked_epoch = memory.code_epoch.load(std::memory_order_relaxed);
            module.matches = true;
            for (const auto &snapshot : module.executable) {
                size_t offset = 0;
                while (offset < snapshot.bytes.size()) {
                    const uint64_t address = snapshot.address + offset;
                    const size_t size = std::min<size_t>(snapshot.bytes.size() - offset,
                                                        page_size - address % page_size);
                    uint64_t validated_epoch = 0;
                    try {
                        if (!memory.validate_code(address, snapshot.bytes.data() + offset, size, validated_epoch)) {
                            module.matches = false;
                            break;
                        }
                    } catch (const MemoryFault &) {
                        module.matches = false;
                        break;
                    }
                    offset += size;
                }
                if (!module.matches)
                    break;
            }
            // Use the epoch before validation. Writes to unrelated code can
            // only force another check; they cannot relabel unchecked bytes.
            module.checked_epoch = checked_epoch;
        }
        if (!module.matches)
            return 0;
        if (copy_loop) {
            const uint64_t available = (cpu.x[2] >> 6) + bool(cpu.x[2] & 63);
            const uint64_t iterations = std::min(helper_budget / 10, std::max<uint64_t>(1, available));
            const uint64_t bytes = iterations * 64;
            if (prepared.copy_pipeline(copy_destination, copy_source, cpu.x.data() + 6,
                                       cpu.x.data() + 6, bytes)) {
                cpu.x[1] += bytes;
                cpu.x[3] += bytes;
                cpu.x[2] = scalar::arithmetic<64>(cpu, cpu.x[2] - (iterations - 1) * 64, 64,
                                                  decoded::set_flags | decoded::subtract);
                cpu.pc = module.base + symbol->value + fixed_copy_loop_offset +
                         (scalar::condition(cpu, 8) ? 0 : 0x28);
                completed_ = iterations * 10;
                ++calls;
                return completed_;
            }
        }
        if (symbol->function == FixedFunction::Length) {
            // Check eligibility without changing registers or instruction counts.
            // Invalid strings remain the interpreter's faulting responsibility.
            uint64_t address = cpu.x[0];
            size_t remaining = max_io;
            bool terminated = false;
            while (remaining) {
                const size_t count = std::min<size_t>(remaining, page_size - address % page_size);
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
        } else if (!ready) {
            if (length > max_io || !memory.accessible(cpu.x[0], length, write_perm))
                return 0;
            if (reads_source && !memory.accessible(cpu.x[1], length, read_perm))
                return 0;
        }

        fixed::FixedExecution execution(cpu, memory, *module.manifest, module.base, helper_budget, completed_);
        switch (symbol->function) {
        case FixedFunction::Copy:
            fixed::copy(execution, copy_loop);
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
