# Operit2 Plugins

This directory contains the Operit2 plugin workspace.

- `packages/buildin/`: ToolPkg sources that are packaged as built-in plugins.
- `packages/external/`: official bundled ToolPkg sources that are packaged with the app and loaded by the user from "more plugins".
- `packages/examples/`: sample ToolPkg sources for development and manual testing.
- `skills/external/`: official bundled Skill sources that are packaged with the app and imported by runtime features when needed.
- `docs/`: plugin authoring notes.
- `tools/`: development and sync tools.

`packages/buildin/`, `packages/external/`, and `packages/examples/` use the same source layout. The difference is packaging: `buildin` is loaded as current built-in plugins, `external` is bundled as optional official plugins, while `examples` is kept for development samples.

ToolPkg TypeScript declarations live in `../sdk/toolpkg-runtime/types` and plugin packages reference that SDK contract directly.

Bundled Skill sources may include `skill.include.json`. The runtime build script resolves each declared source into the final bundled Skill tree, so shared resources such as SDK types, docs, and package examples can stay in their canonical locations.
