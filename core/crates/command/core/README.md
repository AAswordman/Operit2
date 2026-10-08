# operit-command-core

`operit-command-core` contains reusable command-mode behavior for Operit2.
The CLI crate delegates command invocation here so command execution shares the
runtime application context and the existing output shape.

## Responsibilities

- Initialize an `OperitApplication` from an application context for command runs.
- Resolve each root command's unique owner from inherent Core declarations and
  the runtime's ready ToolPkg command catalog.
- Execute the selected inherent handler or immutable plugin handler token once.
- Return command output through the serializable `CoreCommandOutput` structure.
- Keep command behavior independent of terminal UI rendering and plugin domains.

## Key Files

- `src/lib.rs`: public command entry points and JSON output finalization.
- `src/output.rs`: stdout, stderr, and structured output capture.
- `src/commands/catalog.rs`: inherent Core directory, owner resolution, and
  generic routing unit tests.
- `src/commands/mod.rs`: top-level dispatch, Core scope aliases, and dynamically
  discovered root usage.
- `src/commands/delegated.rs`: one ready manager/catalog snapshot and selected
  plugin handler execution.
- `src/commands/plugin.rs`: plugin management and shared-catalog command
  discovery/explicit execution.
- `scripts/check-routing.mjs`: source-contract and generic catalog-fixture
  regression checks without Rust compilation.

## Command Ownership

The router accepts argument vectors after the executable name. Empty arguments
produce usage from the inherent Core directory and the enabled registered
plugin commands. Unknown roots return an explicit routing error.

Core's static directory describes only its own inherent handlers. Plugin root
names are never enumerated there: any root contributed by `ToolPkg.registerCoreCommand`
is selected from the actual registration catalog. An enabled, uniquely owned
new root needs no Core routing change. Existing plugin roots such as `plan` and
`goal` use this same mechanism; they are examples, not reserved Core names.

Lookup is exact and case-insensitive. A disabled matching declaration, duplicate
registered name, or Core/plugin name collision is an error. These outcomes are
distinct from an unregistered name and never execute another owner. Registration
conflicts use the SDK catalog's validation, including conflicts involving disabled
declarations.

### Ready Snapshot and Single Execution

Every invocation, including root usage and `plugin commands`, obtains its
`ReadyCommandRegistry` through `RuntimePackageManager::readySnapshot`.
The SDK waits outside the manager lock for canonical loading to complete and
clones the manager only after confirming readiness. Merely creating the manager
or scheduling a background package scan does not make its directory ready.

The command catalog is built from this completed manager snapshot. Root ownership
is resolved before dispatch. The selected plugin token is passed to
`executeResolvedToolPkgCoreCommand` on that same manager; execution does not
resolve its name again or acquire a second manager snapshot. Runtime loading,
catalog validation, and provider errors terminate the invocation unchanged.

`plugin commands` reads and validates enabled command metadata from the same
catalog. `plugin exec <name> [args...]` resolves the explicitly addressed plugin
root from that catalog and executes its selected token. Explicit plugin execution
of a Core-owned root is an ownership error. Plugin management operations retain
their existing live-manager behavior; they do not replace the selected invocation
catalog.

### Routing Invariants for Integration Acceptance

- The inherent Core directory contains no plugin-domain root allow-list.
- Registration metadata is the authority for every plugin root, including new
  names that Core has never encountered.
- Root resolution returns one inherent enum handler or one immutable plugin
  declaration token before execution; catalog errors prevent execution.
- Disabled declarations, duplicates, ownership collisions, and unknown roots
  have explicit outcomes; they never select a different handler.
- Each invocation has one completed manager/catalog snapshot. Selected-token
  execution never acquires a second snapshot or resolves the name again.
- `plugin commands`, explicit plugin execution, and root help use that same
  command directory, including the SDK's conflict validation.
- Scope aliases are processed only for a resolved inherent Core owner. Provider
  arguments, text output, and explicit JSON documents retain their original
  contracts.

### Outstanding Readiness Lifecycle Integration

The consumer-side readiness barrier is connected in this crate. Startup-side
lifecycle wiring still needs acceptance in the runtime that owns loading.
At this integration checkpoint, `OperitApplication::dispatchPluginLoading`
submits its background task without calling
`claimInitialPackageRegistryLoad`; submission errors use `expect` rather than
publishing the original error through `failPackageRegistryLoad`.
The SDK defines both methods, but their application startup call sites are not
connected yet. This is an integration gap, not proof of cold-start readiness.

The startup owner must claim the initial load before submitting it, publish
actual scheduler/startup failures to all readiness waiters, and keep canonical
source scanning and registration completion tied to the SDK's `begin_scan` and
`complete_scan` lifecycle. `wait_until_ready` explicitly rejects an unscheduled
startup instead of accepting an incomplete manager. Waiting must not initiate a
source scan or hide the original loading error.

Cold commands racing the scheduled load, a failed scheduler submission,
registration failures, repeated startup notifications, and a reload during
selected-token execution still require runtime lifecycle verification. The
Node checker proves the router's snapshot/token contracts; it does not execute
application startup or certify those lifecycle transitions. Future generic
consumer/provider entry points must obey the same readiness and unique-owner
rules instead of relying on an uninitialized registry clone.

### Usage, Aliases, and Output

Root usage builds its root list from the inherent Core directory and registered
plugin metadata. Each contributed command supplies its own usage, description,
and owner. JSON help retains `usage` and adds `builtinCommands` and
`registeredCommands` for structured discovery.

`plugin`, `package`, `skill`, and `mcp` retain their existing `scope` aliases.
Alias processing occurs only after the root is resolved to its inherent Core
handler and passes the original extension kind and scope arguments unchanged.
Other inherent Core command families retain their handlers and argument slices.

Plugin subcommand arguments, including empty values, and JSON mode are forwarded
unchanged to the selected handler. Text stdout/stderr are preserved. JSON mode
requires the provider's explicit `json` document and uses the existing
`CoreCommandOutput` finalizer, including its stderr behavior. Structured plugin
error documents remain provider results; original execution errors remain errors.

## Migration Status and Verification Boundary

The former `people.rs`, `tag.rs`, and `memory.rs` implementations remain on disk
as readable migration source. They are not mounted as Rust modules and are not
executable through this router. Their previous root names consequently require
real enabled plugin registrations and real plugin-owned services.

This routing integration introduces no domain-specific Core host bridge or DTOs.
It does not establish that the character-card/memory plugin's production business
services are ready or that their UI/API/command paths work end to end. Those domain
implementations require separate verification. A registration fixture or static
routing check cannot certify production storage or business behavior.

Run the non-compiling routing check from the repository root:

```sh
node core/crates/command/core/scripts/check-routing.mjs
```

The checker verifies the actual Core caller signatures, one readiness snapshot,
immutable-token execution, generic registered help, unchanged scope aliases,
existing output finalization, and all inherent Core routes. Its explicitly
isolated catalog fixtures cover a runtime-generated command name, `plan`/`goal`,
enabled/disabled ownership, duplicates, Core collisions, unknown names, exact
arguments, one provider call, provider failures, and structured JSON output.
These fixture checks are not Rust runtime or production-plugin integration tests.

The Rust unit tests in `catalog.rs` and `delegated.rs` cover arbitrary registered
roots, exact lookup, disabled/duplicate/colliding/unknown ownership, catalog errors,
scope kinds, immutable-token identity, argument forwarding, empty subcommands,
provider failures, text/JSON output, and the explicit JSON result requirement.
They are present for a later Rust test run; the non-compiling checker does not
execute them.

## Consumers

Desktop CLI and remote command surfaces can invoke this crate while keeping
argument parsing, terminal rendering, and access/session handling outside this
shared command implementation.
