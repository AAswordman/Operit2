#pragma once
#include "cpu/aarch64/state.h"
#include "fs/descriptor_table.h"
#include "fs/filesystem.h"
#include "linux/network/socket.h"
#include "linux/network/unix_namespace.h"
#include "linux/ipc/posix_mq.h"
#include "linux/ipc/sysv_msg.h"
#include "linux/ipc/sysv_sem.h"
#include "linux/ipc/sysv_shared_memory.h"
#include "linux/process/control.h"
#include "linux/process/resource_limits.h"
#include "linux/terminal/manager.h"
#include "linux/system/information.h"
#include "memory/address_space.h"
#include "operit/arm64_runtime.h"
#include "runtime/tasks/scheduler.h"
#include <array>
#include <sys/stat.h>

namespace operit::arm64 {
struct ProcSource;
namespace io_uring { class CompletionContext; }
class Syscalls {
    Memory &memory_;
    Filesystem &filesystem_;
    ort_arm64_config config_;
    DescriptorTable descriptors_;
    std::shared_ptr<TerminalManager> terminals_;
    std::shared_ptr<io_uring::CompletionContext> io_uring_context_;
    std::weak_ptr<Memory> io_uring_memory_;
    struct FsContext {
        std::string cwd = "/";
        uint32_t mask = 0022;
        std::shared_ptr<Path> directory;
    };
    std::shared_ptr<FsContext> fs_context_ = std::make_shared<FsContext>();
    std::string executable_;
    struct ProcessAnchor {
        Syscalls *syscalls;
        uint32_t pid = 1, parent = 0;
    };
    std::shared_ptr<ProcessAnchor> process_anchor_ = std::make_shared<ProcessAnchor>(ProcessAnchor{this});
    std::function<Syscalls &()> process_access(uint32_t pid = 0);
    ProcSource process_source(uint32_t pid = 0);
    std::shared_ptr<process::MemoryState> process_memory_ = std::make_shared<process::MemoryState>();
    void initialize_process_memory(uint64_t stack);
    std::string process_memory_contents(const std::string &name, uint64_t offset = 0,
                                        uint64_t count = UINT64_MAX);
    int64_t process_brk(uint64_t requested);
    void mount_process_filesystems();
    std::string process_contents(uint32_t tid, const std::string &name);
    VirtualNode::Ptr process_fd_link(int fd);
    struct stat descriptor_stat(const Descriptor &entry);
    int open_virtual(const Path &path, int host_flags, int guest_flags);
    int64_t virtual_getdents(Descriptor &entry, uint64_t address, size_t capacity);
    uint64_t mmap_next_ = 0x1000000000;
    Scheduler &scheduler_;
    std::shared_ptr<system::SamplingState> information_;
    std::shared_ptr<NetworkNamespace> network_ = std::make_shared<NetworkNamespace>();
    std::shared_ptr<ipc::PosixMqAccounting> posix_mq_accounting_ = ipc::make_posix_mq_accounting();
    std::shared_ptr<ipc::PosixMqNamespace> posix_mq_namespace_ = ipc::make_posix_mq_namespace(posix_mq_accounting_);
    std::shared_ptr<ipc::MessageQueueNamespace> message_queue_namespace_;
    std::shared_ptr<ipc::SemaphoreNamespace> semaphore_namespace_;
    std::shared_ptr<ipc::SharedMemoryNamespace> shared_memory_namespace_ = ipc::make_shared_memory_namespace();
    std::shared_ptr<ipc::SharedMemoryAddressSpace> shared_memory_space_ = ipc::make_shared_memory_address_space();
    struct IpcCredentials {
        uint32_t uid, euid, egid, fsuid, fsgid;
        std::vector<uint32_t> groups;
    };
    IpcCredentials ipc_credentials();
    std::shared_ptr<SocketState> new_socket_state(int domain, int type, int protocol);
    NativeAddress read_socket_address(uint64_t address, uint64_t length, bool binding);
    std::vector<uint8_t> socket_address_bytes(const sockaddr *address, socklen_t length);
    void write_socket_address(uint64_t address, uint64_t size_pointer, const std::vector<uint8_t> &bytes);
    int64_t socket_wait(Descriptor &entry, bool writing, bool nonblocking,
                        std::function<std::optional<int64_t>(int)> attempt,
                        std::function<int64_t()> timeout_value = {});
    int64_t receive_waitall(Descriptor &entry, const std::vector<std::pair<uint64_t, uint64_t>> &vectors,
                            uint64_t flags, std::function<void()> output = {});
    int64_t dispatch_socket(CpuState &cpu);
    int64_t socket_message(CpuState &cpu);
    int64_t dispatch_poll(CpuState &cpu);
    int64_t dispatch_inotify(CpuState &cpu);
    std::string directory_base(uint64_t fd);
    Path directory_path(uint64_t fd);
    Path resolve_path(uint64_t fd, const std::string &name, bool follow = true,
                      Filesystem::ResolveIntent intent = Filesystem::ResolveIntent::Lookup);
    Path at_path(uint64_t fd, uint64_t address, bool follow = true);
    static int open_flags(uint64_t guest);
    void check_buffer(uint64_t address, uint64_t length, unsigned permissions);
    int64_t io(bool writing, uint64_t fd, uint64_t address, uint64_t length, uint64_t offset = 0,
               bool positioned = false, bool may_block = true);
    int64_t vector_io(CpuState &cpu);
    void guest_stat(uint64_t destination, const struct stat &info);
    int64_t dispatch(CpuState &cpu);
    int64_t dispatch_identity(CpuState &cpu);
    int64_t dispatch_process_vm(CpuState &cpu);
    int64_t dispatch_path(CpuState &cpu);
    int64_t dispatch_mount(CpuState &cpu);
    int64_t dispatch_namespace(CpuState &cpu);
    int64_t dispatch_file(CpuState &cpu);
    int64_t dispatch_file_control(CpuState &cpu);
    int64_t dispatch_open(CpuState &cpu);
    int open_path(const Path &path, uint64_t guest_flags, mode_t mode);
    int64_t descriptor_control(CpuState &cpu);
    int64_t descriptor_ioctl(CpuState &cpu);
    int64_t dispatch_events(CpuState &cpu);
    int64_t dispatch_io_uring(CpuState &cpu);
    int64_t dispatch_ipc(CpuState &cpu);
    int64_t dispatch_timer(CpuState &cpu);
    bool interrupt_for_pending_signal();
    void broken_pipe(uint32_t tid);
    int64_t dispatch_memory(CpuState &cpu);
    int64_t dispatch_limits(CpuState &cpu);
    int64_t dispatch_system(CpuState &cpu);
    int64_t dispatch_signal(CpuState &cpu);
    int64_t dispatch_process(CpuState &cpu);
    int64_t dispatch_file_transfer(CpuState &cpu);
    int64_t terminal_ioctl(CpuState &cpu);
    int64_t terminal_io(bool writing, uint64_t fd, uint64_t address, uint64_t length, bool may_block);
    int64_t dispatch_threads(CpuState &cpu);
    int64_t dispatch_control(CpuState &cpu);
    void release_robust_list(Task &task);
    TaskDeadline thread_deadline(uint64_t address, bool absolute, bool realtime);
    int64_t unsupported(const CpuState &cpu);

  public:
    uint64_t calls = 0, unsupported_count = 0, last_unsupported = 0;
    ~Syscalls() { process_anchor_->syscalls = nullptr; }
    Syscalls(Memory &memory, Filesystem &filesystem, const ort_arm64_config &config, Scheduler &scheduler);
    void set_io_uring_context(std::shared_ptr<io_uring::CompletionContext> context,
                              const std::shared_ptr<Memory> &memory) {
        io_uring_context_ = std::move(context);
        io_uring_memory_ = memory;
    }
    void set_process_image(const std::string &path, uint64_t stack);
    void set_process_identity(uint32_t pid, uint32_t parent);
    const std::shared_ptr<TerminalManager> &terminal_manager() const { return terminals_; }
    const std::shared_ptr<process::ResourceLimits> &resource_limits() const {
        return descriptors_.resource_limits();
    }
    void inherit_process(const Syscalls &parent, uint64_t clone_flags, bool exec = false);
    void sync_shared_memory_mappings();
    void inherit_shared_memory_space(const Syscalls &parent, bool shared, uint32_t child_pid);
    void release_shared_memory_space(uint32_t pid) noexcept;
    void commit_ipc_exec();
    void exec_reset(const std::string &path, uint64_t stack, uint64_t heap_end);
    void release_task_resources(Task &task, bool preserve_sem_undo = false);
    void close_descriptors();
    TerminalOpen open_console(uint32_t columns, uint32_t rows);
    void set_heap(uint64_t end) {
        process_memory_->bounds.start_brk = process_memory_->bounds.brk = align_up(end);
    }
    void handle(CpuState &cpu);
};
} // namespace operit::arm64
