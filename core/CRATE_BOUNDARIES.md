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
- `operit-plugin-sdk`: publishable plugin contracts, standalone JavaScript
  package parsing and management, fixed JavaScript tool APIs, Compose DSL,
  ToolPkg archive parsing/loading, full-package queries, hook dispatch, and
  JavaScript execution contracts implemented by a host.
- `operit-tools`: the tool system, including built-in tools, ToolPkg, MCP,
  skill packages, permission-aware tool registration, package lifecycle
  management, tool execution metadata, and tool-specific runtime support
  traits. Shared plugin formats and loaders live in `operit-plugin-sdk`.
- `operit-local-models`: local model manifests, installed model registry
  contracts, storage path helpers, and local STT/TTS/chat session interfaces.
  Runtime-owned operations are exposed through `LocalModelRuntimeSupport`.
- `operit-providers`: provider contracts and built-in implementations,
  including `AIService`, provider request/result/stream types,
  `ProviderRuntimeSupport`, LLM adapters, conversation orchestration,
  TTS/STT/market services, prompt hooks, and ToolPkg provider integration.
- `operit-js-bridge`: JavaScript engine integration and JS-specific ToolPkg
  execution surfaces. It implements the JS execution trait exposed by
  `operit-plugin-sdk`.
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

`plugin-sdk/host-api` -> `util` -> `model` -> `store` -> `tools/local-models/providers/js-bridge` -> `runtime` -> `core-proxy/command-core`

The arrows describe who may depend on definitions to its left. `runtime` may
depend on all child crates. Child crates must not depend on `operit-runtime`.

Current practical edges:

- `operit-util` depends on `operit-host-api` for cross-target host types.
- `operit-model` depends on `operit-util` and `operit-host-api`.
- `operit-store` depends on `operit-model`, `operit-util`, and
  `operit-host-api`.
- `operit-plugin-sdk` depends only on public ecosystem crates.
- `operit-tools` depends on `operit-plugin-sdk`, `operit-store`,
  `operit-model`, `operit-util`, and `operit-host-api`.
- `operit-local-models` depends on `operit-host-api`, `operit-util`, and public
  ecosystem crates, and does not depend on `operit-runtime` or concrete host
  implementations.
- `operit-providers` depends on `operit-tools`, `operit-store`,
  `operit-plugin-sdk`, `operit-model`, `operit-util`, and `operit-host-api`,
  but not on `operit-runtime`.
- `operit-js-bridge` depends on `operit-plugin-sdk` plus store/util/host-api,
  and has no dependency on `operit-tools`.
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
- `operit-local-models::runtime_support::LocalModelRuntimeSupport` defines the
  local-model-side requests for model data roots, installed registry snapshots,
  registry persistence, and available manifests. `operit-runtime` owns the
  concrete implementation.
- `operit-plugin-sdk::javascript::JsExecutionHost` defines the complete
  JavaScript-to-application execution boundary. `operit-tools` implements it
  with the real tool, package, resource, and IPC logic.
- `operit-plugin-sdk::javascript::JsExecutionProvider` defines the engine
  provider boundary. `operit-js-bridge` implements it with
  `QuickJsExecutionProvider`, and `operit-runtime` injects that provider into
  `operit-tools`.
- `operit-plugin-sdk::javascript::JsPackageExecutor` binds one package runtime
  and execution host for Rust-to-JavaScript package calls.
- `operit-plugin-sdk::javascript::JsExecutionEngine` defines ToolPkg engine
  operations implemented by `operit-js-bridge::javascript::JsEngine`.

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

Provider contracts, built-in implementations, and provider orchestration
belong in `operit-providers`. Runtime-specific operations are exposed through
`ProviderRuntimeSupport`.

## Local Model Boundary

Local model manifests, installed model registry contracts, storage path helpers,
and local inference session interfaces belong in `operit-local-models`. Local
model runtime behavior is exposed through `LocalModelRuntimeSupport`. Network
transfer is submitted through `operit-host-api::HttpHost`; concrete hosts own
download worker concurrency, progress reporting, and cancellation handling.

## Tool Boundary

MCP and skill packages are part of the tool system. Their managers, package
models, repositories, and execution helpers live under `operit-tools`. ToolPkg
package formats, generic package state management, hooks, loaders, DSL, fixed
JavaScript APIs, full-package services, and JS execution contracts live in
`operit-plugin-sdk`. Host persistence, built-in asset scanning, MCP/skill
integration, and Operit-specific package commands remain in `operit-tools`;
the concrete JS engine lives in `operit-js-bridge`.

## Proxy Boundary

`operit-core-proxy` is a projection layer. It scans selected public surfaces
from the split crates and emits transport-facing Rust and Dart code. It does not
own runtime behavior, host implementations, provider logic, tool logic, or
storage.
