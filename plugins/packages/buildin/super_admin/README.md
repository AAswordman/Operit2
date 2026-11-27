# Super Admin

`super_admin` owns independent command execution, persistent interactive shells,
profile discovery, screen reads, input and explicit close. It is a ToolPkg with a
main runtime, so cleanup can continue without another AI invocation.

## Command and interactive sessions

`terminal` starts a fresh process with the complete script supplied at launch.
The Host sets its initial directory to the current chat's file directory using
the selected profile's VFS mapping. Variables, exports and `cd` do not persist
across calls. The process's actual exit status decides command completion.

The tool consumes `Tools.System.terminal.runStream` until actual completion.
The outer package call waits up to 20 seconds across the entire operation; if
still running it returns `execution_id`. Use `package_wait` to collect subsequent
output and the final result. Proxy `timeout_ms: 0` requests immediate background
execution. No separate terminal wait tool or foreground deadline is used.
Start events expose `sessionId` for `get_screen`, `input` and `close`. Streamed
events and final results omit submitted scripts and internal SDK metadata.

`terminal_open` starts a persistent interactive shell. Use it for REPLs, editors,
interactive programs and commands that must share shell state. `input` sends
literal text, `control=enter` submits a line, and `control=ctrl,input=c` sends
Ctrl+C. Text plus `control=enter` can be sent together. Screen reads do not wait
for a command to finish. Interactive input has no per-command managed exit code.

Both modes use the same Host PTY implementation. Platform details, the bounded
output-drain policy, Windows temporary scripts and browser image requirements
are documented in [terminal command execution](../../../../docs/terminal-command-execution.md).

## Idle cleanup

The plugin registers a platform-neutral Host interval of one minute. A completed
command session becomes eligible for automatic close after **10 minutes** without
plugin activity. Actual close occurs on the next delivered clock event; hosts
that suspend event delivery perform cleanup after delivery resumes.

- Running command processes remain protected, including silent commands and
  commands awaiting input. A process newly observed as finished receives a fresh
  idle period.
- A command stream holds its session lease through completion or failure.
  Screen reads and input refresh that session's deadline and hold its lease
  during the call. `package_wait` reads execution records without extending
  terminal activity.
- A live interactive shell remains protected until explicit close or shell exit.
  It cannot be declared idle merely because an editor or REPL is silent.
- Cleanup checks record identity, revision, leases and Host running state, then
  calls atomic `closeIfIdle` with the exact profile-scoped session family.
- Manual workspace terminals and other plugins' sessions are excluded. Direct
  workspace interaction does not update this plugin's activity records.

The main runtime updates lease records synchronously. Idle probes run
independently; a pending probe cannot block commands in another session. Once a
close has started, acquisition waits only for the affected session or profile
family. Success and failure both release that close reservation. No Host screen
read or close occupies a global plugin session queue.

Explicit close removes the plugin's record and terminates the session. Records
are local to the current main runtime and disappear when it unloads or is
replaced; earlier sessions are not retrospectively discovered.

## Checks

Run from the repository root:

```sh
node --test tools/tests/super_admin_terminal_routing.test.mjs tools/tests/terminal_streaming.test.mjs
node .ci-tools/typescript/node_modules/typescript/bin/tsc --noEmit -p plugins/packages/buildin/super_admin/tsconfig.json
```

These tests evaluate production sources and the generated SDK with controlled
Host responses and a simulated idle clock. Real-shell checks live in
`hosts/tests/terminal-contracts/src/process_sessions.rs`; POSIX relay checks live
in `tools/tests/test_terminal_process_relays.py`.
