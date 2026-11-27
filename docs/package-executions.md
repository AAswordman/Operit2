# Package execution and bounded waits

AI calls to namespaced tools (`package:tool`, including MCP namespaces) run as
runtime-owned executions. Both `package_proxy` and CLI `proxy` resolve their
target through the existing permission and hook pipeline. Native built-in tools
retain their ordinary execution contract. Direct SDK calls inside a package keep
their awaited final-result semantics so dependent operations remain ordered.

Every AI package invocation retains an internal opaque `execution_id`. The model waits
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

When a call completes without intermediate output, `package_proxy` and direct
package calls return the original business result, preserving its UI renderer
and failed-tool status. An ordinary failure reports its error once. Package
return values are decoded at the JavaScript JSON boundary, so objects and arrays
remain structured instead of becoming JSON-encoded strings. JSON-looking strings
remain strings, and package-owned keys (including `__type`) are preserved.

Running calls return a flat receipt, for example:

```json
{"execution_id":"package_...","tool_name":"super_admin:terminal","status":"running"}
```

Receipts with intermediate output also include `output`. A completed receipt
includes the business `result` directly, or `error` on failure. Only invocations
with multiple final results use `results`, preserving their names and success
flags. Native result serialization tags are omitted from receipt business data.
The `executions` envelope, `cursor`, `timed_out`, empty output arrays, null errors
and zero counters are not sent to the model. Execution ownership, retained
results, output offsets and delivery progress remain in the runtime.

`package_wait` accepts optional `execution_id` and `timeout_ms` (default 20000,
range 0–20000). With an ID it reads that exact execution, including an already
completed result. Without an ID it snapshots existing executions in the same
authenticated configuration owner and chat, including completed results not yet
delivered. Later invocations do not join an outstanding wait. An empty snapshot
returns `{"status":"idle"}` immediately. CLI models call it through `proxy` with
`tool_name: "package_wait"`.

Wait receipts retain `execution_id`, `tool_name` and `status` so the model can
associate a result with its original operation, even when waiting without an ID.
An observed execution failure is reported inside the receipt; the wait itself
only fails when observation cannot be performed.

The CLI TUI renders `package_wait` as an ordinary tool row (`→`) for direct
calls and calls through either proxy, including merged call/result rows. Its
built-in symbols use an exhaustive match on the canonical `BuiltinToolName`
catalog: adding a built-in requires assigning its TUI symbol at compile time.
The CLI public `search` tool uses the search symbol (`⌕`).

Wait returns on new intermediate output, completion of any selected invocation,
or expiry of its single total budget. Each response collects one ready execution;
`pending_execution_count` reports how many selected executions still need reads,
and is included only when another execution besides the returned one needs a read.
This keeps unrelated completed results from being merged into an oversized tool
response. Each output entry has a monotonically increasing `sequence`, UTF-8 byte
`offset`, textual `value` and `complete` flag. Large intermediate events are
split into Unicode-safe fragments under a 20 KB serialized JSON output budget;
concatenate fragments with the same sequence before interpreting structured JSON
text. `has_more_output: true` means another wait can immediately read retained
fragments; the field is omitted when there are none. A running status,
`has_more_output`, or a nonzero `pending_execution_count` calls for more reads.
Subsequent reads consume
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

The Super Admin AI package exposes `terminal`, `terminal_open`, `list_terminals`, `get_screen`,
`input` and `close`. `terminal_wait` and terminal-specific foreground timeout and
background arguments are removed. Proxy `timeout_ms: 0` provides immediate
background execution through the same execution manager as every package tool.

`Tools.System.terminal.runStream({sessionName, profileId?, terminalType?, command})`
reserves an available named session and streams until actual command completion.
It has no foreground deadline. The start event contains `sessionId` for input,
screen and close; chunks carry live output; the completion event carries the
actual final receipt. Super Admin projects start and chunk events without their
submitted command or internal metadata, forwards them to its package execution
and retains its function and session lease until completion or failure. Closing an output
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
