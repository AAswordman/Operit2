#include "linux/events/inotify.h"
#include "linux/syscalls.h"
#include <memory>

namespace operit::arm64 {
namespace {
// Linux AT_FDCWD differs from the host value on Darwin.
constexpr uint64_t guest_at_fdcwd = uint64_t(int64_t{-100});
} // namespace

int64_t Syscalls::dispatch_inotify(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (a[8]) {
    case 26: { // inotify_init1(int flags)
        const uint32_t flags = uint32_t(a[0]);
        auto file = std::make_shared<InotifyFile>(flags);
        const int fd = descriptors_.insert(file->duplicate_handle(), "", int(flags));
        auto &description = *descriptors_.get(fd).description;
        description.inotify = std::move(file);
        // The native pipe is only a readiness marker and always NONBLOCK;
        // guest blocking reads are resumed by Scheduler with this description.
        description.scheduler_io = true;
        return fd;
    }
    case 27: { // inotify_add_watch(int fd, const char *pathname, uint32_t mask)
        auto file = descriptors_.get(uint32_t(a[0])).description->inotify;
        if (!file)
            guest_error(22);
        const uint32_t mask = uint32_t(a[2]);
        InotifyFile::validate_mask(mask);
        // at_path uses the pinned cwd directory FD for relative names, even
        // after a rename. Never resolve from the cwd's display pathname.
        auto path = at_path(guest_at_fdcwd, a[1], !(mask & inotify::dont_follow));
        return file->add_watch(path, mask);
    }
    case 28: { // inotify_rm_watch(int fd, int wd)
        auto file = descriptors_.get(uint32_t(a[0])).description->inotify;
        if (!file)
            guest_error(22);
        file->remove_watch(int32_t(uint32_t(a[1])));
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
