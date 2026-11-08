#include "base/support.h"
#include "fs/filesystem.h"
#include "fs/mount/storage.h"
#include "linux/error_mapping.h"
#include <cerrno>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
namespace {
FD duplicate(int fd) {
    FD result(::fcntl(fd, F_DUPFD_CLOEXEC, 0));
    check_host(result.get());
    return result;
}
std::shared_ptr<mounts::Source> pinned_file(const Path &path, int descriptor) {
    struct stat original{};
    check_host(::fstat(descriptor, &original));
    if (!S_ISREG(original.st_mode))
        guest_error(95);
    auto storage = mounts::TemporaryTree::create(0700);
    check_host(
        ::linkat(path.parent.get(), path.name.c_str(), storage->descriptor(), "inode", AT_SYMLINK_FOLLOW));
    struct stat pinned{};
    check_host(::fstatat(storage->descriptor(), "inode", &pinned, AT_SYMLINK_NOFOLLOW));
    if (original.st_dev != pinned.st_dev || original.st_ino != pinned.st_ino)
        guest_error(11);
    auto source = std::make_shared<mounts::Source>();
    source->directory = false;
    source->root = duplicate(storage->descriptor());
    source->name = "inode";
    source->storage = std::move(storage);
    return source;
}
} // namespace
std::shared_ptr<mounts::Source> Filesystem::mount_source(const Path &path) {
    const auto info = path.stat();
    auto source = std::make_shared<mounts::Source>();
    source->directory = S_ISDIR(info.st_mode);
    source->locked_read_only = path.mount && path.mount->source->locked_read_only;
    if (path.virtual_path) {
        source->node = path.node;
        if (path.mount) {
            source->context = path.mount->source->context;
            source->components = path.virtual_components;
            source->node = path.mount->source->node;
        }
        return source;
    }
    if (source->directory) {
        source->root = path.open(O_RDONLY | O_DIRECTORY);
        if (path.mount)
            source->storage = path.mount->source->storage;
        return source;
    }
    if (!S_ISREG(info.st_mode))
        guest_error(95);
    // Pin the inode in a host-private directory. This survives guest renames and
    // unlink, while openat creates a fresh file description for every guest open.
    FD anchor = path.open(O_RDONLY | O_NONBLOCK);
    auto pinned = pinned_file(path, anchor.get());
    pinned->locked_read_only = source->locked_read_only;
    return pinned;
}
std::shared_ptr<mounts::Point> Filesystem::add_mount(std::shared_ptr<mounts::Source> source,
                                                     const Path &target, const std::string &type,
                                                     const std::string &label, uint64_t flags) {
    auto point = std::make_shared<mounts::Point>();
    point->id = namespace_->next_id;
    point->parent = target.mount_id;
    point->flags = flags;
    point->target = target.guest;
    point->type = type;
    point->label = label;
    point->source = std::move(source);
    // Register the attachment while the resolved target is still alive. Normal
    // lookups must not keep directory chains just in case a later mount needs one.
    attach_mount(*point, target);
    namespace_->points.push_back(point);
    ++namespace_->next_id;
    return point;
}
void Filesystem::bind(const std::string &source, const std::string &target, bool read_only) {
    if (source.empty() || target.empty() || target.front() != '/')
        guest_error(22);
    auto destination = resolve(target);
    if (destination.guest == "/" || destination.mount_root || destination.virtual_path)
        guest_error(16);
    // An authorized directory need not grant access to its parent (for example
    // an iOS app container). Open the source directly and retain that directory.
    // The external binding API follows source symlinks; guest paths never use
    // this host pathname to resolve outside the guest namespace.
    FD anchor(::open(source.c_str(), O_RDONLY | O_NONBLOCK | O_CLOEXEC));
    check_host(anchor.get());
    struct stat info{};
    check_host(::fstat(anchor.get(), &info));
    std::shared_ptr<mounts::Source> backing;
    if (S_ISDIR(info.st_mode)) {
        backing = std::make_shared<mounts::Source>();
        backing->root = std::move(anchor);
    } else {
        // Regular files still need their parent to pin the inode with linkat.
        size_t slash = source.rfind('/');
        const std::string parent = slash == std::string::npos ? "." : slash == 0 ? "/" : source.substr(0, slash);
        const std::string name = slash == std::string::npos ? source : source.substr(slash + 1);
        Path native;
        native.parent = FD(::open(parent.c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC));
        check_host(native.parent.get());
        native.name = name.empty() ? "." : name;
        backing = pinned_file(native, anchor.get());
    }
    struct stat existing{};
    const int present =
        ::fstatat(destination.parent.get(), destination.name.c_str(), &existing, AT_SYMLINK_NOFOLLOW);
    if (present == 0 && bool(S_ISDIR(existing.st_mode)) != backing->directory)
        guest_error(backing->directory ? 20 : 21);
    if (present < 0 && errno != ENOENT)
        guest_error(linux_errno(errno));
    backing->locked_read_only = read_only;
    add_mount(std::move(backing), destination, "hostfs", "hostfs",
              read_only ? mounts::read_only : 0);
}
void Filesystem::mount_virtual(const std::string &target, VirtualNode::Ptr root) {
    if (!root || target.empty() || target.front() != '/' || target == "/")
        guest_error(22);
    virtual_contexts_[target] = std::move(root);
    if (namespace_->builtins.count(target))
        return;
    // Built-in filesystems may be installed into an otherwise empty external
    // root. Supply missing parents in the namespace without changing that root.
    // Guest mount() deliberately has no corresponding directory creation path.
    size_t end = target.find('/', 1);
    while (end != std::string::npos) {
        const std::string prefix = target.substr(0, end);
        if (!prefix.empty()) {
            auto parent = resolve(prefix);
            bool missing = false;
            try {
                if (!S_ISDIR(parent.stat().st_mode))
                    guest_error(20);
            } catch (const GuestError &error) {
                if (error.number != 2)
                    throw;
                missing = true;
            }
            if (missing) {
                auto directory =
                    std::make_shared<VirtualNode>(VirtualNode::Kind::Directory, parent.guest, 0555);
                auto backing = std::make_shared<mounts::Source>();
                backing->context = prefix;
                backing->node = directory;
                virtual_contexts_[prefix] = std::move(directory);
                add_mount(std::move(backing), parent, "virtualfs", "virtualfs", 0);
                namespace_->builtins[prefix] = prefix;
            }
        }
        end = target.find('/', end + 1);
    }
    auto destination = resolve(target);
    auto source = std::make_shared<mounts::Source>();
    source->context = target;
    source->node = virtual_contexts_.at(target);
    source->directory = source->node->kind == VirtualNode::Kind::Directory;
    const std::string type = target == "/proc" ? "proc" : target == "/dev" ? "devfs" : "virtualfs";
    add_mount(std::move(source), destination, type, type, 0);
    namespace_->builtins[target] = target;
}
} // namespace operit::arm64
