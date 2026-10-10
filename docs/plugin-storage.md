# Plugin Transactional Storage

`Tools.Storage` adds opt-in databases without changing existing `Tools.Files`,
plugin scripts, CoreLink or UI rendering. It uses the registered RuntimeSqliteHost
and RuntimeStorageHost and existing structured JavaScript Host callbacks.

## Interfaces

- `Tools.Storage.sqlite.open({path})`: parameterized query, execute,
  statement-batch transaction and declarative tracked tables.
- `Tools.Storage.objects.open({path})`: named generic object collections,
  record get/list/put/remove, atomic cross-collection commit and tracked editors.
- `Tools.Storage.dataStore.open({path})`: atomic key writes/removals,
  exact-version checks and paginated key reads.

The object collection API is ObjectBox-style, backed by the same SQLite engine.
It does not expose the upstream ObjectBox database, sealed Rust entities, relations
or a vector-index engine.

Persistent databases use `runtime/plugin_data/device/<namespace>/...` or
`runtime/plugin_data/space/<namespace>/...`, independent of plugin installation
scope. `ToolPkg.getLocalDataDir()` and `ToolPkg.getSpaceDataDir()` create and return
these two fixed directories for the current container, during registration and
runtime. Their public `/app/data/plugin_data/...` VFS paths work directly with
storage open calls and existing file APIs. A device-installed plugin can use both.

Namespaces organize data; they do not enforce per-plugin access restrictions.
Different plugins may open and update the same database by path. The durable
namespace, database path, storage kind and synchronization identity are retained
when a different script opens it. Databases are excluded from whole-file sync.
Device data stays local; Space data participates in transaction synchronization.
There is no `sync` open option.

Plugin installation moves and uninstall do not move or delete these data roots.
`ToolPkg.getConfigDir()` retains its existing installation-scope configuration
behavior for ordinary files. Transactional databases require the independent data
roots so an installation move cannot implicitly change their data scope.
Runtime-relative paths and physical Host paths under the runtime root also work.
Invalid segments and reserved SQLite sidecar suffixes reject explicitly.

```javascript
const root = ToolPkg.getSpaceDataDir();
const db = await Tools.Storage.objects.open({path: root + '/memory.sqlite'});
const memories = db.collection('memories');
await memories.put('one', {text: 'Example', metadata: {score: 1}}, {expectedVersion: null});
const editor = await memories.edit('one');
editor.value.metadata.score = 2;
await editor.flush();
await db.close();
```

`expectedVersion: null` requires that the record is absent. An exact string
requires the current stored version to match. Omitting the property requests an
unconditional write. CAS protects local commits, not a distributed global lock.

An editor tracks nested object and array writes without cloning the record via
JSON. `flush()` persists that one complete changed record, not the entire
collection. It checks the version read by `edit()`. A conflict rejects the save
and retains pending edits. Mutations during an in-flight flush reject. Root editor
values must be existing objects; ordinary put/get also support scalar and array
values. Plain finite structured values are supported; cycles, hidden/accessor
properties, undefined and unsupported objects reject instead of losing fields.

```javascript
const localRoot = ToolPkg.getLocalDataDir();
const settings = await Tools.Storage.dataStore.open({path: localRoot + '/settings.sqlite'});
await settings.commit({
  set: {theme: 'dark', nullable: null},
  remove: ['obsolete'],
  expectedVersions: {theme: null}
});
const entry = await settings.get('nullable'); // {value: null, version: ...}
const absent = await settings.get('missing'); // null
await settings.close();
```

## SQL And Transactions

```javascript
const sql = await Tools.Storage.sqlite.open({path: root + '/rows.sqlite'});
await sql.defineTable({
  name: 'memories', primaryKey: 'id',
  columns: [
    {name: 'id', affinity: 'text', nullable: false},
    {name: 'text', affinity: 'text', nullable: false},
    {name: 'score', affinity: 'integer', nullable: false}
  ]
});
await sql.transaction([
  {sql: 'INSERT INTO memories VALUES(?,?,?)', params: ['one', 'Example', 1n]},
  {sql: 'UPDATE memories SET score=? WHERE id=?', params: [2n, 'one']}
]);
const rows = await sql.query('SELECT id,text,score FROM memories WHERE id=? LIMIT 100', ['one']);
await sql.close();
```

All SQLite integer results are bigint; arguments accept safe integer numbers or
int64 bigint. Binary values use Uint8Array. Transactions accept 1..1000 statements
and at most 1000 dirty records. Object/DataStore commits accept 1..1000 unique keys.
Queries use caller-selected SQL pagination; the current Host query interface
materializes its result, so callers must constrain large result sets explicitly.

SELECT is read-only. Writes allow INSERT/UPDATE/DELETE, local unsynchronized
CREATE TABLE, and CREATE INDEX. Synchronized unique indexes reject because they
would introduce cross-record conflict constraints. Synchronized tables use one
nonnullable TEXT primary key and explicit column affinities. Private tables,
introspection, filesystem SQL functions, ATTACH/PRAGMA and executable remote SQL
are not exposed. Arbitrary raw local tables do not produce row deltas; use
`defineTable()` for dirty tracking, including local `changes()` observation.

## Synchronization And Changes

Tracked SQL triggers mark only changed primary keys. The mutation, row/key journal
and exact versions commit in the same SQLite transaction. A transaction transports
only changed rows/objects/keys and the declarations of its affected SQL tables.
This supports independent replay when different origins arrive out of order.

Record conflicts use deterministic per-key `(logical time, origin, sequence)`
last-writer-wins ordering. Remote replay commits all applicable changes atomically.
Duplicate identities with different content reject. Received operations forward
with their original identities; they do not become newly authored local writes.
Each recreated database has a durable new origin epoch, avoiding sequence reuse.
SQL uniqueness beyond the declared primary key is not a distributed constraint.

The database, registration marker and exact WAL/SHM/journal sidecars are excluded
from whole-file sync. JSON text remains an on-disk encoding for each changed
structured record and journal payload. The JS request/reply boundary does not
stringify/parse the request or return value, and no whole collection is serialized.
This is not a statement that the application contains no JSON.

`await db.changes('0', 100)` returns transactions with opaque decimal cursors.
Persist the last returned cursor and request the next page. This is cursor-based
observation, not a live push/watch stream. `close()` releases the retained handle
without deleting its database.

Plugin installation moves preserve both persistent data roots and live database
handles. Installation configuration files retain their existing scope move
behavior. Persistent data has a separate lifetime and is retained on uninstall.

## Adoption And Verification

The character/memory plugin uses `Tools.Storage.objects` at
`ToolPkg.getSpaceDataDir() + "/characters.sqlite"`. Cards, groups, tags, shared
stores, owners, memories, links, chunks, extraction candidates and embedding-cache
entries are separate records. Normal domain mutations commit only changed records
and an exact-version metadata guard. Collection reads use bounded pages; the
current domain session still assembles an operation snapshot in memory.

`USER.md` remains a real file for editing and attachment through `Tools.Files`.
Writes stage a sibling file before the record commit and publish it afterwards.
SQLite records commit atomically; SQLite and external files do not share a
transaction. A publication error remains visible and stops that repository instance.

Released Core v27 data is adopted by `CharacterPluginMigration` before plugin
startup. The same writer handles Operit1 imports. Migration writes an in-progress
marker, bounded transactions, verified documents/resources and a completion marker.
The plugin refuses to open an incomplete migration. Old owners and the four dedicated
preference files are deleted only after Core chat and historical sync bindings are
resolved. Cleanup validates every target before deleting and resumes after interruption.
The removed unreleased plugin `state.json` format has no compatibility reader.

Coverage includes real SQLite rollback/CAS, null-versus-delete, int64/blob fidelity,
quoted private SQL access rejection, unchanged-row exclusion, record sync,
self-contained SQL replay, database reincarnation and file-sync exclusion. The
native JS integration uses the production SDK and a real SQLite Host, with JSON
text codecs blocked specifically during storage request/reply conversion.
Node tests cover the facade, proxy edits and originating execution restoration.
TypeScript declarations include the generated wire schema and author-facing facade.
