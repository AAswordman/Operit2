#include "linux/files/close_range.h"
#include "base/support.h"
#include "fs/descriptor_table.h"
#include <new>
#include <vector>

namespace operit::arm64::files {
int64_t close_range(DescriptorTable &descriptors, uint32_t first, uint32_t last, uint32_t flags,
                    const CloseRangeBeforeClose &before_close) {
    constexpr uint32_t known_flags = close_range_unshare | close_range_cloexec;
    if (first > last || (flags & ~known_flags))
        guest_error(22); // EINVAL, without descriptor-table side effects.

    std::vector<int> numbers;
    try {
        // Snapshot only allocated FD numbers, not every integer in the range.
        // Allocate it before detaching so snapshot allocation cannot fail after
        // a successful unshare. unshare() copies descriptor state, not OFDs.
        numbers = descriptors.numbers();
        if (flags & close_range_unshare)
            descriptors.unshare();
    } catch (const std::bad_alloc &) {
        guest_error(12); // ENOMEM while preparing the snapshot or private table.
    }

    for (const int fd : numbers) {
        if (fd < 0)
            continue;
        const uint32_t number = static_cast<uint32_t>(fd);
        // Do not add one to last: UINT32_MAX must not wrap around.
        if (number < first || number > last)
            continue;

        Descriptor *entry;
        try {
            entry = &descriptors.get(number);
        } catch (const GuestError &error) {
            if (error.number == 9) // A missing descriptor is an ignored hole.
                continue;
            throw;
        }
        if (flags & close_range_cloexec) {
            // FD_CLOEXEC is per descriptor. Do not modify OpenDescription flags,
            // close a backing handle, or run any descriptor-close callbacks.
            entry->descriptor_flags |= 1;
            continue;
        }

        if (before_close) {
            try {
                before_close(*entry);
            } catch (const GuestError &) {
                // Per-file close errors do not stop the range or keep the FD.
            }
        }
        try {
            // Do not retain an OpenDescription reference: erasing the descriptor
            // releases terminal/OFD resources at their normal last-reference time.
            descriptors.close(number);
        } catch (const GuestError &) {
            // Linux close_range does not report individual close errors.
        }
    }
    return 0;
}
} // namespace operit::arm64::files
