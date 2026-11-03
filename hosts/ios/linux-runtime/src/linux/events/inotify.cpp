#include "linux/events/inotify.h"
#include "base/support.h"
#include "fs/filesystem.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <array>
#include <cerrno>
#include <chrono>
#include <climits>
#include <cstring>
#include <deque>
#include <dirent.h>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <poll.h>
#include <set>
#include <sys/stat.h>
#include <utility>
#include <vector>
#ifdef __APPLE__
#include <sys/event.h>
#else
#include <sys/inotify.h>
#endif

namespace operit::arm64 {
namespace {
using Identity = std::pair<uint64_t, uint64_t>;
using Clock = std::chrono::steady_clock;
Identity identity(const struct stat &info) {
    return {uint64_t(info.st_dev), uint64_t(info.st_ino)};
}
FD duplicate(int descriptor) {
    FD result(::fcntl(descriptor, F_DUPFD_CLOEXEC, 0));
    check_host(result.get());
    return result;
}
void configure(int descriptor) {
    check_host(::fcntl(descriptor, F_SETFD, FD_CLOEXEC));
    int flags = ::fcntl(descriptor, F_GETFL);
    check_host(flags);
    check_host(::fcntl(descriptor, F_SETFL, flags | O_NONBLOCK));
}
void store32(uint8_t *output, uint32_t value) {
    for (unsigned n = 0; n != 4; ++n)
        output[n] = uint8_t(value >> (8 * n));
}
struct Event {
    int32_t wd;
    uint32_t mask, cookie;
    std::string name;
    size_t name_size() const {
        return name.empty() ? 0 : (name.size() + 16) & ~size_t{15};
    }
    size_t size() const {
        return inotify::event_header_size + name_size();
    }
};
struct Instances {
    std::mutex mutex;
    std::set<InotifyFile *> files;
#ifdef __APPLE__
    std::set<InotifyOpenFile *> opens;
#endif
};
Instances &instances() {
    static Instances registry;
    return registry;
}
#ifdef __APPLE__
// collect() runs with an instance mutex held. This separate count lock never
// remains held while acquiring an instance lock (registry -> instance -> count).
struct GuestOpens {
    std::mutex mutex;
    std::map<Identity, size_t> counts;
};
GuestOpens &guest_opens() {
    static GuestOpens registry;
    return registry;
}
bool guest_open(const Identity &key) {
    auto &registry = guest_opens();
    std::lock_guard<std::mutex> lock(registry.mutex);
    return registry.counts.count(key) != 0;
}
bool same_time(const timespec &a, const timespec &b) {
    return a.tv_sec == b.tv_sec && a.tv_nsec == b.tv_nsec;
}
bool same_node(const struct stat &a, const struct stat &b, bool pinned = false) {
    return identity(a) == identity(b) && (a.st_mode & S_IFMT) == (b.st_mode & S_IFMT) &&
           (pinned || same_time(a.st_birthtimespec, b.st_birthtimespec));
}
bool born_before(const struct stat &info, const timespec &time) {
    const auto &birth = info.st_birthtimespec;
    return birth.tv_sec > 0 && (birth.tv_sec < time.tv_sec ||
                               (birth.tv_sec == time.tv_sec && birth.tv_nsec < time.tv_nsec));
}
bool attributes_changed(const struct stat &a, const struct stat &b) {
    return a.st_mode != b.st_mode || a.st_uid != b.st_uid || a.st_gid != b.st_gid ||
           (!S_ISDIR(b.st_mode) && a.st_nlink != b.st_nlink) || a.st_flags != b.st_flags;
}
uint32_t type_flag(const struct stat &info) {
    return S_ISDIR(info.st_mode) ? inotify::is_directory : 0;
}
#endif
} // namespace

struct InotifyFile::Impl {
    mutable std::mutex mutex;
    InotifyLimits limits;
    FD reader, writer, backend;
    std::deque<Event> queue;
    size_t queued_bytes = 0;
    bool overflow_queued = false;
    int failure = 0;
#ifdef __APPLE__
    struct Vnode {
        FD descriptor;
        Identity key;
        uint64_t token;
    };
    struct Child {
        struct stat info{};
        std::shared_ptr<Vnode> node;
    };
    using Children = std::map<std::string, Child>;
    struct Detached {
        std::string name;
        Child child;
    };
    struct Watch {
        uint32_t mask;
        Child root;
        Children children;
        std::vector<Detached> unlinked;
        timespec sampled{};
    };
    struct Frame {
        int32_t wd;
        struct stat root{};
        Children children;
        bool scanned = false;
        timespec sampled{};
    };
    struct Change {
        int32_t wd;
        std::string name;
        Child child;
        bool matched = false;
    };
    struct PendingMove {
        uint32_t cookie;
        Clock::time_point expires;
    };
    std::map<int32_t, Watch> watches;
    std::map<Identity, std::weak_ptr<Vnode>> nodes;
    std::map<uint64_t, std::weak_ptr<Vnode>> tokens;
    std::map<Identity, PendingMove> moves;
    uint64_t next_token = 1;
    uint32_t next_wd = 1, next_cookie = 1;
    Clock::time_point next_census{};
    using Hints = std::map<Identity, uint32_t>;
#else
    // Native Linux supplies exact OPEN/CLOSE/ACCESS and rename semantics. Events
    // still go through the same bounded queue and readiness marker as Darwin.
    std::map<int32_t, Identity> watches;
#endif

    explicit Impl(InotifyLimits value) : limits(value) {
        if (limits.queued_events > size_t(INT_MAX) || limits.watches > size_t(INT_MAX) ||
            !limits.snapshot_entries)
            guest_error(22);
        int descriptors[2];
        check_host(::pipe(descriptors));
        reader = FD(descriptors[0]);
        writer = FD(descriptors[1]);
        configure(reader.get());
        configure(writer.get());
#ifdef __APPLE__
        backend = FD(::kqueue());
        check_host(backend.get());
        check_host(::fcntl(backend.get(), F_SETFD, FD_CLOEXEC));
#else
        backend = FD(::inotify_init1(IN_NONBLOCK | IN_CLOEXEC));
        check_host(backend.get());
#endif
    }
    void signal() {
        const uint8_t marker = 1;
        ssize_t result;
        do {
            result = ::write(writer.get(), &marker, 1);
        } while (result < 0 && errno == EINTR);
        if (result < 0 && errno != EAGAIN && errno != EWOULDBLOCK)
            guest_error(linux_errno(errno));
    }
    void append(Event event) {
        const bool empty = queue.empty();
        const size_t bytes = event.size();
        queue.push_back(std::move(event));
        queued_bytes += bytes;
        if (empty)
            signal();
    }
    void overflow() {
        if (!overflow_queued) {
            append({-1, inotify::overflow, 0, {}});
            overflow_queued = true;
        }
    }
    void enqueue(Event event) {
        if (!queue.empty()) {
            const auto &last = queue.back();
            if (!(last.mask & inotify::ignored) && last.wd == event.wd && last.mask == event.mask &&
                last.cookie == event.cookie && last.name == event.name)
                return;
        }
        if (queue.size() >= limits.queued_events) {
            overflow();
            return;
        }
        append(std::move(event));
    }
    void backend_failure(int error) {
        if (!failure)
            failure = error;
        overflow();
    }
    void clear_marker() {
        // reader is permanently NONBLOCK. Keep the drain under the queue lock
        // so no producer can publish a new event between empty-test and drain.
        uint8_t bytes[64];
        for (;;) {
            ssize_t count = ::read(reader.get(), bytes, sizeof(bytes));
            if (count > 0)
                continue;
            if (count < 0 && errno == EINTR)
                continue;
            if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK)
                guest_error(linux_errno(errno));
            break;
        }
    }
    std::optional<size_t> consume(void *buffer, size_t capacity) {
        if (queue.empty()) {
            if (failure)
                guest_error(failure);
            return {};
        }
        if (queue.front().size() > capacity)
            guest_error(22);
        if (!buffer)
            guest_error(14);
        auto *output = static_cast<uint8_t *>(buffer);
        size_t copied = 0;
        while (!queue.empty() && queue.front().size() <= capacity - copied) {
            const auto &event = queue.front();
            size_t size = event.size();
            std::memset(output + copied, 0, size);
            store32(output + copied, uint32_t(event.wd));
            store32(output + copied + 4, event.mask);
            store32(output + copied + 8, event.cookie);
            store32(output + copied + 12, uint32_t(event.name_size()));
            if (!event.name.empty())
                std::memcpy(output + copied + 16, event.name.data(), event.name.size());
            if (event.mask & inotify::overflow)
                overflow_queued = false;
            copied += size;
            queued_bytes -= size;
            queue.pop_front();
        }
        if (queue.empty())
            clear_marker();
        return copied;
    }
#ifdef __APPLE__
    std::shared_ptr<Vnode> monitor(FD descriptor, const struct stat &info) {
        const auto key = identity(info);
        auto found = nodes.find(key);
        if (found != nodes.end())
            if (auto node = found->second.lock())
                return node;
        auto node = std::make_shared<Vnode>(Vnode{std::move(descriptor), key, next_token++});
        // Tokens, unlike host FD numbers, are never reused by a late kevent.
        struct kevent event;
        EV_SET(&event, node->descriptor.get(), EVFILT_VNODE, EV_ADD | EV_CLEAR,
               NOTE_WRITE | NOTE_EXTEND | NOTE_ATTRIB | NOTE_LINK | NOTE_RENAME | NOTE_DELETE | NOTE_REVOKE,
               0, reinterpret_cast<void *>(uintptr_t(node->token)));
        check_host(::kevent(backend.get(), &event, 1, nullptr, 0, nullptr));
        nodes[key] = node;
        tokens[node->token] = node;
        return node;
    }
    Child open_child(int parent, const std::string &name, const struct stat &info) {
        Child child{info, {}};
        auto found = nodes.find(identity(info));
        if (found != nodes.end())
            child.node = found->second.lock();
        if (child.node)
            return child;
        // Darwin rejects O_SYMLINK | O_NOFOLLOW. O_SYMLINK itself pins the
        // final symlink rather than following it; all other nodes use NOFOLLOW.
        int flags = O_EVTONLY | O_CLOEXEC | (S_ISLNK(info.st_mode) ? O_SYMLINK : O_NOFOLLOW);
        FD descriptor(::openat(parent, name.c_str(), flags));
        if (descriptor.get() < 0)
            guest_error(linux_errno(errno));
        struct stat pinned{};
        check_host(::fstat(descriptor.get(), &pinned));
        if (!same_node(pinned, info))
            guest_error(11); // Name replaced between fstatat and openat.
        child.node = monitor(std::move(descriptor), pinned);
        return child;
    }
    Children census(const Watch &watch, bool initial) {
        // A new directory description is essential: dup shares readdir offsets.
        int descriptor = ::openat(watch.root.node->descriptor.get(), ".", O_RDONLY | O_DIRECTORY | O_CLOEXEC);
        if (descriptor < 0)
            guest_error(linux_errno(errno));
        DIR *raw = ::fdopendir(descriptor);
        if (!raw) {
            const int error = errno;
            ::close(descriptor);
            guest_error(linux_errno(error));
        }
        struct Directory {
            DIR *value;
            ~Directory() {
                ::closedir(value);
            }
        } directory{raw};
        Children result;
        for (;;) {
            errno = 0;
            auto *entry = ::readdir(raw);
            if (!entry) {
                if (errno)
                    guest_error(linux_errno(errno));
                break;
            }
            std::string name(entry->d_name);
            if (name == "." || name == "..")
                continue;
            if (result.size() == limits.snapshot_entries)
                guest_error(28);
            struct stat info{};
            if (::fstatat(descriptor, name.c_str(), &info, AT_SYMLINK_NOFOLLOW) < 0) {
                if (!initial && errno == ENOENT) {
                    overflow();
                    continue;
                }
                guest_error(linux_errno(errno));
            }
            auto old = watch.children.find(name);
            if (old != watch.children.end() && same_node(old->second.info, info, bool(old->second.node))) {
                result.emplace(std::move(name), Child{info, old->second.node});
                continue;
            }
            try {
                auto child = open_child(descriptor, name, info);
                result.emplace(std::move(name), std::move(child));
            } catch (const GuestError &error) {
                // Periodic stat snapshots still observe persistent changes if a
                // child cannot be pinned. Report loss instead of inventing a
                // fine-grained event from an incomplete native subscription.
                (void)error;
                if (!initial)
                    overflow();
                result.emplace(std::move(name), Child{info, {}});
            }
        }
        return result;
    }
    bool drain(Hints &hints) {
        bool changed = false;
        std::array<struct kevent, 128> events{};
        timespec immediate{};
        for (unsigned batch = 0; batch < 64; ++batch) {
            int count = ::kevent(backend.get(), nullptr, 0, events.data(), int(events.size()), &immediate);
            if (count < 0 && errno == EINTR)
                continue;
            check_host(count);
            if (!count)
                break;
            changed = true;
            for (int n = 0; n != count; ++n) {
                auto token = tokens.find(reinterpret_cast<uintptr_t>(events[n].udata));
                if (token == tokens.end())
                    continue;
                auto node = token->second.lock();
                if (!node)
                    continue;
                if (events[n].flags & EV_ERROR) {
                    backend_failure(linux_errno(int(events[n].data)));
                    continue;
                }
                hints[node->key] |= events[n].fflags;
            }
            if (batch == 63)
                overflow();
        }
        return changed;
    }
    uint32_t cookie() {
        uint32_t result = next_cookie++;
        if (!result)
            result = next_cookie++;
        return result;
    }
    void retire(int32_t wd) {
        auto found = watches.find(wd);
        if (found == watches.end())
            return;
        watches.erase(found);
        enqueue({wd, inotify::ignored, 0, {}});
    }
    void emit(int32_t wd, uint32_t event, const std::string &name = {}, uint32_t move_cookie = 0) {
        auto found = watches.find(wd);
        if (found == watches.end())
            return;
        uint32_t bits = (event & found->second.mask & inotify::all_events) | (event & inotify::unmount);
        if (!bits)
            return;
        if (!(bits & (inotify::move_self | inotify::delete_self)))
            bits |= event & inotify::is_directory;
        const bool once = found->second.mask & inotify::one_shot;
        enqueue({wd, bits, move_cookie, name});
        if (once)
            retire(wd);
    }
    void metadata(std::vector<Event> &events, int32_t wd, const std::string &name,
                  const struct stat &before, const struct stat &after, uint32_t hint) {
        uint32_t type = type_flag(after);
        const bool written = (hint & (NOTE_WRITE | NOTE_EXTEND)) || before.st_size != after.st_size ||
                             (!(hint & NOTE_ATTRIB) && !same_time(before.st_mtimespec, after.st_mtimespec));
        // Timestamp setters generate ATTRIB. Do not mistake their mtime delta
        // for a content write when kqueue has supplied the precise cause.
        if (!S_ISDIR(after.st_mode) && written)
            events.push_back({wd, inotify::modify | type, 0, name});
        // APFS directory link counts change even when a regular file is
        // added. Linux reports CREATE/DELETE for that, not a parent ATTRIB.
        const uint32_t attributes = NOTE_ATTRIB | (S_ISDIR(after.st_mode) ? 0 : NOTE_LINK);
        if (attributes_changed(before, after) || (hint & attributes))
            events.push_back({wd, inotify::attrib | type, 0, name});
    }
    void collect(const std::set<Identity> &exchanged) {
        Hints hints;
        // Darwin RENAME_SWAP posts NOTE_RENAME for only the source vnode.
        // A successful guest exchange confirms that both inodes moved; supply
        // the missing reverse hint before correlating the snapshot changes.
        for (const auto &key : exchanged)
            hints[key] |= NOTE_RENAME;
        drain(hints);
        const auto now = Clock::now();
        if (hints.empty() && now < next_census)
            return;
        next_census = now + std::chrono::milliseconds(50);
        for (auto it = moves.begin(); it != moves.end();) {
            if (now >= it->second.expires)
                it = moves.erase(it);
            else
                ++it;
        }
        std::vector<Frame> frames;
        for (auto &[wd, watch] : watches) {
            Frame frame{wd, {}, {}, false, {}};
            check_host(::clock_gettime(CLOCK_REALTIME, &frame.sampled));
            if (::fstat(watch.root.node->descriptor.get(), &frame.root) < 0) {
                overflow();
                continue;
            }
            if (S_ISDIR(frame.root.st_mode) && frame.root.st_nlink &&
                !(hints[identity(frame.root)] & NOTE_REVOKE)) {
                try {
                    check_host(::clock_gettime(CLOCK_REALTIME, &frame.sampled));
                    frame.children = census(watch, false);
                    frame.scanned = true;
                } catch (const GuestError &) {
                    // Preserve the last complete census on permission/FD races.
                    overflow();
                }
            }
            frames.push_back(std::move(frame));
        }
        // Notifications posted during readdir must participate in rename
        // correlation while old children still pin their vnode identities.
        if (drain(hints)) {
            // A rename may have happened between two directory enumerations.
            // Re-sample the whole set once so its two ends can be correlated.
            for (auto &frame : frames) {
                auto &watch = watches.at(frame.wd);
                if (!S_ISDIR(frame.root.st_mode))
                    continue;
                try {
                    check_host(::fstat(watch.root.node->descriptor.get(), &frame.root));
                    if (!frame.root.st_nlink)
                        continue;
                    check_host(::clock_gettime(CLOCK_REALTIME, &frame.sampled));
                    frame.children = census(watch, false);
                    frame.scanned = true;
                } catch (const GuestError &) {
                    frame.scanned = false;
                    overflow();
                }
            }
            // An unbounded host mutation stream cannot yield an atomic census.
            if (drain(hints))
                overflow();
        }
        std::vector<Change> removed, added;
        std::vector<Event> events, self_events;
        std::set<int32_t> deleted;
        for (auto &frame : frames) {
            auto &watch = watches.at(frame.wd);
            const uint32_t hint = hints[identity(frame.root)];
            if (hint & NOTE_REVOKE) {
                self_events.push_back({frame.wd, inotify::unmount, 0, {}});
                deleted.insert(frame.wd);
                continue;
            }
            metadata(events, frame.wd, {}, watch.root.info, frame.root, hint);
            // Linux retains a watch on an unlinked inode until the final guest
            // open description closes. NOTE_DELETE may already be consumed by
            // then, so test nlink on every census, including after final CLOSE.
            if (!frame.root.st_nlink && !guest_open(identity(frame.root))) {
                self_events.push_back({frame.wd, inotify::delete_self, 0, {}});
                deleted.insert(frame.wd);
                continue;
            }
            // APFS posts NOTE_RENAME even for a rename between hardlinks
            // to the same inode. A real move also changes the inode ctime.
            if ((hint & NOTE_RENAME) &&
                (exchanged.count(identity(frame.root)) ||
                 !same_time(watch.root.info.st_ctimespec, frame.root.st_ctimespec)))
                self_events.push_back({frame.wd, inotify::move_self, 0, {}});
            if (frame.scanned) {
                size_t structural = 0;
                for (const auto &[name, child] : watch.children) {
                    auto fresh = frame.children.find(name);
                    if (fresh == frame.children.end() ||
                        !same_node(fresh->second.info, child.info, bool(child.node))) {
                        removed.push_back({frame.wd, name, child, false});
                        ++structural;
                    } else
                        metadata(events, frame.wd, name, child.info, fresh->second.info,
                                 hints[identity(child.info)]);
                }
                for (const auto &[name, child] : frame.children) {
                    auto old = watch.children.find(name);
                    if (old == watch.children.end() ||
                        !same_node(old->second.info, child.info, bool(old->second.node))) {
                        added.push_back({frame.wd, name, child, false});
                        ++structural;
                    }
                }
                // APFS also posts NOTE_WRITE for rename(a,b) when both names
                // already refer to the same inode. That no-op leaves directory
                // mtime/ctime unchanged and loses no events. A transient
                // create/delete or rename cycle changes directory metadata even
                // when the final names match; its names cannot be recovered.
                const bool directory_changed =
                    !same_time(watch.root.info.st_mtimespec, frame.root.st_mtimespec) ||
                    !same_time(watch.root.info.st_ctimespec, frame.root.st_ctimespec);
                if ((hint & NOTE_WRITE) && !structural && directory_changed)
                    overflow();
            }
            for (auto &detached : watch.unlinked) {
                auto &child = detached.child;
                struct stat fresh{};
                if (child.node && ::fstat(child.node->descriptor.get(), &fresh) == 0) {
                    metadata(events, frame.wd, detached.name, child.info, fresh, hints[identity(child.info)]);
                    child.info = fresh;
                }
            }
        }
        // Pair by pinned (device,inode), and require a real NOTE_RENAME: unlink
        // plus link of a hardlink must not become a made-up rename cookie.
        std::map<Identity, std::deque<size_t>> destinations;
        for (size_t index = 0; index < added.size(); ++index)
            destinations[identity(added[index].child.info)].push_back(index);
        std::set<std::pair<int32_t, std::string>> replaced;
        for (auto &source : removed) {
            if (!(hints[identity(source.child.info)] & NOTE_RENAME))
                continue;
            uint32_t move_cookie = cookie();
            auto &candidates = destinations[identity(source.child.info)];
            events.push_back({source.wd, inotify::moved_from | type_flag(source.child.info), move_cookie,
                              source.name});
            source.matched = true;
            if (!candidates.empty()) {
                auto &destination = added[candidates.front()];
                candidates.pop_front();
                events.push_back({destination.wd, inotify::moved_to | type_flag(destination.child.info),
                                  move_cookie, destination.name});
                destination.matched = true;
                replaced.emplace(destination.wd, destination.name);
            } else
                moves[identity(source.child.info)] = {move_cookie, now + std::chrono::milliseconds(100)};
        }
        for (auto &source : removed) {
            if (source.matched)
                continue;
            const bool overwritten = replaced.count({source.wd, source.name});
            // rename(old,new) replacing new produces MOVED_TO, not an extra
            // DELETE for the replaced directory entry. A direct inode watch
            // on that replaced vnode still gets DELETE_SELF and IGNORED.
            if (!overwritten)
                events.push_back({source.wd, inotify::remove | type_flag(source.child.info), 0, source.name});
            auto &watch = watches.at(source.wd);
            if (!(watch.mask & inotify::exclude_unlinked) && source.child.node) {
                if (watch.unlinked.size() < limits.snapshot_entries)
                    watch.unlinked.push_back({source.name, source.child});
                else
                    overflow();
            }
        }
        for (auto &destination : added) {
            if (destination.matched)
                continue;
            auto move = moves.find(identity(destination.child.info));
            if (move != moves.end()) {
                events.push_back({destination.wd, inotify::moved_to | type_flag(destination.child.info),
                                  move->second.cookie, destination.name});
                moves.erase(move);
            } else if ((hints[identity(destination.child.info)] & NOTE_RENAME) ||
                       (born_before(destination.child.info, watches.at(destination.wd).sampled) &&
                        (S_ISDIR(destination.child.info.st_mode) || destination.child.info.st_nlink == 1))) {
                // The inode can also be pinned by a direct file watch, even
                // when its former parent has no directory watch in this FD.
                // An older, single-link inode appearing from an unobserved
                // parent is the snapshot fallback for move-in. As with any
                // snapshot backend, a link/unlink sequence entirely outside
                // the observed set cannot be distinguished from that move.
                events.push_back({destination.wd, inotify::moved_to | type_flag(destination.child.info),
                                  cookie(), destination.name});
            } else {
                events.push_back({destination.wd, inotify::create | type_flag(destination.child.info), 0,
                                  destination.name});
            }
        }
        for (auto &frame : frames) {
            auto &watch = watches.at(frame.wd);
            watch.root.info = frame.root;
            if (frame.scanned) {
                watch.children = std::move(frame.children);
                watch.sampled = frame.sampled;
            }
        }
        for (const auto &event : events)
            emit(event.wd, event.mask, event.name, event.cookie);
        for (const auto &event : self_events)
            emit(event.wd, event.mask, event.name, event.cookie);
        for (int32_t wd : deleted)
            retire(wd);
        for (auto it = tokens.begin(); it != tokens.end();)
            if (it->second.expired())
                it = tokens.erase(it);
            else
                ++it;
        for (auto it = nodes.begin(); it != nodes.end();)
            if (it->second.expired())
                it = nodes.erase(it);
            else
                ++it;
    }
    int32_t add(const Path &path, uint32_t mask) {
        struct stat info = path.stat();
        if ((mask & inotify::only_directory) && !S_ISDIR(info.st_mode))
            guest_error(20);
        // O_EVTONLY alone does not enforce Linux's read permission requirement.
        if (!S_ISLNK(info.st_mode))
            check_host(::faccessat(path.parent.get(), path.name.c_str(), R_OK, AT_EACCESS));
        // Path::stat above validates the guest scope. Path::open forcibly adds
        // O_NOFOLLOW, which Darwin cannot combine with O_SYMLINK. For this one
        // case use the already pinned parent directly, and verify the inode.
        FD descriptor = S_ISLNK(info.st_mode)
                            ? FD(::openat(path.parent.get(), path.name.c_str(),
                                          O_EVTONLY | O_SYMLINK | O_CLOEXEC))
                            : path.open(O_EVTONLY);
        if (descriptor.get() < 0)
            guest_error(linux_errno(errno));
        struct stat pinned{};
        check_host(::fstat(descriptor.get(), &pinned));
        if (!same_node(pinned, info))
            guest_error(11);
        for (auto &[wd, watch] : watches) {
            if (identity(watch.root.info) == identity(pinned)) {
                if (mask & inotify::mask_create)
                    guest_error(17);
                uint32_t stored =
                    mask & (inotify::all_events | inotify::exclude_unlinked | inotify::one_shot);
                watch.mask = mask & inotify::mask_add ? watch.mask | stored : stored;
                if (watch.mask & inotify::exclude_unlinked)
                    watch.unlinked.clear();
                return wd;
            }
        }
        if (watches.size() >= limits.watches)
            guest_error(28);
        Watch watch{};
        watch.mask = mask & (inotify::all_events | inotify::exclude_unlinked | inotify::one_shot);
        watch.root = {pinned, monitor(std::move(descriptor), pinned)};
        check_host(::clock_gettime(CLOCK_REALTIME, &watch.sampled));
        if (S_ISDIR(pinned.st_mode))
            watch.children = census(watch, true);
        uint32_t candidate = next_wd;
        for (;;) {
            if (candidate > uint32_t(INT_MAX) || !candidate)
                candidate = 1;
            if (!watches.count(int32_t(candidate)))
                break;
            ++candidate;
            if (candidate == next_wd)
                guest_error(28);
        }
        watches.emplace(int32_t(candidate), std::move(watch));
        next_wd = candidate == uint32_t(INT_MAX) ? 1 : candidate + 1;
        return int32_t(candidate);
    }
#else
    void collect() {
        alignas(struct inotify_event) std::array<uint8_t, 65536> bytes{};
        for (unsigned batch = 0; batch < 64; ++batch) {
            ssize_t count = ::read(backend.get(), bytes.data(), bytes.size());
            if (count < 0 && errno == EINTR)
                continue;
            if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK))
                break;
            if (count < 0) {
                backend_failure(linux_errno(errno));
                break;
            }
            if (!count)
                break;
            for (size_t cursor = 0; cursor + sizeof(struct inotify_event) <= size_t(count);) {
                struct inotify_event event{};
                std::memcpy(&event, bytes.data() + cursor, sizeof(event));
                if (event.len > size_t(count) - cursor - sizeof(event)) {
                    backend_failure(5);
                    break;
                }
                const char *name = reinterpret_cast<const char *>(bytes.data() + cursor + sizeof(event));
                std::string filename(name, ::strnlen(name, event.len));
                if (event.mask & IN_Q_OVERFLOW)
                    overflow();
                else
                    enqueue({event.wd, event.mask, event.cookie, std::move(filename)});
                if (event.mask & IN_IGNORED)
                    watches.erase(event.wd);
                cursor += sizeof(event) + event.len;
            }
        }
    }
    int32_t add(const Path &path, uint32_t mask) {
        struct stat info = path.stat();
        if ((mask & inotify::only_directory) && !S_ISDIR(info.st_mode))
            guest_error(20);
        const auto key = identity(info);
        bool exists = std::any_of(watches.begin(), watches.end(), [&](const auto &watch) {
            return watch.second == key;
        });
        if (!exists && watches.size() >= limits.watches)
            guest_error(28);
        // The parent FD is namespace-resolved and pinned, including bind mounts.
        std::string native = "/proc/self/fd/" + std::to_string(path.parent.get()) + "/" + path.name;
        int wd = ::inotify_add_watch(backend.get(), native.c_str(), mask);
        check_host(wd);
        watches.insert_or_assign(wd, key);
        return wd;
    }
#endif
    void pump(const std::set<Identity> &exchanged = {}) {
        if (failure)
            return;
        try {
#ifdef __APPLE__
            collect(exchanged);
#else
            (void)exchanged;
            collect();
#endif
        } catch (const GuestError &error) {
            backend_failure(error.number);
        }
    }
};

void InotifyFile::validate_flags(uint32_t flags) {
    if (flags & ~(inotify::nonblock | inotify::cloexec))
        guest_error(22);
}
void InotifyFile::validate_mask(uint32_t mask) {
    if ((mask & ~inotify::known_mask) || !(mask & inotify::all_events) ||
        ((mask & inotify::mask_add) && (mask & inotify::mask_create)))
        guest_error(22);
}
InotifyFile::InotifyFile(uint32_t flags) : InotifyFile(flags, InotifyLimits{}) {}
InotifyFile::InotifyFile(uint32_t flags, InotifyLimits limits) {
    validate_flags(flags);
    impl_ = std::make_unique<Impl>(limits);
    auto &registry = instances();
    std::lock_guard<std::mutex> lock(registry.mutex);
    registry.files.insert(this);
}
InotifyFile::~InotifyFile() {
    auto &registry = instances();
    std::lock_guard<std::mutex> lock(registry.mutex);
    registry.files.erase(this);
}
int32_t InotifyFile::add_watch(const Filesystem &filesystem, const std::string &pathname, uint32_t mask,
                                const std::string &base) {
    validate_mask(mask);
    return add_watch(filesystem.resolve(pathname, base, !(mask & inotify::dont_follow)), mask);
}
int32_t InotifyFile::add_watch(const Path &path, uint32_t mask) {
    validate_mask(mask);
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->pump();
    if (impl_->failure)
        guest_error(impl_->failure);
    if (path.virtual_path) {
        auto info = path.stat(); // Preserve ENOENT for a missing virtual node.
        if ((mask & inotify::only_directory) && !S_ISDIR(info.st_mode))
            guest_error(20);
        guest_error(95);
    }
    return impl_->add(path, mask);
}
void InotifyFile::remove_watch(int32_t wd) {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->pump();
    if (!impl_->watches.count(wd))
        guest_error(22);
#ifdef __APPLE__
    impl_->retire(wd);
#else
    if (::inotify_rm_watch(impl_->backend.get(), wd) < 0) {
        if (errno == EINVAL)
            impl_->watches.erase(wd);
        guest_error(linux_errno(errno));
    }
    impl_->watches.erase(wd);
    impl_->pump();
#endif
}
FD InotifyFile::duplicate_handle() const {
    return duplicate(impl_->reader.get());
}
int InotifyFile::native_handle() const noexcept {
    return impl_->reader.get();
}
std::optional<size_t> InotifyFile::try_read(void *buffer, size_t capacity) {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->pump();
    return impl_->consume(buffer, capacity);
}
size_t InotifyFile::read(void *buffer, size_t capacity) {
    auto count = try_read(buffer, capacity);
    if (!count)
        guest_error(11);
    return *count;
}
short InotifyFile::poll_events(short requested) {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->pump();
    return (impl_->queue.empty() ? 0 : requested & (POLLIN | POLLRDNORM)) |
           (impl_->failure ? POLLERR : 0);
}
uint32_t InotifyFile::available() {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->pump();
    return uint32_t(std::min<size_t>(impl_->queued_bytes, INT_MAX));
}
size_t InotifyFile::watch_count() const {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    return impl_->watches.size();
}
void InotifyFile::pump() {
    std::lock_guard<std::mutex> lock(impl_->mutex);
    impl_->pump();
}
void InotifyFile::service_all() {
    auto &registry = instances();
    std::lock_guard<std::mutex> lock(registry.mutex);
    for (auto *file : registry.files)
        file->pump();
}

struct InotifyOpenFile::Impl {
#ifdef __APPLE__
    FD descriptor;
    std::shared_ptr<FD> parent;
    Identity key{}, parent_key{};
    std::string name;
    bool directory = false, writable = false, linked = true;
    bool registered = false, counted = false, announced = false;
#endif
};
InotifyOpenFile::InotifyOpenFile(std::unique_ptr<Impl> impl) : impl_(std::move(impl)) {}
void InotifyOpenFile::overflow_all() noexcept {
#ifdef __APPLE__
    try {
        auto &registry = instances();
        std::lock_guard<std::mutex> lock(registry.mutex);
        for (auto *file : registry.files) {
            try {
                std::lock_guard<std::mutex> instance_lock(file->impl_->mutex);
                file->impl_->overflow();
            } catch (...) {
                // Allocation exhaustion must not turn a completed guest I/O
                // into an unrelated errno. Existing queued events stay intact.
            }
        }
    } catch (...) {
    }
#endif
}
std::shared_ptr<InotifyOpenFile> InotifyOpenFile::opened(const Path &path, int descriptor,
                                                       bool writable, bool anonymous) noexcept {
#ifdef __APPLE__
    if (path.virtual_path)
        return {};
    try {
        struct stat info{};
        check_host(::fstat(descriptor, &info));
        auto impl = std::make_unique<Impl>();
        impl->descriptor = duplicate(descriptor);
        impl->key = identity(info);
        impl->directory = S_ISDIR(info.st_mode);
        impl->writable = writable;
        impl->linked = !anonymous;
        if (!anonymous && !path.mount_root && !path.name.empty() && path.name != "." &&
            path.name != "..") {
            check_host(::fstat(path.parent.get(), &info));
            impl->parent = std::make_shared<FD>(duplicate(path.parent.get()));
            impl->parent_key = identity(info);
            impl->name = path.name;
        }
        auto result = std::shared_ptr<InotifyOpenFile>(new InotifyOpenFile(std::move(impl)));
        {
            auto &registry = instances();
            std::lock_guard<std::mutex> lock(registry.mutex);
            registry.opens.insert(result.get());
            result->impl_->registered = true;
            auto &counts = guest_opens();
            std::lock_guard<std::mutex> count_lock(counts.mutex);
            ++counts.counts[result->impl_->key];
            result->impl_->counted = true;
            result->impl_->announced = true;
        }
        result->emit(inotify::open);
        return result;
    } catch (...) {
        overflow_all();
    }
#else
    (void)path;
    (void)descriptor;
    (void)writable;
    (void)anonymous;
#endif
    return {};
}
void InotifyOpenFile::emit(uint32_t event) noexcept {
#ifdef __APPLE__
    try {
        auto &registry = instances();
        std::lock_guard<std::mutex> lock(registry.mutex);
        for (auto *file : registry.files) {
            auto &backend = *file->impl_;
            std::lock_guard<std::mutex> instance_lock(backend.mutex);
            // CREATE / MODIFY / MOVE must precede a subsequent data event.
            backend.next_census = {};
            backend.pump();
            if (backend.failure)
                continue;
            std::vector<std::pair<int32_t, std::string>> targets;
            for (const auto &[wd, watch] : backend.watches) {
                if (watch.root.node->key == impl_->key)
                    targets.emplace_back(wd, std::string{});
                if (impl_->parent && watch.root.node->key == impl_->parent_key &&
                    (impl_->linked || !(watch.mask & inotify::exclude_unlinked)))
                    targets.emplace_back(wd, impl_->name);
            }
            for (const auto &[wd, name] : targets)
                backend.emit(wd, event | (impl_->directory ? inotify::is_directory : 0), name);
        }
    } catch (...) {
        overflow_all();
    }
#else
    (void)event;
#endif
}
void InotifyOpenFile::accessed() noexcept {
    emit(inotify::access);
}
InotifyOpenFile::~InotifyOpenFile() {
#ifdef __APPLE__
    if (!impl_->registered)
        return;
    if (impl_->announced)
        emit(impl_->writable ? inotify::close_write : inotify::close_nowrite);
    try {
        auto &registry = instances();
        std::lock_guard<std::mutex> lock(registry.mutex);
        registry.opens.erase(this);
        if (impl_->counted) {
            auto &counts = guest_opens();
            std::lock_guard<std::mutex> count_lock(counts.mutex);
            auto found = counts.counts.find(impl_->key);
            if (found != counts.counts.end() && !--found->second)
                counts.counts.erase(found);
        }
        // Release a deferred DELETE_SELF only after CLOSE has been queued.
        for (auto *file : registry.files) {
            auto &backend = *file->impl_;
            std::lock_guard<std::mutex> instance_lock(backend.mutex);
            backend.next_census = {};
            backend.pump();
        }
    } catch (...) {
        overflow_all();
    }
#endif
}
void InotifyOpenFile::unlinked(const Path &path) noexcept {
#ifdef __APPLE__
    if (path.virtual_path)
        return;
    try {
        struct stat parent{};
        check_host(::fstat(path.parent.get(), &parent));
        auto &registry = instances();
        std::lock_guard<std::mutex> lock(registry.mutex);
        for (auto *file : registry.opens) {
            auto &open = *file->impl_;
            if (open.parent && open.linked && open.parent_key == identity(parent) && open.name == path.name)
                open.linked = false;
        }
    } catch (...) {
        overflow_all();
    }
#else
    (void)path;
#endif
}
void InotifyOpenFile::renamed(const Path &source, const Path &target) noexcept {
#ifdef __APPLE__
    if (source.virtual_path || target.virtual_path)
        return;
    try {
        struct stat from{}, to{}, moved{}, remaining{};
        check_host(::fstat(source.parent.get(), &from));
        check_host(::fstat(target.parent.get(), &to));
        check_host(::fstatat(target.parent.get(), target.name.c_str(), &moved, AT_SYMLINK_NOFOLLOW));
        // rename(a,b) is a no-op if a and b are hardlinks to the same inode.
        if (::fstatat(source.parent.get(), source.name.c_str(), &remaining, AT_SYMLINK_NOFOLLOW) == 0 &&
            identity(remaining) == identity(moved))
            return;
        auto parent = std::make_shared<FD>(duplicate(target.parent.get()));
        auto &registry = instances();
        std::lock_guard<std::mutex> lock(registry.mutex);
        for (auto *file : registry.opens) {
            auto &open = *file->impl_;
            if (!open.parent || !open.linked)
                continue;
            if (open.parent_key == identity(from) && open.name == source.name && open.key == identity(moved)) {
                open.name = target.name;
                open.parent = parent;
                open.parent_key = identity(to);
            } else if (open.parent_key == identity(to) && open.name == target.name) {
                // The overwritten dentry may still have open descriptions.
                open.linked = false;
            }
        }
    } catch (...) {
        overflow_all();
    }
#else
    (void)source;
    (void)target;
#endif
}
void InotifyOpenFile::exchanged(const Path &source, const Path &target) noexcept {
#ifdef __APPLE__
    if (source.virtual_path || target.virtual_path)
        return;
    try {
        struct stat from{}, to{}, at_source{}, at_target{};
        check_host(::fstat(source.parent.get(), &from));
        check_host(::fstat(target.parent.get(), &to));
        check_host(::fstatat(source.parent.get(), source.name.c_str(), &at_source, AT_SYMLINK_NOFOLLOW));
        check_host(::fstatat(target.parent.get(), target.name.c_str(), &at_target, AT_SYMLINK_NOFOLLOW));
        if (identity(at_source) == identity(at_target))
            return; // Identical names or hardlinks to the same inode do not move.
        auto source_parent = std::make_shared<FD>(duplicate(source.parent.get()));
        auto target_parent = std::make_shared<FD>(duplicate(target.parent.get()));
        auto &registry = instances();
        std::lock_guard<std::mutex> lock(registry.mutex);
        // Inspect the post-exchange inodes, and move each existing description
        // once. Calling renamed twice would instead unlink the opposite side.
        for (auto *file : registry.opens) {
            auto &open = *file->impl_;
            if (!open.parent || !open.linked)
                continue;
            if (open.parent_key == identity(from) && open.name == source.name &&
                open.key == identity(at_target)) {
                open.name = target.name;
                open.parent = target_parent;
                open.parent_key = identity(to);
            } else if (open.parent_key == identity(to) && open.name == target.name &&
                       open.key == identity(at_source)) {
                open.name = source.name;
                open.parent = source_parent;
                open.parent_key = identity(from);
            }
        }
        const std::set<Identity> exchanged{identity(at_source), identity(at_target)};
        for (auto *file : registry.files) {
            auto &backend = *file->impl_;
            std::lock_guard<std::mutex> instance_lock(backend.mutex);
            // Reconcile both directions now, before a later census can consume
            // the swapped names with only Darwin's one-sided native hint.
            backend.pump(exchanged);
        }
    } catch (...) {
        overflow_all();
    }
#else
    (void)source;
    (void)target;
#endif
}
} // namespace operit::arm64
