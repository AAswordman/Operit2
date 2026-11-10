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
| ObjectBox autosave candidates | Original task status and counters, exact imported message revision, and the importing CoreNode as producer |
| Native conversation folder | Core chat group membership |
| Card name/group association | Resolved plugin selection marker on the imported Core chat |
| API configurations and function models | All providers, credentials and model-index bindings, including unused providers without models |
| Fixed character models | The corresponding imported provider and model together |
| Speech, general preferences, chats, token statistics and workspaces | Existing general import paths |

Existing snapshot resources are rewritten to public runtime paths by the established
file plan. Declared ObjectBox and memory-space profile directories are consumed by
migration and excluded from general resource copying. A workspace/external USER.md
is still an ordinary resource; its filename does not classify it as legacy memory.

The source `memory_space_list` defines the memory spaces to adopt, as in Operit1's
Kotlin preferences manager. Unlisted ObjectBox directories can remain in a raw
snapshot; they neither create stores nor invalidate the listed spaces and bindings.

Importing ordinary appearance clears the target's active saved-theme selection in
the same preference commit. Existing saved themes retain their own snapshots.
Imported extraction tasks belong to the importing CoreNode, so the scheduler can
process them without classifying them as released tasks with unknown provenance.
Candidates already failed by a previous import can be explicitly retried through
the existing memory settings action, which records the requesting CoreNode and
clears the failure before requeuing them. A completed import does not rewrite those
existing records.

Room 10 has no group-association or pinned source field. Room 20/21 preserve both.
Each source bridge targets Core SQLite 28 without changing the source version.
Messages and variants retain their historical text and metadata; the importer does
not construct historical frozen send profiles. Binary embedding vectors are derived
cache data without the new cache's endpoint/model/text identity and are not adopted.

ObjectBox keys declare their record kind, model ID and ID width. Memory adoption
reads forward kind-2 relations 7 (tags) and 8 (properties), including 32/64-bit IDs.
Reverse relations are duplicates; kind-8 index entries are not domain relations.
In particular, MemoryLink source/target indexes 8/9 must never supply tag/property IDs.

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
