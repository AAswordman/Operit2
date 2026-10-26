# Plugin state during send preparation

A retained JavaScript realm owns plugin modules and service state. Prompt hooks
use that lifetime so ordinary reads do not recreate directories, check default
files or repeat storage migrations.

## Worldbook storage

`initializeWorldBookStorage` opens the package configuration root through one
shared Promise, used by `application_on_create` and UI/tool entry points. It
creates the entries, groups and variables files and performs migration once.
The ready state retains paths. A changed root opens a new owner; a failed open
remains failed for that owner and root.

## Per-send snapshots

Rust creates `metadata.promptPreparationId` for each send preparation. System
composition and prompt finalization share that internal ID; it is absent from
stored messages and model prompts. Independent estimation uses a separate
runtime.

Worldbook reads entries, variables and chat history once for a preparation. The
system hook retains that snapshot and the final hook consumes it. The snapshot
also records chat and participant identity to prevent cross-context reuse.
Abandoned snapshots are bounded to 32. Each new send reads current backing files,
so local and synchronized changes affect the next preparation. Variable writes
are serialized within the owner to protect simultaneous chats.

Prompt registration selects a hook family. Registered callbacks receive its
stages and apply their JavaScript stage guard; storage sharing does not change
hook dispatch or plugin lifecycle.

## Completed reply context

Completed-response estimation uses the execution's actual history, final prompt
transformations, tool rounds and latest provider tools. It reuses the provider's
token cache without creating another send runtime or invoking preparation hooks.

Normal completion borrows the history containing its assistant turn. A
thinking-only completion appends its pending turn once; empty completion adds no
synthetic turn. Only the scalar completed context window survives closure for
message commit. A new turn or cancellation clears it, and a failed estimate
produces no completed value.

Explicit preview and estimate APIs still prepare their own current data. Route
changes obtain persisted history only when creating a resume context.

## Maintenance checks

From the repository root:

```sh
node --test tools/tests/worldbook_lifecycle.test.mjs tools/tests/worldbook_plugin_boundary.test.mjs
```

The native integration fixture is
[`plugin_send_performance_test.dart`](../../apps/flutter/app/integration_test/plugin_send_performance_test.dart).
It uses production JavaScript, host tools and SQLite with isolated storage and a
local SSE model endpoint. It checks warm-send I/O counts and rejects extra plugin
or host work between model completion and message commit. Flutter commands use
FVM from `apps/flutter/app`.
