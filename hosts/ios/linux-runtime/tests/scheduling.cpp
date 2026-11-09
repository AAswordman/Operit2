#include "linux/syscalls.h"
#include "linux/threads/scheduling.h"
#include "runtime/machine.h"
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <stdexcept>
#include <string>

using namespace operit::arm64;
namespace {
using Attributes = threads::SchedulingAttributes;
using Resource = process::Resource;
constexpr uint64_t attributes_word = 0x11000, limit_word = 0x13000, previous_word = 0x13020;
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
void require_rr_budget(const Task &task, uint64_t expected, const char *message) {
    if (task.rr_remaining_nanoseconds != expected)
        throw std::runtime_error(std::string(message) + " (expected " + std::to_string(expected) +
                                 "ns, actual " + std::to_string(task.rr_remaining_nanoseconds) + "ns)");
}
struct Fixture {
    std::string root;
    Memory memory{8 << 20};
    std::unique_ptr<Filesystem> fs;
    Scheduler scheduler;
    std::unique_ptr<Syscalls> sys;
    Fixture() {
        char pattern[] = "/tmp/operit-scheduling-XXXXXX";
        auto *created = ::mkdtemp(pattern);
        require(created, "mkdtemp");
        root = created;
        fs = std::make_unique<Filesystem>(root.c_str());
        memory.map(0x10000, 0x8000, read_perm | write_perm);
        ort_arm64_config config{};
        config.root_directory = root.c_str();
        sys = std::make_unique<Syscalls>(memory, *fs, config, scheduler);
    }
    ~Fixture() {
        sys.reset();
        std::filesystem::remove_all(root);
    }
    int64_t call(uint64_t number, std::initializer_list<uint64_t> arguments) {
        auto &cpu = scheduler.current().cpu;
        cpu.x.fill(0);
        std::copy(arguments.begin(), arguments.end(), cpu.x.begin());
        cpu.x[8] = number;
        sys->handle(cpu);
        return int64_t(cpu.x[0]);
    }
    void input(uint32_t policy = 0, int32_t nice = 0, uint32_t priority = 0, uint64_t flags = 0) {
        Attributes value;
        value.policy = policy;
        value.nice = nice;
        value.priority = priority;
        value.flags = flags;
        value.write(memory, attributes_word, Attributes::current_size);
    }
    int64_t set(uint32_t tid = 0) { return call(274, {tid, attributes_word, 0}); }
    void seed(Resource resource, uint64_t soft, uint64_t hard) {
        // Model a preconfigured/inherited kernel limit. Guest syscalls still
        // have no capabilities and must not be able to raise this hard cap.
        sys->resource_limits()->set(resource, {soft, hard}, true);
    }
    int64_t limit(Resource resource, uint64_t soft, uint64_t hard) {
        memory.store(limit_word, soft, 8);
        memory.store(limit_word + 8, hard, 8);
        return call(261, {0, uint32_t(resource), limit_word, previous_word});
    }
};
void getter_sizes_and_validation() {
    Fixture f;
    for (const uint32_t size : {48u, 49u, 55u, 56u, 80u, 4096u}) {
        for (unsigned byte = 0; byte != 80; ++byte)
            f.memory.store(attributes_word + byte, 0xa5, 1);
        require(f.call(275, {0, attributes_word, size, 0}) == 0, "sched_getattr ABI size");
        const auto copied = std::min(size, Attributes::current_size);
        require(f.memory.load(attributes_word, 4) == copied &&
                    f.memory.load(attributes_word + 4, 4) == 0 &&
                    f.memory.load(attributes_word + 8, 8) == 0,
                "sched_getattr initial fields");
        for (uint32_t byte = 16; byte < copied; ++byte)
            require(!f.memory.load(attributes_word + byte, 1), "sched_getattr leaked output bytes");
        for (uint32_t byte = copied; byte < 80; ++byte)
            require(f.memory.load(attributes_word + byte, 1) == 0xa5, "sched_getattr overwrote future tail");
    }
    require(f.call(275, {0, attributes_word, 47, 0}) == -22 &&
                f.call(275, {0, attributes_word, 4097, 0}) == -22 &&
                f.call(275, {0, attributes_word, 56, 1}) == -22, "sched_getattr size/flag validation");
    require(f.call(275, {0, 0, 56, 0}) == -22 && f.call(275, {0, 0x50000, 56, 0}) == -14 &&
                f.call(275, {UINT32_MAX, attributes_word, 56, 0}) == -22 &&
                f.call(275, {999, attributes_word, 56, 0}) == -3, "sched_getattr pointer/pid validation");
    f.memory.protect(attributes_word, 4096, read_perm);
    require(f.call(275, {0, attributes_word, 56, 0}) == -14, "sched_getattr wrote a read-only buffer");
}
void setter_versioning_and_errors() {
    Fixture f;
    f.input(0, 5);
    f.memory.store(attributes_word, 0, 4);
    require(f.set() == 0 && f.scheduler.current().nice == 5, "zero size did not select version 0");
    f.input(0, 6);
    for (unsigned byte = 56; byte != 80; ++byte)
        f.memory.store(attributes_word + byte, 0, 1);
    f.memory.store(attributes_word, 80, 4);
    require(f.set() == 0 && f.scheduler.current().nice == 6, "zero future extension rejected");
    f.memory.store(attributes_word + 79, 1, 1);
    require(f.set() == -7 && f.memory.load(attributes_word, 4) == 56 &&
                f.scheduler.current().nice == 6, "nonzero future tail did not report size without mutation");
    for (uint32_t size : {47u, 4097u}) {
        f.memory.store(attributes_word, size, 4);
        require(f.set() == -7 && f.memory.load(attributes_word, 4) == 56, "invalid setter size feedback");
    }
    f.memory.store(attributes_word, 47, 4);
    f.memory.protect(attributes_word, 4096, read_perm);
    require(f.set() == -7 && f.memory.load(attributes_word, 4) == 47, "read-only size feedback changed E2BIG");
    f.memory.protect(attributes_word, 4096, read_perm | write_perm);
    f.memory.store(0x17ff0, 56, 4);
    require(f.call(274, {0, 0x17ff0, 0}) == -14, "setter read across an unmapped boundary");
    f.input(0, 7, 0, 32);
    f.memory.store(attributes_word, 48, 4);
    require(f.set() == -22, "old-size utilization request accepted");
    f.memory.store(attributes_word, 56, 4);
    require(f.set() == -95, "unsupported utilization request reported success");
    f.input(0, 7, 0, 128);
    require(f.set() == -22, "unknown scheduler flags accepted");
    f.input(6, 7);
    require(f.set() == -95, "unsupported deadline policy reported success");
    f.input(999, 7);
    require(f.set() == -22, "unknown scheduler policy accepted");
    f.input(UINT32_MAX, 7, 0, Attributes::keep_policy);
    require(f.set() == -22, "negative policy accepted under KEEP_POLICY");
    f.input(0, 7);
    f.memory.store(attributes_word + 24, 1, 8);
    require(f.set() == -95, "custom runtime slice accepted without an executor implementation");
    f.input(0, 7);
    require(f.set(999) == -3 && f.set(UINT32_MAX) == -22 &&
                f.call(274, {0, 0, 0}) == -22 && f.call(274, {0, attributes_word, 1}) == -22 &&
                f.call(274, {0, 0x50000, 0}) == -14, "setter pointer/pid/argument validation");
    require(f.scheduler.current().nice == 6, "failed setter changed scheduling state");
    Fixture readonly;
    readonly.input(0, 7);
    readonly.memory.protect(attributes_word, 4096, read_perm);
    require(readonly.set() == 0 && readonly.scheduler.current().nice == 7,
            "valid input incorrectly required a writable sched_attr");
}
void nice_limit_enforcement_and_inheritance() {
    Fixture f;
    require(f.limit(Resource::Nice, 1, 1) == -1, "guest gained authority to raise a hard limit");
    f.seed(Resource::Nice, 30, 40);
    require(f.limit(Resource::Nice, 25, 40) == 0 && f.memory.load(previous_word, 8) == 30 &&
                f.memory.load(previous_word + 8, 8) == 40, "NICE prlimit mutation or old-pair copy");
    require(f.call(140, {0, 0, uint64_t(-5)}) == 0 && f.call(141, {0, 0}) == 25,
            "setpriority ignored the permitted NICE threshold");
    f.input(0, -6);
    require(f.set() == -1 && f.call(140, {0, 0, uint64_t(-6)}) == -13 &&
                f.scheduler.current().nice == -5, "NICE ceiling or errno distinction lost");
    require(f.limit(Resource::Nice, 26, 40) == 0 && f.set() == 0 &&
                f.scheduler.current().nice == -6, "raised soft NICE limit did not enable the requested priority");
    require(f.limit(Resource::Nice, 0, 40) == 0 && f.scheduler.current().nice == -6,
            "lowered NICE limit retroactively changed priority");
    f.input(0, 100);
    require(f.set() == 0 && f.scheduler.current().nice == 19, "positive nice was not clamped");
    require(f.limit(Resource::Nice, 40, 40) == 0, "restore NICE soft ceiling");
    f.input(0, INT32_MIN);
    require(f.set() == 0 && f.scheduler.current().nice == -20, "negative nice clamp or maximum permission");
    const auto limits = f.sys->resource_limits();
    const auto thread = process::inherit_resource_limits(limits, 0x10000);
    const auto process_copy = process::inherit_resource_limits(limits, 0);
    require(thread == limits && process_copy != limits, "thread/fork limit ownership");
    require(f.limit(Resource::Nice, 0, 40) == 0 && thread->get(Resource::Nice).soft == 0 &&
                process_copy->get(Resource::Nice).soft == 40, "NICE inheritance did not share/copy appropriately");
    require(f.limit(Resource::Nice, 41, 41) == -1 && f.limit(Resource::Nice, 41, 40) == -22,
            "NICE limit mutation bypassed generic hard/soft checks");
}
void realtime_limit_enforcement_and_legacy_calls() {
    Fixture f;
    f.seed(Resource::RealtimePriority, 20, 30);
    require(f.limit(Resource::RealtimePriority, 10, 30) == 0, "RTPRIO soft mutation");
    f.memory.store(attributes_word, 11, 4);
    require(f.call(119, {0, 1, attributes_word}) == -1, "legacy RT priority exceeded its ceiling");
    f.memory.store(attributes_word, 10, 4);
    require(f.call(119, {0, 1, attributes_word}) == 0 && f.call(120, {0}) == 1,
            "legacy FIFO priority did not use the permitted ceiling");
    require(f.call(275, {0, attributes_word, 56, 0}) == 0 &&
                f.memory.load(attributes_word + 4, 4) == 1 && f.memory.load(attributes_word + 20, 4) == 10 &&
                f.memory.load(attributes_word + 16, 4) == 0, "RT sched_getattr fields");
    require(f.limit(Resource::RealtimePriority, 0, 30) == 0 && f.scheduler.current().rt_priority == 10,
            "lowered RTPRIO limit retroactively changed priority");
    f.memory.store(attributes_word, 9, 4);
    require(f.call(118, {0, attributes_word}) == 0, "lowering existing RT priority was denied");
    f.memory.store(attributes_word, 10, 4);
    require(f.call(118, {0, attributes_word}) == -1, "raising RT priority ignored a zero ceiling");
    f.input(2, 0, 9);
    require(f.set() == -1, "changing RT policy ignored a zero ceiling");
    require(f.limit(Resource::RealtimePriority, 20, 30) == 0, "restore RTPRIO ceiling");
    f.input(2, 0, 15);
    require(f.set() == 0 && f.scheduler.current().rt_priority == 15 &&
                f.scheduler.effective_priority(1) == 84, "sched_setattr did not update actual RT scheduling priority");
    f.input(2, 0, 100);
    require(f.set() == -22 && f.scheduler.current().rt_priority == 15, "invalid RT priority changed state");
}
void keep_flags_reset_and_child_state() {
    Fixture f;
    f.input(0, 3, 0, Attributes::keep_policy | Attributes::reset_on_fork);
    require(f.set() == 0 && !f.scheduler.current().sched_reset_on_fork,
            "KEEP_POLICY changed the retained reset-on-fork flag");
    f.input(3, 5, 0, Attributes::reset_on_fork);
    require(f.set() == 0 && f.scheduler.current().sched_reset_on_fork, "reset-on-fork attribute setup");
    f.input(0, 6);
    require(f.set() == -1 && f.scheduler.current().nice == 5, "guest cleared sticky reset-on-fork");
    f.input(999, 6, 0, Attributes::keep_policy);
    require(f.set() == 0 && f.scheduler.current().sched_policy == Task::SchedulingPolicy::Batch &&
                f.scheduler.current().nice == 6 && f.scheduler.current().sched_reset_on_fork,
            "KEEP_POLICY lost policy/reset state or failed to apply parameters");
    f.input(0, -20, 99, Attributes::keep_parameters | Attributes::reset_on_fork);
    f.memory.store(attributes_word + 24, 1, 8);
    require(f.set() == 0 && f.scheduler.current().nice == 6 &&
                f.scheduler.current().sched_policy == Task::SchedulingPolicy::Batch,
            "KEEP_PARAMS changed retained parameters");
    f.input(999, -20, 99, Attributes::keep_parameters | Attributes::keep_policy);
    require(f.set() == 0 && f.scheduler.current().sched_reset_on_fork, "KEEP_ALL lost retained attributes");
    f.input(1, 0, 0, Attributes::keep_parameters | Attributes::reset_on_fork);
    require(f.set() == -22, "KEEP_PARAMS bypassed incompatible policy/priority validation");
    f.seed(Resource::Nice, 40, 40);
    f.seed(Resource::RealtimePriority, 20, 20);
    f.input(1, 0, 10, Attributes::reset_on_fork);
    require(f.set() == 0 && f.call(140, {0, 0, uint64_t(-10)}) == 0, "reset child parent setup");
    const auto child = f.scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    const auto *task = f.scheduler.lookup(child);
    require(task && task->sched_policy == Task::SchedulingPolicy::Other && !task->rt_priority &&
                task->nice == 0 && !task->sched_reset_on_fork, "fork reset did not clear RT/negative nice state");
}
void attributes_reach_runnable_selection() {
    Fixture f;
    f.seed(Resource::Nice, 40, 40);
    const auto first = f.scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    const auto second = f.scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    f.input(0, -5);
    require(f.set(second) == 0, "targeted NICE attributes");
    require(f.scheduler.runnable_tids(1) == std::vector<uint32_t>{second},
            "changed attributes did not affect real runnable selection");
    f.input(5, 0);
    require(f.set(second) == 0 && f.scheduler.effective_priority(second) == 140,
            "idle attributes did not reach scheduler priority");
    f.input(0, 0);
    require(f.set(second) == 0 && f.scheduler.lookup(second)->nice == 0,
            "leaving idle with a sufficient NICE ceiling failed");
    (void)first;
}
void set_policy(Scheduler &scheduler, uint32_t tid, Task::SchedulingPolicy policy, unsigned priority) {
    auto &task = scheduler.find(tid);
    task.sched_policy = policy;
    task.rt_priority = priority;
    scheduler.scheduling_changed(tid);
}
void fifo_retains_position_and_preempts() {
    Scheduler s;
    const auto second = s.spawn(CpuState{}, 0x18000, 0, 0);
    const auto high = s.spawn(CpuState{}, 0x18000, 0, 0);
    set_policy(s, 1, Task::SchedulingPolicy::Fifo, 20);
    set_policy(s, second, Task::SchedulingPolicy::Fifo, 20);
    for (unsigned boundary = 0; boundary != 10; ++boundary)
        require(s.select(false) && s.current_tid() == 1, "FIFO rotated at an ordinary execution boundary");
    set_policy(s, high, Task::SchedulingPolicy::Fifo, 30);
    require(s.select(false) && s.current_tid() == high, "higher RT priority did not preempt FIFO");
    s.block({}, 0);
    require(s.select(false) && s.current_tid() == 1, "FIFO lost its place across preemption");
    s.yield();
    require(s.select(false) && s.current_tid() == second, "FIFO yield did not select its equal-priority peer");
    s.wake(high);
    require(s.select(false) && s.current_tid() == high, "higher-priority wake did not preempt");
    s.block({}, 0);
    require(s.select(false) && s.current_tid() == second, "preemption changed FIFO peer order");
    s.block({}, 0);
    s.wake(second);
    require(s.select(false) && s.current_tid() == 1, "FIFO wake did not join the tail");
    s.stop_process(1);
    require(!s.select(false), "stopped RT process remained runnable");
    s.continue_process(1);
    require(s.find(high).state == Task::State::Waiting && s.select(false) && s.current_tid() == 1,
            "continue woke a blocked task or lost runnable FIFO order");
}
void realtime_priority_moves_and_rr_budget() {
    Scheduler s;
    const auto second = s.spawn(CpuState{}, 0x18000, 0, 0);
    set_policy(s, 1, Task::SchedulingPolicy::RoundRobin, 30);
    set_policy(s, second, Task::SchedulingPolicy::Fifo, 20);
    require(s.select(false) && s.current_tid() == 1, "RR setup");
    s.account_runtime(1, 25'000'000);
    require(s.current().rr_remaining_nanoseconds == 75'000'000 && s.select(false) && s.current_tid() == 1,
            "RR rotated before consuming its CPU-time budget");
    // Lowering a priority joins the head of the destination RT queue.
    set_policy(s, 1, Task::SchedulingPolicy::RoundRobin, 20);
    require(s.select(false) && s.current_tid() == 1, "lowered RT priority joined the wrong queue position");
    s.yield();
    require(s.find(1).rr_remaining_nanoseconds == 75'000'000 && s.select(false) && s.current_tid() == second,
            "RR yield reset its unused CPU time or failed to rotate");
    s.block({}, 0);
    require(s.select(false) && s.current_tid() == 1, "RR after peer block");
    s.wake(second);
    s.account_runtime(1, 74'999'999);
    require(s.select(false) && s.current_tid() == 1 && s.current().rr_remaining_nanoseconds == 1,
            "RR slice expired early");
    s.account_runtime(1, 1);
    require(s.find(1).rr_remaining_nanoseconds == round_robin_interval_nanoseconds &&
                s.select(false) && s.current_tid() == second, "RR expiry did not rotate behind FIFO");
    // FIFO never consumes an RR quantum, even across large accounted deltas.
    s.account_runtime(second, UINT64_MAX);
    require(s.select(false) && s.current_tid() == second, "FIFO was time sliced");
    s.yield();
    require(s.select(false) && s.current_tid() == 1, "FIFO yield did not return to RR");
    s.account_runtime(1, 5'000'000);
    s.block({}, 0);
    s.wake(1);
    require(s.find(1).rr_remaining_nanoseconds == 95'000'000 && s.select(false) && s.current_tid() == second,
            "RR sleep reset its budget or wake bypassed FIFO");
    set_policy(s, 1, Task::SchedulingPolicy::RoundRobin, 30);
    require(s.select(false) && s.current_tid() == 1, "raised RT priority did not preempt");
    const auto before_policy_change = s.find(1).rr_remaining_nanoseconds;
    set_policy(s, 1, Task::SchedulingPolicy::Fifo, 30);
    s.account_runtime(1, 200'000'000);
    set_policy(s, 1, Task::SchedulingPolicy::Other, 0);
    s.account_runtime(1, 200'000'000);
    set_policy(s, 1, Task::SchedulingPolicy::RoundRobin, 30);
    require(s.find(1).rr_remaining_nanoseconds == before_policy_change,
            "changing policy reset or consumed the task's saved RR budget");
    s.account_runtime(1, UINT64_MAX);
    require(s.find(1).rr_remaining_nanoseconds > 0 &&
                s.find(1).rr_remaining_nanoseconds <= round_robin_interval_nanoseconds,
            "large RR accounting delta overflowed its budget");
}
void realtime_affinity_batch_matching() {
    Scheduler s;
    s.set_execution_cpus(2);
    const auto peer = s.spawn(CpuState{}, 0x18000, 0, 0);
    const auto ordinary = s.spawn(CpuState{}, 0x18000, 0, 0);
    s.find(1).cpu_affinity = s.find(peer).cpu_affinity = 1;
    s.find(ordinary).cpu_affinity = 2;
    set_policy(s, 1, Task::SchedulingPolicy::Fifo, 20);
    set_policy(s, peer, Task::SchedulingPolicy::Fifo, 20);
    for (unsigned boundary = 0; boundary != 3; ++boundary) {
        const auto batch = s.runnable_batch(2);
        require(batch.size() == 2 && batch[0].tid == 1 && batch[0].cpu == 0 &&
                    batch[1].tid == ordinary && batch[1].cpu == 1,
                "pinned FIFO peer ran sequentially on an occupied CPU or left another CPU idle");
    }
    {
        Scheduler::ScopedTask scope(s, 1);
        s.yield();
    }
    require(s.runnable_batch(2).front().tid == peer, "batch path ignored FIFO yield");
    // The higher-ranked flexible task must move to CPU 1 so the pinned peer
    // can use CPU 0. Greedy first-free placement would incorrectly lose work.
    s.find(peer).cpu_affinity = 3;
    const auto matching = s.runnable_batch(2);
    require(matching.size() == 2 && matching[0].tid == peer && matching[0].cpu == 1 &&
                matching[1].tid == 1 && matching[1].cpu == 0, "CPU augmenting path lost a higher-ranked task");
    s.set_execution_cpus(64);
    s.find(peer).cpu_affinity = uint64_t{1} << 63;
    const auto last_cpu = s.runnable_batch(2);
    require(last_cpu.front().tid == peer && last_cpu.front().cpu == 63, "CPU 63 mask overflow");
}
void rejected_parallel_batch_preserves_serial_rotation() {
    Scheduler scheduler;
    scheduler.set_execution_cpus(4);
    const auto peer = scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    scheduler.find(1).cpu_affinity = scheduler.find(peer).cpu_affinity = 1;
    for (unsigned boundary = 0; boundary != 8; ++boundary) {
        const auto expected = boundary % 2 ? 1 : peer;
        require(scheduler.select(false) && scheduler.current_tid() == expected,
                "rejected parallel batch starved a CPU-pinned ordinary peer");
        require(scheduler.runnable_batch(4, 2).empty() && scheduler.current_tid() == expected,
                "singleton parallel probe replaced the selected serial task");
    }
}
void serial_selection_matches_batch_order() {
    Scheduler serial, batch;
    for (unsigned i = 0; i != 5; ++i) {
        require(serial.spawn(CpuState{}, 0x18000, 0, 0) == batch.spawn(CpuState{}, 0x18000, 0, 0),
                "selection comparison task IDs differ");
    }
    for (unsigned step = 0; step != 180; ++step) {
        for (Scheduler *scheduler : {&serial, &batch}) {
            for (uint32_t tid = 1; tid <= 6; ++tid) {
                auto &task = scheduler->find(tid);
                task.nice = int32_t((step / 7 + tid * 3) % 5) - 2;
                task.cpu_affinity = (step + tid) % 11 == 0 ? 0 : 1;
                task.stopped = (step / 3 + tid) % 13 == 0;
                const bool realtime = (step / 9 + tid) % 4 == 0;
                task.sched_policy = realtime ? Task::SchedulingPolicy::Fifo : Task::SchedulingPolicy::Other;
                task.rt_priority = realtime ? 10 + (tid % 3) : 0;
                scheduler->scheduling_changed(tid);
            }
            if (step % 5 == 0)
                scheduler->yield();
        }
        const bool selected = serial.select(false);
        const auto candidates = batch.runnable_batch(1);
        require(selected == !candidates.empty(), "serial and batch runnable state differs");
        if (selected)
            require(serial.current_tid() == candidates.front().tid, "serial and batch task order differs");
    }
}
void realtime_exec_rekey_and_interval_syscall() {
    Fixture f;
    f.seed(Resource::RealtimePriority, 30, 30);
    f.input(2, 0, 20);
    require(f.set() == 0 && f.call(127, {0, attributes_word}) == 0 &&
                f.memory.load(attributes_word, 8) == 0 &&
                f.memory.load(attributes_word + 8, 8) == round_robin_interval_nanoseconds,
            "sched_rr_get_interval did not expose the real guest RR budget");
    require(f.call(127, {UINT32_MAX, attributes_word}) == -22 &&
                f.call(127, {999, attributes_word}) == -3 && f.call(127, {0, 0}) == -14,
            "sched_rr_get_interval pid/buffer validation");
    f.scheduler.account_runtime(1, 10'000'000);
    const auto child = f.scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    require(f.scheduler.find(child).rr_remaining_nanoseconds == round_robin_interval_nanoseconds &&
                f.scheduler.find(1).rr_remaining_nanoseconds == 90'000'000,
            "new RR task inherited a partly used quantum or reset the parent's slice");
    require(f.call(124, {}) == 0 && f.scheduler.select(false) && f.scheduler.current_tid() == child &&
                f.scheduler.find(1).rr_remaining_nanoseconds == 90'000'000,
            "guest sched_yield did not reach the guest queue or reset RR budget");
    // exec keeps the calling child's budget when it assumes the leader's TID.
    // Give it a partial slice distinct from both the parent's 90ms and a fresh 100ms.
    f.scheduler.account_runtime(child, 25'000'000);
    require_rr_budget(f.scheduler.find(child), 75'000'000, "child RR accounting before exec");
    f.scheduler.replace_process_image(1, CpuState{});
    require(!f.scheduler.lookup(child) && f.scheduler.find(1).sched_policy == Task::SchedulingPolicy::RoundRobin &&
                f.scheduler.select(false) && f.scheduler.current_tid() == 1,
            "nonleader exec left a stale RT queue key");
    require_rr_budget(f.scheduler.find(1), 75'000'000,
                      "nonleader exec reset the caller's RR slice or copied the old leader's slice");
}
void realtime_policy_syscalls_preserve_rr_budget() {
    Fixture f;
    f.seed(Resource::RealtimePriority, 30, 30);
    f.input(2, 0, 20);
    require(f.set() == 0, "RR policy setup for syscall budget checks");
    f.scheduler.account_runtime(1, 10'000'000);
    require_rr_budget(f.scheduler.find(1), 90'000'000, "RR accounting before policy changes");
    f.input(1, 0, 20);
    require(f.set() == 0 && f.call(127, {0, attributes_word}) == 0 &&
                !f.memory.load(attributes_word + 8, 8), "FIFO incorrectly reported an RR time slice");
    f.scheduler.account_runtime(1, 200'000'000);
    require_rr_budget(f.scheduler.find(1), 90'000'000, "FIFO consumed the saved RR slice");
    f.input(2, 0, 20);
    require(f.set() == 0, "sched_setattr RR policy change failed");
    require_rr_budget(f.scheduler.find(1), 90'000'000,
                      "sched_setattr reset the RR slice across FIFO/RR policy changes");
    f.memory.store(attributes_word, 30, 4);
    require(f.call(118, {0, attributes_word}) == 0 && f.scheduler.find(1).rt_priority == 30,
            "sched_setparam did not change RR priority");
    require_rr_budget(f.scheduler.find(1), 90'000'000, "sched_setparam reset the unused RR slice");
    f.input(2, 0, 31);
    require(f.set() == -1 && f.scheduler.find(1).rt_priority == 30,
            "rejected sched_setattr changed RR priority");
    require_rr_budget(f.scheduler.find(1), 90'000'000, "rejected sched_setattr changed the RR slice");
    f.memory.store(attributes_word, 20, 4);
    require(f.call(119, {0, 1, attributes_word}) == 0, "legacy FIFO policy change failed");
    f.scheduler.account_runtime(1, 200'000'000);
    require_rr_budget(f.scheduler.find(1), 90'000'000, "legacy FIFO consumed the saved RR slice");
    require(f.call(119, {0, 2, attributes_word}) == 0, "legacy RR policy change failed");
    require_rr_budget(f.scheduler.find(1), 90'000'000,
                      "legacy sched_setscheduler reset the unused RR slice");
    f.input(0);
    require(f.set() == 0 && f.call(127, {0, attributes_word}) == 0 &&
                !f.memory.load(attributes_word, 8) && !f.memory.load(attributes_word + 8, 8),
            "ordinary policy incorrectly reported a realtime RR quantum");
    f.scheduler.account_runtime(1, 200'000'000);
    f.input(2, 0, 20);
    require(f.set() == 0, "RR policy restore after ordinary service failed");
    require_rr_budget(f.scheduler.find(1), 90'000'000, "ordinary service reset or consumed the saved RR slice");
    const auto peer = f.scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    f.memory.store(attributes_word, 20, 4);
    require(f.call(119, {peer, 1, attributes_word}) == 0, "FIFO peer setup after policy changes");
    f.scheduler.account_runtime(1, 89'999'999);
    require_rr_budget(f.scheduler.find(1), 1, "restored RR slice expired early");
    require(f.scheduler.select(false) && f.scheduler.current_tid() == 1,
            "restored RR slice rotated before expiry");
    f.scheduler.account_runtime(1, 1);
    require_rr_budget(f.scheduler.find(1), round_robin_interval_nanoseconds,
                      "restored RR slice did not replenish at expiry");
    require(f.scheduler.select(false) && f.scheduler.current_tid() == peer,
            "restored RR slice did not rotate behind its FIFO peer at expiry");
}
void realtime_limit_thresholds_and_permissions() {
    Fixture f;
    auto limits = f.sys->resource_limits();
    require(f.limit(Resource::RealtimeTime, 10, 3'000'000) == 0 &&
                f.memory.load(previous_word, 8) == process::resource_infinity,
            "RTTIME prlimit did not accept a supported guest limit or return the old value");
    require(limits->check_realtime_time(0) == 0 && limits->check_realtime_time(9'999) == 0 &&
                limits->check_realtime_time(10'000) == 24 &&
                limits->get(Resource::RealtimeTime).soft == 1'000'010,
            "RTTIME microsecond threshold or one-second soft promotion");
    require(limits->check_realtime_time(1'000'009'999) == 0 &&
                limits->check_realtime_time(1'000'010'000) == 24 &&
                limits->get(Resource::RealtimeTime).soft == 2'000'010,
            "RTTIME repeated soft expiry did not use continuous per-thread CPU time");
    require(limits->check_realtime_time(2'999'999'999) == 24 &&
                limits->check_realtime_time(3'000'000'000) == 9,
            "RTTIME hard ceiling did not override soft promotion");
    require(f.limit(Resource::RealtimeTime, 1, 3'000'001) == -1 &&
                limits->get(Resource::RealtimeTime).hard == 3'000'000,
            "RTTIME allowed a guest hard-limit increase");
    require(f.limit(Resource::RealtimeTime, 5, 4) == -22,
            "RTTIME accepted an inverted soft/hard pair");
    const auto shared = process::inherit_resource_limits(limits, 0x10000);
    const auto copied = process::inherit_resource_limits(limits, 0);
    require(shared == limits && copied != limits && copied->get(Resource::RealtimeTime).hard == 3'000'000,
            "RTTIME limits did not share with threads and copy for child processes");
    require(f.limit(Resource::RealtimeTime, 0, 0) == 0 && limits->check_realtime_time(0) == 0 &&
                limits->check_realtime_time(1) == 9,
            "zero RTTIME hard limit did not wait for actual CPU service");
    Fixture unlimited;
    require(unlimited.sys->resource_limits()->check_realtime_time(UINT64_MAX) == 0,
            "unlimited RTTIME signalled or overflowed");
}
void realtime_watchdog_wake_and_policy_state() {
    Scheduler scheduler;
    set_policy(scheduler, 1, Task::SchedulingPolicy::RoundRobin, 20);
    const auto peer = scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    scheduler.account_runtime(1, 20'000, false);
    require(!scheduler.find(1).realtime_runtime_nanoseconds, "unlimited RT execution started the watchdog");
    scheduler.account_runtime(1, 30'000, true);
    require(scheduler.find(1).realtime_runtime_nanoseconds == 30'000 &&
                !scheduler.find(peer).realtime_runtime_nanoseconds, "RTTIME service was shared across threads");
    scheduler.yield();
    require(scheduler.find(1).realtime_runtime_nanoseconds == 30'000 &&
                scheduler.select(false) && scheduler.current_tid() == peer,
            "yield reset the continuous RTTIME counter");
    scheduler.account_runtime(peer, 40'000, true);
    scheduler.block({}, 0);
    scheduler.account_runtime(peer, 1'000, true); // CPU used by the blocking syscall before it sleeps.
    scheduler.wake(peer);
    require(!scheduler.find(peer).realtime_runtime_nanoseconds &&
                scheduler.find(1).realtime_runtime_nanoseconds == 30'000,
            "a real wake did not reset just the sleeping thread's RTTIME counter");
    scheduler.continue_process(1);
    require(scheduler.find(1).realtime_runtime_nanoseconds == 30'000,
            "SIGCONT on an already running process reset its RTTIME service");
    set_policy(scheduler, 1, Task::SchedulingPolicy::Fifo, 20);
    scheduler.account_runtime(1, 10'000, false); // No finite watchdog limit is active.
    set_policy(scheduler, 1, Task::SchedulingPolicy::Other, 0);
    scheduler.account_runtime(1, 20'000, true);
    set_policy(scheduler, 1, Task::SchedulingPolicy::RoundRobin, 20);
    require(scheduler.find(1).realtime_runtime_nanoseconds == 30'000,
            "ordinary/unlimited execution changed the saved realtime timeout");
    scheduler.account_runtime(1, UINT64_MAX, true);
    scheduler.account_runtime(1, 1, true);
    require(scheduler.find(1).realtime_runtime_nanoseconds == UINT64_MAX,
            "RTTIME CPU counter wrapped and evaded its limit");
    uint32_t child;
    {
        Scheduler::ScopedTask context(scheduler, 1);
        child = scheduler.spawn(CpuState{}, 0x18000, 0, 0);
    }
    require(!scheduler.find(child).realtime_runtime_nanoseconds &&
                scheduler.find(1).realtime_runtime_nanoseconds == UINT64_MAX,
            "a newly created task inherited or reset its parent's realtime timeout");
    scheduler.stop_process(1);
    scheduler.continue_process(1);
    require(!scheduler.find(1).realtime_runtime_nanoseconds,
            "a stopped task did not start a fresh interval when actually resumed");
}
void machine_realtime_watchdog(const char *root) {
    // Deterministic SI_KERNEL/process-directed signal delivery and per-thread
    // thresholds, followed by real CPU service in serial and worker execution.
    {
        ort_arm64_config config{};
        config.root_directory = root;
        config.execution_threads = 1;
        Machine machine(config);
        const char *arguments[] = {"/watchdog", nullptr};
        require(machine.load("/watchdog", arguments, nullptr) == 0, "watchdog ELF load");
        auto &scheduler = machine.scheduler();
        auto *process = machine.find_process(1);
        auto limits = process->image.syscalls->resource_limits();
        limits->set(Resource::RealtimeTime, {1, 5});
        set_policy(scheduler, 1, Task::SchedulingPolicy::Fifo, 20);
        scheduler.find(1).signal_mask = signals::bit(24);
        const auto entry = scheduler.current().cpu;
        const auto peer = scheduler.spawn(entry, entry.sp, 0, 0);
        scheduler.account_runtime(1, 600, true);
        scheduler.account_runtime(peer, 600, true);
        machine.poll_runtime_events();
        require(process->signals->pending.empty(), "RTTIME incorrectly summed different threads");
        scheduler.account_runtime(1, 400, true);
        machine.poll_runtime_events();
        const auto signal = process->signals->pending.take(signals::bit(24));
        require(signal && signal->info.signal == 24 && signal->info.code == 128 &&
                    scheduler.find(1).signals.pending.empty() && scheduler.find(peer).signals.pending.empty() &&
                    limits->get(Resource::RealtimeTime).soft == 1'000'001,
                "RTTIME did not deliver a process-directed SI_KERNEL SIGXCPU or advance its shared soft limit");
        scheduler.account_runtime(peer, 4'400, true);
        machine.poll_runtime_events();
        require(process->signals->pending.mask() & signals::bit(9), "peer hard expiry did not queue SIGKILL");
        const auto result = machine.run(100'000);
        require(result.status == ORT_ARM64_EXITED && result.exit_code == 137,
                "blocking SIGXCPU prevented RTTIME hard-limit process termination");
    }
    {
        ort_arm64_config config{};
        config.root_directory = root;
        config.execution_threads = 1;
        Machine ordinary(config);
        const char *arguments[] = {"/watchdog", nullptr};
        require(ordinary.load("/watchdog", arguments, nullptr) == 0, "ordinary watchdog ELF load");
        ordinary.find_process(1)->image.syscalls->resource_limits()->set(Resource::RealtimeTime, {0, 0});
        const auto result = ordinary.run(10'000);
        require(result.status == ORT_ARM64_BUDGET && result.instructions && !result.unsupported_syscalls &&
                    !ordinary.scheduler().find(1).realtime_runtime_nanoseconds,
                "zero RTTIME limit incorrectly killed ordinary-policy CPU service");
    }
    for (const uint32_t cpus : {1u, 4u}) {
        for (const bool hard : {false, true}) {
            ort_arm64_config config{};
            config.root_directory = root;
            config.execution_threads = cpus;
            Machine machine(config);
            const char *arguments[] = {"/watchdog", nullptr};
            require(machine.load("/watchdog", arguments, nullptr) == 0, "CPU-accounted watchdog ELF load");
            auto &scheduler = machine.scheduler();
            auto limits = machine.find_process(1)->image.syscalls->resource_limits();
            set_policy(scheduler, 1, hard ? Task::SchedulingPolicy::RoundRobin : Task::SchedulingPolicy::Fifo, 20);
            scheduler.find(1).cpu_affinity = cpus > 1 ? 2 : 1;
            uint64_t before_runtime = 0;
            if (cpus > 1) {
                auto background = scheduler.current().cpu;
                background.pc = 0x401300;
                const auto other = scheduler.spawn(background, background.sp, 0, 0);
                set_policy(scheduler, other, Task::SchedulingPolicy::Other, 0);
                scheduler.find(other).cpu_affinity = 1;
                // A one-microsecond expiry can terminate the process in its
                // first batch, before the host overlaps its two jobs. First
                // establish real overlap with both guests still runnable and
                // RTTIME unlimited. Pin the RT guest to worker vCPU 1 so its
                // subsequent expiry must use worker CPU accounting.
                bool parallel = false;
                for (unsigned batch = 0; batch != 20 && !parallel; ++batch) {
                    const auto warmup = machine.run(100'000);
                    require(warmup.status == ORT_ARM64_BUDGET && warmup.instructions &&
                                !warmup.unsupported_syscalls,
                            "RTTIME parallel warmup did not keep both guests running");
                    parallel = warmup.maximum_parallel_tasks >= 2;
                }
                require(parallel && scheduler.find(1).last_cpu == 1 &&
                            scheduler.find(other).last_cpu == 0 && scheduler.find(other).runtime_nanoseconds,
                        "RTTIME warmup did not establish real worker execution");
                require(!scheduler.find(1).realtime_runtime_nanoseconds,
                        "unlimited RTTIME warmup charged the watchdog");
                before_runtime = scheduler.find(1).runtime_nanoseconds;
            }
            limits->set(Resource::RealtimeTime, {1, hard ? 1 : process::resource_infinity});
            const auto result = machine.run(2'000'000);
            require(result.status == ORT_ARM64_EXITED && result.exit_code == (hard ? 137 : 152) &&
                        result.instructions && !result.unsupported_syscalls,
                    "real FIFO/RR CPU service did not enforce RTTIME through syscalls and yield");
            if (cpus > 1) {
                require(result.maximum_parallel_tasks >= 2, "RTTIME fixture did not exercise worker CPU accounting");
                const auto *task = scheduler.lookup(1);
                require(task && task->last_cpu == 1 && task->runtime_nanoseconds > before_runtime &&
                            task->realtime_runtime_nanoseconds >= 1'000,
                        "RTTIME expiry did not charge fresh CPU service on the worker");
            }
        }
    }
}
void machine_realtime_dispatch(const char *root) {
    constexpr uint64_t peer_entry = 0x401200, background_entry = 0x401300, data = 0x401400;
    for (uint32_t cpus : {1u, 4u}) {
        for (bool rr : {false, true}) {
            ort_arm64_config config{};
            config.root_directory = root;
            config.execution_threads = cpus;
            Machine machine(config);
            const char *path = rr ? "/rr" : "/fifo";
            const char *arguments[] = {path, nullptr};
            require(machine.load(path, arguments, nullptr) == 0, "realtime raw ELF load");
            auto &scheduler = machine.scheduler();
            const auto entry = scheduler.current().cpu;
            auto memory = machine.find_process(1)->image.memory;
            set_policy(scheduler, 1, rr ? Task::SchedulingPolicy::RoundRobin : Task::SchedulingPolicy::Fifo, 20);
            scheduler.find(1).cpu_affinity = 1;
            auto peer = entry;
            peer.pc = peer_entry;
            const auto tid = scheduler.spawn(peer, entry.sp, 0, 0);
            scheduler.find(tid).cpu_affinity = 1;
            if (rr)
                scheduler.find(1).rr_remaining_nanoseconds = 1; // Force measurable CPU expiry in the first slice.
            if (cpus > 1) {
                auto background = entry;
                background.pc = background_entry;
                const auto other = scheduler.spawn(background, entry.sp, 0, 0);
                set_policy(scheduler, other, Task::SchedulingPolicy::Other, 0);
                scheduler.find(other).cpu_affinity = 2;
            }
            const auto result = machine.run(2'000'000);
            require(result.status == ORT_ARM64_EXITED && result.exit_code == 0 && !result.unsupported_syscalls,
                    rr ? "RR CPU accounting did not preempt the real guest" :
                         "FIFO switched before guest yield or failed to switch after it");
            require(memory->load(data, 4) == 1 && memory->load(data + 4, 4) == 1,
                    "real guest did not complete the ownership/yield assertions");
            if (cpus > 1)
                require(result.maximum_parallel_tasks >= 2, "realtime fixture did not exercise parallel accounting");
        }
    }
}
} // namespace
int main(int argc, char **argv) {
    try {
        getter_sizes_and_validation();
        setter_versioning_and_errors();
        nice_limit_enforcement_and_inheritance();
        realtime_limit_enforcement_and_legacy_calls();
        keep_flags_reset_and_child_state();
        attributes_reach_runnable_selection();
        fifo_retains_position_and_preempts();
        realtime_priority_moves_and_rr_budget();
        realtime_affinity_batch_matching();
        rejected_parallel_batch_preserves_serial_rotation();
        serial_selection_matches_batch_order();
        realtime_exec_rekey_and_interval_syscall();
        realtime_policy_syscalls_preserve_rr_budget();
        realtime_limit_thresholds_and_permissions();
        realtime_watchdog_wake_and_policy_state();
        if (argc > 2)
            throw Failure("usage: operit-arm64-scheduling-tests [RAW_ELF_ROOT]");
        if (argc == 2) {
            machine_realtime_dispatch(argv[1]);
            machine_realtime_watchdog(argv[1]);
            std::puts("realtime FIFO/RR single/quad-core guest dispatch passed");
            std::puts("realtime RTTIME per-thread watchdog and signal pipeline passed");
        }
        std::puts("scheduling attributes, limits, permission, inheritance and dispatch passed");
        return 0;
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s\n", error.what());
    } catch (const GuestError &error) {
        std::fprintf(stderr, "Linux errno %d\n", error.number);
    }
    return 1;
}
