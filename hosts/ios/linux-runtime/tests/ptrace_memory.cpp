#include "cpu/aarch64/exclusive.h"
#include "cpu/aarch64/interpreter.h"
#include "fs/descriptor_table.h"
#include "linux/files/memory_file.h"
#include "memory/address_space.h"
#include <array>
#include <cstdio>
#include <fcntl.h>
#include <stdexcept>
#include <string>
#include <unistd.h>

using namespace operit::arm64;

namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}

template <class Function> void protection_fault(Function function, const char *message) {
    try {
        function();
    } catch (const MemoryFault &fault) {
        require(fault.kind == MemoryFault::Kind::Protection, message);
        return;
    }
    throw std::runtime_error(message);
}

void same_mappings(const std::vector<Memory::Mapping> &before, const Memory &memory) {
    const auto after = memory.mappings();
    require(before.size() == after.size(), "ptrace changed VMA count");
    for (size_t i = 0; i < before.size(); ++i) {
        const auto &a = before[i];
        const auto &b = after[i];
        require(a.start == b.start && a.end == b.end && a.permissions == b.permissions &&
                    a.backing.path == b.backing.path && a.backing.offset == b.backing.offset &&
                    a.backing.device == b.backing.device && a.backing.inode == b.backing.inode &&
                    a.backing.shared == b.backing.shared,
                "ptrace changed visible VMA metadata");
    }
}

struct TempFile {
    int rw = -1, ro = -1;
    TempFile() {
        char path[] = "/tmp/operit-ptrace-memory-XXXXXX";
        rw = ::mkstemp(path);
        require(rw >= 0, "create temporary file");
        ro = ::open(path, O_RDONLY);
        const int unlinked = ::unlink(path);
        require(ro >= 0 && !unlinked, "open read-only temporary file");
    }
    ~TempFile() {
        if (ro >= 0)
            ::close(ro);
        if (rw >= 0)
            ::close(rw);
    }
    void resize(uint64_t bytes) {
        require(::ftruncate(rw, off_t(bytes)) == 0, "resize temporary file");
    }
    void put(uint64_t offset, const void *data, size_t bytes) {
        require(::pwrite(rw, data, bytes, off_t(offset)) == ssize_t(bytes), "write temporary file");
    }
    template <class Value> Value get(uint64_t offset) {
        Value value{};
        require(::pread(ro, &value, sizeof(value), off_t(offset)) == ssize_t(sizeof(value)),
                "read temporary file");
        return value;
    }
};

Memory::Backing backing(bool shared = false, uint64_t offset = 0) {
    return {"ptrace-fixture", offset, 0, 0, shared};
}

void permissions_and_boundaries(bool serial) {
    Memory memory(12 * page_size);
    memory.set_parallel_access(!serial);
    constexpr uint64_t base = 0x10000;
    memory.map(base, 2 * page_size, read_perm | write_perm);
    const std::array<uint8_t, 8> original{1, 2, 3, 4, 5, 6, 7, 8};
    const std::array<uint8_t, 8> replacement{8, 7, 6, 5, 4, 3, 2, 1};
    memory.copy_in(base + page_size - 4, original.data(), original.size());
    memory.protect(base, page_size, 0);
    memory.protect(base + page_size, page_size, exec_perm);
    memory.describe(base + 17, 3, {"[fragment]"});
    const auto mappings = memory.mappings();
    std::array<uint8_t, 8> actual{};
    require(memory.ptrace_read(base + page_size - 4, actual.data(), actual.size()) && actual == original,
            "force read across PROT_NONE/execute-only pages");
    require(memory.ptrace_write(base + page_size - 4, replacement.data(), replacement.size()),
            "force write across PROT_NONE/execute-only pages");
    require(memory.ptrace_read(base + page_size - 4, actual.data(), actual.size()) && actual == replacement,
            "cross-page forced bytes");
    require(memory.ptrace_write(base + 15, original.data(), original.size()) &&
                memory.ptrace_read(base + 15, actual.data(), actual.size()) && actual == original,
            "forced transfer across unaligned region boundaries");
    same_mappings(mappings, memory);
    protection_fault([&] { memory.store<1>(base, 1); }, "forced write leaked into normal permissions");
    protection_fault([&] { memory.load<1>(base + page_size); }, "forced read leaked into normal permissions");

    constexpr uint64_t end = base + 2 * page_size;
    const uint32_t sentinel = 0xaabbccdd;
    require(memory.ptrace_write(end - 4, &sentinel, sizeof(sentinel)), "prepare end sentinel");
    actual.fill(0x55);
    require(!memory.ptrace_read(end - 4, actual.data(), actual.size()) &&
                actual == std::array<uint8_t, 8>{0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55},
            "unmapped read must not copy a prefix");
    require(!memory.ptrace_write(end - 4, replacement.data(), replacement.size()),
            "unmapped write accepted");
    uint32_t value = 0;
    require(memory.ptrace_read(end - 4, &value, sizeof(value)) && value == sentinel,
            "unmapped write changed prefix");
    require(!memory.ptrace_read(0, &value, 1) && !memory.ptrace_write(UINT64_MAX, &value, 1) &&
                !memory.ptrace_read(address_limit - 1, &value, 2) &&
                !memory.ptrace_read(base, nullptr, 1) && !memory.ptrace_write(base, nullptr, 1),
            "invalid address, overflow or null buffer accepted");
    require(memory.ptrace_read(UINT64_MAX, nullptr, 0) && memory.ptrace_write(0, nullptr, 0),
            "empty transfer should succeed");
    memory.map(address_limit - page_size, page_size, 0);
    require(memory.ptrace_write(address_limit - 1, &sentinel, 1) &&
                memory.ptrace_read(address_limit - 1, &value, 1),
            "last addressable byte rejected");

    Memory limited(page_size);
    limited.set_parallel_access(!serial);
    limited.map(base, 2 * page_size, read_perm | write_perm);
    limited.store<4>(base + page_size - 4, sentinel);
    require(!limited.ptrace_write(base + page_size - 4, replacement.data(), replacement.size()) &&
                limited.load<4>(base + page_size - 4) == sentinel,
            "resource failure wrote a prefix");
}

void tlb_collisions(bool serial) {
    constexpr uint64_t base = page_size;
    constexpr size_t length = 1024 * page_size;
    Memory memory(length);
    memory.set_parallel_access(!serial);
    memory.map(base, length, 0);
    std::vector<uint8_t> expected(length), actual(length);
    for (size_t index = 0; index < length; ++index)
        expected[index] = uint8_t((index * 17 + index / page_size) & 0xff);
    require(memory.ptrace_write(base, expected.data(), expected.size()) &&
                memory.ptrace_read(base, actual.data(), actual.size()) && actual == expected,
            "forced multi-page transfer lost a TLB-collision page");
}

void fork_cow_and_reservations() {
    Memory parent(8 * page_size);
    constexpr uint64_t base = 0x30000;
    parent.map(base, 2 * page_size, read_perm | write_perm);
    parent.store<8>(base, 0x1122334455667788ull);
    auto child = parent.fork_clone();
    child->protect(base, 2 * page_size, read_perm);
    const auto mappings = child->mappings();
    const auto child_token = child->exclusive_epoch(base);
    const auto parent_token = parent.exclusive_epoch(base);
    const uint64_t changed = 0x8877665544332211ull;
    require(child->ptrace_write(base, &changed, sizeof(changed)), "force write fork COW page");
    require(child->load<8>(base) == changed && parent.load<8>(base) == 0x1122334455667788ull,
            "force write did not isolate fork COW");
    require(child->exclusive_epoch(base) != child_token && parent.exclusive_epoch(base) == parent_token,
            "fork COW reservation invalidation");
    require(child->ptrace_write(base + page_size, &changed, sizeof(changed)) &&
                parent.load<8>(base + page_size) == 0,
            "force write of cold fork page leaked into parent");
    parent.store<8>(base, 0x99);
    require(child->load<8>(base) == changed, "parent write leaked into child COW");
    same_mappings(mappings, *child);
    protection_fault([&] { child->store<8>(base, 0); }, "child became writable after forced COW");

    const auto token = child->exclusive_epoch(base);
    require(child->ptrace_write(base, &changed, sizeof(changed)) && child->exclusive_epoch(base) != token,
            "in-place forced write did not invalidate reservation");
}

void exclusive_instructions() {
    constexpr uint64_t address = 0x48000;
    constexpr uint32_t ldxr_x0_x1 = 0xc85f7c20, stxr_w2_x3_x1 = 0xc8027c23;
    const uint64_t initial = 11, changed = 37;
    Memory parent(2 * page_size);
    parent.map(address, page_size, read_perm | write_perm);
    parent.store<8>(address, initial);
    auto child = parent.fork_clone();
    CpuState cpu;
    cpu.x[1] = address;
    cpu.x[3] = 42;
    require(execute_exclusive(cpu, *child, ldxr_x0_x1) && cpu.x[0] == initial && cpu.exclusive_valid,
            "LDXR did not establish a reservation");
    require(child->ptrace_write(address, &changed, sizeof(changed)), "force write between LDXR and STXR");
    require(execute_exclusive(cpu, *child, stxr_w2_x3_x1) && cpu.x[2] == 1 &&
                child->load<8>(address) == changed && parent.load<8>(address) == initial,
            "STXR succeeded after forced COW write");
    require(execute_exclusive(cpu, *child, ldxr_x0_x1) &&
                child->ptrace_write(address, &changed, sizeof(changed)) &&
                execute_exclusive(cpu, *child, stxr_w2_x3_x1) && cpu.x[2] == 1,
            "STXR succeeded after forced in-place write");
}

void private_files_and_bus_faults() {
    TempFile file;
    file.resize(2 * page_size);
    const uint64_t original = 0x0102030405060708ull;
    const uint64_t changed = 0xf0e0d0c0b0a09080ull;
    file.put(page_size - 4, &original, sizeof(original));
    Memory first(8 * page_size), second(8 * page_size);
    constexpr uint64_t base = 0x50000;
    first.map_file(base, 2 * page_size, read_perm | exec_perm, file.ro, backing());
    second.map_file(base, 2 * page_size, read_perm, file.ro, backing());
    require(first.load<4>(base + page_size - 4) == uint32_t(original), "prime private file page");
    const auto mappings = first.mappings();
    require(first.ptrace_write(base + page_size - 4, &changed, sizeof(changed)),
            "force private write from read-only file");
    uint64_t actual = 0;
    require(first.ptrace_read(base + page_size - 4, &actual, sizeof(actual)) && actual == changed,
            "private file write bytes");
    second.copy_out(base + page_size - 4, &actual, sizeof(actual));
    require(actual == original && file.get<uint64_t>(page_size - 4) == original,
            "private file COW modified backing or another mapping");
    same_mappings(mappings, first);
    protection_fault([&] { first.store<1>(base, 1); }, "private file became guest-writable");
    auto child = first.fork_clone();
    require(child->ptrace_write(base + page_size - 4, &original, sizeof(original)),
            "force write forked private file page");
    require(first.ptrace_read(base + page_size - 4, &actual, sizeof(actual)) && actual == changed,
            "forked private file write changed parent");

    Memory bus(4 * page_size);
    bus.map_file(base, 2 * page_size, read_perm, file.ro, backing());
    {
        Memory::ExecutionScope scope(bus);
        require(bus.load<1>(base + page_size) == uint8_t(original >> 32), "prime file metadata");
        file.resize(page_size);
        std::array<uint8_t, 8> unchanged{};
        unchanged.fill(0x55);
        require(!bus.ptrace_read(base + page_size - 4, unchanged.data(), unchanged.size()) &&
                    unchanged == std::array<uint8_t, 8>{0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55},
                "truncated file read used stale metadata or copied a prefix");
        require(!bus.ptrace_write(base + page_size - 4, &changed, sizeof(changed)) &&
                    file.get<uint32_t>(page_size - 4) == uint32_t(original),
                "truncated file write changed a prefix");
    }
}

void shared_mappings() {
    TempFile file;
    file.resize(page_size);
    const uint64_t original = 0x123456789abcdef0ull;
    const uint64_t changed = 0xfedcba9876543210ull;
    file.put(0, &original, sizeof(original));
    constexpr uint64_t base = 0x70000;
    Memory memory(8 * page_size);
    memory.map_file(base, page_size, read_perm, file.ro, backing(true));
    memory.map_file(base + page_size, page_size, read_perm, file.rw, backing(true));
    memory.map_file(base + 2 * page_size, page_size, read_perm, file.rw, backing(true), false, false);
    memory.map_file(base + 3 * page_size, page_size, read_perm | write_perm, file.rw, backing(true));
    const auto mappings = memory.mappings();
    for (uint64_t page = 0; page < 3; ++page) {
        uint64_t value = 0;
        require(memory.ptrace_read(base + page * page_size, &value, sizeof(value)) && value == original,
                "read shared file mapping");
        require(!memory.ptrace_write(base + page * page_size, &changed, sizeof(changed)),
                "forced write accepted read-only shared mapping");
    }
    require(file.get<uint64_t>(0) == original, "read-only shared backend was modified");
    require(memory.ptrace_write(base + 3 * page_size, &changed, sizeof(changed)) &&
                file.get<uint64_t>(0) == changed,
            "writable shared file mapping did not propagate");
    same_mappings(mappings, memory);

    Memory mixed(4 * page_size);
    mixed.map(base, page_size, read_perm | write_perm);
    mixed.map_file(base + page_size, page_size, read_perm, file.ro, backing(true));
    const uint32_t prefix = 0xabcdef01;
    mixed.store<4>(base + page_size - 4, prefix);
    require(!mixed.ptrace_write(base + page_size - 4, &original, sizeof(original)) &&
                mixed.load<4>(base + page_size - 4) == prefix && file.get<uint64_t>(0) == changed,
            "shared protection failure changed a preceding private page");

    Memory anonymous(4 * page_size);
    anonymous.map(base, page_size, read_perm | write_perm, false, true);
    anonymous.store<8>(base, original);
    auto child = anonymous.fork_clone();
    child->protect(base, page_size, read_perm);
    auto token = child->exclusive_epoch(base);
    require(!child->ptrace_write(base, &changed, sizeof(changed)), "read-only shared anonymous write");
    const auto epoch = child->code_epoch.load();
    require(anonymous.ptrace_write(base, &changed, sizeof(changed)), "writable shared anonymous write");
    require(child->load<8>(base) == changed && child->exclusive_epoch(base) != token,
            "shared anonymous alias or reservation not updated");
    child->synchronize_shared();
    require(child->code_epoch.load() > epoch, "fork shared revision did not invalidate code cache");
}

void memory_file_seals() {
    constexpr uint64_t name = 0x90000, before = 0x91000, after = 0x92000, private_map = 0x93000;
    Memory memory(8 * page_size);
    DescriptorTable descriptors;
    memory.map(name, page_size, read_perm | write_perm);
    constexpr char label[] = "ptrace-seals";
    memory.copy_in(name, label, sizeof(label));
    const int fd = files::memory_file_create(memory, descriptors, name, files::mfd_allow_sealing);
    auto &entry = descriptors.get(fd);
    require(::ftruncate(entry.host.get(), page_size) == 0, "size sealed memory file");
    const uint64_t initial = 0x11223344, changed = 0x55667788, private_value = 0x99aabbcc;
    require(::pwrite(entry.host.get(), &initial, sizeof(initial), 0) == ssize_t(sizeof(initial)),
            "initialize sealed memory file");
    memory.map_file(before, page_size, read_perm | write_perm, entry.host.get(), backing(true));
    files::memory_file_add_seals(entry, files::memory_file_seal_future_write);
    memory.map_file(after, page_size, read_perm, entry.host.get(), backing(true));
    memory.map_file(private_map, page_size, read_perm, entry.host.get(), backing());
    require(memory.ptrace_write(before, &changed, sizeof(changed)), "pre-seal writable mapping lost access");
    require(!memory.ptrace_write(after, &private_value, sizeof(private_value)),
            "future-write seal bypassed on shared mapping");
    require(memory.ptrace_write(private_map, &private_value, sizeof(private_value)),
            "seal blocked private COW write");
    uint64_t disk = 0;
    require(::pread(entry.host.get(), &disk, sizeof(disk), 0) == ssize_t(sizeof(disk)) && disk == changed,
            "sealed shared backend changed via private mapping");
}

void interpreter_invalidation() {
    constexpr uint64_t address = 0xb0000;
    constexpr uint32_t mov7 = 0xd28000e0, mov11 = 0xd2800160, svc = 0xd4000001;
    const std::array<uint32_t, 2> program{mov7, svc};
    Memory memory(4 * page_size);
    memory.map(address, page_size, read_perm | write_perm | exec_perm);
    memory.copy_in(address, program.data(), sizeof(program));
    memory.protect(address, page_size, exec_perm);
    const auto mappings = memory.mappings();
    Interpreter interpreter;
    CpuState cpu;
    cpu.pc = address;
    interpreter.run(cpu, memory, 1);
    require(cpu.x[0] == 7, "initial decoded instruction");
    const auto epoch = memory.code_epoch.load();
    require(memory.ptrace_write(address, &mov11, sizeof(mov11)) && memory.code_epoch.load() > epoch,
            "forced executable write did not bump code epoch");
    cpu.pc = address;
    interpreter.run(cpu, memory, 1);
    require(cpu.x[0] == 11, "interpreter reused stale decoded instruction");
    same_mappings(mappings, memory);

    TempFile file;
    file.resize(page_size);
    file.put(0, program.data(), sizeof(program));
    Memory writer(2 * page_size), reader(2 * page_size);
    writer.map_file(address, page_size, read_perm | write_perm, file.rw, backing(true));
    reader.map_file(address, page_size, read_perm | exec_perm, file.ro, backing(true));
    Interpreter shared_interpreter;
    CpuState shared_cpu;
    shared_cpu.pc = address;
    shared_interpreter.run(shared_cpu, reader, 1);
    require(shared_cpu.x[0] == 7, "initial shared decoded instruction");
    const auto token = reader.exclusive_epoch(address);
    require(writer.ptrace_write(address, &mov11, sizeof(mov11)) && reader.exclusive_epoch(address) != token,
            "separate file alias reservation not invalidated");
    reader.synchronize_shared();
    shared_cpu.pc = address;
    shared_interpreter.run(shared_cpu, reader, 1);
    require(shared_cpu.x[0] == 11, "shared file alias retained stale decoded instruction");
}
} // namespace

int main() {
    try {
        permissions_and_boundaries(false);
        permissions_and_boundaries(true);
        tlb_collisions(false);
        tlb_collisions(true);
        fork_cow_and_reservations();
        exclusive_instructions();
        private_files_and_bus_faults();
        shared_mappings();
        memory_file_seals();
        interpreter_invalidation();
        std::puts("ptrace memory permissions, boundaries, COW, shared mappings and cache passed");
        return 0;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "unexpected guest error: %d\n", error.number);
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    }
    return 1;
}
