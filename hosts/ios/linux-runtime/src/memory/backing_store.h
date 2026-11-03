#pragma once
#include "base/fd.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/files/memory_file.h"
#include "memory/fault.h"
#include "memory/page_mutex.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <map>
#include <memory>
#include <mutex>
#include <sys/mman.h>
#include <sys/stat.h>
#include <tuple>
#include <unistd.h>
#include <unordered_map>
#include <vector>

namespace operit::arm64::memory_detail {
inline uint64_t host_page_size() {
    static const uint64_t size = uint64_t(::sysconf(_SC_PAGESIZE));
    return size;
}

struct FileBacking;

// File syscalls execute outside an interpreter quantum. A quantum observes one
// metadata version per backing; direct Memory callers retain fresh size checks.
class FileMetadataCache {
    struct Entry {
        const FileBacking *file = nullptr;
        uint64_t generation = 0, bytes = 0;
    };
    struct State {
        std::array<Entry, 64> entries{};
        uint64_t active = 0;
    };
    inline static std::atomic<uint64_t> next_generation_{1};
    static State &current() {
        static thread_local State state;
        return state;
    }

  public:
    uint64_t generation() const {
        return current().active;
    }
    uint64_t enter() {
        auto &state = current();
        const uint64_t previous = state.active;
        state.active = next_generation_.fetch_add(1, std::memory_order_relaxed);
        return previous;
    }
    void leave(uint64_t previous) {
        current().active = previous;
    }
    uint64_t size(const FileBacking &file);
};

struct FileBacking {
    FD descriptor;
    uint64_t device, inode;
    bool writable;
    // A mapped file retains its guest data-open until the final mapping/page
    // reference disappears, just as the host descriptor retains the real file.
    std::shared_ptr<void> open_reference;

    FileBacking(int fd, bool permit_write, std::shared_ptr<void> reference = nullptr)
        : descriptor(::fcntl(fd, F_DUPFD_CLOEXEC, 0)), open_reference(std::move(reference)) {
        if (descriptor.get() < 0)
            guest_error(linux_errno(errno));
        struct stat info{};
        if (::fstat(descriptor.get(), &info) < 0)
            guest_error(linux_errno(errno));
        if (!S_ISREG(info.st_mode))
            guest_error(19);
        const int flags = ::fcntl(descriptor.get(), F_GETFL);
        if (flags < 0)
            guest_error(linux_errno(errno));
        if ((flags & O_ACCMODE) == O_WRONLY)
            guest_error(13);
        device = uint64_t(info.st_dev);
        inode = uint64_t(info.st_ino);
        writable = permit_write && (flags & O_ACCMODE) == O_RDWR;
    }
    uint64_t size() const {
        struct stat info{};
        if (::fstat(descriptor.get(), &info) < 0)
            guest_error(linux_errno(errno));
        return info.st_size < 0 ? 0 : uint64_t(info.st_size);
    }
    void check_page(uint64_t offset, uint64_t guest_address, unsigned permissions,
                    FileMetadataCache *metadata = nullptr) const {
        const uint64_t bytes = metadata ? metadata->size(*this) : size();
        if (offset >= ((bytes + page_size - 1) & ~(page_size - 1)))
            throw MemoryFault(guest_address, permissions, MemoryFault::Kind::Bus);
    }
    template <class Operation> void file_range(uint64_t offset, uint64_t length, Operation operation) const {
        const uint64_t bytes = size();
        if (offset >= bytes)
            return;
        length = std::min(length, bytes - offset);
        const uint64_t alignment = host_page_size();
        while (length) {
            const uint64_t aligned = offset / alignment * alignment;
            const uint64_t prefix = offset - aligned;
            const uint64_t count = std::min<uint64_t>(length, 16 << 20);
            const size_t mapped_length = size_t(prefix + count);
            void *mapping =
                ::mmap(nullptr, mapped_length, PROT_READ, MAP_SHARED, descriptor.get(), off_t(aligned));
            if (mapping == MAP_FAILED)
                guest_error(linux_errno(errno));
            const int result = operation(mapping, mapped_length);
            const int error = errno;
            ::munmap(mapping, mapped_length);
            if (result < 0)
                guest_error(linux_errno(error));
            offset += count;
            length -= count;
        }
    }
    void sync(uint64_t offset, uint64_t length, int flags) const {
        file_range(offset, length,
                   [flags](void *mapping, size_t count) { return ::msync(mapping, count, flags); });
    }
    void advise(uint64_t offset, uint64_t length, int advice) const {
        file_range(offset, length,
                   [advice](void *mapping, size_t count) { return ::madvise(mapping, count, advice); });
    }
};

inline uint64_t FileMetadataCache::size(const FileBacking &file) {
    auto &state = current();
    if (!state.active)
        return file.size();
    const auto identity = reinterpret_cast<uintptr_t>(&file);
    auto &entry = state.entries[((identity >> 4) ^ (identity >> 12)) & (state.entries.size() - 1)];
    if (entry.file != &file || entry.generation != state.active)
        entry = {&file, state.active, file.size()};
    return entry.bytes;
}

// Accessed under the page lock or Memory::AccessGuard. Allocate a token only
// for an active reservation; ordinary writes invalidate it without a global
// atomic increment.
struct ReservationState {
    PageMutex mutex;
    inline static std::atomic<uint64_t> next_token{1};
    uint64_t token = 0, mapping_revision = 0;
    void written() {
        token = 0;
    }
    uint64_t epoch(uint64_t revision) {
        if (!token || mapping_revision != revision) {
            token = next_token.fetch_add(1, std::memory_order_relaxed);
            mapping_revision = revision;
        }
        return token;
    }
    static std::shared_ptr<ReservationState> file_page(uint64_t device, uint64_t inode, uint64_t offset) {
        using Identity = std::tuple<uint64_t, uint64_t, uint64_t>;
        static std::mutex mutex;
        static std::map<Identity, std::weak_ptr<ReservationState>> pages;
        static unsigned allocations = 0;
        std::lock_guard<std::mutex> lock(mutex);
        auto &slot = pages[{device, inode, offset}];
        auto result = slot.lock();
        if (!result) {
            result = std::make_shared<ReservationState>();
            slot = result;
            if (++allocations == 256) {
                for (auto it = pages.begin(); it != pages.end();) {
                    if (it->second.expired())
                        it = pages.erase(it);
                    else
                        ++it;
                }
                allocations = 0;
            }
        }
        return result;
    }
};

struct FilePage {
    std::shared_ptr<FileBacking> backing;
    std::shared_ptr<ReservationState> reservation;
    void *mapping = MAP_FAILED;
    size_t length = 0, prefix = 0;

    FilePage(std::shared_ptr<FileBacking> file, uint64_t at, bool writing)
        : backing(std::move(file)),
          reservation(ReservationState::file_page(backing->device, backing->inode, at)) {
        const uint64_t alignment = host_page_size();
        const uint64_t aligned = at / alignment * alignment;
        prefix = size_t(at - aligned);
        length = prefix + page_size;
        mapping = ::mmap(nullptr, length, PROT_READ | (writing ? PROT_WRITE : 0), MAP_SHARED,
                         backing->descriptor.get(), off_t(aligned));
        if (mapping == MAP_FAILED)
            guest_error(linux_errno(errno));
    }
    ~FilePage() {
        if (mapping != MAP_FAILED)
            ::munmap(mapping, length);
    }
    uint8_t *data() const {
        return static_cast<uint8_t *>(mapping) + prefix;
    }
    void advise(int advice) const {
        if (::madvise(mapping, length, advice) < 0)
            guest_error(linux_errno(errno));
    }
    bool resident() const {
#if defined(__APPLE__)
        char state = 0;
#else
        unsigned char state = 0;
#endif
        if (::mincore(mapping, size_t(host_page_size()), &state) < 0)
            guest_error(linux_errno(errno));
        return state & 1;
    }
};

struct Page {
    ReservationState private_reservation;
    std::unique_ptr<uint8_t[]> owned;
    std::shared_ptr<FilePage> file;
    Page() : owned(std::make_unique<uint8_t[]>(page_size)) {}
    explicit Page(std::shared_ptr<FilePage> mapping) : file(std::move(mapping)) {}
    uint8_t *data() const {
        return file ? file->data() : owned.get();
    }
    ReservationState *reservation() {
        return file ? file->reservation.get() : &private_reservation;
    }
    std::shared_ptr<Page> private_copy() const {
        auto copy = std::make_shared<Page>();
        std::memcpy(copy->data(), data(), page_size);
        return copy;
    }
};

struct MappingObject;
struct MappingView {
    std::weak_ptr<MappingObject> object;
    uint64_t start, end;
    ~MappingView();
};

struct MappingObject : std::enable_shared_from_this<MappingObject> {
    inline static std::atomic<uint64_t> next_identity{1};
    const uint64_t identity = next_identity.fetch_add(1, std::memory_order_relaxed);
    std::shared_ptr<FileBacking> file;
    std::shared_ptr<files::MemoryFileMapping> seal_token;
    std::recursive_mutex mutex;
    bool collecting = false;
    std::vector<std::weak_ptr<MappingView>> views;
    std::shared_ptr<MappingView> view(uint64_t start, uint64_t length) {
        auto result = std::make_shared<MappingView>();
        result->object = shared_from_this();
        result->start = start;
        result->end = start + length;
        std::lock_guard<std::recursive_mutex> lock(mutex);
        views.emplace_back(result);
        return result;
    }
    void collect() noexcept {
        std::lock_guard<std::recursive_mutex> lock(mutex);
        // A range acquired below can lose its last external owner concurrently;
        // its destructor may reenter collection on this same executor.
        if (collecting)
            return;
        collecting = true;
        views.erase(
            std::remove_if(views.begin(), views.end(), [](const auto &view) { return view.expired(); }),
            views.end());
        for (auto page = anonymous_pages.begin(); page != anonymous_pages.end();) {
            bool referenced = false;
            for (const auto &view : views) {
                if (auto range = view.lock())
                    referenced |= page->first >= range->start && page->first < range->end;
            }
            if (referenced)
                ++page;
            else
                page = anonymous_pages.erase(page);
        }
        for (auto page = file_pages.begin(); page != file_pages.end();) {
            if (page->second.expired())
                page = file_pages.erase(page);
            else
                ++page;
        }
        collecting = false;
    }
    // Anonymous shared pages must outlive any one process's page-table entry.
    std::unordered_map<uint64_t, std::shared_ptr<Page>> anonymous_pages;
    std::unordered_map<uint64_t, std::weak_ptr<Page>> file_pages;

    std::shared_ptr<Page> page(uint64_t offset, bool writable) {
        std::lock_guard<std::recursive_mutex> lock(mutex);
        if (!file) {
            auto &slot = anonymous_pages[offset];
            if (!slot)
                slot = std::make_shared<Page>();
            return slot;
        }
        auto &slot = file_pages[offset];
        auto result = slot.lock();
        if (!result) {
            result = std::make_shared<Page>(std::make_shared<FilePage>(file, offset, writable));
            slot = result;
        }
        return result;
    }
    bool has_anonymous_page(uint64_t offset) {
        std::lock_guard<std::recursive_mutex> lock(mutex);
        return anonymous_pages.find(offset) != anonymous_pages.end();
    }
};

inline MappingView::~MappingView() {
    if (auto mapping = object.lock())
        mapping->collect();
}
} // namespace operit::arm64::memory_detail
