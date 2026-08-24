# Command

Command crates expose user-facing command orchestration over the core application surface.

This domain should consume facade handles, local clients, and explicit access handles. It should not create runtime, node, proxy, provider, tool, or access internals directly.

## Crates

- `core`: current command orchestration crate for chat, model, memory, tool, plugin, package, skill, MCP, storage, workspace, host, prefs, update, and log commands.
