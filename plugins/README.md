# Operit2 Plugins

This directory contains the Operit2 plugin workspace.

- `types/`: current v2 TypeScript declarations generated from the Rust SDK.
- `types-v1/`: the independent handwritten v1 declaration snapshot, with source provenance.
- `sdk/`: language clients, native carrier, and runnable SDK examples under `sdk/examples/`.
- `packages/buildin/`: ToolPkg sources that are packaged as built-in plugins.
- `packages/external/`: official bundled ToolPkg sources that are packaged with the app and loaded by the user from "more plugins".
- `packages/examples/`: sample ToolPkg sources for development and manual testing.
- `skills/external/`: official bundled Skill sources that are packaged with the app and imported by runtime features when needed.
- `docs/`: plugin authoring notes.
- `tools/`: development and sync tools.

`packages/buildin/`, `packages/external/`, and `packages/examples/` use the same source layout. The difference is packaging: `buildin` is loaded as current built-in plugins, `external` is bundled as optional official plugins, while `examples` is kept for development samples.

Bundled Skill sources may include `skill.include.json`. The runtime build script resolves each declared source into the final bundled Skill tree, so large shared resources such as `types/`, docs, and package examples can stay in their canonical locations.
# Plugin development hot reload

Start a debug Flutter application containing the `ext.operit.reloadPlugins`
extension, then run from the repository root:

```powershell
.venv/Scripts/python.exe plugins/tools/sync_plugin_packages.py
```

The script discovers the authenticated VM Service created by the running
Flutter development service, builds packages, transfers them through the VM service, writes them
into runtime package storage through the host API, and reloads the catalog and
visible plugin page. Development packages persist in runtime package storage
and replace matching bundled packages through the existing package scan rules.
Reloading discards unsaved plugin UI state and recreates plugin execution engines.
The VM service must be reachable from this computer (including the forwarded
port used by Flutter for a connected phone). Keep the authenticated URL private.
`--no-hot-reload` disables remote delivery explicitly.

## Edge hardware ports

See [Core 插件调用 Edge 硬件端口](docs/edge-ports.md) for the initial `Tools.Edge.execute` /
`Tools.Io.execute` APIs, bounded payloads, authorization and the GPIO-only scope.
Serial streaming/session support is not implemented yet.

Generic Edge-to-Core action events and the `on_edge_event` listener are documented in [edge-events.md](docs/edge-events.md).
