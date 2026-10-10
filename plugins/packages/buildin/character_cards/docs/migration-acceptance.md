# Character and memory storage migration

The unreleased plugin JSON backend is removed. Released Core v27 data and Operit1
snapshots are adopted by one removable migration boundary in Core:
`data/backup/CharacterPluginMigration.rs`. Production business operations remain in
the plugin, using generic `Tools.Storage.objects` and `Tools.Files` APIs.

`CharacterPluginLegacyData.rs` contains only the released source record layouts,
settings/profile readers and source directory encoding. These types are private to
the backup module. Core no longer exports character/memory models, repositories,
search/graph algorithms, extraction schedulers, ObjectBox writers or synchronization,
native memory tool descriptions, or dedicated SDK result and active-prompt types.
Missing optional source settings retain the defaults declared by that released schema;
malformed source fields remain errors. Reading a source USER.md never creates or
rewrites the old profile file.

## Persistence contract

The database is `characters.sqlite` under the stable Space plugin data directory.
Collections contain separate card, group, tag, shared-store, owner, memory, chunk,
link, autosave-candidate and embedding records. Metadata holds the active selection,
next decimal-string identity and migration state. Ordinary domain writes use one
atomic changed-record commit with exact-version guards. Reads use bounded pages;
operation sessions currently assemble a validated snapshot in memory.

USER.md remains an actual editable file. Writes stage a sibling before the record
commit and publish after it. Record transactions and external file publication have
separate failure boundaries. Original failures remain visible and stop subsequent
operations on the same repository instance.

## Migration and deletion sequence

1. Database 27→28 stages old card names and group IDs in a temporary extension
   namespace before removing those columns, preserving native conversation folders.
2. Startup reads the four dedicated preference stores, owner settings, USER.md and
   legacy memory SQLite tables through established store/Host structures.
3. The common writer checks references, records an in-progress marker and writes
   bounded batches through the production plugin storage dispatcher. Numeric source
   identities become exact decimal strings. Referenced legacy resources are copied
   into plugin ownership and their Files API paths are updated.
4. Each written record, document and resource is read back and verified. Completion
   is recorded only after all batches are stored. The plugin refuses incomplete data.
5. Core resolves staged bindings in both current chats and historical sync chat rows.
6. Cleanup validates every exact source path, removes migrated owner directories and
   dedicated preferences, removes empty owner parent directories, and records cleanup
   completion. Reopening continues incomplete cleanup without importing completed
   source data again. Unrelated paths and directories remain outside the cleanup set.

Released automatic memory candidates store a reply-completion history cutoff, not
the message creation timestamp. Core and Operit1 share one conversion: the final
persisted message at or before that cutoff supplies the actual assistant timestamp
and selected variant. Selected-user candidates retain exact timestamp matching.
An empty or ambiguous source range, an incorrect sender or an invalid variant is an
explicit migration error; candidates are never discarded or assigned invented versions.

The synchronization service no longer requests the retired ObjectBox domain, and
Core no longer applies its former character/memory operations.

## Evidence boundaries

| Check | What it establishes |
| --- | --- |
| Strict host/browser checks | Current sources agree with actual declarations without relaxed TypeScript settings |
| Node repository/service tests | Current plugin operations use a generic real SQLite/filesystem adapter; edits commit changed records, preserve exact identities and original errors, and survive reopen |
| Rust migration tests | The production object storage dispatcher and native Hosts execute paged adoption, verify documents/resources, reject unsafe cleanup and resume interruption |
| Rust DB migration tests | Actual 27→28 upgrades preserve records, source bindings, schema and transaction rollback at each statement boundary |
| Operit1 source tests | Real Room 10/20/21 readers preserve native folders and stage legacy associations; settings XML and schema scalar semantics are explicitly parsed |
| Package checks | Current generated bundle/source/archive bytes agree |

These checks do not establish a complete Flutter launch, live application activation,
or successful import of a user-supplied ObjectBox snapshot. Synthetic or source-only
checks must be reported separately from executed Rust Host behavior.

## Commands

```powershell
node plugins/packages/buildin/character_cards/scripts/check.mjs
node --test plugins/packages/buildin/character_cards/tests/storage-persistence.test.mjs
node --test plugins/packages/buildin/character_cards/tests/operit1-snapshot.test.mjs
cargo test -p operit-runtime CharacterPluginMigration --lib --manifest-path core/Cargo.toml
cargo test -p operit-store migration_tests --lib --manifest-path core/Cargo.toml
node plugins/packages/buildin/character_cards/scripts/build.mjs
git diff --check
```
