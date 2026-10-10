# Operit1 snapshot import into plugin storage

The existing snapshot reader, protobuf preference decoder, explicit Room schema
bridges and streaming LMDB reader remain the source boundary. Domain adoption uses
`Operit1CharacterPluginMigration` and the common `CharacterPluginWriter`, rather than
restoring the removed Core character/memory managers or writing intermediate legacy
Core preferences.

## Adopted data

| Source | Plugin/Core destination |
| --- | --- |
| Character cards, groups and tags | Individual plugin collections, original identities and settings |
| Memory-space metadata | Shared plugin stores and character/shared owner metadata |
| Per-space user.md | The corresponding real plugin USER.md files |
| MemorySearchSettings SharedPreferences XML | Autosave interval, scheduling, extraction rules and search weights |
| CloudEmbeddingSettings SharedPreferences XML | Enabled state, endpoint, model and API key |
| Profile update flags | Owner memory settings |
| ObjectBox memories | Complete memories with original timestamps, UUIDs, tags and properties |
| ObjectBox links | Original identities, exact endpoints, types, weights and descriptions |
| ObjectBox document chunks | Original identity, content and order, bound by memory UUID |
| ObjectBox autosave candidates | Original task status and counters, with selected variant resolved from actual imported messages |
| Native conversation folder | Core chat group membership |
| Card name/group association | Resolved plugin selection marker on the imported Core chat |
| Models, speech, general preferences, chats, token statistics and workspaces | Existing general import paths |

Existing snapshot resources are rewritten to public runtime paths by the established
file plan. Declared ObjectBox and memory-space profile directories are consumed by
migration and excluded from general resource copying. A workspace/external USER.md
is still an ordinary resource; its filename does not classify it as legacy memory.

Room 10 has no group-association or pinned source field. Room 20/21 preserve both.
Each source bridge targets Core SQLite 28 without changing the source version.
Messages and variants retain their historical text and metadata; the importer does
not construct historical frozen send profiles. Binary embedding vectors are derived
cache data without the new cache's endpoint/model/text identity and are not adopted.

## Completion and failures

The import key uses the snapshot package identity and creation timestamp. The writer
records incomplete migration before bounded writes, verifies records/documents and
marks completion. Plugin reads reject incomplete migration. Reimporting a completed
source does not overwrite subsequent plugin edits. Memory and link result counters
represent adopted source records.

Invalid source settings, damaged tables, missing relationships and ambiguous role
names are explicit errors. Migration does not manufacture replacement records. The
source snapshot itself is retained; old Core owner directories are handled by the
separate released-Core cleanup step.

## Verification limits

Node source-contract tests execute actual production SELECT statements against real
Room 10/20/21 SQLite fixtures and inspect the restore call graph. Rust source-reader
and settings tests execute the actual projection/parsing functions. The common writer
has separate production native Host tests. These are distinct from a complete live
application import of a user-provided LMDB archive.
