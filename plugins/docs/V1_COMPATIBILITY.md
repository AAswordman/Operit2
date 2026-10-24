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

Omitted `api_version` selects the original ToolPkg contract (`1.0.0`), including
its compatibility adapters and effective runtime prerequisites. Explicit `1.0.0`,
`1.0.1`, and `2.0.0` declarations retain their exact versions. New Operit2 packages
must declare `2.0.0`; versions are not inferred from filenames or source text.
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

### Global object alias

The shared engine bootstrap exposes `global` as the same object as `globalThis`
and `window` for both v1 and v2 plugins. Modules and timer callbacks can share
state through any of these names. The v1 declaration records this explicit
Operit2 runtime extension. It does not install Node.js modules or APIs.

### SoftwareSettings environment variables and V2 domain resources

The historical V1 SoftwareSettings declarations remain unchanged.
`readEnvironmentVariable` and
`writeEnvironmentVariable` retain their original signatures through an explicitly
versioned adapter for 1.0.0 and 1.0.1. JSDoc checks both the historical signatures
and the canonical V2 host calls. The adapter preserves complete host results and
original failures.

V2 exposes configured resources through `Tools.Models`, `Tools.Tts`, `Tools.Theme`
and `Tools.Catalog`. The seven domain methods are available since 2.0.0 and use
fully qualified native dispatch paths, including separate `Tts.listConfigs` and
`Theme.listConfigs` paths. These methods were never part of the copied V1
`SoftwareSettings` contract, so no historical aliases are installed for them.

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

The conditional `v1_runtime` package executes the supported Linux file operations
in the same terminal filesystem used by legacy commands. Other VFS addresses are
handled by the existing host/VFS implementation. The adapter does not create
mounts, emulate Android/Linux filesystems, or bypass
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

### Terminal and runtime script packages

`compat/v1/runtime.js` installs after the public structured `toolCall` bridge.
It uses the existing per-call API dispatcher and the built-in `v1_runtime` script
package. Rust selects the package state using its host condition capabilities;
`platform.ohos` is exposed alongside the other platform booleans.

| Package state | Legacy terminal | Node tool | Package commands |
| --- | --- | --- | --- |
| Windows | Existing Git Bash host (`bash`) | Existing PowerShell host | Native; no APK shim |
| Linux | Native Bash | Native Bash | Native; no APK shim |
| macOS | Native Bash | Native Bash | Native; no APK shim |
| Android | Existing proot Bash | Existing proot Bash | Explicit apt/apt-get to APK mapping |
| iOS | Existing iSH shell | Existing iSH shell | Explicit apt/apt-get to APK mapping |
| OHOS | Existing QEMU-vroot shell | Existing QEMU-vroot shell | Explicit apt/apt-get to APK mapping |
| Web | Existing Buildroot V86 shell | Existing Buildroot V86 shell | Native; no APK shim |

For API 1.0.0 and 1.0.1, `Tools.System.terminal.create/exec` dispatch to that
package. These exact historical tool identities also dispatch there:

- `super_admin:terminal`
- `code_runner:run_javascript_node`
- `ffmpeg:ffmpeg_execute`
- `ffmpeg:ffmpeg_info`

The existing v1 runtime adapter also translates the executing container's old
`/data/user/0/com.ai.assistance.operit/files/toolpkg_cache/<package>-*` namespace
(and the `/data/data` spelling). Concrete hexadecimal cache suffixes are accepted.
It reads the already extracted `/app/data/cache/toolpkg` tree through canonical
Files APIs and imports it through `v1_runtime:file` into the terminal's existing
`/tmp/operit-v1/toolpkg` directory. File calls using these legacy addresses and
terminal commands then access that same copy. No host bindings or mounts are added.

Translation replaces only that container's cache prefix with a shell-safe concrete
prefix. It keeps shell quotes, nested `bash -lc` quoting, and suffix globs intact;
the same namespace is translated when it appears in command text or diagnostics.
It does not provide a general VFS mount or resolve arbitrary shell expressions.
The host's existing `.toolpkg-cache-signature` avoids repeated imports across SDK
executions; it is committed after a successful copy. Concurrent first calls in one
SDK execution share an import and failed copies can retry. Revisions are refreshed
when the SDK execution is recreated. The terminal copy is a cache, not a durable
plugin data directory, and is not synchronized back to the host cache.

An empty session ID uses the existing caller/chat-scoped default compatibility
session. Missing or malformed IDs still fail. The undocumented synchronous
`Tools.System.file` object used by some plugins is not emulated by these async
Files adapters. Installed Node, SQLite support, background execution, and iOS
WebView integration still require validation in the real host environment.

The declared v1 `terminal.execStreaming(sessionId, command, options)` now uses
live output from that same compatibility session. The SDK keeps
`onIntermediateResult` in JavaScript and sends only the command and timeout
through the structured JSON bridge. It retains the existing cache-path
translation, selected interpreter, 15-second default, start/chunk event metadata,
and complete result. A callback exception closes the stream and interrupts the
current command; the session remains available. No temporary output file is used.

API 2.0.0 also exposes the same execution stream as an async iterable:

```ts
for await (const event of Tools.System.terminal.execStream({
  sessionId,
  command: "printf 'starting\\n'; sleep 1; printf 'done\\n'",
  timeoutMs: 5000,
})) {
  if (event.type === "chunk") console.log(event.chunk);
  if (event.type === "completed") console.log(event.result.exitCode);
}
```

A stream opens on its first pull and permits one pending `next()` at a time.
Breaking out of the loop or calling `return()` interrupts the command and keeps
the terminal session. The `completed` event contains the normal full command
result, including exit code and timeout status. Chunks contain raw terminal text,
including echoed input and control sequences, with private completion markers
removed and split UTF-8 buffered. Their concatenation can differ from the
normalized final output. `receivedChars` counts JavaScript UTF-16 code units.
The callback-style `execStreaming` remains available in v2 as well.

Native observations have bounded queues and worker counts. Output over the
queue limit or 16 MiB total fails explicitly and interrupts the command. The
native PTY hosts and iOS iSH bridge implement live delivery. Browser V86 is not
yet connected to this streaming host interface and reports unsupported execution.

API 2.0.0 retains canonical host calls and tool identities. Native host errors,
nonzero exit codes and timeouts are retained. Program helpers reject unsuccessful
execution. Node source uses UTF-8 hex encoding before native invocation because
Windows PowerShell 5.1 strips embedded double quotes in native arguments.

Alpine states install command shims in the existing home directory. The supported
apt grammar is `update`, `install`, `remove`, and `purge`, with explicit options
and package-name mappings, including `python3-pip` to `py3-pip`,
`build-essential` to `build-base`, and `libssl-dev` to `openssl-dev`. The shims
execute real APK commands; unsupported options/packages are errors. Native
Linux/Windows commands and Web Buildroot never run this APK shim.

The managed-filesystem bridge covers text/binary read/write, existence checks,
mkdir, delete, move, copy and unzip. Cross-environment file copies transfer
actual Base64 bytes through the canonical host filesystem and the selected
terminal filesystem. Writes are bounded to avoid native argument limits and
preserve Unicode surrogate pairs. ZIP extraction rejects escaping entries and
symbolic links. Directory creation follows the v1 declarations:
`Tools.Files.mkdir(path, create_parents?, environment?)`, or the declared
low-level `toolCall('make_directory', params)` tool. `Tools.Files.makeDirectory`
and synchronous `Tools.System.file` are not declared v1 interfaces and are not
added as package-specific aliases.

Required executables remain real host requirements: Windows legacy shell tools
need the existing Git Bash host; managed file operations need Python, Node tools
need Node, and FFmpeg tools need FFmpeg. This layer does not translate arbitrary
Bash source into PowerShell, add Ubuntu, install new runtime images, or reproduce
Linux `/proc` on Windows. Hardcoded `/root` paths remain subject to native write
permissions. Web Buildroot has no Bash or dynamic npm/pip package installation.

### Marketplace archive audit (2026-10-19)

The downloaded marketplace archives for TA's pocket phone 1.5.2, questionnaire
0.1.0, enhanced questionnaire 1.8.0, and WeChat iLink bridge 2.5.3 all omit
`api_version`, so their loading contract is 1.0.0. Actual entry registration,
initial Compose rendering, and the original questionnaire submit callback were
checked using the production dispatcher and registration scripts for both
1.0.0 and 1.0.1. These checks do not establish full host execution equivalence.

The pocket phone uses the terminal and explicit Linux Files APIs covered above.
The iLink bridge uses the legacy terminal, Node and FFmpeg tool identities now
routed by this adapter. Its old Android installed-package search directory and
`/proc` process scanning remain package-specific portability issues. The
enhanced questionnaire also uses an Android Java clipboard API. A separately
audited two-person phone archive 1.1.4 is missing its registered
`ui/phone.ui.js`; runtime adaptation cannot restore an absent archive member.

### Additional marketplace samples (2026-10-21)

Six more unmodified market archives were inspected: BetterFileTools 1.0.0,
Chat History Converter 1.0.1, Minesweeper 1.0.0, Standalone Multi Search 1.0.4,
Quiz Bank Manager 1.6.1 and Messenger 0.3.0. All omit `api_version` and therefore
select API 1.0.0, regardless of the archive's own version. Their original entry
functions registered successfully through the production version dispatcher and
registration bridge, and each registered initial screen rendered through the
production Compose compiler/runtime in a Node VM.

Additional business checks used the original package functions with explicit
host fixtures, without modifying the archives:

- BetterFileTools created a directory and file, then moved the actual file bytes
  into its recycle bin and wrote the matching record through the production v1
  Files adapter backed by temporary local files.
- Chat History Converter read a sample ChatGPT export and wrote an Operit archive
  containing one conversation and two messages through that same Files adapter.
- Minesweeper's original serialized start-button callback created a 9x9 board
  with ten mines. Its registered game handlers also passed flag/unflag/reveal
  checks. IPC routing in this check was an in-process fixture.
- Quiz Bank Manager created and listed a real temporary bank. Its delete call
  removed the index entry but left the bank JSON file behind: the original code
  only probes undeclared `Files.delete/remove/unlink` aliases, never the declared
  `Files.deleteFile`. This is a package contract issue, not a reason to add
  those aliases to the engine.

The Quiz Bank Manager also probes undeclared clipboard and shell aliases behind
feature checks. Missing guarded alternatives alone do not establish that its
other features fail. Messenger depends on Android Java SharedPreferences for
settings; successful initial rendering does not verify persistence or cross-chat
forwarding on other platforms. Multi Search's original subpackage loaded with
the production OkHttp wrapper, but remote searches and credentials were not
exercised. No mobile host, platform WebView or external service was simulated as
successful. These checks establish only the paths listed above.

The v1 chat adapter also routes declared sender/control options that structured
streaming cannot express through the existing native `send_message_to_ai` tool.
It preserves the real result and host errors and delivers the completed response
as one callback chunk. This fallback does not provide incremental token delivery;
`waifu` sentence aggregation remains unsupported.

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
node --test tools/tests/toolpkg_api_compatibility.test.mjs tools/tests/toolpkg_v1_adapters.test.mjs tools/tests/toolpkg_v1_runtime.test.mjs tools/tests/plugin_bridge_contracts.test.mjs
```

The TypeScript check intentionally skips checking upstream declaration bodies,
while checking adapter use of both contracts under strict mode. This isolates
upstream global declaration conventions from adapter implementation checking.
Rust archive-loading and version-admission tests are included in the SDK source;
they require a separate Rust test run and are not exercised by the Node checks.

The runtime compatibility tests additionally execute Node through real Windows
PowerShell/Git Bash, test APK argument mapping with an argv-recording executable,
and exercise Python file operations and bounded resource writes in temporary
directories. They do not boot Android, iOS, OHOS or V86 runtimes.
