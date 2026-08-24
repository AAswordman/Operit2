# Tool

Tool crates own tool contracts, tool execution, builtin tools, skills, packages, MCP, and tool scripting.

This domain may use host, model, store, provider, plugin, and runtime capability handles. It should not own the application root, node route decisions, or access server tasks.

## Crates

- `services`: current aggregate tool crate containing tool execution, builtin tools, permissions, skills, packages, MCP, and JavaScript tool support.

## Target Split

The target layout separates this aggregate into `contracts`, `runtime`, `builtin`, `skill`, `package`, `mcp`, and `javascript` crates.
