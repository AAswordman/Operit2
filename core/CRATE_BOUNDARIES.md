# Operit Core Crate Boundaries

This document defines the crate split around `operit-runtime`. The important
rule is dependency direction: lower crates expose data, services, and traits;
the parent runtime wires concrete behavior together.

## Crates

- `operit-host-api`: host capability traits and `HostManager`. Platform hosts
  implement this boundary; core crates consume host behavior through traits.
- `operit-util`: shared utility code, stream helpers, logging helpers,
  serializers, path layout constants, and media/document helpers.
- `operit-model`: shared data structures used by runtime, store, tools,
  providers, proxy generation, and host-facing DTOs.
- `operit-store`: persistence layer, DAO/repository code, preference stores,
  SQLite/ObjectBox-style helpers, sync stores, and storage host registration.
- `operit-tools`: the tool system, including built-in tools, ToolPkg, MCP,
  skill packages, permission-aware tool registration, tool execution metadata,
  and tool-specific runtime support traits.
- `operit-providers`: provider-facing code moved out of the old runtime `api`
  tree, including chat provider orchestration, LLM provider adapters, market
  APIs, voice providers, prompt composition helpers, and provider runtime
  support traits.
- `operit-js-bridge`: JavaScript engine integration and JS-specific ToolPkg
  execution surfaces. It implements the JS execution trait exposed by
  `operit-tools`.
- `operit-runtime`: the parent orchestration crate. It owns application startup,
  service wiring, plugin assets, preferences, workspace services, chat service
  lifecycles, provider support implementations, and tool support
  implementations.
- `operit-core-proxy`: build-time proxy generation and local dispatch. It scans
  selected public surfaces from runtime/model/store/tools/providers and emits
  Rust and Dart proxy code.
- `operit-link`: transport protocol and client/server primitives.
- `operit-command-core`: CLI-facing command layer built on the runtime and the
  split core crates.

## Dependency Direction

The intended direction is:

`host-api` -> `util` -> `model` -> `store` -> `tools/providers/js-bridge` -> `runtime` -> `core-proxy/command-core`

The arrows describe who may depend on definitions to its left. `runtime` may
depend on all child crates. Child crates must not depend on `operit-runtime`.

Current practical edges:

- `operit-util` depends on `operit-host-api` for cross-target host types.
- `operit-model` depends on `operit-util` and `operit-host-api`.
- `operit-store` depends on `operit-model`, `operit-util`, and
  `operit-host-api`.
- `operit-tools` depends on `operit-store`, `operit-model`, `operit-util`, and
  `operit-host-api`.
- `operit-providers` depends on `operit-tools`, `operit-store`,
  `operit-model`, `operit-util`, and `operit-host-api`.
- `operit-js-bridge` depends on `operit-tools` plus store/util/host-api to run
  JS tools.
- `operit-runtime` depends on all child crates and installs their runtime
  support implementations.

## Parent-Owned Behavior

When a child crate needs behavior owned by the parent runtime, the child crate
defines a trait and consumes a trait object. The parent runtime implements and
installs that trait.

Examples:

- `operit-tools::runtime_support::ToolRuntimeSupport` defines the tool-side
  requests for character-card tool access, built-in assets, environment
  variables, MCP description generation, chat operations, memory bindings, and
  structured edits. `operit-runtime` implements it in
  `services/ToolRuntimeSupportService.rs`.
- `operit-providers::runtime_support::ProviderRuntimeSupport` defines the
  provider-side requests for model bindings, provider profiles, prompt context,
  token accounting, ToolPkg AI provider hooks, and timing logs.
  `operit-runtime` implements it in `services/ProviderRuntimeSupportService.rs`.
- `operit-tools::tools::javascript::JsExecutionEngine` defines JS execution
  operations needed by ToolPkg. `operit-js-bridge` implements it with
  `javascript/JsEngine.rs`, and runtime wires the engine into the package
  manager.

This keeps child crates independent from runtime internals while still allowing
them to call parent-owned services through explicit contracts.

## Host Boundary

Host integrations belong behind `operit-host-api`. `HostManager` owns the active
host capability set and lifecycle-facing host handles. Platform crates provide
implementations; runtime code sees explicit traits and environment descriptors.

Host APIs are not part of the tool crate. Tools may consume host capabilities
through the host-api traits or through runtime-installed tool support when the
operation is runtime-owned.

## Store Boundary

Repositories and persistence data structures belong in `operit-store`.
Reusable value models belong in `operit-model`. Runtime services may compose
repositories, but repository ownership stays in store. Workspace orchestration
belongs in `operit-runtime/src/services/WorkspaceService.rs`.

## Provider Boundary

The old runtime `api` tree belongs in `operit-providers`. Provider code may use
model, store, util, tools, and host-api. Runtime-specific operations are exposed
through `ProviderRuntimeSupport`.

## Tool Boundary

MCP and skill packages are part of the tool system. Their managers, package
models, repositories, and execution helpers live under `operit-tools`. ToolPkg
JS execution is expressed as the `JsExecutionEngine` trait in `operit-tools`;
the concrete JS engine lives in `operit-js-bridge`.

## Proxy Boundary

`operit-core-proxy` is a projection layer. It scans selected public surfaces
from the split crates and emits transport-facing Rust and Dart code. It does not
own runtime behavior, host implementations, provider logic, tool logic, or
storage.
