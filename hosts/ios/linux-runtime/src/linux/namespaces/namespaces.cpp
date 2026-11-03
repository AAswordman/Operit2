#include "linux/namespaces/namespaces.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "fs/filesystem.h"
#include <atomic>
#include <mutex>
#include <string>
#include <vector>

namespace operit::arm64::namespaces {
namespace {
constexpr uint64_t unsupported_namespaces = namespace_flags & ~clone_newns;
constexpr uint32_t open_cloexec = 0x80000, open_path = 0x200000;

// A real namespace FD owns its attachment through the same control block as
// the open description. Dup/fork preserve it; descriptor transfer must also
// preserve that control block, or reject this virtual FD.
struct NamespaceOwner {
    std::shared_ptr<MountNamespace> file;
    void operator()(OpenDescription *description) noexcept {
        delete description;
        file.reset();
    }
};
} // namespace

class MountNamespace {
  public:
    Filesystem filesystem;
    std::shared_ptr<const UserNamespaceIdentity> owner;
    uint64_t inode;

    MountNamespace(const Filesystem &source, std::shared_ptr<const UserNamespaceIdentity> user_namespace,
                   uint64_t number)
        : filesystem(source), owner(std::move(user_namespace)), inode(number) {}
};

namespace {
struct NamespaceRegistry {
    std::mutex mutex;
    std::vector<std::weak_ptr<MountNamespace>> mounts;
    uint64_t next_inode = 1;
};
NamespaceRegistry &registry() {
    static NamespaceRegistry value;
    return value;
}
std::shared_ptr<MountNamespace> pin_namespace(const Filesystem &filesystem,
                                               std::shared_ptr<const UserNamespaceIdentity> owner) {
    if (!owner)
        guest_error(22);
    auto &state = registry();
    std::lock_guard<std::mutex> lock(state.mutex);
    for (auto it = state.mounts.begin(); it != state.mounts.end();) {
        auto existing = it->lock();
        if (!existing) {
            it = state.mounts.erase(it);
        } else {
            if (existing->filesystem.shares_mounts(filesystem)) {
                if (existing->owner != owner)
                    guest_error(22);
                return existing;
            }
            ++it;
        }
    }
    auto created = std::make_shared<MountNamespace>(filesystem, std::move(owner), state.next_inode++);
    state.mounts.emplace_back(created);
    return created;
}
} // namespace

int64_t unshare(uint64_t flags, UnshareContext &context) {
    constexpr uint64_t accepted = clone_fs | clone_files | namespace_flags;
    if (flags & ~accepted)
        guest_error(22);
    if (!flags)
        return 0;
    if (flags & unsupported_namespaces) {
        if (!context.cap_sys_admin)
            guest_error(1);
        guest_error(22);
    }
    if ((flags & clone_newns) && !context.cap_sys_admin)
        guest_error(1);
    if (!context.task_scoped)
        guest_error(22);

    const bool copy_fs = flags & (clone_fs | clone_newns);
    std::unique_ptr<Filesystem> mounts;
    std::shared_ptr<MountNamespace> next_identity;
    if (flags & clone_newns) {
        if (!context.mount_identity || !*context.mount_identity ||
            !same_mount_namespace(*context.mount_identity, context.filesystem))
            guest_error(22);
        mounts = std::make_unique<Filesystem>(context.filesystem);
        mounts->unshare_mounts();
        next_identity = pin_namespace(*mounts, (*context.mount_identity)->owner);
    }
    std::unique_ptr<PreparedChange> fs;
    if (copy_fs) {
        if (!context.prepare_fs)
            guest_error(22);
        fs = context.prepare_fs(mounts.get());
        if (!fs)
            guest_error(22);
    }
    // unshare() copies only a shared table and leaves it untouched on failure.
    // Do it last: all remaining publications are non-throwing.
    if (flags & clone_files)
        context.descriptors.unshare();
    if (mounts)
        context.filesystem = std::move(*mounts);
    if (fs)
        fs->commit();
    if (next_identity)
        context.mount_identity->swap(next_identity);
    return 0;
}

void check_clone(uint64_t flags, bool cap_sys_admin) {
    if ((flags & clone_newns) && (flags & clone_fs))
        guest_error(22);
    if (flags & unsupported_namespaces) {
        if (!cap_sys_admin)
            guest_error(1);
        guest_error(22);
    }
    if ((flags & clone_newns) && !cap_sys_admin)
        guest_error(1);
}

std::shared_ptr<MountNamespace> acquire_mount_namespace(
    const Filesystem &filesystem, std::shared_ptr<const UserNamespaceIdentity> owner) {
    return pin_namespace(filesystem, std::move(owner));
}

int open_mount_namespace(DescriptorTable &descriptors, const Filesystem &filesystem,
                         std::shared_ptr<MountNamespace> file, uint32_t flags) {
    if (flags & ~(open_cloexec | open_path))
        guest_error(22);
    if (!same_mount_namespace(file, filesystem))
        guest_error(22);
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::File,
                                              "/proc/ns/mnt/" + std::to_string(file->inode), 0444);
    node->read_range = [](uint64_t, size_t) -> std::string { guest_error(22); };
    auto description = std::shared_ptr<OpenDescription>(new OpenDescription(int(flags)), NamespaceOwner{file});
    description->virtual_file = std::make_shared<VirtualOpen>(std::move(node));
    const int fd = descriptors.insert(FD(), "", int(flags));
    auto &entry = descriptors.get(fd);
    description->inode = entry.description->inode;
    entry.description = std::move(description);
    return fd;
}

std::shared_ptr<MountNamespace> mount_namespace_file(
    const std::shared_ptr<OpenDescription> &description) noexcept {
    const auto *owner = std::get_deleter<NamespaceOwner>(description);
    return owner ? owner->file : nullptr;
}

bool same_mount_namespace(const std::shared_ptr<MountNamespace> &file,
                          const Filesystem &filesystem) noexcept {
    return file && file->filesystem.shares_mounts(filesystem);
}

int64_t setns(uint64_t fd, uint64_t type, SetnsContext &context) {
    // The Linux ABI declares both arguments as int; discard upper register bits.
    const auto &descriptor = context.descriptors.get(uint32_t(fd));
    auto file = mount_namespace_file(descriptor.description);
    if (!file)
        guest_error(22);
    const auto kind = uint32_t(type);
    if (kind && kind != clone_newns)
        guest_error(22);
    // User namespace ancestry and capability inheritance are not implemented.
    // Exact identity plus both capabilities is the supported, safe subset.
    if (!context.user_namespace || context.user_namespace != file->owner ||
        !context.cap_sys_admin || !context.cap_sys_chroot)
        guest_error(1);
    if (!context.task_scoped || !context.fs_private || !context.prepare_mount_install ||
        !context.filesystem || !context.mount_identity || !*context.mount_identity ||
        !same_mount_namespace(*context.mount_identity, *context.filesystem))
        guest_error(22);
    auto change = context.prepare_mount_install(file->filesystem);
    if (!change)
        guest_error(22);
    change->commit();
    context.mount_identity->swap(file);
    return 0;
}
} // namespace operit::arm64::namespaces
