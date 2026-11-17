# Public APIs between prerequisite packages

A prerequisite publishes one self-contained TypeScript client in its `.toolpkg`.
Consumers copy that file and import it locally. There is no npm publication,
automatic dependency download, generated client package, or cross-package `require`.

## Provider

Declare the client path relative to the manifest:

```json
{
  "toolpkg_id": "com.operit.example",
  "api_version": "2.0.0",
  "version": "1.0.0",
  "main": "dist/main.js",
  "public_api": "src/api.ts"
}
```

Publish each runtime method explicitly from `registerToolPkg`. Its handler must be
exported from a package module, just like other durable ToolPkg callbacks:

```ts
/** Implements the public echo contract inside the provider's runtime. */
export async function echo(event: ToolPkg.PublicApiEvent<{ text: string }>): Promise<string> {
    if (typeof event.payload.text !== "string") throw new Error("text must be a string");
    return event.payload.text;
}

/** Registers only the methods intended for dependency callers. */
export function registerToolPkg(): void {
    ToolPkg.registerApi({ name: "echo", function: echo });
}
```

The handler receives `payload` and `callerPackage`. The latter is supplied by the
Rust execution engine and cannot be selected through the call's JavaScript arguments.
Providers should validate input and apply any resource-specific access rules they need.
Declaring a dependency does not automatically establish per-record ownership rules.

A client file contains public types and small transport wrappers only:

```ts
export interface EchoRequest { text: string }

/** Calls the provider's explicitly published echo method. */
export function echo(request: EchoRequest): Promise<string> {
    return ToolPkg.callDependency<EchoRequest, string>("com.operit.example", "echo", request);
}
```

Keep this file self-contained. Do not import provider implementation files or
other packages. The loader validates that `public_api` is a non-empty existing
TypeScript file at a safe package-relative path. Self-containment is an authoring
requirement, not a TypeScript parser check performed by the archive loader.

## Consumer

Declare the runtime prerequisite:

```json
{
  "requires": [
    { "id": "com.operit.example", "min_version": "1.0.0" }
  ]
}
```

Copy its declared client file to your own source directory and import it normally:

```ts
import { echo } from "./dependencies/example/api";
const result = await echo({ text: "hello" });
```

The local client provides autocomplete for methods, arguments, and results. Update
that file explicitly when adopting a newer provider API. Keep the manifest version
bounds aligned with the copied client. Both packages must be installed and enabled;
this API does not install or enable dependencies implicitly.

## Runtime boundary

- Only explicitly registered public method names are callable.
- The host checks the caller's `requires`, package enabled states, and dependency version bounds.
- Calls execute asynchronously in the provider's existing main runtime, sharing its state with UI/tools/schedules.
- Requests and responses cross a JSON boundary; functions and live objects are not transferable.
- The existing execution timeout applies (currently 60 seconds for this call path).
- Errors reject the caller's promise and do not select another implementation.
- Registration-time calls are rejected by the public JavaScript API.
- Package-private IPC is not the public API. The native IPC entry point also rejects a target outside its bound package context.
- Direct self-dependency calls are rejected. Public APIs are not a replacement for ordinary intra-package function calls.

The core `ToolPkg` declarations continue to be generated from `js_sdk/toolpkg.rs`.
A provider's `public_api` file remains handwritten and owned by that provider.

## Workflow and v1

`com.operit.workflow` declares `public_api: "src/api.ts"`. Copy that file and use:

```ts
import { workflow } from "./dependencies/workflow/api";
const entry = await workflow.create("Example");
const detail = await workflow.get(entry.id);
```

The provider exposes list/create/get/update/patch/setEnabled/delete/trigger operations.
The client also offers `enable` and `disable` helpers. The same provider serves
`Tools.Workflow` for ToolPkg API `1.0.0` and `1.0.1`; these calls no longer need to
route through tool metadata and `toolCall`.

For `api_version: "1.0.0"` and `"1.0.1"`, the loader automatically includes
`com.operit.workflow >= 0.2.0` in the effective runtime prerequisites. Legacy
archives do not need a new `requires` entry, and their manifest bytes are not
rewritten. Existing stronger minimums and compatible maximums are retained;
conflicting version bounds fail explicitly. This is a fixed v1 contract rule,
not a reaction to a failed call or an inspection of plugin source code.

The resulting requirement participates in the normal dependency state, ordering,
and public-call authorization checks. Installation and activation remain explicit.
V2 packages still declare workflow dependencies in their own `requires`.

The bundled workflow plugin is disabled by default, so enable it before enabling
the dependent package. Existing v1 handwritten declarations remain unchanged.
The v1 adapter preserves positional arguments and forwards graph inputs to the
provider, which converts scalar/reference shorthand and validates mutations before
committing them. Patches preserve unspecified node fields and workflow statistics.

The current workflow engine does not implement Tasker/intent/speech triggers or
non-empty legacy `extract.defaultValue`; those inputs fail explicitly. Historical
workflow databases from the original application are not imported automatically.
Workflows exposed by this API are the workflow plugin's shared stored workflows.
A failed execution rejects `trigger`; it is not reported as a successful request.

## Checks

```powershell
tsc -p plugins/packages/buildin/workflow/tsconfig.json --noEmit
tsc -p core/crates/plugin/sdk/src/compat/v1/tsconfig.json
node --test tools/tests/toolpkg_public_api.test.mjs tools/tests/toolpkg_single_file_clients.test.mjs
```

Node contract tests load current workflow TypeScript sources in memory and do not
write `dist` or archives. Rust manifest and dependency authorization tests are also
included in source; they require a separate Rust test run.

The single-file client check copies each manifest-declared client to an isolated temporary directory and compiles it with only the standard SDK declarations. It rejects imports and re-exports from other files, checks request/response inference for channel and maintenance clients, and verifies copied Bot clients match their provider sources.
