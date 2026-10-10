# ToolPkg v1 compatibility in Operit2

## Supported versions and platform scope

Operit2's supported ToolPkg API version is **2.0.0**. Its loading compatibility
for **1.0.0** and **1.0.1** is incomplete; accepting those version identifiers
does not imply complete implementation of their contracts.

Operit1 fully supports **1.0.0** and **1.0.1**. These are the older API forms
primarily designed for Android, not a promise of cross-platform behavior.

API **2.0.0** is designed for multiple platforms. Nearly all public interfaces
provide cross-platform compatibility through the shared host layer, so ordinary
features should use those interfaces without separate per-platform implementations.
Authors must still consider other platforms when using platform-specific
interfaces, document their scope, and verify installation, UI, and core behavior
on the other intended platforms. Migrating a legacy package requires checking
API, path, and platform semantics, not merely changing its version declaration.

New Operit2 ToolPkg manifests should explicitly declare `"api_version": "2.0.0"`
and use the current bundled declarations. Manifest `schema_version`, ToolPkg
`api_version`, and the plugin's release `version` identify different things.

## Contract ownership

- **v1:** `plugins/types-v1/*.d.ts`, copied without edits from the handwritten v1 SDK.
- **v2:** `core/crates/plugin/sdk/src/js_sdk/*.rs`, rendered to `plugins/types` by the existing code generator.
- **Adapters:** `core/crates/plugin/sdk/src/compat/v1/*.js`. JSDoc imports check the input/output contract against v1 and calls against v2 using `tsc --checkJs --noEmit`.

There are two contract families, not one declaration tree for every patch version.
Minor/patch availability remains documented by upstream version annotations.
No Flutter branches or alternative platform runtimes are introduced.

## Loading and dispatch

The archive parser accepts explicitly declared `api_version` values `1.0.0`,
`1.0.1`, and `2.0.0`. The original version is retained for registration and execution.
Unknown releases remain rejected until admitted explicitly.

Omitted `api_version` retains the existing Operit2 meaning (`2.0.0`). Legacy
archives relying on the original application's omitted-version rule must declare
`1.0.0`; their origin is not inferred from filenames or source text.
Standalone scripts executed outside a ToolPkg context use the current SDK contract.

The runtime method builder supports:

```js
api.method()
    .between('1.0.0', '2.0.0', legacyImplementation)
    .since('2.0.0', currentImplementation)
```

`between` is lower-inclusive and upper-exclusive. `since` is bounded by the next
major API family. Shared implementations can bind multiple explicit ranges:

```js
api.method()
    .between('1.0.1', '2.0.0')
    .since('2.0.0')
    .implement(sharedImplementation)
```

An explicit range spanning multiple major families is possible but must be
intentional. Overlap, unbound ranges, and unmatched versions are errors. Dispatch
uses the active call, not the version present when the engine was constructed.
Adapter failures propagate; they never select a different implementation.

## Implemented adapters

The existing generator appends the handwritten adapters to `js_tools.js` after
emitting the canonical v2 bindings. Both inline and modular engine bootstrap paths
therefore install the same code. Adapters capture the canonical functions before
installing versioned entry points, so v1 cannot recursively invoke itself.

### Files

All functions in the copied v1 `Files` namespace have signature adapters:

- Positional `environment`, `sourceEnvironment`, and `destEnvironment` arguments.
- `read` and `download` object overloads, without modifying caller-owned options.
- `zip`'s fourth `include_root_directory` argument and `download`'s fourth headers argument.
- `env` metadata on returned file results and nested apply/create/edit operations.
- Root and child paths returned by find/grep, including nullable grep context conversion.

Environment interpretation follows the old API, not the OS running Operit2:

- Omitted environment means the documented v1 `android` environment.
- Linux absolute paths map to `/mnt/linux`.
- Android `/sdcard` and `/data` use the existing VFS roots.
- `/storage/emulated/0` maps to `/sdcard`.
- `/app` and `/mnt` paths returned by current resource/config APIs remain VFS paths.
- Unmapped Android roots and unsupported environment names are explicit errors.

Actual mount availability is decided by the existing host/VFS implementation.
This adapter does not create mounts, emulate Android/Linux filesystems, or bypass
host permissions. In particular, a Linux environment call on a host without that
mount fails. Legacy content-URI access is not implemented by this mapping.

### Chat.call (1.0.1)

- Nullable legacy `toolName` becomes the optional current field.
- Returned role and finish-reason values are checked against the closed v1 vocabulary.
- Metadata is validated as JSON rather than asserted to a wider type.
- Request objects and original turns are not mutated.
- `1.0.0` cannot invoke this API; `2.0.0` retains its existing implementation.

### Registration

`registerChatMessageMenuItem` and `registerChatRuntimeHook` explicitly cover
`1.0.1` and `2.0.0`. Their current registration normalizers are shared. Other
existing registration implementations remain unchanged.

### Workflow

`Tools.Workflow` now calls the public API of `com.operit.workflow` through the
existing dependency transport. It covers list/create/get/update/patch/enable/disable/
delete/trigger. Every v1 package receives the workflow prerequisite (`>= 0.2.0`)
in its effective runtime `requires` during archive loading; old manifests need
no edits. Explicit stricter bounds remain in effect. Both packages must be enabled. The workflow plugin owns all state
and validates legacy graph inputs. See `PUBLIC_API.md` for the distributed typed
client and provider capability limits. No historical database migration is implicit.

### Caller identity after the role-provider migration

API 2.0.0 exposes `getCallerParticipantId()` consistently in the runtime and
current declarations. Its value belongs to the authenticated configuration
provider and is not a runtime-wide character-card identity.

For API 1.0.0 and 1.0.1, `getCallerCardId()` projects the execution participant
only when the authenticated owner is `com.operit.character_cards`. A call without
an execution participant returns the documented `undefined`; another provider's
participant is rejected. The legacy getter is absent from the 2.0.0 surface, and
a retained legacy function still checks the version of the active call.
Core transports the generic owner identity without exposing the owner's message
extension or restoring any character-card fields to the core model.

### Chat and memory after the role-provider migration

Every v1 archive now declares the effective character-provider prerequisite
`com.operit.character_cards >= 0.1.0` alongside its workflow prerequisite.
Explicit stricter bounds remain in effect; incompatible upper bounds are errors.

- `Chat.listCharacterCards` reads the provider's real role directory, including
  roles with fixed models or shared memory, without translating settings bindings.
- Positional `Chat.createNew(group, setAsCurrentChat, characterCardId)` becomes
  current creation options with the provider's namespaced selection input.
  The requested role is validated before creation; folder membership uses the
  existing generic chat folder API.
- Ordinary `Chat.sendMessage` uses the native send tool, translating the explicit
  role into `participant_id` and retaining sender and legacy turn controls.
- `Chat.sendMessageStreaming` converts growing visible-text snapshots into
  legacy start/chunk callbacks and returns the actual completion receipt.
  Snapshot rewrites cannot be expressed by the old append-only callback and are
  explicit errors. Waifu aggregation, custom timeouts, sender overrides and
  non-default persistence/visibility/warning controls remain outside this stream
  adapter. Calls without a completion receipt fail.
- `Memory` methods invoke the provider's actual executable memory tools and
  decode the package executor's serialized results. Both positional and object
  overloads preserve explicitly supplied caller scope and option values;
  `Memory.query` retains its legacy default limit of 20 and structured result.
- Relationship IDs are canonical decimal strings in the provider, not UUIDs.
  Positive safe integer IDs round-trip losslessly; IDs outside JavaScript's safe
  integer range are rejected. Single-link updates use the legacy result envelope.
- Memory move title arrays must be representable by the executable tool's title
  delimiters. Non-document `Memory.getByTitle` encodes the validated legacy query
  projection as JSON text; document reads retain the provider's text result.

Legacy `SoftwareSettings` management is explicitly excluded from this repair.
Role/model configuration and memory-profile binding adapters are not installed.

## Scope and outstanding work

This is not a claim that every historical plugin runs unchanged. Copying the
complete type tree preserves the upstream contract; it does not implement all of
its capabilities. The following are not yet covered by compatibility adapters:

- Removed v1 SoftwareSettings model/character/speech/sandbox management methods.
- FFmpeg and Tasker namespaces absent from the current generated Tools surface.
- Workflow engine features not supported by the prerequisite plugin (Tasker/intent/speech triggers and non-empty extract.defaultValue).
- A complete audit/conversion of every ToolPkg hook input, return value, and stream callback.
- Direct low-level `toolCall` requests using legacy environment parameters (the current Files adapters operate on `Tools.Files`).
- Exact legacy formatted `toString()` output for all result types.
- End-to-end equivalence of underlying v2 services; for example the current zip executor does not consume `include_root_directory`, even though the binding and adapter accept it.

These capabilities must be implemented or explicitly reported, never simulated
with successful empty results. Future work belongs in bounded adapter modules and
contract fixtures, not in a second loader or platform-specific Flutter code.

## Checks

```powershell
.venv/Scripts/python.exe plugins/tools/sync_v1_types.py check
tsc -p core/crates/plugin/sdk/src/compat/v1/tsconfig.json
node --test tools/tests/toolpkg_api_compatibility.test.mjs tools/tests/toolpkg_v1_adapters.test.mjs tools/tests/plugin_bridge_contracts.test.mjs
```

The TypeScript check intentionally skips checking upstream declaration bodies,
while checking adapter use of both contracts under strict mode. This isolates
upstream global declaration conventions from adapter implementation checking.
Rust archive-loading and version-admission tests are included in the SDK source;
they require a separate Rust test run and are not exercised by the Node checks.
