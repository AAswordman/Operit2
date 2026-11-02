#include "linux/files/record_lock.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <climits>
#include <fcntl.h>
#include <map>
#include <mutex>
#include <set>
#include <sys/stat.h>
#include <tuple>
#include <vector>

namespace operit::arm64::files {
namespace {
constexpr uint64_t forever = uint64_t{1} << 63;
struct LinuxFlock {
    int16_t type, whence;
    uint32_t padding;
    int64_t start, length;
    int32_t pid;
    uint32_t padding2;
};
static_assert(sizeof(LinuxFlock) == 32);
struct FileKey {
    uint64_t device, inode;
    bool operator<(const FileKey &other) const {
        return std::tie(device, inode) < std::tie(other.device, other.inode);
    }
};
struct ProcessKey {
    uintptr_t scheduler;
    uint32_t pid;
    bool operator<(const ProcessKey &other) const {
        return std::tie(scheduler, pid) < std::tie(other.scheduler, other.pid);
    }
    bool operator==(const ProcessKey &other) const {
        return scheduler == other.scheduler && pid == other.pid;
    }
};
struct Owner {
    ProcessKey process;
    bool ofd;
    const OpenDescription *identity;
    std::weak_ptr<OpenDescription> description;
    bool operator==(const Owner &other) const {
        return ofd == other.ofd && (ofd ? identity == other.identity : process == other.process);
    }
};
struct Segment {
    uint64_t start, end;
    int type; // -1 unlocked, 0 read, 1 write.
};
struct Record : Segment {
    Owner owner;
};
struct FileLocks {
    std::shared_ptr<FD> anchor;
    int mode = 0;
    std::vector<Record> records;
    std::vector<Segment> native;
};
struct Registry {
    std::mutex mutex;
    std::map<FileKey, FileLocks> files;
    std::map<std::pair<uintptr_t, uint32_t>, std::pair<ProcessKey, std::vector<ProcessKey>>> waiters;
    int error = 0;
};
Registry &registry() {
    static Registry instance;
    return instance;
}
ProcessKey process_key(Scheduler &scheduler, uint32_t pid) {
    return {reinterpret_cast<uintptr_t>(&scheduler), pid};
}
int native_type(int type) {
    return type < 0 ? F_UNLCK : (type ? F_WRLCK : F_RDLCK);
}
int64_t checked_sum(int64_t base, int64_t offset) {
    if ((offset > 0 && base > INT64_MAX - offset) || (offset < 0 && base < INT64_MIN - offset))
        guest_error(75);
    return base + offset;
}
bool overlaps(const Segment &a, const Segment &b) {
    return a.start < b.end && b.start < a.end;
}
std::vector<Segment> aggregate(const std::vector<Record> &records) {
    std::map<uint64_t, std::pair<int, int>> edges;
    for (const auto &record : records) {
        auto &begin = edges[record.start];
        auto &end = edges[record.end];
        (record.type ? begin.second : begin.first)++;
        (record.type ? end.second : end.first)--;
    }
    int readers = 0, writers = 0;
    uint64_t position = 0;
    std::vector<Segment> result;
    for (const auto &[next, delta] : edges) {
        if (position < next && (readers || writers)) {
            const int type = writers ? 1 : 0;
            if (!result.empty() && result.back().end == position && result.back().type == type)
                result.back().end = next;
            else
                result.push_back({position, next, type});
        }
        readers += delta.first;
        writers += delta.second;
        position = next;
    }
    return result;
}
int set_native(FileLocks &file, const Segment &segment) {
    struct flock lock{};
    lock.l_type = native_type(segment.type);
    lock.l_whence = SEEK_SET;
    lock.l_start = segment.start;
    lock.l_len = segment.end == forever ? 0 : segment.end - segment.start;
    if (::fcntl(file.anchor->get(), F_OFD_SETLK, &lock) == 0)
        return 0;
    return errno == EACCES || errno == EAGAIN ? 11 : linux_errno(errno);
}
int commit(FileLocks &file, std::vector<Record> records) {
    auto desired = aggregate(records);
    for (const auto &part : desired)
        if ((part.type == 0 && file.mode == O_WRONLY) || (part.type == 1 && file.mode == O_RDONLY))
            return 95;
    struct Delta {
        uint64_t start, end;
        int before, after;
    };
    std::set<uint64_t> edges;
    for (const auto &part : file.native) {
        edges.insert(part.start);
        edges.insert(part.end);
    }
    for (const auto &part : desired) {
        edges.insert(part.start);
        edges.insert(part.end);
    }
    std::vector<Delta> changes;
    size_t old_index = 0, new_index = 0;
    for (auto edge = edges.begin(); edge != edges.end();) {
        const uint64_t start = *edge++;
        if (edge == edges.end())
            break;
        const uint64_t end = *edge;
        while (old_index < file.native.size() && file.native[old_index].end <= start)
            ++old_index;
        while (new_index < desired.size() && desired[new_index].end <= start)
            ++new_index;
        const int before = old_index < file.native.size() && file.native[old_index].start <= start
                               ? file.native[old_index].type
                               : -1;
        const int after =
            new_index < desired.size() && desired[new_index].start <= start ? desired[new_index].type : -1;
        if (before != after)
            changes.push_back({start, end, before, after});
    }
    std::vector<Delta> acquired;
    // Strengthen locks first. A failed extension/conversion leaves every old
    // lock held; rollback only downgrades or releases newly acquired ranges.
    for (const auto &change : changes) {
        if (change.after <= change.before)
            continue;
        const int error = set_native(file, {change.start, change.end, change.after});
        if (error) {
            for (auto it = acquired.rbegin(); it != acquired.rend(); ++it)
                if (set_native(file, {it->start, it->end, it->before}))
                    throw Failure("cannot restore native OFD lock after a failed range conversion");
            return error;
        }
        acquired.push_back(change);
    }
    for (const auto &change : changes)
        if (change.after < change.before && set_native(file, {change.start, change.end, change.after}))
            throw Failure("cannot release native OFD lock range");
    file.records = std::move(records);
    file.native = std::move(desired);
    return 0;
}
std::vector<Record> replace(const std::vector<Record> &records, const Segment &range, const Owner &owner) {
    std::vector<Record> result;
    for (const auto &record : records) {
        if (!(record.owner == owner) || !overlaps(record, range)) {
            result.push_back(record);
            continue;
        }
        if (record.start < range.start)
            result.push_back({{record.start, range.start, record.type}, record.owner});
        if (record.end > range.end)
            result.push_back({{range.end, record.end, record.type}, record.owner});
    }
    if (range.type >= 0)
        result.push_back({range, owner});
    // Canonicalize each owner's intervals without merging across owners or
    // lock types; the native aggregate is calculated independently.
    std::sort(result.begin(), result.end(), [](const Record &a, const Record &b) {
        const auto key = [](const Record &record) {
            return std::make_tuple(record.owner.ofd,
                                   record.owner.ofd ? reinterpret_cast<uintptr_t>(record.owner.identity)
                                                    : record.owner.process.scheduler,
                                   record.owner.ofd ? 0u : record.owner.process.pid, record.start);
        };
        return key(a) < key(b);
    });
    std::vector<Record> merged;
    for (const auto &record : result) {
        if (!merged.empty() && merged.back().owner == record.owner && merged.back().type == record.type &&
            merged.back().end >= record.start)
            merged.back().end = std::max(merged.back().end, record.end);
        else
            merged.push_back(record);
    }
    return merged;
}
void prune(FileLocks &file) {
    auto records = file.records;
    records.erase(std::remove_if(records.begin(), records.end(),
                                 [](const Record &record) {
                                     return record.owner.ofd && record.owner.description.expired();
                                 }),
                  records.end());
    if (records.size() != file.records.size()) {
        const int error = commit(file, std::move(records));
        if (error)
            guest_error(error);
    }
}
std::shared_ptr<FD> anchor(Filesystem &filesystem, const std::string &path, int source,
                           const FileKey &identity) {
    // Prefer an independent read/write OFD so future read/write conversions do
    // not depend on the access mode of the first locking guest descriptor.
    if (!path.empty() && path.front() == '/') {
        try {
            auto resolved = filesystem.resolve(path);
            if (!resolved.virtual_path) {
                FD opened(::openat(resolved.parent.get(), resolved.name.c_str(),
                                   O_RDWR | O_NONBLOCK | O_NOFOLLOW | O_CLOEXEC));
                if (opened.get() >= 0) {
                    struct stat info{};
                    check_host(::fstat(opened.get(), &info));
                    if (uint64_t(info.st_dev) == identity.device && uint64_t(info.st_ino) == identity.inode)
                        return std::make_shared<FD>(std::move(opened));
                } else if (errno != EACCES && errno != EPERM && errno != EROFS && errno != EISDIR &&
                           errno != ENOENT && errno != ENOTDIR && errno != ELOOP)
                    guest_error(linux_errno(errno));
            }
        } catch (const GuestError &error) {
            if (error.number != 2 && error.number != 13 && error.number != 20 && error.number != 40)
                throw;
        }
    }
    // An unlinked inode remains lockable through its actual open description.
    auto held = std::make_shared<FD>(::fcntl(source, F_DUPFD_CLOEXEC, 0));
    check_host(held->get());
    return held;
}
bool cycle(const Registry &state, const ProcessKey &start, const ProcessKey &target,
           std::set<ProcessKey> &visited) {
    if (start == target)
        return true;
    if (!visited.insert(start).second)
        return false;
    for (const auto &[key, waiter] : state.waiters) {
        (void)key;
        if (waiter.first == start)
            for (const auto &owner : waiter.second)
                if (cycle(state, owner, target, visited))
                    return true;
    }
    return false;
}
struct Request {
    Memory &memory;
    Filesystem &filesystem;
    Owner owner;
    FileKey identity;
    std::string path;
    std::shared_ptr<FD> source;
    std::shared_ptr<OpenDescription> description;
    Segment range;
    LinuxFlock value;
    uint64_t argument;
    uint32_t tid;
    bool query, waiting;
    std::optional<int64_t> attempt() {
        auto &state = registry();
        std::lock_guard<std::mutex> guard(state.mutex);
        if (state.error)
            return -state.error;
        auto inserted = state.files.try_emplace(identity);
        auto &file = inserted.first->second;
        if (!file.anchor) {
            try {
                file.anchor = anchor(filesystem, path, source->get(), identity);
                const int flags = ::fcntl(file.anchor->get(), F_GETFL);
                check_host(flags);
                file.mode = flags & O_ACCMODE;
                struct flock supported{};
                supported.l_type = F_RDLCK;
                supported.l_whence = SEEK_SET;
                check_host(::fcntl(file.anchor->get(), F_OFD_GETLK, &supported));
            } catch (...) {
                state.files.erase(inserted.first);
                throw;
            }
        }
        prune(file);
        const Record *conflict = nullptr;
        std::vector<ProcessKey> dependencies;
        for (const auto &record : file.records) {
            if (!(record.owner == owner) && overlaps(record, range) && (record.type || range.type == 1)) {
                if (!conflict || record.start < conflict->start)
                    conflict = &record;
                if (!record.owner.ofd)
                    dependencies.push_back(record.owner.process);
            }
        }
        const auto waiter_key = std::make_pair(owner.process.scheduler, tid);
        auto blocked = [&]() -> std::optional<int64_t> {
            if (!waiting)
                return -11;
            if (!owner.ofd) {
                for (const auto &dependency : dependencies) {
                    std::set<ProcessKey> visited;
                    if (cycle(state, dependency, owner.process, visited)) {
                        state.waiters.erase(waiter_key);
                        return -35;
                    }
                }
                state.waiters[waiter_key] = {owner.process, dependencies};
            }
            return {};
        };
        if (query) {
            if (!memory.accessible(argument, sizeof(LinuxFlock), write_perm))
                return -14;
            auto result = value;
            result.type = 2;
            if (conflict) {
                result.type = conflict->type;
                result.whence = SEEK_SET;
                result.start = conflict->start;
                result.length = conflict->end == forever ? 0 : conflict->end - conflict->start;
                result.pid = conflict->owner.ofd
                                 ? -1
                                 : (conflict->owner.process.scheduler == owner.process.scheduler
                                        ? int32_t(conflict->owner.process.pid)
                                        : 0);
            } else {
                struct flock native{};
                native.l_type = native_type(range.type);
                native.l_whence = SEEK_SET;
                native.l_start = range.start;
                native.l_len = range.end == forever ? 0 : range.end - range.start;
                if (::fcntl(file.anchor->get(), F_OFD_GETLK, &native) < 0)
                    return -linux_errno(errno);
                if (native.l_type != F_UNLCK) {
                    if (native.l_type != F_RDLCK && native.l_type != F_WRLCK)
                        return -5;
                    result.type = native.l_type == F_WRLCK ? 1 : 0;
                    result.whence = native.l_whence;
                    result.start = native.l_start;
                    result.length = native.l_len;
                    // Native processes outside this Linux PID namespace have
                    // no guest PID. OFD locks retain Linux's owner value -1.
                    result.pid = native.l_pid == -1 ? -1 : 0;
                }
            }
            memory.copy_in(argument, &result, sizeof(result));
            if (file.records.empty())
                state.files.erase(inserted.first);
            return 0;
        }
        if (range.type >= 0 && conflict)
            return blocked();
        const int error = commit(file, replace(file.records, range, owner));
        if (error == 11) {
            auto result = blocked();
            if (!waiting && file.records.empty())
                state.files.erase(inserted.first);
            return result;
        }
        state.waiters.erase(waiter_key);
        if (file.records.empty())
            state.files.erase(inserted.first);
        return -error;
    }
};
template <class Predicate> void release_matching(Registry &state, Predicate matches) {
    for (auto it = state.files.begin(); it != state.files.end();) {
        auto records = it->second.records;
        records.erase(std::remove_if(records.begin(), records.end(), matches), records.end());
        if (records.size() != it->second.records.size()) {
            const int error = commit(it->second, std::move(records));
            if (error)
                guest_error(error);
        }
        if (it->second.records.empty())
            it = state.files.erase(it);
        else
            ++it;
    }
}
} // namespace
int64_t record_lock(Memory &memory, Filesystem &filesystem, Scheduler &scheduler, Descriptor &entry,
                    uint64_t command, uint64_t argument) {
    const bool ofd = command >= 36 && command <= 38;
    const bool query = command == 5 || command == 36;
    const bool waiting = command == 7 || command == 38;
    if (!ofd && command != 5 && command != 6 && command != 7)
        guest_error(22);
    if (!memory.accessible(argument, sizeof(LinuxFlock), read_perm | (query ? write_perm : 0)))
        guest_error(14);
    LinuxFlock value{};
    memory.copy_out(argument, &value, sizeof(value));
    if (value.type < 0 || value.type > 2 || (query && value.type == 2) || (ofd && value.pid))
        guest_error(22);
    if (entry.description->virtual_file || entry.description->counter || entry.description->poll ||
        entry.description->timer)
        guest_error(9);
    if (!query && value.type != 2) {
        const int mode = entry.description->flags & 3;
        if ((value.type == 0 && mode == 1) || (value.type == 1 && mode == 0))
            guest_error(9);
    }
    struct stat info{};
    check_host(::fstat(entry.host.get(), &info));
    int64_t base = 0;
    if (value.whence == SEEK_CUR) {
        base = ::lseek(entry.host.get(), 0, SEEK_CUR);
        if (base < 0)
            guest_error(linux_errno(errno));
    } else if (value.whence == SEEK_END)
        base = info.st_size;
    else if (value.whence != SEEK_SET)
        guest_error(22);
    int64_t start = checked_sum(base, value.start);
    int64_t length = value.length;
    if (length < 0) {
        if (length == INT64_MIN)
            guest_error(75);
        start = checked_sum(start, length);
        length = -length;
    }
    if (start < 0)
        guest_error(22);
    if (length && start > INT64_MAX - (length - 1))
        guest_error(75);
    const auto &task = scheduler.current();
    Owner owner{process_key(scheduler, task.pid), ofd, ofd ? entry.description.get() : nullptr,
                ofd ? entry.description : std::weak_ptr<OpenDescription>{}};
    auto held = std::make_shared<FD>(::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    check_host(held->get());
    auto request = std::make_shared<Request>(
        Request{memory,
                filesystem,
                owner,
                {uint64_t(info.st_dev), uint64_t(info.st_ino)},
                entry.path,
                held,
                entry.description,
                {uint64_t(start), length ? uint64_t(start) + uint64_t(length) : forever,
                 value.type == 2 ? -1 : value.type},
                value,
                argument,
                task.tid,
                query,
                waiting});
    if (auto result = request->attempt())
        return *result;
    const auto waiter_key = std::make_pair(owner.process.scheduler, task.tid);
    auto previous_cleanup = std::move(scheduler.current().wait_cleanup);
    scheduler.current().wait_cleanup = [waiter_key, identity = request->identity, previous_cleanup]() {
        auto &state = registry();
        {
            std::lock_guard<std::mutex> guard(state.mutex);
            state.waiters.erase(waiter_key);
            auto file = state.files.find(identity);
            if (file != state.files.end() && file->second.records.empty())
                state.files.erase(file);
        }
        if (previous_cleanup)
            previous_cleanup();
    };
    scheduler.block({}, -4, [request]() { return request->attempt(); });
    return 0;
}
void record_lock_close(Scheduler &scheduler, uint32_t pid, const Descriptor &entry) {
    if (entry.host.get() < 0)
        return;
    struct stat info{};
    check_host(::fstat(entry.host.get(), &info));
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    auto found = state.files.find({uint64_t(info.st_dev), uint64_t(info.st_ino)});
    if (found == state.files.end())
        return;
    const ProcessKey owner = process_key(scheduler, pid);
    auto records = found->second.records;
    records.erase(std::remove_if(records.begin(), records.end(),
                                 [&](const Record &record) {
                                     return !record.owner.ofd && record.owner.process == owner;
                                 }),
                  records.end());
    const int error = commit(found->second, std::move(records));
    if (error)
        guest_error(error);
    if (found->second.records.empty())
        state.files.erase(found);
}
void record_lock_process_exit(Scheduler &scheduler, uint32_t pid) {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    const ProcessKey owner = process_key(scheduler, pid);
    release_matching(
        state, [&](const Record &record) { return !record.owner.ofd && record.owner.process == owner; });
    for (auto it = state.waiters.begin(); it != state.waiters.end();)
        if (it->second.first == owner)
            it = state.waiters.erase(it);
        else
            ++it;
}
void record_lock_vm_exit(Scheduler &scheduler) {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    const uintptr_t identity = reinterpret_cast<uintptr_t>(&scheduler);
    release_matching(state, [&](const Record &record) { return record.owner.process.scheduler == identity; });
    for (auto it = state.waiters.begin(); it != state.waiters.end();)
        if (it->first.first == identity)
            it = state.waiters.erase(it);
        else
            ++it;
}
void record_lock_description_closed(const OpenDescription *description) noexcept {
    auto &state = registry();
    std::lock_guard<std::mutex> guard(state.mutex);
    try {
        release_matching(state, [&](const Record &record) {
            return record.owner.ofd && record.owner.identity == description;
        });
    } catch (const GuestError &error) {
        state.error = error.number;
    } catch (...) {
        state.error = 5;
    }
}
} // namespace operit::arm64::files
