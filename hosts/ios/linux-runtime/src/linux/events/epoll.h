#pragma once
#include "base/fd.h"
#include <cstdint>
#include <map>
#include <memory>
#include <set>
#include <vector>
namespace operit::arm64 {
struct OpenDescription;
struct PollReady {
    uint32_t events;
    uint64_t data;
};
class EventPoll {
    using Key = std::pair<int, uintptr_t>;
    struct Watch {
        FD host;
        std::weak_ptr<OpenDescription> owner;
        Key key;
        EventPoll *nested;
        uint32_t events;
        uint64_t data;
        bool armed = true;
#ifdef __APPLE__
        bool child_ready = false;
        uint64_t child_revision = 0;
#endif
    };
    FD queue_;
    uint64_t next_token_ = 1;
    std::map<uint64_t, Watch> watches_;
    std::map<Key, uint64_t> keys_;
    std::set<std::pair<EventPoll *, uint64_t>> parents_;
    void prune();
    void erase(uint64_t token) noexcept;
    void remove(uint64_t token);
    void validate_nested(EventPoll &child) const;
#ifdef __APPLE__
    std::map<uint64_t, uint32_t> pending_;
    uint64_t last_ready_ = 0;
    uint64_t revision_ = 0;
    bool marker_enabled_ = false;
    void change(Watch &watch, uint64_t token);
    void marker(bool enabled);
    uint32_t ready_mask(const Watch &watch, uint32_t events) const;
    void refresh(std::set<EventPoll *> &refreshed);
    void collect_queues(std::set<EventPoll *> &queues);
    void await_events(int timeout);
#endif

  public:
    EventPoll();
    ~EventPoll();
    static void description_closed(const OpenDescription *description) noexcept;
    int native_handle() const {
        return queue_.get();
    }
    short poll_events(short requested);
    void control(int operation, int guest_fd, int host_fd, const std::shared_ptr<OpenDescription> &owner,
                 uint32_t events, uint64_t data);
    std::vector<PollReady> wait(int maximum, int timeout);
};
} // namespace operit::arm64
