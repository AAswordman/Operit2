# Operit1 snapshot: read the source, do not adopt legacy domain data

## Contract

A legitimate Operit1 snapshot can contain character cards, character groups,
prompt tags, memory-space preferences, ObjectBox databases, and per-memory-space
`user.md` documents. Their presence is **not** an unknown-format error and must
not prevent restoring the snapshot's supported general application data.

`Operit1SnapshotArchive::fromSource` still indexes every valid ZIP entry and
reads every DataStore payload through the existing protobuf decoder. Existing
manifest/model JSON handling is unchanged, including tolerated unknown JSON
properties. Directory/path traversal, duplicates, malformed protobuf, invalid
model configuration, and unsupported source-schema errors remain errors.
Reading the original source is not character/memory migration or a fallback.

## Restore boundary

| Entry point | Current source behavior | Adopted target behavior |
| --- | --- | --- |
| Archive / `ParsedOperit1Snapshot::fromSource` | Retains ZIP entries, preferences, model configs and mapping | No legacy-domain adoption during parsing |
| Structured preferences | TTS source preferences remain readable | `importSpeechPreferences` imports global TTS configuration only |
| Dedicated card/group/tag preferences | Retained and visible in preview | Not mapped to Core preferences; no card/group/tag manager calls |
| Mixed general preferences | All keys remain present in the source map | Only exact legacy keys/declared namespaces are filtered during target projection; other mapped preferences are still written |
| Memory-space Markdown | Remains an addressable source ZIP entry | No shared-store creation or USER.md write to Core/plugin storage |
| ObjectBox profiles | Remain indexed source ZIP entries | No memory-repository import or memory-link adoption |
| Resource copy | Validated source resource paths remain readable | Copies workspace, ordinary internal and external resources; excludes the DataStore directory and declared ObjectBox / memory-space document directories |
| Room chat data | Reads original `group`, `characterCardName` and, in Room 20/21, `characterGroupId` columns | Projects generic chat/message data; does not bind an old card/group or create a default prompt marker |
| Models / token statistics / chats / workspaces | Existing supported source paths retained | Existing general import methods remain in the restore call chain |

Ordinary workspace or external `USER.md` files are **not** classified as old
memory by filename. The non-adoption rule uses the exact legacy storage layout:
`payload/files/objectbox/`, `payload/files/objectbox_<profile>/`, and
`payload/files/memory-space-profiles/`.

Room 10 reads 13 selected chat columns (`locked` at index 12). Room 20/21 reads
15 (`locked` at 13, `pinned` at 14). Room 10 genuinely has no
`characterGroupId`/`pinned` source field; its source DTO explicitly uses `None`
and `false`. These version definitions are not recovery logic.

The explicit archive bridge now targets SQLite 28, matching
`AppDatabase::DATABASE_VERSION`, while source Room versions remain 10, 20 and
21. The exact target-version check is retained. No source database version is
rewritten to pretend it is the new Core schema.

## Result semantics

- `detectedDomains` reflects actual source preferences and memory entries; it
  does not hide a legacy domain merely because restoration declines to adopt it.
- `importedMemories = 0` and `importedMemoryLinks = 0` count **adopted** records,
  not source records. They do not mean the snapshot contained no memory data.
- Imported chats/messages/variants explicitly initialize `pluginExtensions` to
  an empty map. This means **no plugin association was restored**.
- Historical message text, including its original role text, is not a recovered
  card binding, historical participant identity, or participant TTS association.
  Importing the general TTS preference is not proof that historical participant
  TTS state was restored.
- Any later explicit binding of an imported conversation belongs to the plugin
  workflow. The importer must not write an active/default-character marker or
  insert an invented plugin namespace to accomplish that binding.

## Evidence and limits

```powershell
node --test plugins/packages/buildin/character_cards/tests/operit1-snapshot.test.mjs
```

Current evidence: **10/10 Node tests pass, no skips**. Three execute the actual
production source SELECT statements against real SQLite for Room 10/20/21,
including non-null old associations, locked/pinned fields and an int64 token
counter. Another checks the old short-query/out-of-range regression is rejected.
The remaining checks read the current Rust source restore/projection/format
contract. They do **not** execute Rust projection or the full importer.

`data/backup/operit1/Operit1SnapshotImportTests.rs` contains eight Rust tests
covering actual ZIP/DataStore parsing, real staged SQLite source reads and target
projection, speech/model preview, resource streaming, mixed-preference projection,
and explicit projection errors. The synthetic ZIP includes the legacy fields
and entries; its ObjectBox entry is intentionally opaque and not evidence of
executing an ObjectBox decoder or of a real user-exported database.

The edited Rust files have been syntax-parsed with `rustfmt --emit stdout`.
**No Rust compilation or Rust test execution has been performed.** There is no
passing end-to-end full-import / runtime-host integration claim. The general
restore chain has been preserved and statically checked, but its actual model,
TTS, preferences, chat and token persistence still requires runtime verification
when Rust execution is permitted. No production stub or successful empty host
result was introduced to conceal this limit.
