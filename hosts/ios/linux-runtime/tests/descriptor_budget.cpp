#include "base/support.h"
#include "fs/filesystem.h"
#include "fs/descriptor_table.h"
#include "native_descriptor_budget.h"
#include <cstdio>
#include <cstdlib>
#include <fcntl.h>
#include <filesystem>
#include <fstream>
#include <stdexcept>

using namespace operit::arm64;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
struct Temporary {
    std::string path;
    Temporary() {
        char pattern[] = "/tmp/operit-fd-budget-XXXXXX";
        auto *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        path = created;
    }
    ~Temporary() { std::filesystem::remove_all(path); }
};
void descriptor_inheritance_without_native_duplicates() {
    Temporary temp;
    std::ofstream(temp.path + "/message") << "shared";
    DescriptorTable parent;
    const int original = parent.insert(FD(::open((temp.path + "/message").c_str(), O_RDONLY)),
                                       "/message", 0);
    const int native = parent.get(original).host.get();
    require(native >= 0, "open test file");
    const auto initial = NativeDescriptorBudget::count();
    {
        NativeDescriptorBudget budget(0);
        const int duplicate = parent.duplicate(original, 10, true);
        const int target = parent.duplicate_to(original, 20, false, [](const Descriptor &) {});
        DescriptorTable child(parent);
        DescriptorTable peer;
        peer.share_with(child);
        child.unshare();
        child.get(duplicate).descriptor_flags = 0;
        require(parent.get(duplicate).descriptor_flags == 1 &&
                    peer.get(duplicate).descriptor_flags == 1,
                "fork/unshare shared descriptor flags");
        child.get(original).descriptor_flags = 1;
        child.close_on_exec([](const Descriptor &) {});
        require(child.numbers() == std::vector<int>({duplicate, target}),
                "close-on-exec did not remove only the child's flagged descriptor");
        parent.close(original);
        parent.close(duplicate);
        parent.close(target);
        peer.clear();
        require(NativeDescriptorBudget::count() == initial,
                "dup/fork/unshare multiplied native descriptors");
        char byte = 0;
        require(::read(child.get(duplicate).host.get(), &byte, 1) == 1 && byte == 's',
                "closing parent descriptors invalidated the child");
        require(::read(child.get(target).host.get(), &byte, 1) == 1 && byte == 'h',
                "duplicated descriptors lost their shared offset");
        child.clear();
        require(::fcntl(native, F_GETFD) == -1 && errno == EBADF,
                "last guest close retained the native descriptor");
    }
}
const std::string container =
    "/var/mobile/Containers/Data/Application/77122FF4-test/Library/Application Support/Operit/workspaces/default";
void ordinary_walks_with_constant_budget() {
    Temporary temp;
    std::string deep;
    for (int i = 0; i < 120; ++i)
        deep += "/d";
    std::filesystem::create_directories(temp.path + deep);
    std::ofstream(temp.path + deep + "/message") << "ok";
    Filesystem fs(temp.path.c_str());
    NativeDescriptorBudget budget(8);
    for (int i = 0; i < 20; ++i) {
        auto file = fs.resolve(deep + "/message").open(O_RDONLY);
        char bytes[2]{};
        require(::read(file.get(), bytes, 2) == 2 && std::string(bytes, 2) == "ok",
                "ordinary path depth consumed native descriptors");
    }
}
void repeated_deep_lookups() {
    Temporary temp;
    std::filesystem::create_directories(temp.path + container);
    std::ofstream(temp.path + container + "/message") << "ok";
    Filesystem fs(temp.path.c_str());
    NativeDescriptorBudget budget(36);
    Path cwd;
    for (int i = 0; i < 100; ++i) {
        auto resolved = fs.resolve(container);
        cwd = fs.resolve_at(".", resolved);
        auto file = fs.resolve_at("message", cwd).open(O_RDONLY);
        char bytes[2]{};
        require(::read(file.get(), bytes, sizeof(bytes)) == 2 && std::string(bytes, 2) == "ok",
                "deep cwd/file lookup failed under native descriptor pressure");
        auto root = fs.resolve_at("../../../../../../../../../../../../", cwd);
        require(root.guest == "/", "deep parent traversal did not reach guest root");
    }
}
void mounted_parent_identity_with_small_budget() {
    Temporary temp;
    std::filesystem::create_directories(temp.path + "/root" + container);
    std::filesystem::create_directory(temp.path + "/source");
    std::ofstream(temp.path + "/source/message") << "bound";
    Filesystem fs((temp.path + "/root").c_str());
    NativeDescriptorBudget budget(36);
    // Several mounts at a deep target must retain only their actual attachment
    // directory, not independent descriptor chains for all its ancestors.
    for (int i = 0; i < 6; ++i)
        fs.bind(temp.path + "/source", container + "/mount" + std::to_string(i), false);
    // Rename immediately after bind(), before the next resolve. The attachment
    // must already pin its parent inode, even if the old spelling is reused.
    std::filesystem::rename(temp.path + "/root" + container,
                            temp.path + "/root" + container + "-moved");
    std::filesystem::create_directory(temp.path + "/root" + container);
    Filesystem peer(fs);
    for (int i = 0; i < 6; ++i) {
        const auto name = "/mount" + std::to_string(i);
        auto file = peer.resolve(container + "-moved" + name + "/message").open(O_RDONLY);
        char bytes[5]{};
        require(::read(file.get(), bytes, sizeof(bytes)) == 5 && std::string(bytes, 5) == "bound",
                "mount lost its renamed attachment inode");
        auto hidden = peer.resolve(container + name);
        try {
            hidden.stat();
        } catch (const GuestError &error) {
            require(error.number == 2, "replacement mount target errno");
            continue;
        }
        throw std::runtime_error("mount followed the replaced parent pathname");
    }
}
void recursive_mount_attachments() {
    Temporary temp;
    std::filesystem::create_directories(temp.path + "/root/a/sub/deep");
    std::filesystem::create_directory(temp.path + "/root/copy");
    std::filesystem::create_directory(temp.path + "/source");
    std::ofstream(temp.path + "/source/message") << "bound";
    std::ofstream(temp.path + "/root/a/sub/deep/parent") << "parent";
    auto original = std::make_unique<Filesystem>((temp.path + "/root").c_str());
    original->bind(temp.path + "/source", "/a/sub/deep/inner", false);
    original->mount("/a", "/copy/.", "", mounts::bind | mounts::recursive);
    Filesystem fs(*original);
    fs.unshare_mounts();
    original.reset(); // Namespace copies must own their attachment lifetimes.
    std::filesystem::rename(temp.path + "/root/a/sub", temp.path + "/root/a/renamed");
    std::filesystem::create_directories(temp.path + "/root/a/sub/deep");
    NativeDescriptorBudget budget(36);
    auto directory = fs.resolve("/copy/renamed/deep/inner");
    auto file = fs.resolve_at("message", directory).open(O_RDONLY);
    char bytes[6]{};
    require(::read(file.get(), bytes, 5) == 5 && std::string(bytes, 5) == "bound",
            "recursive mount did not keep its attachment inode");
    auto parent = fs.resolve_at("../parent", directory).open(O_RDONLY);
    require(::read(parent.get(), bytes, 6) == 6 && std::string(bytes, 6) == "parent",
            "recursive mount parent crossed into the wrong namespace");
    fs.unmount("/copy", mounts::detach);
    require(fs.resolve("/copy").mount_id == 0, "recursive mount detach did not remove attachment");
}
} // namespace
int main() {
    try {
        descriptor_inheritance_without_native_duplicates();
        ordinary_walks_with_constant_budget();
        repeated_deep_lookups();
        mounted_parent_identity_with_small_budget();
        recursive_mount_attachments();
        std::puts("descriptor budget: deep cwd, repeated I/O, mount identity and fork sharing passed");
        return 0;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "descriptor budget: unexpected guest errno %d\n", error.number);
    } catch (const std::exception &error) {
        std::fprintf(stderr, "descriptor budget: %s\n", error.what());
    }
    return 1;
}
