# operit-js-bridge

`operit-js-bridge` owns JavaScript execution for Operit2. It contains the JS
engine integration, embedded JavaScript assets, Java bridge code, Compose DSL
bridge code, and ToolPkg JS runtime.

## Responsibilities

- Provide the concrete `JsEngine` implementation.
- Execute ToolPkg JavaScript hooks and Compose DSL scripts.
- Load embedded JavaScript libraries and runtime bridge scripts.
- Expose JS execution through the trait defined by `operit-tools`.
- Keep JS-engine-specific code out of `operit-runtime` and `operit-tools`.

## Main Modules

- `src/javascript/JsEngine.rs`: concrete JavaScript engine implementation.
- `src/javascript/JsToolManager.rs`: ToolPkg JS tool execution support.
- `src/javascript/JsToolPkgExecutionContext.rs`: ToolPkg execution context.
- `src/javascript/JsComposeDslBridge.rs`: Compose DSL bridge support.
- `src/javascript/JsExecutionScriptBuilder.rs`: execution script assembly.
- `src/javascript/JsJavaBridge.rs` and delegate modules: host/runtime bridge
  calls exposed to JavaScript.
- `src/javascript/*.script.js`: embedded JS runtime libraries and bridge
  scripts.

## Boundary

This crate implements JS execution; tool registration and package ownership
remain in `operit-tools`. Runtime startup creates and wires the engine into
ToolPkg execution.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.
