# operit-js-bridge

`operit-js-bridge` owns the concrete JavaScript engine integration for Operit2.
Stable JavaScript package APIs, ToolPkg registration APIs, Compose DSL scripts,
and Rust execution contracts live in `operit-plugin-sdk`.

## Responsibilities

- Provide the concrete `JsEngine` implementation.
- Execute ToolPkg JavaScript hooks and Compose DSL scripts.
- Enforce request-scoped native QuickJS deadlines, including synchronous script execution.
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
injects `QuickJsExecutionProvider` into the tool system, while `operit-tools`
implements `JsExecutionHost` with the real tool and package logic.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.

## Prepared Page Engines

Creating `QuickJsExecutionProvider` schedules preparation of two pristine engines through
`HostRuntimeTaskSchedulerHost`. Preparation creates the Host-owned runtime, registers native
interfaces and executes the common SDK bootstrap without binding a package or executing its code.
The embedding application must install its JavaScript runtime Host and task scheduler first.

Page acquisition uses `JsExecutionProvider::acquire_toolpkg_execution_engine` through
`ToolPkgExecutionEngineFactory::acquireToolPkgExecutionEngine`. It asynchronously awaits a ready
engine, authenticates its real package owner and installs a single immutable context on the Host
executor. IPC and dependency callbacks read that same bound context. Each consumption immediately
schedules preparation to restore two ready or currently preparing engines. Burst requests queue
in order; preparation and scheduling failures are returned explicitly.

A page's final lease destroys its used engine. Used engines never reenter the prepared pool.
Cancellation releases unclaimed engines, and container/factory revocation prevents engines acquired
under an old generation from being published. The explicit synchronous creation APIs remain for
main-runtime and registration callers.

Successful engine preparation, acquisition, resource reads, compilation and IPC calls do not emit
routine timing logs. Engine preparation failures and IPC failures retain error diagnostics. IPC timing
is disabled by `TOOLPKG_IPC_TIMING_ENABLED` in `ToolJsRuntime.rs`. Flutter page timing is disabled by
default and can be enabled explicitly with `--dart-define=OPERIT_PROFILE_PLUGIN_UI=true`.
