#include "linux/syscalls.h"
#include "linux/files/advice.h"
#include "linux/files/allocation.h"
#include "linux/files/close_range.h"
#include "linux/files/record_lock.h"
#include "linux/process/limit_enforcement.h"

namespace operit::arm64 {
int64_t Syscalls::dispatch_file_control(CpuState &cpu) {
    const auto &a = cpu.x;
    if (a[8] != 436 && ipc::is_posix_mq_file(descriptors_.get(uint32_t(a[0])).description))
        guest_error(95);
    switch (a[8]) {
    case 47:
        return files::fallocate(memory_, descriptors_, a[0], uint32_t(a[1]), int64_t(a[2]),
                                 int64_t(a[3]), [this] {
            process::send_limit_signal(scheduler_, scheduler_.current().tid, 25);
        });
    case 84:
        return files::sync_file_range(memory_, descriptors_, int32_t(a[0]), int64_t(a[1]),
                                       int64_t(a[2]), uint32_t(a[3]));
    case 213:
        return files::readahead(memory_, descriptors_, int32_t(a[0]), int64_t(a[1]), a[2]);
    case 223:
        return files::fadvise64(memory_, descriptors_, int32_t(a[0]), int64_t(a[1]),
                                 int64_t(a[2]), int32_t(a[3]));
    case 436:
        return files::close_range(descriptors_, uint32_t(a[0]), uint32_t(a[1]), uint32_t(a[2]),
                                   [this](const Descriptor &entry) {
            files::record_lock_close(scheduler_, scheduler_.current().pid, entry);
            ipc::posix_mq_close(entry.description, scheduler_.current().pid);
        });
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
