# iOS sandbox compatibility audit — 2026-11-08

The screenshot reports `ARM64 PTY open failed: terminal open failed with Linux
errno 1`. It does not identify the individual failing host syscall. The terminal
allocator used `posix_openpt`, `grantpt`, `unlockpt`, slave-device open and native
termios initialization. A permission denial at any of those stages previously
prevented a console from being created. The guest `/dev` filesystem is virtual;
recreating guest device nodes does not remove a host sandbox denial.

## Terminal fix

`TerminalManager` still uses native PTYs when their complete initialization
succeeds. Linux `EPERM` or `EACCES` during that initialization selects a
socket-backed software terminal. Other allocation/setup errors remain errors.
The fallback does not open a host PTY device or change host process identity.

The software backend implements canonical and raw input, echo and editing,
EOF, input/output translations, flow control, bounded queues and window size.
The existing guest session/group manager delivers foreground signals and
SIGWINCH. Console APIs, scalar/vector syscalls and terminal output through
sendfile/splice route through the same backend. FIONREAD excludes private EOF
wake tokens. Async io_uring operations on a software terminal return EOPNOTSUPP
so that a worker cannot silently bypass its line discipline.

This backend is not a claim of complete Linux N_TTY parity. In particular,
packet-mode data headers are supported but packet control notifications and
modem controls are not fully emulated. Poll/epoll hangup behavior for guest PTY
masters that outlive their last slave also needs further compatibility work.
The public console API reports drained output as EIO after the last slave closes.

## Native descriptor lifetime

A later report showed `cd: ... No file descriptors available`. With 64 native
FD slots spare, the simulator reproduced the same `cd` failure and exit code 2.
The filesystem retained four complete lookup histories, cwd retained another
history, and filesystem copies duplicated an otherwise unused root FD.

Ordinary path walks now release each previous directory as they advance.
BENEATH/IN_ROOT walks retain the ancestry required for race revalidation.
Each mount directly owns its compact attachment from creation, including
recursive bind copies; the old lookup cache and anchor synchronization/recovery
machinery have been removed. Cwd owns its leaf descriptor, and the namespace's
root source owns the root descriptor.

`arm64-descriptor-budget` reads a file 120 directories deep with eight native
FD slots spare, then checks repeated cwd/file operations, renamed attachments,
recursive binds, namespace copies and detach. `arm64-terminal-descriptor-budget`
runs shell cd and BusyBox list/date/pipelines in a deep workspace with 64 slots spare.
The same lookup probe's sampled open-FD count fell from 55 to 7.

The final source passed 51/51 host CTest cases and both iOS arm64 library builds.
Native and permission-denied software terminal modes each passed five simulator
create/close cycles and 50 command sequences with 64 slots spare, returning to
8–9 open host descriptors after close. Coreutils commands, which load additional
shared libraries in this rootfs, passed with a separate 128-slot budget. These
injected limits are targeted resource-pressure tests, not affected-device logs.
Evidence is retained under
`$CODEX_HOME/visualizations/2026/11/07/01a1b5e9-91d2-7e33-90b8-39c112696f0c/ios-terminal-diagnostic/fd-2026-11-08/`.

## Verification

- `arm64-terminal-discipline`: canonical/raw bytes, line boundaries, editing,
  UTF-8 erase, EOF, raw-mode transitions, echo, XON/XOFF, binary output,
  bounded backpressure, Ctrl-C with a full output queue, foreground signals,
  resize and VMIN/VTIME.
- `arm64-terminal-syscalls`: guest `/dev/ptmx` and `/dev/pts`, scalar/vector I/O,
  EOF, FIONREAD, sendfile output processing, termios, flow and slave hangup.
- `arm64-terminal-sandbox`: Apple sandbox denies `/dev/ptmx` access and host
  fork. The fixture verifies that native allocation actually fails
  with EPERM/EACCES and that the manager selects the software backend, then
  exercises the guest syscall paths under the same restriction.
- `arm64-terminal-shell-sandbox` (requires `OPERIT_ARM64_ROOTFS`): additionally
  denies executable mappings of the downloaded rootfs and runs Alpine `/bin/sh
  -i`, external commands, pipes, file/symlink operations, resize, typed Ctrl-C,
  API foreground interruption, continued shell use and exit status 7.
- The final isolated host build passed **47/47 CTest cases**, including Alpine,
  filesystem, namespace, network, io_uring, the new terminal syscall fixture and
  restricted-host terminal tests. The snapshot uses commit `dd390f256` as its
  baseline plus the terminal changes; concurrent execution-engine edits in the
  shared workspace are outside this result. Source SHA-256 values and the full
  CTest log are retained in the machine evidence directory.
- An iPhone 18 Pro / iOS 27.0 simulator diagnostic app links the production
  runtime, `Arm64TerminalBridge.mm` and `Arm64RootFilesystem.mm`. Both native
  allocation and a test-only EPERM injection passed terminal creation, command
  execution, host/guest file access, old snapshot restoration and persisted
  rootfs checks. The injected run recorded two denied native allocations.
  Injection is confined to the temporary diagnostic executable.

Both iOS device arm64 and Simulator arm64 static-library builds passed from
that same frozen source snapshot. Evidence is retained at
`$CODEX_HOME/visualizations/2026/11/07/01a1b5e9-91d2-7e33-90b8-39c112696f0c/ios-terminal-diagnostic/pty-2026-11-08/`:
`pty-final-ctest.log`, `pty-fix-source-manifest.json`,
`report-v5-{native,software}-pty.json`, `host-sandbox-audit.json` and
`validation-summary.json`.

The desktop sandbox is a targeted denial regression, not the full iOS device
sandbox. The simulator injection verifies the iOS bridge/fallback integration,
not an observation of the user's actual failing syscall. A signed device build
still needs verification on the affected device.

## Other host dependencies found

| Area | Current behavior / evidence | Status |
| --- | --- | --- |
| Directory binds | Authorized directories are opened directly; denied-parent bind regression exists. Automatic app-container paths are rebuilt when restoring old configuration. | Previous fix, covered again by regressions and simulator restore. |
| Linux processes and executables | Guest fork/exec operate through runtime process/image state. Guest backing mappings use read/write protection, not host executable mappings of Linux code. Restricted Shell tests exercise external commands and pipelines. | Core path works with the tested host restrictions. |
| File ownership | `fchown`/`fchownat` forward guest UID/GID to host filesystem operations; identity defaults to host authority. A guest `chown 0:0` probe returned `Operation not permitted`, exit 1, while guest `id -u` returned 501 on the test host. | Existing design limitation. Package installers that need root ownership need virtual ownership metadata; ignoring EPERM would not implement it. |
| Named/abstract Unix sockets | `UnixNamespace` places aliases under host TMPDIR, then builds a native `sockaddr_un`. A Python AF_UNIX bind inside the runtime succeeded with normal TMPDIR and returned EINVAL with an intentionally long container-style TMPDIR. | Reproduced path-length compatibility issue; independent of PTY allocation. |
| Temporary storage / shared mappings | Backing files and mount storage use TMPDIR with `/tmp` fallback. The actual iOS environment's permitted temporary directory remains relevant, especially when configuration/environment changes. | Source reviewed; not an exhaustive device permission test. |
| Device-specific APIs and network policy | Host socket/file operations still obey the app's actual entitlements, sandbox grants and user permissions. A passing desktop network test does not validate every iOS device policy. | Requires targeted device checks for the feature being used. |

The remaining ownership and Unix-socket issues are recorded separately from this
terminal fix. Passing startup/terminal checks must not be described as proving
that all Linux tools have the host permissions they expect.
