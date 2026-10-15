# Device tools (ToolPkg API v2)

`Tools.device(nodeId)` selects the device that executes a tool. Core namespaces
(`Files`, `Net`, `System`, `SoftwareSettings`, `Chat`, `Memory`) retain the
overloads and return types of local `Tools`. The device view also exposes
`Edge.execute(interfaceInfo, args?)` and `Io.execute(interfaceInfo, args?)` for
firmware actions; the global `Tools.Edge` and `Tools.Io` APIs have been removed.
The interface requires `manifest.api_version` in the v2 family. It is absent from
the v1 declarations, and v1 execution contexts reject it before submitting work.

```ts
const desktop = Tools.device(desktopNodeId);
const file = await desktop.Files.read('/app/data/example.txt');
const sameFile = await desktop.Files.read({ path: '/app/data/example.txt' });

// Local calls retain their existing behavior.
const localFile = await Tools.Files.read('/app/data/example.txt');
```

The selected Core executes its own registered tool with its own filesystem,
terminal, installed plugins and host capabilities. File paths and session IDs
belong to that device. The calling device does not need to install a plugin whose
tool is executed remotely; the selected device must have that plugin available
and enabled. Explicit selection of the local node ID uses the same device-tool
entrypoint and permission checks.

The node must belong to the current device Space and be reachable through the
existing paired Link routes, including multi-hop routes through relay nodes.
Both the authenticated caller node and the selected Core require the `runtime.execute` capability. The receiver exposes public tools
only and retains parameter validation, permission preflight and lifecycle hooks.
The caller's interception hooks also run before an execution is submitted.
Internal tools, ToolPkg IPC handlers and prerequisite public APIs are not exported
as device tools. Edge firmware does not provide the generic Core tool service;
its declared hardware actions use `Tools.device(nodeId).Edge` and
`Tools.device(nodeId).Io`.

## Plugin tools and discovery

```ts
const desktop = Tools.device(desktopNodeId);
const catalog = await desktop.catalog(); // { tools: [...] }, including parameter schemas
const result = await desktop.call('package_name:tool_name', { text: 'hello' });
```

Qualified plugin names remain `package_name:tool_name`. Device selection is
transport metadata and never becomes a tool parameter. Missing or disabled tools,
permission denials, unsupported hosts and disconnected devices reject the Promise
with the reported error. A remote failure does not run the tool locally.
Structured results, literal strings containing JSON, and binary results keep the
existing plugin bridge representation.

## Deadlines and cancellation

```ts
const desktop = Tools.device(desktopNodeId, { timeoutMs: 30_000 });
const pending = desktop.Files.read('/app/data/large.txt');

// Cancels pending calls from this device view. Other views are unaffected,
// and the view can be used for new calls after cancellation.
desktop.cancel();
await pending; // rejects when cancelled
```

An optional `signal` accepts an AbortSignal-compatible object when one is available
in the embedding environment. `cancel()` works without browser globals.
The default deadline is 120,000 ms; `timeoutMs` must be an integer from 1 through
86,400,000. It covers opening the route, receiver permission checks and execution.
For Core tools, cancellation and timeout close the execution's Link watch; the
receiver drops its pending future and returns the executor to its registry.
Hardware calls use the same deadline and cancellation options. Once a firmware
action has been submitted, Core drains its Link acknowledgment even if the caller
stops waiting; cancellation does not stop that action. A completed side effect
cannot be undone by cancelling, and underlying synchronous host work may finish
before cancellation can be observed. Calls are not automatically resubmitted
following an execution error or missing final result.
