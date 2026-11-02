#pragma once
#include "fs/descriptor_table.h"
#include "fs/filesystem.h"
#include "memory/address_space.h"
#include "runtime/tasks/scheduler.h"

namespace operit::arm64::files {
int64_t record_lock(Memory &memory, Filesystem &filesystem, Scheduler &scheduler, Descriptor &entry,
                    uint64_t command, uint64_t argument);
void record_lock_close(Scheduler &scheduler, uint32_t pid, const Descriptor &entry);
void record_lock_process_exit(Scheduler &scheduler, uint32_t pid);
void record_lock_vm_exit(Scheduler &scheduler);
void record_lock_description_closed(const OpenDescription *description) noexcept;
} // namespace operit::arm64::files
