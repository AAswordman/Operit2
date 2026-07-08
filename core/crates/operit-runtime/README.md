# operit-runtime

`operit-runtime` is the parent orchestration crate for Operit2 core. It wires
host capabilities, storage, tools, providers, JavaScript execution, preferences,
plugins, workspace services, chat services, and UI-facing services into one
runtime.

## Responsibilities

- Manage chat state, message history, branching, summaries, attachments,
  token statistics, and workspace bindings.
- Start and stop runtime services used by UI, CLI, and proxy layers.
- Install `operit-tools::runtime_support::ToolRuntimeSupport` so tool code can
  call runtime-owned services through a child-defined trait.
- Install `operit-providers::runtime_support::ProviderRuntimeSupport` so
  provider code can call runtime-owned services through a child-defined trait.
- Create the `operit-js-bridge` JavaScript engine and pass it to ToolPkg
  execution through the `operit-tools` JS execution trait.
- Compose repositories from `operit-store` with models from `operit-model`.
- Load bundled plugin assets, workspace templates, and ToolPkg assets owned by
  the runtime crate.
- Bridge runtime services to host capabilities through `operit-host-api`.

## Main Modules

- `src/core/application/`: `OperitApplication` startup and crate wiring.
- `src/core/chat/`: AI message manager and message-processing plugins.
- `src/data/`: runtime-owned preferences, backup, and runtime data services.
- `src/plugins/`: built-in plugin assets, registry, ToolPkg bridges, toolbox,
  and workflow lifecycle plugins.
- `src/services/`: chat service core, delegates, TTS synthesis/playback, host
  info, host interaction, terminal, workspace orchestration, runtime event
  ingress, and support implementations installed into child crates.

## Key Files

- `src/services/ChatServiceCore.rs`: high-level chat state service used by UI
  and proxy layers.
- `src/services/ToolRuntimeSupportService.rs`: runtime implementation of the
  tool crate support trait.
- `src/services/ProviderRuntimeSupportService.rs`: runtime implementation of
  the provider crate support trait.
- `src/services/WorkspaceService.rs`: workspace orchestration service.
- `src/core/application/OperitApplication.rs`: startup wiring for host,
  storage, provider support, tool support, JS engine, and services.

## Build-Time Assets

The runtime build script packages bundled plugins and workspace templates into
Rust modules so runtime code can load built-in extension assets without reading
project source directories at runtime.

## Split Crates

See `core/CRATE_BOUNDARIES.md` for the full crate boundary and dependency
direction rules.
