# Operit2 Plugins

This directory contains the Operit2 plugin workspace.

- `types/`: shared TypeScript declarations for ToolPkg authors.
- `buildin/`: ToolPkg sources that are packaged as built-in plugins.
- `external/`: official bundled ToolPkg sources that are packaged with the app and loaded by the user from "more plugins".
- `examples/`: sample ToolPkg sources for development and manual testing.
- `docs/`: plugin authoring notes.
- `tools/`: development and sync tools.

`buildin/`, `external/`, and `examples/` use the same source layout. The difference is packaging: `buildin/` is loaded as current built-in plugins, `external/` is bundled as optional official plugins, while `examples/` is kept for development samples.
