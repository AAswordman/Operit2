#include "fs/virtual/node.h"
#include "base/random.h"
#include "base/support.h"
#include <cstring>
#include <algorithm>
#include <unistd.h>

namespace operit::arm64 {
VirtualNode::VirtualNode(Kind type, std::string key, mode_t mode)
    : kind(type), identity(std::move(key)), permissions(mode) {}
bool VirtualNode::device() const {
    return kind == Kind::Null || kind == Kind::Zero || kind == Kind::Full || kind == Kind::Random ||
           kind == Kind::Terminal;
}
struct stat VirtualNode::stat() const {
    if (stream_stat && kind != Kind::Symlink)
        return stream_stat();
    struct stat value{};
    uint64_t hash = 14695981039346656037ull;
    for (unsigned char c : identity)
        hash = (hash ^ c) * 1099511628211ull;
    value.st_ino = hash & 0x7fffffffffffffffull;
    value.st_dev = identity.compare(0, 5, "/proc") == 0 ? 0x7001 : 0x7002;
    value.st_mode = permissions | (kind == Kind::Directory ? S_IFDIR
                                   : kind == Kind::Symlink ? S_IFLNK
                                   : device()              ? S_IFCHR
                                                           : S_IFREG);
    value.st_nlink = kind == Kind::Directory ? 2 : 1;
    value.st_uid = ::getuid();
    value.st_gid = ::getgid();
    value.st_blksize = 4096;
    if (kind == Kind::Symlink && target)
        value.st_size = target().size();
    else if (kind == Kind::File && !sequence && contents)
        value.st_size = contents().size();
    if (device()) {
        unsigned minor = kind == Kind::Null          ? 3
                         : kind == Kind::Zero        ? 5
                         : kind == Kind::Full        ? 7
                         : identity == "/dev/random" ? 8
                                                     : 9;
        value.st_rdev = 256 | minor; // Linux major 1, minor < 256.
    }
    return value;
}
void VirtualOpen::capture() {
    if (!captured) {
        snapshot = node->contents ? node->contents() : std::string{};
        captured = true;
    }
}
size_t VirtualOpen::read(void *data, size_t count, uint64_t at, bool positioned) {
    if (node->kind == VirtualNode::Kind::Directory)
        guest_error(21);
    if (node->kind == VirtualNode::Kind::Symlink)
        guest_error(40);
    if (!count)
        return 0;
    if (node->kind == VirtualNode::Kind::Null)
        return 0;
    if (node->kind == VirtualNode::Kind::Random) {
        random_bytes(data, count);
        return count;
    }
    if (node->device()) {
        std::memset(data, 0, count);
        return count;
    }
    const uint64_t position = positioned ? at : offset;
    if (node->read_range) {
        auto bytes = node->read_range(position, count);
        count = std::min(count, bytes.size());
        if (count)
            std::memcpy(data, bytes.data(), count);
        if (!positioned)
            offset += count;
        return count;
    }
    // Rewinding a seq file, including pread at zero, samples fresh counters.
    // Subsequent partial reads retain this sample until another rewind.
    if (node->sequence && !position) {
        captured = false;
        snapshot.clear();
    }
    capture();
    count = position >= snapshot.size() ? 0 : std::min<uint64_t>(count, snapshot.size() - position);
    if (count)
        std::memcpy(data, snapshot.data() + position, count);
    if (!positioned)
        offset += count;
    return count;
}
size_t VirtualOpen::write(const void *data, size_t count, uint64_t, bool positioned) {
    if (node->write_contents) {
        size_t result = node->write_contents(std::string(static_cast<const char *>(data), count));
        if (!positioned)
            offset += result;
        captured = false;
        return result;
    }
    if (node->kind == VirtualNode::Kind::Full && count)
        guest_error(28);
    if (!node->device())
        guest_error(node->kind == VirtualNode::Kind::Directory ? 21 : 13);
    return count;
}
uint64_t VirtualOpen::seek(int64_t delta, int whence) {
    if (whence < 0 || whence > 2)
        guest_error(22);
    if (node->device())
        return offset = 0;
    uint64_t base = whence == 1 ? offset : 0;
    if (whence == 2) {
        if (node->sequence || node->kind == VirtualNode::Kind::Directory)
            guest_error(22);
        capture();
        base = snapshot.size();
    }
    if ((delta < 0 && uint64_t(-(delta + 1)) + 1 > base) ||
        (delta >= 0 && uint64_t(delta) > uint64_t(INT64_MAX) - base))
        guest_error(22);
    offset = delta < 0 ? base - (uint64_t(-(delta + 1)) + 1) : base + uint64_t(delta);
    if (!offset) {
        captured = false;
        snapshot.clear();
        directory_names.clear();
    }
    return offset;
}
} // namespace operit::arm64
