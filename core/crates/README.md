# Core Crate Layout

This directory is organized by domain first and by crate second.

Each first-level directory groups related crates. Each second-level directory is an actual Cargo crate with its own `Cargo.toml`, `src`, and `README.md`.

## Domains

- `foundation`: shared host contracts, models, utilities, and link protocol types.
- `persistence`: stores, repositories, identity data, and synchronization persistence.
- `provider`: LLM, media, market, memory, and local model services.
- `tool`: tool execution, builtin tools, skills, packages, MCP, and tool scripting.
- `plugin`: plugin SDK, SDK code generation, plugin runtime support, and JavaScript bridge.
- `runtime`: core business runtime and application-level services.
- `node`: route macros, node routing, local route runtime, space runtime, and node synchronization.
- `access`: identity, pairing, authentication, peer transport, server, and web access control.
- `rslink`: pure Rust-to-link-to-Rust protocol runtime and shared code generation foundations.
- `proxy`: core-app proxy scan, language emitters, bridge contracts, and local proxy runtime.
- `application`: composition roots that assemble runtime, proxy, node, and access into one Core tree.
- `command`: command orchestration over the core application facade.

## Naming

Directory names describe the responsibility at that layer. Package names may retain public crate names during migration, but new physical paths should keep the domain name out of the second-level directory unless it adds meaning.
