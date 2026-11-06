#include "acceleration/fixed_components.h"
#include "cpu/aarch64/execution_engine.h"
#include "elf/loader.h"
#include "linux/threads/rseq.h"
#include <array>
#include <cstdio>
#include <stdexcept>
#include <string>

using namespace operit::arm64;
using namespace operit::arm64::threads;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}

int run(const char *root) {
    Memory memory{32 << 20};
    Filesystem filesystem(root);
    FixedComponents acceleration(false);
    ElfLoader loader(memory, filesystem, 32 << 20, acceleration);
    const auto image = loader.load("/rseq", 0);
    memory.map(0x70000000, 0x100000, read_perm | write_perm);
    CpuState cpu;
    cpu.pc = image.entry;
    cpu.sp = 0x70100000 - 16;
    ExecutionEngine interpreter;
    RseqState state;
    const std::array<RseqEvent, 3> events{RseqEvent::Preempt, RseqEvent::Signal, RseqEvent::Migrate};
    size_t injected = 0;
    uint32_t cpu_id = 0;
    std::string output;
    for (unsigned step = 0; step != 100000; ++step) {
        if (state.address && injected < events.size()) {
            const auto descriptor = memory.load(state.address + 8, 8);
            if (descriptor) {
                const auto first = memory.load(descriptor + 8, 8);
                if (cpu.pc == first + 4) {
                    const auto before = memory.load(cpu.x[1], 4);
                    const auto abort = memory.load(descriptor + 24, 8);
                    rseq_event(state, events[injected]);
                    if (events[injected] == RseqEvent::Migrate)
                        cpu_id = 1;
                    require(rseq_resume(memory, cpu, state, cpu_id, cpu_id), "injected resume fault");
                    require(cpu.pc == abort && memory.load(state.address + 8, 8) == 0 &&
                                memory.load(cpu.x[1], 4) == before,
                            "interrupted guest reached stale commit");
                    ++injected;
                }
            }
        }
        const auto result = interpreter.run(cpu, memory, 1);
        if (result.trap != Trap::Syscall)
            continue;
        const auto nr = cpu.x[8];
        if (nr == 293) {
            cpu.x[0] = uint64_t(rseq_syscall(memory, state, cpu.x[0], uint32_t(cpu.x[1]),
                                             uint32_t(cpu.x[2]), uint32_t(cpu.x[3])));
            require(rseq_resume(memory, cpu, state, cpu_id, cpu_id), "syscall return fault");
        } else if (nr == 64 && cpu.x[0] == 1) {
            const auto count = cpu.x[2];
            require(count <= 1024, "guest write too large");
            std::string bytes(count, '\0');
            memory.copy_out(cpu.x[1], bytes.data(), bytes.size());
            output += bytes;
            cpu.x[0] = count;
        } else if (nr == 93) {
            require(cpu.x[0] == 0, "guest reported failure");
            require(injected == events.size(), "not all events were injected");
            require(output == "rseq-guest-ok\n", "unexpected guest output");
            std::printf("rseq-aarch64-guest-ok: %zu asynchronous events\n", injected);
            return 0;
        } else {
            throw std::runtime_error("unexpected guest syscall " + std::to_string(nr));
        }
    }
    throw std::runtime_error("guest exceeded instruction budget");
}
} // namespace

int main(int argc, char **argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s guest-root\n", argv[0]);
        return 2;
    }
    try {
        return run(argv[1]);
    } catch (const std::exception &error) {
        std::fprintf(stderr, "rseq guest harness: %s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "rseq guest harness: guest errno %d\n", error.number);
    }
    return 1;
}
