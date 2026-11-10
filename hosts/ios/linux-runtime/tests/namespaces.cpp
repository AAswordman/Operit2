#include "base/support.h"
#include "fs/descriptor_table.h"
#include "fs/filesystem.h"
#include "linux/namespaces/namespaces.h"
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <functional>
#include <stdexcept>
#include <fcntl.h>
#include <sys/resource.h>
#include <unistd.h>

using namespace operit::arm64;
namespace ns = operit::arm64::namespaces;

namespace {
void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}
void error(int expected, const std::function<void()> &call) {
    try {
        call();
    } catch (const GuestError &value) {
        if (value.number != expected)
            throw std::runtime_error("expected guest errno " + std::to_string(expected) + ", got " +
                                     std::to_string(value.number));
        return;
    }
    throw std::runtime_error("expected a guest error");
}
struct Root {
    std::string path;
    Root() {
        char pattern[] = "/tmp/operit-namespaces-XXXXXX";
        auto *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        path = created;
        std::ofstream(path + "/file") << "abcdef";
    }
    ~Root() { std::filesystem::remove_all(path); }
};
struct FsState {
    std::string cwd = "/";
    unsigned mask = 022;
    std::shared_ptr<Path> directory;
};
std::function<std::unique_ptr<ns::PreparedChange>(const Filesystem *)>
prepare_fs(std::shared_ptr<FsState> &current) {
    return [&current](const Filesystem *mounts) -> std::unique_ptr<ns::PreparedChange> {
        auto copy = std::make_shared<FsState>(*current);
        if (mounts) {
            // Fixture paths are unchanged. Production must also handle
            // renamed or detached directories without resolving by spelling.
            copy->directory = std::make_shared<Path>(mounts->resolve(copy->cwd));
        }
        return ns::replace_shared(current, std::move(copy));
    };
}
int native_file(DescriptorTable &table, const std::string &path) {
    FD host(::open(path.c_str(), O_RDONLY | O_CLOEXEC));
    require(host.get() >= 0, "open fixture file");
    return table.insert(std::move(host), "/file", 0);
}
bool exists(DescriptorTable &table, int fd) {
    try {
        table.get(uint32_t(fd));
        return true;
    } catch (const GuestError &value) {
        require(value.number == 9, "descriptor lookup errno");
        return false;
    }
}

void unshare_fs_and_files() {
    Root root;
    Filesystem parent(root.path.c_str()), child(parent);
    DescriptorTable parent_files, child_files;
    const int fd = native_file(parent_files, root.path + "/file");
    child_files.share_with(parent_files);
    auto parent_fs = std::make_shared<FsState>();
    auto child_fs = parent_fs;
    ns::UnshareContext context{child, child_files, true, false, prepare_fs(child_fs)};
    require(ns::unshare(ns::clone_fs | ns::clone_files, context) == 0, "detach fs/files");
    require(parent.shares_mounts(child), "unshare fs/files changed mount namespace");
    require(parent_fs != child_fs && child_fs->cwd == "/" && child_fs->mask == 022,
            "fs context was not copied");
    child_fs->cwd = "/other";
    child_fs->mask = 077;
    require(parent_fs->cwd == "/" && parent_fs->mask == 022, "fs context remains shared");
    require(parent_files.get(fd).description == child_files.get(fd).description,
            "unshare must retain open description identity");
    child_files.get(fd).descriptor_flags = 1;
    require(parent_files.get(fd).descriptor_flags == 0, "descriptor flags remain shared");
    char byte = 0;
    require(::read(parent_files.get(fd).host.get(), &byte, 1) == 1 && byte == 'a', "parent read");
    require(::read(child_files.get(fd).host.get(), &byte, 1) == 1 && byte == 'b',
            "open-file offsets must remain shared");
    child_files.close(fd);
    require(exists(parent_files, fd) && !exists(child_files, fd), "file tables remain shared");
    const int later = native_file(parent_files, root.path + "/file");
    require(!exists(child_files, later), "new parent descriptor leaked into child");
}

void descriptor_exhaustion_and_rollback() {
    Root root;
    Filesystem fs(root.path.c_str());
    DescriptorTable private_files, shared_files, peer_files;
    const int private_fd = native_file(private_files, root.path + "/file");
    const int shared_fd = native_file(shared_files, root.path + "/file");
    const int private_host_fd = private_files.get(private_fd).host.get();
    peer_files.share_with(shared_files);
    auto state = std::make_shared<FsState>();
    auto original_state = state;
    ns::UnshareContext private_context{fs, private_files, true, false, prepare_fs(state)};
    ns::UnshareContext shared_context{fs, shared_files, true, false, prepare_fs(state)};

    struct Limit {
        struct rlimit original{};
        Limit() {
            require(::getrlimit(RLIMIT_NOFILE, &original) == 0, "getrlimit");
            auto limited = original;
            require(::fcntl(0, F_GETFD) >= 0, "descriptor zero must be occupied");
            limited.rlim_cur = 1;
            require(::setrlimit(RLIMIT_NOFILE, &limited) == 0, "setrlimit");
        }
        ~Limit() { ::setrlimit(RLIMIT_NOFILE, &original); }
    } limit;

    require(ns::unshare(ns::clone_files, private_context) == 0 &&
                private_files.get(private_fd).host.get() == private_host_fd,
            "private file table was copied under descriptor exhaustion");
    shared_context.prepare_fs = [](const Filesystem *) -> std::unique_ptr<ns::PreparedChange> {
        guest_error(5);
    };
    error(5, [&] { ns::unshare(ns::clone_fs | ns::clone_files, shared_context); });
    require(state == original_state, "failed unshare committed fs state");
    shared_files.get(shared_fd).descriptor_flags = 1;
    require(peer_files.get(shared_fd).descriptor_flags == 1,
            "failed unshare detached shared table");

    // Copying guest entries no longer requires native descriptor capacity.
    shared_context.prepare_fs = prepare_fs(state);
    require(ns::unshare(ns::clone_fs | ns::clone_files, shared_context) == 0 && state != original_state,
            "shared file table could not unshare under native descriptor exhaustion");
    shared_files.get(shared_fd).descriptor_flags = 0;
    require(peer_files.get(shared_fd).descriptor_flags == 1,
            "successful unshare retained shared descriptor flags");
    shared_files.close(shared_fd);
    require(exists(peer_files, shared_fd), "unshared close invalidated the peer descriptor");
}

void mount_isolation_and_rollback() {
    Root root;
    Filesystem parent(root.path.c_str());
    parent.mount_virtual("/base", std::make_shared<VirtualNode>(VirtualNode::Kind::Directory, "/base", 0555));
    Filesystem child(parent), sibling(parent);
    DescriptorTable parent_files, child_files;
    const int fd = native_file(parent_files, root.path + "/file");
    child_files.share_with(parent_files);
    auto parent_fs = std::make_shared<FsState>();
    parent_fs->cwd = "/base";
    parent_fs->directory = std::make_shared<Path>(parent.resolve("/base"));
    auto child_fs = parent_fs;
    ns::UnshareContext context{child, child_files, true, true, prepare_fs(child_fs)};
    auto owner = std::make_shared<ns::UserNamespaceIdentity>();
    auto parent_identity = ns::acquire_mount_namespace(parent, owner);
    auto child_identity = parent_identity;
    context.mount_identity = &child_identity;
    require(ns::unshare(ns::clone_newns, context) == 0, "unshare mount namespace");
    require(!parent.shares_mounts(child) && parent.shares_mounts(sibling), "mount identity copy");
    require(child_identity != parent_identity && ns::same_mount_namespace(child_identity, child),
            "unshare did not replace namespace identity");
    require(parent_fs != child_fs && child_fs->directory->mount != parent_fs->directory->mount &&
                child_fs->directory->mount == child.resolve("/base").mount,
            "new mount namespace did not rebase cwd");
    child.mount("", "/base", "", mounts::remount | mounts::read_only);
    require(child.resolve("/base").read_only && !parent.resolve("/base").read_only &&
                !sibling.resolve("/base").read_only && child_fs->directory->is_read_only() &&
                !parent_fs->directory->is_read_only(),
            "remount leaked to old namespace");
    Filesystem inherited(child);
    require(inherited.shares_mounts(child), "fork must inherit mount namespace");
    child.mount("", "/base", "", mounts::remount | mounts::no_execute);
    require((inherited.resolve("/base").mount_flags & mounts::no_execute) &&
                !(parent.resolve("/base").mount_flags & mounts::no_execute),
            "inherited namespace does not share later changes");

    auto before_fs = child_fs;
    auto before_identity = child_identity;
    Filesystem before_mounts(child);
    context.prepare_fs = [](const Filesystem *) -> std::unique_ptr<ns::PreparedChange> {
        guest_error(5);
    };
    error(5, [&] { ns::unshare(ns::clone_newns | ns::clone_files, context); });
    require(child.shares_mounts(before_mounts) && child_fs == before_fs && child_identity == before_identity,
            "failure committed mount or fs state");
    child_files.close(fd);
    require(!exists(parent_files, fd), "failed operation partially detached descriptor table");
}

void flag_and_permission_errors() {
    Root root;
    Filesystem fs(root.path.c_str());
    DescriptorTable table;
    auto state = std::make_shared<FsState>();
    ns::UnshareContext context{fs, table, false, false, prepare_fs(state)};
    require(ns::unshare(0, context) == 0, "unshare zero");
    error(22, [&] { ns::unshare(uint64_t{1} << 60, context); });
    error(22, [&] { ns::unshare(ns::clone_fs, context); });
    error(22, [&] { ns::unshare(ns::clone_files, context); });
    error(1, [&] { ns::unshare(ns::clone_newns, context); });
    for (auto flag : {ns::clone_newtime, ns::clone_newcgroup, ns::clone_newuts, ns::clone_newipc,
                      ns::clone_newuser, ns::clone_newpid, ns::clone_newnet})
        error(1, [&] { ns::unshare(flag, context); });
    context.task_scoped = true;
    context.cap_sys_admin = true;
    for (auto flag : {ns::clone_newtime, ns::clone_newcgroup, ns::clone_newuts, ns::clone_newipc,
                      ns::clone_newuser, ns::clone_newpid, ns::clone_newnet})
        error(22, [&] { ns::unshare(flag, context); });
    context.prepare_fs = {};
    error(22, [&] { ns::unshare(ns::clone_fs, context); });
    error(22, [&] { ns::unshare(ns::clone_newns, context); });
    error(22, [&] { ns::check_clone(ns::clone_fs | ns::clone_newns, false); });
    error(1, [&] { ns::check_clone(ns::clone_newns, false); });
    error(1, [&] { ns::check_clone(ns::clone_newnet, false); });
    error(22, [&] { ns::check_clone(ns::clone_newnet, true); });
    ns::check_clone(ns::clone_newns, true);
    ns::check_clone(ns::clone_fs | ns::clone_files, false);
}

class FixtureInstall final : public ns::PreparedChange {
    Filesystem &current_;
    Filesystem target_;
    std::shared_ptr<FsState> &state_;
    std::shared_ptr<FsState> next_;

  public:
    FixtureInstall(Filesystem &current, const Filesystem &target, std::shared_ptr<FsState> &state)
        : current_(current), target_(target), state_(state), next_(std::make_shared<FsState>(*state)) {
        next_->cwd = "/";
        next_->directory = std::make_shared<Path>(target_.resolve("/"));
    }
    void commit() noexcept override {
        // This fixture has the same guest root and no local virtual contexts.
        // Production must install only the namespace, not the target's context.
        current_ = std::move(target_);
        state_.swap(next_);
    }
};

void namespace_fds_and_setns() {
    Root root;
    Filesystem parent(root.path.c_str());
    parent.mount_virtual("/base", std::make_shared<VirtualNode>(VirtualNode::Kind::Directory, "/base", 0555));
    Filesystem child(parent);
    DescriptorTable parent_files;
    auto owner = std::make_shared<ns::UserNamespaceIdentity>();
    auto parent_identity = ns::acquire_mount_namespace(parent, owner);
    auto child_identity = parent_identity;
    std::weak_ptr<const ns::UserNamespaceIdentity> owner_lifetime = owner;
    int temporary = ns::open_mount_namespace(parent_files, parent, parent_identity);
    auto inode = parent_files.get(temporary).description->virtual_file->node->stat().st_ino;
    parent_files.close(temporary);
    temporary = ns::open_mount_namespace(parent_files, parent, parent_identity);
    require(ns::mount_namespace_file(parent_files.get(temporary).description) == parent_identity &&
                parent_files.get(temporary).description->virtual_file->node->stat().st_ino == inode,
            "namespace identity changed after every namespace FD was closed");
    parent_files.close(temporary);
    const int fd = ns::open_mount_namespace(parent_files, parent, parent_identity, 0x80000);
    require(parent_files.get(fd).descriptor_flags == 1, "namespace FD CLOEXEC");
    auto file = ns::mount_namespace_file(parent_files.get(fd).description);
    require(file && ns::same_mount_namespace(file, parent), "namespace FD identity");
    const int another = ns::open_mount_namespace(parent_files, parent, parent_identity);
    require(ns::mount_namespace_file(parent_files.get(another).description) == file,
            "opens of same namespace must retain identity");
    const int path_fd = ns::open_mount_namespace(parent_files, parent, parent_identity, 0x200000 | 0x80000);
    require((parent_files.get(path_fd).description->flags & 0x200000) &&
                parent_files.get(path_fd).descriptor_flags == 1 &&
                ns::mount_namespace_file(parent_files.get(path_fd).description) == file,
            "O_PATH namespace FD lost flags or identity");
    error(22, [&] { ns::open_mount_namespace(parent_files, parent, {}, 0); });
    error(22, [&] { ns::open_mount_namespace(parent_files, parent, parent_identity, 2); });
    error(22, [&] { ns::acquire_mount_namespace(parent, {}); });
    error(22, [&] { ns::acquire_mount_namespace(parent, std::make_shared<ns::UserNamespaceIdentity>()); });
    const int duplicate = parent_files.duplicate(fd);
    DescriptorTable child_files(parent_files);
    parent_files.close(fd);
    parent_files.close(another);
    require(ns::mount_namespace_file(child_files.get(duplicate).description) == file,
            "dup/fork lost namespace FD identity");
    owner.reset();
    require(!owner_lifetime.expired(), "namespace owner identity was not pinned");
    auto caller_owner = owner_lifetime.lock();
    auto child_fs = std::make_shared<FsState>();
    ns::UnshareContext unshare{child, child_files, true, true, prepare_fs(child_fs)};
    unshare.mount_identity = &child_identity;
    require(ns::unshare(ns::clone_newns, unshare) == 0 && !parent.shares_mounts(child),
            "fixture detached namespace");
    error(22, [&] { ns::open_mount_namespace(parent_files, parent, child_identity); });
    error(22, [&] { ns::open_mount_namespace(child_files, child, parent_identity); });
    const int ordinary = native_file(child_files, root.path + "/file");
    const int forged = child_files.insert(FD(), "", 0);
    child_files.get(forged).description->virtual_file =
        child_files.get(duplicate).description->virtual_file;
    ns::SetnsContext context{child_files, true, true, false, false, caller_owner,
        [&child, &child_fs](const Filesystem &target) -> std::unique_ptr<ns::PreparedChange> {
            return std::make_unique<FixtureInstall>(child, target, child_fs);
        }};
    context.filesystem = &child;
    context.mount_identity = &child_identity;
    error(9, [&] { ns::setns(uint64_t(-1), 0, context); });
    error(22, [&] { ns::setns(ordinary, 0, context); });
    error(22, [&] { ns::setns(forged, 0, context); });
    error(22, [&] { ns::setns(duplicate, ns::clone_newnet, context); });
    error(1, [&] { ns::setns(duplicate, 0, context); });
    context.cap_sys_admin = true;
    error(1, [&] { ns::setns(duplicate, 0, context); });
    context.cap_sys_chroot = true;
    context.user_namespace = std::make_shared<ns::UserNamespaceIdentity>();
    error(1, [&] { ns::setns(duplicate, 0, context); });
    context.user_namespace = caller_owner;
    context.fs_private = false;
    error(22, [&] { ns::setns(duplicate, 0, context); });
    context.fs_private = true;
    context.task_scoped = false;
    error(22, [&] { ns::setns(duplicate, 0, context); });
    context.task_scoped = true;
    auto detached_identity = child_identity;
    child_identity = parent_identity;
    error(22, [&] { ns::setns(duplicate, 0, context); });
    child_identity = detached_identity;
    auto install = context.prepare_mount_install;
    context.prepare_mount_install = {};
    error(22, [&] { ns::setns(duplicate, 0, context); });
    context.prepare_mount_install = [](const Filesystem &) -> std::unique_ptr<ns::PreparedChange> {
        guest_error(5);
    };
    auto before = child_fs;
    error(5, [&] { ns::setns(duplicate, 0, context); });
    require(!parent.shares_mounts(child) && before == child_fs, "failed setns committed state");
    context.prepare_mount_install = install;
    require(ns::setns((uint64_t{1} << 32) | uint32_t(path_fd),
                      (uint64_t{1} << 32) | ns::clone_newns, context) == 0,
            "setns O_PATH namespace fd and int argument truncation");
    require(parent.shares_mounts(child) && child_fs->cwd == "/" && child_identity == parent_identity,
            "setns did not join target identity");
    parent.mount("", "/base", "", mounts::remount | mounts::no_execute);
    require(child.resolve("/base").mount_flags & mounts::no_execute,
            "joined namespace does not share subsequent mount changes");
    parent.unshare_mounts();
    require(!parent.shares_mounts(child) && ns::same_mount_namespace(file, child),
            "namespace FD did not pin original namespace after owner unshared");
}
} // namespace

int main() {
    try {
        unshare_fs_and_files();
        descriptor_exhaustion_and_rollback();
        mount_isolation_and_rollback();
        flag_and_permission_errors();
        namespace_fds_and_setns();
        std::puts("namespace tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "namespace tests: %s\n", error.what());
        return 1;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "namespace tests: unexpected guest errno %d\n", error.number);
        return 1;
    }
}
