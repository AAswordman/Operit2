#include "linux/namespaces/namespaces.h"
#include "linux/syscalls.h"

namespace operit::arm64 {
int64_t Syscalls::dispatch_namespace(CpuState &cpu) {
    const bool task_scoped = scheduler_.live_tids(scheduler_.current().pid).size() == 1;
    if (cpu.x[8] == 97) {
        namespaces::UnshareContext context{filesystem_, descriptors_, task_scoped, false, {}, nullptr};
        context.prepare_fs = [this](const Filesystem *mounts) -> std::unique_ptr<namespaces::PreparedChange> {
            if (mounts)
                return {};
            return namespaces::replace_shared(fs_context_, std::make_shared<FsContext>(*fs_context_));
        };
        return namespaces::unshare(cpu.x[0], context);
    }
    namespaces::SetnsContext context{descriptors_, task_scoped, false, false, false, {}, {}, &filesystem_,
                                     nullptr};
    return namespaces::setns(cpu.x[0], cpu.x[1], context);
}
} // namespace operit::arm64
