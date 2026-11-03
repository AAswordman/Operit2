#pragma once
#include "base/fd.h"
#include "fs/mount/namespace.h"
#include "fs/virtual/node.h"
#include <cstdint>
#include <map>
#include <memory>
#include <string>
#include <vector>

namespace operit::arm64 {
struct DirectoryAnchor;
struct MountAnchors;
namespace path_resolution {
constexpr uint64_t no_xdev = 0x01, no_magiclinks = 0x02, no_symlinks = 0x04;
constexpr uint64_t beneath = 0x08, in_root = 0x10, cached = 0x20;
constexpr uint64_t known = no_xdev | no_magiclinks | no_symlinks | beneath | in_root | cached;
} // namespace path_resolution
struct Path {
    FD parent;
    std::string name;
    std::string guest;
    uint64_t mount_id = 0, mount_flags = 0;
    bool read_only = false;
    bool mount_root = false;
    bool virtual_path = false;
    VirtualNode::Ptr node;
    std::shared_ptr<mounts::Point> mount;
    std::vector<std::string> virtual_components;
    // Pinned directory ancestry. guest is a namespace/display coordinate only.
    std::shared_ptr<const DirectoryAnchor> anchor;
    std::shared_ptr<const DirectoryAnchor> scope_root;
    struct stat stat() const;
    std::string readlink() const;
    FD open(int flags, mode_t mode = 0) const;
    bool is_read_only() const;
    bool permits_devices() const;
    void require_executable() const;
    void require_writable() const;
    void require_removable() const;
};
class Filesystem {
    FD root_;
    std::shared_ptr<mounts::Namespace> namespace_ = std::make_shared<mounts::Namespace>();
    std::map<std::string, VirtualNode::Ptr> virtual_contexts_;
    mutable std::shared_ptr<MountAnchors> mount_anchors_;
    VirtualNode::Ptr virtual_root(const mounts::Source &source) const;
    std::shared_ptr<mounts::Source> mount_source(const Path &path);
    std::shared_ptr<mounts::Point> add_mount(std::shared_ptr<mounts::Source> source,
                                             const std::string &target, uint64_t parent,
                                             const std::string &type, const std::string &label,
                                             uint64_t flags);

  public:
    enum class ResolveIntent { Lookup, CreateDirectory, CreateFile, CreateExclusive };
    explicit Filesystem(const char *root);
    Filesystem(const Filesystem &source);
    Filesystem &operator=(const Filesystem &source);
    Filesystem(Filesystem &&) noexcept = default;
    Filesystem &operator=(Filesystem &&) noexcept = default;
    // External roots and directory/file bindings are used in place. Mounted file
    // inodes are pinned independently of the source pathname and open offsets.
    void bind(const std::string &source, const std::string &target, bool read_only);
    void mount_virtual(const std::string &target, VirtualNode::Ptr root);
    void mount(const std::string &source, const std::string &target, const std::string &type, uint64_t flags,
               const std::string &data = {}, const std::string &base = "/", VirtualNode::Ptr root = {});
    void unmount(const std::string &target, uint32_t flags, const std::string &base = "/");
    void unshare_mounts();
    bool shares_mounts(const Filesystem &other) const {
        return namespace_ == other.namespace_;
    }
    Path resolve(const std::string &path, const std::string &base = "/", bool follow_final = true,
                 ResolveIntent intent = ResolveIntent::Lookup) const;
    // Pass the live directory FD from the descriptor table, never reopen guest.
    // A virtual directory passes its node and mount with descriptor == -1.
    Path directory_anchor(int descriptor, const std::string &guest,
                          std::shared_ptr<mounts::Point> mount = {}, VirtualNode::Ptr node = {},
                          std::vector<std::string> virtual_components = {}) const;
    Path resolve_at(const std::string &path, const Path &directory, uint64_t constraints = 0,
                    bool follow_final = true, ResolveIntent intent = ResolveIntent::Lookup) const;
    std::string mount_table() const;
    std::vector<std::string> mounted_children(const std::string &directory) const;
    FD open(const std::string &path, const std::string &base, int flags, mode_t mode);
    std::vector<uint8_t> read_file(const std::string &path, uint64_t limit);

  private:
    void sync_mount_anchors() const;
    void remember_mount_target(const Path &path) const;
    std::shared_ptr<mounts::Point> anchored_child_mount(
        const std::shared_ptr<const DirectoryAnchor> &parent, const std::string &component) const;
    std::shared_ptr<mounts::Point> anchored_cover_mount(const std::shared_ptr<mounts::Point> &point) const;
    Path root_path() const;
    Path resolve_anchored(const std::string &path, const Path &directory, uint64_t constraints,
                          bool follow_final, ResolveIntent intent, unsigned &links) const;
};
} // namespace operit::arm64
