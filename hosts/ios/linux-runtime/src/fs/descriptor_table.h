#pragma once
#include "base/fd.h"
#include "fs/virtual/node.h"
#include "fs/mount/namespace.h"
#include "linux/process/resource_limits.h"
#include <dirent.h>
#include <map>
#include <memory>
#include <string>
#include <vector>

namespace operit::arm64 {
namespace files { class PipeBuffer; }
namespace io_uring { class Ring; }
struct SocketState;
struct TerminalEndpoint;
class EventCounter;
class EventPoll;
class TimerFile;
class InotifyFile;
class InotifyOpenFile;
struct OpenDescription {
    int flags;
    uint64_t inode = 0;
    std::shared_ptr<SocketState> socket;
    std::shared_ptr<TerminalEndpoint> terminal;
    std::shared_ptr<files::PipeBuffer> pipe;
    std::shared_ptr<VirtualOpen> virtual_file;
    std::shared_ptr<mounts::Point> mount;
    bool read_only = false;
    bool scheduler_io = false;
    int standard_stream = -1;
    DIR *directory = nullptr;
    std::vector<std::string> mount_children;
    size_t mount_cursor = 0;
    std::shared_ptr<TimerFile> timer;
    std::shared_ptr<EventCounter> counter;
    std::shared_ptr<EventPoll> poll;
    // The host FD is only a readiness marker; this object owns event records.
    std::shared_ptr<InotifyFile> inotify;
    std::shared_ptr<InotifyOpenFile> inotify_open;
    std::shared_ptr<io_uring::Ring> io_uring;
    explicit OpenDescription(int f) : flags(f & ~(64 | 128 | 256 | 512 | 524288)) {}
    bool is_anon_inode() const;
    bool is_read_only() const;
    void require_writable() const;
    ~OpenDescription();
};
struct Descriptor {
    SharedFD host;
    std::string path;
    int descriptor_flags;
    std::shared_ptr<OpenDescription> description;
    Descriptor(FD fd, std::string p, int f);
    Descriptor(FD fd, const Descriptor &source, bool cloexec);
    Descriptor(const Descriptor &source, bool cloexec);
    // Metadata display only; anonymous files keep path empty for /proc reopening.
    std::string display_path() const;
};
class DescriptorTable {
    struct State {
        std::map<int, std::unique_ptr<Descriptor>> entries;
    };
    std::shared_ptr<State> state_ = std::make_shared<State>();
    // CLONE_FILES may share State between processes with independent rlimits.
    std::shared_ptr<process::ResourceLimits> limits_ = std::make_shared<process::ResourceLimits>();
    int available(int minimum) const;

  public:
    DescriptorTable() = default;
    DescriptorTable(const DescriptorTable &source);
    DescriptorTable &operator=(const DescriptorTable &source);
    DescriptorTable(DescriptorTable &&) noexcept = default;
    DescriptorTable &operator=(DescriptorTable &&) noexcept = default;
    void share_with(const DescriptorTable &source);
    const std::shared_ptr<process::ResourceLimits> &resource_limits() const {
        return limits_;
    }
    void bind_resource_limits(std::shared_ptr<process::ResourceLimits> limits);
    void unshare();
    void close_on_exec(const std::function<void(const Descriptor &)> &closing);
    void clear();
    Descriptor &get(uint64_t fd);
    std::vector<int> numbers() const;
    int insert(FD fd, const std::string &path, int flags, int minimum = 0);
    int duplicate(uint64_t source, int minimum = 0, bool cloexec = false);
    // F_DUPFD has EINVAL for an invalid minimum; dup uses EMFILE for exhaustion.
    int duplicate_minimum(uint64_t source, uint64_t minimum, bool cloexec = false);
    int duplicate_to(uint64_t source, uint64_t target, bool cloexec,
                     const std::function<void(const Descriptor &)> &before_close);
    void close(uint64_t fd);
    void standard(int fd, FD host);
};
} // namespace operit::arm64
