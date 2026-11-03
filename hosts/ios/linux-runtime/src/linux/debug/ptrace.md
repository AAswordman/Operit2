# AArch64 guest ptrace integration

`Ptrace` is a per-VM, coordinator-only state machine. IDs are guest TIDs;
target CPU pointers must remain valid for the duration of an API call. Hooks
must not re-enter `Ptrace` or retire tasks. The production runtime wires this
state machine into syscall dispatch, wait, signals, memory and task lifecycle
through `src/runtime/process/ptrace.cpp` and the surrounding Machine hooks.
When a VM has a tracing relationship, execution falls back to serial scheduling
to keep tracee state stable while the tracer accesses it.

## Hooks

- `target(tid)` supplies the task's TGID, real parent's TID, process group,
  real/effective/saved credentials, dumpability, permitted capabilities and
  `CAP_SYS_PTRACE`, plus the task's stable CPU state.
- `memory(tid, addr, buffer, size, access)` copies from or to the named task.
  `Read`/`Write` use normal guest permissions. `ForceRead`/`ForceWrite` must
  implement the applicable ptrace `FOLL_FORCE` rules, including private COW,
  permitted maximum protections, exclusive-reservation invalidation, and
  executable-code cache invalidation. Never change guest-visible protections
  temporarily. Return false for inaccessible memory.
- `set_stopped(tid, value)` changes a per-thread ptrace-stop, independently
  of group-stop and `Task::Waiting`. Pending blocking operations remain intact.
- `attach_stop(tid)` queues SIGSTOP, wakes an interruptible blocked target as
  appropriate, and supports an already group-stopped target without executing
  guest instructions. The current module reports this through signal delivery;
  exact Linux group-stop versus signal-stop distinctions remain incomplete.
- `resume_signal(tid, optional<Queued>)` delivers or suppresses the previously
  intercepted signal once, without producing the same delivery stop again.
  Preserve the queued siginfo, forced flag and charge when applicable.
- `queue_signal(tid, Queued)` posts a new thread-directed signal through the
  normal delivery path. It can cause another ptrace signal-delivery stop.
- `notify(tracer_tid, tracee_tid, status)` wakes relevant blocked waits and
  issues the appropriate SIGCHLD; ptrace stops are `CLD_TRAPPED` rather than
  ordinary `CLD_STOPPED`. `status` is an encoded wait status.
- `release_exit(tid)` releases an unconsumed traced exit to its real parent
  when the tracer dies, without reaping it, and wakes the parent's waiter.

## Runtime call order

1. Dispatch raw guest syscall 117 to `request(caller_tid, request, pid, addr,
   data)`. Raw PEEK writes the word to the tracer's `data` pointer and returns
   zero; it does not use the libc wrapper's return-value convention.
2. On both serial and parallel SVC paths, advance PC, save the actual syscall
   number, and call `syscall_entry(tid)` before executing the syscall. If it
   returns true, save the pending operation and schedule away; after resume,
   dispatch it exactly once without re-entering the interpreter. Arguments
   must reflect tracer edits. Linux arm64 saves the syscall number separately
   from x8; this module does not implement `NT_ARM_SYSTEM_CALL` mutation.
3. Call `syscall_exit(tid)` exactly once after the final result is in x0,
   including asynchronous completion of a blocked syscall. Preserve pending
   syscall state across unrelated scheduling and ptrace stops. A fatal signal
   must be processed before a saved operation can execute after wakeup.
4. After selecting and removing a deliverable signal but before disposition or
   handler setup, call `signal_delivery_stop`. If it returns true, leave the
   queued item with the module and schedule away. Invoke `fatal_signal` to wake
   a tracee already in ptrace-stop when SIGKILL arrives.
5. Check `wait` before normal child waiting. Honor `eligible`, `WNOWAIT`,
   `__WNOTHREAD`, process-group selection and non-child traced TIDs. Normalize
   wait4 so `WEXITED` is included. Distinguish a tracer who is also the real
   parent from a different tracer when releasing a consumed exit; the latter
   must leave the exit available to the real parent. A ptrace `waitid` stop is
   `CLD_TRAPPED`. Preserve normal child accounting and copyout behavior.
6. Before a non-leader exec commits, require `can_exec(old_tid, new_tid)`.
   After the TID rekey and before the new image runs, call `exec(old_tid,
   new_tid)`. Do not report the execing thread itself to `tracer_exit` merely
   because its TID changed. The module rekeys its tracee and tracer ownership;
   other threads that actually die still need their lifecycle hooks.
7. Call `task_exit(tid, encoded_wait_status)` before tracee retirement, on
   every exit path including fatal signals and group exit. If it returns true,
   withhold the real parent's normal notification and reap until the tracer
   consumes the exit or dies. Call `tracer_exit(tid)` for each genuinely dying
   tracer thread, including non-leaders. Preserve any independent group-stop.
8. After a successful clone has been installed, call `clone` before either
   task returns to userspace. It selects the fork/vfork/clone event from the
   flags and exit signal, inherits tracer ownership and options where needed,
   and queues the child's initial SIGSTOP. Call `vfork_done` before waking a
   parent whose vfork child has released its address space. Keep event-stop
   x0 separate from the eventual syscall return via
   `preserve_event_syscall_result` after clone dispatch and vfork wakeup.

Stop/resume and hook implementations must be serialized with interpreter
workers, mappings, and exit. In particular, attach to an already group-stopped
task must not deadlock waiting for it to execute another user instruction.

## Implemented and missing

The implemented subset is TRACEME, ATTACH, CONT, SYSCALL, DETACH, PEEK/POKE,
GET/SETREGSET for AArch64 GPR and FPSIMD, GET/SETSIGINFO, GETEVENTMSG, and
SETOPTIONS for TRACESYSGOOD, TRACEFORK, TRACEVFORK, TRACECLONE, TRACEEXEC,
TRACEVFORKDONE and TRACEEXIT. Followed children inherit options and receive an
initial SIGSTOP; CLONE_PTRACE and CLONE_UNTRACED affect inheritance and event
reporting. Syscall stops expose x7=0 at entry
and x7=1 at exit and restore its previous value on resume. Legacy exec queues
a SIGTRAP; TRACEEXEC produces an exec event containing the former TID.
TRACEEXIT covers the tested normal, group and signal exit paths. Exec is
rejected with `EOPNOTSUPP` if retiring a sibling with TRACEEXIT would require
an unsupported event barrier.

SEIZE/INTERRUPT/LISTEN, SINGLESTEP, hardware breakpoints, other regsets
including `NT_ARM_SYSTEM_CALL`, and
`PTRACE_GET_SYSCALL_INFO` are unsupported. Unknown requests are not reported
as successful. Only NZCV is represented in PSTATE. Linux LSM, Yama and user
namespace capability semantics are not fully modeled. Group-stop distinctions
and non-leader exec when the destination TID is already traced are incomplete;
the latter is rejected before exec commits. Complete strace compatibility and
all Linux ptrace races and job-control interactions are not established.

## Independent tests

`tests/ptrace.cpp` exercises the state machine with host fixtures.
`tests/ptrace_memory.cpp` checks memory access and private COW behavior.
`tests/ptrace_guest.py --runtime <integrated-runtime>` builds a freestanding
AArch64 tracer/tracee and executes it with one and four configured CPUs. The
guest checks TRACEME, ATTACH, stop/wait, GPR edits, PEEK/POKE, read-only code
patching with fork COW, syscall entry/exit, x7 restoration, CONT and DETACH.
`tests/ptrace_lifecycle_guest.py` and `tests/ptrace_clone_x8_guest.py` exercise
additional wait, exit, exec, blocked-syscall and clone paths in the integrated
CLI. The lifecycle guest also exercises followed forks and descendants,
clone3, threads, exec, syscall stops and both vfork completion orders.
`tests/ptrace_threads_exit_guest.py` checks independent continuation of
traced threads at group exit, traced leader ordering, fatal signal exit,
detach and tracer death. These tests do not establish complete Linux ptrace
semantics.
