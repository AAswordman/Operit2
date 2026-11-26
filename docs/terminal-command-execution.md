# Terminal command execution

Super Admin exposes two launch modes over the existing Host PTY transport.

- `terminal` / SDK `run`: start a fresh command process in the current file
  directory. The complete script is supplied before execution. Each invocation
  has its own variables, environment changes and working directory changes.
- `terminal_open` / SDK `open`: start a persistent interactive shell in the
  current file directory. Input is literal, and the shell retains its variables,
  directory and foreground programs until it exits or the caller closes it.
  Typing a command does not create a managed exit receipt.

Both modes support `get_screen`, `input` and `close` while running. `input` can
send text and Enter in one atomic write. Ctrl+C sends only the control byte; it
does not type a literal `c` first. A write copies its writer handle under the
registry lock, then releases that lock before OS I/O, so a slow write cannot
retain the registry needed by screen reads or other sessions.

The old SDK `create` + `exec` interfaces retain their existing persistent-shell
protocol for compatibility. New command launches never invoke those interfaces,
type `cd` into a shell, or wait for a prompt marker. There is no emulation of the
new process mode using the legacy command wrapper.

## Process launch and completion

`TerminalHost::openTerminalSession` receives the selected terminal profile,
resolved working directory and an optional complete script. Omitting the script
opens an interactive shell. `ProfileTerminalHost` records the immutable backend
owner; the publishing wrapper forwards the same request and session identity.

POSIX, Android PRoot and Android system shells receive `-c` with a literal script
argument. Chroot passes launch mode and script as literal arguments through the
root/mount bootstrap. Termux passes them through the Android service to its
Python PTY owner. The iOS ARM64 bridge supplies the script before loading the
guest shell. OHOS preserves its QEMU launcher prefix and passes shell arguments
through the guest directory bootstrap.

Windows uses a Host-owned temporary UTF-8 script file and launches Bash with the
file path or PowerShell with `-File`. This avoids Windows command-line size limits
and the observed silent truncation of long `-c` arguments in the packaged Bash
runtime. WSL maps the script path through the explicitly selected distribution.
The script file is deleted on observed process exit or session teardown.

The browser guest receives a read-only launch file over its existing 9P mount.
The guest relay starts the shell with that file before accepting terminal input
and emits a process-exit record derived from `waitpid`. This requires rebuilding
and distributing the V86 runtime image with the updated relay and init program.
An older guest image reports an explicit version error rather than running a
different execution protocol.

The shared observer samples Host process status and drains output using finite
Host tasks, with Host-owned 25 ms delays between samples. Completion depends on
actual process exit, never silence, prompt appearance or output-channel EOF.
After exit, it drains until 100 ms without output, with a one-second maximum
drain period. A descendant retaining the PTY cannot hold the parent's receipt
indefinitely. Termux and the browser relay likewise observe the child separately
from output EOF. Retained command output is bounded to the latest 1 MiB.

## Foreground waiting and cleanup

Super Admin waits at most one second **after startup**. A `status: running`
receipt includes the session ID and a null final exit code. `terminal_wait`
observes the same original process for another bounded wait without submitting
input. Expiry neither cancels the process nor reserves its input or screen.
Startup, privilege IPC and writing a large output file are outside that budget.

Running processes remain Host-owned after the tool returns. After completion,
the plugin's minute clock hook can close a session following ten minutes without
plugin activity. Screen, input and result waits refresh activity. A command first
observed as finished receives a fresh idle period; cleanup can therefore occur
later than ten minutes after the OS exit. Exact record identity, activity revision
and leases are checked after each async probe, and Host `closeIfIdle` validates
the session family and running state atomically.

A live interactive shell remains protected until explicit `close` or shell exit.
Its `commandRunning` field represents the live session process, not the status
of each manually typed command. Manual workspace terminals and terminals owned
by other plugins are outside Super Admin's cleanup registry.

The managed observer registry retains at most 32 entries and may evict completed
receipts to admit a new process. It never evicts running work. Session resources
remain under explicit close or the plugin's idle cleanup policy.

## Verification

The terminal contract fixture exercises real Linux Bash and Windows Bash and
PowerShell processes: scripts without a trailing newline, long scripts, syntax
errors, silent exit, output redirection, input while observing, Ctrl+C, close and
persistent interactive state. Core tests cover bounded waits, owner isolation,
late streamed output and independent fresh commands. Node tests exercise the
generated SDK and production plugin's cleanup and concurrency behavior.

`tools/tests/test_terminal_process_relays.py` runs the production Termux Python
helper and browser C relay on POSIX, including a parent exiting while a
descendant holds output open. The Android fixture also supports an ARM64 target
check. These checks do not establish Android Root/Shizuku/PRoot device behavior,
iOS/Objective-C++ execution, OHOS execution or an installed browser guest image.
