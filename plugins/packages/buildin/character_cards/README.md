# Character cards and their memories

## Status: extraction in progress, not a production replacement

The package owns character cards and memory in one ToolPkg
(`com.operit.character_cards`). Memory stays inside the character workflow.
The offline Material UI is present, but passing Node or browser tests does **not**
mean the application is connected or the original feature has been fully extracted.

## Compose route entries

Each of the five UI routes has its own default-exported entry at
`src/ui/<route-id>/index.ui.ts`, built as `dist/ui/<route-id>/index.ui.js`.
The main bundle keeps these imports external and does not re-export the screen
functions: the SDK serializes a screen's module path, not its named export, and
root export tagging would otherwise overwrite the independently loaded module
identity. The build includes all five executable screen modules in the `.toolpkg`
archive alongside the main provider and memory tools.

The route regression tests execute the actual JavaScript registration and Compose
SDK scripts, check the serialized paths and render each entry with explicit input.
Installed module bytes and the Core asset archive are compared against the current
build. The embedded chat sidebar uses native Compose DSL rows, search, group menus
and dialogs; it does not instantiate a WebView. The obsolete browser sidebar source, resource
registration and packaging entry have been removed. Role categories contain their
conversation groups, so this package contributes only the legacy Characters tab;
the native Workspace tab remains owned by Flutter. These are Node/script checks, not a live native-host acceptance result.

## Character settings scope

The plugin-menu character settings follow the native settings page immediately
before commit `824dcb7c`: character/group lists; basic, content and binding editor
tabs; model/TTS/memory/tool bindings; and existing import/export operations.
The character page's entire inner title bar (icon, title and refresh action) is
removed; the character/group section titles remain. Character and group editors
retain the independent theme reference picker. Extra group preview/duplicate edit
buttons, member reordering actions and the implementation notice remain removed.
Group rows show member names as their subtitle and only a member-count badge.
Theme references remain editable and are preserved by persistence/interchange.
Chat identity previews and the independent memory page are unchanged surfaces.

## Current Chat integration review

The current send/control contract is documented in `docs/chat-send-api.md` at the
repository root: one non-streaming send, one semantic async iterable, and one
owner/chat cancellation method. The group transport no longer exposes native
execution handles or requires author-owned native finalization. Generic SDK
lifecycle/settings/JSON declarations now match the current role-plugin source.

Strict host/browser no-emit checks, explicit architecture checks and routing checks
pass. The direct group transport tests use controlled native receipts and frozen
message snapshots; they do not establish a live Core execution lease or configured
AI response. The existing group fixture was brought onto the actual two-argument
controller factory and its explicit local planning lifecycle.

The main registration does **not** yet mount `GroupExecutionController` in a chat
input-submit hook. Installed production archives also remain different from
current source until packaging is explicitly performed. Neither gap is described
as completed integration.

## Data policy: new plugin-owned files only

**The plugin does not adopt or migrate old user character and memory data.** The
plugin must not read old Core managers, preference stores, or SQLite memory tables
to populate its library. Lossless legacy migration is not an acceptance requirement.
Explicit character interchange/import/export is a separate feature; it does not
mean that old application data is imported automatically or remains compatible.

This does **not** remove support for reading a complete Operit1 snapshot. A legal
snapshot may contain old cards, groups and memory. The existing full-snapshot
reader must accept those source entries; restoration deliberately declines to
adopt them into Core or this plugin while keeping the general model/TTS/chat/
workspace import paths. Imported chats and messages remain explicitly unassociated
(empty plugin extension namespaces), not secretly bound to a default card. Zero
adopted-memory counters do not mean the source contained no memory. See
`docs/operit1-restore-boundary.md` for the exact policy, current SQLite query checks,
and the **unexecuted Rust/full-import verification boundary**.

The plugin's own directory files are the sole authoritative source for its cards,
groups, tags, active prompt, shared stores, owner-scoped memories, document/chunk
records, links, settings/search configuration, and USER.md. The existing reusable
host file API supplies IO; domain paths, record validation, and persistence logic
belong to the plugin. No new storage or provider framework is approved by this
acceptance document.

First initialization of a genuinely new directory must be tested separately from
opening existing files. Malformed files, invalid records, inaccessible files, and
failed writes must report explicit errors, never recreate or replace an existing
library as if it were a successful fresh start. A card and its staged tag changes
must publish as one commit. Reopening the same directory in a new runtime must read
back saved data, not a process-local cache or fixture.

## Ownership boundary

- The plugin owns domain models, validation, complete-record writes, staged tags,
  serialization/interchange, commands, public APIs, and editing requests.
- `registerCoreCommand` registers command **providers**, not callers of the old
  CLI executor. Public APIs and Web IPC invoke the same plugin service.
- Core owns reusable command discovery/readiness, routing, IPC, and host IO. It
  must not acquire dedicated character/memory Host services, SDK DTOs, globals,
  runtime adapters, dedicated domain import services, or unapproved domain frameworks.
- Old mounted manager consumers, chat ownership policy, memory background tasks,
  AI send-path memory logic, and builtin memory executors remain extraction
  failures even when the five command roots have been registered.
- Unknown/disabled providers and real dependency failures must return explicit
  errors. No success-shaped empty results, production test fixtures, browser-local
  database, CLI business bridge, or alternate transport are accepted.

## Browser source and packaging

Hand-written browser source is strict TypeScript under `web/`. esbuild produces a
browser IIFE inside the offline HTML; generated JavaScript is a packaging artifact,
not a second source implementation. `tsconfig.web.json` checks browser DOM code
separately from the non-DOM host project. UI tests build current TS source with the
build worker's document builder instead of reading stale generated resources.
The archive must include every current nested `src/` and `web/` source module.
The Web/build worker owns `scripts/build.mjs` and the archive.

## Verification and evidence limits

- Pure domain/codec tests verify current complete-record fields, decimal-string
  IDs, owner grammar, staged changes, interchange formats, and explicit errors.
  They do not establish file persistence or application integration.
- Registry and static entrypoint checks inspect actual registrations, SDK
  declarations, and the command/API/IPC service call graph. Capturing definitions
  is not live registration or a production UI launch.
- Isolated browser fixtures verify only Material view behavior, request/response
  shapes, error states, and actual SVG click/pan/drag gestures. Their finite import
  response is not a working importer or repository.
- A temporary-directory disk harness replaces **only file IO** with real Node
  filesystem operations. Repository tests must execute the actual plugin repository
  and service. Harness self-tests alone are not repository acceptance, and even a
  passing repository test is **not host-runtime integration**.
- Architecture checks fail on dedicated production dependencies and explicit old
  mounted paths, including EnhancedAIService, MemoryLibrary, and the autosave
  scheduler. Retained unmounted reference files are not blanket violations.
- `check.mjs` performs strict no-emit host/browser checking with actual declarations,
  test type contracts, architecture checks, and the routing owner's static checker.
  It does not hide SDK imports or ID-contract mismatches.

No Rust or Flutter compilation, live package activation, actual host filesystem
execution, scheduler lifecycle, or cross-platform UI integration has been verified
by this worker. The production file backend and its entrypoint wiring still need
real acceptance; an unfinished implementation is never replaced by a successful
mock Host or store.

## Remaining acceptance

See `docs/migration-acceptance.md` for the file-backend acceptance matrix. The
filename describes feature extraction, **not old-user data migration**. Open items
include actual disk initialization/reopen/corruption tests, single-commit card/tag
writes, owner/document/link/settings/USER.md persistence, production menu/sidebar/
popup/chat binding/attachment routes, and removal of mounted old domain consumers.
Resource selection, disabled owner controls, rebuilds, backups, document workflows,
and categorization must be reported as gaps until their actual path is supported.
Do not describe this package as usable, 1:1, or complete while these remain open.
`docs/mounted-theme-workspace-follow-up.md` records separately confirmed live theme/
workspace manager consumers; a passing sidebar checker does not cover these chains.

## Repeatable checks

From the repository root, without Rust or Flutter compilation:

```powershell
node --test plugins/packages/buildin/character_cards/tests/*.test.mjs
node plugins/packages/buildin/character_cards/scripts/check.mjs
node core/crates/command/core/scripts/check-routing.mjs
git diff --check
```

For a browser-only strict check, use `node plugins/packages/buildin/character_cards/scripts/check.mjs --web`.
It must not be presented as a passing full host/architecture check.
