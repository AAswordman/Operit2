#include "elf/loader.h"
#include <set>

namespace operit::arm64 {
namespace {
uint64_t field(const std::vector<uint8_t> &data, size_t offset, unsigned size) {
    if (offset > data.size() || size > data.size() - offset)
        throw Failure("truncated ELF metadata");
    uint64_t value = 0;
    for (unsigned j = 0; j < size; ++j)
        value |= uint64_t(data[offset + j]) << (j * 8);
    return value;
}
struct Segment {
    uint64_t address, offset, file_size, memory_size;
    unsigned permissions;
};
} // namespace
Image ElfLoader::load(const std::string &path, uint64_t pie_base, bool interpreter) {
    auto data = filesystem_.read_file(path, file_limit_);
    if (data.size() < 64 || std::memcmp(data.data(), "\177ELF", 4) || data[4] != 2 || data[5] != 1 ||
        data[6] != 1)
        throw Failure("expected a little-endian ELF64 executable");
    const uint64_t type = field(data, 16, 2);
    if ((type != 2 && type != 3) || field(data, 18, 2) != 183 || field(data, 20, 4) != 1 ||
        field(data, 52, 2) != 64)
        throw Failure("expected an AArch64 ET_EXEC or ET_DYN ELF");
    const uint64_t base = type == 3 ? pie_base : 0;
    const uint64_t phoff = field(data, 32, 8), phent = field(data, 54, 2), phnum = field(data, 56, 2);
    if (phent != 56 || !phnum || phnum > 1024 || phoff > data.size() || phnum * phent > data.size() - phoff)
        throw Failure("invalid ELF program-header table");
    Image image{};
    image.base = base;
    image.phent = phent;
    image.phnum = phnum;
    image.path = path;
    const uint64_t entry = field(data, 24, 8);
    if (entry >= address_limit || base >= address_limit - entry)
        throw Failure("ELF entry address overflow");
    image.entry = base + entry;
    std::vector<Segment> segments;
    std::set<uint64_t> boundaries;
    for (uint64_t j = 0; j < phnum; ++j) {
        const size_t header = phoff + j * phent;
        const uint64_t kind = field(data, header, 4), flags = field(data, header + 4, 4);
        const uint64_t offset = field(data, header + 8, 8), virtual_address = field(data, header + 16, 8);
        const uint64_t file_size = field(data, header + 32, 8), memory_size = field(data, header + 40, 8);
        if (kind == 3) {
            if (interpreter || !image.interpreter.empty() || file_size < 2 || file_size > max_path ||
                offset > data.size() || file_size > data.size() - offset || data[offset + file_size - 1])
                throw Failure("invalid or nested ELF interpreter");
            image.interpreter.assign(reinterpret_cast<char *>(data.data() + offset), file_size - 1);
            if (image.interpreter.front() != '/' || image.interpreter.find('\0') != std::string::npos)
                throw Failure("ELF interpreter must be a guest absolute path");
        }
        if (kind != 1)
            continue;
        if (file_size > memory_size || offset > data.size() || file_size > data.size() - offset ||
            virtual_address >= address_limit || base >= address_limit - virtual_address ||
            !Memory::valid_range(base + virtual_address, memory_size))
            throw Failure("invalid ELF load segment");
        if (!memory_size)
            continue;
        const uint64_t alignment = field(data, header + 48, 8);
        if (alignment > 1 && ((alignment & (alignment - 1)) || alignment > address_limit ||
                              (virtual_address % alignment) != (offset % alignment)))
            throw Failure("invalid ELF segment alignment");
        const uint64_t address = base + virtual_address;
        unsigned permissions =
            ((flags & 4) ? read_perm : 0) | ((flags & 2) ? write_perm : 0) | ((flags & 1) ? exec_perm : 0);
        segments.push_back({address, offset, file_size, memory_size, permissions});
        boundaries.insert(align_down(address));
        boundaries.insert(align_up(address + memory_size));
        image.end = std::max(image.end, address + memory_size);
        if (offset <= phoff && phoff - offset <= file_size && phnum * phent <= file_size - (phoff - offset))
            image.phdr = address + (phoff - offset);
    }
    if (segments.empty() || !image.phdr)
        throw Failure("ELF needs loadable segments and mapped program headers");
    std::sort(segments.begin(), segments.end(),
              [](const Segment &a, const Segment &b) { return a.address < b.address; });
    for (size_t j = 1; j < segments.size(); ++j)
        if (segments[j].address < segments[j - 1].address + segments[j - 1].memory_size)
            throw Failure("overlapping ELF segment byte ranges");
    struct Mapping {
        uint64_t start, length;
        unsigned permissions;
    };
    std::vector<Mapping> mappings;
    for (auto it = boundaries.begin(); it != boundaries.end(); ++it) {
        auto next = std::next(it);
        if (next == boundaries.end())
            break;
        bool covered = false;
        unsigned permissions = 0;
        for (const auto &segment : segments) {
            if (align_down(segment.address) <= *it &&
                align_up(segment.address + segment.memory_size) >= *next) {
                covered = true;
                permissions |= segment.permissions;
            }
        }
        if (covered) {
            memory_.map(*it, *next - *it, read_perm | write_perm);
            mappings.push_back({*it, *next - *it, permissions});
        }
    }
    for (const auto &segment : segments)
        if (segment.file_size)
            memory_.copy_in(segment.address, data.data() + segment.offset, segment.file_size);
    for (const auto &mapping : mappings)
        memory_.protect(mapping.start, mapping.length, mapping.permissions);
    const auto source = filesystem_.resolve(path);
    const auto info = source.stat();
    for (const auto &segment : segments) {
        if (!segment.file_size)
            continue;
        uint64_t start = align_down(segment.address);
        uint64_t size = align_up(segment.address + segment.file_size) - start;
        memory_.describe(
            start, size,
            {source.guest, align_down(segment.offset), uint64_t(info.st_dev), uint64_t(info.st_ino), false});
    }
    if (!memory_.accessible(image.entry, 4, exec_perm))
        throw Failure("ELF entry is not executable");
    // Fingerprint before allocating snapshots: unrecognized ELF mappings can be arbitrarily large.
    acceleration_.register_image_lazy(data, base, [&] {
        std::vector<ExecutableSnapshot> executable;
        for (const auto &mapping : mappings)
            if (mapping.permissions & exec_perm) {
                ExecutableSnapshot snapshot{mapping.start, std::vector<uint8_t>(mapping.length)};
                memory_.copy_out(mapping.start, snapshot.bytes.data(), snapshot.bytes.size(), exec_perm);
                executable.push_back(std::move(snapshot));
            }
        return executable;
    });
    return image;
}
} // namespace operit::arm64
