#include "linux/process/control.h"
#include "linux/process/pidfd.h"
#include "linux/syscalls.h"
#include <algorithm>

namespace operit::arm64 {
namespace {
void validate_bounds(const process::MemoryBounds &bounds) {
    const uint64_t addresses[] = {bounds.start_code, bounds.end_code, bounds.start_data,
                                  bounds.end_data, bounds.start_brk, bounds.brk,
                                  bounds.start_stack, bounds.arg_start, bounds.arg_end,
                                  bounds.env_start, bounds.env_end};
    for (auto address : addresses)
        if (address < page_size || address >= address_limit)
            guest_error(22);
    if (bounds.start_code >= bounds.end_code || bounds.start_data > bounds.end_data ||
        bounds.start_brk > bounds.brk || bounds.arg_start > bounds.arg_end ||
        bounds.env_start > bounds.env_end)
        guest_error(22);
    // Guest RLIMIT_DATA is unbounded. The memory residency limit still applies
    // when brk faults in pages; changing these bounds does not allocate a VMA.
}
std::string read_process_bytes(Memory &memory, uint64_t address, uint64_t length) {
    std::string result;
    std::array<char, page_size> bytes;
    for (const auto &mapping : memory.mappings()) {
        if (!length || mapping.start > address)
            break;
        if (mapping.end <= address)
            continue;
        // Linux uses access_remote_vm(FOLL_ANON) for cmdline and environ.
        if (!(mapping.permissions & read_perm) || mapping.backing.inode)
            break;
        while (length && address < mapping.end) {
            const auto count = std::min<uint64_t>(
                {length, mapping.end - address, page_size - address % page_size});
            try {
                memory.copy_out(address, bytes.data(), count);
            } catch (const MemoryFault &) {
                return result;
            } catch (const GuestError &error) {
                if (error.number != 12)
                    throw;
                return result;
            }
            result.append(bytes.data(), count);
            address += count;
            length -= count;
        }
    }
    return result;
}
std::string read_process_ranges(Memory &memory, const std::vector<process::StringRange> &ranges,
                                uint64_t offset, uint64_t count) {
    std::string result;
    for (const auto &range : ranges) {
        if (offset >= range.size) {
            offset -= range.size;
            continue;
        }
        const auto length = std::min(count, range.size - offset);
        auto bytes = read_process_bytes(memory, range.address + offset, length);
        result += bytes;
        count -= bytes.size();
        if (bytes.size() != length || !count)
            break;
        offset = 0;
    }
    return result;
}
uint64_t page_end(uint64_t address) {
    return (address + page_size - 1) & ~(page_size - 1);
}
} // namespace

void Syscalls::initialize_process_memory(uint64_t stack) {
    auto image = std::make_shared<process::MemoryState>();
    auto &bounds = image->bounds;
    bounds.start_brk = process_memory_->bounds.start_brk;
    bounds.brk = process_memory_->bounds.brk;
    bounds.start_stack = stack;
    uint64_t cursor = stack + 8;
    std::vector<process::StringRange> arguments, environment;
    auto strings = [&](uint64_t &begin, uint64_t &end, std::vector<process::StringRange> &ranges) {
        bool first = true, contiguous = true;
        for (;;) {
            const uint64_t address = memory_.load(cursor, 8);
            cursor += 8;
            if (!address)
                break;
            if (first)
                begin = address;
            else if (end != address)
                contiguous = false;
            first = false;
            end = address;
            while (memory_.load(end++, 1)) {
            }
            ranges.push_back({address, end - address});
        }
        if (first)
            end = begin;
        if (contiguous)
            ranges.clear();
    };
    strings(bounds.arg_start, bounds.arg_end, arguments);
    bounds.env_start = bounds.arg_end;
    strings(bounds.env_start, bounds.env_end, environment);
    uint64_t phdr = 0, phent = 0, phnum = 0;
    for (size_t index = 0; index < image->saved_auxv.size(); index += 2) {
        const auto type = memory_.load(cursor, 8), value = memory_.load(cursor + 8, 8);
        cursor += 16;
        image->saved_auxv[index] = type;
        image->saved_auxv[index + 1] = value;
        if (!type)
            break;
        if (type == 3)
            phdr = value;
        else if (type == 4)
            phent = value;
        else if (type == 5)
            phnum = value;
        if (index + 2 == image->saved_auxv.size())
            throw Failure("initial auxiliary vector has no AT_NULL");
    }
    if (!phdr) {
        image->argument_ranges = std::move(arguments);
        image->environment_ranges = std::move(environment);
    }
    // Recover byte-accurate PT_LOAD bounds from the main executable's mapped
    // program headers. Interpreter mappings must not contribute to these fields.
    if (phdr && phent == 56 && phnum && phnum <= 1024) {
        // These are loader metadata, not guest reads. PT_LOAD may be execute-only.
        auto load_header = [this](uint64_t address, unsigned size) {
            return memory_.load(address, size, 0);
        };
        uint64_t phdr_offset = 0;
        bool have_offset = false;
        for (const auto &mapping : memory_.mappings()) {
            if (mapping.backing.path == executable_ && mapping.start <= phdr && phdr < mapping.end) {
                phdr_offset = mapping.backing.offset + phdr - mapping.start;
                have_offset = true;
                break;
            }
        }
        uint64_t bias = 0;
        bool have_bias = false;
        for (uint64_t index = 0; have_offset && index < phnum; ++index) {
            const auto header = phdr + index * phent;
            if (load_header(header, 4) != 1)
                continue;
            const auto offset = load_header(header + 8, 8);
            const auto address = load_header(header + 16, 8);
            const auto file_size = load_header(header + 32, 8);
            if (offset <= phdr_offset && phdr_offset - offset < file_size) {
                bias = phdr - address - (phdr_offset - offset);
                have_bias = true;
                break;
            }
        }
        for (uint64_t index = 0; have_bias && index < phnum; ++index) {
            const auto header = phdr + index * phent;
            if (load_header(header, 4) != 1)
                continue;
            const auto flags = load_header(header + 4, 4);
            const auto begin = bias + load_header(header + 16, 8);
            const auto end = begin + load_header(header + 32, 8);
            if (flags & 1) {
                if (!bounds.start_code || begin < bounds.start_code)
                    bounds.start_code = begin;
                bounds.end_code = std::max(bounds.end_code, end);
            }
            bounds.start_data = std::max(bounds.start_data, begin);
            bounds.end_data = std::max(bounds.end_data, end);
        }
    }
    process_memory_ = std::move(image);
}

std::string Syscalls::process_memory_contents(const std::string &name, uint64_t offset, uint64_t count) {
    if (!count)
        return {};
    const auto &state = *process_memory_;
    const auto &bounds = state.bounds;
    if (name == "auxv") {
        size_t words = 0;
        do {
            words += 2;
        } while (words < state.saved_auxv.size() && state.saved_auxv[words - 2]);
        const uint64_t length = words * sizeof(uint64_t);
        if (offset >= length)
            return {};
        return {reinterpret_cast<const char *>(state.saved_auxv.data()) + offset,
                size_t(std::min(count, length - offset))};
    }
    if (name == "cmdline" && !state.argument_ranges.empty())
        return read_process_ranges(memory_, state.argument_ranges, offset, count);
    if (name == "environ" && !state.environment_ranges.empty())
        return read_process_ranges(memory_, state.environment_ranges, offset, count);
    if (!bounds.env_end)
        return {};
    if (name == "environ") {
        const auto length = bounds.env_end - bounds.env_start;
        if (offset >= length)
            return {};
        return read_process_bytes(memory_, bounds.env_start + offset, std::min(count, length - offset));
    }
    if (bounds.arg_start >= bounds.arg_end)
        return {};
    const auto limit = bounds.env_start == bounds.arg_end && bounds.env_end >= bounds.env_start
                           ? bounds.env_end : bounds.arg_end;
    if (offset >= limit - bounds.arg_start)
        return {};
    count = std::min(count, limit - bounds.arg_start - offset);
    // setproctitle can replace the argv terminator and continue into an adjacent
    // environment area. Linux exposes one NUL-terminated title, at most one page.
    const auto last = read_process_bytes(memory_, bounds.arg_end - 1, 1);
    if (!last.empty() && last[0]) {
        if (offset >= page_size)
            return {};
        auto title = read_process_bytes(memory_, bounds.arg_start, page_size);
        const auto end = title.find('\0');
        if (end != std::string::npos)
            title.resize(end + 1);
        return offset >= title.size() ? std::string{} : title.substr(offset, count);
    }
    const auto length = bounds.arg_end - bounds.arg_start;
    if (offset >= length)
        return {};
    return read_process_bytes(memory_, bounds.arg_start + offset, std::min(count, length - offset));
}

int64_t Syscalls::process_brk(uint64_t requested) {
    const Memory::AccessGuard memory_access(memory_);
    auto &bounds = process_memory_->bounds;
    if (!requested || requested < bounds.start_brk || requested >= address_limit - page_size)
        return bounds.brk;
    const uint64_t old_end = page_end(bounds.brk), new_end = page_end(requested);
    try {
        resource_limits()->check_brk(memory_, bounds.start_data, bounds.end_data,
                                     bounds.start_brk, bounds.brk, requested);
        if (new_end > old_end) {
            memory_.map(old_end, new_end - old_end, read_perm | write_perm);
            memory_.describe(old_end, new_end - old_end, {"[heap]", 0, 0, 0, false});
        } else if (new_end < old_end) {
            memory_.unmap(new_end, old_end - new_end);
        }
    } catch (const GuestError &) {
        return bounds.brk;
    }
    bounds.brk = requested;
    return bounds.brk;
}

int64_t Syscalls::dispatch_control(CpuState &cpu) {
    const auto &a = cpu.x;
    switch (uint32_t(a[0])) {
    case 21: // PR_GET_SECCOMP
    case 22: // PR_SET_SECCOMP
    case 38: // PR_SET_NO_NEW_PRIVS
    case 39: // PR_GET_NO_NEW_PRIVS
        return security::seccomp_prctl(scheduler_.current().seccomp, memory_, a[0], a[1], a[2], a[3], a[4]);
    case 3: { // PR_GET_DUMPABLE: process-wide guest ptrace access state.
        auto *controller = scheduler_.processes();
        if (!controller)
            guest_error(38);
        return process::pidfd_credentials(*controller, scheduler_.current().pid).dumpable ? 1 : 0;
    }
    case 4: { // PR_SET_DUMPABLE permits only SUID_DUMP_DISABLE/USER (0/1).
        if (a[1] > 1)
            guest_error(22);
        auto *controller = scheduler_.processes();
        if (!controller)
            guest_error(38);
        const auto pid = scheduler_.current().pid;
        auto credentials = process::pidfd_credentials(*controller, pid);
        credentials.dumpable = bool(a[1]);
        process::pidfd_update_credentials(*controller, pid, credentials);
        return 0;
    }
    case 15: { // PR_SET_NAME
        char name[16]{};
        for (unsigned index = 0; index < 15; ++index) {
            check_buffer(a[1] + index, 1, read_perm);
            name[index] = char(memory_.load(a[1] + index, 1));
            if (!name[index])
                break;
        }
        scheduler_.current().name = name;
        return 0;
    }
    case 16: { // PR_GET_NAME
        check_buffer(a[1], 16, write_perm);
        char name[16]{};
        scheduler_.current().name.copy(name, 15);
        memory_.copy_in(a[1], name, sizeof(name));
        return 0;
    }
    case 35: { // PR_SET_MM
        const auto option = process::MmOption(uint32_t(a[1]));
        if (a[4] || (a[3] && option != process::MmOption::Auxv && option != process::MmOption::Map &&
                     option != process::MmOption::MapSize))
            guest_error(22);
        if (option == process::MmOption::MapSize) {
            check_buffer(a[2], 4, write_perm);
            memory_.store(a[2], sizeof(process::MmMap), 4);
            return 0;
        }
        // capget/capset exposes an empty guest capability set. All individual
        // setters, including AUXV and EXE_FILE, require CAP_SYS_RESOURCE, even
        // when the embedding host happens to have uid zero. Linux checks this
        // before validating their addresses or rejecting an unknown MM option.
        if (option != process::MmOption::Map)
            guest_error(1);
        if (a[3] != sizeof(process::MmMap))
            guest_error(22);
        check_buffer(a[2], sizeof(process::MmMap), read_perm);
        process::MmMap map;
        memory_.copy_out(a[2], &map, sizeof(map));
        validate_bounds(map.bounds);
        std::array<uint64_t, process::auxv_words> auxiliary{};
        if (map.auxv_size) {
            if (!map.auxv || map.auxv_size > sizeof(auxiliary))
                guest_error(22);
            check_buffer(map.auxv, map.auxv_size, read_perm);
            memory_.copy_out(map.auxv, auxiliary.data(), map.auxv_size);
            auxiliary[auxiliary.size() - 2] = auxiliary[auxiliary.size() - 1] = 0;
        }
        // MAP itself is unprivileged. Replacing exe requires guest CAP_SYS_ADMIN
        // or CAP_CHECKPOINT_RESTORE; neither can be acquired by this guest.
        if (map.exe_fd != UINT32_MAX)
            guest_error(1);
        // MAP deliberately accepts unmapped addresses, and does not mmap or
        // move strings. proc readers and subsequent brk calls use these bounds.
        process_memory_->bounds = map.bounds;
        process_memory_->argument_ranges.clear();
        process_memory_->environment_ranges.clear();
        if (map.auxv_size)
            process_memory_->saved_auxv = auxiliary;
        return 0;
    }
    default:
        return unsupported(cpu);
    }
}
} // namespace operit::arm64
