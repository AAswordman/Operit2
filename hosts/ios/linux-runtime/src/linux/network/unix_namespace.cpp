#include "linux/network/unix_namespace.h"
#include "linux/network/socket.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <cstdlib>
#include <fcntl.h>

namespace operit::arm64 {
NetworkNamespace::~NetworkNamespace() {
    // SCM_RIGHTS queues can contain their own receiving socket. Break the
    // descriptor cycles only when the whole guest network namespace ends.
    for (auto &weak : sockets)
        if (auto socket = weak.lock())
            socket->pending_rights.clear();
}
void UnixNamespace::prepare() {
    if (!temporary_.empty())
        return;
    const char *directory = std::getenv("TMPDIR");
    std::string pattern = std::string(directory ? directory : "/tmp/") + "/ort-XXXXXX";
    std::vector<char> value(pattern.begin(), pattern.end());
    value.push_back(0);
    if (!::mkdtemp(value.data()))
        guest_error(linux_errno(errno));
    temporary_ = value.data();
}
UnixNamespace::~UnixNamespace() {
    for (const auto &item : aliases_)
        ::unlink(item.second.c_str());
    for (const auto &item : abstract_)
        ::unlink(item.second.c_str());
    if (!temporary_.empty())
        ::rmdir(temporary_.c_str());
}
std::string UnixNamespace::pathname(const Path &path) {
    char parent[max_path];
#ifdef __APPLE__
    check_host(::fcntl(path.parent.get(), F_GETPATH, parent));
#else
    auto link = "/proc/self/fd/" + std::to_string(path.parent.get());
    ssize_t count = ::readlink(link.c_str(), parent, sizeof(parent) - 1);
    check_host(count);
    parent[count] = 0;
#endif
    prepare();
    std::string key(parent);
    auto found = aliases_.find(key);
    if (found == aliases_.end()) {
        std::string link = temporary_ + "/d" + std::to_string(aliases_.size());
        check_host(::symlink(parent, link.c_str()));
        found = aliases_.emplace(key, link).first;
    }
    return found->second + "/" + path.name;
}
std::string UnixNamespace::abstract(const std::string &name) {
    prepare();
    auto found = abstract_.find(name);
    if (found == abstract_.end())
        found = abstract_.emplace(name, temporary_ + "/s" + std::to_string(abstract_.size())).first;
    return found->second;
}
} // namespace operit::arm64
