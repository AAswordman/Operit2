#include "fs/filesystem.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <deque>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
struct DirectoryAnchor {
    FD fd;
    std::weak_ptr<mounts::Point> mount;
    std::shared_ptr<mounts::Source> source;
    uint64_t mount_id = 0;
    VirtualNode::Ptr node;
    std::vector<std::string> virtual_components;
    std::shared_ptr<const DirectoryAnchor> parent;
    std::string guest, component;
    bool entered_mount = false;
    void set_mount(const std::shared_ptr<mounts::Point> &point) {
        mount = point;
        source = point ? point->source : nullptr;
        mount_id = point ? point->id : 0;
    }
    std::shared_ptr<mounts::Point> point() const {
        auto result = mount.lock();
        if (!result)
            guest_error(11);
        return result;
    }
    uint64_t id() const {
        return mount_id;
    }
};
// A mount owns its attachment from creation until detach/destruction. Keeping
// this on the mount removes the need to cache lookups or repair anchors later.
struct MountAttachment {
    std::shared_ptr<const DirectoryAnchor> parent;
    std::string component;
};
namespace {
FD duplicate(int fd) {
    FD result(::fcntl(fd, F_DUPFD_CLOEXEC, 0));
    check_host(result.get());
    return result;
}
void prepend(std::deque<std::string> &queue, const std::string &path) {
    std::vector<std::string> parts;
    for (size_t begin = 0; begin < path.size();) {
        size_t end = path.find('/', begin);
        if (end == std::string::npos)
            end = path.size();
        if (end > begin)
            parts.emplace_back(path.substr(begin, end - begin));
        begin = end + 1;
    }
    for (auto it = parts.rbegin(); it != parts.rend(); ++it)
        queue.push_front(*it);
}
std::shared_ptr<mounts::Point> child_mount(const mounts::Namespace &space, uint64_t parent,
                                           const std::string &target) {
    for (auto it = space.points.rbegin(); it != space.points.rend(); ++it)
        if ((*it)->parent == parent && (*it)->target == target)
            return *it;
    return {};
}
using Anchor = std::shared_ptr<const DirectoryAnchor>;
int directory_flags() {
#if defined(O_PATH)
    return O_PATH | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC;
#elif defined(O_SEARCH)
    return O_SEARCH | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC;
#else
    return O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC;
#endif
}
bool same_inode(const struct stat &first, const struct stat &second) {
    return first.st_dev == second.st_dev && first.st_ino == second.st_ino;
}
struct stat descriptor_stat(int descriptor) {
    struct stat info{};
    check_host(::fstat(descriptor, &info));
    return info;
}
bool same_directory(int first, int second) {
    return same_inode(descriptor_stat(first), descriptor_stat(second));
}
bool same_virtual_node(const VirtualNode::Ptr &first, const VirtualNode::Ptr &second) {
    return first && second && first->kind == second->kind && first->identity == second->identity;
}
std::vector<std::string> components(const std::string &path) {
    std::deque<std::string> queue;
    prepend(queue, path);
    return {queue.begin(), queue.end()};
}
std::string parent_name(const std::string &path) {
    const auto slash = path.rfind('/');
    return slash == std::string::npos || slash == 0 ? "/" : path.substr(0, slash);
}
std::string child_name(const std::string &parent, const std::string &name) {
    return (parent == "/" ? "/" : parent + "/") + name;
}
Anchor pin_mount_parent(const Anchor &directory, const std::shared_ptr<mounts::Point> &point,
                        const std::string &guest) {
    if (!directory)
        return {};
    auto pinned = std::make_shared<DirectoryAnchor>();
    pinned->set_mount(point);
    pinned->guest = guest;
    pinned->node = directory->node;
    pinned->virtual_components = directory->virtual_components;
    if (directory->fd.get() >= 0)
        pinned->fd = duplicate(directory->fd.get());
    return pinned;
}
bool numeric_component(const std::string &value) {
    return !value.empty() && std::all_of(value.begin(), value.end(), [](unsigned char c) {
        return c >= '0' && c <= '9';
    });
}
bool magic_link(const Anchor &directory, const VirtualNode::Ptr &node) {
    if (node->open_stream)
        return true;
    const auto point = directory->point();
    if (!point->source || (point->source->context != "/proc" && point->type != "proc"))
        return false;
    const auto names = components(node->identity);
    if (names.size() < 3 || names[0] != "proc" || !numeric_component(names[1]))
        return false;
    size_t leaf = 2;
    if (names[leaf] == "task") {
        if (names.size() < 5 || !numeric_component(names[3]))
            return false;
        leaf = 4;
    }
    if (names.size() == leaf + 1)
        return names[leaf] == "cwd" || names[leaf] == "root" || names[leaf] == "exe";
    return names.size() == leaf + 2 && names[leaf] == "fd" && numeric_component(names[leaf + 1]);
}
// Confirm every actual directory edge rather than comparing guest path prefixes.
// A renamed component makes a scoped walk retry, even when a saved FD survives.
void verify_scope(Anchor directory, const Anchor &root) {
    if (!root)
        return;
    while (directory != root) {
        if (!directory || !directory->parent)
            guest_error(11);
        const auto &parent = directory->parent;
        if (!directory->entered_mount) {
            if (directory->node) {
                if (!parent->node || !parent->node->lookup ||
                    !same_virtual_node(parent->node->lookup(directory->component), directory->node))
                    guest_error(11);
            } else {
                struct stat entry{};
                if (parent->fd.get() < 0 ||
                    ::fstatat(parent->fd.get(), directory->component.c_str(), &entry,
                              AT_SYMLINK_NOFOLLOW) < 0 ||
                    !same_inode(entry, descriptor_stat(directory->fd.get())))
                    guest_error(11);
            }
        }
        directory = parent;
    }
}
Anchor as_directory(const Path &path) {
    if (path.anchor && path.name == ".") {
        if (path.virtual_path ? same_virtual_node(path.anchor->node, path.node)
                              : path.anchor->fd.get() >= 0 &&
                                    same_directory(path.anchor->fd.get(), path.parent.get())) {
            verify_scope(path.anchor, path.scope_root);
            return path.anchor;
        }
    }
    auto directory = std::make_shared<DirectoryAnchor>();
    directory->set_mount(path.mount);
    directory->guest = path.guest;
    directory->virtual_components = path.virtual_components;
    directory->parent = path.anchor;
    directory->component = path.name;
    directory->entered_mount = path.mount_root;
    if (path.virtual_path) {
        if (!path.node)
            guest_error(2);
        if (path.node->kind != VirtualNode::Kind::Directory)
            guest_error(20);
        directory->node = path.node;
    } else {
        const auto before = path.stat();
        if (!S_ISDIR(before.st_mode))
            guest_error(20);
        directory->fd = path.open(directory_flags());
        if (!same_inode(before, descriptor_stat(directory->fd.get())))
            guest_error(11);
    }
    return directory;
}
// Used only when a caller supplied a directory FD without recorded ancestry.
// Host '..' traversal stops at this mount's pinned source inode, including binds.
bool within_native_mount(int descriptor, int root) {
    FD cursor = duplicate(descriptor);
    const auto boundary = descriptor_stat(root);
    for (size_t depth = 0; depth < max_path; ++depth) {
        const auto here = descriptor_stat(cursor.get());
        if (same_inode(here, boundary))
            return true;
        FD parent(::openat(cursor.get(), "..", directory_flags()));
        check_host(parent.get());
        if (same_inode(here, descriptor_stat(parent.get())))
            return false;
        cursor = std::move(parent);
    }
    guest_error(11);
}
} // namespace
struct stat Path::stat() const {
    verify_scope(anchor, scope_root);
    if (virtual_path) {
        if (!node)
            guest_error(2);
        return node->stat();
    }
    struct stat info{};
    check_host(::fstatat(parent.get(), name.c_str(), &info, AT_SYMLINK_NOFOLLOW));
    return info;
}
std::string Path::readlink() const {
    verify_scope(anchor, scope_root);
    if (virtual_path) {
        if (!node)
            guest_error(2);
        if (node->kind != VirtualNode::Kind::Symlink || !node->target)
            guest_error(22);
        return node->target();
    }
    char buffer[max_path];
    ssize_t count = ::readlinkat(parent.get(), name.c_str(), buffer, sizeof(buffer));
    check_host(count);
    if (size_t(count) == sizeof(buffer))
        guest_error(36);
    return std::string(buffer, count);
}
FD Path::open(int flags, mode_t mode) const {
    verify_scope(anchor, scope_root);
    if (virtual_path)
        guest_error(node ? (node->kind == VirtualNode::Kind::Directory ? 21 : 13) : 2);
    if ((flags & O_ACCMODE) != O_RDONLY || (flags & (O_CREAT | O_TRUNC)))
        require_writable();
    struct stat info{};
    if (!permits_devices() && ::fstatat(parent.get(), name.c_str(), &info, AT_SYMLINK_NOFOLLOW) == 0 &&
        (S_ISCHR(info.st_mode) || S_ISBLK(info.st_mode)))
        guest_error(13);
    FD result(::openat(parent.get(), name.c_str(), flags | O_NOFOLLOW | O_CLOEXEC, mode));
    check_host(result.get());
    return result;
}
bool Path::is_read_only() const {
    return mount ? mount->read_only() : read_only;
}
bool Path::permits_devices() const {
    return !((mount ? mount->flags : mount_flags) & mounts::no_devices);
}
void Path::require_executable() const {
    if ((mount ? mount->flags : mount_flags) & mounts::no_execute)
        guest_error(13);
}
void Path::require_writable() const {
    if (is_read_only() || virtual_path)
        guest_error(30);
}
void Path::require_removable() const {
    if (mount_root || guest == "/")
        guest_error(16);
    require_writable();
}
Filesystem::Filesystem(const char *root) {
    FD directory(::open(root, O_RDONLY | O_DIRECTORY | O_CLOEXEC));
    if (directory.get() < 0)
        throw Failure("cannot open guest root directory");
    namespace_->root = std::make_shared<mounts::Point>();
    namespace_->root->target = "/";
    namespace_->root->type = "hostfs";
    namespace_->root->label = "rootfs";
    namespace_->root->source = std::make_shared<mounts::Source>();
    namespace_->root->source->root = std::move(directory);
}
VirtualNode::Ptr Filesystem::virtual_root(const mounts::Source &source) const {
    auto found = virtual_contexts_.find(source.context);
    VirtualNode::Ptr root = found == virtual_contexts_.end() ? source.node : found->second;
    if (source.context == "/dev/pts" && found == virtual_contexts_.end()) {
        auto dev = virtual_contexts_.find("/dev");
        if (dev != virtual_contexts_.end() && dev->second->lookup)
            root = dev->second->lookup("pts");
    }
    for (const auto &component : source.components) {
        if (!root || root->kind != VirtualNode::Kind::Directory || !root->lookup)
            guest_error(2);
        root = root->lookup(component);
    }
    if (!root)
        guest_error(2);
    return root;
}
void Filesystem::attach_mount(mounts::Point &point, const Path &path) const {
    if (path.mount_root) {
        point.attachment = path.mount->attachment ? path.mount->attachment
                                                  : std::make_shared<MountAttachment>();
        return;
    }
    auto attachment = std::make_shared<MountAttachment>();
    auto parent = path.name == "." ? as_directory(resolve_at("..", path)) : path.anchor;
    attachment->component = point.target.substr(point.target.rfind('/') + 1);
    attachment->parent = pin_mount_parent(parent, parent->point(), parent_name(point.target));
    point.attachment = std::move(attachment);
}
void Filesystem::rebase_mount_attachment(mounts::Point &point,
                                        const std::shared_ptr<mounts::Point> &parent) const {
    auto attachment = std::make_shared<MountAttachment>(*point.attachment);
    attachment->parent = pin_mount_parent(attachment->parent, parent, parent_name(point.target));
    point.attachment = std::move(attachment);
}
std::shared_ptr<mounts::Point> Filesystem::anchored_child_mount(
    const std::shared_ptr<const DirectoryAnchor> &parent, const std::string &component) const {
    for (auto it = namespace_->points.rbegin(); it != namespace_->points.rend(); ++it) {
        const auto &point = *it;
        if (point->parent != parent->id())
            continue;
        const auto &where = *point->attachment;
        if (!where.parent || where.component != component || where.parent->id() != parent->id())
            continue;
        const bool same = parent->node ? same_virtual_node(parent->node, where.parent->node)
            : !where.parent->node && same_directory(parent->fd.get(), where.parent->fd.get());
        if (same)
            return point;
    }
    return {};
}
std::shared_ptr<mounts::Point> Filesystem::anchored_cover_mount(
    const std::shared_ptr<mounts::Point> &point) const {
    const auto &covered = *point->attachment;
    for (auto it = namespace_->points.rbegin(); it != namespace_->points.rend(); ++it) {
        const auto &candidate = *it;
        if (candidate->parent != point->id)
            continue;
        const auto &overlay = *candidate->attachment;
        if (covered.component != overlay.component || !covered.parent || !overlay.parent ||
            covered.parent->id() != overlay.parent->id())
            continue;
        const bool same = covered.parent->node ? same_virtual_node(covered.parent->node, overlay.parent->node)
            : !overlay.parent->node && same_directory(covered.parent->fd.get(), overlay.parent->fd.get());
        if (same)
            return candidate;
    }
    return {};
}
Path Filesystem::root_path() const {
    auto point = namespace_->root;
    while (auto over = child_mount(*namespace_, point->id, "/"))
        point = std::move(over);
    if (!point->source->directory)
        guest_error(20);
    if (!point->source->context.empty() || point->source->node)
        return directory_anchor(-1, "/", point, virtual_root(*point->source), point->source->components);
    return directory_anchor(point->source->root.get(), "/", point);
}
Path Filesystem::resolve(const std::string &path, const std::string &base, bool follow_final,
                         ResolveIntent intent) const {
    if (path.empty())
        guest_error(2);
    if (path.size() >= max_path || base.size() >= max_path)
        guest_error(36);
    if (base.empty() || base.front() != '/' || base.find('\0') != std::string::npos)
        guest_error(22);
    unsigned links = 0;
    Path directory;
    if (path.front() != '/')
        directory = root_path();
    if (path.front() != '/' && base != "/")
        directory = resolve_anchored(base, directory, 0, true, ResolveIntent::Lookup, links);
    return resolve_anchored(path, directory, 0, follow_final, intent, links);
}
Path Filesystem::directory_anchor(int descriptor, const std::string &guest,
                                  std::shared_ptr<mounts::Point> mount, VirtualNode::Ptr node,
                                  std::vector<std::string> virtual_components) const {
    if (guest.empty() || guest.front() != '/' || guest.find('\0') != std::string::npos)
        guest_error(22);
    if (guest.size() >= max_path)
        guest_error(36);
    auto directory = std::make_shared<DirectoryAnchor>();
    directory->set_mount(mount ? mount : namespace_->root);
    directory->guest = guest;
    directory->node = std::move(node);
    directory->virtual_components = std::move(virtual_components);
    if (directory->node) {
        if (directory->node->kind != VirtualNode::Kind::Directory)
            guest_error(20);
        // A virtual FD can outlive the path used to open it. Derive only its
        // backing-tree coordinates here; containment uses the pinned node.
        if (directory->virtual_components.empty() && directory->source) {
            const auto root = virtual_root(*directory->source);
            auto source_names = components(root->identity);
            auto node_names = components(directory->node->identity);
            if (node_names.size() < source_names.size() ||
                !std::equal(source_names.begin(), source_names.end(), node_names.begin()))
                guest_error(22);
            directory->virtual_components = directory->source->components;
            directory->virtual_components.insert(directory->virtual_components.end(),
                                                  node_names.begin() + source_names.size(), node_names.end());
        }
    } else {
        if (!S_ISDIR(descriptor_stat(descriptor).st_mode))
            guest_error(20);
        directory->fd = duplicate(descriptor);
    }
    Path result;
    if (directory->fd.get() >= 0)
        result.parent = duplicate(directory->fd.get());
    result.name = ".";
    result.guest = guest;
    result.mount = directory->point();
    result.mount_id = directory->id();
    result.mount_flags = result.mount->flags;
    result.read_only = result.mount->read_only();
    result.mount_root = directory->node
        ? same_virtual_node(directory->node, virtual_root(*directory->source))
        : directory->source->directory && directory->source->root.get() >= 0 &&
              same_directory(directory->fd.get(), directory->source->root.get());
    result.virtual_path = bool(directory->node);
    result.node = directory->node;
    result.virtual_components = directory->virtual_components;
    result.anchor = std::move(directory);
    return result;
}
Path Filesystem::resolve_at(const std::string &path, const Path &directory, uint64_t constraints,
                            bool follow_final, ResolveIntent intent) const {
    if (constraints & ~path_resolution::known ||
        (constraints & path_resolution::beneath && constraints & path_resolution::in_root))
        guest_error(22);
    // Every current backing filesystem needs host lookup or a virtual callback
    // revalidation. There is no authoritative dentry cache to satisfy CACHED.
    if (constraints & path_resolution::cached)
        guest_error(11);
    unsigned links = 0;
    return resolve_anchored(path, directory, constraints, follow_final, intent, links);
}
Path Filesystem::resolve_anchored(const std::string &path, const Path &base, uint64_t constraints,
                                  bool follow_final, ResolveIntent intent, unsigned &links) const {
    if (path.empty())
        guest_error(2);
    if (path.size() >= max_path)
        guest_error(36);
    if (path.find('\0') != std::string::npos)
        guest_error(22);
    const bool beneath = constraints & path_resolution::beneath;
    const bool in_root = constraints & path_resolution::in_root;
    const bool no_xdev = constraints & path_resolution::no_xdev;
    if (beneath && path.front() == '/')
        guest_error(18);
    const bool scoped = beneath || in_root;
    Anchor directory = path.front() == '/' && !in_root ? as_directory(root_path()) : as_directory(base);
    const Anchor boundary = scoped ? directory : Anchor{};
    std::deque<std::string> queue;
    prepend(queue, path);
    bool trailing_slash = path.back() == '/';
    const bool creating_directory = intent == ResolveIntent::CreateDirectory;
    const bool exclusive = intent == ResolveIntent::CreateExclusive;
    const bool creating_file = exclusive || intent == ResolveIntent::CreateFile;
    auto finish = [&](const Anchor &parent, const std::string &name, const std::string &guest,
                      bool mount_root, VirtualNode::Ptr node,
                      std::vector<std::string> virtual_components = {}) {
        verify_scope(parent, boundary);
        Path result;
        if (parent->fd.get() >= 0)
            result.parent = duplicate(parent->fd.get());
        result.name = name;
        result.guest = guest;
        result.mount = parent->point();
        result.mount_id = parent->id();
        result.mount_flags = result.mount->flags;
        result.read_only = result.mount->read_only();
        result.mount_root = mount_root;
        result.virtual_path = bool(parent->node);
        result.node = std::move(node);
        result.virtual_components = std::move(virtual_components);
        result.anchor = parent;
        result.scope_root = boundary;
        return result;
    };
    auto mount_directory = [&](const std::shared_ptr<mounts::Point> &point,
                               const Anchor &parent, const std::string &name, const std::string &guest) {
        auto result = std::make_shared<DirectoryAnchor>();
        result->set_mount(point);
        if (scoped)
            result->parent = parent;
        result->component = name;
        result->guest = guest;
        result->entered_mount = true;
        if (!point->source->context.empty() || point->source->node) {
            result->node = virtual_root(*point->source);
            result->virtual_components = point->source->components;
        } else
            result->fd = duplicate(point->source->root.get());
        return result;
    };
    auto reset_root = [&] {
        Anchor root = in_root ? boundary : as_directory(root_path());
        if (beneath || (no_xdev && root->id() != directory->id()))
            guest_error(18);
        directory = std::move(root);
    };
    auto ascend = [&] {
        verify_scope(directory, boundary);
        if (directory == boundary) {
            if (beneath)
                guest_error(18);
            return; // IN_ROOT treats its anchor as '/'.
        }
        if (directory->parent) {
            if (directory->entered_mount &&
                std::none_of(namespace_->points.begin(), namespace_->points.end(),
                             [&](const auto &active) { return active->id == directory->id(); }))
                return;
            if (no_xdev && directory->id() != directory->parent->id())
                guest_error(18);
            directory = directory->parent;
            return;
        }
        // An FD supplied by the descriptor table need not have a path history.
        // Reconstruct its parent by inode, stopping at this mount's source root.
        const auto point = directory->point();
        const auto &source = *point->source;
        const bool at_root = directory->node
            ? same_virtual_node(directory->node, virtual_root(source))
            : source.root.get() >= 0 && same_directory(directory->fd.get(), source.root.get());
        if (at_root) {
            if (point->id == namespace_->root->id || point->target == "/" ||
                std::none_of(namespace_->points.begin(), namespace_->points.end(),
                             [&](const auto &active) { return active->id == point->id; }))
                return;
            if (no_xdev)
                guest_error(18);
            const auto attachment = point->attachment ? point->attachment->parent : Anchor{};
            if (attachment) {
                std::shared_ptr<mounts::Point> parent_point = namespace_->root;
                for (const auto &active : namespace_->points)
                    if (active->id == attachment->id())
                        parent_point = active;
                directory = as_directory(directory_anchor(attachment->fd.get(), attachment->guest,
                    parent_point, attachment->node, attachment->virtual_components));
            } else
                guest_error(11);
            return;
        }
        auto parent = std::make_shared<DirectoryAnchor>();
        parent->set_mount(point);
        parent->guest = parent_name(directory->guest);
        if (directory->node) {
            auto names = directory->virtual_components;
            if (names.size() <= source.components.size())
                guest_error(11);
            names.pop_back();
            auto node = virtual_root(source);
            for (size_t i = source.components.size(); i < names.size(); ++i) {
                if (!node || node->kind != VirtualNode::Kind::Directory || !node->lookup)
                    guest_error(11);
                node = node->lookup(names[i]);
            }
            if (!node || node->kind != VirtualNode::Kind::Directory)
                guest_error(11);
            parent->node = std::move(node);
            parent->virtual_components = std::move(names);
        } else {
            parent->fd = FD(::openat(directory->fd.get(), "..", directory_flags()));
            check_host(parent->fd.get());
            if (no_xdev && descriptor_stat(directory->fd.get()).st_dev !=
                               descriptor_stat(parent->fd.get()).st_dev)
                guest_error(18);
            if (source.root.get() < 0 || !within_native_mount(parent->fd.get(), source.root.get()))
                guest_error(11);
        }
        directory = std::move(parent);
    };
    while (!queue.empty()) {
        const std::string component = std::move(queue.front());
        queue.pop_front();
        if (component == ".")
            continue;
        if (component == "..") {
            ascend();
            continue;
        }
        if (component.size() > 255)
            guest_error(36);
        const bool final = queue.empty();
        const std::string guest = child_name(directory->guest, component);
        if (guest.size() >= max_path)
            guest_error(36);
        if (final && trailing_slash && creating_file)
            guest_error(21);
        auto mount = anchored_child_mount(directory, component);
        if (mount) {
            if (no_xdev)
                guest_error(18);
            while (auto over = anchored_cover_mount(mount))
                mount = std::move(over);
            if (final && (creating_directory || exclusive))
                guest_error(17);
            if ((!final || trailing_slash) && !mount->source->directory)
                guest_error(20);
            auto mounted = mount_directory(mount, directory, component, guest);
            if (final)
                return finish(mounted, mount->source->name, guest, true,
                              mounted->node, mounted->virtual_components);
            directory = std::move(mounted);
            continue;
        }
        VirtualNode::Ptr node;
        std::vector<std::string> virtual_components;
        struct stat info{};
        bool exists, symlink;
        if (directory->node) {
            node = directory->node->lookup ? directory->node->lookup(component) : nullptr;
            exists = bool(node);
            symlink = node && node->kind == VirtualNode::Kind::Symlink;
            virtual_components = directory->virtual_components;
            virtual_components.push_back(component);
        } else {
            const int rc = ::fstatat(directory->fd.get(), component.c_str(), &info, AT_SYMLINK_NOFOLLOW);
            if (rc < 0 && errno != ENOENT)
                guest_error(linux_errno(errno));
            exists = rc == 0;
            symlink = exists && S_ISLNK(info.st_mode);
            if (exists && no_xdev && info.st_dev != descriptor_stat(directory->fd.get()).st_dev)
                guest_error(18);
        }
        if (!exists && !final)
            guest_error(2);
        if (exists && final && (creating_directory || exclusive))
            guest_error(17);
        if (symlink && (!final || follow_final || trailing_slash)) {
            if (constraints & path_resolution::no_symlinks || ++links > 40)
                guest_error(40);
            const bool magic = node && magic_link(directory, node);
            if (magic && constraints & path_resolution::no_magiclinks)
                guest_error(40);
            if (magic && scoped)
                guest_error(18);
            if (magic && node->open_stream) {
                // These links target anonymous streams in an internal mount,
                // not an inode on the proc mount that contains the link.
                if (no_xdev)
                    guest_error(18);
                if (!final || trailing_slash)
                    guest_error(20);
                return finish(directory, component, guest, false, node, virtual_components);
            }
            std::string target;
            if (node) {
                if (!node->target)
                    guest_error(2);
                target = node->target();
            } else {
                char buffer[max_path];
                const ssize_t count = ::readlinkat(directory->fd.get(), component.c_str(),
                                                   buffer, sizeof(buffer));
                check_host(count);
                if (size_t(count) == sizeof(buffer))
                    guest_error(36);
                target.assign(buffer, size_t(count));
            }
            if (target.empty())
                guest_error(2);
            if (target.size() >= max_path)
                guest_error(36);
            if (target.find('\0') != std::string::npos)
                guest_error(22);
            if (magic) {
                // A proc magic link jumps to its target; the pathname used to
                // describe that target is not walked under NO_XDEV.
                auto root = root_path();
                auto destination = resolve_anchored(target, root, 0, true, ResolveIntent::Lookup, links);
                if (no_xdev && destination.mount_id != directory->id())
                    guest_error(18);
                if (final && !trailing_slash)
                    return destination;
                directory = as_directory(destination);
                continue;
            }
            if (final && target.back() == '/')
                trailing_slash = true;
            if (target.front() == '/')
                reset_root();
            prepend(queue, target);
            continue;
        }
        if (final) {
            const bool is_directory = node ? node->kind == VirtualNode::Kind::Directory
                                           : exists && S_ISDIR(info.st_mode);
            if (trailing_slash && !creating_directory && !is_directory)
                guest_error(exists ? 20 : 2);
            return finish(directory, component, guest, false, node, std::move(virtual_components));
        }
        auto child = std::make_shared<DirectoryAnchor>();
        child->set_mount(directory->point());
        // Ordinary walks need only the current directory. Scoped openat2
        // keeps ancestry to revalidate every edge against rename/escape races.
        if (scoped)
            child->parent = directory;
        child->component = component;
        child->guest = guest;
        if (node) {
            if (node->kind != VirtualNode::Kind::Directory)
                guest_error(20);
            child->node = std::move(node);
            child->virtual_components = std::move(virtual_components);
        } else {
            if (!S_ISDIR(info.st_mode))
                guest_error(20);
            child->fd = FD(::openat(directory->fd.get(), component.c_str(), directory_flags()));
            check_host(child->fd.get());
            if (!same_inode(info, descriptor_stat(child->fd.get())))
                guest_error(11);
        }
        directory = std::move(child);
    }
    if (creating_file)
        guest_error(21);
    if (creating_directory)
        guest_error(17);
    const bool mount_root = directory->entered_mount ||
        (directory->node ? same_virtual_node(directory->node, virtual_root(*directory->source))
                         : directory->source->directory && directory->source->root.get() >= 0 &&
                               same_directory(directory->fd.get(), directory->source->root.get()));
    return finish(directory, ".", directory->guest, mount_root,
                  directory->node, directory->virtual_components);
}
std::vector<std::string> Filesystem::mounted_children(const std::string &directory) const {
    std::vector<std::string> children;
    const auto parent = resolve(directory);
    if (!S_ISDIR(parent.stat().st_mode))
        guest_error(20);
    const std::string prefix = parent.guest == "/" ? "/" : parent.guest + "/";
    for (const auto &point : namespace_->points) {
        if (point->parent != parent.mount_id || point->target.compare(0, prefix.size(), prefix) != 0)
            continue;
        const std::string suffix = point->target.substr(prefix.size());
        if (!suffix.empty() && suffix.find('/') == std::string::npos &&
            std::find(children.begin(), children.end(), suffix) == children.end())
            children.push_back(suffix);
    }
    return children;
}
FD Filesystem::open(const std::string &path, const std::string &base, int flags, mode_t mode) {
    return resolve(path, base, !(flags & O_NOFOLLOW)).open(flags, mode);
}
std::vector<uint8_t> Filesystem::read_file(const std::string &path, uint64_t limit) {
    auto resolved = resolve(path);
    resolved.require_executable();
    FD fd = resolved.open(O_RDONLY);
    struct stat info{};
    check_host(::fstat(fd.get(), &info));
    if (!S_ISREG(info.st_mode) || info.st_size < 0 || uint64_t(info.st_size) > limit)
        throw Failure("ELF file exceeds resource limit or is not regular");
    std::vector<uint8_t> data(static_cast<size_t>(info.st_size));
    size_t offset = 0;
    while (offset < data.size()) {
        ssize_t count = ::pread(fd.get(), data.data() + offset, data.size() - offset, offset);
        if (count < 0 && errno == EINTR)
            continue;
        if (count <= 0)
            throw Failure("cannot read complete ELF file");
        offset += static_cast<size_t>(count);
    }
    return data;
}
} // namespace operit::arm64
