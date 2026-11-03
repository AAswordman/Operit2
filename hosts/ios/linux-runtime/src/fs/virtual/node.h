#pragma once
#include "base/fd.h"
#include <functional>
#include <memory>
#include <string>
#include <sys/stat.h>
#include <vector>

namespace operit::arm64 {
class TerminalManager;
// Filesystem-independent nodes: no host path or host descriptor is required.
struct VirtualNode {
    using Ptr = std::shared_ptr<VirtualNode>;
    enum class Kind { Directory, File, Symlink, Null, Zero, Full, Random, Terminal } kind;
    std::string identity;
    mode_t permissions;
    bool sequence = false; // Proc sequence files have no fixed size or SEEK_END.
    std::function<std::string()> contents;
    // Live files read only this range; they never enter an open-file snapshot.
    std::function<std::string(uint64_t, size_t)> read_range;
    std::function<size_t(const std::string &)> write_contents;
    std::function<std::string()> target;
    std::function<std::vector<std::string>()> names;
    std::function<Ptr(const std::string &)> lookup;
    // A proc fd magic link may refer to an anonymous host-backed stream.
    std::function<FD(int)> open_stream;
    std::function<struct stat()> stream_stat;
    int standard_stream = -1;
    VirtualNode(Kind type, std::string key, mode_t mode);
    struct stat stat() const;
    bool device() const;
};
struct VirtualOpen {
    VirtualNode::Ptr node;
    uint64_t offset = 0;
    std::string snapshot;
    bool captured = false;
    std::vector<std::string> directory_names;
    explicit VirtualOpen(VirtualNode::Ptr value) : node(std::move(value)) {}
    size_t read(void *data, size_t count, uint64_t at, bool positioned);
    size_t write(const void *data, size_t count, uint64_t at, bool positioned);
    uint64_t seek(int64_t offset, int whence);

  private:
    void capture();
};
VirtualNode::Ptr device_filesystem(const std::shared_ptr<TerminalManager> &terminals);
} // namespace operit::arm64
