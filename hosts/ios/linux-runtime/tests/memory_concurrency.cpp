#include "memory/address_space.h"
#include <array>
#include <atomic>
#include <chrono>
#include <cstdio>
#include <future>
#include <stdexcept>
#include <thread>
#include <vector>
#include <unistd.h>

using namespace operit::arm64;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}

void resident_tlb_collisions() {
    Memory memory(8192);
    // Both pages hash to TLB slot 1. Populate before holding the reader gate.
    constexpr uint64_t first = 0x1000, second = 0x400000;
    memory.map(first, 4096, read_perm | write_perm);
    memory.map(second, 4096, read_perm | write_perm);
    memory.store<8>(first, 17);
    memory.store<8>(second, 29);
    std::future<bool> work;
    bool completed_without_writer = false;
    {
        memory_detail::AccessSynchronization::ReadGuard reader(true);
        work = std::async(std::launch::async, [&] {
            for (unsigned count = 0; count < 10000; ++count) {
                if (memory.load<8>(first) != 17 || memory.load<8>(second) != 29)
                    return false;
                memory.store<8>(first, 17);
                memory.store<8>(second, 29);
            }
            return true;
        });
        completed_without_writer = work.wait_for(std::chrono::seconds(2)) == std::future_status::ready;
    }
    // Release the blocker before joining even when the old writer path is used.
    require(work.get(), "resident TLB collision lost bytes");
    require(completed_without_writer, "resident TLB refills stopped other memory readers");
    auto child = memory.fork_clone();
    child->store<8>(first, 41);
    require(child->load<8>(first) == 41 && memory.load<8>(first) == 17,
            "resident refill bypassed fork COW");
    memory.protect(second, 4096, read_perm);
    try {
        memory.store<8>(second, 1);
        throw std::runtime_error("resident refill bypassed write protection");
    } catch (const MemoryFault &fault) {
        require(fault.kind == MemoryFault::Kind::Protection, "resident refill wrong fault kind");
    }
}

void serial_prepared_tlb_collisions() {
    Memory memory(3 * page_size);
    memory.set_parallel_access(false);
    // Executable/file mappings keep the reader gate active even when only one
    // task is runnable. Prepared pages can evict each other from the serial TLB.
    std::unique_ptr<FILE, decltype(&std::fclose)> file(std::tmpfile(), &std::fclose);
    require(bool(file) && !ftruncate(fileno(file.get()), page_size), "create serial file fixture");
    memory.map_file(0x800000, page_size, read_perm, fileno(file.get()), {"/serial-file"});
    constexpr uint64_t first = 0x1000, second = 0x400000;
    memory.map(first, page_size, read_perm | write_perm);
    memory.map(second, page_size, read_perm | write_perm);
    memory.store<8>(first, 17);
    memory.store<8>(second, 29);
    Memory::PreparedAccess prepared(memory);
    require(prepared.add(first, 8, read_perm) && prepared.add(second, 8, write_perm) && prepared.lock(),
            "serial collision pages did not prepare");
    memory.store<8>(second, memory.load<8>(first));
    require(memory.load<8>(second) == 17, "serial prepared TLB refill lost bytes");
}

void prepared_page_ranges() {
    Memory memory(5 * page_size);
    constexpr uint64_t first = 0x10000, second = 0x20000, cold = 0x30000, code = 0x40000;
    memory.map(first, page_size, read_perm | write_perm);
    memory.map(second, page_size, read_perm | write_perm);
    memory.map(cold, page_size, read_perm | write_perm);
    memory.map(code, page_size, read_perm | write_perm | exec_perm);
    memory.store<8>(first, 17);
    memory.store<8>(second, 29);
    memory.store<4>(code, 0xd503201f);
    // Declining a cold page releases the reader gate before writer fallback.
    {
        Memory::PreparedAccess prepared(memory);
        require(!prepared.add(cold, 8, read_perm), "cold helper range did not decline");
        const Memory::AccessGuard fallback(memory);
        memory.store<8>(cold, 41);
    }
    {
        Memory::PreparedAccess prepared(memory);
        require(prepared.add(first, 8, read_perm) && prepared.add(first, 8, write_perm) &&
                    prepared.add(code, 4, exec_perm) && prepared.add(code, 4, write_perm) && prepared.lock(),
                "overlapping code/data helper pages did not prepare");
        memory.store<8>(first, memory.load<8>(first) + 1);
        // The executable/data alias must take one exclusive lock, not upgrade
        // a shared code lock recursively after acquisition.
        memory.store<4>(code, 0xd503201f);
        require(memory.accessible(first, 8, write_perm), "metadata query attempted writer upgrade");
    }
    auto child = memory.fork_clone();
    {
        Memory::PreparedAccess prepared(*child);
        require(!prepared.add(first, 8, write_perm), "prepared helper bypassed fork COW");
        const Memory::AccessGuard fallback(*child);
        child->store<8>(first, 99);
    }
    require(memory.load<8>(first) == 18, "prepared fallback changed parent COW bytes");
    memory.protect(second, page_size, read_perm);
    {
        Memory::PreparedAccess prepared(memory);
        require(!prepared.add(second, 8, write_perm), "prepared helper bypassed write protection");
    }
    {
        Memory::PreparedAccess prepared(memory);
        require(!prepared.add(first, 17 * page_size, read_perm), "oversized prepared helper did not decline");
    }
}

void immutable_page_readers() {
    Memory memory(page_size);
    constexpr uint64_t address = 0x10000;
    memory.map(address, page_size, read_perm | write_perm);
    memory.store<8>(address, 73);
    memory.protect(address, page_size, read_perm);
    std::future<bool> reader;
    bool progressed;
    {
        // An exclusive-load reservation changes only its token. Readers of
        // immutable private bytes need not contend on that page's lock.
        const Memory::AtomicAccess reservation(memory, address, read_perm);
        reader = std::async(std::launch::async, [&] { return memory.load<8>(address) == 73; });
        progressed = reader.wait_for(std::chrono::seconds(2)) == std::future_status::ready;
    }
    require(reader.get(), "immutable page reader lost bytes");
    require(progressed, "immutable page reader still contended on a page lock");
}

void page_readers_and_writers() {
    Memory memory(4096);
    constexpr uint64_t address = 0x10000;
    memory.map(address, 4096, read_perm | write_perm);
    const std::array<uint64_t, 2> initial{0, ~uint64_t{0}};
    memory.copy_in<16>(address, initial.data());
    std::vector<std::future<void>> work;
    for (unsigned id = 0; id < 4; ++id) {
        work.push_back(std::async(std::launch::async, [&, id] {
            for (uint64_t index = 1; index <= 10000; ++index) {
                if (id < 2) {
                    const uint64_t value = (uint64_t{id} << 32) | index;
                    const std::array<uint64_t, 2> pair{value, ~value};
                    memory.copy_in<16>(address, pair.data());
                } else {
                    std::array<uint64_t, 2> pair{};
                    memory.copy_out<16>(address, pair.data());
                    require(pair[1] == ~pair[0], "page reader observed a partial write");
                }
            }
        }));
    }
    for (auto &job : work)
        job.get();

    memory.store<8>(address, 0);
    work.clear();
    for (unsigned id = 0; id < 4; ++id) {
        work.push_back(std::async(std::launch::async, [&] {
            for (unsigned index = 0; index < 10000; ++index) {
                const Memory::AtomicAccess atomic(memory, address, read_perm | write_perm);
                memory.store<8>(address, memory.load<8>(address) + 1);
            }
        }));
    }
    for (auto &job : work)
        job.get();
    require(memory.load<8>(address) == 40000, "nested page guards lost atomic updates");
}
} // namespace

int main() {
    try {
        resident_tlb_collisions();
        serial_prepared_tlb_collisions();
        prepared_page_ranges();
        immutable_page_readers();
        page_readers_and_writers();
        std::puts("resident TLB progress, COW and concurrent page access passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
        return 1;
    }
}
