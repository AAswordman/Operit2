#include "linux/threads/rseq.h"
#include "cpu/aarch64/state.h"
#include "linux/signals/signals.h"
#include "memory/address_space.h"
#include <atomic>
#include <cstdio>
#include <stdexcept>
#include <thread>

using namespace operit::arm64;
using namespace operit::arm64::threads;
namespace {
constexpr uint64_t abi = 0x10000, descriptor = 0x12000;
constexpr uint64_t start = 0x14000, end = start + 16, abort_ip = 0x15000;
constexpr uint32_t signature = 0xd428bc00;

void require(bool value, const char *message) {
    if (!value)
        throw std::runtime_error(message);
}

struct Fixture {
    Memory memory{1 << 20};
    CpuState cpu;
    RseqState state;
    Fixture() {
        memory.map(abi, 0x8000, read_perm | write_perm);
        memory.store(abort_ip - 4, signature, 4);
        write_cs();
    }
    void write_cs(uint64_t at = descriptor, uint32_t version = 0, uint32_t flags = 0,
                  uint64_t first = start, uint64_t length = end - start, uint64_t abort = abort_ip) {
        memory.store(at, version, 4);
        memory.store(at + 4, flags, 4);
        memory.store(at + 8, first, 8);
        memory.store(at + 16, length, 8);
        memory.store(at + 24, abort, 8);
    }
    int64_t call(uint64_t address = abi, uint32_t size = 32, uint32_t flags = 0,
                 uint32_t sig = signature) {
        return rseq_syscall(memory, state, address, size, flags, sig);
    }
    void register_and_resume() {
        require(call() == 0 && state.notify, "registration");
        require(rseq_resume(memory, cpu, state, 2, 2), "initial notification");
    }
    void enter(uint64_t pc = start) {
        memory.store(abi + 8, descriptor, 8);
        cpu.pc = pc;
    }
};

void syscall_validation() {
    Fixture f;
    require(f.call(abi, 32, 1) == -22, "unregister absent");
    require(f.call(abi, 31) == -22 && f.call(abi + 4) == -22, "size and area alignment");
    require(f.call(abi, 32, 2) == -22 && f.call(abi, 32, 3) == -22, "unknown syscall flags");
    require(f.call(address_limit - 32, 33) == -14, "address range overflow");
    require(f.call(abi, 33) == 0, "extended size");
    require(f.call(abi, 33) == -16, "duplicate registration");
    require(f.call(abi, 33, 0, 123) == -1, "duplicate signature");
    require(f.call(abi + 32, 33) == -22 && f.call(abi, 32) == -22, "duplicate address or size");
    require(f.call(abi, 32, 1) == -22 && f.call(abi + 32, 33, 1) == -22,
            "unregister address or size");
    require(f.call(abi, 33, 1, 123) == -1, "unregister signature");
    require(f.call(abi, 33, 3) == -22, "unregister flags");
    require(rseq_resume(f.memory, f.cpu, f.state, 7, 11), "extended registration resume");
    require(f.memory.load(abi, 4) == 7 && f.memory.load(abi + 4, 4) == 7 &&
                f.memory.load(abi + 20, 4) == 0 && f.memory.load(abi + 24, 4) == 11,
            "CPU/node/mm_cid update");
    f.memory.protect(abi, page_size, read_perm);
    require(f.call(abi, 33, 1) == -14 && f.state.address == abi, "unregister fault retained registration");
    f.memory.protect(abi, page_size, read_perm | write_perm);
    f.memory.store(abi + 8, descriptor, 8);
    require(f.call(abi, 33, 1) == 0 && f.state.address == 0 && f.memory.load(abi, 4) == 0 &&
                f.memory.load(abi + 4, 4) == UINT32_MAX && f.memory.load(abi + 8, 8) == descriptor &&
                f.memory.load(abi + 20, 4) == 0 && f.memory.load(abi + 24, 4) == 0,
            "unregister reset fields");
    require(f.call() == 0, "register after unregister");
}

void deferred_faults() {
    Fixture f;
    require(f.call(0x80000) == 0, "registration must not require mapped pages");
    require(!rseq_resume(f.memory, f.cpu, f.state, 0, 0), "unmapped registration must fault on resume");
    rseq_reset(f.state);
    f.memory.protect(abi, page_size, read_perm);
    require(f.call() == 0 && !rseq_resume(f.memory, f.cpu, f.state, 0, 0),
            "read-only registration must fault on resume");

    Fixture extended;
    extended.memory.unmap(abi + page_size, page_size);
    require(extended.call(abi, page_size + 32) == 0 &&
                rseq_resume(extended.memory, extended.cpu, extended.state, 0, 0),
            "access_ok must not require the whole extended area to be mapped");
    Fixture null_pointer;
    require(null_pointer.call(0) == 0 && !null_pointer.state.address &&
                null_pointer.call(0, 32, 1) == -22,
            "Linux treats a null registration as the unregistered sentinel");
}

void events_and_boundaries() {
    for (const auto event : {RseqEvent::Preempt, RseqEvent::Signal, RseqEvent::Migrate}) {
        for (const uint64_t pc : {start, start + 4, end - 1}) {
            Fixture f;
            f.register_and_resume();
            f.enter(pc);
            rseq_event(f.state, event);
            require(rseq_resume(f.memory, f.cpu, f.state, 5, 5), "event fixup");
            require(f.cpu.pc == abort_ip && f.memory.load(abi + 8, 8) == 0 && f.state.events == 0 &&
                        f.memory.load(abi, 4) == 5 && f.memory.load(abi + 4, 4) == 5 &&
                        f.memory.load(abi + 24, 4) == 5,
                    "inside critical section must abort and update CPU");
        }
        for (const uint64_t pc : {start - 4, end, end + 4}) {
            Fixture f;
            f.register_and_resume();
            f.enter(pc);
            rseq_event(f.state, event);
            require(rseq_resume(f.memory, f.cpu, f.state, 3, 3), "outside fixup");
            require(f.cpu.pc == pc && f.memory.load(abi + 8, 8) == 0,
                    "outside critical section should only clear descriptor");
        }
    }
    Fixture f;
    f.enter();
    require(f.call() == 0 && rseq_resume(f.memory, f.cpu, f.state, 1, 1), "resume without event");
    require(f.cpu.pc == start && f.memory.load(abi + 8, 8) == descriptor,
            "registration notification without event must not abort");
    rseq_event(f.state, RseqEvent::Preempt);
    require(rseq_resume(f.memory, f.cpu, f.state, 1, 1, true) && f.state.notify && f.cpu.pc == start,
            "kernel-only resume must defer event fixup");
    require(rseq_resume(f.memory, f.cpu, f.state, 1, 1) && f.cpu.pc == abort_ip,
            "deferred event must be fixed up before user return");
}

void invalid_descriptors() {
    for (int scenario = 0; scenario != 12; ++scenario) {
        Fixture f;
        f.register_and_resume();
        f.enter();
        switch (scenario) {
        case 0: f.memory.store(abi + 8, address_limit, 8); break;
        case 1: f.memory.store(abi + 8, 0x80000, 8); break;
        case 2: f.write_cs(descriptor, 1); break;
        case 3: f.write_cs(descriptor, 0, 0, address_limit); break;
        case 4: f.write_cs(descriptor, 0, 0, start, address_limit); break;
        case 5: f.write_cs(descriptor, 0, 0, start, end - start, address_limit); break;
        case 6: f.write_cs(descriptor, 0, 0, start, end - start, start + 4); break;
        case 7: f.memory.store(abort_ip - 4, signature + 1, 4); break;
        case 8: f.write_cs(descriptor, 0, 1); break;
        case 9: f.memory.store(abi + 16, 1, 4); break;
        case 10: f.write_cs(descriptor, 0, 0, start, end - start, 0); break;
        case 11: f.memory.protect(abi, page_size, read_perm); break;
        }
        rseq_event(f.state, RseqEvent::Signal);
        require(!rseq_resume(f.memory, f.cpu, f.state, 1, 1), "invalid descriptor did not fault");
        require(f.cpu.pc == start, "invalid descriptor changed PC");
    }
    Fixture f;
    f.register_and_resume();
    // The UAPI specifies 32-byte descriptor alignment, but Linux v6.12 does
    // not reject unaligned descriptors in rseq_get_rseq_cs().
    f.write_cs(descriptor + 1);
    f.memory.store(abi + 8, descriptor + 1, 8);
    f.cpu.pc = start;
    rseq_event(f.state, RseqEvent::Preempt);
    require(rseq_resume(f.memory, f.cpu, f.state, 0, 0) && f.cpu.pc == abort_ip,
            "unaligned descriptor differs from v6.12");
    Fixture outside;
    outside.register_and_resume();
    outside.write_cs(descriptor, 0, 1);
    outside.enter(end);
    rseq_event(outside.state, RseqEvent::Preempt);
    require(rseq_resume(outside.memory, outside.cpu, outside.state, 0, 0),
            "v6.12 checks CS flags only inside a critical section");
    Fixture bad_signature_outside;
    bad_signature_outside.register_and_resume();
    bad_signature_outside.enter(end);
    bad_signature_outside.memory.store(abort_ip - 4, 0, 4);
    rseq_event(bad_signature_outside.state, RseqEvent::Preempt);
    require(!rseq_resume(bad_signature_outside.memory, bad_signature_outside.cpu,
                         bad_signature_outside.state, 0, 0),
            "signature must be checked even outside the critical section");
}

void lifecycle_and_cpu_reuse() {
    Fixture f;
    f.register_and_resume();
    f.enter();
    const auto shared = rseq_fork(f.state, true);
    require(!shared.address && !shared.notify, "CLONE_VM must start unregistered");
    auto child_memory = f.memory.fork_clone();
    auto child = rseq_fork(f.state, false);
    require(child.address == abi && child.signature == signature && child.notify, "fork inheritance");
    require(rseq_syscall(*child_memory, child, abi, 32, 0, signature) == -16, "child registration copied");
    CpuState child_cpu;
    child_cpu.pc = end;
    require(rseq_resume(*child_memory, child_cpu, child, 1, 1), "child resume");
    require(child_memory->load(abi + 4, 4) == 1 && f.memory.load(abi + 4, 4) == 2,
            "fork uses its own address space");
    rseq_reset(child);
    require(!child.address && !child.events && !child.notify && f.state.address == abi,
            "exec/exit state reset");

    // Task A read a per-CPU counter before being switched out. Task B on the
    // same virtual CPU commits an increment. A must abort before its stale
    // store, even though it resumes on the same CPU.
    f.memory.store(0x16000, 0, 4);
    const auto stale = f.memory.load(0x16000, 4);
    rseq_event(f.state, RseqEvent::Preempt);
    f.memory.store(0x16000, 1, 4);
    require(rseq_resume(f.memory, f.cpu, f.state, 2, 2), "CPU reuse resume");
    if (f.cpu.pc != abort_ip)
        f.memory.store(0x16000, stale + 1, 4);
    require(f.cpu.pc == abort_ip && f.memory.load(0x16000, 4) == 1, "stale per-CPU commit");
}

void signal_frame_after_abort() {
    Fixture f;
    f.register_and_resume();
    f.enter();
    f.memory.map(0x20000, 0x10000, read_perm | write_perm);
    f.cpu.sp = 0x30000;
    signals::ProcessState process;
    signals::ThreadState thread;
    uint64_t mask = 0;
    process.actions->entries[9].handler = 0x16000;
    require(process.pending.enqueue(signals::Info::user(10, 1, 0)), "signal enqueue");
    rseq_event(f.state, RseqEvent::Signal);
    require(rseq_resume(f.memory, f.cpu, f.state, 2, 2), "signal fixup");
    const auto delivery = signals::deliver(f.memory, f.cpu, process, thread, mask, 0x16004);
    require(delivery.effect == signals::Effect::Handler && f.cpu.pc == 0x16000, "signal delivery");
    require(signals::restore_frame(f.memory, f.cpu, thread, mask) && f.cpu.pc == abort_ip,
            "signal frame did not retain abort PC");
}

void parallel_distinct_cpu_slots() {
    Memory memory{1 << 20};
    memory.map(0x10000, 0x8000, read_perm | write_perm);
    memory.store(abort_ip - 4, signature, 4);
    memory.store(descriptor, 0, 4);
    memory.store(descriptor + 4, 0, 4);
    memory.store(descriptor + 8, start, 8);
    memory.store(descriptor + 16, end - start, 8);
    memory.store(descriptor + 24, abort_ip, 8);
    std::atomic<bool> failed{false};
    auto worker = [&](uint32_t id) {
        try {
            const auto address = abi + id * 32;
            RseqState state;
            CpuState cpu;
            if (rseq_syscall(memory, state, address, 32, 0, signature) != 0 ||
                !rseq_resume(memory, cpu, state, id, id)) {
                failed = true;
                return;
            }
            for (unsigned i = 0; i != 1000; ++i) {
                memory.store(address + 8, descriptor, 8);
                cpu.pc = start + 4;
                rseq_event(state, RseqEvent::Preempt);
                if (!rseq_resume(memory, cpu, state, id, id) || cpu.pc != abort_ip ||
                    memory.load(address + 4, 4) != id || memory.load(address + 24, 4) != id) {
                    failed = true;
                    return;
                }
            }
        } catch (...) {
            failed = true;
        }
    };
    std::thread first(worker, 0), second(worker, 1);
    first.join();
    second.join();
    require(!failed, "parallel CPU slot or registration state failure");
}
} // namespace

int main() {
    try {
        syscall_validation();
        deferred_faults();
        events_and_boundaries();
        invalid_descriptors();
        lifecycle_and_cpu_reuse();
        signal_frame_after_abort();
        parallel_distinct_cpu_slots();
        std::puts("rseq-host-tests-ok");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "rseq-host-tests: %s\n", error.what());
        return 1;
    } catch (const GuestError &error) {
        std::fprintf(stderr, "rseq-host-tests: unexpected guest errno %d\n", error.number);
        return 1;
    }
}
