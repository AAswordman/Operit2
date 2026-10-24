# ToolPkg installation, storage, and automatic routes

A manifest can fix its installation scope with `"installation_scope": "device"` or `"installation_scope": "space"`. The parser, package registration, bundled import, marketplace install, and scope moves enforce this declaration. A fixed-scope package cannot be installed or moved into the other scope. The package UI shows its required scope and hides scope moves.

Older binaries registered every built-in package in device scope regardless of its manifest. When an upgrade leaves both a legacy device record and the required space record for the same built-in package, the package scan merges their enabled lists, members, subpackage states, and order into the space owner, then removes the stale device record. Only unambiguous built-in duplicates are healed; non-built-in owners and every other duplicate still fail with an ownership conflict instead of being merged.

Device installations own local data. Space installations own synchronized data. Storage synchronization follows the registered directory owner; `Storage.open` does not accept a synchronization switch. Goal and Plan both require a space installation and keep their state in their own databases, independently of chat workspaces. Workspace records are not migrated.

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
