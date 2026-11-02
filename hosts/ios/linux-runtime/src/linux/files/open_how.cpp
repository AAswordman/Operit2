#include "linux/files/open_how.h"
#include "base/support.h"
#include "memory/address_space.h"
#include <algorithm>
#include <array>

namespace operit::arm64::files {
namespace {
uint64_t little_endian_u64(const uint8_t *bytes) {
    uint64_t result = 0;
    for (unsigned i = 0; i < 8; ++i)
        result |= uint64_t(bytes[i]) << (i * 8);
    return result;
}
} // namespace
linux_abi::OpenHow read_open_how(Memory &memory, uint64_t address, uint64_t size) {
    constexpr uint64_t version_zero = sizeof(linux_abi::OpenHow);
    if (size < version_zero)
        guest_error(22);
    if (size > page_size)
        guest_error(7);
    // Match copy_struct_from_user: validate extension bytes before copying v0.
    std::array<uint8_t, 256> extra{};
    for (uint64_t offset = version_zero; offset < size;) {
        const size_t count = size_t(std::min<uint64_t>(size - offset, extra.size()));
        if (address > UINT64_MAX - offset)
            guest_error(14);
        memory.copy_out(address + offset, extra.data(), count);
        if (std::any_of(extra.begin(), extra.begin() + count, [](uint8_t byte) { return byte != 0; }))
            guest_error(7);
        offset += count;
    }
    std::array<uint8_t, version_zero> bytes{};
    memory.copy_out(address, bytes.data(), bytes.size());
    linux_abi::OpenHow result;
    result.flags = little_endian_u64(bytes.data());
    result.mode = little_endian_u64(bytes.data() + 8);
    result.resolve = little_endian_u64(bytes.data() + 16);
    validate_open_how(result);
    return result;
}
void validate_open_how(const linux_abi::OpenHow &how) {
    using namespace arm64_open;
    if ((how.flags & ~known) || (how.resolve & ~path_resolution::known))
        guest_error(22);
    if ((how.resolve & path_resolution::beneath) && (how.resolve & path_resolution::in_root))
        guest_error(22);
    if (how.mode & ~uint64_t{07777})
        guest_error(22);
    if (!(how.flags & (create | temporary)) && how.mode)
        guest_error(22);
    if ((how.flags & (create | directory)) == (create | directory))
        guest_error(22);
    if (how.flags & temporary) {
        const auto access = how.flags & access_mode;
        if (!(how.flags & directory) || (access != 1 && access != 2))
            guest_error(22);
    }
    if ((how.flags & path) && (how.flags & ~(path | close_on_exec | directory | no_follow)))
        guest_error(22);
    if ((how.resolve & path_resolution::cached) && (how.flags & (create | truncate | temporary)))
        guest_error(11);
}
OpenRequest resolve_openat2(const Filesystem &filesystem, const std::string &name,
                           const Path &directory, const linux_abi::OpenHow &how) {
    validate_open_how(how);
    using namespace arm64_open;
    const bool exclusive_create = (how.flags & (create | exclusive)) == (create | exclusive);
    const bool follow = !(how.flags & no_follow) && !exclusive_create;
    const auto intent = exclusive_create ? Filesystem::ResolveIntent::CreateExclusive
                        : how.flags & create ? Filesystem::ResolveIntent::CreateFile
                                             : Filesystem::ResolveIntent::Lookup;
    OpenRequest result;
    result.path = filesystem.resolve_at(name, directory, how.resolve, follow, intent);
    result.flags = how.flags;
    if (!(result.flags & path))
        result.flags |= large_file;
    if (result.flags & full_sync)
        result.flags |= data_sync;
    result.mode = mode_t(how.mode);
    struct stat info{};
    bool exists = true;
    try {
        info = result.path.stat();
    } catch (const GuestError &error) {
        if (error.number != 2)
            throw;
        exists = false;
    }
    if (!exists && !(how.flags & create))
        guest_error(2);
    if (exists && S_ISLNK(info.st_mode) && !(how.flags & path) && !follow)
        guest_error(how.flags & arm64_open::directory ? 20 : 40);
    if ((how.flags & arm64_open::directory) && exists && !S_ISDIR(info.st_mode))
        guest_error(20);
    if ((how.flags & create) && exists && S_ISDIR(info.st_mode))
        guest_error(21);
    return result;
}
} // namespace operit::arm64::files
