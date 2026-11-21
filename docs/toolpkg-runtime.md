# ToolPkg runtime

Tool executors, including MCP servers, receive business arguments from the model or the calling script. The common dispatcher carries caller name, chat, participant, workspace and authenticated owner in `ToolInvocationContext`; it does not append those fields to tool parameters or proxy envelopes. A proxy accepts only `tool_name` and `params` and preserves the target's argument values.

The JavaScript bootstrap detaches temporary package runtime fields before evaluating the package module or invoking its exported function. Package tools therefore receive ordinary business arguments, such as `{ "text": "性能" }`, without environment, API version, package identity or caller metadata. Read caller information through `getChatId()`, `getCallerName()` and `getCallerParticipantId()`, and package configuration through the existing SDK helpers. These values retain their owning call across asynchronous continuations. Passing the business input directly to `Tools.UI.clickElement(input)` or `toolCall("server:method", input)` does not forward runtime metadata. The v1 adapter also inherits caller context through the native invocation instead of inserting private fields into memory-tool arguments.

Declared hook event fields, including `toolPkgId` and `containerPackageName`, remain part of the typed hook event. Nested user data is preserved verbatim; detachment targets exact runtime field names at the call boundary and does not remove arbitrary keys by prefix.

A manifest can fix its installation scope with `"installation_scope": "device"` or `"installation_scope": "space"`. The parser, package registration, bundled import, marketplace install, and scope moves enforce this declaration. A fixed-scope package cannot be installed or moved into the other scope. The package UI shows its required scope and hides scope moves.

Device installations own local data. Space installations own synchronized data. Storage synchronization follows the registered directory owner; `Storage.open` does not accept a synchronization switch. Goal and Plan both require a space installation and keep their state in their own databases, independently of chat workspaces. Workspace records are not migrated.

Package environment variables belong to the installation owner, including all of its subpackages. The environment editor, JavaScript `getEnv`/`setEnv`, and environment tools use the same owner resolution. Values are stored in `runtime/extensions/{scope}/plugins/configs/{owner}/environment.json` through the existing runtime storage host and file synchronization service. Space installations synchronize these configuration snapshots; device installations keep them local. Moving or deleting an installation moves or deletes its environment with the rest of its configuration. Concurrent changes to the same environment file follow the existing complete-file conflict rules.

Environment tools and preference APIs require an explicit package name. Package JavaScript obtains its owner from the native call context; asynchronous callbacks retain that context, and registration uses the scope fixed before module evaluation. Missing package variables remain absent. The retired node-global preferences file and process environment are not runtime data sources. Existing declared values are imported once into each installation's environment before registration evaluates the main module; Space imports are initial seeds below explicit edits in conflict order. The editor saves only changed keys and preserves unrelated values.

## Chat processing status

`Tools.Chat.agentStatus(chatId)` reads the authoritative execution-state flows of
all live main, floating and detached chat runtimes. The read handle is bound from
the initialized holder before runtime publication. New cores register their
existing shared flows; stopping chat services removes those sources before cores
are rebuilt. Retained workers from stopped cores cannot reintroduce old activity.

Status reads do not acquire `ChatRuntimeHolder`, so lifecycle hooks, approval
notifications and IPC can query status while navigation or synchronization holds
that mutex. Streaming and nonterminal work such as summarization count as
processing; idle, completed and failed states without streaming do not. Missing
chats are idle. Unbound state and poisoned registry locks remain explicit errors.

## Implemented methods and decorators

A routed method has a normal local implementation. Mark it with the standard TypeScript method decorator and call it normally:

```ts
class GoalService {
  /** Reads the current record on the Binding-selected runtime. */
  @ToolPkg.route({ binding: 0 })
  static async read(chatId: string): Promise<Storage.Value | null> {
    const db = await Tools.Storage.objects.open({
      path: ToolPkg.getSpaceDataDir() + "/goals.sqlite",
    });
    try {
      const entry = await db.collection<Storage.Value>("goals").get(chatId);
      return entry === null ? null : entry.value;
    } finally {
      await db.close();
    }
  }
}

const goal = await GoalService.read(chatId);
```

`binding` is the zero-based argument position containing the existing Core Binding key. The decorator derives its method identity from the declaring class and method, and serializes the positional arguments automatically. Callers do not declare remote targets, transport methods, or Rust parameter mappings. Local invocations execute the original method with its receiver intact. Remote invocations execute the registered original body on the destination package's main runtime.

The Rust host obtains package identity from the authenticated engine, adds it to the internal argument envelope, and uses the existing annotation-generated route catalog, Binding resolution, permission checks, and CoreNodeRouter. Only space-installed packages can expose remote implementations. Destination execution also requires an enabled package and a decorated method with a matching Binding argument. There is no separate plugin network protocol.

Use named classes with public async methods or async generators. Static services are registered when their class is initialized; instance services are registered when their owning instance is initialized. Each service method identity has one registered implementation per engine. Its service module must be imported by the package's main entry and each consuming UI entry. Configure TypeScript for standard decorators, without `experimentalDecorators` or `emitDecoratorMetadata`.

## Async streams and input

`Stream` and `ReverseStream` are the highest stream abstractions; Watch and Push carry their output and input below that layer. An `async *` implementation automatically exposes an output stream through Core's existing embedded stream descriptors and Watch abstraction. Consumers receive ordinary values:

```ts
class GoalService {
  /** Streams this chat goal with collector-owned cancellation. */
  @ToolPkg.route({ binding: 0 })
  static async *watch(chatId: string, signal?: AbortSignal) {
    if (!signal) throw new Error("Collector signal is required");
    const db = await Tools.Storage.objects.open({
      path: ToolPkg.getSpaceDataDir() + "/goals.sqlite",
    });
    try {
      for await (const entry of db.collection("goals").watch(chatId, { signal })) {
        yield entry;
      }
    } finally {
      await db.close();
    }
  }
}

for await (const goal of GoalService.watch(chatId)) {
  render(goal);
}
```

The final optional `AbortSignal` of a generator implementation is supplied by its collector. Pass it into operations that wait for changes. Calling the collector's `return()` aborts pending waits before returning the generator, so cancellation does not queue behind an outstanding `next()`. Consumers use `for await`, `next`, and `return` directly. The Rust stream owner retains the package engine lease until source completion or cancellation, not merely until the opening IPC returns.

An async method accepting one `AsyncIterable` input maps to the existing typed Rust `ReverseStream`; the existing Link Push transport carries it remotely. The existing bounded stream preserves input ordering and backpressure; the JS input adapter acknowledges each forwarded item when the implementation accepts it. Input consumers return `Promise<void>` and use ordinary `for await` syntax. Producer errors abort their session; consumer errors propagate to the producer. Local input consumers execute their original method directly.

## Storage publication and UI ownership

`database.collection("goals").watch(chatId, { signal })` emits the current record and subsequent changes to that exact key. DataStore uses `store.watch(key, { signal })`. Rust enforces the authenticated database handle and collection; initial reads and version comparison happen inside the storage layer, without replaying history or requiring plugin identity filters. The collector owns an independent handle; returning it or aborting its signal closes that handle and releases the pending read. Both device-local and synchronized Space storage support this API.

Compose `ctx.useEffect(key, setup, dependencies)` owns subscriptions and timers. Setup returns a cleanup callback. Cleanup runs when dependencies change, the owning branch disappears, the program is replaced, or the context is replaced. Engine release also cancels pending native route/storage futures and drops their owned handles.

Goal's UI consumes `GoalService.watch(chatId)`. Its clock exists only while the displayed goal is active and never polls storage. Plan's UI consumes `PlanModeShared.watchStarted(chatId, content)` and replaces its collector when the chat, plan content, or readiness changes. Goal records, input flags, plan mode flags, plan content, and handoff markers live in plugin-owned Object Storage. Plan content and mode removal commit atomically; handoff markers use an explicit absence precondition.

All public route, cancellation, storage facade, and Compose lifecycle declarations originate in the Rust SDK. Regenerate `plugins/types/*.d.ts` using `operit-plugin-sdk-codegen`; do not edit those generated files by hand.

## Text file write results

`Tools.Files.write(path, content, append)` returns `FileApplyResultData`, identical
to `toolCall("write_file", ...)`: `operation` contains the `FileOperationData`
receipt, alongside `aiDiffInstructions` and optional `diffContent`. Read
`result.operation.successful` and `result.operation.details`, not top-level receipt
fields. `writeBinary`, `mkdir`, and `move` return `FileOperationData` directly.
The existing v1 adapter retains its declared flat write receipt; v2 does not
unwrap or accept alternate result shapes.

## Registration and configuration directories

`ToolPkg.getConfigDir()` is available at module top level, inside
`registerToolPkg()`, and during execution. Archive loading fixes the package ID
and selected `ToolPkgConfigScope` before evaluating JavaScript. Registration
binds that Rust-owned snapshot for the operation and clears it on return.

Registration resolves configuration through this context without acquiring the
package-manager mutex or requiring an installation record that has not yet been
committed. Runtime execution resolves container/subpackage ownership from
`ExtensionStore`. Both paths share `configPathForScope()` and the filesystem
Host. Named directories stay inside the owner's `namespaces` directory. Invalid
scope, owner mismatch and filesystem failures are errors.

Local imports can replace an existing ordinary external `.toolpkg` with the same
ID. The update keeps its record's scope and source filename, validates a sibling
staging archive, and restores the previous archive and installation settings on
failure. Existing configuration is not deleted or relocated. Installation order,
identity and existing member activation states are retained; new subpackages are
explicitly disabled, including those that declare an enabled default.

The import UI keeps the scope selected by Core. Conflicting installation-scope
declarations and names owned by other packages are rejected. This path does not
replace built-in, marketplace or bundled installations; their existing update
and customization paths remain in use. Updates do not move plugin databases or
call uninstallation. Explicit uninstallation keeps its existing configuration
deletion rules. Archive and settings rollback does not undo side effects performed
by the plugin's registration code. If restoration itself fails, the error is
reported and the backup is retained for recovery.

## Compose page resources

Before rendering, the package manager snapshots UTF-8 resources from the
extracted package cache onto the page-owned JavaScript engine. Render, rerender
and action execution use the structured
`__operitNativeReadToolPkgTextResource` binding to read that snapshot. Module
loading must not reenter the package-manager mutex while its caller owns it.
The page engine owns and releases the snapshot; binary resources use their
materialization API.

Flutter parses returned Compose results on its UI isolate. Page execution and
resource ownership remain in the Rust runtime and Host. The current program,
buffer and event lifecycle contract is maintained in the
[Compose runtime guide](../core/crates/plugin/sdk/src/toolpkg/compose/README.md).

## Compose card constraints

`Card`, `ElevatedCard` and `OutlinedCard` pass their available height directly to
a single content node declaring `fillMaxSize` or `fillMaxHeight`, through either
props or modifiers. This lets a full-height `Column` allocate its remaining
space to a weighted `ScrollArea`, including inside floating windows. Both direct
children and `slots.content` follow this contract, with `contentPadding` deducted
from the available space. Ordinary content retains its natural size; a content
node with positive `weight` retains the card's implicit column flex scope.
Legacy `Row(mainAxisSize: "max")` content also retains direct height constraints
for cross-axis alignment.

## Script deadlines

Ordinary JavaScript calls, workflows, IPC, public APIs, notifications and after
hooks run until completion, caller cancellation or runtime destruction. Active
cancellation can interrupt synchronous loops.

Only `submit_requested` and pre-hooks share a total deadline: 10 seconds by
default, configurable from 1 to 60 seconds in advanced tool settings. Queueing,
synchronous work and asynchronous waits consume that same budget. Each hook
receives the remaining time. Timeout terminates the owning hook, reports an
error and aborts the dependent operation. Its interruption token must not cancel
unrelated calls waiting in the same runtime.

## XML renderer changes

`ToolPkgXmlRenderBridge` commits normalized hook declarations, releases its
mutex, then publishes a changed `StateFlow<i64>` revision. Identical declarations
publish no revision. `ChatServiceCore.xmlRenderRegistryRevisionFlow()` exposes
this committed state through the existing Core proxy.

Mounted XML nodes observe the selected chat runtime's revision, including the
initial snapshot. A changed revision starts a new render generation; stale
responses cannot replace current output. Runtime changes and widget disposal
release the subscription. Existing message content does not need modification.

## Related contracts

- [Plugin storage](plugin-storage.md): preference and object-store ownership.
- [Chat send API](chat-send-api.md): submission and control semantics.
- [Theme templates](theme-v1-contract.md): manifest declarations and validation.
- [Plugin author documentation](../plugins/docs/README.md): public SDK usage.
