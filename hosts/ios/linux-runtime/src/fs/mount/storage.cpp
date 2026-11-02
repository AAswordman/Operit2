#include "fs/mount/storage.h"
#include "base/support.h"

#include "linux/error_mapping.h"
#include <cerrno>
#include <cstdlib>
#include <dirent.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>
#include <vector>

namespace operit::arm64::mounts {
namespace {
void remove_contents(int root) noexcept {
    int fd = ::openat(root, ".", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0)
        return;
    DIR *directory = ::fdopendir(fd);
    if (!directory) {
        ::close(fd);
        return;
    }
    while (auto *entry = ::readdir(directory)) {
        const std::string name = entry->d_name;
        if (name == "." || name == "..")
            continue;
        struct stat info{};
        if (::fstatat(root, name.c_str(), &info, AT_SYMLINK_NOFOLLOW) != 0)
            continue;
        if (S_ISDIR(info.st_mode)) {
            FD child(::openat(root, name.c_str(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
            if (child.get() >= 0) {
                // A guest may have made the temporary directory non-writable.
                ::fchmod(child.get(), 0700);
                remove_contents(child.get());
            }
            ::unlinkat(root, name.c_str(), AT_REMOVEDIR);
        } else
            ::unlinkat(root, name.c_str(), 0);
    }
    ::closedir(directory);
}
} // namespace
TemporaryTree::TemporaryTree(std::string path, FD root) : path_(std::move(path)), root_(std::move(root)) {}
TemporaryTree::~TemporaryTree() {
    if (root_.get() >= 0) {
        ::fchmod(root_.get(), 0700);
        remove_contents(root_.get());
    }
    ::rmdir(path_.c_str());
}
std::shared_ptr<TemporaryTree> TemporaryTree::create(mode_t mode) {
    const char *configured = std::getenv("TMPDIR");
    std::string prefix = configured && *configured ? configured : "/tmp";
    if (prefix.back() != '/')
        prefix += '/';
    std::string pattern = prefix + "operit-mount-XXXXXX";
    std::vector<char> path(pattern.begin(), pattern.end());
    path.push_back('\0');
    if (!::mkdtemp(path.data()))
        guest_error(linux_errno(errno));
    FD root(::open(path.data(), O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC));
    if (root.get() < 0) {
        const int error = errno;
        ::rmdir(path.data());
        guest_error(linux_errno(error));
    }
    auto result = std::shared_ptr<TemporaryTree>(new TemporaryTree(path.data(), std::move(root)));
    check_host(::fchmod(result->descriptor(), mode));
    return result;
}
} // namespace operit::arm64::mounts
