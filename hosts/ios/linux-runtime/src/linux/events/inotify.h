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
#ifdef __APPLE__
constexpr uint32_t supported_events = all_events & ~(access | open | close_write | close_nowrite);
#else
constexpr uint32_t supported_events = all_events;
#endif
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

// Integration (implemented by the syscall/descriptor owner, not this module):
//  * OpenDescription: shared_ptr<InotifyFile> inotify; forward-declare the class.
//  * arm64 26: construct with uint32_t(flags), insert(duplicate_handle(),
//    "", flags), set description->inotify and scheduler_io=true.
//    Validate the syscall's int/u32 arguments after truncation to Linux widths.
//  * arm64 27/28: get(fd) first (EBADF), require description->inotify (EINVAL),
//    then add_watch(filesystem, pathname, mask, cwd) / remove_watch(int32_t(wd)).
//  * io: before zero-length/general stream handling, reject writes (EBADF) and
//    positioned reads (ESPIPE); in its existing attempt lambda, try_read into a
//    temporary buffer and copy only returned bytes. nullopt uses the existing
//    NONBLOCK/may_block check, signal interruption and Scheduler::block callback.
//    Retain the shared description in the callback, as other stream reads do.
//  * poll/select: call poll_events(native_events) in the per-description loop.
//    Existing EventPoll::control works with the duplicated marker pipe as-is.
//  * epoll wait's attempt: service_all() before EventPoll::wait(maximum, 0).
//  * safe points: service_all() at the START of Scheduler::poll_waiting(), and
//    Machine::run's coordinator loop before selecting/running the next batch.
//    A safe point only in runnable guest syscalls cannot wake an idle watcher.
//  * FIONREAD: available(); descriptor_stat: add to the anon_inode 0600 branch.
//    Keep Descriptor::path empty. process_fd_link's target is "anon_inode:inotify";
//    open_stream rejects reopening with ENXIO alongside counter/poll/timer.
//    Include in existing anon-FD guards where applicable (lseek/flock/mmap etc.).
//    F_SETFL/FIONBIO must retain scheduler_io's native NONBLOCK behavior.
//  * build: add linux/events/inotify.cpp to the runtime library sources.
} // namespace operit::arm64
