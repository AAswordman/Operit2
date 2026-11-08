#include "linux/terminal/manager.h"
#include "linux/terminal/software_pty.h"
#include "base/host_io.h"
#include "base/support.h"
#include "linux/error_mapping.h"
#include <algorithm>
#include <cerrno>
#include <cstdlib>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <sys/stat.h>
#include <unistd.h>

namespace operit::arm64 {
struct TerminalEvents {
    std::function<void(uint32_t, unsigned)> deliver;
    std::vector<TerminalSignal> pending;
    void send(uint32_t group, unsigned number) {
        if (!group)
            return;
        if (deliver) {
            try {
                deliver(group, number);
                return;
            } catch (const GuestError &error) {
                if (error.number == 3)
                    return; // The last process in this group has exited.
            }
        }
        pending.push_back({group, number});
    }
};
struct TerminalState {
    uint32_t number = 0, session = 0, foreground = 0;
    bool locked = true, packet = false, master_alive = true, slave_ever_opened = false;
    std::array<bool, 2> exclusive{};
    bool literal_next = false;
    size_t slave_opens = 0;
    uint64_t settings_generation = 1;
    std::string native_slave;
    std::shared_ptr<SoftwarePty> software;
    std::weak_ptr<TerminalEndpoint> master;
    std::weak_ptr<TerminalEvents> events;
    // Linux N_TTY leaves these three character slots uninterpreted.
    std::array<uint8_t, 3> unused_characters{};
};
namespace {
constexpr uint32_t maximum_terminals = 1u << 20;
FD duplicate(int fd) {
    FD result(::fcntl(fd, F_DUPFD_CLOEXEC, 0));
    check_host(result.get());
    return result;
}
void nonblocking(int fd) {
    int flags = ::fcntl(fd, F_GETFL);
    check_host(flags);
    check_host(::fcntl(fd, F_SETFL, flags | O_NONBLOCK));
    int descriptor = ::fcntl(fd, F_GETFD);
    check_host(descriptor);
    check_host(::fcntl(fd, F_SETFD, descriptor | FD_CLOEXEC));
#ifdef F_SETNOSIGPIPE
    check_host(::fcntl(fd, F_SETNOSIGPIPE, 1));
#endif
}
uint64_t device_number(unsigned major, unsigned minor) {
    return uint64_t(minor & 0xff) | (uint64_t(major & 0xfff) << 8) | (uint64_t(minor & ~0xffu) << 12) |
           (uint64_t(major & ~0xfffu) << 32);
}
struct stat terminal_stat(uint32_t number, bool master) {
    struct stat value{};
    value.st_dev = 0x7004;
    value.st_ino = master ? 2 : uint64_t(number) + 3;
    value.st_mode = S_IFCHR | (master ? 0666 : 0620);
    value.st_nlink = 1;
    value.st_uid = ::getuid();
    value.st_gid = ::getgid();
    value.st_rdev = device_number(master ? 5 : 136, master ? 2 : number);
    value.st_blksize = 1024;
    return value;
}
std::optional<uint32_t> parse_number(const std::string &name) {
    if (name.empty() || (name.size() > 1 && name[0] == '0'))
        return {};
    uint32_t value = 0;
    for (char byte : name) {
        if (byte < '0' || byte > '9' || value > (maximum_terminals - 1) / 10)
            return {};
        value = value * 10 + unsigned(byte - '0');
        if (value >= maximum_terminals)
            return {};
    }
    return value;
}
terminal::Attributes initial_attributes() {
    terminal::Attributes value;
    value.input = 0x500;  // ICRNL | IXON
    value.output = 5;     // OPOST | ONLCR
    value.control = 0xbf; // B38400 | CS8 | CREAD
    value.local = 0x8a3b; // ISIG, ICANON, ECHO*, IEXTEN
    value.characters = {3, 28, 127, 21, 4, 0, 1, 0, 17, 19, 26, 0, 18, 15, 23, 22, 0, 0, 0};
    value.input_speed = value.output_speed = 38400;
    return value;
}
void require_master(const TerminalEndpoint &endpoint) {
    if (!endpoint.master)
        guest_error(25);
}
} // namespace
TerminalEndpoint::TerminalEndpoint(std::shared_ptr<TerminalState> value, FD fd, bool is_master,
                                   int guest_flags)
    : state(std::move(value)), anchor(std::move(fd)), master(is_master), readable((guest_flags & 3) != 1) {
    if (!master) {
        ++state->slave_opens;
        state->slave_ever_opened = true;
    }
}
TerminalEndpoint::~TerminalEndpoint() {
    if (!master) {
        --state->slave_opens;
        return;
    }
    state->master_alive = false;
    if (state->software)
        state->software->close_master();
    auto foreground = state->foreground;
    state->session = state->foreground = 0;
    if (auto events = state->events.lock()) {
        events->send(foreground, 1);  // SIGHUP
        events->send(foreground, 18); // SIGCONT
    }
}
TerminalManager::TerminalManager() : events_(std::make_shared<TerminalEvents>()) {}
void TerminalManager::set_hooks(TerminalHooks hooks) {
    hooks_ = std::move(hooks);
    events_->deliver = hooks_.signal_group;
    if (events_->deliver) {
        auto pending = std::move(events_->pending);
        events_->pending.clear();
        for (auto event : pending)
            events_->send(event.group, event.signal);
    }
}
std::vector<TerminalSignal> TerminalManager::take_signals() {
    auto result = std::move(events_->pending);
    events_->pending.clear();
    return result;
}
void TerminalManager::signal(uint32_t group, unsigned number) {
    events_->send(group, number);
}
TerminalProcess TerminalManager::process(uint32_t pid) const {
    const auto known = known_processes_.find(pid);
    if (!hooks_.process) {
        if (known != known_processes_.end() && known->second.session && known->second.group)
            return known->second;
        guest_error(38);
    }
    auto result = hooks_.process(pid);
    if (!result.pid || result.pid != pid || !result.session || !result.group)
        guest_error(3);
    if (!result.parent && known != known_processes_.end())
        result.parent = known->second.parent;
    known_processes_[pid] = result;
    return result;
}
std::vector<TerminalProcess> TerminalManager::known_processes() const {
    std::vector<uint32_t> ids;
    for (const auto &[pid, identity] : known_processes_) {
        if (hooks_.process || (identity.session && identity.group))
            ids.push_back(pid);
    }
    std::vector<TerminalProcess> result;
    result.reserve(ids.size());
    for (auto pid : ids) {
        try {
            result.push_back(process(pid));
        } catch (const GuestError &error) {
            if (error.number != 3)
                throw;
            known_processes_.erase(pid);
        }
    }
    return result;
}
std::optional<uint32_t> TerminalManager::group_session(uint32_t group, uint32_t expected_session) const {
    if (!group)
        return {};
    if (hooks_.group_session)
        return hooks_.group_session(group);
    if (hooks_.group_in_session && hooks_.group_in_session(group, expected_session))
        return expected_session;
    const auto identities = known_processes();
    for (const auto &identity : identities) {
        if (identity.group == group &&
            (!hooks_.group_in_session || hooks_.group_in_session(group, identity.session)))
            return identity.session;
    }
    if (hooks_.group_in_session)
        for (const auto &identity : identities)
            if (hooks_.group_in_session(group, identity.session))
                return identity.session;
    return {};
}
bool TerminalManager::controls(const TerminalEndpoint &endpoint, const TerminalProcess &current) const {
    if (!endpoint.state->master_alive || !endpoint.state->session ||
        endpoint.state->session != current.session)
        return false;
    const auto found = controlling_.find(current.pid);
    return found != controlling_.end() && found->second.lock() == endpoint.state;
}
void TerminalManager::clear_controlling(const std::shared_ptr<TerminalState> &state) {
    for (auto it = controlling_.begin(); it != controlling_.end();) {
        const auto terminal = it->second.lock();
        if (!terminal || terminal == state)
            it = controlling_.erase(it);
        else
            ++it;
    }
}
bool TerminalManager::orphaned(uint32_t group) const {
    if (hooks_.orphaned_group)
        return hooks_.orphaned_group(group);
    for (const auto &member : known_processes()) {
        if (member.group != group || !member.parent || *member.parent <= 1)
            continue;
        if (!hooks_.process && known_processes_.find(*member.parent) == known_processes_.end())
            continue;
        try {
            const auto parent = process(*member.parent);
            if (parent.group != group && parent.session == member.session)
                return false;
        } catch (const GuestError &error) {
            if (error.number != 3)
                throw;
        }
    }
    return true;
}
FD TerminalManager::control_descriptor(const TerminalEndpoint &endpoint) const {
    if (!endpoint.master || endpoint.state->slave_opens)
        return duplicate(endpoint.anchor.get());
    if (!endpoint.state->master_alive)
        guest_error(5);
    FD slave(::open(endpoint.state->native_slave.c_str(), O_RDWR | O_NOCTTY | O_NONBLOCK | O_CLOEXEC));
    check_host(slave.get());
    return slave;
}
uint32_t TerminalManager::next_number() const {
    for (uint32_t number = 0; number < maximum_terminals; ++number) {
        auto found = terminals_.find(number);
        if (found == terminals_.end())
            return number;
        auto state = found->second.lock();
        if (!state || !state->master_alive)
            return number;
    }
    guest_error(28);
}
std::shared_ptr<TerminalState> TerminalManager::find(uint32_t number) const {
    auto found = terminals_.find(number);
    if (found != terminals_.end())
        if (auto state = found->second.lock(); state && state->master_alive)
            return state;
    guest_error(2);
}
TerminalOpen TerminalManager::open_multiplexer(int guest_flags) {
    if ((guest_flags & 3) == 3)
        guest_error(22);
    auto state = std::make_shared<TerminalState>();
    state->number = next_number();
    state->events = events_;
    FD host;
    try {
        FD native(::posix_openpt(O_RDWR | O_NOCTTY | O_NONBLOCK));
        check_host(native.get());
        nonblocking(native.get());
        check_host(::grantpt(native.get()));
        check_host(::unlockpt(native.get()));
        char name[PATH_MAX]{};
        int result = ::ptsname_r(native.get(), name, sizeof(name));
        if (result)
            guest_error(linux_errno(result > 0 ? result : errno));
        FD slave(::open(name, O_RDWR | O_NOCTTY | O_NONBLOCK | O_CLOEXEC));
        check_host(slave.get());
        terminal::set_attributes(slave.get(), initial_attributes());
        state->native_slave = name;
        host = std::move(native);
    } catch (const GuestError &error) {
        // iOS application sandboxes can deny the host PTY device or its setup
        // ioctls. Guest /dev/ptmx remains usable without those host privileges.
        if (error.number != 1 && error.number != 13)
            throw;
        state->software = std::make_shared<SoftwarePty>(initial_attributes());
        host = duplicate(state->software->descriptor(true));
    }
    auto endpoint = std::make_shared<TerminalEndpoint>(state, duplicate(host.get()), true, guest_flags);
    state->master = endpoint;
    terminals_[state->number] = state;
    return {std::move(host), std::move(endpoint), "/dev/ptmx"};
}
TerminalOpen TerminalManager::open_slave(uint32_t number, int guest_flags) {
    if ((guest_flags & 3) == 3)
        guest_error(22);
    auto state = find(number);
    if (state->locked)
        guest_error(5);
    if (state->exclusive[1])
        guest_error(16);
    FD host = state->software ? duplicate(state->software->descriptor(false))
                              : FD(::open(state->native_slave.c_str(),
                                          (guest_flags & 3) | O_NOCTTY | O_NONBLOCK | O_CLOEXEC));
    check_host(host.get());
    nonblocking(host.get());
    auto endpoint = std::make_shared<TerminalEndpoint>(state, duplicate(host.get()), false, guest_flags);
    return {std::move(host), std::move(endpoint), "/dev/pts/" + std::to_string(number)};
}
TerminalOpen TerminalManager::open_controlling(uint32_t pid, int guest_flags) {
    auto current = process(pid);
    auto found = controlling_.find(pid);
    if (found == controlling_.end())
        guest_error(6);
    auto state = found->second.lock();
    if (!state || !state->master_alive || state->session != current.session)
        guest_error(6);
    return open_slave(state->number, guest_flags);
}
std::optional<TerminalOpen> TerminalManager::open_node(const VirtualNode &node, int flags, uint32_t pid) {
    if (node.kind != VirtualNode::Kind::Terminal)
        return {};
    if (flags & 65536)
        guest_error(20);
    TerminalOpen opened_terminal;
    if (node.identity == "/dev/ptmx")
        opened_terminal = open_multiplexer(flags);
    else if (node.identity == "/dev/tty")
        opened_terminal = open_controlling(pid, flags);
    else {
        constexpr char prefix[] = "/dev/pts/";
        if (node.identity.compare(0, sizeof(prefix) - 1, prefix) != 0)
            guest_error(19);
        auto number = parse_number(node.identity.substr(sizeof(prefix) - 1));
        if (!number)
            guest_error(2);
        opened_terminal = open_slave(*number, flags);
    }
    opened(*opened_terminal.endpoint, pid, flags & 256);
    return std::optional<TerminalOpen>(std::move(opened_terminal));
}
std::pair<TerminalOpen, TerminalOpen> TerminalManager::open_pair(int guest_flags,
                                                                 const terminal::Attributes *attributes,
                                                                 const struct winsize *size) {
    auto master = open_multiplexer(guest_flags);
    lock(*master.endpoint, false);
    auto slave = open_slave(number(*master.endpoint), guest_flags);
    if (attributes)
        this->attributes(*slave.endpoint, *attributes);
    if (size)
        resize(*slave.endpoint, *size);
    return {std::move(master), std::move(slave)};
}
VirtualNode::Ptr TerminalManager::multiplexer_node() {
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::Terminal, "/dev/ptmx", 0666);
    node->stream_stat = [] { return terminal_stat(0, true); };
    return node;
}
VirtualNode::Ptr TerminalManager::controlling_node() {
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::Terminal, "/dev/tty", 0666);
    node->stream_stat = [] {
        auto value = terminal_stat(0, true);
        value.st_dev = 0x7002;
        value.st_ino = 1;
        value.st_rdev = device_number(5, 0);
        return value;
    };
    return node;
}
VirtualNode::Ptr TerminalManager::slave_node(uint32_t number) {
    auto state = find(number);
    auto node = std::make_shared<VirtualNode>(VirtualNode::Kind::Terminal,
                                              "/dev/pts/" + std::to_string(number), 0620);
    node->stream_stat = [state] {
        if (!state->master_alive)
            guest_error(2);
        return terminal_stat(state->number, false);
    };
    return node;
}
VirtualNode::Ptr TerminalManager::slave_filesystem() {
    auto root = std::make_shared<VirtualNode>(VirtualNode::Kind::Directory, "/dev/pts", 0755);
    root->names = [this] {
        std::vector<std::string> names{"ptmx"};
        for (auto &[number, weak] : terminals_)
            if (auto state = weak.lock(); state && state->master_alive)
                names.push_back(std::to_string(number));
        return names;
    };
    root->lookup = [this](const std::string &name) -> VirtualNode::Ptr {
        if (name == "ptmx")
            return multiplexer_node();
        auto number = parse_number(name);
        if (!number)
            return {};
        try {
            return slave_node(*number);
        } catch (const GuestError &error) {
            if (error.number == 2)
                return {};
            throw;
        }
    };
    return root;
}
struct stat TerminalManager::stat(const TerminalEndpoint &endpoint) const {
    return terminal_stat(endpoint.state->number, endpoint.master);
}
uint32_t TerminalManager::device(const TerminalEndpoint &endpoint) const {
    return uint32_t(device_number(136, endpoint.state->number));
}
uint32_t TerminalManager::number(const TerminalEndpoint &endpoint) const {
    require_master(endpoint);
    return endpoint.state->number;
}
bool TerminalManager::locked(const TerminalEndpoint &endpoint) const {
    require_master(endpoint);
    return endpoint.state->locked;
}
void TerminalManager::lock(const TerminalEndpoint &endpoint, bool enabled) {
    require_master(endpoint);
    endpoint.state->locked = enabled;
}
bool TerminalManager::exclusive(const TerminalEndpoint &endpoint) const {
    return endpoint.state->exclusive[endpoint.master ? 0 : 1];
}
void TerminalManager::exclusive(const TerminalEndpoint &endpoint, bool enabled) {
    if (!endpoint.master && !endpoint.state->software)
        terminal::exclusive(endpoint.anchor.get(), enabled);
    endpoint.state->exclusive[endpoint.master ? 0 : 1] = enabled;
}
bool TerminalManager::packet(const TerminalEndpoint &endpoint) const {
    require_master(endpoint);
    return endpoint.state->packet;
}
void TerminalManager::packet(const TerminalEndpoint &endpoint, bool enabled) {
    require_master(endpoint);
    if (endpoint.state->software)
        endpoint.state->software->packet(enabled);
    else
        terminal::packet_mode(endpoint.anchor.get(), enabled);
    endpoint.state->packet = enabled;
}
terminal::Attributes TerminalManager::attributes(const TerminalEndpoint &endpoint) const {
    if (endpoint.state->software)
        return endpoint.state->software->attributes();
    const auto fd = control_descriptor(endpoint);
    auto value = terminal::attributes(fd.get());
    value.characters[7] = endpoint.state->unused_characters[0];
    value.characters[17] = endpoint.state->unused_characters[1];
    value.characters[18] = endpoint.state->unused_characters[2];
    return value;
}
void TerminalManager::attributes(const TerminalEndpoint &endpoint, const terminal::Attributes &value,
                                 unsigned action) {
    const auto previous = attributes(endpoint);
    if (endpoint.state->software)
        endpoint.state->software->attributes(value, action);
    else {
        const auto fd = control_descriptor(endpoint);
        terminal::set_attributes(fd.get(), value, action);
    }
    endpoint.state->unused_characters = {value.characters[7], value.characters[17], value.characters[18]};
    if (action == 2 || ((previous.local ^ value.local) & 0x10002))
        endpoint.state->literal_next = false;
    ++endpoint.state->settings_generation;
}
void TerminalManager::flush(const TerminalEndpoint &endpoint, unsigned action) {
    if (endpoint.state->software)
        endpoint.state->software->flush(action);
    else {
        const auto fd = control_descriptor(endpoint);
        terminal::flush(fd.get(), action);
    }
    if (action != 1)
        endpoint.state->literal_next = false;
}
void TerminalManager::flow(const TerminalEndpoint &endpoint, unsigned action) {
    if (endpoint.state->software)
        endpoint.state->software->flow(action);
    else
        terminal::flow(endpoint.anchor.get(), action);
}
bool TerminalManager::software(const TerminalEndpoint &endpoint) {
    return bool(endpoint.state->software);
}
int64_t TerminalManager::read(const TerminalEndpoint &endpoint, int fd, void *bytes, size_t length) {
    if (endpoint.state->software)
        return endpoint.state->software->read(endpoint.master, bytes, length);
    const auto result = ::read(fd, bytes, length);
    return result < 0 ? -linux_errno(errno) : result;
}
int64_t TerminalManager::write(const TerminalEndpoint &endpoint, int fd, const void *bytes, size_t length) {
    if (endpoint.state->software)
        return endpoint.state->software->write(endpoint.master, bytes, length);
    const auto result = host_write(fd, bytes, length);
    return result < 0 ? -linux_errno(errno) : result;
}
void TerminalManager::opened(const TerminalEndpoint &endpoint, uint32_t pid, bool no_controlling) {
    if (endpoint.master || no_controlling || !endpoint.state->master_alive)
        return;
    auto current = process(pid);
    if (current.pid != current.session || endpoint.state->session)
        return;
    const auto found = controlling_.find(pid);
    if (found != controlling_.end())
        if (const auto state = found->second.lock(); state && state->master_alive)
            return;
    if (!endpoint.readable && !(hooks_.can_steal && hooks_.can_steal(pid)))
        return;
    acquire(endpoint, pid, false);
}
void TerminalManager::acquire(const TerminalEndpoint &endpoint, uint32_t pid, bool steal) {
    auto current = process(pid);
    auto &state = *endpoint.state;
    if (!state.master_alive)
        guest_error(5);
    if (current.pid != current.session)
        guest_error(1);
    if (state.session == current.session)
        return; // An existing foreground job keeps ownership on repeated TIOCSCTTY.
    const auto found = controlling_.find(pid);
    if (found != controlling_.end())
        if (const auto other = found->second.lock(); other && other->master_alive)
            guest_error(1);
    const bool privileged = hooks_.can_steal && hooks_.can_steal(pid);
    if (state.session && (!steal || !privileged))
        guest_error(1);
    if (!endpoint.readable && !privileged)
        guest_error(1);
    if (state.session)
        clear_controlling(endpoint.state);
    state.session = current.session;
    state.foreground = current.group;
    controlling_[pid] = endpoint.state;
}
void TerminalManager::detach(const TerminalEndpoint &endpoint, uint32_t pid) {
    auto current = process(pid);
    auto &state = *endpoint.state;
    if (endpoint.master || !controls(endpoint, current))
        guest_error(25);
    controlling_.erase(pid);
    if (current.pid != current.session)
        return;
    const auto foreground = state.foreground;
    clear_controlling(endpoint.state);
    state.session = state.foreground = 0;
    signal(foreground, 1);
    signal(foreground, 18);
}
uint32_t TerminalManager::foreground_group(const TerminalEndpoint &endpoint, uint32_t pid) const {
    if (!endpoint.master) {
        auto current = process(pid);
        if (!controls(endpoint, current))
            guest_error(25);
    }
    return endpoint.state->foreground;
}
void TerminalManager::foreground_group(const TerminalEndpoint &endpoint, uint32_t pid, uint32_t group) {
    auto current = process(pid);
    try {
        before_control(endpoint, pid);
    } catch (const GuestError &error) {
        if (error.number == 5)
            guest_error(25); // TIOCSPGRP translates an orphaned-background EIO to ENOTTY.
        throw;
    }
    if (group > INT32_MAX)
        guest_error(22);
    if (!controls(endpoint, current))
        guest_error(25);
    const auto session = group_session(group, current.session);
    if (!session)
        guest_error(3);
    if (*session != current.session)
        guest_error(1);
    endpoint.state->foreground = group;
}
uint32_t TerminalManager::session(const TerminalEndpoint &endpoint, uint32_t pid) const {
    if (!endpoint.master) {
        auto current = process(pid);
        if (!controls(endpoint, current))
            guest_error(25);
    }
    if (!endpoint.state->session)
        guest_error(25);
    return endpoint.state->session;
}
void TerminalManager::session_created(uint32_t pid, uint32_t) {
    controlling_.erase(pid);
    const auto found = known_processes_.find(pid);
    if (found != known_processes_.end())
        found->second.session = found->second.group = pid;
}
void TerminalManager::process_forked(uint32_t child, uint32_t parent) {
    const auto known = known_processes_.find(parent);
    auto inherited = known == known_processes_.end() ? TerminalProcess{} : known->second;
    inherited.pid = child;
    inherited.parent = parent;
    known_processes_[child] = inherited;
    controlling_.erase(child);
    const auto found = controlling_.find(parent);
    if (found != controlling_.end())
        if (const auto state = found->second.lock(); state && state->master_alive && state->session)
            controlling_[child] = state;
}
void TerminalManager::process_exited(uint32_t pid) {
    known_processes_.erase(pid);
    for (auto &[child, identity] : known_processes_) {
        (void)child;
        if (identity.parent == pid)
            identity.parent = 0;
    }
    const auto found = controlling_.find(pid);
    if (found == controlling_.end())
        return;
    const auto state = found->second.lock();
    controlling_.erase(found);
    if (!state || state->session != pid)
        return;
    const auto foreground = state->foreground;
    clear_controlling(state);
    state->session = state->foreground = 0;
    signal(foreground, 1);
}
void TerminalManager::window_changed(const TerminalEndpoint &endpoint) {
    signal(endpoint.state->foreground, 28);
}
struct winsize TerminalManager::window_size(const TerminalEndpoint &endpoint) const {
    if (endpoint.state->software)
        return endpoint.state->software->window_size();
    const auto fd = control_descriptor(endpoint);
    winsize size{};
    check_host(::ioctl(fd.get(), TIOCGWINSZ, &size));
    return size;
}
int TerminalManager::input_queued(const TerminalEndpoint &endpoint) const {
    if (endpoint.state->software)
        return endpoint.state->software->input_queued(endpoint.master);
    return terminal::input_queued(endpoint.anchor.get());
}
int TerminalManager::output_queued(const TerminalEndpoint &endpoint) const {
    if (endpoint.state->software)
        return endpoint.state->software->output_queued();
    const auto fd = control_descriptor(endpoint);
    return terminal::output_queued(fd.get());
}
void TerminalManager::resize(const TerminalEndpoint &endpoint, const struct winsize &size) {
    const auto previous = window_size(endpoint);
    if (endpoint.state->software)
        endpoint.state->software->resize(size);
    else {
        const auto fd = control_descriptor(endpoint);
        check_host(::ioctl(fd.get(), TIOCSWINSZ, &size));
    }
    if (previous.ws_row != size.ws_row || previous.ws_col != size.ws_col ||
        previous.ws_xpixel != size.ws_xpixel || previous.ws_ypixel != size.ws_ypixel)
        window_changed(endpoint);
}
void TerminalManager::signal_foreground(const TerminalEndpoint &endpoint, unsigned number) {
    require_master(endpoint);
    if (!number || number > 64)
        guest_error(22);
    const auto &state = *endpoint.state;
    const auto session = group_session(state.foreground, state.session);
    if (!session || *session != state.session)
        guest_error(3);
    signal(state.foreground, number);
}
void TerminalManager::send_signal(const TerminalEndpoint &endpoint, unsigned number) {
    require_master(endpoint);
    if (number != 2 && number != 3 && number != 20)
        guest_error(22);
    signal(endpoint.state->foreground, number);
}
void TerminalManager::inject(const TerminalEndpoint &endpoint, uint32_t pid, uint8_t byte) {
    auto current = process(pid);
    if (endpoint.master || !controls(endpoint, current))
        guest_error(1);
    auto master = endpoint.state->master.lock();
    if (!master)
        guest_error(5);
    int64_t count = write(*master, master->anchor.get(), &byte, 1);
    if (count < 0)
        guest_error(int(-count));
    if (count != 1)
        guest_error(5);
    written_input(*master, &byte, 1);
}
void TerminalManager::before_io(const TerminalEndpoint &endpoint, uint32_t pid, bool writing) {
    if (endpoint.master) {
        if (!writing && endpoint.state->slave_ever_opened && !endpoint.state->slave_opens) {
            pollfd event{endpoint.anchor.get(), POLLIN, 0};
            int result = ::poll(&event, 1, 0);
            check_host(result);
            if (!(event.revents & POLLIN))
                guest_error(5);
        }
        return;
    }
    auto current = process(pid);
    auto &state = *endpoint.state;
    if (writing && !state.master_alive)
        guest_error(5);
    if (!state.foreground || !controls(endpoint, current) || current.group == state.foreground)
        return;
    auto value = attributes(endpoint);
    if (writing && !(value.local & 0x100)) // TOSTOP
        return;
    unsigned number = writing ? 22 : 21; // SIGTTOU / SIGTTIN
    bool blocked = hooks_.blocked_or_ignored && hooks_.blocked_or_ignored(pid, number);
    if (blocked) {
        if (!writing)
            guest_error(5);
        return;
    }
    if (orphaned(current.group))
        guest_error(5);
    signal(current.group, number);
    guest_error(4);
}
void TerminalManager::before_control(const TerminalEndpoint &endpoint, uint32_t pid) {
    auto current = process(pid);
    if (!endpoint.state->foreground || !controls(endpoint, current) ||
        current.group == endpoint.state->foreground)
        return;
    if (hooks_.blocked_or_ignored && hooks_.blocked_or_ignored(pid, 22))
        return;
    if (orphaned(current.group))
        guest_error(5);
    signal(current.group, 22);
    guest_error(4);
}
void TerminalManager::written_input(const TerminalEndpoint &endpoint, const void *bytes, size_t size) {
    if (!endpoint.master || !size || !endpoint.state->master_alive)
        return;
    const auto value = attributes(endpoint);
    if (value.local & 0x10000) // EXTPROC bypasses all terminal character processing.
        return;
    const auto *input = static_cast<const uint8_t *>(bytes);
    for (size_t i = 0; i < size; ++i) {
        uint8_t byte = (value.input & 0x20) ? input[i] & 0x7f : input[i];
        if (endpoint.state->literal_next) {
            endpoint.state->literal_next = false;
            continue;
        }
        if ((value.input & 0x400) && ((value.characters[8] && byte == value.characters[8]) ||
                                    (value.characters[9] && byte == value.characters[9])))
            continue;
        if (value.local & 1) {
            unsigned number = 0;
            if (value.characters[0] && byte == value.characters[0])
                number = 2;
            else if (value.characters[1] && byte == value.characters[1])
                number = 3;
            else if (value.characters[10] && byte == value.characters[10])
                number = 20;
            if (number) {
                signal(endpoint.state->foreground, number);
                continue;
            }
        }
        // ISIG precedes CR/NL translations and canonical VLNEXT processing.
        if (byte == '\r') {
            if (value.input & 0x80)
                continue;
            if (value.input & 0x100)
                byte = '\n';
        } else if (byte == '\n' && (value.input & 0x40))
            byte = '\r';
        if ((value.local & 0x8002) == 0x8002 && value.characters[15] && byte == value.characters[15])
            endpoint.state->literal_next = true;
    }
}
void TerminalManager::register_read_interruption(uint32_t tid,
                                                 std::function<std::optional<int64_t>()> complete) {
    if (!tid || !complete || interrupted_reads_.count(tid))
        throw Failure("invalid terminal read interruption registration");
    interrupted_reads_.emplace(tid, std::move(complete));
}
void TerminalManager::cancel_read(uint32_t tid) {
    interrupted_reads_.erase(tid);
}
std::optional<int64_t> TerminalManager::interrupt_read(uint32_t tid) {
    const auto found = interrupted_reads_.find(tid);
    if (found == interrupted_reads_.end())
        return {};
    auto complete = std::move(found->second);
    interrupted_reads_.erase(found);
    try {
        return complete();
    } catch (const GuestError &error) {
        return -int64_t(error.number);
    }
}
TerminalManager::ReadStatus TerminalManager::read_status(const TerminalEndpoint &endpoint, size_t capacity,
                                                       bool nonblocking, bool readable, bool hungup,
                                                       ReadState &wait) const {
    if (!endpoint.master && !endpoint.state->master_alive)
        return ReadStatus::Ready;
    if (hungup || endpoint.master)
        return readable || hungup ? ReadStatus::Ready : ReadStatus::Waiting;
    const auto value = attributes(endpoint);
    if ((value.local & 2) && !(value.local & 0x10000))
        return readable ? ReadStatus::Ready : ReadStatus::Waiting;
    const auto available = size_t(input_queued(endpoint));
    const bool external_canonical = (value.local & 0x10002) == 0x10002;
    const auto minimum = external_canonical ? size_t{1} : std::min<size_t>(capacity, value.characters[6]);
    const unsigned deciseconds = external_canonical ? 0 : value.characters[5];
    if (wait.interrupted)
        return available ? ReadStatus::Ready : ReadStatus::Waiting;
    if (nonblocking) {
        if (available)
            return ReadStatus::Ready;
        return !minimum && !deciseconds ? ReadStatus::Empty : ReadStatus::Waiting;
    }
    const auto now = std::chrono::steady_clock::now();
    if (!wait.initialized || wait.settings_generation != endpoint.state->settings_generation) {
        wait = {};
        wait.initialized = true;
        wait.settings_generation = endpoint.state->settings_generation;
        if (!minimum && deciseconds)
            wait.deadline = now + std::chrono::milliseconds(deciseconds * 100);
    }
    if (minimum && deciseconds && available > wait.previous_available)
        wait.deadline = now + std::chrono::milliseconds(deciseconds * 100);
    wait.previous_available = available;
    const bool expired = wait.deadline && now >= *wait.deadline;
    if (!available)
        return (!minimum && !deciseconds) || expired ? ReadStatus::Empty
                                                   : ReadStatus::Waiting;
    if (minimum && available < minimum && !expired)
        return ReadStatus::Waiting;
    return ReadStatus::Ready;
}
int64_t TerminalManager::read_result(const TerminalEndpoint &endpoint, void *bytes, int64_t result) const {
    if (endpoint.master && !endpoint.state->slave_opens) {
        if (!endpoint.state->slave_ever_opened && (result == 0 || result == -5))
            return -11;
        if (result == 0 || (endpoint.state->software && result == -11 && endpoint.state->slave_ever_opened))
            return -5;
    }
    if ((result == -5 || result == -11) && !endpoint.master && !endpoint.state->master_alive)
        return 0;
    if (result > 0 && endpoint.master && endpoint.state->packet)
        *static_cast<uint8_t *>(bytes) = terminal::packet_flags(*static_cast<uint8_t *>(bytes));
    return result;
}
} // namespace operit::arm64
