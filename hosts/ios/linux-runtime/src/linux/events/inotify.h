#pragma once
#include "base/fd.h"
#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>

namespace operit::arm64 {
class Filesystem;
struct Path;

// Linux values, independent of the host's fcntl, poll and event headers.
namespace inotify {
constexpr uint32_t access = 0x00000001, modify = 0x00000002, attrib = 0x00000004;
constexpr uint32_t close_write = 0x00000008, close_nowrite = 0x00000010, open = 0x00000020;
constexpr uint32_t moved_from = 0x00000040, moved_to = 0x00000080;
constexpr uint32_t create = 0x00000100, remove = 0x00000200;
constexpr uint32_t delete_self = 0x00000400, move_self = 0x00000800;
constexpr uint32_t unmount = 0x00002000, overflow = 0x00004000, ignored = 0x00008000;
constexpr uint32_t only_directory = 0x01000000, dont_follow = 0x02000000;
constexpr uint32_t exclude_unlinked = 0x04000000, mask_create = 0x10000000, mask_add = 0x20000000;
constexpr uint32_t is_directory = 0x40000000, one_shot = 0x80000000;
constexpr uint32_t all_events = 0x00000fff;
constexpr uint32_t known_mask = all_events | unmount | overflow | ignored | only_directory |
                                dont_follow | exclude_unlinked | mask_create | mask_add |
                                is_directory | one_shot;
constexpr uint32_t nonblock = 2048, cloexec = 524288;
constexpr size_t event_header_size = 16;
} // namespace inotify

struct InotifyLimits {
    size_t queued_events = 16384;
    size_t watches = 8192;
    // Bound each directory census, and retained unlinked children, on Darwin.
    size_t snapshot_entries = 65536;
};

// One instance per guest open file description, shared by dup/fork/SCM_RIGHTS.
// Host descriptors are CLOEXEC; the readiness pipe stays NONBLOCK regardless
// of guest flags.
// The native handle is a readiness marker: NEVER read event bytes from it.
// Darwin uses kqueue plus snapshots. Rename pairs of observed vnodes use real
// NOTE_RENAME; move-ins from unobserved parents use inode birth/link metadata.
// Arbitrary operations entirely outside the observed set cannot be replayed.
// Observed transient names that disappeared between censuses report OVERFLOW.
class InotifyFile final {
    friend class InotifyOpenFile;
    struct Impl;
    std::unique_ptr<Impl> impl_;

  public:
    explicit InotifyFile(uint32_t flags = 0);
    InotifyFile(uint32_t flags, InotifyLimits limits);
    ~InotifyFile();
    InotifyFile(const InotifyFile &) = delete;
    InotifyFile &operator=(const InotifyFile &) = delete;

    static void validate_flags(uint32_t flags);
    static void validate_mask(uint32_t mask);
    // Resolves through the guest namespace, including IN_DONT_FOLLOW.
    int32_t add_watch(const Filesystem &filesystem, const std::string &pathname, uint32_t mask,
                      const std::string &base = "/");
    // A resolved Path must have followed its final symlink unless DONT_FOLLOW.
    // VirtualNode paths have no host change source and return EOPNOTSUPP.
    int32_t add_watch(const Path &path, uint32_t mask);
    void remove_watch(int32_t wd);
    FD duplicate_handle() const;
    int native_handle() const noexcept;

    // Whole little-endian Linux records, with NUL names padded to 16 bytes.
    // Empty queue: try_read returns nullopt, read throws GuestError{11}.
    // Too small for the first queued record: EINVAL, without consuming it.
    // Validate guest memory BEFORE calling: a successful read consumes records.
    std::optional<size_t> try_read(void *buffer, size_t capacity);
    size_t read(void *buffer, size_t capacity);
    short poll_events(short requested);
    uint32_t available(); // Linux FIONREAD: event bytes, not marker pipe bytes.
    size_t watch_count() const;
    void pump();
    // Serial safe point, including when every guest task is blocked. No host
    // thread touches guest memory or Scheduler. Backend failure becomes a
    // queued OVERFLOW and sticky POLLERR/read errno on the affected instance.
    static void service_all();
};

// Tracks the dentry used by a guest data-open, independently of path display
// strings. Keep ONE reference in OpenDescription, so dup/fork/SCM_RIGHTS share
// OPEN and final CLOSE. Linux already reports these through native inotify.
class InotifyOpenFile final {
    struct Impl;
    std::unique_ptr<Impl> impl_;
    explicit InotifyOpenFile(std::unique_ptr<Impl> impl);
    void emit(uint32_t event) noexcept;
    static void overflow_all() noexcept;

  public:
    static std::shared_ptr<InotifyOpenFile> opened(const Path &path, int descriptor,
                                                  bool writable, bool anonymous = false) noexcept;
    ~InotifyOpenFile();
    InotifyOpenFile(const InotifyOpenFile &) = delete;
    InotifyOpenFile &operator=(const InotifyOpenFile &) = delete;
    void accessed() noexcept;
    // Call only AFTER successful namespace mutations. Pinned parent + name
    // distinguishes hardlinks, and preserves the old dentry after unlink.
    static void unlinked(const Path &path) noexcept;
    static void renamed(const Path &source, const Path &target) noexcept;
    static void exchanged(const Path &source, const Path &target) noexcept;
};

} // namespace operit::arm64
