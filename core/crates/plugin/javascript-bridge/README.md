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
