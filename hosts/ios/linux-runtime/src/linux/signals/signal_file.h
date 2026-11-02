#pragma once
#include "base/fd.h"
#include "linux/signals/signals.h"
#include <cstddef>

namespace operit::arm64 {
class DescriptorTable;
struct OpenDescription;
}

namespace operit::arm64::signals {
struct alignas(8) SignalFileInfo {
    uint32_t signo = 0;
    int32_t error = 0, code = 0;
    uint32_t pid = 0, uid = 0;
    int32_t fd = 0;
    uint32_t tid = 0, band = 0, overrun = 0, trapno = 0;
    int32_t status = 0, integer = 0;
    uint64_t pointer = 0, user_time = 0, system_time = 0, address = 0;
    uint16_t address_lsb = 0, padding = 0;
    int32_t syscall = 0;
    uint64_t call_address = 0;
    uint32_t architecture = 0;
    std::array<uint8_t, 28> reserved{};
};
static_assert(sizeof(SignalFileInfo) == 128);
static_assert(offsetof(SignalFileInfo, pointer) == 48);
static_assert(offsetof(SignalFileInfo, address_lsb) == 80);
static_assert(offsetof(SignalFileInfo, syscall) == 84);
static_assert(offsetof(SignalFileInfo, architecture) == 96);
SignalFileInfo signal_file_info(const Info &info);

class SignalFile {
    FD reader_, writer_;
    uint64_t mask_;
    bool ready_ = false;
    std::shared_ptr<OpenDescription> description_;
    SignalFile(FD reader, FD writer, uint64_t mask);
    void marker(bool ready);

  public:
    ~SignalFile();
    SignalFile(const SignalFile &) = delete;
    SignalFile &operator=(const SignalFile &) = delete;
    // The aliasing description owns this object: dup/fork share the same mask
    // and file status flags, and final close releases its native readiness pipe.
    static int create(DescriptorTable &descriptors, uint64_t mask, uint32_t flags);
    static std::shared_ptr<SignalFile> find(const std::shared_ptr<OpenDescription> &description);
    uint64_t mask() const {
        return mask_;
    }
    void set_mask(uint64_t mask);
    bool readable(const ProcessState &process, const ThreadState &thread, uint64_t blocked) const;
    void refresh(const ProcessState &process, const ThreadState &thread, uint64_t blocked);
    short poll_events(const ProcessState &process, const ThreadState &thread, uint64_t blocked,
                      short requested);
    // Nonblocking primitive for scalar/vector I/O. Output must be prepared
    // before calling: it consumes whole records, or returns 0 if none match.
    // The reader's queues and live blocked mask are used, never the creator's.
    size_t read(ProcessState &process, ThreadState &thread, uint64_t blocked, void *output, size_t count);
};

int64_t signalfd4(Memory &memory, DescriptorTable &descriptors, Hooks &hooks, int32_t fd,
                  uint64_t mask_address, uint64_t mask_size, uint32_t flags);
// nullopt means a different file type; 0 may mean the guest task was blocked.
// Call before generic host I/O, including its zero-length fast path.
std::optional<int64_t> signal_file_io(Memory &memory, DescriptorTable &descriptors, Hooks &hooks,
                                     const std::shared_ptr<OpenDescription> &description, bool writing,
                                     uint64_t address, uint64_t count, bool positioned = false,
                                     bool may_block = true);
std::optional<short> signal_file_poll(const std::shared_ptr<OpenDescription> &description, Hooks &hooks,
                                     short requested);
// Invoke at serial safe points after signal/timer/mask changes and before each
// poll/epoll attempt (including scheduler retries) in that waiting task's scope.
// Native readiness is contextual: a forked/shared FD observes each reader's
// pending queues and blocked mask when that reader is selected.
void signal_files_safe_point(DescriptorTable &descriptors, Hooks &hooks);
} // namespace operit::arm64::signals
