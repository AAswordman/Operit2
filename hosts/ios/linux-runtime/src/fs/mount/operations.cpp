#include "base/support.h"
#include "fs/filesystem.h"
#include "fs/mount/storage.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <fcntl.h>
#include <set>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
FD duplicate(int fd) {
    FD result(::fcntl(fd, F_DUPFD_CLOEXEC, 0));
    check_host(result.get());
    return result;
}
bool below(const std::string &path, const std::string &directory) {
    return directory == "/"
               ? path != "/"
               : path.size() > directory.size() && path.compare(0, directory.size(), directory) == 0 &&
                     path[directory.size()] == '/';
}
std::string relative(const std::string &path, const std::string &directory) {
    return directory == "/" ? path : path.substr(directory.size());
}
std::string graft(const std::string &target, const std::string &suffix) {
    return target == "/" ? suffix : target + suffix;
}
uint64_t option_number(const std::string &text, unsigned base) {
    if (text.empty())
        guest_error(22);
    uint64_t result = 0;
    for (unsigned char c : text) {
        if (c < '0' || c >= '0' + base || result > (UINT64_MAX - (c - '0')) / base)
            guest_error(22);
        result = result * base + c - '0';
    }
    return result;
}
mode_t temporary_mode(const std::string &data) {
    mode_t mode = 01777;
    size_t begin = 0;
    while (begin < data.size()) {
        size_t end = data.find(',', begin);
        if (end == std::string::npos)
            end = data.size();
        const std::string option = data.substr(begin, end - begin);
        size_t equal = option.find('=');
        if (equal == std::string::npos)
            guest_error(22);
        const auto key = option.substr(0, equal), value = option.substr(equal + 1);
        if (key == "mode") {
            uint64_t parsed = option_number(value, 8);
            if (parsed > 07777)
                guest_error(22);
            mode = mode_t(parsed);
        } else if (key == "uid" || key == "gid") {
            const uint64_t owner = key == "uid" ? ::getuid() : ::getgid();
            if (option_number(value, 10) != owner)
                guest_error(1);
        } else
            guest_error(95);
        begin = end + 1;
    }
    return mode;
}
std::string escaped_mount_name(const std::string &value) {
    std::string result;
    for (unsigned char c : value) {
        if (c == ' ' || c == '\t' || c == '\n' || c == '\\') {
            char escaped[5];
            std::snprintf(escaped, sizeof(escaped), "\\%03o", c);
            result += escaped;
        } else
            result += c;
    }
    return result;
}
std::string mount_options(uint64_t flags, bool locked = false) {
    std::string result = (flags & mounts::read_only) || locked ? "ro" : "rw";
    if (flags & mounts::no_suid)
        result += ",nosuid";
    if (flags & mounts::no_devices)
        result += ",nodev";
    if (flags & mounts::no_execute)
        result += ",noexec";
    return result;
}
} // namespace
void Filesystem::mount(const std::string &source, const std::string &target, const std::string &type,
                       uint64_t flags, const std::string &data, const std::string &base,
                       VirtualNode::Ptr root) {
    constexpr uint64_t attributes =
        mounts::read_only | mounts::no_suid | mounts::no_devices | mounts::no_execute;
    constexpr uint64_t operations = mounts::bind | mounts::recursive | mounts::remount | mounts::silent;
    if (flags & ~(attributes | operations))
        guest_error(95);
    if ((flags & mounts::recursive) && !(flags & mounts::bind))
        guest_error(22);
    auto destination = resolve(target, base);
    const auto destination_info = destination.stat();
    if (flags & mounts::remount) {
        if (!destination.mount_root && destination.guest != "/")
            guest_error(22);
        if (!(flags & mounts::bind) && !type.empty() &&
            type != (destination.mount ? destination.mount->type : "hostfs"))
            guest_error(22);
        if (!data.empty())
            guest_error(95);
        if (!(flags & mounts::read_only) && destination.mount->source->locked_read_only)
            guest_error(30);
        std::set<uint64_t> selected{destination.mount_id};
        if (flags & mounts::recursive)
            for (const auto &point : namespace_->points)
                if (selected.count(point->parent)) {
                    if (!(flags & mounts::read_only) && point->source->locked_read_only)
                        guest_error(30);
                    selected.insert(point->id);
                }
        for (auto &point : namespace_->points)
            if (selected.count(point->id))
                point->flags = flags & attributes;
        return;
    }
    if (flags & mounts::bind) {
        const auto origin = resolve(source, base);
        const auto info = origin.stat();
        if (bool(S_ISDIR(info.st_mode)) != bool(S_ISDIR(destination_info.st_mode)))
            guest_error(S_ISDIR(info.st_mode) ? 20 : 21);
        auto backing = mount_source(origin);
        const uint64_t inherited = origin.mount ? origin.mount->flags : namespace_->root->flags;
        std::vector<std::shared_ptr<mounts::Point>> descendants;
        if ((flags & mounts::recursive) && backing->directory) {
            for (const auto &point : namespace_->points) {
                if (!below(point->target, origin.guest))
                    continue;
                try {
                    if (resolve(point->target).mount_id == point->id)
                        descendants.push_back(point);
                } catch (const GuestError &error) {
                    if (error.number != 2 && error.number != 20)
                        throw;
                }
            }
            std::sort(descendants.begin(), descendants.end(),
                      [](const auto &a, const auto &b) { return a->target.size() < b->target.size(); });
        }
        uint64_t next_id = namespace_->next_id;
        std::vector<std::shared_ptr<mounts::Point>> staged;
        auto bound = std::make_shared<mounts::Point>();
        bound->id = next_id++;
        bound->parent = destination.mount_id;
        bound->flags = (inherited | flags) & attributes;
        bound->target = destination.guest;
        bound->type = origin.mount ? origin.mount->type : "hostfs";
        bound->label = origin.guest;
        bound->source = std::move(backing);
        staged.push_back(bound);
        for (const auto &point : descendants) {
            auto child = std::make_shared<mounts::Point>(*point);
            child->id = next_id++;
            child->parent = bound->id;
            child->target = graft(destination.guest, relative(point->target, origin.guest));
            for (const auto &ancestor : staged)
                if (below(child->target, ancestor->target))
                    child->parent = ancestor->id;
            staged.push_back(std::move(child));
        }
        namespace_->points.reserve(namespace_->points.size() + staged.size());
        namespace_->points.insert(namespace_->points.end(), staged.begin(), staged.end());
        namespace_->next_id = next_id;
        return;
    }
    if (!S_ISDIR(destination_info.st_mode))
        guest_error(20);
    auto backing = std::make_shared<mounts::Source>();
    std::string filesystem_type = type;
    if (type == "tmpfs") {
        backing->storage = mounts::TemporaryTree::create(temporary_mode(data));
        backing->root = duplicate(backing->storage->descriptor());
    } else if (type == "proc" || type == "devfs" || type == "devtmpfs" || type == "devpts") {
        if (!data.empty())
            guest_error(95);
        if (!root || root->kind != VirtualNode::Kind::Directory)
            guest_error(19);
        backing->node = std::move(root);
        backing->context = type == "proc" ? "/proc" : type == "devpts" ? "/dev/pts" : "/dev";
        // Canonical contexts are initialized per process by mount_virtual. A
        // guest mount adds only a namespace attachment, never another process's
        // proc/dev callbacks.
        if (!virtual_contexts_.count(backing->context))
            virtual_contexts_[backing->context] = backing->node;
    } else
        guest_error(19);
    add_mount(std::move(backing), destination.guest, destination.mount_id, filesystem_type,
              source.empty() ? type : source, flags & attributes);
}
void Filesystem::unmount(const std::string &target, uint32_t flags, const std::string &base) {
    if (flags & ~(mounts::detach | mounts::no_follow))
        guest_error(95);
    const auto destination = resolve(target, base, !(flags & mounts::no_follow));
    if (!destination.mount_root || !destination.mount || !destination.mount_id)
        guest_error(22);
    std::set<uint64_t> removed{destination.mount_id};
    for (const auto &point : namespace_->points)
        if (removed.count(point->parent))
            removed.insert(point->id);
    if (!(flags & mounts::detach)) {
        if (removed.size() != 1 || destination.mount.use_count() > 2)
            guest_error(16);
    }
    auto &points = namespace_->points;
    points.erase(std::remove_if(points.begin(), points.end(),
                                [&](const auto &point) { return removed.count(point->id) != 0; }),
                 points.end());
}
void Filesystem::unshare_mounts() {
    auto copy = std::make_shared<mounts::Namespace>();
    copy->next_id = namespace_->next_id;
    copy->root = std::make_shared<mounts::Point>(*namespace_->root);
    copy->builtins = namespace_->builtins;
    copy->points.reserve(namespace_->points.size());
    for (const auto &point : namespace_->points)
        copy->points.push_back(std::make_shared<mounts::Point>(*point));
    namespace_ = std::move(copy);
}
std::string Filesystem::mount_table() const {
    std::string result = "rootfs / hostfs " + mount_options(namespace_->root->flags) + " 0 0\n";
    for (const auto &point : namespace_->points)
        result += escaped_mount_name(point->label) + " " + escaped_mount_name(point->target) + " " +
                  point->type + " " + mount_options(point->flags, point->source->locked_read_only) + " 0 0\n";
    return result;
}
} // namespace operit::arm64
