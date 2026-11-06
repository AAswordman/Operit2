#include "linux/io_uring/io_uring.h"

#include "base/fd.h"
#include "base/host_io.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include "linux/abi.h"
#include "linux/error_mapping.h"
#include "linux/events/inotify.h"
#include "linux/files/descriptor_access.h"
#include "linux/files/memory_file.h"
#include "linux/process/limit_enforcement.h"
#include "memory/access_sync.h"
#include "memory/address_space.h"
#include "memory/fault.h"
#include "runtime/tasks/scheduler.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <list>
#include <map>
#include <mutex>
#include <new>
#include <poll.h>
#include <pthread.h>
#include <sys/mman.h>
#include <sys/stat.h>
#ifndef __APPLE__
#include <sys/uio.h>
#endif
#include <unordered_map>
#include <unistd.h>
#include <vector>

namespace operit::arm64::io_uring {
namespace {
constexpr uint32_t setup_cqsize = 1u << 3, setup_clamp = 1u << 4;
constexpr uint32_t enter_getevents = 1u;
constexpr uint64_t off_sq = 0, off_cq = 0x8000000, off_sqes = 0x10000000;
constexpr uint32_t sq_head = 0, sq_tail = 4, sq_mask = 8, sq_entries = 12;
constexpr uint32_t sq_flags = 16, sq_dropped = 20;
constexpr uint32_t cq_head = 64, cq_tail = 68, cq_mask = 72, cq_entries = 76;
constexpr uint32_t cq_overflow = 80, cq_flags = 84, cqes = 128;
constexpr uint32_t sq_cq_overflow = 2;
constexpr uint32_t max_sq = 32768, max_cq = 65536, max_overflow = 65536;
constexpr uint8_t op_nop = 0, op_readv = 1, op_writev = 2, op_fsync = 3;
constexpr uint8_t op_read = 22, op_write = 23, op_last = 59;
constexpr uint8_t sqe_link = 1u << 2, sqe_hardlink = 1u << 3;
constexpr uint32_t fsync_datasync = 1;

struct SqOffsets {
    uint32_t head, tail, mask, entries, flags, dropped, array, reserved;
    uint64_t user_address;
};
struct CqOffsets {
    uint32_t head, tail, mask, entries, overflow, cqes, flags, reserved;
    uint64_t user_address;
};
struct Params {
    uint32_t sq_entries, cq_entries, flags, sq_thread_cpu, sq_thread_idle, features, wq_fd;
    uint32_t reserved[3];
    SqOffsets sq;
    CqOffsets cq;
};
struct Sqe {
    uint8_t opcode, flags;
    uint16_t ioprio;
    int32_t fd;
    uint64_t offset, address;
    uint32_t length, operation_flags;
    uint64_t user_data;
    uint16_t buffer_index, personality;
    uint32_t file_index;
    uint64_t address3, reserved;
};
struct Cqe {
    uint64_t user_data;
    int32_t result;
    uint32_t flags;
};
struct ProbeHeader {
    uint8_t last_op, ops_len;
    uint16_t reserved;
    uint32_t reserved2[3];
};
struct ProbeOp {
    uint8_t op, reserved;
    uint16_t flags;
    uint32_t reserved2;
};
static_assert(sizeof(Params) == 120 && sizeof(Sqe) == 64 && sizeof(Cqe) == 16);
static_assert(sizeof(ProbeHeader) == 16 && sizeof(ProbeOp) == 8);

uint32_t round_power_of_two(uint32_t value) {
    --value;
    value |= value >> 1;
    value |= value >> 2;
    value |= value >> 4;
    value |= value >> 8;
    value |= value >> 16;
    return value + 1;
}
uint64_t round_page(uint64_t value) {
    return (value + page_size - 1) & ~(page_size - 1);
}
FD temporary_file() {
    FILE *file = std::tmpfile();
    if (!file)
        guest_error(linux_errno(errno));
    int fd = ::fcntl(::fileno(file), F_DUPFD_CLOEXEC, 0);
    int error = errno;
    std::fclose(file);
    if (fd < 0)
        guest_error(linux_errno(error));
    return FD(fd);
}
void make_nonblocking(int fd) {
    int flags = ::fcntl(fd, F_GETFL);
    check_host(flags);
    check_host(::fcntl(fd, F_SETFL, flags | O_NONBLOCK));
    check_host(::fcntl(fd, F_SETFD, FD_CLOEXEC));
}

struct Waits {
    struct Entry {
        int64_t submitted;
        std::weak_ptr<Ring> ring;
    };
    std::mutex mutex;
    std::unordered_map<Task *, Entry> pending;
};
Waits &waits() {
    static Waits value;
    return value;
}
void forget_wait(Task *task) {
    auto &state = waits();
    std::lock_guard<std::mutex> lock(state.mutex);
    state.pending.erase(task);
}
struct WaitRegistration {
    Task *task;
    explicit WaitRegistration(Task *value) : task(value) {}
    void release() {
        if (task) {
            forget_wait(task);
            task = nullptr;
        }
    }
    ~WaitRegistration() {
        release();
    }
};

} // namespace

namespace {
struct WorkerBudget {
    std::atomic<uint32_t> active{0};
};
std::atomic<uint32_t> global_workers{0};

bool acquire_worker(std::atomic<uint32_t> &count, uint32_t maximum) noexcept {
    uint32_t current = count.load(std::memory_order_relaxed);
    while (current < maximum) {
        if (count.compare_exchange_weak(current, current + 1, std::memory_order_relaxed))
            return true;
    }
    return false;
}

struct HostWork;
struct CompletionQueue {
    std::mutex mutex;
    HostWork *first = nullptr, *last = nullptr;
    // The coordinator polls every quantum, including programs that never
    // submit I/O. Publish readiness under the queue mutex; an empty probe
    // needs neither that mutex nor a host read(EAGAIN).
    std::atomic<bool> ready{false};
    FD reader, writer;

    CompletionQueue() {
        int descriptors[2];
        check_host(::pipe(descriptors));
        reader = FD(descriptors[0]);
        writer = FD(descriptors[1]);
        make_nonblocking(reader.get());
        make_nonblocking(writer.get());
    }
    ~CompletionQueue();
    void enqueue(HostWork *work) noexcept;
    HostWork *take() noexcept;
};

struct HostWork {
    std::shared_ptr<CompletionQueue> queue;
    std::shared_ptr<process::ResourceLimits> limits;
    std::shared_ptr<WorkerBudget> budget;
    FD descriptor;
    std::vector<uint8_t> bytes;
    HostWork *next = nullptr;
    uint64_t id = 0, offset = 0;
    size_t requested_size = 0;
    uint8_t opcode = 0;
    uint32_t flags = 0;
    int32_t result = -5;
    bool first_buffer_fault = false, file_size_signal = false;

    bool reserve_worker(const std::shared_ptr<WorkerBudget> &owner) noexcept {
        if (!acquire_worker(global_workers, max_async_in_flight_global))
            return false;
        if (!acquire_worker(owner->active, max_async_in_flight_per_context)) {
            global_workers.fetch_sub(1, std::memory_order_relaxed);
            return false;
        }
        budget = owner;
        return true;
    }
    ~HostWork() {
        if (budget) {
            budget->active.fetch_sub(1, std::memory_order_relaxed);
            global_workers.fetch_sub(1, std::memory_order_relaxed);
        }
    }

    void execute() noexcept {
        try {
            const int fd = descriptor.get();
            struct stat info{};
            if (::fstat(fd, &info) < 0) {
                result = -linux_errno(errno);
                return;
            }
            if (!S_ISREG(info.st_mode)) {
                result = -95;
                return;
            }
            // Memfd seal checks need coordination with synchronous users and
            // cannot be implemented by merely copying a native descriptor.
            // Identifying one can call fstat, so do it on the worker too.
            if (files::memory_file_link(fd)) {
                result = -95;
                return;
            }
            if (opcode == op_fsync) {
#ifdef __APPLE__
                const int status = ::fsync(fd);
#else
                const int status = flags & fsync_datasync ? ::fdatasync(fd) : ::fsync(fd);
#endif
                result = status < 0 ? -linux_errno(errno) : 0;
                return;
            }
            const bool writing = opcode == op_write || opcode == op_writev;
            const bool positioned = offset != UINT64_MAX;
            if (first_buffer_fault && !writing) {
                const off_t at = positioned ? off_t(offset) : ::lseek(fd, 0, SEEK_CUR);
                if (at < 0) {
                    result = -linux_errno(errno);
                    return;
                }
                uint8_t byte;
                const ssize_t peek = ::pread(fd, &byte, 1, at);
                result = peek < 0 ? -linux_errno(errno) : peek ? -14 : 0;
                return;
            }
            ssize_t transferred;
            int error;
            if (writing) {
                const int native_flags = ::fcntl(fd, F_GETFL);
                if (native_flags < 0) {
                    result = -linux_errno(errno);
                    return;
                }
                // Restoring a shared open-file offset after an append is not
                // atomic against other guest operations.
                if (positioned && (native_flags & O_APPEND)) {
                    result = -95;
                    return;
                }
                auto guard = process::guard_file_write(
                    *limits, fd, first_buffer_fault ? requested_size : bytes.size(),
                    positioned ? std::optional<uint64_t>(offset) : std::nullopt,
                    std::nullopt, [this] { file_size_signal = true; });
                if (first_buffer_fault) {
                    result = -14;
                    return;
                }
                const size_t count = size_t(guard.length());
                transferred = positioned ? ::pwrite(fd, bytes.data(), count, off_t(offset))
                                         : host_write(fd, bytes.data(), count);
                error = errno;
            } else {
#ifdef __APPLE__
                transferred = positioned ? ::pread(fd, bytes.data(), bytes.size(), off_t(offset))
                                         : ::read(fd, bytes.data(), bytes.size());
#else
                if (opcode == op_readv) {
                    struct iovec vector{bytes.data(), bytes.size()};
                    transferred = positioned ? ::preadv(fd, &vector, 1, off_t(offset))
                                             : ::readv(fd, &vector, 1);
                } else {
                    transferred = positioned ? ::pread(fd, bytes.data(), bytes.size(), off_t(offset))
                                             : ::read(fd, bytes.data(), bytes.size());
                }
#endif
                error = errno;
            }
            result = transferred < 0 ? -linux_errno(error) :
                size_t(transferred) <= bytes.size() ? int32_t(transferred) : -5;
        } catch (const GuestError &error) {
            result = -error.number;
        } catch (const std::bad_alloc &) {
            result = -12;
        } catch (...) {
            result = -5;
        }
    }
};

CompletionQueue::~CompletionQueue() {
    while (first) {
        auto *next = first->next;
        delete first;
        first = next;
    }
}
void CompletionQueue::enqueue(HostWork *work) noexcept {
    std::lock_guard<std::mutex> lock(mutex);
    if (last)
        last->next = work;
    else
        first = work;
    last = work;
    ready.store(true, std::memory_order_release);
    const char marker = 1;
    ssize_t status;
    do {
        status = ::write(writer.get(), &marker, 1);
    } while (status < 0 && errno == EINTR);
}
HostWork *CompletionQueue::take() noexcept {
    if (!ready.load(std::memory_order_acquire))
        return nullptr;
    std::lock_guard<std::mutex> lock(mutex);
    HostWork *result = first;
    first = last = nullptr;
    ready.store(false, std::memory_order_release);
    char buffer[128];
    ssize_t status;
    do {
        status = ::read(reader.get(), buffer, sizeof(buffer));
    } while (status > 0 || (status < 0 && errno == EINTR));
    return result;
}
void *host_worker(void *argument) noexcept {
    std::unique_ptr<HostWork> work(static_cast<HostWork *>(argument));
    work->execute();
    // Even a slow last close stays off the guest coordinator.
    work->descriptor = FD();
    auto queue = std::move(work->queue);
    queue->enqueue(work.release());
    return nullptr;
}
int start_host_work(std::unique_ptr<HostWork> &work) {
    pthread_attr_t attributes;
    int status = pthread_attr_init(&attributes);
    if (status)
        return status;
    status = pthread_attr_setdetachstate(&attributes, PTHREAD_CREATE_DETACHED);
    pthread_t thread;
    if (!status)
        status = pthread_create(&thread, &attributes, host_worker, work.get());
    pthread_attr_destroy(&attributes);
    if (!status)
        work.release();
    return status;
}

struct PendingCompletion {
    std::shared_ptr<Ring> ring;
    std::shared_ptr<OpenDescription> description;
    std::weak_ptr<Memory> memory;
    MappingLease lease;
    std::vector<MemoryRange> ranges;
    std::list<Cqe> overflow_slot;
    uint64_t user_data = 0, task_start = 0;
    uint32_t tid = 0, pid = 0;
    bool reading = false;
};

void prepare_buffers(Memory &memory, const Sqe &sqe, HostWork &work, PendingCompletion &pending,
                     const CaptureMappingLease &capture) {
    const bool writing = sqe.opcode == op_write || sqe.opcode == op_writev;
    const bool vector = sqe.opcode == op_readv || sqe.opcode == op_writev;
    std::vector<linux_abi::Iovec> imported;
    if (vector) {
        if (sqe.length > 1024)
            guest_error(22);
        const size_t count = sqe.length;
        if (!memory.accessible(sqe.address, count * sizeof(linux_abi::Iovec), read_perm))
            guest_error(14);
        imported.resize(count);
        memory.copy_out(sqe.address, imported.data(), count * sizeof(linux_abi::Iovec));
    } else {
        imported.push_back({sqe.address, sqe.length});
    }
    size_t total = 0;
    for (auto &entry : imported) {
        if (vector && entry.length > uint64_t(INT64_MAX))
            guest_error(22);
        if (vector && (entry.base > address_limit || entry.length > address_limit - entry.base))
            guest_error(14);
        entry.length = std::min<uint64_t>(entry.length, max_io - total);
        total += size_t(entry.length);
    }
    if (sqe.offset != UINT64_MAX && total > uint64_t(INT64_MAX) - sqe.offset)
        guest_error(22);
    if (writing && total)
        pending.description->require_writable();
    work.requested_size = total;
    work.bytes.resize(total);
    size_t used = 0;
    bool fault = false;
    for (const auto &entry : imported) {
        uint64_t address = entry.base;
        size_t remaining = size_t(entry.length);
        while (remaining) {
            const size_t count = std::min<size_t>(remaining, page_size - address % page_size);
            if (!memory.accessible(address, count, writing ? read_perm : write_perm)) {
                fault = true;
                break;
            }
            try {
                if (writing)
                    memory.copy_out(address, work.bytes.data() + used, count);
                else
                    memory.translate(address, write_perm);
            } catch (const MemoryFault &) {
                fault = true;
                break;
            }
            if (!writing)
                pending.ranges.push_back({address, count});
            used += count;
            address += count;
            remaining -= count;
        }
        if (fault)
            break;
    }
    work.bytes.resize(used);
    work.first_buffer_fault = fault && !used;
    pending.reading = !writing;
    if (pending.reading && used) {
        pending.lease = capture(memory, pending.ranges);
        if (!pending.lease)
            guest_error(95);
    }
}
} // namespace

class CompletionContext {
  public:
    std::shared_ptr<CompletionQueue> queue = std::make_shared<CompletionQueue>();
    std::shared_ptr<WorkerBudget> budget = std::make_shared<WorkerBudget>();
    std::map<uint64_t, std::unique_ptr<PendingCompletion>> pending;
    uint64_t next_id = 1;
    ~CompletionContext();
};

class Ring {
    FD backing_, marker_reader_, marker_writer_;
    uint8_t *data_ = nullptr;
    uint64_t ring_bytes_, sqes_bytes_, total_bytes_;
    uint32_t sq_count_, cq_count_, array_offset_, cached_sq_head_ = 0, cached_cq_tail_ = 0;
    std::list<Cqe> overflow_;
    uint32_t in_flight_ = 0;
    std::mutex mutex_;

    uint32_t read32(uint64_t offset) const {
        uint32_t value;
        std::memcpy(&value, data_ + offset, sizeof(value));
        return value;
    }
    void write32(uint64_t offset, uint32_t value) {
        std::memcpy(data_ + offset, &value, sizeof(value));
    }
    uint32_t ready() const {
        return std::min<uint32_t>(cached_cq_tail_ - read32(cq_head), cq_count_);
    }
    bool room() const {
        return cached_cq_tail_ - read32(cq_head) < cq_count_;
    }
    void marker() {
        const bool now = ready() != 0;
        pollfd descriptor{marker_reader_.get(), POLLIN, 0};
        int status;
        do {
            status = ::poll(&descriptor, 1, 0);
        } while (status < 0 && errno == EINTR);
        if (status < 0 || descriptor.revents & (POLLERR | POLLNVAL))
            throw Failure("io_uring readiness marker poll failed");
        const bool present = descriptor.revents & POLLIN;
        if (now == present)
            return;
        char byte = 1;
        ssize_t result;
        do {
            result = now ? ::write(marker_writer_.get(), &byte, 1)
                         : ::read(marker_reader_.get(), &byte, 1);
        } while (result < 0 && errno == EINTR);
        if (result != 1 && !(result < 0 && errno == EAGAIN))
            throw Failure("io_uring readiness marker failed");
    }
    void publish(const Cqe &event) {
        std::memcpy(data_ + cqes + uint64_t(cached_cq_tail_ & (cq_count_ - 1)) * sizeof(Cqe),
                    &event, sizeof(event));
        ++cached_cq_tail_;
        write32(cq_tail, cached_cq_tail_);
    }
    void flush() {
        while (!overflow_.empty() && room()) {
            publish(overflow_.front());
            overflow_.pop_front();
        }
        uint32_t flags = read32(sq_flags);
        write32(sq_flags, overflow_.empty() ? flags & ~sq_cq_overflow : flags | sq_cq_overflow);
        marker();
    }
  public:
    Ring(FD backing, FD marker_reader, FD marker_writer, uint32_t sq_count, uint32_t cq_count)
        : backing_(std::move(backing)), marker_reader_(std::move(marker_reader)),
          marker_writer_(std::move(marker_writer)),
          ring_bytes_(round_page((cqes + uint64_t(cq_count) * sizeof(Cqe) + 63 & ~uint64_t{63}) +
                                 uint64_t(sq_count) * sizeof(uint32_t))),
          sqes_bytes_(round_page(uint64_t(sq_count) * sizeof(Sqe))),
          total_bytes_(ring_bytes_ + sqes_bytes_), sq_count_(sq_count), cq_count_(cq_count),
          array_offset_(uint32_t((cqes + uint64_t(cq_count) * sizeof(Cqe) + 63) & ~uint64_t{63})) {
        check_host(::ftruncate(backing_.get(), off_t(total_bytes_)));
        void *mapping = ::mmap(nullptr, size_t(total_bytes_), PROT_READ | PROT_WRITE, MAP_SHARED,
                               backing_.get(), 0);
        if (mapping == MAP_FAILED)
            guest_error(linux_errno(errno));
        data_ = static_cast<uint8_t *>(mapping);
        write32(sq_mask, sq_count_ - 1);
        write32(sq_entries, sq_count_);
        write32(cq_mask, cq_count_ - 1);
        write32(cq_entries, cq_count_);
    }
    ~Ring() {
        if (data_)
            ::munmap(data_, size_t(total_bytes_));
    }
    Ring(const Ring &) = delete;
    Ring &operator=(const Ring &) = delete;

    Params params(uint32_t flags) const {
        Params result{};
        result.sq_entries = sq_count_;
        result.cq_entries = cq_count_;
        result.flags = flags;
        // SINGLE_MMAP, NODROP, SUBMIT_STABLE and RW_CUR_POS.
        result.features = 1 | 2 | 4 | 8;
        result.sq = {sq_head, sq_tail, sq_mask, sq_entries, sq_flags, sq_dropped, array_offset_, 0, 0};
        result.cq = {cq_head, cq_tail, cq_mask, cq_entries, cq_overflow, cqes, cq_flags, 0, 0};
        return result;
    }
    void map(Memory &memory, const std::shared_ptr<Ring> &owner, uint64_t address, uint64_t length,
             unsigned permissions, uint64_t flags, uint64_t offset, bool replace) {
        const uint64_t type = flags & 15;
        if (type != 1 && type != 3)
            guest_error(95);
        uint64_t backing_offset, maximum;
        if (offset == off_sq || offset == off_cq) {
            backing_offset = 0;
            maximum = ring_bytes_;
        } else if (offset == off_sqes) {
            backing_offset = ring_bytes_;
            maximum = sqes_bytes_;
        } else
            guest_error(22);
        if (!length || length > maximum || length % page_size)
            guest_error(22);
        memory.map_file(address, length, permissions, backing_.get(),
                        {"", backing_offset, 0, 0, true}, replace, true, owner);
    }
    uint32_t refresh() {
        const memory_detail::AccessSynchronization::WriteGuard memory_guard(true);
        std::lock_guard<std::mutex> lock(mutex_);
        flush();
        return ready();
    }
    uint32_t completion_capacity() const {
        return cq_count_;
    }
    void complete_async(const Cqe &event, std::list<Cqe> &slot) {
        const memory_detail::AccessSynchronization::WriteGuard memory_guard(true);
        std::lock_guard<std::mutex> lock(mutex_);
        if (!in_flight_ || slot.empty())
            throw Failure("missing io_uring completion reservation");
        --in_flight_;
        if (overflow_.empty() && room())
            publish(event);
        else {
            slot.front() = event;
            overflow_.splice(overflow_.end(), slot);
        }
        flush();
    }
    void abandon_async() noexcept {
        std::lock_guard<std::mutex> lock(mutex_);
        if (in_flight_)
            --in_flight_;
    }
    int64_t start_file(const std::shared_ptr<Ring> &owner, Memory &memory,
                       DescriptorTable &descriptors, Scheduler &scheduler, const Sqe &sqe,
                       const AsyncSubmission *async, bool &queued) {
        if (!async || !async->context || !async->capture_mapping)
            return -95;
        const auto live_memory = async->memory.lock();
        if (!live_memory || live_memory.get() != &memory)
            return -95;
        if (in_flight_ >= max_async_in_flight_per_ring ||
            overflow_.size() + in_flight_ >= max_overflow)
            return -16;
        auto &entry = descriptors.get(uint32_t(sqe.fd));
        const bool sync = sqe.opcode == op_fsync;
        const bool writing = sqe.opcode == op_write || sqe.opcode == op_writev;
        if (sync)
            files::require_data_descriptor(entry);
        else
            files::require_descriptor_io(entry, writing);
        if (entry.host.get() < 0 || entry.description->virtual_file ||
            entry.description->standard_stream >= 0)
            return -95;
        if (sync && (sqe.offset > uint64_t(INT64_MAX) ||
                     sqe.length > uint64_t(INT64_MAX) - sqe.offset))
            return -22;
        if (!sync && sqe.offset != UINT64_MAX && sqe.offset > uint64_t(INT64_MAX))
            return -22;

        auto work = std::make_unique<HostWork>();
        if (!work->reserve_worker(async->context->budget))
            return -11;
        auto pending = std::make_unique<PendingCompletion>();
        pending->overflow_slot.emplace_back();
        pending->ring = owner;
        pending->description = entry.description;
        pending->memory = async->memory;
        pending->user_data = sqe.user_data;
        const auto &task = scheduler.current();
        pending->tid = task.tid;
        pending->pid = task.pid;
        pending->task_start = task.start_boottime_nanoseconds;
        work->descriptor = FD(::fcntl(entry.host.get(), F_DUPFD_CLOEXEC, 0));
        if (work->descriptor.get() < 0)
            guest_error(linux_errno(errno));
        work->queue = async->context->queue;
        work->limits = descriptors.resource_limits();
        work->opcode = sqe.opcode;
        work->flags = sqe.operation_flags;
        work->offset = sqe.offset;
        if (!sync)
            prepare_buffers(memory, sqe, *work, *pending, async->capture_mapping);
        const uint64_t id = async->context->next_id++;
        work->id = id;
        auto inserted = async->context->pending.emplace(id, std::move(pending));
        if (!inserted.second)
            return -16;
        ++in_flight_;
        const int status = start_host_work(work);
        if (status) {
            --in_flight_;
            async->context->pending.erase(inserted.first);
            return -linux_errno(status);
        }
        queued = true;
        return 0;
    }
    int64_t submit(Memory &memory, DescriptorTable &descriptors, Scheduler &scheduler,
                   const std::shared_ptr<Ring> &owner, uint32_t requested, const AsyncSubmission *async) {
        const memory_detail::AccessSynchronization::WriteGuard memory_guard(true);
        std::lock_guard<std::mutex> lock(mutex_);
        flush();
        const uint32_t available = std::min<uint32_t>(read32(sq_tail) - cached_sq_head_, sq_count_);
        const uint32_t count = std::min(requested, available);
        uint32_t submitted = 0;
        bool cancel_link = false;
        int error = 0;
        while (submitted < count) {
            const bool deferred = !overflow_.empty() || !room();
            if (deferred) {
                if (overflow_.size() + in_flight_ >= max_overflow) {
                    error = 16;
                    break;
                }
                // Reserve a completion before any operation changes a file.
                try {
                    overflow_.emplace_back();
                } catch (const std::bad_alloc &) {
                    error = 12;
                    break;
                }
            }
            const uint32_t index = read32(array_offset_ +
                                          uint64_t(cached_sq_head_ & (sq_count_ - 1)) * 4);
            ++cached_sq_head_;
            if (index >= sq_count_) {
                if (deferred)
                    overflow_.pop_back();
                write32(sq_dropped, read32(sq_dropped) + 1);
                break;
            }
            Sqe sqe{};
            std::memcpy(&sqe, data_ + ring_bytes_ + uint64_t(index) * sizeof(Sqe), sizeof(sqe));
            int64_t result = 0;
            bool queued = false;
            const bool linked = sqe.flags & (sqe_link | sqe_hardlink);
            if (cancel_link) {
                result = -125; // ECANCELED
                cancel_link = linked;
            } else if (sqe.flags) {
                result = -95; // Unsupported SQE flags must not run the operation.
                cancel_link = linked;
            } else if (sqe.personality) {
                result = -22;
            } else if (sqe.opcode == op_nop) {
                if (sqe.operation_flags & ~1u)
                    result = -22;
                else if (sqe.operation_flags & 1)
                    result = int32_t(sqe.length);
            } else if (sqe.opcode == op_read || sqe.opcode == op_write ||
                       sqe.opcode == op_readv || sqe.opcode == op_writev ||
                       sqe.opcode == op_fsync) {
                const bool sync = sqe.opcode == op_fsync;
                if (sqe.ioprio || sqe.buffer_index || sqe.file_index || sqe.address3 || sqe.reserved ||
                    (sync ? (sqe.address || (sqe.operation_flags & ~fsync_datasync))
                          : sqe.operation_flags))
                    result = sync ? -22 : -95;
                if (!result) {
                    try {
                        result = start_file(owner, memory, descriptors, scheduler, sqe, async, queued);
                    } catch (const GuestError &error) {
                        result = -error.number;
                    } catch (const MemoryFault &) {
                        result = -14;
                    } catch (const std::bad_alloc &) {
                        result = -12;
                    }
                }
            } else
                result = -22;
            if (queued) {
                if (deferred)
                    overflow_.pop_back();
                ++submitted;
                continue;
            }
            // All supported operations return Linux's signed 32-bit CQE result.
            if (result > INT32_MAX || result < INT32_MIN)
                result = -5;
            const Cqe completion{sqe.user_data, int32_t(result), 0};
            if (deferred)
                overflow_.back() = completion;
            else
                publish(completion);
            ++submitted;
        }
        write32(sq_head, cached_sq_head_);
        flush();
        return !submitted && error ? -error : submitted;
    }
};

CompletionContext::~CompletionContext() {
    for (auto &entry : pending)
        entry.second->ring->abandon_async();
}

std::shared_ptr<CompletionContext> make_completion_context() {
    return std::make_shared<CompletionContext>();
}

void service_completions(const std::shared_ptr<CompletionContext> &context, Scheduler &scheduler) {
    if (!context)
        return;
    auto *next = context->queue->take();
    while (next) {
        std::unique_ptr<HostWork> work(next);
        next = work->next;
        const auto found = context->pending.find(work->id);
        if (found == context->pending.end())
            continue;
        auto &pending = *found->second;
        int32_t result = work->result;
        if (pending.reading && result > 0) {
            auto memory = pending.memory.lock();
            try {
                if (!memory || !pending.lease || !pending.lease(*memory)) {
                    result = -14;
                } else {
                    size_t remaining = size_t(result), copied = 0;
                    for (const auto &range : pending.ranges) {
                        const size_t count = std::min<size_t>(range.length, remaining);
                        if (count)
                            memory->copy_in(range.address, work->bytes.data() + copied, count);
                        copied += count;
                        remaining -= count;
                        if (!remaining)
                            break;
                    }
                    if (remaining)
                        result = -5;
                }
            } catch (const MemoryFault &) {
                result = -14;
            } catch (const GuestError &) {
                result = -14;
            }
        }
        if (pending.reading && result >= 0 && pending.description->inotify_open)
            pending.description->inotify_open->accessed();
        if (work->file_size_signal) {
            const auto *task = scheduler.lookup(pending.tid);
            if (task && task->pid == pending.pid && task->start_boottime_nanoseconds == pending.task_start)
                process::send_limit_signal(scheduler, pending.tid, 25);
        }
        pending.ring->complete_async({pending.user_data, result, 0}, pending.overflow_slot);
        context->pending.erase(found);
    }
}

SetupResult setup(Memory &memory, DescriptorTable &descriptors, uint32_t entries,
                  uint64_t params_address) {
    if (!memory.accessible(params_address, sizeof(Params), read_perm | write_perm))
        guest_error(14);
    Params input{};
    memory.copy_out(params_address, &input, sizeof(input));
    if (input.reserved[0] || input.reserved[1] || input.reserved[2] ||
        (input.flags & ~(setup_cqsize | setup_clamp)) || !entries)
        guest_error(22);
    if (entries > max_sq) {
        if (!(input.flags & setup_clamp))
            guest_error(22);
        entries = max_sq;
    }
    const uint32_t sq = round_power_of_two(entries);
    uint32_t cq = sq * 2;
    if (input.flags & setup_cqsize) {
        cq = input.cq_entries;
        if (!cq)
            guest_error(22);
        if (cq > max_cq) {
            if (!(input.flags & setup_clamp))
                guest_error(22);
            cq = max_cq;
        }
        cq = round_power_of_two(cq);
        if (cq < sq)
            guest_error(22);
    }
    int pipefd[2];
    check_host(::pipe(pipefd));
    FD reader(pipefd[0]), writer(pipefd[1]);
    make_nonblocking(reader.get());
    make_nonblocking(writer.get());
    FD retained_reader(::fcntl(reader.get(), F_DUPFD_CLOEXEC, 0));
    check_host(retained_reader.get());
    auto ring = std::make_shared<Ring>(temporary_file(), std::move(retained_reader), std::move(writer), sq, cq);
    Params output = ring->params(input.flags);
    memory.copy_in(params_address, &output, sizeof(output));
    int fd = descriptors.insert(std::move(reader), "", 2 | 524288);
    descriptors.get(fd).description->scheduler_io = true;
    return {fd, std::move(ring)};
}

int64_t enter(Memory &memory, DescriptorTable &descriptors, Scheduler &scheduler,
              const std::shared_ptr<Ring> &ring, uint32_t to_submit, uint32_t min_complete,
              uint32_t flags, uint64_t argument, uint64_t, const ExecuteIo &execute_io,
              const InterruptPending &interrupt_pending, const AsyncSubmission *async) {
    (void)execute_io;
    if (!ring)
        guest_error(95);
    if (flags & ~enter_getevents)
        guest_error(22);
    if ((flags & enter_getevents) && argument)
        guest_error(95);
    int64_t submitted = to_submit ? ring->submit(memory, descriptors, scheduler, ring, to_submit, async) : 0;
    if (submitted < 0 || uint64_t(submitted) != to_submit || !(flags & enter_getevents))
        return submitted;
    min_complete = std::min(min_complete, ring->completion_capacity());
    if (ring->refresh() >= min_complete)
        return submitted;
    auto *task = &scheduler.current();
    if (task->signals.restart)
        task->signals.restart->restartable = false;
    if (interrupt_pending && interrupt_pending()) {
        const int64_t result = submitted ? submitted : ring->refresh() ? 0 : -4;
        if (result != -4 && task->signals.restart)
            task->signals.restart->interrupted = false;
        return result;
    }
    auto registration = std::make_shared<WaitRegistration>(task);
    std::function<void()> cleanup = [registration] { registration->release(); };
    std::function<std::optional<int64_t>()> poll = [ring, min_complete, submitted]() -> std::optional<int64_t> {
        if (ring->refresh() >= min_complete)
            return submitted;
        return {};
    };
    {
        auto &state = waits();
        std::lock_guard<std::mutex> lock(state.mutex);
        state.pending[task] = {submitted, ring};
    }
    try {
        scheduler.block({}, 0, std::move(poll));
        task->wait_cleanup = std::move(cleanup);
    } catch (...) {
        forget_wait(task);
        throw;
    }
    return submitted;
}

int64_t register_op(Memory &memory, const std::shared_ptr<Ring> &ring, uint32_t opcode,
                    uint64_t argument, uint32_t count, bool async_files) {
    if (!ring)
        guest_error(95);
    if (opcode != 8)
        guest_error(22);
    if (!argument || count > 256)
        guest_error(22);
    count = std::min<uint32_t>(count, op_last);
    const size_t bytes = sizeof(ProbeHeader) + size_t(count) * sizeof(ProbeOp);
    if (!memory.accessible(argument, bytes, read_perm | write_perm))
        guest_error(14);
    std::vector<uint8_t> data(bytes);
    memory.copy_out(argument, data.data(), bytes);
    if (std::any_of(data.begin(), data.end(), [](uint8_t byte) { return byte != 0; }))
        guest_error(22);
    ProbeHeader header{};
    header.last_op = op_last - 1;
    header.ops_len = uint8_t(count);
    std::memcpy(data.data(), &header, sizeof(header));
    for (uint32_t index = 0; index < count; ++index) {
        ProbeOp op{};
        op.op = uint8_t(index);
        if (index == op_nop || (async_files && (index == op_readv || index == op_writev ||
            index == op_fsync || index == op_read || index == op_write)))
            op.flags = 1;
        std::memcpy(data.data() + sizeof(header) + index * sizeof(op), &op, sizeof(op));
    }
    memory.copy_in(argument, data.data(), bytes);
    return 0;
}

void map_ring(const std::shared_ptr<Ring> &ring, Memory &memory, uint64_t address, uint64_t length,
              unsigned permissions, uint64_t flags, uint64_t offset, bool replace) {
    if (!ring)
        guest_error(95);
    ring->map(memory, ring, address, length, permissions, flags, offset, replace);
}
void refresh_readiness(const std::shared_ptr<Ring> &ring) {
    if (ring)
        ring->refresh();
}
std::optional<int64_t> interrupt_wait(Task &task) {
    auto &state = waits();
    int64_t submitted;
    std::shared_ptr<Ring> ring;
    {
        std::lock_guard<std::mutex> lock(state.mutex);
        auto found = state.pending.find(&task);
        if (found == state.pending.end())
            return {};
        submitted = found->second.submitted;
        ring = found->second.ring.lock();
    }
    // Linux returns success on interruption when any CQE is already visible,
    // even if the requested minimum has not been reached.
    if (!submitted && (!ring || !ring->refresh()))
        return {};
    if (task.signals.restart)
        task.signals.restart->interrupted = false;
    return submitted;
}
} // namespace operit::arm64::io_uring
