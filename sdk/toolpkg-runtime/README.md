# ToolPkg Runtime SDK

`sdk/toolpkg-runtime` is the public ToolPkg runtime product area for embedders.

```text
runtime   Rust runtime and CLI
npm       Node/Electron wrapper and host-facing TypeScript types
fixtures  Real ToolPkg samples used by runtime tests
types     ToolPkg JavaScript API declarations used by plugin packages
```

The runtime code here is migrated from the existing ToolPkg/JS runtime. Operit app business code stays in `core`.
Shared host and storage crates live under `sdk/shared`.
The Node wrapper talks to the Rust runtime over JSONL and can hand tool calls back to the host via `host.invokeTool(...)`.
