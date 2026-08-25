# Rslink

Rslink crates own the reusable Rust-to-link-to-Rust protocol layer.

This domain is the pure protocol and shared type-conversion foundation reused by proxy, route, CLI local core, and route-space Rust flows.

## Crates

- `runtime`: runtime helpers for converting Rust values to link values and back.
- `codegen`: shared IR, type parsing, naming, and schema foundations used by code generators.

## Boundary

Rslink is not core-app proxy runtime, node routing, or Dart emission. Those sit in `proxy`, `node`, and proxy emitter crates.
