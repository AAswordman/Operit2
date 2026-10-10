# Character cards and their memories

## Status: extraction in progress, not a production replacement

The package owns character cards and memory in one ToolPkg
(`com.operit.character_cards`). Memory stays inside the character workflow.
The offline Material UI is present, but passing Node or browser tests does **not**
mean the application is connected or the original feature has been fully extracted.

## Compose route entries

Each of the seven UI routes has its own default-exported entry at
`src/ui/<route-id>/index.ui.ts`, built as `dist/ui/<route-id>/index.ui.js`.
The main bundle keeps these imports external and does not re-export the screen
functions: the SDK serializes a screen's module path, not its named export, and
root export tagging would otherwise overwrite the independently loaded module
identity. The build includes all seven executable screen modules in the `.toolpkg`
archive alongside the main provider and memory tools.

The route regression tests execute the actual JavaScript registration and Compose
SDK scripts, check the serialized paths and render each entry with explicit input.
Installed module bytes and the Core asset archive are compared against the current
build. The embedded chat sidebar uses native Compose DSL rows, search, group menus
and dialogs; it does not instantiate a WebView. The obsolete browser sidebar source, resource
registration and packaging entry have been removed. Role categories contain their
conversation groups, so this package contributes only the legacy Characters tab;
the native Workspace tab remains owned by Flutter. These are Node/script checks, not a live native-host acceptance result.

## Chat input menu

The package registers one `chat_input_menu` navigation entry pointing to
`src/ui/chat-input-menu/index.ui.ts`. Flutter embeds that Compose DSL module
above its native statistics section. The TS renderer owns the avatar, current
character label, name, spacing and click action; the public chat-context API
contains only identity and background data.

The row emits `{type: "toolpkg.ui.present", routeId, input}` to open the existing
registered selector. The host validates the requesting package and route, keeps
opaque input unchanged, and uses its existing chat lifecycle guard and
presentation completion refresh. Character menu fields are drawn by TS.

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

## Storage and released-data migration

`Tools.Storage.objects` owns the plugin's `characters.sqlite` database in the stable
Space data directory returned by `ToolPkg.getSpaceDataDir()`. Each domain record has
its own collection/key and exact version; commits publish changed records, not an
entire JSON library. Current operation sessions still load paged records into a
validated domain snapshot. USER.md remains a real file for editing and attachment.
Its staged publication and the SQLite transaction have separate failure boundaries.

The unreleased plugin's former `state.json` format is removed without a compatibility
reader. Released application data takes a distinct migration path: Core's centralized
`data/backup/CharacterPluginMigration.rs` reads old preferences and SQLite records,
uses the production generic Host object store, verifies the copied documents and
resources, resolves saved chat associations, and deletes migrated legacy owners and
the four dedicated preference files. An interrupted migration blocks plugin reads;
completed imports and cleanup have durable markers.

Operit1 snapshot restore uses the same writer directly. It adopts characters,
groups, prompt tags, memory profiles, USER.md, memories, properties, document chunks,
links, pending extraction tasks and declared memory preferences. Source Room schemas
remain explicit; chat name/group bindings resolve to plugin selection markers before
Core chat import. Historical message send profiles are not reconstructed. Derived
binary embedding vectors are not imported into the plugin's endpoint/model/text cache.
See `docs/migration-acceptance.md` and `docs/operit1-restore-boundary.md` for evidence.

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

Strict host/browser TypeScript checks run through `scripts/check.mjs`. Repository
and service tests use current plugin code and a generic Node SQLite/file Host adapter;
that adapter implements storage semantics rather than character business operations.
It is distinct from production Rust Host integration.

Rust migration tests use the production `PluginStorageSession`, SQLite and native
storage Hosts. Database migration tests exercise actual schema upgrades, every
transaction failure boundary and complete persisted rows. Operit1 tests cover real
Room source databases and structured settings; they do not constitute a complete
application launch or a full import of a user-supplied ObjectBox archive.

Production artifacts are generated by `scripts/build.mjs`; packaging checks compare
current source, installed bundle and Core archive bytes. Flutter UI and application
startup require their own integration acceptance.

## Public dependency client

`manifest.json` publishes the self-contained client `src/api.ts`. Consumers
copy that file into their own source tree, import it locally and declare
`com.operit.character_cards >= 0.1.0` in `requires`, following
`plugins/docs/PUBLIC_API.md`. Public calls use `ToolPkg.callDependency`.

`characterCards.memory.query({ participantId, query, limit, snapshotId })`
resolves the explicit participant's actual memory binding and reuses the
plugin's existing query implementation. Results contain complete typed
matches and document chunks; snapshots remain in the provider's main runtime.
A `null` snapshot ID requests a fresh query. An explicit snapshot ID retains
owner-scoped deduplication across calls. The extra information injection
plugin consumes this method through an exact local SDK copy.
