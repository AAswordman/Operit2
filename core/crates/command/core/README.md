# operit-command-core

`operit-command-core` contains reusable command-mode behavior for Operit2.
The CLI crate delegates command families here so command execution can share the
same runtime application context and output shape.

## Responsibilities

- Initialize an `OperitApplication` from an application context for command
  runs.
- Dispatch top-level command families such as chat, model, memory, workspace,
  tool, package, plugin, skill, MCP, market, preferences, approval, host, log,
  and update.
- Return command output through a serializable `CoreCommandOutput` structure.
- Keep command behavior independent from terminal UI rendering.

## Key Files

- `src/lib.rs`: public command entry points.
- `src/output.rs`: stdout and stderr capture structure.
- `src/commands/mod.rs`: top-level command router and usage output.
- `src/commands/*.rs`: individual command family implementations.

## Command Surface

The command router accepts argument vectors after the executable name. Unknown
or empty command families print the core usage surface instead of mutating
runtime state.

## Consumers

The desktop CLI and remote command surfaces can call into this crate while
keeping their own argument parsing, terminal rendering, and access/session
handling outside the shared command implementation.
