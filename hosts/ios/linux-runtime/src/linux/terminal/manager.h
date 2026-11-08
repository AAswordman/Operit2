#pragma once
#include "base/fd.h"
#include "fs/virtual/node.h"
#include "linux/terminal/termios.h"
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <map>
#include <memory>
#include <optional>
#include <string>
#include <sys/ioctl.h>
#include <vector>

namespace operit::arm64 {
struct TerminalState;
struct TerminalEvents;
struct TerminalEndpoint {
    std::shared_ptr<TerminalState> state;
    FD anchor;
    bool master = false, readable = false;
    TerminalEndpoint(std::shared_ptr<TerminalState> state, FD anchor, bool master, int guest_flags);
    ~TerminalEndpoint();
    TerminalEndpoint(const TerminalEndpoint &) = delete;
    TerminalEndpoint &operator=(const TerminalEndpoint &) = delete;
};
struct TerminalOpen {
    FD host;
    std::shared_ptr<TerminalEndpoint> endpoint;
    std::string path;
};
struct TerminalProcess {
    uint32_t pid = 0, session = 0, group = 0;
    std::optional<uint32_t> parent = std::nullopt;
};
struct TerminalSignal {
    uint32_t group = 0;
    unsigned signal = 0;
};
struct TerminalHooks {
    std::function<TerminalProcess(uint32_t)> process;
    std::function<bool(uint32_t, uint32_t)> group_in_session;
    std::function<void(uint32_t, unsigned)> signal_group;
    std::function<bool(uint32_t, unsigned)> blocked_or_ignored;
    std::function<bool(uint32_t)> orphaned_group;
    // Whether this guest PID has CAP_SYS_ADMIN, including write-only acquisition.
    std::function<bool(uint32_t)> can_steal;
    // nullopt distinguishes a missing group from a group in another session.
    std::function<std::optional<uint32_t>(uint32_t)> group_session;
};
// Native PTYs provide the line discipline when allowed by the host sandbox.
// A socket-backed implementation handles restricted hosts. Session ownership,
// devpts IDs and foreground groups belong solely to this guest instance.
class TerminalManager {
    std::map<uint32_t, std::weak_ptr<TerminalState>> terminals_;
    std::map<uint32_t, std::weak_ptr<TerminalState>> controlling_;
    mutable std::map<uint32_t, TerminalProcess> known_processes_;
    std::map<uint32_t, std::function<std::optional<int64_t>()>> interrupted_reads_;
    TerminalHooks hooks_;
    std::shared_ptr<TerminalEvents> events_;
    uint32_t next_number() const;
    std::shared_ptr<TerminalState> find(uint32_t number) const;
    TerminalProcess process(uint32_t pid) const;
    std::vector<TerminalProcess> known_processes() const;
    std::optional<uint32_t> group_session(uint32_t group, uint32_t expected_session) const;
    bool controls(const TerminalEndpoint &endpoint, const TerminalProcess &process) const;
    void clear_controlling(const std::shared_ptr<TerminalState> &state);
    bool orphaned(uint32_t group) const;
    FD control_descriptor(const TerminalEndpoint &endpoint) const;
    void signal(uint32_t group, unsigned number);
    VirtualNode::Ptr slave_node(uint32_t number);

  public:
    struct ReadState {
        size_t previous_available = 0;
        std::optional<std::chrono::steady_clock::time_point> deadline;
        uint64_t settings_generation = 0;
        bool initialized = false, interrupted = false;
    };
    enum class ReadStatus { Waiting, Empty, Ready };
    TerminalManager();
    void set_hooks(TerminalHooks hooks);
    std::vector<TerminalSignal> take_signals();
    TerminalOpen open_multiplexer(int guest_flags);
    TerminalOpen open_slave(uint32_t number, int guest_flags);
    TerminalOpen open_controlling(uint32_t pid, int guest_flags);
    std::optional<TerminalOpen> open_node(const VirtualNode &node, int guest_flags, uint32_t pid);
    // Allocate an already-unlocked pair for embedding APIs, with optional real
    // termios and winsize initialization. Both endpoints are native-nonblocking.
    std::pair<TerminalOpen, TerminalOpen> open_pair(int guest_flags = 2,
                                                    const terminal::Attributes *attributes = nullptr,
                                                    const struct winsize *size = nullptr);
    VirtualNode::Ptr multiplexer_node();
    VirtualNode::Ptr controlling_node();
    VirtualNode::Ptr slave_filesystem();
    struct stat stat(const TerminalEndpoint &endpoint) const;
    uint32_t device(const TerminalEndpoint &endpoint) const;
    uint32_t number(const TerminalEndpoint &endpoint) const;
    bool locked(const TerminalEndpoint &endpoint) const;
    void lock(const TerminalEndpoint &endpoint, bool enabled);
    bool exclusive(const TerminalEndpoint &endpoint) const;
    void exclusive(const TerminalEndpoint &endpoint, bool enabled);
    bool packet(const TerminalEndpoint &endpoint) const;
    void packet(const TerminalEndpoint &endpoint, bool enabled);
    terminal::Attributes attributes(const TerminalEndpoint &endpoint) const;
    void attributes(const TerminalEndpoint &endpoint, const terminal::Attributes &value, unsigned action = 0);
    void flush(const TerminalEndpoint &endpoint, unsigned action);
    void flow(const TerminalEndpoint &endpoint, unsigned action);
    static bool software(const TerminalEndpoint &endpoint);
    int input_queued(const TerminalEndpoint &endpoint) const;
    // Both backends return byte counts or negative Linux errno values.
    int64_t read(const TerminalEndpoint &endpoint, int fd, void *bytes, size_t length);
    int64_t write(const TerminalEndpoint &endpoint, int fd, const void *bytes, size_t length);
    void opened(const TerminalEndpoint &endpoint, uint32_t pid, bool no_controlling);
    void acquire(const TerminalEndpoint &endpoint, uint32_t pid, bool steal);
    void detach(const TerminalEndpoint &endpoint, uint32_t pid);
    uint32_t foreground_group(const TerminalEndpoint &endpoint, uint32_t pid) const;
    void foreground_group(const TerminalEndpoint &endpoint, uint32_t pid, uint32_t group);
    uint32_t session(const TerminalEndpoint &endpoint, uint32_t pid) const;
    void session_created(uint32_t pid, uint32_t previous_session);
    void process_forked(uint32_t child, uint32_t parent);
    void process_exited(uint32_t pid);
    void window_changed(const TerminalEndpoint &endpoint);
    struct winsize window_size(const TerminalEndpoint &endpoint) const;
    int output_queued(const TerminalEndpoint &endpoint) const;
    void resize(const TerminalEndpoint &endpoint, const struct winsize &size);
    void signal_foreground(const TerminalEndpoint &endpoint, unsigned signal);
    void send_signal(const TerminalEndpoint &endpoint, unsigned signal);
    void inject(const TerminalEndpoint &endpoint, uint32_t pid, uint8_t byte);
    // I/O hooks: before_io enforces guest background job rules; written_input
    // delivers ISIG characters to the guest foreground group after a successful
    // master write. read_result translates EOF/packet headers to Linux.
    void before_io(const TerminalEndpoint &endpoint, uint32_t pid, bool writing);
    void before_control(const TerminalEndpoint &endpoint, uint32_t pid);
    void written_input(const TerminalEndpoint &endpoint, const void *bytes, size_t size);
    void register_read_interruption(uint32_t tid, std::function<std::optional<int64_t>()> complete);
    void cancel_read(uint32_t tid);
    // Call in this TID's scheduler scope before replacing an interrupted read with EINTR.
    std::optional<int64_t> interrupt_read(uint32_t tid);
    ReadStatus read_status(const TerminalEndpoint &endpoint, size_t capacity, bool nonblocking,
                                  bool readable, bool hungup, ReadState &wait) const;
    int64_t read_result(const TerminalEndpoint &endpoint, void *bytes, int64_t result) const;
};
} // namespace operit::arm64
