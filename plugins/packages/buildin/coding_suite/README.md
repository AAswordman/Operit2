# Coding Suite

Enable `com.operit.coding_suite` in plugin management to use the coding environment.
It is installed as a built-in plugin but disabled by default so ordinary character
conversations keep their existing configuration. Activation applies to the current
plugin installation scope, not just one conversation.

While enabled, its manifest `suspends` declaration excludes
`com.operit.character_cards` from the executable plugin graph. Required dependents,
such as Waifu Mode and Voice Call, are excluded transitively. Their hooks, public
APIs, background clocks, UI contributions and subpackage tools are unavailable.
Stored role data and requested enable settings are preserved. Disabling Coding
Suite removes this exclusion; it does not enable previously disabled plugins.

Open **Coding Suite** in the toolbox or plugin sidebar to edit its plain prompt,
enable/disable background memory, inspect failures, or retry failed summaries.
The prompt is independent of character-card fields.

The editable default prompt introduces the Operit agent and guides it to inspect
existing code, solve the task's root cause, and preserve user edits. Implementation
and verification share the user's requested scope: choose the smallest check that
directly establishes the required behavior, use existing relevant tests, and stop
when those checks pass. A failed scratch test is diagnosed before changing product
behavior. Hypothetical scenarios do not justify new compatibility promises, larger
test suites, repeated demonstrations, or additional execution environments.
It can be cleared. Package usage is described by the host system prompt;
platform and path guidance belongs to the terminal package's advice.
Workspace instructions and the optional timeline locator
are separate dynamic sections. When all coding context is empty, the plugin adds
no system message. Saved custom text is preserved.

For each chat request and independent token estimate, the plugin reads the current
workspace root `AGENTS.md` through VFS and includes its current content. A missing
file contributes no project instructions. Read failures are reported through the
prompt hook error mechanism; an inaccessible file is not treated as missing.
Nested directory instructions remain the coding agent's responsibility when it
works in those directories. The plugin reads the exact `AGENTS.md` filename.

## Project timeline

The sole human-readable memory file is `.operit/coding-memory.md`, relative to the
bound workspace. Conversations in the same workspace share it. An unbound
conversation uses the host's actual persistent conversation output directory,
including inherited parent/branch output bindings. A workspace switch changes
the destination for subsequent replies; already queued jobs keep their original
destination.

When the memory file exists and memory is enabled, the main prompt contains its
locator and instructions to search it when past context is relevant. A missing
file contributes no timeline section. Its contents are never automatically recalled or loaded
into the foreground request. The agent can search it with `rg` or file grep tools.

After an assistant reply is persisted, the plugin enqueues its exact timestamp and
selected revision. It extracts the current exchange from at most 12 recent
messages and 24,000 characters, without calling a model on the persistence path.
The Host's one-minute clock processes one queued job at a time using the configured
`MEMORY` functional model with token accounting enabled. The summary records
decisions, changed paths, actual checks, outcomes and remaining work in the user's
language. This is separate from conversation context compaction.

Entries contain the source timestamp, conversation ID and revision. Each write
appends; existing content is never rewritten or compacted. A completed entry's
receipt in that same file prevents duplicate appends after interrupted queue
checkpointing. Generated summaries are saved before file append, so retrying a
failed append reuses the paid result. A failed job remains visible in settings and
waits for explicit retry. Pending jobs and settings survive a restart; no separate
vector database or recall index is created. The queue is plugin-owned operational
state, not an additional memory corpus.

The writer serializes within the executing Core. Concurrent offline writers on
different devices are governed by the project's existing file sync behavior;
this plugin does not provide distributed append transactions.
