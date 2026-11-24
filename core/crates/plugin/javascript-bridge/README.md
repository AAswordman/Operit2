# operit-js-bridge

`operit-js-bridge` owns the concrete JavaScript engine integration for Operit2.
Stable JavaScript package APIs, ToolPkg registration APIs, Compose DSL scripts,
and Rust execution contracts live in `operit-plugin-sdk`.

## Responsibilities

- Provide the concrete `JsEngine` implementation.
- Execute ToolPkg JavaScript hooks and Compose DSL scripts.
- Enforce request-scoped native JavaScript deadlines, including synchronous script execution.
- Install SDK-owned JavaScript package, ToolPkg, and Compose DSL definitions.
- Load engine-specific embedded JavaScript libraries and native bridge scripts.
- Expose JS execution through traits defined by `operit-plugin-sdk`.
- Keep JS-engine-specific code out of `operit-runtime` and `operit-tools`.

## Main Modules

- `src/javascript/JsEngine.rs`: concrete JavaScript engine implementation.
- `src/javascript/JsExecutionProvider.rs`: SDK `JsExecutionProvider` implementation.
- `src/javascript/PreparedEnginePool.rs`: bounded background preparation and cancellable acquisition.
- `src/javascript/JsToolManager.rs`: JavaScript package execution coordinator.
- `src/javascript/JsToolPkgExecutionContext.rs`: ToolPkg execution context.
- `src/javascript/JsLibraries.rs`: assembles SDK definitions and engine-specific libraries.
- `src/javascript/JsJavaBridge.rs` and delegate modules: host/runtime bridge
  calls exposed to JavaScript.
- `src/javascript/*.script.js`: engine-specific embedded JavaScript libraries.

## Boundary

This crate implements JS execution. `operit-plugin-sdk` owns the public package
models, fixed JavaScript APIs, DSL, ToolPkg package services, and execution
contracts. This crate does not depend on `operit-tools`. The top-level runtime
injects `HostJsExecutionProvider` into the tool system, while `operit-tools`
implements `JsExecutionHost` with the real tool and package logic.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.

## Call Ownership

Every asynchronous handoff selects the exact call's complete `JsCallContext`, including its Host,
module resources, environment and cancellation budget. Pending and detached work retain their own
context until cleanup, and nested execution restores the enclosing realm's capabilities.
See [JavaScript call context ownership](../../../../docs/js-call-context.md) for the lifecycle,
historical cause and regression coverage.

Each call also owns its output channel: ordinary scripts deliver text, including JSON
serialization of `emit(object)`, while Compose delivers transferred binary frames.
Native bindings reject channel mismatches before invoking a callback. A reported
`{success:false,message:""}` remains a runtime failure; malformed error envelopes
complete as protocol failures. Neither path coerces binary data into text.

## Prepared Page Engines

Native execution uses the four existing affine workers, with one shared V8 isolate per worker.
Each plugin owns a separate context, host bindings and microtask queue; cancellation targets only
its active execution. Preparing a page creates a context rather than another isolate.

Creating `HostJsExecutionProvider` schedules preparation of four pristine engines through
`HostRuntimeTaskSchedulerHost`. Preparation creates the Host-owned runtime, registers native
interfaces and executes the common SDK bootstrap without binding a package or executing its code.
The embedding application must install its JavaScript runtime Host and task scheduler first.

Page acquisition uses `JsExecutionProvider::acquire_toolpkg_execution_engine` through
`ToolPkgExecutionEngineFactory::acquireToolPkgExecutionEngine`. It asynchronously awaits a ready
engine, authenticates its real package owner and installs a single immutable context on the Host
executor. IPC and dependency callbacks read that same bound context. Each consumption immediately
schedules preparation to restore four ready or currently preparing engines. Burst requests queue
in order; preparation and scheduling failures are returned explicitly.

A page's final lease destroys its used engine. Used engines never reenter the prepared pool.
Cancellation releases unclaimed engines, and container/factory revocation prevents engines acquired
under an old generation from being published. The explicit synchronous creation APIs remain for
main-runtime and registration callers.

Successful engine preparation, acquisition, resource reads, compilation and IPC calls do not emit
routine timing logs. Engine preparation failures and IPC failures retain error diagnostics. IPC timing
is disabled by `TOOLPKG_IPC_TIMING_ENABLED` in `ToolJsRuntime.rs`. Flutter page timing is disabled by
default and can be enabled explicitly with `--dart-define=OPERIT_PROFILE_PLUGIN_UI=true`.

## Asynchronous Compose Preparation

Page requests prepare their immutable source snapshot before entering the page's affine execution
queue. Four isolated compiler contexts use the Host's four compilation workers, separate from the
four plugin execution workers. Identical source/identity requests share one in-flight result;
independent misses select the least loaded compiler. A short shared LRU lookup serves warm results
without entering any compiler queue. The cache retains at most 128 entries and 16 MiB, including
exact parser errors; mounted pages retain their own immutable results across cache eviction.

Dependency discovery uses the parsed CommonJS call structure. Literal imports prepare their
transitive resource graph with the loader's path normalization and candidate ordering. Computed or
aliased loaders declare the whole resource snapshot as their preparation scope. JSON stays on its
existing loader path. Preparation does not evaluate plugin code, and an unused module's syntax
error is reported only when that exact module is required. Dependency analysis is cached with the
lowered source and runs on compilation workers.

Synchronous `require()` reads only the owning call's prepared results. It never waits on another
runtime or starts compilation. Suspended actions retain their original prepared snapshot through
page refreshes. Shared preparation outlives an individual cancelled waiter; dropped Host tasks
complete all remaining waiters with an explicit error. Compiler initialization uses Host blocking
capacity and asynchronous completion, without platform branches in shared code.

The pool permits parallel independent pages and asynchronous Host work. A page's mutable realm and
Flutter rendering still preserve their own ordering; synchronous JavaScript running on the same
affine worker shares that worker's CPU time. Web Hosts retain their existing browser execution
model; four compiler contexts do not imply four browser worker threads.

Focused validation: `cargo test -p operit-js-bridge JsComposeCompiler` covers cache ownership,
coalescing, cancellation, dependency scope and blocked-worker isolation; `PreparedEnginePool`
tests cover the four-engine ready target and shutdown. The JS bridge and Compose session suites
cover CommonJS loading, source replacement, live state and callback ownership.
