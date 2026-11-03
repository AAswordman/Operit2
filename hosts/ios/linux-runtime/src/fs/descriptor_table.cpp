#include "fs/descriptor_table.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include "linux/events/epoll.h"
#include "linux/events/inotify.h"
#include "linux/files/record_lock.h"
#include "linux/files/memory_file.h"
#include <algorithm>
#include <atomic>
#include <climits>
#include <fcntl.h>

namespace operit::arm64 {
namespace {
std::atomic<uint64_t> next_inode{1};
}
Descriptor::Descriptor(FD fd, std::string p, int f)
    : host(std::move(fd)), path(std::move(p)), descriptor_flags((f & 524288) ? 1 : 0),
      description(std::make_shared<OpenDescription>(f)) {
    files::memory_file_attach(*this);
}
Descriptor::Descriptor(FD fd, const Descriptor &source, bool cloexec)
    : host(std::move(fd)), path(source.path), descriptor_flags(cloexec ? 1 : 0),
      description(source.description) {}
std::string Descriptor::display_path() const {
    if (description->inotify)
        return "anon_inode:inotify";
    if (auto anonymous = files::memory_file_link(host.get()))
        return *anonymous;
    return path;
}
bool OpenDescription::is_anon_inode() const {
    return bool(poll || counter || timer || inotify || io_uring);
}
bool OpenDescription::is_read_only() const {
    return read_only || (mount && mount->read_only());
}
void OpenDescription::require_writable() const {
    if (is_read_only())
        guest_error(30);
}
OpenDescription::~OpenDescription() {
    EventPoll::description_closed(this);
    files::record_lock_description_closed(this);
    files::memory_file_description_closed(this);
    if (directory)
        ::closedir(directory);
}
DescriptorTable::DescriptorTable(const DescriptorTable &source) : limits_(source.limits_) {
    for (const auto &item : source.state_->entries) {
        const auto &entry = *item.second;
        FD host(entry.host.get() < 0 ? -1 : ::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
        if (entry.host.get() >= 0)
            check_host(host.get());
        state_->entries.emplace(item.first, std::make_unique<Descriptor>(std::move(host), entry,
                                                                entry.descriptor_flags & 1));
    }
}
DescriptorTable &DescriptorTable::operator=(const DescriptorTable &source) {
    if (this != &source) {
        DescriptorTable copy(source);
        *this = std::move(copy);
    }
    return *this;
}
void DescriptorTable::share_with(const DescriptorTable &source) {
    state_ = source.state_;
}
void DescriptorTable::bind_resource_limits(std::shared_ptr<process::ResourceLimits> limits) {
    if (!limits)
        throw Failure("missing descriptor resource limits");
    limits_ = std::move(limits);
}
void DescriptorTable::unshare() {
    if (state_.use_count() > 1) {
        DescriptorTable copy(*this);
        *this = std::move(copy);
    }
}
void DescriptorTable::close_on_exec(const std::function<void(const Descriptor &)> &closing) {
    for (auto it = state_->entries.begin(); it != state_->entries.end();) {
        if (it->second->descriptor_flags & 1) {
            closing(*it->second);
            it = state_->entries.erase(it);
        } else
            ++it;
    }
}
void DescriptorTable::clear() {
    state_->entries.clear();
}
Descriptor &DescriptorTable::get(uint64_t fd) {
    if (fd > INT_MAX)
        guest_error(9);
    auto found = state_->entries.find(static_cast<int>(fd));
    if (found == state_->entries.end())
        guest_error(9);
    return *found->second;
}
int DescriptorTable::available(int minimum) const {
    const uint64_t limit = limits_->get(process::Resource::OpenFiles).soft;
    int candidate = std::max(0, minimum);
    for (auto it = state_->entries.lower_bound(candidate);
         it != state_->entries.end() && uint64_t(it->first) < limit; ++it) {
        if (it->first != candidate)
            break;
        ++candidate;
    }
    if (uint64_t(candidate) >= limit)
        guest_error(24);
    return candidate;
}
int DescriptorTable::insert(FD fd, const std::string &path, int flags, int minimum) {
    int candidate = available(minimum);
    auto entry = std::make_unique<Descriptor>(std::move(fd), path, flags);
    entry->description->inode = next_inode.fetch_add(1, std::memory_order_relaxed);
    state_->entries.emplace(candidate, std::move(entry));
    return candidate;
}
int DescriptorTable::duplicate(uint64_t source, int minimum, bool cloexec) {
    auto &entry = get(source);
    if (minimum < 0)
        guest_error(22);
    int candidate = available(minimum);
    FD fd(entry.host.get() < 0 ? -1 : ::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    if (entry.host.get() >= 0)
        check_host(fd.get());
    state_->entries.emplace(candidate, std::make_unique<Descriptor>(std::move(fd), entry, cloexec));
    return candidate;
}
int DescriptorTable::duplicate_minimum(uint64_t source, uint64_t minimum, bool cloexec) {
    get(source);
    if (minimum > INT_MAX || minimum >= limits_->get(process::Resource::OpenFiles).soft)
        guest_error(22);
    return duplicate(source, int(minimum), cloexec);
}
int DescriptorTable::duplicate_to(uint64_t source, uint64_t target, bool cloexec,
                                  const std::function<void(const Descriptor &)> &before_close) {
    if (source == target)
        guest_error(22); // dup3, including an invalid or now-out-of-limit fd.
    auto &entry = get(source);
    if (target > INT_MAX || target >= limits_->get(process::Resource::OpenFiles).soft)
        guest_error(9);
    FD fd(entry.host.get() < 0 ? -1 : ::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
    if (entry.host.get() >= 0)
        check_host(fd.get());
    auto copy = std::make_unique<Descriptor>(std::move(fd), entry, cloexec);
    auto existing = state_->entries.find(int(target));
    if (existing != state_->entries.end())
        before_close(*existing->second);
    state_->entries.insert_or_assign(int(target), std::move(copy));
    return int(target);
}
std::vector<int> DescriptorTable::numbers() const {
    std::vector<int> result;
    for (const auto &entry : state_->entries)
        result.push_back(entry.first);
    return result;
}
void DescriptorTable::close(uint64_t fd) {
    get(fd);
    state_->entries.erase(static_cast<int>(fd));
}
void DescriptorTable::standard(int fd, FD host) {
    if (fd < 0 || uint64_t(fd) >= limits_->get(process::Resource::OpenFiles).soft)
        guest_error(9);
    auto entry = std::make_unique<Descriptor>(std::move(host), "", fd == 0 ? 0 : 1);
    entry->description->standard_stream = fd;
    entry->description->inode = next_inode.fetch_add(1, std::memory_order_relaxed);
    state_->entries.insert_or_assign(fd, std::move(entry));
}
} // namespace operit::arm64
