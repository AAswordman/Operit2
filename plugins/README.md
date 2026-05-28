# Operit2 Plugins

This directory contains the Operit2 plugin workspace.

- `types/`: shared TypeScript declarations for ToolPkg authors.
- `buildin/`: ToolPkg sources that are packaged as built-in plugins.
- `examples/`: sample ToolPkg sources for development and manual testing.
- `docs/`: plugin authoring notes.
- `tools/`: development and sync tools.

`buildin/` and `examples/` use the same source layout. The difference is packaging: `buildin/` is intended for compile-time bundled plugins, while `examples/` is kept for development samples.
