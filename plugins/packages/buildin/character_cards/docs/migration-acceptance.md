# Plugin-owned character and memory file acceptance

## Decision and scope

**Existing users' character/memory data is not compatible and is not migrated.**
The plugin directory's files are the only authoritative character/memory source.
Legacy-data compatibility, int64 migration, old preference import, and old SQLite
migration are deliberately unsupported, not completed work or pending release gates.
Manual interchange/import/export is evaluated against the current plugin contract.

This document does not approve a new SDK, storage wrapper, or provider framework.
It records evidence required from the actual implementation. Node IO adaptation,
browser fixtures, source checks, declarations, or unexecuted Rust tests must never
be described as live application or host integration.

## Acceptance matrix

| Area | Required evidence | Current evidence boundary |
| --- | --- | --- |
| First initialization | Actual repository opens a real new temporary plugin directory and persists its declared initial records | Pending production file repository |
| Restart/reopen | A newly loaded service/runtime reads saved files from the same directory, including identities and edited fields | Pending actual disk test |
| Bad files | Corrupt JSON, wrong schema, and malformed records explicitly fail without overwriting bytes or returning an empty library | Pending actual disk test |
| IO errors | Read/write/publish failures propagate, preserve the authoritative file, and do not publish partial mutations | Pending actual disk test; negative dependency tests are not persistence |
| Card and tags | All staged creates/updates/deletes and card bindings publish once; failed commit publishes neither | Pending actual service/file commit test |
| Ownership | Character and shared namespaces remain isolated; unknown owners fail without creating another store | Pure grammar covered; actual disk ownership pending |
| Complete records | Current card/group/tag/store/memory/document/chunk/link fields survive writes and reopen | Pure record/codec checks only |
| Documents and links | Chunk content/ordering, document paths, UUID endpoints, types, weights, and relationship descriptions persist | Browser/pure shapes only; actual disk pending |
| Settings and USER.md | Complete owner settings/search weights and profile text persist independently and reopen correctly | Browser request scope only; actual disk pending |
| Same service | Registered command/API/Web IPC mutations and reads share one actual file-backed service | Static imports checked; real entrypoints pending |
| UI production path | Real menu/sidebar/popup/chat binding/attachment entry opens current typed views and saves through that service | Isolated views and route captures are not production launch |
| Provider errors | Unknown/disabled providers and dependency failures produce explicit actual-path errors | Routing static/catalog cases only; no application runtime execution |
| Extraction | Named mounted managers, chat ownership/background policy, AI memory consumers, and builtin tools no longer own this domain | Failing architecture assertions remain visible |
| Legacy user data | No automatic reads/import of existing Core character or memory data | Unsupported and not migrated by decision |

## Explicit mounted-path audit

Checks follow named `mod` declarations and real calls rather than treating the
entire repository's retained source as active. Required inspected paths include:

- ProviderRuntimeSupportService and ToolRuntimeSupportService manager consumption;
- exported provider/tool runtime domain traits and DTOs;
- OperitApplication startup and MemoryManagementService background controls;
- ChatServiceCore and MessageCoordinationDelegate manager/owner consumers;
- ChatMemoryOwnerResolver;
- mounted provider EnhancedAIService character prompt/USER.md/autosave calls;
- mounted MemoryLibrary repository/search policy and MemoryAutoSaveScheduler calls;
- StandardMemoryTools and ToolRegistration public/internal builtin executors;
- plugin canonical dedicated SDK/global dependencies and all command/API/IPC imports.

## Disk-harness boundary

Use a real temporary directory and the real plugin repository. Adapt only existing
host file calls to Node fs, confine every path to that directory, inspect actual
file bytes, and reopen a new runtime. Do not implement domain records, first-start
policy, commits, owner resolution, or caches in the harness. Predetermined browser
responses belong only to isolated UI tests. Successful harness IO alone is not a
passing repository or host integration test.

## Reproduction and reporting

From the repository root:

```powershell
node --test plugins/packages/buildin/character_cards/tests/*.test.mjs
node plugins/packages/buildin/character_cards/scripts/check.mjs
node core/crates/command/core/scripts/check-routing.mjs
git diff --check
```

Report test counts including failures/skips, strict host and browser results
separately, and exact failure paths. No Rust/Flutter compilation or live host
persistence is claimed. Do not erase architecture failures to make pure UI green.
