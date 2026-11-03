#include "memory/address_space.h"
#include <stdexcept>
#include <utility>
#include <vector>

using namespace operit::arm64;

namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
} // namespace

int main() {
    constexpr uint64_t base = 0x10000;
    Memory memory(8 << 20);
    memory.map(base, page_size * 3, read_perm | write_perm);
    memory.store(base + 8, 1, 4);
    memory.store(base + page_size + 8, 2, 4);
    const std::vector<std::pair<uint64_t, uint64_t>> ranges{{base + 8, page_size}};
    const auto lease = memory.capture_write_lease(ranges);
    require(bool(lease) && lease(memory), "initial lease");
    memory.store(base + 12, 3, 4);
    require(lease(memory), "ordinary guest writes retain the lease");

    auto child = memory.fork_clone();
    require(!lease(*child), "a lease belongs to one address space");
    memory.protect(base, page_size, read_perm);
    require(!lease(memory), "non-writable buffer");
    memory.protect(base, page_size, read_perm | write_perm);
    require(lease(memory), "restored permissions on the original pages");
    memory.store(base + 8, 4, 4);
    require(!lease(memory), "COW page replacement invalidates the lease");

    const auto replacement_lease = memory.capture_write_lease(ranges);
    require(bool(replacement_lease) && replacement_lease(memory), "replacement page lease");
    memory.unmap(base + 2 * page_size, page_size);
    require(replacement_lease(memory), "unrelated unmap preserves the lease");
    memory.unmap(base, page_size);
    memory.map(base, page_size, read_perm | write_perm);
    memory.store(base + 8, 5, 4);
    require(!replacement_lease(memory), "unmap and remap at the same address");

    Memory other(8 << 20);
    other.map(base, page_size * 2, read_perm | write_perm);
    other.store(base + 8, 5, 4);
    other.store(base + page_size + 8, 6, 4);
    require(!replacement_lease(other), "new address space at the same addresses");
}
