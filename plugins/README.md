# Operit2 Plugins

This directory contains the Operit2 plugin workspace.

Current packages explicitly declare `"api_version": "2.0.0"`: in `manifest.json`
for ToolPkg and in the `METADATA` root object for standalone scripts. Packages
that omit the field use the `1.0.0` compatibility contract. ToolPkg subpackages
inherit the contract from their container manifest.

The marketplace and local script details display the same API contract for
scripts and ToolPkg. API `2.0.0` releases carry an API v2 marker. Publishing
reads the version from the package metadata; unversioned historical releases
are displayed as API `1.0.0`.

- `types/`: current v2 TypeScript declarations generated from the Rust SDK.
- `types-v1/`: the independent handwritten v1 declaration snapshot, with source provenance.
- `sdk/`: language clients, native carrier, and runnable SDK examples under `sdk/examples/`.
- `packages/buildin/`: ToolPkg sources that are packaged as built-in plugins.
- `packages/external/`: official bundled ToolPkg sources that are packaged with the app and loaded by the user from "more plugins".
- `packages/examples/`: sample ToolPkg sources for development and manual testing.
- `skills/buildin/`: built-in Skills that are installed automatically and hidden from AI until enabled.
- `skills/external/`: official bundled Skill sources that are packaged with the app and imported by runtime features when needed.
- `docs/`: plugin authoring notes.
- `tools/`: development and sync tools.

`packages/external/proactive_messages/` provides optional AI proactive messages with per-chat intervals, random delays and trigger probability. It uses the uniform typed `Tools.Chat.scheduleMessage`, `cancelScheduledMessage` and `listScheduledMessages` APIs; persistence, delivery, notifications and iOS resume reconciliation are owned by the host.

`packages/external/image_workbench/` provides the optional official Image Workbench, available from "more plugins" and disabled by default. It owns the unified image-generation tools and six internal provider adapters.

`packages/buildin/docs_editor/` provides the built-in Office documents editor for DOCX, PPTX and PDF. It is disabled by default and can be enabled in the current space.

`packages/buildin/`, `packages/external/`, and `packages/examples/` use the same source layout. The difference is packaging: `buildin` is loaded as current built-in plugins, `external` is bundled as optional official plugins, while `examples` is kept for development samples.

Bundled Skill sources may include `skill.include.json`. The runtime build script resolves each declared source into the final bundled Skill tree, so shared resources such as `types/` and docs can stay in their canonical locations. PackageBuilder provides VFS entry paths for browsing installed plugins on demand instead of bundling package examples or indexing their archive members.

`skills/buildin/PackageBuilder/` contains the built-in plugin development suite, displayed as “Operit 插件开发套件”. `PackageBuilder` remains its stable skill identity and directory name. It starts hidden from AI; quick plugin creation explicitly enables it. `packages/external/12306.ts` is an optional bundled external plugin, while the built-in `operit_editor` package is enabled by default.
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

## Edge hardware integration

See [硬件 ToolPkg 接入](docs/edge-plugin-guide.md) for generic Call / Watch / Push,
`Tools.device(nodeId).Edge` / `Io`, payload limits and device-local interaction.
[Edge responsibilities](../edge/docs/architecture.md) defines Space membership,
device configuration and the boundary between firmware and host processing.
