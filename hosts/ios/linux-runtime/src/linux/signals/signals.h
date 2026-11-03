#pragma once
#include "cpu/aarch64/state.h"
#include "memory/address_space.h"
#include <chrono>
#include <deque>
#include <functional>
#include <memory>
#include <optional>
#include <vector>

namespace operit::arm64::process {
class UidLimitCharge;
}

namespace operit::arm64::signals {
constexpr unsigned maximum_signal = 64;
constexpr int kill_signal = 9, stop_signal = 19, continue_signal = 18, segv_signal = 11;
constexpr uint64_t bit(unsigned signal) {
    return signal && signal <= maximum_signal ? uint64_t{1} << (signal - 1) : 0;
}
constexpr uint64_t unblockable = bit(kill_signal) | bit(stop_signal);
constexpr uint64_t sa_no_cldstop = 1, sa_no_cldwait = 2, sa_siginfo = 4;
constexpr uint64_t sa_restorer = 0x04000000, sa_onstack = 0x08000000, sa_restart = 0x10000000;
constexpr uint64_t sa_nodefer = 0x40000000, sa_resethand = 0x80000000;
constexpr uint32_t ss_onstack = 1, ss_disable = 2, ss_autodisarm = 0x80000000;
constexpr uint64_t minimum_signal_stack = 5120;

struct alignas(8) Info {
    int32_t signal = 0, error = 0, code = 0;
    uint32_t padding = 0;
    std::array<uint8_t, 112> payload{};
    static Info user(int signal, uint32_t pid, uint32_t uid, int code = 0);
    static Info fault(int signal, uint64_t address, int code);
    static Info child(uint32_t pid, uint32_t uid, int status, int code);
    uint32_t sender_pid() const;
    uint32_t sender_uid() const;
    uint64_t fault_address() const;
};
static_assert(sizeof(Info) == 128);
struct Action {
    uint64_t handler = 0, flags = 0, restorer = 0, mask = 0;
};
static_assert(sizeof(Action) == 32);
struct AltStack {
    uint64_t pointer = 0;
    uint32_t flags = ss_disable, padding = 0;
    uint64_t size = 0;
};
static_assert(sizeof(AltStack) == 24);
struct Queued {
    Info info;
    bool forced = false;
    std::shared_ptr<process::UidLimitCharge> charge;
};
class Pending {
    std::deque<Queued> entries_;

  public:
    using DequeueObserver = std::function<bool(Info &)>;
    using DiscardObserver = std::function<void(const Info &)>;
    using ChargeSource =
        std::function<std::shared_ptr<process::UidLimitCharge>(const Info &, bool forced)>;

  private:
    DequeueObserver dequeued_;
    DiscardObserver discarded_;
    ChargeSource charge_source_;

  public:
    // Bind both the process queue and each thread queue to their owning process.
    // A dequeue observer may update siginfo, or return false to suppress a stale
    // notification. It runs after removal and before take() exposes the info;
    // returning false consumes that entry and continues to the next candidate.
    // Discard observers run for discard() and for a forced replacement, without
    // also invoking the dequeue observer. All callbacks run at a serial safe point.
    void set_observers(DequeueObserver dequeued, DiscardObserver discarded = {});
    // Called only for a new queue entry or a forced replacement, after standard
    // signal coalescing. The owner resolves target credentials/limits here.
    // A preallocated charge bypasses this callback; EAGAIN rejects admission.
    void set_charge_source(ChargeSource source);
    uint64_t mask() const;
    bool empty() const {
        return entries_.empty();
    }
    size_t size() const {
        return entries_.size();
    }
    void discard(uint64_t set);
    // Predicate sees the original siginfo. Each selected entry is removed before
    // its discard observer runs; other entries keep their order and siginfo.
    size_t discard_if(const std::function<bool(const Info &)> &predicate);
    // Standard signals coalesce; realtime signals retain FIFO order per signal.
    // False means queue capacity or the charge source rejected admission.
    bool enqueue(const Info &info, bool forced = false,
                 std::shared_ptr<process::UidLimitCharge> reserved = {});
    // Returned siginfo snapshots do not retain queue allocation charges.
    std::optional<Queued> peek(uint64_t allowed, bool synchronous_first = false,
                               bool include_forced = true) const;
    std::optional<Queued> take(uint64_t allowed, bool synchronous_first = false,
                               bool include_forced = true);
};
struct ActionTable {
    std::array<Action, maximum_signal> entries{};
};
struct ProcessState {
    std::shared_ptr<ActionTable> actions = std::make_shared<ActionTable>();
    Pending pending;
    bool stopped = false;
};
struct Restart {
    std::array<uint64_t, 9> registers{};
    uint64_t after_svc = 0;
    bool restartable = false, interrupted = false, without_handler_only = false;
    bool always_restart = false; // ERESTARTNOINTR, even without SA_RESTART.
};
struct ThreadState {
    Pending pending;
    AltStack altstack;
    // An atomic-mask wait saves the original mask here until a handler frame
    // has captured it. The canonical live mask remains Task::signal_mask.
    std::optional<uint64_t> restore_mask, interrupted_mask;
    std::optional<Restart> restart;
    std::optional<uint64_t> wait_set;
    bool suspending = false;
};
struct ThreadRef {
    uint32_t tid = 0, pid = 0;
    CpuState *cpu = nullptr;
    ThreadState *signals = nullptr;
    uint64_t *mask = nullptr;
    bool traced = false;
};
struct ProcessRef {
    uint32_t pid = 0, parent = 0, group = 0, session = 0;
    uint32_t real_uid = 0, effective_uid = 0, saved_uid = 0;
    bool zombie = false;
    std::shared_ptr<ProcessState> signals;
    std::vector<ThreadRef> threads;
};
enum class Effect { None, Handler, Terminate, Core, Stop, Continue };
struct Delivery {
    Effect effect = Effect::None;
    int signal = 0;
    Info info;
    explicit operator bool() const {
        return effect != Effect::None;
    }
};
using Clock = std::chrono::steady_clock;
using Deadline = std::optional<Clock::time_point>;
using Poll = std::function<std::optional<int64_t>()>;
struct Hooks {
    std::function<ThreadRef()> current;
    std::function<ProcessRef(uint32_t)> process;
    std::function<std::vector<ProcessRef>()> processes;
    std::function<void(Deadline, int64_t, Poll)> block;
    // interrupt=false requests a scheduler recheck for sigtimedwait; it must
    // not complete that wait with EINTR. interrupt=true completes an
    // interruptible wait with EINTR, calling interrupted() before wait_cleanup.
    std::function<void(uint32_t, bool)> wake;
    // Applies group stop/continue/termination to guest tasks only. A stopped
    // process must be runnable for SIGKILL; Continue with signal=9 is that wake.
    std::function<void(uint32_t, const Delivery &)> effect;
    // Installed in every new address space; handler returns execute this stub.
    uint64_t trampoline = 0;
};

Effect default_effect(int signal);
bool ignored(const ProcessState &process, int signal);
bool has_deliverable(const ProcessState &process, const ThreadState &thread, uint64_t mask);
uint64_t pending_mask(const ProcessState &process, const ThreadState &thread);
// These synchronous consumers preserve thread-before-process priority and never
// take a forced fault. Call at a serial guest safe point, as with delivery.
bool has_waitable(const ProcessState &process, const ThreadState &thread, uint64_t set);
std::optional<Queued> take_waitable(ProcessState &process, ThreadState &thread, uint64_t set);
std::shared_ptr<ProcessState> fork_process(const ProcessState &parent, bool share_actions = false);
ThreadState fork_thread(const ThreadState &parent, bool shared_memory = false, bool vfork = false);
void exec_reset(ProcessState &process, ThreadState &thread);
void remember_syscall(ThreadState &thread, const CpuState &cpu, bool restartable);
void interrupted(ThreadState &thread, uint64_t current_mask);
void begin_temporary_mask(ThreadState &thread, uint64_t &mask, uint64_t temporary);
void end_temporary_mask(ThreadState &thread, uint64_t &mask);

// Queue helpers drive stop/continue cancellation, ignored dispositions, and
// scheduler notifications. User-request permission checks live in dispatch().
int send_process(Hooks &hooks, const ProcessRef &target, const Info &info, bool force = false,
                 std::shared_ptr<process::UidLimitCharge> reserved = {});
int send_thread(Hooks &hooks, const ProcessRef &process, const ThreadRef &target, const Info &info,
                bool force = false, std::shared_ptr<process::UidLimitCharge> reserved = {});
void force_fault(ProcessState &process, ThreadState &thread, uint64_t &mask, int signal, uint64_t address,
                 int code = 1);

uint64_t install_trampoline(Memory &memory, uint64_t preferred = 0x7fffffe00000);
Delivery deliver(Memory &memory, CpuState &cpu, ProcessState &process, ThreadState &thread, uint64_t &mask,
                 uint64_t trampoline, const std::function<bool(CpuState &)> &before_handler = {},
                 std::optional<Queued> *tracer_signal = nullptr,
                 const std::function<bool(const Queued &)> &before_delivery = {});
bool restore_frame(Memory &memory, CpuState &cpu, ThreadState &thread, uint64_t &mask);
AltStack visible_altstack(const ThreadState &thread, uint64_t sp);
void change_altstack(ThreadState &thread, const AltStack &stack, uint64_t sp);

// Returns the restored X0 for rt_sigreturn, so the existing handle() assignment
// does not overwrite registers recovered from a user-edited signal frame.
int64_t dispatch(CpuState &cpu, Memory &memory, Hooks &hooks);
} // namespace operit::arm64::signals
