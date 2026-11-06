#pragma once

namespace operit::arm64 {
inline void Memory::require_access(uint64_t address, uint64_t size, unsigned permissions) const {
    if (!size)
        return;
    if (!valid_range(address, size))
        throw MemoryFault(address < address_limit ? address : address_limit, permissions,
                          MemoryFault::Kind::Unmapped);
    auto region = regions_.upper_bound(address);
    if (region == regions_.begin())
        throw MemoryFault(address, permissions, MemoryFault::Kind::Unmapped);
    --region;
    const uint64_t end = address + size;
    while (address < end) {
        if (region == regions_.end() || region->first > address || region->second.end <= address)
            throw MemoryFault(address, permissions, MemoryFault::Kind::Unmapped);
        if ((region->second.permissions & permissions) != permissions)
            throw MemoryFault(address, permissions, MemoryFault::Kind::Protection);
        address = std::min(end, region->second.end);
        ++region;
    }
}

inline bool Memory::ptrace_read(uint64_t address, void *destination, size_t size) {
    return ptrace_transfer(address, destination, size, false);
}

inline bool Memory::ptrace_write(uint64_t address, const void *source, size_t size) {
    return ptrace_transfer(address, const_cast<void *>(source), size, true);
}

inline bool Memory::ptrace_transfer(uint64_t address, void *buffer, size_t size, bool writing) {
    if (!size)
        return true;
    if (!buffer || !valid_range(address, size))
        return false;
    try {
        // Serial execution is still required: serial guest page accesses can
        // bypass the reader gate. The writer gate also protects file aliases.
        const memory_detail::AccessSynchronization::WriteGuard access(true);
        require_access(address, size, 0);
        struct Chunk {
            Translation translation;
            size_t offset, size;
        };
        std::vector<Chunk> chunks;
        chunks.reserve((address % page_size + size - 1) / page_size + 1);
        uint64_t current = address;
        size_t remaining = size;
        while (remaining) {
            auto region = regions_.upper_bound(current);
            --region; // require_access validated the complete range above.
            auto &cached = translation(current);
            refill(cached, current, writing ? write_perm : read_perm, true);
            const size_t offset = current % page_size;
            const size_t count = std::min<uint64_t>(
                std::min<size_t>(remaining, page_size - offset), region->second.end - current);
            chunks.push_back({cached, offset, count});
            current += count;
            remaining -= count;
        }
        auto *bytes = static_cast<uint8_t *>(buffer);
        for (const auto &chunk : chunks) {
            auto *guest = chunk.translation.data + chunk.offset;
            if (writing) {
                written(chunk.translation);
                std::memcpy(guest, bytes, chunk.size);
            } else
                std::memcpy(bytes, guest, chunk.size);
            bytes += chunk.size;
        }
        return true;
    } catch (const Failure &) {
        return false;
    } catch (const GuestError &) {
        return false;
    } catch (const std::bad_alloc &) {
        return false;
    }
}

inline std::unique_ptr<Memory> Memory::fork_clone() {
    const AccessGuard access(*this);
    auto child = std::make_unique<Memory>(limit_);
    child->shared_revision_ = shared_revision_;
    child->observed_shared_revision_.store(observed_shared_revision_.load(std::memory_order_relaxed),
                                          std::memory_order_relaxed);
    for (const auto &entry : regions_) {
        if (!entry.second.dont_fork)
            child->regions_.emplace(entry.first, entry.second);
    }
    child->pages_.reserve(pages_.size());
    for (const auto &entry : pages_) {
        auto region = regions_.upper_bound(entry.first);
        --region;
        if (!region->second.dont_fork && !region->second.wipe_on_fork)
            child->pages_.emplace(entry.first, entry.second);
    }
    // No parent translation may retain write access to a newly shared private
    // page. The child's translation cache starts empty.
    invalidate();
    child->code_epoch.store(code_epoch.load(std::memory_order_relaxed), std::memory_order_relaxed);
    child->refresh_mapping_summary();
    return child;
}

inline void Memory::synchronize_shared(bool code_proofs) {
    const AccessGuard access(*this);
    const uint64_t revision = shared_revision_->load(std::memory_order_relaxed);
    const uint64_t observed = observed_shared_revision_.load(std::memory_order_relaxed);
    if (mapping_summary_epoch_ != epoch_)
        refresh_mapping_summary();
    if (revision != observed || file_mapping_) {
        exclusive_revision_.fetch_add(1, std::memory_order_relaxed);
        // Serial region links consult shared per-page proofs once per quantum.
        // Parallel CPUs retain the conservative epoch barrier.
        if (revision != observed || (executable_file_ && (!code_proofs || parallel_access_.load(std::memory_order_relaxed))))
            code_epoch.fetch_add(1, std::memory_order_relaxed);
        observed_shared_revision_.store(revision, std::memory_order_relaxed);
    }
}

inline std::optional<Memory::SharedIdentity> Memory::shared_identity(uint64_t address) const {
    const AccessGuard access(*this);
    auto region = regions_.upper_bound(address);
    if (region == regions_.begin())
        return {};
    --region;
    const auto &mapping = region->second;
    if (address >= mapping.end || !mapping.backing.shared || !mapping.object)
        return {};
    const uint64_t offset = mapping.object_offset + address - region->first;
    if (mapping.object->file)
        return SharedIdentity{mapping.object->file->device, mapping.object->file->inode, 0, offset};
    return SharedIdentity{0, 0, mapping.object->identity, offset};
}

inline uint64_t Memory::find_free(uint64_t start, uint64_t size) const {
    const AccessGuard access(*this);
    if (!size || size > address_limit - page_size)
        guest_error(12);
    start = start > address_limit - page_size ? page_size : std::max(page_size, align_up(start));
    auto search = [&](uint64_t begin, uint64_t end) -> std::optional<uint64_t> {
        uint64_t candidate = begin;
        auto region = regions_.upper_bound(candidate);
        if (region != regions_.begin())
            candidate = std::max(candidate, std::prev(region)->second.end);
        while (candidate < end && size <= end - candidate) {
            if (region == regions_.end() || region->first >= candidate + size)
                return candidate;
            candidate = std::max(candidate, region->second.end);
            ++region;
        }
        return {};
    };
    if (auto address = search(start, address_limit))
        return *address;
    if (start > page_size) {
        if (auto address = search(page_size, address_limit))
            return *address;
    }
    guest_error(12);
}

inline void Memory::coalesce() {
    for (auto it = regions_.begin(); it != regions_.end();) {
        auto next = std::next(it);
        if (next == regions_.end())
            break;
        auto &left = it->second;
        const auto &right = next->second;
        const uint64_t length = left.end - it->first;
        if (left.end == next->first && left.permissions == right.permissions &&
            left.maximum_permissions == right.maximum_permissions && left.advice == right.advice &&
            left.dont_fork == right.dont_fork && left.wipe_on_fork == right.wipe_on_fork &&
            left.object == right.object &&
            (!left.object || left.object_offset + length == right.object_offset) &&
            left.backing.path == right.backing.path && left.backing.device == right.backing.device &&
            left.backing.inode == right.backing.inode && left.backing.shared == right.backing.shared &&
            (!left.backing.inode || left.backing.offset + length == right.backing.offset)) {
            if (left.object)
                left.view = left.object->view(left.object_offset, right.end - it->first);
            left.end = right.end;
            regions_.erase(next);
        } else
            it = next;
    }
}

inline void Memory::install(uint64_t address, uint64_t size, Region region, bool replace) {
    if (!size || !valid_range(address, size) || address % page_size || size % page_size ||
        region.permissions > 7)
        guest_error(22);
    if (replace)
        unmap(address, size);
    if (!free(address, size))
        guest_error(12);
    const bool executable = region.permissions & exec_perm;
    region.end = address + size;
    if (region.object)
        region.view = region.object->view(region.object_offset, size);
    regions_.emplace(address, std::move(region));
    coalesce();
    invalidate(executable);
}

inline void Memory::map(uint64_t address, uint64_t size, unsigned permissions, bool replace, bool shared) {
    const AccessGuard access(*this);
    Region region;
    region.permissions = permissions;
    region.backing.shared = shared;
    if (shared)
        region.object = std::make_shared<MappingObject>();
    install(address, size, std::move(region), replace);
}

inline void Memory::map_file(uint64_t address, uint64_t size, unsigned permissions, int descriptor,
                             const Backing &backing, bool replace, bool permit_write,
                             std::shared_ptr<void> open_reference) {
    const AccessGuard access(*this);
    if (backing.offset % page_size)
        guest_error(22);
    if (backing.offset > uint64_t(INT64_MAX) || size > uint64_t(INT64_MAX) - backing.offset)
        guest_error(75);
    auto file = std::make_shared<memory_detail::FileBacking>(descriptor, permit_write,
                                                           std::move(open_reference));
    if (backing.shared && permissions & write_perm && !file->writable)
        guest_error(13);
    Region region;
    region.permissions = permissions;
    region.backing = backing;
    region.backing.device = file->device;
    region.backing.inode = file->inode;
    region.object = std::make_shared<MappingObject>();
    region.object->file = std::move(file);
    region.object->seal_token =
        files::memory_file_check_mmap(descriptor, backing.shared, permissions, permit_write);
    if (region.object->seal_token) {
        if (auto name = files::memory_file_link(descriptor))
            region.backing.path = std::move(*name);
    }
    region.object_offset = backing.offset;
    if (backing.shared &&
        (!region.object->file->writable ||
         (region.object->seal_token && !region.object->seal_token->may_write())))
        region.maximum_permissions &= ~write_perm;
    install(address, size, std::move(region), replace);
}

inline void Memory::unmap(uint64_t address, uint64_t size) {
    const AccessGuard access(*this);
    if (!size || !valid_range(address, size) || address % page_size || size % page_size)
        guest_error(22);
    split(address);
    split(address + size);
    bool executable = false;
    for (auto it = regions_.lower_bound(address); it != regions_.end() && it->first < address + size; ++it)
        executable |= bool(it->second.permissions & exec_perm);
    regions_.erase(regions_.lower_bound(address), regions_.lower_bound(address + size));
    for (auto it = pages_.begin(); it != pages_.end();) {
        if (it->first >= address && it->first < address + size)
            it = pages_.erase(it);
        else
            ++it;
    }
    invalidate(executable);
}

inline void Memory::protect(uint64_t address, uint64_t size, unsigned permissions) {
    const AccessGuard access(*this);
    if (!valid_range(address, size) || address % page_size || size % page_size || permissions > 7)
        guest_error(22);
    if (!accessible(address, size, 0))
        guest_error(12);
    auto first = regions_.upper_bound(address);
    if (first != regions_.begin())
        --first;
    for (auto it = first; it != regions_.end() && it->first < address + size; ++it) {
        if (it->second.object)
            files::memory_file_check_mprotect(it->second.object->seal_token, permissions);
        if (permissions & ~it->second.maximum_permissions)
            guest_error(13);
    }
    split(address);
    split(address + size);
    bool executable = permissions & exec_perm;
    for (auto it = regions_.lower_bound(address); it != regions_.end() && it->first < address + size; ++it) {
        executable |= bool(it->second.permissions & exec_perm);
        it->second.permissions = permissions;
    }
    coalesce();
    invalidate(executable);
}

inline void Memory::populate(uint64_t address, uint64_t size, unsigned permissions) {
    if (!accessible(address, size, permissions))
        guest_error(22);
    for (uint64_t page = address; page < address + size; page += page_size) {
        if (pages_.find(page) == pages_.end() && resident() + page_size > limit_)
            guest_error(12);
        try {
            auto *data = static_cast<volatile uint8_t *>(translate_locked(page, permissions));
            const uint8_t value = *data;
            if (permissions & write_perm) {
                *data = value;
                written(translation(page));
            }
        } catch (const Failure &) {
            guest_error(14);
        } catch (const std::bad_alloc &) {
            guest_error(12);
        }
    }
}

inline void Memory::sync(uint64_t address, uint64_t size, unsigned flags) {
    const AccessGuard access(*this);
    if (address % page_size || flags & ~7u || (flags & 1 && flags & 4))
        guest_error(22);
    if (!size)
        return;
    if (!valid_range(address, size) || size % page_size || !accessible(address, size, 0))
        guest_error(12);
    int host_flags = flags & 4 ? MS_SYNC : MS_ASYNC;
    if (flags & 2)
        host_flags |= MS_INVALIDATE;
    auto it = regions_.upper_bound(address);
    --it;
    while (it != regions_.end() && it->first < address + size) {
        const auto &region = it->second;
        if (region.backing.shared && region.object && region.object->file) {
            const uint64_t begin = std::max(address, it->first);
            const uint64_t end = std::min(address + size, region.end);
            region.object->file->sync(region.object_offset + begin - it->first, end - begin, host_flags);
        }
        ++it;
    }
}

inline uint64_t Memory::remap(uint64_t address, uint64_t old_size, uint64_t new_size, unsigned flags,
                              uint64_t new_address, uint64_t hint) {
    const AccessGuard access(*this);
    constexpr unsigned may_move = 1, fixed = 2, keep_old = 4;
    if (address % page_size || !new_size || old_size % page_size || new_size % page_size ||
        flags & ~(may_move | fixed | keep_old) || (flags & (fixed | keep_old) && !(flags & may_move)) ||
        (flags & keep_old && old_size != new_size))
        guest_error(22);
    if (!valid_range(address, std::max<uint64_t>(old_size, 1)) || new_size > address_limit - page_size)
        guest_error(22);
    auto source_it = regions_.upper_bound(address);
    if (source_it == regions_.begin())
        guest_error(14);
    --source_it;
    if (address >= source_it->second.end || old_size > source_it->second.end - address)
        guest_error(14);
    Region source = source_it->second;
    const uint64_t displacement = address - source_it->first;
    source.object_offset += displacement;
    if (source.backing.inode)
        source.backing.offset += displacement;
    if (!old_size && (!source.backing.shared || !(flags & may_move)))
        guest_error(22);
    if (flags & fixed) {
        if (new_address % page_size || !valid_range(new_address, new_size) ||
            (old_size && new_address < address + old_size && address < new_address + new_size))
            guest_error(22);
    } else if (old_size && !(flags & keep_old)) {
        if (new_size <= old_size) {
            if (new_size < old_size)
                unmap(address + new_size, old_size - new_size);
            return address;
        }
        if (valid_range(address, new_size) && free(address + old_size, new_size - old_size)) {
            Region tail = source;
            tail.object_offset += old_size;
            if (tail.backing.inode)
                tail.backing.offset += old_size;
            install(address + old_size, new_size - old_size, std::move(tail), false);
            return address;
        }
        if (!(flags & may_move))
            guest_error(12);
    }
    if (!(flags & fixed))
        new_address = find_free(hint, new_size);
    const uint64_t preserved =
        old_size ? std::min(old_size, new_size) : std::min(new_size, source_it->second.end - address);
    std::vector<std::pair<uint64_t, std::shared_ptr<Page>>> moved;
    for (const auto &entry : pages_) {
        if (entry.first >= address && entry.first - address < preserved)
            moved.emplace_back(new_address + entry.first - address, entry.second);
    }
    if (flags & fixed)
        unmap(new_address, new_size);
    if (old_size && !(flags & keep_old))
        unmap(address, old_size);
    else if (flags & keep_old) {
        for (auto it = pages_.begin(); it != pages_.end();) {
            if (it->first >= address && it->first - address < old_size)
                it = pages_.erase(it);
            else
                ++it;
        }
        invalidate();
    }
    install(new_address, new_size, std::move(source), false);
    for (auto &entry : moved)
        pages_.emplace(entry.first, std::move(entry.second));
    return new_address;
}

inline void Memory::advise(uint64_t address, uint64_t size, int advice) {
    const AccessGuard access(*this);
    if (address % page_size || !valid_range(address, size) || size % page_size)
        guest_error(22);
    if (!accessible(address, size, 0))
        guest_error(12);
    if (!size)
        return;
    switch (advice) {
    case 0:  // NORMAL
    case 1:  // RANDOM
    case 2:  // SEQUENTIAL
    case 3:  // WILLNEED
    case 4:  // DONTNEED
    case 8:  // FREE: eager reclaim is permitted by Linux's lazy-free contract.
    case 10: // DONTFORK
    case 11: // DOFORK
    case 18: // WIPEONFORK
    case 19: // KEEPONFORK
    case 22: // POPULATE_READ
    case 23: // POPULATE_WRITE
        break;
    default:
        guest_error(22);
    }
    auto first = regions_.upper_bound(address);
    --first;
    if (advice == 8 || advice == 18) {
        for (auto it = first; it != regions_.end() && it->first < address + size; ++it) {
            if (it->second.backing.shared || (it->second.object && it->second.object->file))
                guest_error(22);
        }
    }
    if (advice == 22 || advice == 23) {
        populate(address, size, advice == 23 ? write_perm : read_perm);
        return;
    }
    split(address);
    split(address + size);
    if (advice == 0 || advice == 1 || advice == 2) {
        const int host = advice == 0 ? MADV_NORMAL : advice == 1 ? MADV_RANDOM : MADV_SEQUENTIAL;
        for (auto it = regions_.lower_bound(address); it != regions_.end() && it->first < address + size;
             ++it)
            it->second.advice = host;
        coalesce();
        for (auto &entry : pages_) {
            if (entry.first >= address && entry.first < address + size && entry.second->file)
                entry.second->file->advise(host);
        }
        return;
    }
    if (advice == 3) {
        for (auto it = regions_.lower_bound(address); it != regions_.end() && it->first < address + size;
             ++it) {
            const auto &region = it->second;
            if (region.object && region.object->file)
                region.object->file->advise(region.object_offset, region.end - it->first, MADV_WILLNEED);
        }
        for (const auto &entry : pages_) {
            if (entry.first < address || entry.first >= address + size)
                continue;
            if (entry.second->file)
                entry.second->file->advise(MADV_WILLNEED);
            else {
                const uintptr_t data = reinterpret_cast<uintptr_t>(entry.second->data());
                const uintptr_t aligned =
                    data / memory_detail::host_page_size() * memory_detail::host_page_size();
                if (::madvise(reinterpret_cast<void *>(aligned), data - aligned + page_size, MADV_WILLNEED) <
                    0)
                    guest_error(linux_errno(errno));
            }
        }
        return;
    }
    if (advice == 4 || advice == 8) {
        for (auto it = pages_.begin(); it != pages_.end();) {
            if (it->first >= address && it->first < address + size) {
                if (it->second->file)
                    it->second->file->advise(MADV_DONTNEED);
                it = pages_.erase(it);
            } else
                ++it;
        }
        invalidate();
        return;
    }
    for (auto it = regions_.lower_bound(address); it != regions_.end() && it->first < address + size; ++it) {
        auto &region = it->second;
        if (advice == 10)
            region.dont_fork = true;
        else if (advice == 11)
            region.dont_fork = false;
        else if (advice == 18)
            region.wipe_on_fork = true;
        else if (advice == 19)
            region.wipe_on_fork = false;
    }
    coalesce();
}

inline std::vector<uint8_t> Memory::residency(uint64_t address, uint64_t size) {
    const AccessGuard access(*this);
    if (address % page_size || !valid_range(address, size) || size % page_size)
        guest_error(22);
    if (!accessible(address, size, 0))
        guest_error(12);
    std::vector<uint8_t> result(size / page_size);
    for (uint64_t page = address; page < address + size; page += page_size) {
        auto region = regions_.upper_bound(page);
        --region;
        const auto &mapping = region->second;
        const uint64_t offset = mapping.object_offset + page - region->first;
        if (mapping.object && mapping.object->file) {
            const uint64_t bytes = mapping.object->file->size();
            if (offset >= ((bytes + page_size - 1) & ~(page_size - 1)))
                continue;
        }
        auto found = pages_.find(page);
        if (found != pages_.end())
            result[(page - address) / page_size] = found->second->file ? found->second->file->resident() : 1;
        else if (mapping.object) {
            if (mapping.object->file) {
                auto storage =
                    mapping.object->page(offset, mapping.backing.shared && mapping.object->file->writable);
                result[(page - address) / page_size] = storage->file->resident();
            } else if (mapping.object->has_anonymous_page(offset))
                result[(page - address) / page_size] = 1;
        }
    }
    return result;
}
} // namespace operit::arm64
