# Package execution and bounded waits

AI calls to namespaced tools (`package:tool`, including MCP namespaces) run as
runtime-owned executions. Both `package_proxy` and CLI `proxy` resolve their
target through the existing permission and hook pipeline. Native built-in tools
retain their ordinary execution contract. Direct SDK calls inside a package keep
their awaited final-result semantics so dependent operations remain ordered.

Every AI package invocation receives an opaque `execution_id`. The model waits
up to 20 seconds for completion. A top-level proxy `timeout_ms` accepts integers
from 0 to 20000; 0 returns immediately. This control is separate from business
arguments inside `params`. Expiry returns a running receipt and never cancels or
restarts the invocation. The Host scheduler retains the execution future, engine
lease, authenticated owner, chat, workspace and hook context through completion.
Tool completion notifications and after hooks follow actual execution completion.
Foreground deadlines use the configured platform Host's monotonic clock and
delay interface, independently of package JavaScript and its event loop.

Each model response's tool list owns a separate asynchronous execution sequence.
Within that batch, an operation starts only after its predecessor actually ends;
a foreground timeout does not release its position. A queued package invocation
already has an execution ID, and its foreground wait budget includes queue time.
Its `running` receipt covers both waiting for its turn and executing. Retained
tasks keep their sequence after the batch's foreground results have returned.
Failures and cancelled admissions release their positions without bypassing
unfinished predecessors. Native built-in operations retain their existing await
contract while respecting preceding work in the batch.

Different batches, including batches in the same chat using the same handler,
have independent sequences and can execute concurrently. Individual tool
executors remain concurrent: direct SDK dependencies and JavaScript
`Promise.all` calls do not join the outer AI batch sequence. ToolPkg calls can
overlap asynchronously in their existing shared JavaScript realm, preserving
package state. `package_wait` is an observation primitive outside the operation
sequence, so reading progress never waits for an unfinished predecessor first.

```json
{
  "tool_name": "super_admin:terminal",
  "params": {"command": "long-running-command"},
  "timeout_ms": 20000
}
```

`package_proxy` returns an envelope containing `executions` and `timed_out`.
Each execution contains `execution_id`, `tool_name`, `status` (`running`,
`completed` or `failed`), `output`, `cursor`, `results` and `error`. A running
receipt is a successful acknowledgement; an already failed invocation retains
the ordinary failed-tool status. Final `results` preserve each original tool's
name, success, textual result and error.

`package_wait` accepts optional `execution_id` and `timeout_ms` (default 20000,
range 0–20000). With an ID it reads that exact execution, including an already
completed result. Without an ID it snapshots existing executions in the same
authenticated configuration owner and chat, including completed results not yet
delivered. Later invocations do not join an outstanding wait. An empty snapshot
returns immediately. CLI models call it through `proxy` with
`tool_name: "package_wait"`.

Wait returns on new intermediate output, completion of any selected invocation,
or expiry of its single total budget. Each response collects one ready execution;
`pending_execution_count` reports how many selected executions still need reads.
This keeps unrelated completed results from being merged into an oversized tool
response. Each output entry has a monotonically increasing `sequence`, UTF-8 byte
`offset`, textual `value` and `complete` flag. Large intermediate events are
split into Unicode-safe fragments under a 20 KB serialized JSON output budget;
concatenate fragments with the same sequence before interpreting structured JSON
text. `cursor` is the last fully delivered sequence, and `has_more_output` means
another wait can immediately read retained fragments. Subsequent reads consume
only new output; final results remain available through an explicit ID. Completed
executions disappear from the implicit snapshot only after all their output and
final results have been delivered. Executions and their retained output belong to the current runtime;
they are not persisted across application restarts or moved to another CoreNode.
Reading execution records does not invoke package interception hooks: work was
already admitted by the original call, and waiting must remain available even
while a package's shared JS engine is busy. Native read permission and exact
execution ownership checks still apply. Character tool policies expose this
observation primitive alongside catalog search.

Ordinary JS tools produce intermediate output through `sendIntermediateResult`.
Tools that only return a final value still benefit from the bounded foreground
wait. No additional automatic completion message is injected into a later model
turn: the model receives later results through `package_wait`.

## Terminal integration

The Super Admin AI package exposes `terminal`, `list_terminals`, `get_screen`,
`input` and `close`. `terminal_wait` and terminal-specific foreground timeout and
background arguments are removed. Proxy `timeout_ms: 0` provides immediate
background execution through the same execution manager as every package tool.

`Tools.System.terminal.runStream({sessionName, profileId?, terminalType?, command})`
reserves an available named session and streams until actual command completion.
It has no foreground deadline. The start event contains `sessionId` for input,
screen and close; chunks carry live output; the completion event carries the
actual final receipt. Super Admin forwards start and chunk events to its package
execution and retains its function until the completion event. Closing an output
iterator detaches observation while the session still owns the running command.
The existing SDK `run`, `wait` and `execStream` interfaces retain their contracts.

Slow or suspended tasks may retain resources. Foreground expiry does not imply
execution failure or release its engine. `package_wait` reads Core execution
records independently of the package's JavaScript realm.

Browser JavaScript runs in a dedicated worker. Core awaits command frames through
`Atomics.waitAsync`, including registration and Promise continuations; synchronous
guest callbacks return through the awaiting Rust task's invocation context.
Synchronous transport waits remain confined to the guest and trusted startup
steps. Slow guest code leaves Core timers, execution reads and other realms
available. Browser terminal filesystem requests and Host completion use producer
notifications instead of periodic polling.

Terminal command observation retains output and a producer waker in the shared
Host contract. Platform Hosts own timers and explicit synchronous compatibility
adapters. Native POSIX readers use descriptor readiness, and Windows ConPTY output
uses overlapped pipes and IOCP. Apple ARM64 guest sessions expose blocked syscall
descriptors and their actual deadlines to the native event loop; output
backpressure resumes when the consumer drains it. Intentional SDK delays and
command deadlines remain platform-owned asynchronous timers.
