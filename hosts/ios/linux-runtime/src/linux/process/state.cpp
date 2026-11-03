#include "linux/syscalls.h"
#include "linux/files/record_lock.h"

namespace operit::arm64 {
void Syscalls::set_process_identity(uint32_t pid, uint32_t parent) {
    process_anchor_->pid = pid;
    process_anchor_->parent = parent;
}
void Syscalls::inherit_process(const Syscalls &parent, uint64_t clone_flags, bool exec) {
    auto limits = exec ? parent.resource_limits()
                       : process::inherit_resource_limits(parent.resource_limits(), clone_flags);
    if (clone_flags & 0x400)
        descriptors_.share_with(parent.descriptors_);
    else
        descriptors_ = parent.descriptors_;
    descriptors_.bind_resource_limits(std::move(limits));
    fs_context_ = clone_flags & 0x200 ? parent.fs_context_
                                     : std::make_shared<FsContext>(*parent.fs_context_);
    terminals_ = parent.terminals_;
    network_ = parent.network_;
    posix_mq_accounting_ = parent.posix_mq_accounting_;
    posix_mq_namespace_ = parent.posix_mq_namespace_;
    message_queue_namespace_ = parent.message_queue_namespace_;
    semaphore_namespace_ = parent.semaphore_namespace_;
    shared_memory_namespace_ = parent.shared_memory_namespace_;
    information_ = parent.information_;
    executable_ = parent.executable_;
    process_memory_ = clone_flags & 0x100 ? parent.process_memory_
                                         : std::make_shared<process::MemoryState>(*parent.process_memory_);
    mmap_next_ = parent.mmap_next_;
    mount_process_filesystems();
}
void Syscalls::exec_reset(const std::string &path, uint64_t stack, uint64_t heap_end) {
    descriptors_.unshare();
    descriptors_.close_on_exec([this](const Descriptor &entry) {
        files::record_lock_close(scheduler_, process_anchor_->pid, entry);
    });
    fs_context_ = std::make_shared<FsContext>(*fs_context_);
    process_memory_ = std::make_shared<process::MemoryState>();
    set_heap(heap_end);
    mmap_next_ = 0x1000000000;
    set_process_image(path, stack);
}
void Syscalls::close_descriptors() {
    ipc::posix_mq_process_exit(posix_mq_namespace_, process_anchor_->pid);
    files::record_lock_process_exit(scheduler_, process_anchor_->pid);
    DescriptorTable replacement;
    replacement.bind_resource_limits(resource_limits());
    descriptors_ = std::move(replacement);
}
} // namespace operit::arm64
