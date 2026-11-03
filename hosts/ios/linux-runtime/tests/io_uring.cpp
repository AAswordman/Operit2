#include "fs/descriptor_table.h"
#include "linux/abi.h"
#include "linux/error_mapping.h"
#include "linux/files/memory_file.h"
#include "linux/io_uring/io_uring.h"
#include "memory/address_space.h"
#include "memory/fault.h"
#include "runtime/tasks/scheduler.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cerrno>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <cstring>
#ifdef __APPLE__
#include <dlfcn.h>
#elif defined(__linux__)
#include <sys/syscall.h>
#endif
#include <fcntl.h>
#include <mutex>
#include <new>
#include <poll.h>
#include <stdexcept>
#include <string>
#include <sys/stat.h>
#include <thread>
#include <unistd.h>
#include <vector>

#if defined(__APPLE__) || defined(__linux__)
namespace {
std::atomic<int> failing_sync_fd{-1};
std::atomic<unsigned> failing_sync_calls{0};

enum class GatedOperation { None, Read, Write, Sync };
struct ControlledIo {
    std::mutex mutex;
    std::condition_variable changed;
    GatedOperation operation = GatedOperation::None;
    unsigned entered = 0, waiting = 0, completed = 0;

    bool wait(GatedOperation kind) {
        std::unique_lock<std::mutex> lock(mutex);
        if (operation != kind)
            return false;
        ++entered;
        ++waiting;
        changed.notify_all();
        changed.wait_for(lock, std::chrono::seconds(15), [&] { return operation != kind; });
        --waiting;
        changed.notify_all();
        return true;
    }
    void finish() {
        std::lock_guard<std::mutex> lock(mutex);
        ++completed;
        changed.notify_all();
    }
};
ControlledIo &controlled_io() {
    static ControlledIo state;
    return state;
}
class ScopedIoGate {
    ControlledIo &state_ = controlled_io();
  public:
    explicit ScopedIoGate(GatedOperation kind) {
        std::lock_guard<std::mutex> lock(state_.mutex);
        state_.entered = 0;
        state_.completed = 0;
        state_.operation = kind;
    }
    bool wait_for(unsigned count) {
        std::unique_lock<std::mutex> lock(state_.mutex);
        return state_.changed.wait_for(lock, std::chrono::seconds(10),
                                       [&] { return state_.entered >= count; });
    }
    bool wait_finished(unsigned count) {
        std::unique_lock<std::mutex> lock(state_.mutex);
        return state_.changed.wait_for(lock, std::chrono::seconds(10),
                                       [&] { return state_.completed >= count; });
    }
    void release() {
        std::unique_lock<std::mutex> lock(state_.mutex);
        state_.operation = GatedOperation::None;
        state_.changed.notify_all();
        state_.changed.wait(lock, [&] { return !state_.waiting; });
    }
    ~ScopedIoGate() { release(); }
};
}
extern "C" int fsync(int fd) {
    const bool gated = controlled_io().wait(GatedOperation::Sync);
    struct stat actual{}, expected{};
    const int target = failing_sync_fd.load();
    int status;
    if (target >= 0 && ::fstat(fd, &actual) == 0 && ::fstat(target, &expected) == 0 &&
        actual.st_dev == expected.st_dev && actual.st_ino == expected.st_ino) {
        ++failing_sync_calls;
        errno = EIO;
        status = -1;
    } else {
#ifdef __APPLE__
        using Sync = int (*)(int);
        static const auto host_sync = reinterpret_cast<Sync>(dlsym(RTLD_NEXT, "fsync"));
        if (!host_sync) {
            errno = EIO;
            status = -1;
        } else
            status = host_sync(fd);
#else
        status = int(::syscall(SYS_fsync, fd));
#endif
    }
    const int error = errno;
    if (gated)
        controlled_io().finish();
    errno = error;
    return status;
}
extern "C" ssize_t pread(int fd, void *bytes, size_t count, off_t offset) {
    controlled_io().wait(GatedOperation::Read);
#ifdef __APPLE__
    using Read = ssize_t (*)(int, void *, size_t, off_t);
    static const auto host_read = reinterpret_cast<Read>(dlsym(RTLD_NEXT, "pread"));
    if (!host_read) {
        errno = EIO;
        return -1;
    }
    return host_read(fd, bytes, count, offset);
#else
    return ::syscall(SYS_pread64, fd, bytes, count, offset);
#endif
}
extern "C" ssize_t pwrite(int fd, const void *bytes, size_t count, off_t offset) {
    controlled_io().wait(GatedOperation::Write);
#ifdef __APPLE__
    using Write = ssize_t (*)(int, const void *, size_t, off_t);
    static const auto host_write = reinterpret_cast<Write>(dlsym(RTLD_NEXT, "pwrite"));
    if (!host_write) {
        errno = EIO;
        return -1;
    }
    return host_write(fd, bytes, count, offset);
#else
    return ::syscall(SYS_pwrite64, fd, bytes, count, offset);
#endif
}
#endif

using namespace operit::arm64;
namespace uring = operit::arm64::io_uring;
namespace {
constexpr uint64_t params_address = 0x10000, buffer = 0x11000;
constexpr uint64_t sq_address = 0x100000, cq_address = 0x200000, sqes_address = 0x300000;
struct SqOffsets {
    uint32_t head, tail, mask, entries, flags, dropped, array, reserved;
    uint64_t user_address;
};
struct CqOffsets {
    uint32_t head, tail, mask, entries, overflow, cqes, flags, reserved;
    uint64_t user_address;
};
struct Params {
    uint32_t sq_entries, cq_entries, flags, sq_cpu, sq_idle, features, wq_fd, reserved[3];
    SqOffsets sq;
    CqOffsets cq;
};
struct Sqe {
    uint8_t opcode = 0, flags = 0;
    uint16_t ioprio = 0;
    int32_t fd = -1;
    uint64_t offset = 0, address = 0;
    uint32_t length = 0, operation_flags = 0;
    uint64_t user_data = 0;
    uint16_t buffer_index = 0, personality = 0;
    uint32_t file_index = 0;
    uint64_t address3 = 0, reserved = 0;
};
struct Cqe { uint64_t user_data; int32_t result; uint32_t flags; };
static_assert(sizeof(Params) == 120 && sizeof(Sqe) == 64 && sizeof(Cqe) == 16);

void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
template <typename F> int64_t result(F operation) {
    try {
        return operation();
    } catch (const GuestError &error) {
        return -error.number;
    } catch (const MemoryFault &) {
        return -14;
    }
}
uint64_t page_round(uint64_t value) {
    return (value + page_size - 1) & ~(page_size - 1);
}

struct Fixture {
    std::shared_ptr<Memory> memory_owner = std::make_shared<Memory>(32 << 20);
    Memory &memory = *memory_owner;
    DescriptorTable descriptors;
    Scheduler scheduler;
    std::shared_ptr<uring::Ring> ring;
    std::shared_ptr<uring::CompletionContext> context = uring::make_completion_context();
    uring::AsyncSubmission async;
    Params p{};
    int ring_fd = -1;
    uint64_t ring_length = 0, sqes_length = 0;

    explicit Fixture(uint32_t entries = 2, uint32_t cqsize = 0) {
        async.context = context;
        async.memory = memory_owner;
        // The unit fixture does not remap buffers while I/O is pending. The
        // guest integration test uses the production mapping-identity lease.
        async.capture_mapping = [](Memory &, const std::vector<uring::MemoryRange> &ranges) {
            return [ranges](const Memory &target) {
                return std::all_of(ranges.begin(), ranges.end(), [&](const uring::MemoryRange &range) {
                    return target.accessible(range.address, range.length, write_perm);
                });
            };
        };
        memory.map(params_address, 0x8000, read_perm | write_perm);
        Params input{};
        if (cqsize) {
            input.flags = 8;
            input.cq_entries = cqsize;
        }
        memory.copy_in(params_address, &input, sizeof(input));
        auto setup = uring::setup(memory, descriptors, entries, params_address);
        ring_fd = setup.fd;
        ring = std::move(setup.ring);
        descriptors.get(ring_fd).description->io_uring = ring;
        memory.copy_out(params_address, &p, sizeof(p));
        ring_length = page_round(std::max<uint64_t>(p.sq.array + p.sq_entries * 4,
                                                    p.cq.cqes + p.cq_entries * 16));
        sqes_length = page_round(p.sq_entries * 64);
        uring::map_ring(ring, memory, sq_address, ring_length, 3, 1, 0, false);
        uring::map_ring(ring, memory, cq_address, ring_length, 3, 1, 0x8000000, false);
        uring::map_ring(ring, memory, sqes_address, sqes_length, 3, 1, 0x10000000, false);
    }
    uint32_t get(uint64_t address) { return uint32_t(memory.load(address, 4)); }
    void put(uint64_t address, uint32_t value) { memory.store(address, value, 4); }
    void queue(const Sqe &sqe) {
        const uint32_t tail = get(sq_address + p.sq.tail);
        const uint32_t index = tail & (p.sq_entries - 1);
        memory.copy_in(sqes_address + uint64_t(index) * sizeof(sqe), &sqe, sizeof(sqe));
        put(sq_address + p.sq.array + uint64_t(index) * 4, index);
        put(sq_address + p.sq.tail, tail + 1);
    }
    int64_t enter(uint32_t submit, uint32_t minimum = 0, uint32_t flags = 0, uint64_t arg = 0) {
        return result([&] {
            return uring::enter(memory, descriptors, scheduler, ring, submit, minimum, flags, arg, 0,
                                {}, [] { return false; }, &async);
        });
    }
    Cqe pop() {
        uint32_t head = get(cq_address + p.cq.head);
        const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
        while (head == get(cq_address + p.cq.tail) && std::chrono::steady_clock::now() < deadline) {
            uring::service_completions(context, scheduler);
            scheduler.poll_waiting();
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
        }
        require(head != get(cq_address + p.cq.tail), "missing completion");
        Cqe cqe{};
        memory.copy_out(cq_address + p.cq.cqes + uint64_t(head & (p.cq_entries - 1)) * sizeof(cqe),
                        &cqe, sizeof(cqe));
        put(cq_address + p.cq.head, head + 1);
        return cqe;
    }
    bool readable() {
        pollfd target{descriptors.get(ring_fd).host.get(), POLLIN, 0};
        require(::poll(&target, 1, 0) >= 0, "host poll");
        return target.revents & POLLIN;
    }
    int file() {
        char path[] = "/tmp/operit-uring-test-XXXXXX";
        int fd = ::mkstemp(path);
        require(fd >= 0, "mkstemp");
        require(::unlink(path) == 0, "unlink tempfile");
        return descriptors.insert(FD(fd), "", 2);
    }
};

void setup_and_mapping() {
    Memory memory(8 << 20);
    DescriptorTable descriptors;
    memory.map(params_address, 4096, 3);
    Params input{};
    auto attempt = [&](uint32_t entries, uint64_t address) {
        return result([&] { return int64_t(uring::setup(memory, descriptors, entries, address).fd); });
    };
    require(attempt(0, params_address) == -22, "zero entries");
    require(attempt(1, 0) == -14, "invalid params");
    input.reserved[2] = 1;
    memory.copy_in(params_address, &input, sizeof(input));
    require(attempt(1, params_address) == -22, "reserved params");
    input = {};
    input.flags = 2;
    memory.copy_in(params_address, &input, sizeof(input));
    require(attempt(1, params_address) == -22, "unsupported setup flags");
    input = {};
    input.flags = 8;
    input.cq_entries = 3;
    memory.copy_in(params_address, &input, sizeof(input));
    require(attempt(5, params_address) == -22, "cq smaller than rounded sq");

    Fixture f(3);
    require(f.p.sq_entries == 4 && f.p.cq_entries == 8 && (f.p.features & 15) == 15,
            "rounded sizes/features");
    require(f.descriptors.get(f.ring_fd).descriptor_flags == 1, "ring not close-on-exec");
    f.put(sq_address + f.p.cq.head, 3);
    require(f.get(cq_address + f.p.cq.head) == 3, "SQ/CQ aliases do not share storage");
    f.put(cq_address + f.p.cq.head, 0);
    require(result([&] {
        uring::map_ring(f.ring, f.memory, 0x400000, 4096, 3, 1, 0x20000000, false);
        return int64_t{0};
    }) == -22, "invalid ring mmap offset");
    require(result([&] {
        uring::map_ring(f.ring, f.memory, 0x400000, 4096, 3, 2, 0, false);
        return int64_t{0};
    }) == -95, "private ring mmap must be rejected");
    int duplicate = f.descriptors.duplicate(f.ring_fd);
    require(f.descriptors.get(duplicate).description == f.descriptors.get(f.ring_fd).description,
            "dup must share open description");
    auto child = f.memory.fork_clone();
    f.put(sq_address + f.p.sq.tail, 1);
    require(child->load(sq_address + f.p.sq.tail, 4) == 1, "fork lost shared mapping");
    std::weak_ptr<uring::Ring> weak = f.ring;
    f.descriptors.close(f.ring_fd);
    f.descriptors.close(duplicate);
    f.ring.reset();
    require(!weak.expired(), "mapping must retain closed ring");
    f.memory.unmap(sq_address, f.ring_length);
    f.memory.unmap(cq_address, f.ring_length);
    f.memory.unmap(sqes_address, f.sqes_length);
    require(!weak.expired(), "child mapping must retain ring");
    child.reset();
    require(weak.expired(), "last unmap must release ring");
}

void file_io_and_rejections() {
    Fixture f(4);
    const int fd = f.file();
    const char text[] = "hello ring";
    f.memory.copy_in(buffer, text, sizeof(text));
    Sqe write{};
    write.opcode = 23; write.fd = fd; write.address = buffer; write.length = sizeof(text);
    write.offset = 7; write.user_data = 41;
    f.queue(write);
    require(f.enter(1) == 1, "submit write");
    auto cqe = f.pop();
    require(cqe.user_data == 41 && cqe.result == int(sizeof(text)) && !cqe.flags, "write CQE");
    std::array<char, sizeof(text)> host{};
    require(::pread(f.descriptors.get(fd).host.get(), host.data(), host.size(), 7) == int(host.size()) &&
            std::memcmp(host.data(), text, host.size()) == 0, "write did not reach file");
    Sqe read = write;
    read.opcode = 22; read.address = buffer + 256; read.user_data = 42;
    f.queue(read);
    require(f.enter(1, 1, 1) == 1 && f.pop().result == int(sizeof(text)), "submit/read CQE");
    std::array<char, sizeof(text)> guest{};
    f.memory.copy_out(read.address, guest.data(), guest.size());
    require(std::memcmp(guest.data(), text, guest.size()) == 0, "read did not reach guest memory");
    read.address = 0;
    f.queue(read);
    require(f.enter(1) == 1 && f.pop().result == -14, "invalid guest buffer");
    read.address = buffer;
    read.fd = -1;
    f.queue(read);
    require(f.enter(1) == 1 && f.pop().result == -9, "invalid descriptor");
    read.fd = f.ring_fd;
    f.queue(read);
    require(f.enter(1) == 1 && f.pop().result == -95, "non-regular descriptor rejection");
    Sqe unsupported{};
    unsupported.opcode = 99; unsupported.user_data = 77;
    f.queue(unsupported);
    require(f.enter(1) == 1 && f.pop().result == -22, "unsupported opcode");
    Sqe link{};
    link.flags = 4; link.user_data = 1;
    f.queue(link);
    write.user_data = 2; write.offset = 100;
    f.queue(write);
    require(f.enter(2) == 2, "linked submission");
    require(f.pop().result == -95 && f.pop().result == -125, "linked operations not safely canceled");
    require(::pread(f.descriptors.get(fd).host.get(), host.data(), host.size(), 100) == 0,
            "canceled linked write changed file");
    require(f.enter(0, 0, 8) == -22 && f.enter(0, 1, 1, 0x10000) == -95,
            "unsupported enter options");
}

void vector_io_and_sync() {
    Fixture f(4);
    const int fd = f.file();
    const int host_fd = f.descriptors.get(fd).host.get();
    constexpr uint64_t vectors_address = buffer + 0x1800;
    const char first[] = "alpha", second[] = "BETA";
    f.memory.copy_in(buffer + 0x100, first, sizeof(first) - 1);
    f.memory.copy_in(buffer + 0x200, second, sizeof(second) - 1);
    auto vectors = [&](std::initializer_list<linux_abi::Iovec> entries) {
        f.memory.copy_in(vectors_address, entries.begin(), entries.size() * sizeof(linux_abi::Iovec));
    };
    uint64_t token = 100;
    auto submit = [&](Sqe request, int expected, const char *message) {
        request.user_data = ++token;
        f.queue(request);
        require(f.enter(1, 1, 1) == 1, "submit vector/sync SQE");
        const Cqe event = f.pop();
        require(event.user_data == token && event.flags == 0, "vector/sync CQE identity");
        require(event.result == expected, message);
    };
    require(::lseek(host_fd, 3, SEEK_SET) == 3, "set initial file position");
    vectors({{buffer + 0x100, 5}, {0, 0}, {buffer + 0x200, 4}});
    Sqe request{};
    request.opcode = 2; request.fd = fd; request.address = vectors_address;
    request.length = 3; request.offset = 7;
    submit(request, 9, "positioned WRITEV");
    std::array<char, 9> host{};
    require(::pread(host_fd, host.data(), host.size(), 7) == 9 &&
            std::memcmp(host.data(), "alphaBETA", 9) == 0, "WRITEV did not write file");
    require(::lseek(host_fd, 0, SEEK_CUR) == 3, "positioned WRITEV changed file position");

    std::array<char, 12> sentinel{};
    sentinel.fill('.');
    f.memory.copy_in(buffer + 0x400, sentinel.data(), sentinel.size());
    vectors({{buffer + 0x400, 3}, {buffer + 0x403, 9}});
    request.opcode = 1; request.length = 2;
    submit(request, 9, "short READV at EOF");
    std::array<char, 12> received{};
    f.memory.copy_out(buffer + 0x400, received.data(), received.size());
    require(std::memcmp(received.data(), "alphaBETA...", received.size()) == 0,
            "READV scatter or short read corrupted buffers");
    require(::lseek(host_fd, 0, SEEK_CUR) == 3, "positioned READV changed file position");

    require(::lseek(host_fd, 0, SEEK_SET) == 0, "rewind before current-position write");
    vectors({{buffer + 0x100, 5}, {buffer + 0x200, 4}});
    request.opcode = 2; request.offset = UINT64_MAX;
    submit(request, 9, "current-position WRITEV");
    require(::lseek(host_fd, 0, SEEK_CUR) == 9, "WRITEV did not advance file position");
    require(::lseek(host_fd, 7, SEEK_SET) == 7, "seek before current-position read");
    vectors({{buffer + 0x500, 2}, {buffer + 0x600, 3}});
    request.opcode = 1;
    submit(request, 5, "current-position READV");
    std::array<char, 5> current{};
    f.memory.copy_out(buffer + 0x500, current.data(), 2);
    f.memory.copy_out(buffer + 0x600, current.data() + 2, 3);
    require(std::memcmp(current.data(), "TApha", 5) == 0 &&
            ::lseek(host_fd, 0, SEEK_CUR) == 12, "READV current position or scatter");

    vectors({{buffer + 0x100, 3}, {0, 4}});
    request.opcode = 2; request.offset = 20;
    submit(request, 3, "WRITEV partial source fault");
    require(::pread(host_fd, host.data(), 3, 20) == 3 &&
            std::memcmp(host.data(), "alp", 3) == 0, "partial WRITEV data");
    vectors({{buffer + 0x700, 3}, {0, 4}});
    request.opcode = 1;
    submit(request, 3, "READV partial destination fault");
    f.memory.copy_out(buffer + 0x700, host.data(), 3);
    require(std::memcmp(host.data(), "alp", 3) == 0, "partial READV data");
    vectors({{0, 1}});
    request.length = 1; request.offset = UINT64_MAX;
    require(::lseek(host_fd, 20, SEEK_SET) == 20, "seek before fault");
    submit(request, -14, "READV first destination fault");
    require(::lseek(host_fd, 0, SEEK_CUR) == 20, "fault advanced current position");
    require(::lseek(host_fd, 0, SEEK_END) == 23, "seek to EOF");
    submit(request, 0, "READV inaccessible destination at EOF");
    require(::lseek(host_fd, 0, SEEK_CUR) == 23, "EOF advanced current position");

    request.address = 0; request.length = 0;
    submit(request, 0, "empty READV");
    request.length = 1;
    submit(request, -14, "invalid iovec table");
    request.address = vectors_address; request.length = 1025;
    submit(request, -22, "too many iovecs");
    request.length = 1;
    vectors({{buffer, UINT64_MAX}});
    submit(request, -22, "invalid iovec length");
    vectors({{address_limit - 1, 2}});
    submit(request, -14, "overflowing iovec address");
    vectors({{buffer + 0x100, 2}});
    request.offset = INT64_MAX;
    submit(request, -22, "positioned vector range overflow");
    request.offset = 0; request.fd = -1;
    submit(request, -9, "invalid vector file descriptor");
    request.fd = fd; request.operation_flags = 1;
    submit(request, -95, "unsupported vector flags");
    request.operation_flags = 0;
    int readonly = f.descriptors.insert(FD(::dup(host_fd)), "", 0);
    require(f.descriptors.get(readonly).host.get() >= 0, "duplicate read-only guest descriptor");
    request.fd = readonly; request.opcode = 2;
    submit(request, -9, "WRITEV on read-only descriptor");
    f.descriptors.get(fd).description->read_only = true;
    request.fd = fd;
    submit(request, -30, "WRITEV on read-only mount");
    f.descriptors.get(fd).description->read_only = false;
    f.descriptors.get(fd).description->standard_stream = 1;
    request.offset = UINT64_MAX;
    submit(request, -95, "current-position WRITEV must not bypass stream callbacks");
    f.descriptors.get(fd).description->standard_stream = -1;

    const int old_flags = ::fcntl(host_fd, F_GETFL);
    require(old_flags >= 0 && ::fcntl(host_fd, F_SETFL, old_flags | O_APPEND) == 0,
            "enable append");
    f.descriptors.get(fd).description->flags |= 1024;
    require(::lseek(host_fd, 4, SEEK_SET) == 4, "seek before positioned append");
    request.offset = 0;
    submit(request, -95, "positioned append WRITEV must be rejected");
    require(::lseek(host_fd, 0, SEEK_END) == 23 && ::lseek(host_fd, 4, SEEK_SET) == 4,
            "rejected positioned append changed the file");
    require(::fcntl(host_fd, F_SETFL, old_flags) == 0, "disable append");
    f.descriptors.get(fd).description->flags &= ~1024;

    f.descriptors.resource_limits()->set(process::Resource::FileSize, {32, process::resource_infinity});
    vectors({{buffer + 0x100, 5}, {buffer + 0x200, 4}});
    request.length = 2; request.offset = 30;
    submit(request, 2, "WRITEV must honor file size limit with a short write");
    request.offset = 32;
    submit(request, -27, "WRITEV must report EFBIG at file size limit");
    f.descriptors.resource_limits()->set(process::Resource::FileSize,
                                         {process::resource_infinity, process::resource_infinity});
    const char name[] = "uring-sealed";
    f.memory.copy_in(buffer + 0x800, name, sizeof(name));
    const int sealed = files::memory_file_create(f.memory, f.descriptors, buffer + 0x800,
                                                  files::mfd_allow_sealing);
    files::memory_file_add_seals(f.descriptors.get(sealed), files::memory_file_seal_write);
    request.fd = sealed; request.offset = 0;
    submit(request, -95, "asynchronous memfd writes must be rejected");

    constexpr uint64_t page_end = 0x18000;
    f.memory.copy_in(page_end - 3, first, 3);
    vectors({{page_end - 3, 6}});
    request.fd = fd; request.offset = 40; request.length = 1;
    submit(request, 3, "WRITEV partial fault within one iovec");
    require(::pread(host_fd, host.data(), 3, 40) == 3 &&
            std::memcmp(host.data(), "alp", 3) == 0, "cross-page WRITEV data");
    request.opcode = 1;
    submit(request, 3, "READV partial fault within one iovec");
    f.memory.copy_out(page_end - 3, host.data(), 3);
    require(std::memcmp(host.data(), "alp", 3) == 0, "cross-page READV data");

    request = {};
    request.opcode = 3; request.fd = fd;
    f.descriptors.get(fd).description->standard_stream = 1;
    submit(request, -95, "FSYNC must not claim durability for stream callbacks");
    f.descriptors.get(fd).description->standard_stream = -1;
    submit(request, 0, "FSYNC");
#if defined(__APPLE__) || defined(__linux__)
    failing_sync_fd = host_fd;
    submit(request, -5, "FSYNC must report host EIO");
    failing_sync_fd = -1;
    require(failing_sync_calls.load() == 1, "FSYNC did not call the host sync function");
#endif
    request.operation_flags = 1; request.offset = 3; request.length = 7;
    submit(request, 0, "FSYNC DATASYNC range");
    request.fd = readonly;
    submit(request, 0, "FSYNC read-only descriptor");
    request.fd = -1;
    submit(request, -9, "FSYNC invalid descriptor");
    request.fd = f.ring_fd;
    submit(request, -95, "FSYNC non-regular descriptor");
    request.fd = fd; request.operation_flags = 2;
    submit(request, -22, "FSYNC unknown flag");
    request.operation_flags = 0; request.address = buffer;
    submit(request, -22, "FSYNC unexpected address");
    request.address = 0; request.offset = INT64_MAX; request.length = 1;
    submit(request, -22, "FSYNC range overflow");
    request.offset = 0; request.length = 0;
    f.descriptors.get(fd).description->flags |= 2097152;
    submit(request, -9, "FSYNC on O_PATH descriptor");
    f.descriptors.get(fd).description->flags &= ~2097152;
}

void overflow_and_probe() {
    Fixture f(2, 2);
    Sqe nop{};
    for (int batch = 0; batch < 2; ++batch) {
        nop.user_data = uint64_t(batch * 2 + 1); f.queue(nop);
        nop.user_data = uint64_t(batch * 2 + 2); f.queue(nop);
        require(f.enter(2) == 2, "overflow submission");
    }
    require(f.readable(), "CQ completion must make ring readable");
    char marker = 0;
    require(::read(f.descriptors.get(f.ring_fd).host.get(), &marker, 1) == 1,
            "consume host marker for recovery test");
    uring::refresh_readiness(f.ring);
    require(f.readable(), "refresh must recover a lost readiness marker");
    require((f.get(sq_address + f.p.sq.flags) & 2) != 0, "overflow flag missing");
    require(f.get(cq_address + f.p.cq.overflow) == 0, "completion was dropped");
    require(f.pop().user_data == 1, "first CQE");
    require(f.enter(0, 0, 1) == 0, "flush overflow");
    require(f.pop().user_data == 2, "second CQE");
    uring::refresh_readiness(f.ring);
    require(f.pop().user_data == 3 && f.pop().user_data == 4, "overflow FIFO");
    uring::refresh_readiness(f.ring);
    require(!f.readable() && !(f.get(sq_address + f.p.sq.flags) & 2), "ring remains readable after drain");
    nop.user_data = 10; f.queue(nop);
    const uint32_t head = f.get(sq_address + f.p.sq.head);
    f.put(sq_address + f.p.sq.array + uint64_t(head & 1) * 4, 99);
    require(f.enter(1) == 0 && f.get(sq_address + f.p.sq.dropped) == 1 &&
            f.get(sq_address + f.p.sq.head) == head + 1, "invalid SQE index must be dropped");
    constexpr uint64_t probe = buffer;
    require(uring::register_op(f.memory, f.ring, 8, probe, 30, true) == 0, "probe");
    require(f.memory.load(probe, 1) == 58 && f.memory.load(probe + 1, 1) == 30, "probe header");
    require(f.memory.load(probe + 16 + 0 * 8 + 2, 2) == 1 &&
            f.memory.load(probe + 16 + 1 * 8 + 2, 2) == 1 &&
            f.memory.load(probe + 16 + 2 * 8 + 2, 2) == 1 &&
            f.memory.load(probe + 16 + 3 * 8 + 2, 2) == 1 &&
            f.memory.load(probe + 16 + 22 * 8 + 2, 2) == 1 &&
            f.memory.load(probe + 16 + 23 * 8 + 2, 2) == 1 &&
            f.memory.load(probe + 16 + 4 * 8 + 2, 2) == 0, "probe supported opcodes");
    require(result([&] { return uring::register_op(f.memory, f.ring, 8, probe, 30); }) == -22,
            "probe must start zeroed");
    require(result([&] { return uring::register_op(f.memory, f.ring, 4, probe, 0); }) == -22,
            "unsupported registration");
}

void io_allocation_failure_completion() {
    Fixture f(2, 2);
    const int fd = f.file();
    Sqe read{};
    read.opcode = 22;
    read.fd = fd;
    read.address = buffer;
    read.length = 1;
    read.user_data = 11;
    f.queue(read);
    auto failing = f.async;
    failing.capture_mapping = [](Memory &, const std::vector<uring::MemoryRange> &) -> uring::MappingLease {
        throw std::bad_alloc();
    };
    require(uring::enter(f.memory, f.descriptors, f.scheduler, f.ring, 1, 0, 0, 0, 0,
                         {}, {}, &failing) == 1, "allocation failure should complete the consumed SQE");
    const auto failed = f.pop();
    require(failed.user_data == 11 && failed.result == -12, "allocation failure CQE");
    Sqe nop{};
    nop.user_data = 12;
    f.queue(nop);
    require(f.enter(1) == 1 && f.pop().user_data == 12, "ring remains usable after allocation failure");
}

#if defined(__APPLE__) || defined(__linux__)
void async_descriptor_and_mapping_lifetime() {
    {
        Fixture f;
        const int fd = f.file();
        FD original(::dup(f.descriptors.get(fd).host.get()));
        require(original.get() >= 0, "duplicate original file");
        const char payload[] = "original data";
        f.memory.copy_in(buffer, payload, sizeof(payload));
        Sqe request{};
        request.opcode = 23; request.fd = fd; request.address = buffer;
        request.length = sizeof(payload); request.user_data = 201;
        ScopedIoGate gate(GatedOperation::Write);
        f.queue(request);
        const auto before = std::chrono::steady_clock::now();
        require(f.enter(1) == 1, "submit gated write");
        require(std::chrono::steady_clock::now() - before < std::chrono::seconds(3),
                "gated native write blocked submit");
        require(gate.wait_for(1), "worker did not enter gated write");
        require(!f.readable(), "ring readable before gated write completed");
        f.descriptors.close(fd);
        const int replacement = f.file();
        require(replacement == fd, "guest file descriptor was not reused");
        const char changed[] = "changed data!";
        f.memory.copy_in(buffer, changed, sizeof(changed));
        gate.release();
        const auto event = f.pop();
        require(event.user_data == 201 && event.result == int(sizeof(payload)), "gated write CQE");
        require(f.readable(), "completion did not signal ring readiness");
        std::array<char, sizeof(payload)> received{};
        require(::pread(original.get(), received.data(), received.size(), 0) == int(received.size()) &&
                std::memcmp(received.data(), payload, received.size()) == 0,
                "write did not retain the original descriptor and submitted bytes");
        require(::pread(f.descriptors.get(replacement).host.get(), received.data(), received.size(), 0) == 0,
                "write reached reused descriptor");
    }
    {
        Fixture f;
        const int fd = f.file();
        const char payload[] = "file data";
        require(::pwrite(f.descriptors.get(fd).host.get(), payload, sizeof(payload), 0) ==
                    int(sizeof(payload)), "prepare gated read");
        auto valid = std::make_shared<std::atomic<bool>>(true);
        f.async.capture_mapping = [valid](Memory &, const std::vector<uring::MemoryRange> &) {
            return [valid](const Memory &) { return valid->load(); };
        };
        Sqe request{};
        request.opcode = 22; request.fd = fd; request.address = buffer;
        request.length = sizeof(payload); request.user_data = 202;
        ScopedIoGate gate(GatedOperation::Read);
        f.queue(request);
        require(f.enter(1) == 1 && gate.wait_for(1), "submit gated read");
        valid->store(false);
        f.memory.unmap(buffer, page_size);
        f.memory.map(buffer, page_size, read_perm | write_perm);
        const char sentinel[] = "unchanged";
        f.memory.copy_in(buffer, sentinel, sizeof(sentinel));
        gate.release();
        const auto event = f.pop();
        require(event.user_data == 202 && event.result == -14, "invalidated read lease CQE");
        std::array<char, sizeof(sentinel)> received{};
        f.memory.copy_out(buffer, received.data(), received.size());
        require(std::memcmp(received.data(), sentinel, sizeof(sentinel)) == 0,
                "read overwrote a replacement mapping");
    }
    {
        Fixture f;
        const int fd = f.file();
        Sqe request{};
        request.opcode = 3; request.fd = fd;
        ScopedIoGate gate(GatedOperation::Sync);
        f.queue(request);
        require(f.enter(1) == 1 && gate.wait_for(1), "submit sync before context destruction");
        std::weak_ptr<uring::Ring> ring = f.ring;
        std::weak_ptr<Memory> memory = f.memory_owner;
        f.descriptors.close(fd);
        f.descriptors.close(f.ring_fd);
        f.memory.unmap(sq_address, f.ring_length);
        f.memory.unmap(cq_address, f.ring_length);
        f.memory.unmap(sqes_address, f.sqes_length);
        f.ring.reset();
        require(!ring.expired(), "pending completion must retain ring");
        f.async.context.reset();
        f.context.reset();
        require(ring.expired(), "destroying context retained ring during native I/O");
        f.memory_owner.reset();
        require(memory.expired(), "native worker retained destroyed guest memory");
        gate.release();
        require(gate.wait_finished(1), "abandoned native sync did not finish");
    }
}

void async_worker_limits() {
    constexpr uint32_t per_context = uring::max_async_in_flight_per_context;
    constexpr uint32_t per_ring = uring::max_async_in_flight_per_ring;
    constexpr uint32_t global = uring::max_async_in_flight_global;
    static_assert(per_context == per_ring && global == 2 * per_context && per_context % 2 == 0);
    Fixture first(per_ring), shared(per_ring), second(per_ring), excess(2);
    shared.context = first.context;
    shared.async.context = first.context;
    const int first_fd = first.file(), shared_fd = shared.file();
    const int second_fd = second.file(), excess_fd = excess.file();
    ScopedIoGate gate(GatedOperation::Sync);
    auto submit = [](Fixture &f, int fd, uint32_t count) {
        Sqe sync{};
        sync.opcode = 3; sync.fd = fd;
        for (uint32_t i = 0; i < count; ++i) {
            sync.user_data = i;
            f.queue(sync);
        }
        require(f.enter(count) == count, "submit gated sync batch");
    };
    submit(first, first_fd, per_context / 2);
    submit(shared, shared_fd, per_context / 2);
    require(gate.wait_for(per_context), "context workers did not enter gated sync");
    submit(first, first_fd, 1);
    require(first.pop().result == -11, "shared context did not enforce its worker limit");

    submit(second, second_fd, per_ring);
    require(gate.wait_for(global), "global workers did not enter gated sync");
    submit(second, second_fd, 1);
    require(second.pop().result == -16, "ring did not enforce its in-flight limit");
    submit(excess, excess_fd, 1);
    require(excess.pop().result == -11, "host process did not enforce its global worker limit");

    first.async.context.reset();
    shared.async.context.reset();
    first.context.reset();
    shared.context.reset();
    submit(excess, excess_fd, 1);
    require(excess.pop().result == -11, "destroyed context released active workers too early");

    gate.release();
    require(gate.wait_finished(global), "released native sync workers did not finish");
    for (uint32_t i = 0; i < per_ring; ++i)
        require(second.pop().result == 0, "released sync did not complete successfully");
    bool resumed = false;
    for (unsigned attempt = 0; attempt < 100 && !resumed; ++attempt) {
        submit(excess, excess_fd, 1);
        const int status = excess.pop().result;
        require(status == 0 || status == -11, "unexpected worker admission result");
        resumed = status == 0;
        if (!resumed)
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    require(resumed, "worker quota was not released after completion");
}
#endif

void scheduler_wait() {
    Fixture f(2, 2);
    Sqe nop{}; nop.user_data = 1; f.queue(nop);
    require(f.enter(1, 2, 1) == 1 && f.scheduler.current().state == Task::State::Waiting,
            "GETEVENTS did not wait");
    auto partial = uring::interrupt_wait(f.scheduler.current());
    require(partial && *partial == 1, "partial submission interrupt result");
    const uint32_t tid = f.scheduler.spawn(CpuState{}, 0, 0, 0);
    {
        Scheduler::ScopedTask scope(f.scheduler, tid);
        nop.user_data = 2; f.queue(nop);
        require(f.enter(1) == 1, "peer submission");
    }
    f.scheduler.poll_waiting();
    require(f.scheduler.find(1).state == Task::State::Runnable, "GETEVENTS did not wake");
    require(int64_t(f.scheduler.find(1).cpu.x[0]) == 1, "GETEVENTS returned wrong submitted count");
    require(!uring::interrupt_wait(f.scheduler.find(1)), "GETEVENTS did not clean up wait state");
    f.pop(); f.pop(); uring::refresh_readiness(f.ring);
    require(f.enter(0, 1, 1) == 0 && f.scheduler.find(1).state == Task::State::Waiting,
            "empty GETEVENTS must block");
    require(!uring::interrupt_wait(f.scheduler.find(1)), "empty wait should report EINTR");
    f.scheduler.interrupt(1, -4);
    require(int64_t(f.scheduler.find(1).cpu.x[0]) == -4, "empty wait interruption");
}

void interruption_and_lifetime() {
    {
        Fixture f(2, 2);
        Sqe nop{};
        f.queue(nop);
        require(f.enter(1) == 1, "prepare partial CQ");
        auto &task = f.scheduler.current();
        task.signals.restart.emplace();
        task.signals.restart->restartable = true;
        const auto interrupted = [&task] {
            task.signals.restart->interrupted = true;
            return true;
        };
        require(uring::enter(f.memory, f.descriptors, f.scheduler, f.ring, 0, 2, 1, 0, 0,
                             {}, interrupted) == 0,
                "pending signal with a nonempty CQ must return success");
        require(!task.signals.restart->interrupted, "completed wait marked for restart");
    }
    {
        Fixture f(2, 2);
        Sqe nop{};
        f.queue(nop);
        require(f.enter(1) == 1, "prepare CQ before waiting");
        auto &task = f.scheduler.current();
        task.signals.restart.emplace();
        task.signals.restart->restartable = true;
        require(f.enter(0, 2, 1) == 0 && task.state == Task::State::Waiting,
                "partial CQ should wait for the requested minimum");
        task.signals.restart->interrupted = true;
        f.scheduler.interrupt(task.tid, -4);
        require(int64_t(task.cpu.x[0]) == 0 && !task.signals.restart->interrupted,
                "signal with a nonempty CQ must preserve success");
    }
    {
        Fixture f(2, 2);
        Sqe nop{};
        f.queue(nop);
        auto &task = f.scheduler.current();
        task.signals.restart.emplace();
        task.signals.restart->restartable = true;
        require(f.enter(1, 2, 1) == 1 && task.state == Task::State::Waiting,
                "submitted request should wait for the requested minimum");
        task.signals.restart->interrupted = true;
        f.scheduler.interrupt(task.tid, -4);
        require(int64_t(task.cpu.x[0]) == 1 && !task.signals.restart->interrupted,
                "signal must preserve the number of submitted requests");
        require(f.pop().result == 0, "submitted completion must remain visible");
    }
    {
        Fixture f(2, 2);
        auto &task = f.scheduler.current();
        task.signals.restart.emplace();
        task.signals.restart->restartable = true;
        require(f.enter(0, 1, 1) == 0 && task.state == Task::State::Waiting,
                "empty CQ should wait");
        require(!task.signals.restart->restartable,
                "io_uring_enter GETEVENTS must not restart after a signal handler");
        task.signals.restart->interrupted = true;
        f.scheduler.interrupt(task.tid, -4);
        require(int64_t(task.cpu.x[0]) == -4, "empty CQ signal must return EINTR");
    }
    {
        Fixture f(2, 2);
        std::weak_ptr<uring::Ring> weak = f.ring;
        int duplicate = f.descriptors.duplicate(f.ring_fd);
        DescriptorTable child(f.descriptors);
        f.memory.unmap(sq_address, f.ring_length);
        f.memory.unmap(cq_address, f.ring_length);
        f.memory.unmap(sqes_address, f.sqes_length);
        f.ring.reset();
        f.descriptors.close(f.ring_fd);
        f.descriptors.close(duplicate);
        require(!weak.expired(), "forked descriptor table must retain the ring");
        child.close(f.ring_fd);
        require(!weak.expired(), "duplicate descriptor must retain the ring");
        child.close(duplicate);
        require(weak.expired(), "last descriptor and mapping must release the ring");
    }
    {
        Fixture f(2, 2);
        std::weak_ptr<uring::Ring> weak = f.ring;
        require(f.enter(0, 1, 1) == 0 && f.scheduler.current().state == Task::State::Waiting,
                "prepare wait retaining ring");
        f.descriptors.close(f.ring_fd);
        f.memory.unmap(sq_address, f.ring_length);
        f.memory.unmap(cq_address, f.ring_length);
        f.memory.unmap(sqes_address, f.sqes_length);
        f.ring.reset();
        require(!weak.expired(), "blocked wait must retain the ring after close and unmap");
        f.scheduler.interrupt(1, -4);
        require(weak.expired(), "completed wait must release the last ring reference");
    }
}
} // namespace

int main() {
    try {
        setup_and_mapping();
        file_io_and_rejections();
        vector_io_and_sync();
        overflow_and_probe();
        io_allocation_failure_completion();
#if defined(__APPLE__) || defined(__linux__)
        async_descriptor_and_mapping_lifetime();
        async_worker_limits();
#endif
        scheduler_wait();
        interruption_and_lifetime();
        std::puts("io_uring setup, mmap, scalar/vector I/O, fsync, CQ overflow and scheduler tests passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "io_uring test: %s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "io_uring test: Linux errno %d\n", error.number);
    }
    return 1;
}
