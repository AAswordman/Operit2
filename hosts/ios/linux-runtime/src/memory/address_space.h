#pragma once
#include "base/support.h"
#include "memory/access_sync.h"
#include "memory/backing_store.h"
#include <algorithm>
#include <array>
#include <cstring>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <tuple>
#include <unordered_map>
#include <vector>

namespace operit::arm64 {
class Memory {
  public:
    struct Backing {
        std::string path;
        uint64_t offset = 0, device = 0, inode = 0;
        bool shared = false;
    };
    struct Mapping {
        uint64_t start, end;
        unsigned permissions;
        Backing backing;
    };
    struct SharedIdentity {
        uint64_t device, inode, anonymous, offset;
        bool operator==(const SharedIdentity &other) const {
            return std::tie(device, inode, anonymous, offset) ==
                   std::tie(other.device, other.inode, other.anonymous, other.offset);
        }
        bool operator<(const SharedIdentity &other) const {
            return std::tie(device, inode, anonymous, offset) <
                   std::tie(other.device, other.inode, other.anonymous, other.offset);
        }
    };

  private:
    using Page = memory_detail::Page;
    using MappingObject = memory_detail::MappingObject;
    struct Region {
        uint64_t end = 0;
        unsigned permissions = 0;
        Backing backing;
        std::shared_ptr<MappingObject> object;
        std::shared_ptr<memory_detail::MappingView> view;
        uint64_t object_offset = 0;
        unsigned maximum_permissions = read_perm | write_perm | exec_perm;
        int advice = MADV_NORMAL;
        bool dont_fork = false, wipe_on_fork = false;
    };
    struct Translation {
        uint64_t tag = UINT64_MAX;
        uint64_t epoch = 0;
        uint8_t *data = nullptr;
        memory_detail::ReservationState *reservation = nullptr;
        memory_detail::FileBacking *file = nullptr;
        uint64_t file_offset = 0, file_generation = 0;
        unsigned permissions = 0;
        bool shared = false, file_storage = false;
    };
    // Different address spaces can alias the same shared anonymous pages or files.
    inline static std::atomic<uint64_t> next_identity_{1};
    const uint64_t identity_ = next_identity_.fetch_add(1, std::memory_order_relaxed);
    inline static std::atomic<uint64_t> exclusive_revision_{1};
    std::atomic<bool> parallel_access_{true};
    std::map<uint64_t, Region> regions_;
    std::unordered_map<uint64_t, std::shared_ptr<Page>> pages_;
    std::array<Translation, 1024> tlb_{};
    struct ThreadTranslations {
        uint64_t identity = 0;
        std::array<Translation, 1024> entries{};
    };
    static ThreadTranslations &thread_translations() {
        static thread_local ThreadTranslations translations;
        return translations;
    }
    Translation &translation(uint64_t address, bool parallel) {
        if (!parallel)
            return tlb_[translation_index(address)];
        auto &cache = thread_translations();
        if (cache.identity != identity_) {
            cache.entries.fill({});
            cache.identity = identity_;
        }
        return cache.entries[translation_index(address)];
    }
    Translation &translation(uint64_t address) {
        return translation(address, parallel_access_.load(std::memory_order_relaxed));
    }
    inline static memory_detail::FileMetadataCache file_metadata_;
    static size_t translation_index(uint64_t address) {
        const uint64_t page = address / page_size;
        return (page ^ (page >> 10)) & 1023;
    }
    uint64_t epoch_ = 1;
    uint64_t limit_;
    std::shared_ptr<std::atomic<uint64_t>> shared_revision_ = std::make_shared<std::atomic<uint64_t>>(1);
    std::atomic<uint64_t> observed_shared_revision_{1};
    uint64_t mapping_summary_epoch_ = 0;
    bool file_mapping_ = false, executable_file_ = false;

    void refresh_mapping_summary() {
        file_mapping_ = executable_file_ = false;
        for (const auto &entry : regions_) {
            const auto &region = entry.second;
            if (region.object && region.object->file) {
                file_mapping_ = true;
                executable_file_ |= bool(region.permissions & exec_perm);
            }
        }
        mapping_summary_epoch_ = epoch_;
    }
    void split(uint64_t at) {
        auto it = regions_.upper_bound(at);
        if (it == regions_.begin())
            return;
        --it;
        if (at > it->first && at < it->second.end) {
            auto tail = it->second;
            const uint64_t displacement = at - it->first;
            if (tail.backing.inode)
                tail.backing.offset += displacement;
            tail.object_offset += displacement;
            if (tail.object) {
                auto left_view = tail.object->view(it->second.object_offset, displacement);
                auto right_view = tail.object->view(tail.object_offset, tail.end - at);
                it->second.view = std::move(left_view);
                tail.view = std::move(right_view);
            }
            it->second.end = at;
            regions_.emplace(at, std::move(tail));
        }
    }
    void invalidate(bool executable = true) {
        ++epoch_;
        refresh_mapping_summary();
        exclusive_revision_.fetch_add(1, std::memory_order_relaxed);
        if (executable)
            code_epoch.fetch_add(1, std::memory_order_relaxed);
    }
    void written(const Translation &translation) {
        translation.reservation->written();
        if (translation.permissions & exec_perm || translation.shared)
            code_epoch.fetch_add(1, std::memory_order_relaxed);
        if (translation.shared)
            observed_shared_revision_.store(shared_revision_->fetch_add(1, std::memory_order_relaxed) + 1,
                                           std::memory_order_relaxed);
    }
    void require_access(uint64_t address, uint64_t size, unsigned permissions) const;
    void coalesce();
    void install(uint64_t address, uint64_t size, Region region, bool replace);
    void populate(uint64_t address, uint64_t size, unsigned permissions);

  public:
    class AccessGuard {
        memory_detail::AccessSynchronization::WriteGuard guard_;

      public:
        explicit AccessGuard(const Memory &memory)
            : guard_(memory.parallel_access_.load(std::memory_order_relaxed) || memory.file_mapping_) {}
        AccessGuard(const AccessGuard &) = delete;
        AccessGuard &operator=(const AccessGuard &) = delete;
    };
    class ExecutionScope {
        memory_detail::FileMetadataCache &metadata_;
        uint64_t previous_;

      public:
        explicit ExecutionScope(Memory &memory)
            : metadata_(memory.file_metadata_), previous_(metadata_.enter()) {}
        ~ExecutionScope() {
            metadata_.leave(previous_);
        }
        ExecutionScope(const ExecutionScope &) = delete;
        ExecutionScope &operator=(const ExecutionScope &) = delete;
    };
    std::atomic<uint64_t> code_epoch{1};
    explicit Memory(uint64_t limit) : limit_(limit) {}
    // Change only at a CPU barrier. Serial CPUs still synchronize actual file
    // pages and raw spans; anonymous and privately copied pages bypass locks.
    void set_parallel_access(bool enabled) noexcept {
        parallel_access_.store(enabled, std::memory_order_relaxed);
    }
    uint64_t exclusive_epoch(uint64_t address) {
        PageAccess access(*this, address, read_perm, true);
        return access.entry().reservation->epoch(exclusive_revision_.load(std::memory_order_relaxed));
    }
    Memory(const Memory &) = delete;
    Memory &operator=(const Memory &) = delete;
    std::unique_ptr<Memory> fork_clone();
    void synchronize_shared();
    std::optional<SharedIdentity> shared_identity(uint64_t address) const;
    uint64_t limit() const {
        return limit_;
    }
    std::vector<Mapping> mappings() const {
        const AccessGuard access(*this);
        std::vector<Mapping> result;
        for (const auto &item : regions_)
            result.push_back({item.first, item.second.end, item.second.permissions, item.second.backing});
        return result;
    }
    void describe(uint64_t start, uint64_t size, const Backing &backing) {
        const AccessGuard access(*this);
        if (!accessible(start, size, 0))
            guest_error(12);
        split(start);
        split(start + size);
        for (auto it = regions_.lower_bound(start); it != regions_.end() && it->first < start + size; ++it) {
            if (backing.shared != it->second.backing.shared)
                guest_error(22);
            it->second.backing = backing;
            if (backing.inode)
                it->second.backing.offset += it->first - start;
        }
        coalesce();
    }
    uint64_t resident() const {
        const AccessGuard access(*this);
        return pages_.size() * page_size;
    }
    static bool valid_range(uint64_t address, uint64_t size) {
        return address >= page_size && address < address_limit && size <= address_limit - address;
    }
    bool free(uint64_t address, uint64_t size) const {
        const AccessGuard access(*this);
        if (!valid_range(address, size))
            return false;
        auto next = regions_.lower_bound(address);
        if (next != regions_.end() && next->first < address + size)
            return false;
        return next == regions_.begin() || std::prev(next)->second.end <= address;
    }
    uint64_t find_free(uint64_t start, uint64_t size) const;
    bool accessible(uint64_t address, uint64_t size, unsigned permissions) const {
        const memory_detail::AccessSynchronization::ReadGuard access(
            parallel_access_.load(std::memory_order_relaxed) || file_mapping_);
        if (size == 0)
            return true;
        if (!valid_range(address, size))
            return false;
        const uint64_t end = address + size;
        auto it = regions_.upper_bound(address);
        if (it == regions_.begin())
            return false;
        --it;
        while (address < end) {
            if (it == regions_.end() || it->first > address || it->second.end <= address ||
                (it->second.permissions & permissions) != permissions)
                return false;
            address = std::min(end, it->second.end);
            ++it;
        }
        return true;
    }
    // Capture only weak page references: queued host I/O must neither keep an
    // address space alive nor write into a later mapping at the same address.
    // Both capture and validation are called at the coordinator barrier.
    std::function<bool(const Memory &)> capture_write_lease(
        const std::vector<std::pair<uint64_t, uint64_t>> &ranges) const {
        const AccessGuard access(*this);
        struct PageLease {
            uint64_t address;
            std::weak_ptr<Page> page;
        };
        std::vector<PageLease> pages;
        for (const auto &[address, length] : ranges) {
            if (!accessible(address, length, write_perm))
                return {};
            if (!length)
                continue;
            const uint64_t end = address + length;
            for (uint64_t page = align_down(address); page < end; page += page_size) {
                const auto found = pages_.find(page);
                if (found == pages_.end())
                    return {};
                pages.push_back({page, found->second});
            }
        }
        const uint64_t identity = identity_;
        return [identity, ranges, pages = std::move(pages)](const Memory &memory) {
            const AccessGuard guard(memory);
            if (memory.identity_ != identity)
                return false;
            for (const auto &[address, length] : ranges)
                if (!memory.accessible(address, length, write_perm))
                    return false;
            for (const auto &entry : pages) {
                const auto expected = entry.page.lock();
                const auto found = memory.pages_.find(entry.address);
                if (!expected || found == memory.pages_.end() || found->second != expected)
                    return false;
            }
            return true;
        };
    }
    void unmap(uint64_t address, uint64_t size);
    void map(uint64_t address, uint64_t size, unsigned permissions, bool replace = false,
             bool shared = false);
    void map_file(uint64_t address, uint64_t size, unsigned permissions, int descriptor,
                  const Backing &backing, bool replace = false, bool permit_write = true,
                  std::shared_ptr<void> open_reference = nullptr);
    void protect(uint64_t address, uint64_t size, unsigned permissions);
    void sync(uint64_t address, uint64_t size, unsigned flags);
    uint64_t remap(uint64_t address, uint64_t old_size, uint64_t new_size, unsigned flags,
                   uint64_t new_address = 0, uint64_t hint = 0x1000000000);
    void advise(uint64_t address, uint64_t size, int advice);
    std::vector<uint8_t> residency(uint64_t address, uint64_t size);

    // Coordinator-only: all guest workers sharing these pages must be stopped.
    // The host buffer must be valid and must not overlap guest page storage.
    // Bypass current VMA permissions for reads and private COW writes, without
    // changing them. Shared writes require current and maximum write access.
    // An empty transfer succeeds; otherwise a null buffer or any inaccessible
    // page fails. On failure no bytes are transferred, although pages may have
    // been faulted in or privately copied during preparation.
    // Synchronize shared aliases with synchronize_shared() before they resume.
    // Concurrent external file truncation is not supported by the mmap backend.
    bool ptrace_read(uint64_t address, void *destination, size_t size);
    bool ptrace_write(uint64_t address, const void *source, size_t size);

  private:
    bool ptrace_transfer(uint64_t address, void *buffer, size_t size, bool writing);

    [[gnu::noinline]] void refill(Translation &cached, uint64_t address, unsigned permissions,
                                 bool force = false) {
        const uint64_t page = align_down(address);
        auto region = regions_.upper_bound(address);
        if (region == regions_.begin())
            throw MemoryFault(address, permissions, MemoryFault::Kind::Unmapped);
        --region;
        auto &mapping = region->second;
        if (address >= mapping.end)
            throw MemoryFault(address, permissions, MemoryFault::Kind::Unmapped);
        const bool force_write_denied =
            force && (permissions & write_perm) &&
            (!(mapping.maximum_permissions & write_perm) ||
             (mapping.backing.shared &&
              (!(mapping.permissions & write_perm) ||
               (mapping.object && mapping.object->file && !mapping.object->file->writable) ||
               (mapping.object && mapping.object->seal_token && !mapping.object->seal_token->may_write()))));
        if ((!force && (mapping.permissions & permissions) != permissions) || force_write_denied)
            throw MemoryFault(address, permissions, MemoryFault::Kind::Protection);
        const uint64_t offset = mapping.object_offset + page - region->first;
        auto *file = mapping.object ? mapping.object->file.get() : nullptr;
        if (file)
            file->check_page(offset, address, permissions, force ? nullptr : &file_metadata_);
        auto found = pages_.find(page);
        if (found == pages_.end()) {
            if (resident() + page_size > limit_)
                throw Failure("guest resident memory limit exceeded");
            auto storage = mapping.object ? mapping.object->page(offset, mapping.backing.shared && file &&
                                                                             file->writable)
                                          : std::make_shared<Page>();
            if (storage->file && mapping.advice != MADV_NORMAL)
                storage->file->advise(mapping.advice);
            found = pages_.emplace(page, std::move(storage)).first;
        }
        auto &storage = found->second;
        if (!mapping.backing.shared && permissions & write_perm && (storage.use_count() > 1 || storage->file)) {
            storage = storage->private_copy();
            // Other workers may have cached the previous physical page.
            ++epoch_;
        }
        cache_translation(cached, page, region->first, mapping, storage);
    }
    void cache_translation(Translation &cached, uint64_t page, uint64_t start,
                           const Region &mapping, const std::shared_ptr<Page> &storage) {
        unsigned available = mapping.permissions;
        if (!mapping.backing.shared && (storage.use_count() > 1 || storage->file))
            available &= ~write_perm;
        cached = Translation{page,
                             epoch_,
                             storage->data(),
                             storage->reservation(),
                             mapping.object ? mapping.object->file.get() : nullptr,
                             mapping.object_offset + page - start,
                             file_metadata_.generation(),
                             available,
                             mapping.backing.shared,
                             bool(storage->file)};
    }
    // A software TLB collision is not a mapping change. With the reader gate
    // held, an existing resident page can be cached without stopping all CPUs.
    // Allocation and private-file/fork COW still require the writer path.
    bool refill_resident(Translation &cached, uint64_t address, unsigned permissions) {
        auto region = regions_.upper_bound(address);
        if (region == regions_.begin())
            return false;
        --region;
        const auto &mapping = region->second;
        if (address >= mapping.end || (mapping.permissions & permissions) != permissions)
            return false;
        const uint64_t page = align_down(address);
        const auto found = pages_.find(page);
        if (found == pages_.end())
            return false;
        const auto &storage = found->second;
        if (!mapping.backing.shared && (permissions & write_perm) &&
            (storage.use_count() > 1 || storage->file))
            return false;
        if (mapping.object && mapping.object->file)
            mapping.object->file->check_page(mapping.object_offset + page - region->first,
                                             address, permissions, &file_metadata_);
        cache_translation(cached, page, region->first, mapping, storage);
        return true;
    }
    [[gnu::noinline]] void validate_file(Translation &cached, uint64_t address, unsigned permissions) {
        cached.file->check_page(cached.file_offset, address, permissions, &file_metadata_);
        cached.file_generation = file_metadata_.generation();
    }

    [[gnu::always_inline]] uint8_t *translate_locked(uint64_t address, unsigned permissions) {
        auto &cached = translation(address);
        if (cached.tag != align_down(address) || cached.epoch != epoch_ ||
            (cached.permissions & permissions) != permissions)
            refill(cached, address, permissions);
        else if (cached.file &&
                 (!file_metadata_.generation() || cached.file_generation != file_metadata_.generation()))
            validate_file(cached, address, permissions);
        return cached.data + address % page_size;
    }
    class PageAccess {
        memory_detail::AccessSynchronization::ReadGuard read_;
        memory_detail::AccessSynchronization::PageGuard lock_;
        Translation *entry_ = nullptr;
        uint64_t address_;

        [[gnu::noinline]] void prepare(Memory &memory, Translation &entry, uint64_t address,
                                       unsigned permissions, bool parallel) {
            while (entry.tag != align_down(address) || entry.epoch != memory.epoch_ ||
                   (entry.permissions & permissions) != permissions) {
                // Prepared helpers may retain a reader even in serial execution.
                if ((parallel || memory_detail::AccessSynchronization::read_depth()) &&
                    memory.refill_resident(entry, address, permissions))
                    return;
                read_.unlock();
                assert(!memory_detail::AccessSynchronization::read_depth());
                {
                    const AccessGuard update(memory);
                    memory.translate_locked(address, permissions);
                }
                read_.lock(parallel);
            }
        }

      public:
        [[gnu::always_inline]] PageAccess(Memory &memory, uint64_t address, unsigned permissions,
                                        bool exclusive = false)
            : address_(address) {
            const bool parallel = memory.parallel_access_.load(std::memory_order_relaxed);
            read_.lock(parallel);
            auto &entry = memory.translation(address, parallel);
            if (entry.tag != align_down(address) || entry.epoch != memory.epoch_ ||
                (entry.permissions & permissions) != permissions)
                prepare(memory, entry, address, permissions, parallel);
            if (!parallel)
                read_.lock(entry.file_storage);
            if (entry.file && (!file_metadata_.generation() ||
                               entry.file_generation != file_metadata_.generation()))
                memory.validate_file(entry, address, permissions);
            entry_ = &entry;
            // Immutable private pages need only the mapping reader gate.
            const bool immutable = !(entry.permissions & write_perm) && !entry.shared &&
                                   !entry.file_storage && !exclusive;
            if ((parallel || entry.file_storage) && !immutable &&
                !memory_detail::AccessSynchronization::exclusive())
                lock_.lock(entry.reservation->mutex, exclusive || (permissions & write_perm));
        }
        uint8_t *data() const {
            return entry_->data + address_ % page_size;
        }
        Translation &entry() const {
            return *entry_;
        }
        PageAccess(const PageAccess &) = delete;
        PageAccess &operator=(const PageAccess &) = delete;
    };
  public:
    // Prepare a bounded set of already resident pages for a compiled helper.
    // Keep mappings stable under a reader gate, then acquire physical page
    // locks in one order. A cold page or COW declines before any page is held;
    // the caller can release the gate and use the ordinary writer path.
    class PreparedAccess {
        struct Entry {
            memory_detail::ReservationState *page = nullptr;
            bool writing = false;
        };
        static constexpr size_t capacity = 16;
        Memory &memory_;
        memory_detail::AccessSynchronization::ReadGuard read_;
        std::array<Entry, capacity> entries_{};
        std::array<memory_detail::AccessSynchronization::PageGuard, capacity> locks_{};
        size_t count_ = 0;
        bool available_ = true, locked_ = false;

      public:
        explicit PreparedAccess(Memory &memory)
            : memory_(memory), read_(memory.parallel_access_.load(std::memory_order_relaxed) ||
                                     memory.file_mapping_) {}
        bool add(uint64_t address, uint64_t size, unsigned permissions) {
            assert(!locked_);
            if (!available_)
                return false;
            if (!size)
                return true;
            if (!valid_range(address, size) || size > capacity * page_size) {
                cancel();
                return false;
            }
            while (size) {
                if (count_ == capacity) {
                    cancel();
                    return false;
                }
                auto &entry = memory_.translation(address);
                // This also rejects a private page that still needs COW.
                // No mapping or physical storage is changed on this path.
                try {
                    if (!memory_.refill_resident(entry, address, permissions)) {
                        cancel();
                        return false;
                    }
                } catch (const MemoryFault &) {
                    // Eligibility probes cannot move a guest fault ahead of
                    // the helper instruction/register updates that cause it.
                    cancel();
                    return false;
                }
                entries_[count_++] = {entry.reservation, bool(permissions & write_perm)};
                const uint64_t count = std::min<uint64_t>(size, page_size - address % page_size);
                address += count;
                size -= count;
            }
            return true;
        }
        bool lock() {
            assert(!locked_);
            if (!available_)
                return false;
            std::sort(entries_.begin(), entries_.begin() + count_, [](const Entry &left, const Entry &right) {
                return std::less<memory_detail::ReservationState *>{}(left.page, right.page);
            });
            size_t distinct = 0;
            for (size_t index = 0; index < count_; ++index) {
                if (distinct && entries_[distinct - 1].page == entries_[index].page)
                    entries_[distinct - 1].writing |= entries_[index].writing;
                else
                    entries_[distinct++] = entries_[index];
            }
            count_ = distinct;
            if (!memory_detail::AccessSynchronization::exclusive()) {
                for (size_t index = 0; index < count_; ++index)
                    locks_[index].lock(entries_[index].page->mutex, entries_[index].writing);
            }
            locked_ = true;
            return true;
        }
        // Complete a bounded pipelined copy while the corresponding resident
        // pages are already locked. File/shared/executable pages and physical
        // aliases retain the ordinary per-instruction implementation.
        bool copy_pipeline(uint64_t destination, uint64_t source, const void *first, void *last,
                           size_t size) {
            assert(locked_);
            if (size < 64 || !valid_range(source, size) || !valid_range(destination, size) ||
                size > page_size - source % page_size || size > page_size - destination % page_size)
                return false;
            auto lookup = [&](uint64_t address, unsigned permissions, Translation &result) {
                auto &entry = memory_.translation(address);
                try {
                    if ((entry.tag != align_down(address) || entry.epoch != memory_.epoch_ ||
                         (entry.permissions & permissions) != permissions) &&
                        !memory_.refill_resident(entry, address, permissions))
                        return false;
                } catch (const MemoryFault &) {
                    return false;
                }
                result = entry;
                for (size_t index = 0; index < count_; ++index)
                    if (entries_[index].page == entry.reservation &&
                        (!(permissions & write_perm) || entries_[index].writing))
                        return true;
                return false;
            };
            Translation input, output;
            if (!lookup(source, read_perm, input) || !lookup(destination, write_perm, output) ||
                input.reservation == output.reservation || input.shared || output.shared || input.file ||
                output.file || (output.permissions & exec_perm))
                return false;
            auto *from = input.data + source % page_size;
            auto *to = output.data + destination % page_size;
            memory_.written(output);
            std::memcpy(to, first, 64);
            if (size > 64)
                std::memcpy(to + 64, from, size - 64);
            std::memcpy(last, from + size - 64, 64);
            return true;
        }
        void cancel() noexcept {
            assert(!locked_);
            available_ = false;
            read_.unlock();
        }
        PreparedAccess(const PreparedAccess &) = delete;
        PreparedAccess &operator=(const PreparedAccess &) = delete;
    };
    class AtomicAccess {
        PageAccess page_;

      public:
        AtomicAccess(Memory &memory, uint64_t address, unsigned permissions)
            : page_(memory, address, permissions, true) {}
        AtomicAccess(const AtomicAccess &) = delete;
        AtomicAccess &operator=(const AtomicAccess &) = delete;
    };
  public:
    // Keep an AccessGuard alive while using the returned guest span.
    [[gnu::always_inline]] uint8_t *translate(uint64_t address, unsigned permissions) {
        const AccessGuard access(*this);
        auto *data = translate_locked(address, permissions);
        if (permissions & write_perm)
            written(translation(address));
        return data;
    }
    // Guest data and instructions share the same backing bytes. Maintenance
    // still validates the operand (including file-backing faults), establishes
    // ordering, and makes every interpreter retire/revalidate its decoded code.
    // Do not pass guest virtual addresses to native host cache instructions.
    void synchronize_code(uint64_t address) {
        PageAccess access(*this, address, read_perm);
        std::atomic_thread_fence(std::memory_order_seq_cst);
        code_epoch.fetch_add(1, std::memory_order_seq_cst);
    }
    bool validate_code(uint64_t address, const void *expected, size_t size, uint64_t &validated_epoch) {
        if (!size) {
            validated_epoch = code_epoch.load(std::memory_order_relaxed);
            return true;
        }
        if (valid_range(address, size) && size <= page_size - address % page_size) {
            PageAccess access(*this, address, exec_perm);
            if (std::memcmp(access.data(), expected, size))
                return false;
            // Capture before releasing the code page, so a concurrent store
            // cannot relabel these verified bytes with a newer code epoch.
            validated_epoch = code_epoch.load(std::memory_order_relaxed);
            return true;
        }
        const AccessGuard access(*this);
        require_access(address, size, exec_perm);
        const auto *bytes = static_cast<const uint8_t *>(expected);
        while (size) {
            const size_t count = std::min<size_t>(size, page_size - address % page_size);
            if (std::memcmp(translate_locked(address, exec_perm), bytes, count))
                return false;
            address += count;
            bytes += count;
            size -= count;
        }
        validated_epoch = code_epoch.load(std::memory_order_relaxed);
        return true;
    }
    template <size_t Size>
    [[gnu::always_inline]] void copy_out(uint64_t address, void *destination,
                                        unsigned permissions = read_perm) {
        static_assert(Size > 0);
        if (valid_range(address, Size) && Size <= page_size - address % page_size) {
            PageAccess access(*this, address, permissions);
            std::memcpy(destination, access.data(), Size);
        } else
            copy_out(address, destination, Size, permissions);
    }
    template <size_t Size>
    [[gnu::always_inline]] void copy_in(uint64_t address, const void *source) {
        static_assert(Size > 0);
        if (valid_range(address, Size) && Size <= page_size - address % page_size) {
            PageAccess access(*this, address, write_perm);
            written(access.entry());
            std::memcpy(access.data(), source, Size);
        } else
            copy_in(address, source, Size);
    }
    void copy_out(uint64_t address, void *destination, size_t size, unsigned permissions = read_perm) {
        if (size && valid_range(address, size) && size <= page_size - address % page_size) {
            PageAccess access(*this, address, permissions);
            std::memcpy(destination, access.data(), size);
            return;
        }
        const AccessGuard access(*this);
        require_access(address, size, permissions);
        auto *out = static_cast<uint8_t *>(destination);
        while (size) {
            size_t count = std::min<size_t>(size, page_size - address % page_size);
            std::memcpy(out, translate_locked(address, permissions), count);
            out += count;
            address += count;
            size -= count;
        }
    }
    void copy_in(uint64_t address, const void *source, size_t size) {
        if (size && valid_range(address, size) && size <= page_size - address % page_size) {
            PageAccess access(*this, address, write_perm);
            written(access.entry());
            std::memcpy(access.data(), source, size);
            return;
        }
        const AccessGuard access(*this);
        require_access(address, size, write_perm);
        auto *in = static_cast<const uint8_t *>(source);
        while (size) {
            size_t count = std::min<size_t>(size, page_size - address % page_size);
            auto *target = translate_locked(address, write_perm);
            written(translation(address));
            std::memcpy(target, in, count);
            in += count;
            address += count;
            size -= count;
        }
    }
    template <unsigned Size>
    [[gnu::always_inline]] uint64_t load(uint64_t address, unsigned permissions = read_perm) {
        static_assert(Size == 1 || Size == 2 || Size == 4 || Size == 8);
        uint64_t value = 0;
        copy_out<Size>(address, &value, permissions);
        return value;
    }
    template <unsigned Size>
    [[gnu::always_inline]] void store(uint64_t address, uint64_t value) {
        static_assert(Size == 1 || Size == 2 || Size == 4 || Size == 8);
        copy_in<Size>(address, &value);
    }
    uint64_t load(uint64_t address, unsigned size, unsigned permissions = read_perm) {
        if (size > sizeof(uint64_t))
            throw Failure("invalid scalar load size");
        uint64_t value = 0;
        copy_out(address, &value, size, permissions);
        return value;
    }
    void store(uint64_t address, uint64_t value, unsigned size) {
        if (size > sizeof(uint64_t))
            throw Failure("invalid scalar store size");
        copy_in(address, &value, size);
    }
    std::string string(uint64_t address) {
        const AccessGuard access(*this);
        std::string value;
        while (value.size() < max_path) {
            if (!accessible(address, 1, read_perm))
                guest_error(14);
            const size_t count = std::min<size_t>(max_path - value.size(), page_size - address % page_size);
            const auto *data = translate_locked(address, read_perm);
            const auto *zero = static_cast<const uint8_t *>(std::memchr(data, 0, count));
            value.append(reinterpret_cast<const char *>(data), zero ? size_t(zero - data) : count);
            if (zero)
                return value;
            address += count;
        }
        guest_error(36);
    }
};
} // namespace operit::arm64

#include "memory/mapping_operations.h"
