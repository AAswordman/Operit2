# Market plugin compatibility audit — 2026-10-22

Seven additional marketplace archives were downloaded and exercised without
changing their contents. All seven omit `api_version` and therefore select API
1.0.0. Tests use the current workspace's SDK generator, version dispatcher,
structured tool transport, module registration, OkHttp implementation, and
Compose compiler/runtime. The results below describe tested paths, not complete
end-to-end compatibility on every platform.

## Results

All seven entry points registered, all eight declared subpackages loaded, and
all nine unique registered screens completed their initial render after the
shared bootstrap gained the `global` alias.

| Marketplace issue | Package | Archive version | Exercised behavior and result |
| --- | --- | --- | --- |
| 289 | `com.operit.sidebar_musicfree` | 0.4.6 | Two screens render. The original fresh-install branch now accepts `terminal.execStreaming`, receives live progress before the fixture process exits, and propagates its nonzero exit code. Actual dependency installation and service startup remain untested. |
| 285 | `com.operit.wechat_ilink_bot` | 2.3.0 | Settings screen renders and its save button succeeds. Original set/get functions retain all four media paths across a fresh VM using real temporary JSON files. The marketplace advertises 2.3.2; the downloaded manifest says 2.3.0. |
| 281 | `com.operit.openclaw` | 0.2.0 | Settings screen renders and its save button succeeds. Original configuration persists across a fresh VM; a loopback health check succeeds. Starting without a logged-in account is correctly rejected. |
| 254 | `com.operit.memory_system` | 1.0.0 | Three screens render. Original todo save/toggle/load functions persist data across a fresh VM and reject an invalid index. |
| 253 | `com.operit.telegram_bridge_market` | 0.1.0 | Settings screen renders. Original JS calls the archive's original Python HTTP handler: queue query, single-event consumption, retained history, persistence across a Python process restart, clear, and empty queue pass. |
| 245 | `com.operit.ai_workflow_suite` | 0.1.0 | Entry and subpackage load. Inline execution reports `completed` for an unknown step without executing host tools; its source uses random results. File-path execution fails when the plugin parses the structured `Files.read` result directly. These observations are plugin failures, not successful workflow execution. |
| 282 | `com.raineiris.xhsreader` | 1.0.0 | Settings screen renders and the subpackage loads. Real note retrieval and vision-model calls were not exercised. |

## Shared global alias

OpenClaw and WeChat iLink initially failed to render with `global is not defined`.
Both use that object to store timer handles. The shared engine bootstrap now
sets `global = globalThis`, alongside its existing `window` alias, for v1 and v2.
Both declaration trees describe the alias. The v1 provenance records it as a
local extension rather than attributing it to the upstream declaration snapshot.

Both affected screens were retested through the production Compose action
protocol. Their save buttons call the original package functions and persist
configuration in temporary files. OpenClaw creates a refresh timer handle;
WeChat initializes its QR polling handle. These checks use recorded timers and
in-process package dispatch, so they do not establish native background polling
or cross-runtime IPC correctness.

## Terminal streaming

MusicFree uses the declared v1 `Tools.System.terminal.execStreaming` API with an
`onIntermediateResult` function. It initially failed with `Tool request
contains a non-JSON value`. The shared terminal SDK now retains callbacks in
JavaScript and consumes live native observations; v1 adapts to that same stream
in its existing runtime adapter. The new v2 `execStream(request)` exposes an
async iterable of start/chunk/completed events.

The unchanged MusicFree runtime was retested through its absent-git branch. A
host fixture substitutes a harmless local Bash process for the installation
command: print a line, wait 0.2 seconds, print another line, exit 37. The original
plugin receives its first installation progress before that process exits,
reports `Git 安装失败 (exit 37)`, and closes the observation. No callback crosses
the JSON boundary. This verifies stream admission, progress, and failure
propagation; it does not establish successful dependency installation or service
startup.

Separate tests use the real native PTY host to verify output before completion,
nonzero exit status, persistent working directory/environment, Unicode split
across frames, cancel/timeout recovery, pending-pull disposal, and bounded output.
Two iOS simulator XCTests exercise the actual iSH streaming bridge: the first
line arrives before the delayed final line and completion, and canceling a
silent command retains a usable session.
Linux/Windows cross-compilation is not established on this Mac because the
required cross C compilers are absent; Android/OHOS device execution and browser
V86 streaming are not established by these tests.

## Remaining findings

AI Workflow Suite's file-path mode calls `JSON.parse(await Tools.Files.read(path))`.
The v1 contract returns a file-result object containing `.content`, so this
fails with `"[object Object]" is not valid JSON`. The adapter's structured return
value should not change to accommodate this incorrect plugin usage.

Static inspection also finds undeclared calls in Memory System
(`Tools.Files.makeDirectory`, clipboard aliases) and guarded optional calls in
OpenClaw (`registerToolPkgCleanupHook`, `deleteEnvironmentVariable`). These do
not by themselves prove that every flow in either plugin fails. In particular,
Memory System catches the missing directory helper, and its tested writes work
because the real file host already creates parent directories.

## Verification and limits

Terminal validation in this workspace: 46 JavaScript tests passed, two optional
Ferry tests skipped, five real native PTY tests passed, and two iSH simulator
tests passed (xcresult confirms two executed tests, zero failures). TypeScript
samples for the v1 callback and v2 iterable/callback APIs compile. The unsigned
simulator app also logs existing Keychain entitlement errors during unrelated
preference initialization; terminal test success is not a clean whole-app
startup or crash-regression result.

The native-terminal crate's separate existing four-test suite is not all green
on this Mac: two tests pass and two fail (the macOS Bash deprecation banner in
screen output, and command echo normalization from the long working directory).
An isolated copy of `HEAD` run from the identical working directory reproduces
both failures unchanged. They are recorded rather than folded into the streaming
fix, and the five passing streaming tests do not imply that suite passed.

The shared-bootstrap regression exercises object identity, property writes,
callback reads, and deletion for API 1.0.0, 1.0.1, and 2.0.0. The bridge, API
compatibility, and v1 runtime test suites pass, with the two optional Ferry tests
skipped when no archive path is configured. Separate TypeScript samples using
`global.setInterval`, `global.clearInterval`, and `global.console` compile against
both declaration trees without diagnostics.

The audit's native file host is a temporary-directory fixture. HTTP requests use
real loopback connections; Telegram runs the archive's original Python server
with seeded events and does not poll Telegram. Environment variables and IPC
use in-process fixtures. Real WeChat/Telegram authentication and sending,
remote SSH, vector-memory services, native timer scheduling, iOS/iSH dependency
installation, Xiaohongshu retrieval, and external model calls remain untested.
No marketplace archive was edited. Native terminal evidence is limited to the
PTY tests and iOS simulator bridge test described above; no physical device
end-to-end plugin run is claimed.

Audit scripts, downloaded archives, and machine-readable results for this session
are under `/tmp/operit-market-audit-20261022/`. These temporary artifacts are not
required by the application and are not committed to the repository.
