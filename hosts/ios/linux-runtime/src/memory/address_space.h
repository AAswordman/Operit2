#pragma once
#include "base/support.h"
#include "memory/access_sync.h"
#include "memory/backing_store.h"
#include <cstddef>
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
    struct alignas(64) ExecutionTranslation {
        uint64_t tag = UINT64_MAX;
        uint64_t epoch = 0;
        uint8_t *data = nullptr;
        memory_detail::ReservationState *reservation = nullptr;
        memory_detail::FileBacking *file = nullptr;
        uint64_t file_offset = 0, file_generation = 0;
        unsigned permissions = 0;
        bool shared = false, file_storage = false;
        uint8_t serial_permissions = 0;
    };
    // Serial capabilities carry full page tags and a host-address bias. They
    // expire on scope entry or mapping changes; physical file pages stay locked.
    struct ExecutionPage {
        uint64_t read_tag=UINT64_MAX;
        uintptr_t bias=0;
        uint64_t write_tag=UINT64_MAX;
        memory_detail::ReservationState *reservation=nullptr;
    };
    static_assert(sizeof(ExecutionPage)==32 && offsetof(ExecutionPage,bias)==8 &&
                  offsetof(ExecutionPage,write_tag)==16 && offsetof(ExecutionPage,reservation)==24);
    using CodeWords = std::vector<uint32_t>;
    // Owned by one address-space snapshot cache and its vCPU regions. Serial
    // execution alone publishes these proofs; parallel execution uses the
    // synchronized span-validation path. UINT64_MAX denotes private storage.
    struct CodeProof {
        const uint64_t identity;
        uint64_t epoch = 0, generation = 0, verified = 0;
        // Borrowed page pointers may be used only by serial native execution
        // holding the mapping reader, at these exact code/mapping epochs.
        uint64_t native_epoch=0,native_mapping_epoch=0;
        const uint8_t *native_data=nullptr;
        const uint32_t *native_words=nullptr;
        const memory_detail::FileBacking *native_file=nullptr;
        uint64_t native_file_offset=0;
        memory_detail::PageMutex *native_mutex=nullptr;
        void bind_native(const ExecutionTranslation &page,const CodeWords &words,
                         uint64_t code,uint64_t mapping) noexcept {
            native_epoch=page.file ? code:0;
            native_mapping_epoch=mapping;
            native_data=page.data;native_words=words.data();native_file=page.file;
            native_file_offset=page.file_offset;
            native_mutex=page.file_storage ? &page.reservation->mutex:nullptr;
        }
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
        uint64_t diagnostic_epoch = 0, diagnostic_scope = 0;
        uint64_t diagnostic_compared = 0, diagnostic_mismatched = 0;
        void diagnose_comparison(uint64_t at_epoch,uint64_t scope,uint64_t lines) noexcept {
            if (diagnostic_epoch != at_epoch || diagnostic_scope != scope) {
                diagnostic_epoch = at_epoch; diagnostic_scope = scope;
                diagnostic_compared = diagnostic_mismatched = 0;
            }
            OPERIT_MEMORY_DIAGNOSTIC(code_lines_recompared, __builtin_popcountll(lines & diagnostic_compared));
            OPERIT_MEMORY_DIAGNOSTIC(code_lines_recompared_after_mismatch,
                                     __builtin_popcountll(lines & diagnostic_mismatched));
            diagnostic_compared |= lines;
        }
        void diagnose_mismatch(uint64_t lines) noexcept { diagnostic_mismatched |= lines; }
#else
        void diagnose_comparison(uint64_t,uint64_t,uint64_t) noexcept {}
        void diagnose_mismatch(uint64_t) noexcept {}
#endif
        static uint64_t mask(size_t offset,size_t count) noexcept {
            const auto first=offset*4/64,last=(offset+count-1)*4/64;
            return (UINT64_MAX<<first)&(UINT64_MAX>>(63-last));
        }
        explicit CodeProof(uint64_t identity) : identity(identity) {}
        bool current(uint64_t scope,uint64_t mask=UINT64_MAX) const noexcept {
            return (generation == UINT64_MAX || generation == scope) && (verified&mask)==mask;
        }
    };
    static_assert(offsetof(CodeProof,native_epoch)==32 && offsetof(CodeProof,native_mapping_epoch)==40);
    static_assert(offsetof(CodeProof,native_data)==48 && offsetof(CodeProof,native_words)==56);
    static_assert(offsetof(CodeProof,native_file)==64 && offsetof(CodeProof,native_file_offset)==72);
    static_assert(offsetof(CodeProof,native_mutex)==80);
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
    using Translation = ExecutionTranslation;
    // Keep the hot translation in one aligned 64-byte entry. A word-sized
    // serial_permissions field enlarges entries to 72 bytes: indexing needs a
    // multiply, and hot fields can cross cache-line boundaries.
    static_assert(sizeof(Translation)==64 && alignof(Translation)==64);
    // Different address spaces can alias the same shared anonymous pages or files.
    inline static std::atomic<uint64_t> next_identity_{1};
    const uint64_t identity_ = next_identity_.fetch_add(1, std::memory_order_relaxed);
    inline static std::atomic<uint64_t> exclusive_revision_{1};
    std::atomic<bool> parallel_access_{true};
    uint64_t epoch_ = 1;
    std::map<uint64_t, Region> regions_;
    std::unordered_map<uint64_t, std::shared_ptr<Page>> pages_;
    std::array<Translation, 1024> tlb_{};
    std::array<Translation, 1024> tlb_second_{};
    std::array<uint8_t,1024> tlb_replace_{};
    using ExecutionPages=std::array<ExecutionPage,8192>;
    std::unique_ptr<ExecutionPages> execution_pages_;
    struct ThreadTranslations {
        uint64_t identity = 0;
        std::array<Translation, 1024> entries{};
    };
    static ThreadTranslations &thread_translations() {
        static thread_local ThreadTranslations translations;
        return translations;
    }
    Translation &translation(uint64_t address, bool parallel) {
        OPERIT_MEMORY_DIAGNOSTIC(tlb_lookups, 1);
        if (!parallel) {
            const auto index=translation_index(address);
            auto &first=tlb_[index];auto &second=tlb_second_[index];
            const auto tag=align_down(address);
            if(first.tag==tag && first.epoch==epoch_) {
                OPERIT_MEMORY_DIAGNOSTIC(tlb_first_hits, 1); return first;
            }
            if(second.tag==tag && second.epoch==epoch_) {
                OPERIT_MEMORY_DIAGNOSTIC(tlb_second_hits, 1); return second;
            }
            OPERIT_MEMORY_DIAGNOSTIC(tlb_misses, 1);
            if(first.epoch!=epoch_) return first;
            if(second.epoch!=epoch_) return second;
            // Select a victim only on a double miss. Hits retain their page
            // heads; no Translation or page contents are copied on promotion.
            return (tlb_replace_[index]^=1) ? first : second;
        }
        OPERIT_MEMORY_DIAGNOSTIC(tlb_parallel_lookups, 1);
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
    struct alignas(64) CodeValidation {
        uint64_t address = UINT64_MAX, epoch = 0, generation = 0;
        std::shared_ptr<const CodeWords> words;
        uint64_t verified = 0;
    };
    static_assert(sizeof(CodeValidation)==64 && page_size==4096);
    struct CodeValidations {
        uint64_t identity = 0;
        std::array<CodeValidation,256> entries{};
    };
    std::unique_ptr<CodeValidations> serial_code_validations_;
    uint64_t serial_execution_generation_ = 0;
    static CodeValidations &thread_code_validations() {
        static thread_local CodeValidations cache;
        return cache;
    }
    [[gnu::noinline]] bool validate_snapshot_slow(uint64_t address,CodeValidation &entry,
        size_t offset,size_t count,uint64_t mask,uint64_t &epoch) {
        OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_slow, 1);
        const auto before=entry.epoch;
        PageAccess access(*this,address,exec_perm);
        const auto *data=access.data()-address%page_size;
        const auto *expected=reinterpret_cast<const uint8_t *>(entry.words->data());
        const size_t first=offset*4/64,last=(offset+count-1)*4/64;
        // A 64-byte proof amortizes file/page guards across nearby regions.
        // If only an uncovered byte differs, compare the exact dependency and
        // leave this line unproved rather than invalidating unrelated code.
        const bool lines_match=memory_detail::code_bytes_equal(data+first*64,expected+first*64,
            (last-first+1)*64,memory_detail::CodeComparison::Line);
        if(!lines_match && !memory_detail::code_bytes_equal(data+offset*4,expected+offset*4,
            count*4,memory_detail::CodeComparison::Exact)) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_failures, 1); return false;
        }
        epoch=code_epoch.load(std::memory_order_relaxed);
        if(epoch!=before) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_epoch_failures, 1); return false;
        }
        if(lines_match) entry.verified|=mask;
        return true;
    }
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
        if(execution_pages_)execution_pages_->fill({});
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
        Memory &memory_;
        uint64_t previous_;
        bool serial_;
        uint64_t previous_serial_;

      public:
        explicit ExecutionScope(Memory &memory)
            : metadata_(memory.file_metadata_), memory_(memory), previous_(metadata_.enter()),
              serial_(!memory.parallel_access_.load(std::memory_order_relaxed)),
              previous_serial_(serial_ ? memory.serial_execution_generation_ : 0) {
            if(serial_) {
                OPERIT_MEMORY_DIAGNOSTIC(serial_execution_scopes, 1);
                memory_.serial_execution_generation_=metadata_.generation();
            }
        }
        ~ExecutionScope() {
            if(serial_) memory_.serial_execution_generation_=previous_serial_;
            metadata_.leave(previous_);
        }
        ExecutionScope(const ExecutionScope &) = delete;
        ExecutionScope &operator=(const ExecutionScope &) = delete;
    };
    std::atomic<uint64_t> code_epoch{1};
    explicit Memory(uint64_t limit) : limit_(limit) { memory_detail::MemoryDiagnostics::arm(); }
    // Change only at a CPU barrier. Serial CPUs still synchronize actual file
    // pages and raw spans; anonymous and privately copied pages bypass locks.
    void set_parallel_access(bool enabled) noexcept {
        if(enabled && !parallel_access_.load(std::memory_order_relaxed)) {
            if(execution_pages_)execution_pages_->fill({});
            ++epoch_; // No serial capability survives a parallel mutation/COW.
        }
        parallel_access_.store(enabled, std::memory_order_relaxed);
    }
    struct ExecutionView {
        const ExecutionTranslation *first, *second;
        const uint64_t *mapping_epoch;
        const void *pages;
    };
    ExecutionView execution_view() {
        const bool parallel=parallel_access_.load(std::memory_order_relaxed);
        if(!parallel) {
            if(!execution_pages_)execution_pages_=std::make_unique<ExecutionPages>();
            else execution_pages_->fill({});
        }
        return {parallel ? nullptr:tlb_.data(),tlb_second_.data(),&epoch_,
                parallel ? nullptr:execution_pages_->data()};
    }
    bool wasm_serial() const noexcept { return !parallel_access_.load(std::memory_order_relaxed); }
    unsigned wasm_page(uint64_t base,unsigned permissions,void *destination,bool &file) {
        if(!wasm_serial() || !valid_range(base,page_size))return 0;
        const AccessGuard access(*this);
        auto it=regions_.upper_bound(base);
        if(it==regions_.begin())return 0;
        const auto &region=(--it)->second;
        unsigned granted=region.permissions&(read_perm|write_perm);
        if(region.permissions&exec_perm)granted&=~write_perm;
        if(base<it->first || base+page_size>region.end || (granted&permissions)!=permissions || region.backing.shared)return 0;
        copy_out(base,destination,page_size,permissions);
        PageAccess page(*this,base,permissions);file=page.entry().file_storage;
        if(file)granted&=~write_perm;
        return granted;

    }
    uint64_t wasm_revision(uint64_t address,unsigned permissions) {
        PageAccess access(*this,address,permissions,true);
        return access.entry().reservation->epoch(exclusive_revision_.load(std::memory_order_relaxed));
    }
    std::vector<uint64_t> wasm_resident_pages() const {
        std::vector<uint64_t> result;result.reserve(pages_.size());
        for(const auto &page:pages_)result.push_back(page.first);
        return result;
    }
    uint64_t identity() const noexcept { return identity_; }
    static_assert(offsetof(ExecutionTranslation,data)==16 && offsetof(ExecutionTranslation,reservation)==24);
    static_assert(offsetof(ExecutionTranslation,file)==32 && offsetof(ExecutionTranslation,file_generation)==48);
    static_assert(offsetof(ExecutionTranslation,serial_permissions)==62);
    const auto *file_observations() const noexcept { return file_metadata_.observations(); }
    uint64_t execution_generation() const { return file_metadata_.generation(); }
    bool owns_code_proof(const CodeProof &proof) const noexcept { return proof.identity == identity_; }
    uint64_t code_proof_generation() const noexcept {
        return parallel_access_.load(std::memory_order_relaxed) ? 0 : serial_execution_generation_;
    }
    uint64_t exclusive_epoch(uint64_t address) {
        PageAccess access(*this, address, read_perm, true);
        return access.entry().reservation->epoch(exclusive_revision_.load(std::memory_order_relaxed));
    }
    Memory(const Memory &) = delete;
    Memory &operator=(const Memory &) = delete;
    std::unique_ptr<Memory> fork_clone();
    void synchronize_shared(bool code_proofs=false);
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
        OPERIT_MEMORY_DIAGNOSTIC(tlb_refills, 1);
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
            if(execution_pages_)execution_pages_->fill({});
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
                             bool(storage->file),
                             uint8_t(mapping.backing.shared ? 0u :
                                 available & (read_perm | ((storage->file || (available & exec_perm)) ? 0u : write_perm)))};
    }
    // A software TLB collision is not a mapping change. With the reader gate
    // held, an existing resident page can be cached without stopping all CPUs.
    // Allocation and private-file/fork COW still require the writer path.
    bool refill_resident(Translation &cached, uint64_t address, unsigned permissions) {
        OPERIT_MEMORY_DIAGNOSTIC(tlb_resident_attempts, 1);
        auto region = regions_.upper_bound(address);
        if (region == regions_.begin()) {
            OPERIT_MEMORY_DIAGNOSTIC(tlb_resident_declines, 1); return false;
        }
        --region;
        const auto &mapping = region->second;
        if (address >= mapping.end || (mapping.permissions & permissions) != permissions) {
            OPERIT_MEMORY_DIAGNOSTIC(tlb_resident_declines, 1); return false;
        }
        const uint64_t page = align_down(address);
        const auto found = pages_.find(page);
        if (found == pages_.end()) {
            OPERIT_MEMORY_DIAGNOSTIC(tlb_resident_declines, 1); return false;
        }
        const auto &storage = found->second;
        if (!mapping.backing.shared && (permissions & write_perm) &&
            (storage.use_count() > 1 || storage->file)) {
            OPERIT_MEMORY_DIAGNOSTIC(tlb_resident_declines, 1); return false;
        }
        if (mapping.object && mapping.object->file)
            mapping.object->file->check_page(mapping.object_offset + page - region->first,
                                             address, permissions, &file_metadata_);
        cache_translation(cached, page, region->first, mapping, storage);
        OPERIT_MEMORY_DIAGNOSTIC(tlb_resident_hits, 1);
        return true;
    }
    [[gnu::noinline]] void validate_file(Translation &cached, uint64_t address, unsigned permissions) {
        OPERIT_MEMORY_DIAGNOSTIC(validate_file_calls, 1);
        cached.file->check_page(cached.file_offset, address, permissions, &file_metadata_);
        cached.file_generation = file_metadata_.generation();
    }

    [[gnu::always_inline]] uint8_t *translate_locked(uint64_t address, unsigned permissions) {
        auto &cached = translation(address);
        if (cached.tag != align_down(address) || cached.epoch != epoch_ ||
            (cached.permissions & permissions) != permissions) {
            OPERIT_MEMORY_DIAGNOSTIC(tlb_fallbacks, 1); refill(cached, address, permissions);
        }
        else if (cached.file) {
            const auto generation = file_metadata_.generation();
            if (!generation || cached.file_generation != generation)
                validate_file(cached, address, permissions);
            else OPERIT_MEMORY_DIAGNOSTIC(validate_file_hits, 1);
        }
        return cached.data + address % page_size;
    }
    class PageAccess {
        memory_detail::AccessSynchronization::ReadGuard read_;
        memory_detail::AccessSynchronization::PageGuard lock_;
        Translation *entry_ = nullptr;
        uint64_t address_;

        [[gnu::noinline]] void prepare(Memory &memory, Translation &entry, uint64_t address,
                                       unsigned permissions, bool parallel) {
            OPERIT_MEMORY_DIAGNOSTIC(tlb_fallbacks, 1);
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
                    memory.refill(entry, address, permissions);
                }
                read_.lock(parallel);
            }
        }

      public:
        [[gnu::always_inline]] PageAccess(Memory &memory, uint64_t address, unsigned permissions,
                                        bool exclusive = false)
            : address_(address) {
            OPERIT_MEMORY_DIAGNOSTIC(page_access_calls, 1);
            const bool parallel = memory.parallel_access_.load(std::memory_order_relaxed);
            read_.lock(parallel);
            auto &entry = memory.translation(address, parallel);
            if (entry.tag != align_down(address) || entry.epoch != memory.epoch_ ||
                (entry.permissions & permissions) != permissions)
                prepare(memory, entry, address, permissions, parallel);
            if (!parallel)
                read_.lock(entry.file_storage);
            if (entry.file) {
                const auto generation = file_metadata_.generation();
                if (!generation || entry.file_generation != generation)
                    memory.validate_file(entry, address, permissions);
                else OPERIT_MEMORY_DIAGNOSTIC(validate_file_hits, 1);
            }
            entry_ = &entry;
            // Immutable private pages need only the mapping reader gate.
            const bool immutable = !(entry.permissions & write_perm) && !entry.shared &&
                                   !entry.file_storage && !exclusive;
            // A required reader is inactive only when this thread already owns
            // the writer gate. Its state records the result of the earlier TLS check.
            if ((parallel || entry.file_storage) && !immutable && read_.active()) {
                OPERIT_MEMORY_DIAGNOSTIC(page_access_locks, 1);
                lock_.lock(entry.reservation->mutex, exclusive || (permissions & write_perm));
            }
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
    struct CodeSnapshot {
        std::shared_ptr<const CodeWords> words;
        uint64_t epoch = 0;
        std::shared_ptr<CodeProof> proof;
    };
    // A thread owns its cache, while regions retain immutable shared snapshots.
    // Refresh once per code epoch and execution scope; never retain a raw guest
    // pointer, or cache an access fault. PageAccess still checks file bounds,
    // execute permission, mappings, and synchronization before refreshing.
    CodeSnapshot code_snapshot(uint64_t address) {
        OPERIT_MEMORY_DIAGNOSTIC(snapshot_calls, 1);
        struct Entry {
            uint64_t epoch = 0, generation = 0;
            std::shared_ptr<const CodeWords> words;
            std::shared_ptr<CodeProof> proof;
        };
        struct Cache {
            uint64_t identity = 0;
            std::unordered_map<uint64_t,Entry> entries;
        };
        static thread_local Cache cache;
        if (cache.identity != identity_) {
            cache.entries.clear();
            cache.identity = identity_;
        }
        const auto base = align_down(address);
        auto found=cache.entries.find(base);
        if(found==cache.entries.end()) {
            // Bound retained snapshots even for guests cycling through new VAs.
            // Regions keep their own immutable owner across this cold reset.
            if(cache.entries.size()==4096) cache.entries.clear();
            found=cache.entries.try_emplace(base).first;
        }
        auto &entry=found->second;
        const auto before = code_epoch.load(std::memory_order_relaxed);
        const auto generation = execution_generation();
        if (generation && entry.words && entry.epoch == before && entry.generation == generation) {
            OPERIT_MEMORY_DIAGNOSTIC(snapshot_hits, 1);
            return {entry.words,before,entry.proof};
        }
        PageAccess access(*this,address,exec_perm);
        const auto *data = access.data()-address%page_size;
        const bool unchanged = entry.words && memory_detail::code_bytes_equal(
            data,entry.words->data(),page_size,memory_detail::CodeComparison::Snapshot);
        if (!unchanged) {
            OPERIT_MEMORY_DIAGNOSTIC(snapshot_copies, 1);
            auto words = std::make_shared<CodeWords>(page_size/sizeof(uint32_t));
            std::memcpy(words->data(),data,page_size);
            entry.words = std::move(words);
            entry.proof = std::make_shared<CodeProof>(identity_);
        }
        if (unchanged) OPERIT_MEMORY_DIAGNOSTIC(snapshot_unchanged, 1);
        const auto after = code_epoch.load(std::memory_order_relaxed);
        // A store during acquisition must not relabel old bytes as a new epoch.
        entry.epoch = before == after ? before : 0;
        entry.generation = generation;
        if (entry.proof && code_proof_generation()) {
            entry.proof->epoch = entry.epoch;
            entry.proof->verified = entry.epoch ? UINT64_MAX : 0;
            entry.proof->generation = entry.epoch ?
                (access.entry().file ? generation : UINT64_MAX) : 0;
            entry.proof->bind_native(access.entry(),*entry.words,entry.epoch,epoch_);
        }
        return {entry.words,entry.epoch,entry.proof};
    }
    // Validate the selected lines of one owned page under one PageAccess.
    // A graph can have many disjoint instruction spans on this page. Batching
    // them avoids repeating synchronization, file-size checks and TLS lookup.
    // A mismatch is inconclusive: callers must check their exact instruction
    // spans, since an uncovered byte in a selected line may have changed.
    [[gnu::noinline]] bool validate_code_lines(uint64_t address,
        const std::shared_ptr<const CodeWords> &words,uint64_t mask,uint64_t &epoch,CodeProof *proof) {
        OPERIT_MEMORY_DIAGNOSTIC(validate_lines_calls, 1);
        OPERIT_MEMORY_DIAGNOSTIC(validate_lines_requested, __builtin_popcountll(mask));
        const auto scope=code_proof_generation();
        if(!scope || !proof || proof->identity!=identity_) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_ineligible, 1); return false;
        }
        const auto before=code_epoch.load(std::memory_order_relaxed);
        if(proof->epoch==before && proof->current(scope,mask)) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_hits, 1);
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_reused, __builtin_popcountll(mask));
            epoch=before;return true;
        }
        const bool same_observation=proof->epoch==before &&
            (proof->generation==scope || proof->generation==UINT64_MAX);
        uint64_t verified=same_observation ? proof->verified:0;
        OPERIT_MEMORY_DIAGNOSTIC(validate_lines_reused, __builtin_popcountll(mask & verified));
        if(mask & verified) OPERIT_MEMORY_DIAGNOSTIC(validate_lines_partial_hits, 1);
        const auto pending=mask&~verified;
#if defined(OPERIT_EXECUTION_DIAGNOSTICS) && OPERIT_EXECUTION_DIAGNOSTICS
        if(proof->diagnostic_epoch==before && proof->diagnostic_scope==scope)
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_repeated_mismatches,
                                     __builtin_popcountll(pending & proof->diagnostic_mismatched));
#endif
        PageAccess access(*this,align_down(address),exec_perm);
        auto remaining=pending;
        while(remaining) {
            const auto first=unsigned(__builtin_ctzll(remaining));
            const auto shifted=remaining>>first;
            const auto count=shifted==UINT64_MAX ? 64u:unsigned(__builtin_ctzll(~shifted));
            const auto run=(UINT64_MAX>>(64-count))<<first;
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_compared, count);
            proof->diagnose_comparison(before,scope,run);
            if(memory_detail::code_bytes_equal(access.data()+first*64,
                reinterpret_cast<const uint8_t*>(words->data())+first*64,count*64,
                memory_detail::CodeComparison::Line)) verified|=run;
            else {
                OPERIT_MEMORY_DIAGNOSTIC(validate_lines_inconclusive, count);
                proof->diagnose_mismatch(run);
            }
            remaining&=~run;
        }
        epoch=code_epoch.load(std::memory_order_relaxed);
        if(epoch!=before) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_epoch_failures, 1); return false;
        }
        proof->epoch=before;
        proof->generation=access.entry().file ? scope:UINT64_MAX;
        proof->verified=verified;
        proof->bind_native(access.entry(),*words,before,epoch_);
        if((verified&mask)!=mask) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_lines_failures, 1); return false;
        }
        return true;
    }
    [[gnu::noinline]] bool validate_code_snapshot_cached(uint64_t address,
        const std::shared_ptr<const CodeWords> &words,size_t offset,size_t count,uint64_t &epoch,
        CodeProof *proof=nullptr) {
        OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_calls, 1);
        const auto scope=code_proof_generation();
        if (scope && proof && proof->identity==identity_) {
            const auto before=code_epoch.load(std::memory_order_relaxed);
            const auto mask=CodeProof::mask(offset,count);
            if (proof->epoch==before && proof->current(scope,mask)) {
                OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_hits, 1); epoch=before; return true;
            }
            const bool new_observation=proof->epoch!=before ||
                (proof->generation!=scope && proof->generation!=UINT64_MAX);
            if(new_observation)proof->verified=0;
            if(proof->verified & mask) OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_partial_hits, 1);
            PageAccess access(*this,align_down(address),exec_perm);
            const auto first=offset*4/64,last=(offset+count-1)*4/64;
            proof->diagnose_comparison(before,scope,mask);
            const bool match=memory_detail::code_bytes_equal(access.data()+first*64,
                reinterpret_cast<const uint8_t *>(words->data())+first*64,(last-first+1)*64,
                memory_detail::CodeComparison::Line);
            if(!match)proof->diagnose_mismatch(mask);
            if(!match && !memory_detail::code_bytes_equal(access.data()+offset*4,words->data()+offset,
                count*4,memory_detail::CodeComparison::Exact)) {
                OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_failures, 1); return false;
            }
            epoch=code_epoch.load(std::memory_order_relaxed);
            if(epoch!=before) {
                OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_epoch_failures, 1); return false;
            }
            proof->epoch=before;
            proof->generation=access.entry().file ? scope : UINT64_MAX;
            if(match)proof->verified|=mask;
            proof->bind_native(access.entry(),*words,before,epoch_);
            return true;
        }
        const bool parallel=parallel_access_.load(std::memory_order_relaxed);
        auto &cache=parallel ? thread_code_validations() :
            *(serial_code_validations_ ? serial_code_validations_.get() :
              (serial_code_validations_=std::make_unique<CodeValidations>()).get());
        if(cache.identity!=identity_) { cache.entries.fill({});cache.identity=identity_; }
        auto &entry=cache.entries[translation_index(address)&(cache.entries.size()-1)];
        const auto before=code_epoch.load(std::memory_order_relaxed);
        const auto generation=parallel ? execution_generation() : serial_execution_generation_;
        if(entry.address!=align_down(address) || entry.words!=words || entry.epoch!=before || entry.generation!=generation) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_cache_resets, 1);
            entry.address=align_down(address);
            if(entry.words!=words) entry.words=words;
            entry.epoch=before;entry.generation=generation;
            entry.verified=0;
        }
        const size_t first=offset*4/64,last=(offset+count-1)*4/64;
        const uint64_t mask=(UINT64_MAX<<first)&(UINT64_MAX>>(63-last));
        if(generation && (entry.verified&mask)==mask) {
            OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_hits, 1); epoch=before;return true;
        }
        if(entry.verified & mask) OPERIT_MEMORY_DIAGNOSTIC(validate_snapshot_partial_hits, 1);
        return validate_snapshot_slow(address,entry,offset,count,mask,epoch);
    }
    bool code_snapshot_validation_required() const noexcept {
        return parallel_access_.load(std::memory_order_relaxed) || file_mapping_;
    }
    bool validate_code(uint64_t address, const void *expected, size_t size, uint64_t &validated_epoch) {
        if (!size) {
            validated_epoch = code_epoch.load(std::memory_order_relaxed);
            return true;
        }
        if (valid_range(address, size) && size <= page_size - address % page_size) {
            PageAccess access(*this, address, exec_perm);
            if (!memory_detail::code_bytes_equal(access.data(), expected, size, memory_detail::CodeComparison::Exact))
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
            if (!memory_detail::code_bytes_equal(translate_locked(address, exec_perm), bytes, count,
                                               memory_detail::CodeComparison::Exact))
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
