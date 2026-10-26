# Plugin storage

`Tools.Storage.dataStore` exposes the existing Rust `PreferencesDataStore`. It stores typed JSON in a `.preferences.json` file, uses the native preference operation journal, and participates in the same Space merge executor as Rust preference managers. It does not use object tables, a plugin-specific merge engine, or whole-file last-write-wins synchronization.

```ts
const settings = (await Tools.Storage.dataStore.open({
  path: ToolPkg.getSpaceDataDir() + "/settings.preferences.json",
})).withSchema(1, async (version, preferences) => {
  switch (version) {
    case 0:
      // Convert released source data into the first plugin-owned format here.
      break;
    default:
      throw new Error("Missing schema migration: " + version);
  }
});

await settings.edit(preferences => {
  preferences.set("appearance", { theme: "dark" });
  preferences.remove("retired_setting");
});
const appearance = (await settings.data()).get<{ theme: string }>("appearance");
```

`data()` returns an immutable snapshot. `get()` preserves a stored `null` independently of an absent key (`undefined`). `edit()` accepts a synchronous or asynchronous callback and publishes its mutations through the native preference transaction. A callback exception publishes no preference changes. Native snapshot validation rejects concurrent edits without rerunning the callback. The mutable view cannot be used after its callback completes.

`withSchema()` declares the current version and each numbered upgrade before the handle is used. Unsupported future versions reject. All schema steps for one file publish together. `incrementInteger()` and `setObjectField()` preserve native edit intentions rather than replacing them with a JavaScript synchronization implementation. `dataFlow()` observes native changes, including Rust writes and synchronization replay, through a collector-owned handle.

A plugin may supply bounded data-only merge declarations identifying a catalog's identity/name fields and reference paths. These declarations are retained with the preference journal and adapted to the existing native `PreferenceMerge` executor. No TypeScript merge callbacks run during Space replay. Declared arrays, union variants, and identity prefixes describe reference locations; native UUID/content/name rules make the merge decisions.

When a merge aliases one identity onto a surviving canonical identity, the decision is durable in the database's merge map and stays invisible to the plugin: local commits under an aliased root or child key are redirected onto the canonical rows (with their declared reference fields rewritten), and opening a merged database moves rows that an older writer left under a retired key back onto the canonical identities. A plugin therefore never needs to observe or replay an alias itself.

Use `ToolPkg.getSpaceDataDir()` for the plugin's synchronized folder, or its local data directory for device-owned preferences. A Space cannot inject device-owned preference operations. SQLite/object openers reject preference files; JSON preference files are excluded from whole-file synchronization.

The character plugin keeps its preferences inside its own persistent folder:

```text
runtime/plugin_data/space/com.operit.character_cards/
  cards.preferences.json
  groups.preferences.json
  tags.preferences.json
  stores.preferences.json
  conversationGroups.preferences.json
  meta.preferences.json
  chats.preferences.json
  migration.preferences.json       # Temporary released-data conversion work, when present.
  owners/                         # User documents and memory resources.
  memory.sqlite                   # Current memory engine; not ObjectBox.
```

The first plugin schema is **v1**. Released Core data is adopted into these JSON owners; TypeScript converts plugin-owned identities and references to UUIDs before completing v1. There is no unpublished plugin v2 format or v1-to-v2 migration. Existing released backup formats have their own independent version numbers.

`Tools.Storage.objects` currently uses SQLite. It must not be described as an ObjectBox implementation. Character configuration no longer uses that engine. Each preference file and the memory database has its own transaction boundary; a multi-owner business operation is not a single atomic transaction across those files and the memory engine.

Object handles also expose read-only SQL through `query(sql, params)`. The
`plugin_objects` view has `collection`, `key`, `value` (JSON text), and
`version` columns. Queries use the same typed SQL parameters/results and read-only
statement validation as SQLite handles; internal tables remain protected. Object
writes still use version-checked mutations and the normal synchronization journal.

`objects.commit(mutations, guards?)` accepts up to 1000 related-record guards.
Each guard contains `collection`, `key`, and `expectedVersion`; a null version
requires absence. Rust checks guards in the mutation transaction and rejects a
mismatch without publishing changes. Guards do not rewrite their referenced records.

Character memory uses an envelope containing `ownerKey`, `record`, and
`embedding: { space, text, vector } | null`. Source and normalized vector are one
versioned row, so their local writes, deletion and synchronization share the same
record boundary. The vector space identifies the endpoint, model and normalization
contract. Writes perform HTTP work before opening their commit transaction, then
check both the source version and owner configuration version. Queries create an
ephemeral whole-query vector, rank persisted sources in SQLite, and return bounded
hits without loading the corpus into TypeScript. Missing, stale or dimensionally
incompatible vectors require an explicit rebuild. This is exact scoped vector search,
not an ANN index. Explicit rebuilds/imports use independently atomic batches, so
an error can leave earlier batches committed.
