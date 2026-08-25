# Plugin

Plugin crates own plugin contracts, SDK generation, plugin runtime support, and JavaScript bridge integration.

This domain provides plugin and script execution capabilities to runtime and tool services. It does not own the core application root or node route decisions.

## Crates

- `sdk`: public plugin SDK, ToolPkg models, hooks, compose DSL, Wasm runtime, and JavaScript SDK types.
- `codegen`: SDK binding and declaration generation support.
- `javascript-bridge`: JavaScript engine bridge, script loading, Java bridge, and JS tool support.

## Target Split

The target layout adds a dedicated `runtime` crate for plugin registry and ToolPkg lifecycle ownership.
