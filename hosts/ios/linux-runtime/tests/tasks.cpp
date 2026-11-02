#include "runtime/tasks/scheduler.h"
#include <cstdio>
#include <stdexcept>
using namespace operit::arm64;
namespace {
void require(bool condition, const char *message) {
    if (!condition)
        throw std::runtime_error(message);
}
void inheritance_and_isolation() {
    Scheduler s;
    auto &parent = s.current();
    parent.cpu.x[0] = 77;
    parent.cpu.pc = 0x1234;
    parent.cpu.tls = 8;
    parent.signal_mask = 0x123;
    parent.name = "main";
    parent.cpu.exclusive_valid = true;
    require(s.spawn(parent.cpu, 0x8000, 0x9000, 0xa000) == 2, "TID allocation");
    require(s.select() && s.current().tid == 2, "round robin select");
    auto &child = s.current();
    require(child.cpu.x[0] == 0 && child.cpu.pc == 0x1234 && child.cpu.sp == 0x8000 &&
                child.cpu.tls == 0x9000,
            "clone register state");
    require(child.signal_mask == 0x123 && child.name == "main" && child.clear_tid == 0xa000 &&
                !child.cpu.exclusive_valid,
            "clone task state");
    child.cpu.x[4] = 99;
    child.signal_mask = 0;
    require(s.select() && s.current().tid == 1 && s.current().cpu.x[4] == 0 &&
                s.current().signal_mask == 0x123,
            "task isolation");
    require(!s.current().cpu.exclusive_valid, "switch must invalidate monitor");
}
void futex_masks_requeue_and_timeouts() {
    Scheduler s;
    s.spawn(s.current().cpu, 16, 0, 0);
    s.wait_futex(0x100, 2, {});
    s.select();
    require(s.wake_futex(0x100, 1, 1) == 0, "futex bitset filtering");
    require(s.requeue_futex(0x100, 0x200, 1) == 1, "futex requeue count");
    require(s.wake_futex(0x100, 1) == 0 && s.wake_futex(0x200, 1, 2) == 1, "requeue destination");
    s.select();
    require(s.current().tid == 1 && s.current().cpu.x[0] == 0, "futex return on wake");
    s.wait_futex(0x300, 1, TaskClock::now());
    s.select();
    s.select();
    require(s.current().tid == 1 && int64_t(s.current().cpu.x[0]) == -110, "futex timeout");
}
void callbacks_errors_and_exit() {
    Scheduler s;
    s.block({}, 0, []() -> std::optional<int64_t> { guest_error(14); });
    require(s.select() && int64_t(s.current().cpu.x[0]) == -14, "poll errno completion");
    s.current().cpu.exclusive_valid = true;
    s.select();
    require(s.current().cpu.exclusive_valid, "same-task scheduling must retain exclusive monitor");
    s.spawn(s.current().cpu, 16, 0, 0);
    s.exit(4, false);
    require(!s.exited() && s.select() && s.current().tid == 2, "exit must not kill siblings");
    s.exit(5, false);
    require(s.exited() && s.exit_code() == 5 && !s.select(), "last-thread exit");
    Scheduler group;
    group.spawn(group.current().cpu, 16, 0, 0);
    group.exit(7, true);
    require(group.exited() && group.exit_code() == 7, "exit_group");
}
} // namespace
int main() {
    try {
        inheritance_and_isolation();
        futex_masks_requeue_and_timeouts();
        callbacks_errors_and_exit();
        std::puts("task state, futex masks/requeue/timeouts, polling and exit checks passed");
        return 0;
    } catch (const std::exception &e) {
        std::fprintf(stderr, "%s\n", e.what());
        return 1;
    }
}
