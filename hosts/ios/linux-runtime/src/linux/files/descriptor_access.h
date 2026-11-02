#pragma once
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "fs/filesystem.h"
#include "linux/files/open_how.h"

namespace operit::arm64::files {
inline bool path_only(const OpenDescription &description) {
    return uint64_t(description.flags) & arm64_open::path;
}
inline bool descriptor_readable(const Descriptor &entry) {
    const auto access = uint64_t(entry.description->flags) & arm64_open::access_mode;
    return !path_only(*entry.description) && (access == 0 || access == 2);
}
inline bool descriptor_writable(const Descriptor &entry) {
    const auto access = uint64_t(entry.description->flags) & arm64_open::access_mode;
    return !path_only(*entry.description) && (access == 1 || access == 2);
}
inline void require_descriptor_io(const Descriptor &entry, bool writing) {
    if (!(writing ? descriptor_writable(entry) : descriptor_readable(entry)))
        guest_error(9);
}
// Check before buffer validation, a zero-length fast path, or native I/O.
inline void require_data_descriptor(const Descriptor &entry) {
    if (path_only(*entry.description))
        guest_error(9);
}
inline void check_path_fcntl(const Descriptor &entry, uint64_t command) {
    if (!path_only(*entry.description))
        return;
    switch (command) {
    case 0:    // F_DUPFD
    case 1:    // F_GETFD
    case 2:    // F_SETFD
    case 3:    // F_GETFL
    case 1030: // F_DUPFD_CLOEXEC
        return;
    default:
        guest_error(9);
    }
}
// Display-only recovery. Path resolution itself must continue using the FD.
std::string directory_guest_name(const Filesystem &filesystem, const Path &directory);
} // namespace operit::arm64::files
