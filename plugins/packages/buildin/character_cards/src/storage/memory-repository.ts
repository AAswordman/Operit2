import type { StorageRecordGuard } from "../../../../../types/storage.generated";
import type { Storage } from "../../../../../types/storage";
import type { DocumentChunk, Memory, MemoryAutoSaveStatus, MemoryGraph, MemoryImportResult, MemoryJobState, MemoryLink, MemoryLinkUpdate, MemoryLinkValues, MemoryRebuildProgress, MemorySearchConfig, MemorySearchDebug, MemorySearchOptions, MemorySettings, MemorySpace, MemoryValues } from "../model";
import { uuid } from "../identity";
import { assertMemory, assertMemoryValues, assertMemorySettings, assertNumber, assertSearchConfig, assertSearchOptions, assertString, normalizeNames, requireId, requireName } from "../validation";
import { assertMemorySpace } from "./state";
import { clone, equal, storedValue } from "./records";
import { CharacterMemoryDatabase } from "./memory";
import type { ObjectMutation } from "./memory";
import { embedTexts, embeddingSpace } from "../memory-jobs/embeddings";
import type { SourceEmbedding } from "../memory-jobs/embeddings";
import { searchDocumentChunks } from "../search/chunks";
import { prepareMemoryQuery, searchMemoryDatabase } from "../memory-search";

type Owner = Omit<MemorySpace, "memories" | "links" | "chunks" | "candidates" | "embeddings">;
interface Envelope<T> { ownerKey: string; record: T; embedding?: SourceEmbedding | null }
interface Row { collection: string; key: string; value: Storage.Value; version: string }

/** Requires a stable owner-scoped record identity. */
function key(ownerKey: string, id: string): string { return requireId(ownerKey, "owner key") + "/" + requireId(id, "record id"); }

/** Normalizes folder separators and rejects relative traversal. */
function folder(value: string | null): string | null {
  if (value === null) return null;
  const parts = value.replace(/\\/g, "/").split("/").map(/** Normalizes authored folder components. */ part => part.trim()).filter(/** Removes empty path components. */ part => part !== "");
  if (parts.some(/** Rejects relative traversal components. */ part => part === "." || part === "..")) throw new Error("Relative folder traversal is not allowed");
  return parts.length === 0 ? null : parts.join("/");
}

/** Builds one exact-version mutation without changing unrelated records. */
function put(collection: string, recordKey: string, value: object, version: string | null): ObjectMutation {
  return { op: "put", collection, key: recordKey, value: storedValue(value), expectedVersion: version };
}

/** Builds one guarded deletion from its original database snapshot. */
function remove(row: Row): ObjectMutation { return { op: "delete", collection: row.collection, key: row.key, expectedVersion: row.version }; }

/** Implements memory operations on synchronized database records using synchronized source records and derived owner-local search indexes. */
export class DatabaseMemoryRepository {
  private readonly snapshots = new WeakMap<object, Row[]>();

  /** Retains only the storage capability; network work never owns a database transaction. */
  constructor(protected readonly memoryDatabase: CharacterMemoryDatabase) {}

  /** Requires the persisted metadata for one exact owner. */
  protected async owner(ownerKey: string): Promise<Storage.Entry<Owner>> {
    const entry = await this.memoryDatabase.get<Owner>("owners", requireId(ownerKey, "memory owner"));
    if (entry === null) throw new Error("Missing memory owner: " + ownerKey);
    return entry;
  }

  /** Requires one existing source or relationship without scanning its collection. */
  private async entry<T>(collection: string, ownerKey: string, id: string): Promise<Storage.Entry<Envelope<T>>> {
    const entry = await this.memoryDatabase.get<Envelope<T>>(collection, key(ownerKey, id));
    if (entry === null) throw new Error("Missing " + collection + " record: " + id);
    return entry;
  }

  /** Reads explicit owner exports while preserving exact versions for later edits. */
  private async rows(collection: string, ownerKey: string, condition = "1", params: Storage.SqlValue[] = []): Promise<Row[]> {
    return (await this.memoryDatabase.all<Storage.Value>(collection, ownerKey, condition, params)).map(
      /** Preserves record provenance for guarded writes. */ row => ({ collection, ...row }));
  }

  /** Publishes one bounded operation atomically and rejects oversized operations before writing. */
  private async commit(mutations: ObjectMutation[], guards: StorageRecordGuard[] = []): Promise<void> {
    if (mutations.length === 0) return;
    if (mutations.length > 1000) throw new Error("Memory operation exceeds the 1000-record atomic transaction limit");
    await this.memoryDatabase.commit(mutations, guards);
  }

  /** Builds or reuses the vector belonging to this exact source input. */
  private async sourceEmbedding(text: string, settings: MemorySettings, existing: SourceEmbedding | null | undefined): Promise<SourceEmbedding | null> {
    if (!settings.cloudEmbeddingEnabled) return null;
    const space = embeddingSpace(settings);
    if (existing !== undefined && existing !== null && existing.space === space && existing.text === text) return existing;
    return (await embedTexts([text], settings))[0];
  }

  /** Reads settings from one owner record. */
  async readMemorySettings(ownerKey: string): Promise<MemorySettings> { return (await this.owner(ownerKey)).value.settings; }

  /** Updates complete settings with the owner's exact version precondition. */
  async writeMemorySettings(ownerKey: string, settings: MemorySettings): Promise<void> {
    assertMemorySettings(settings);
    if (settings.cloudEmbeddingEnabled) embeddingSpace(settings);
    const owner = await this.owner(ownerKey);
    await this.commit([put("owners", ownerKey, { ...owner.value, settings }, owner.version)]);
  }

  /** Saves editor settings and weights atomically while preserving the scheduler's current due time. */
  async writeMemoryOwnerSettings(ownerKey: string, settings: MemorySettings, config: MemorySearchConfig): Promise<{ settings: MemorySettings; config: MemorySearchConfig }> {
    assertMemorySettings(settings); assertSearchConfig(config);
    if (settings.cloudEmbeddingEnabled) embeddingSpace(settings);
    const owner = await this.owner(ownerKey), saved = { ...settings, nextAutoSaveRunAtMs: owner.value.settings.nextAutoSaveRunAtMs };
    await this.commit([put("owners", ownerKey, { ...owner.value, settings: saved, searchConfig: config }, owner.version)]);
    return { settings: saved, config };
  }

  /** Reads persisted search weights without reading memory bodies. */
  async readMemorySearchConfig(ownerKey: string): Promise<MemorySearchConfig> { return (await this.owner(ownerKey)).value.searchConfig; }

  /** Writes complete search configuration through optimistic concurrency. */
  async writeMemorySearchConfig(ownerKey: string, searchConfig: MemorySearchConfig): Promise<void> {
    assertSearchConfig(searchConfig); const owner = await this.owner(ownerKey);
    await this.commit([put("owners", ownerKey, { ...owner.value, searchConfig }, owner.version)]);
  }

  /** Returns bounded ranked results produced by the database. */
  async searchMemories(options: MemorySearchOptions): Promise<Memory[]> {
    assertSearchOptions(options); const owner = await this.owner(options.ownerKey);
    return (await searchMemoryDatabase(this.memoryDatabase, owner.value.settings, options, owner.value.searchConfig)).result.candidates.map(
      /** Returns the complete source records selected in the database. */ hit => hit.memory);
  }

  /** Returns one filtered page and its database-counted snapshot exclusions. */
  async searchMemoryPage(options: MemorySearchOptions): Promise<import("../api").DomainOutput<"memory.searchWithOptions">> {
    assertSearchOptions(options); const owner = await this.owner(options.ownerKey);
    const { result, queryEmbedding } = await searchMemoryDatabase(this.memoryDatabase, owner.value.settings, options, owner.value.searchConfig);
    const items = result.candidates.map(/** Projects only selected source records. */ hit => hit.memory);
    const documents = await Promise.all(items.filter(/** Selects only returned document sources. */ memory => memory.isDocumentNode).map(
      /** Reuses one query vector for every selected document without further HTTP requests. */
      async memory => ({ memoryId: memory.id, ...await this.queryChunks(options.ownerKey, owner.value, memory, options.query, Math.min(options.limit ?? 20, 20), queryEmbedding) })));
    return { ownerKey: options.ownerKey, items, documents, excludedBySnapshotCount: result.excludedBySnapshotCount };
  }

  /** Returns bounded score explanations from the same database query used by recall. */
  async searchMemoryDebug(options: MemorySearchOptions, config: MemorySearchConfig): Promise<MemorySearchDebug> {
    assertSearchOptions(options); assertSearchConfig(config); const owner = await this.owner(options.ownerKey);
    return (await searchMemoryDatabase(this.memoryDatabase, owner.value.settings, options, config, true)).result;
  }

  /** Lists an explicitly requested owner library without loading other owners. */
  async listMemories(ownerKey: string): Promise<Memory[]> {
    await this.owner(ownerKey);
    const rows = await this.memoryDatabase.all<Envelope<Memory>>("memories", ownerKey);
    return rows.map(/** Projects original records for the library view or export. */ row => row.value.record);
  }

  /** Reads only the explicitly selected exact titles, including genuine duplicate clusters. */
  async readMemoriesByTitles(ownerKey: string, titles: string[]): Promise<Memory[]> {
    await this.owner(ownerKey);
    const rows = await this.memoryDatabase.all<Envelope<Memory>>("memories", ownerKey, "json_extract(value,'$.record.title') IN (SELECT value FROM json_each(?))", [JSON.stringify(titles)]);
    return rows.map(/** Projects only selected source records. */ row => row.value.record);
  }

  /** Requires one exact title and rejects ambiguity within a two-record database page. */
  async readMemoryByTitle(ownerKey: string, title: string): Promise<Memory> {
    await this.owner(ownerKey); const name = requireName(title, "memory title");
    const rows = await this.memoryDatabase.select<Envelope<Memory>>("memories", ownerKey, "json_extract(value,'$.record.title')=?", [name], 2);
    if (rows.length === 0) throw new Error("记忆不存在：" + name);
    if (rows.length !== 1) throw new Error("记忆标题不唯一：" + name);
    return rows[0].value.record;
  }

  /** Reads distinct folder paths in the database and expands their parent directories. */
  async listMemoryFolders(ownerKey: string): Promise<string[]> {
    await this.owner(ownerKey);
    const rows = await this.memoryDatabase.query("SELECT DISTINCT json_extract(value,'$.record.folderPath') AS folder FROM plugin_objects WHERE collection='memories' AND key>=? AND key<?", [ownerKey + "/", ownerKey + "0"]);
    const folders = new Set<string>();
    for (const row of rows) {
      if (row.folder === null) { folders.add(""); continue; }
      const parts = String(row.folder).split("/");
      for (let length = 1; length <= parts.length; length += 1) folders.add(parts.slice(0, length).join("/"));
    }
    return [...folders].sort();
  }

  /** Creates an intentional new non-document memory. */
  async createMemory(ownerKey: string, values: MemoryValues): Promise<Memory> {
    assertMemoryValues(values); const now = Date.now();
    return this.saveMemory(ownerKey, { ...values, id: "", uuid: "", title: requireName(values.title, "memory title"), folderPath: folder(values.folderPath),
      documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: now, updatedAt: now, lastAccessedAt: now,
      tags: normalizeNames(values.tags).map(/** Allocates explicit authored tag identities. */ name => ({ id: uuid(), name })), properties: [] });
  }

  /** Captures the source version before embedding work begins. */
  async saveMemory(ownerKey: string, memory: Memory): Promise<Memory> {
    const original = memory.id === "" ? null : await this.entry<Memory>("memories", ownerKey, memory.id);
    return this.saveMemoryRecord(ownerKey, memory, original);
  }

  /** Commits a source and its vector under the version that supplied its original fields. */
  private async saveMemoryRecord(ownerKey: string, memory: Memory, original: Storage.Entry<Envelope<Memory>> | null): Promise<Memory> {
    const owner = await this.owner(ownerKey), incoming = clone(memory), now = Date.now();
    incoming.title = requireName(incoming.title, "memory title"); incoming.folderPath = folder(incoming.folderPath);
    if (incoming.id === "") {
      incoming.id = incoming.uuid === "" ? uuid() : incoming.uuid; incoming.uuid = incoming.id;
      incoming.createdAt = now; incoming.lastAccessedAt = now;
    } else {
      if (original === null) throw new Error("Existing memory requires its original source version");
      if (original.value.record.uuid !== incoming.uuid) throw new Error("An existing memory UUID is immutable");
      incoming.createdAt = original.value.record.createdAt;
    }
    incoming.updatedAt = now; assertMemory(incoming);
    const occupied = await this.memoryDatabase.select<Envelope<Memory>>("memories", ownerKey,
      "(json_extract(value,'$.record.title')=? OR json_extract(value,'$.record.uuid')=?) AND json_extract(value,'$.record.id')<>?", [incoming.title, incoming.uuid, incoming.id], 1);
    if (occupied.length !== 0) throw new Error("Memory title or UUID already exists");
    const embedding = await this.sourceEmbedding(incoming.isDocumentNode ? incoming.title : incoming.content, owner.value.settings, original?.value.embedding);
    await this.commit([put("memories", key(ownerKey, incoming.id), { ownerKey, record: incoming, embedding }, original === null ? null : original.version)], [{ collection: "owners", key: ownerKey, expectedVersion: owner.version }]);
    return incoming;
  }

  /** Updates authored fields while retaining the source identity and document metadata. */
  async updateMemory(ownerKey: string, id: string, values: MemoryValues): Promise<Memory> {
    assertMemoryValues(values); const original = await this.entry<Memory>("memories", ownerKey, id);
    return this.saveMemoryRecord(ownerKey, { ...original.value.record, ...values, tags: normalizeNames(values.tags).map(
      /** Allocates the explicit replacement tag list. */ name => ({ id: uuid(), name })) }, original);
  }

  /** Merges selected sources, redirects their relationships and commits the new vector atomically. */
  async mergeMemories(ownerKey: string, sourceTitles: string[], values: MemoryValues): Promise<Memory> {
    assertMemoryValues(values); const owner = await this.owner(ownerKey);
    const titles = [...new Set(sourceTitles.map(/** Validates explicit merge source titles. */ title => requireName(title, "merge source title")))];
    const sources = await this.rows("memories", ownerKey, "json_extract(value,'$.record.title') IN (SELECT value FROM json_each(?))", [JSON.stringify(titles)]);
    const memories = sources.map(/** Projects selected merge sources only. */ row => (row.value as unknown as Envelope<Memory>).record);
    for (const title of titles) if (!memories.some(/** Requires an exact persisted source title. */ memory => memory.title === title)) throw new Error("Memory merge source does not exist: " + title);
    if (memories.length < 2) throw new Error("Memory merge requires at least two unique source records");
    const ids = memories.map(/** Captures stable source identities for relationship rewrites. */ memory => memory.id), uuids = memories.map(/** Captures document identities for chunk deletion. */ memory => memory.uuid);
    const occupied = await this.memoryDatabase.select("memories", ownerKey, "json_extract(value,'$.record.title')=? AND json_extract(value,'$.record.id') NOT IN (SELECT value FROM json_each(?))", [values.title.trim(), JSON.stringify(ids)], 1);
    if (occupied.length !== 0) throw new Error("Memory merge target title already exists");
    const now = Date.now(), id = uuid();
    const record: Memory = { ...values, id, uuid: id, title: requireName(values.title, "memory title"), folderPath: folder(values.folderPath),
      tags: normalizeNames(values.tags).map(/** Allocates the merged record's authored tags. */ name => ({ id: uuid(), name })), properties: [],
      createdAt: now, updatedAt: now, lastAccessedAt: now, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null };
    assertMemory(record);
    const chunks = await this.rows("chunks", ownerKey, "json_extract(value,'$.record.memoryUuid') IN (SELECT value FROM json_each(?))", [JSON.stringify(uuids)]);
    const links = await this.rows("links", ownerKey, "json_extract(value,'$.record.sourceMemoryId') IN (SELECT value FROM json_each(?)) OR json_extract(value,'$.record.targetMemoryId') IN (SELECT value FROM json_each(?))", [JSON.stringify(ids), JSON.stringify(ids)]);
    const embedding = await this.sourceEmbedding(record.content, owner.value.settings, null);
    const mutations: ObjectMutation[] = [...sources.map(remove), ...chunks.map(remove), put("memories", key(ownerKey, id), { ownerKey, record, embedding }, null)];
    const selected = new Set(ids);
    for (const row of links) {
      const envelope = row.value as unknown as Envelope<MemoryLink>, link = { ...envelope.record };
      if (selected.has(link.sourceMemoryId)) link.sourceMemoryId = id;
      if (selected.has(link.targetMemoryId)) link.targetMemoryId = id;
      mutations.push(link.sourceMemoryId === link.targetMemoryId ? remove(row) : put("links", row.key, { ...envelope, record: link }, row.version));
    }
    await this.commit(mutations); return record;
  }

  /** Deletes the source vector, document chunks and incident links in the same transaction. */
  async deleteMemory(ownerKey: string, id: string): Promise<void> {
    await this.owner(ownerKey); const memory = await this.entry<Memory>("memories", ownerKey, id);
    const chunks = await this.rows("chunks", ownerKey, "json_extract(value,'$.record.memoryUuid')=?", [memory.value.record.uuid]);
    const links = await this.rows("links", ownerKey, "json_extract(value,'$.record.sourceMemoryId')=? OR json_extract(value,'$.record.targetMemoryId')=?", [id, id]);
    await this.commit([{ op: "delete", collection: "memories", key: key(ownerKey, id), expectedVersion: memory.version }, ...chunks.map(remove), ...links.map(remove)]);
  }

  /** Moves selected records without regenerating unchanged source vectors. */
  async moveMemories(ownerKey: string, ids: string[], folderPath: string): Promise<void> {
    await this.owner(ownerKey); const destination = folder(folderPath), mutations: ObjectMutation[] = [];
    for (const id of new Set(ids)) {
      const entry = await this.entry<Memory>("memories", ownerKey, id);
      mutations.push(put("memories", key(ownerKey, id), { ...entry.value, record: { ...entry.value.record, folderPath: destination, updatedAt: Date.now() } }, entry.version));
    }
    await this.commit(mutations);
  }

  /** Lists explicit graph relationships for one owner. */
  async listMemoryLinks(ownerKey: string): Promise<MemoryLink[]> {
    await this.owner(ownerKey);
    return (await this.memoryDatabase.all<Envelope<MemoryLink>>("links", ownerKey)).map(/** Projects complete graph relationships. */ row => row.value.record);
  }

  /** Reads one relationship by its exact owner-scoped identity. */
  async readMemoryLink(ownerKey: string, id: string): Promise<MemoryLink> { await this.owner(ownerKey); return (await this.entry<MemoryLink>("links", ownerKey, id)).value.record; }

  /** Creates a relationship after checking both exact source identities. */
  async createLink(ownerKey: string, values: MemoryLinkValues): Promise<MemoryLink> {
    await this.owner(ownerKey); await this.entry("memories", ownerKey, values.sourceMemoryId); await this.entry("memories", ownerKey, values.targetMemoryId);
    if (values.sourceMemoryId === values.targetMemoryId) throw new Error("Self relationships are not allowed");
    assertNumber(values.weight, "link weight", 0, 1); assertString(values.description, "link description");
    const record = { ...values, type_: requireName(values.type_, "link type"), id: uuid() };
    await this.commit([put("links", key(ownerKey, record.id), { ownerKey, record }, null)]); return record;
  }

  /** Updates the exact relationship version that supplied the original fields. */
  async updateLink(ownerKey: string, id: string, values: MemoryLinkUpdate): Promise<MemoryLink> {
    await this.owner(ownerKey); const original = await this.entry<MemoryLink>("links", ownerKey, id);
    assertNumber(values.weight, "link weight", 0, 1); assertString(values.description, "link description");
    const record = { ...original.value.record, ...values, type_: requireName(values.type_, "link type") };
    await this.commit([put("links", key(ownerKey, id), { ownerKey, record }, original.version)]); return record;
  }

  /** Deletes one exact relationship with its optimistic concurrency precondition. */
  async deleteLink(ownerKey: string, id: string): Promise<void> {
    await this.owner(ownerKey); const original = await this.entry<MemoryLink>("links", ownerKey, id);
    await this.commit([{ op: "delete", collection: "links", key: key(ownerKey, id), expectedVersion: original.version }]);
  }

  /** Reads job metadata and candidates independently of memory bodies and vectors. */
  async readMemoryJobState(ownerKey: string): Promise<MemoryJobState> {
    const owner = await this.owner(ownerKey), candidates = await this.rows("candidates", ownerKey);
    const state: MemoryJobState = { ...owner.value, candidates: candidates.map(/** Projects job candidates without memory data. */ row => (row.value as unknown as Envelope<MemoryJobState["candidates"][number]>).record) };
    this.snapshots.set(state, clone([{ collection: "owners", key: ownerKey, value: storedValue(owner.value), version: owner.version }, ...candidates]));
    return state;
  }

  /** Commits only changed job records against the versions read by this operation. */
  async writeMemoryJobState(ownerKey: string, state: MemoryJobState): Promise<void> {
    if (state.ownerKey !== ownerKey) throw new Error("Memory job owner cannot change");
    const before = this.snapshots.get(state);
    if (before === undefined) throw new Error("Memory job edits require their original database snapshot");
    const { candidates, ...owner } = state;
    const after = [{ collection: "owners", key: ownerKey, value: storedValue(owner) }, ...candidates.map(
      /** Uses stable candidate identities for concurrent scheduler claims. */ record => ({ collection: "candidates", key: key(ownerKey, record.id), value: storedValue({ ownerKey, record }) }))];
    await this.commit(this.diff(before, after)); this.snapshots.delete(state);
  }

  /** Computes record-level mutations only for explicitly selected records. */
  private diff(before: Row[], after: { collection: string; key: string; value: Storage.Value }[]): ObjectMutation[] {
    const previous = new Map(before.map(/** Indexes the operation's original exact versions. */ row => [row.collection + "\0" + row.key, row]));
    const mutations: ObjectMutation[] = [];
    for (const row of after) {
      const identity = row.collection + "\0" + row.key, original = previous.get(identity);
      if (original === undefined || !equal(original.value, row.value)) mutations.push({ op: "put", ...row, expectedVersion: original === undefined ? null : original.version });
      previous.delete(identity);
    }
    mutations.push(...[...previous.values()].map(remove)); return mutations;
  }

  /** Counts candidate states in SQLite without loading their payloads. */
  async readMemoryAutoSaveStatus(ownerKey: string): Promise<MemoryAutoSaveStatus> {
    const owner = await this.owner(ownerKey);
    const rows = await this.memoryDatabase.query(`SELECT json_extract(value,'$.record.status') AS status,count(*) AS count,count(DISTINCT json_extract(value,'$.record.chatId')) AS chats
      FROM plugin_objects WHERE collection='candidates' AND key>=? AND key<? GROUP BY json_extract(value,'$.record.status')`, [ownerKey + "/", ownerKey + "0"]);
    let pendingCandidates = 0, pendingChats = 0, processingCandidates = 0, failedCandidates = 0;
    for (const row of rows) {
      if (row.status === "pending") { pendingCandidates = Number(row.count); pendingChats = Number(row.chats); }
      if (row.status === "processing") processingCandidates = Number(row.count);
      if (row.status === "failed") failedCandidates = Number(row.count);
    }
    const failed = await this.memoryDatabase.select<Envelope<MemoryJobState["candidates"][number]>>("candidates", ownerKey, "json_extract(value,'$.record.status')='failed'", [], 1);
    const nextRunAtMs = owner.value.settings.nextAutoSaveRunAtMs;
    return { ownerKey, pendingCandidates, pendingChats, processingCandidates, failedCandidates, nextRunAtMs,
      minutesUntilNextRun: Math.max(0, Math.ceil((nextRunAtMs - Date.now()) / 60000)), lastError: failed.length === 0 ? "" : failed[0].value.record.lastError };
  }

  /** Reads persisted rebuild progress without waiting for model requests. */
  async readMemoryRebuildProgress(ownerKey: string): Promise<MemoryRebuildProgress> { return (await this.owner(ownerKey)).value.rebuildProgress; }

  /** Cancels a running task through an exact owner-version update. */
  async cancelMemoryRebuild(ownerKey: string): Promise<void> {
    const owner = await this.owner(ownerKey), progress = owner.value.rebuildProgress;
    if (progress.status !== "running" && progress.status !== "preparing") throw new Error("No memory rebuild is active");
    await this.commit([put("owners", ownerKey, { ...owner.value, rebuildProgress: { ...progress, status: "cancelled" } }, owner.version)]);
  }

  /** Rebuilds source vectors page by page and publishes completed pages durably. */
  async rebuildMemoryEmbeddings(ownerKey: string): Promise<number> {
    const owner = await this.owner(ownerKey); embeddingSpace(owner.value.settings);
    let count = 0;
    for (const collection of ["memories", "chunks"] as const) {
      let after = "";
      for (;;) {
        const rows = await this.memoryDatabase.select<Envelope<Memory | DocumentChunk>>(collection, ownerKey, "1", [], 256, after);
        if (rows.length === 0) break;
        const texts = rows.map(/** Selects the exact searchable source input. */ row => collection === "memories" && (row.value.record as Memory).isDocumentNode ? (row.value.record as Memory).title : row.value.record.content);
        const embeddings = await embedTexts(texts, owner.value.settings);
        const mutations = rows.map(/** Publishes source provenance and vector together under the original version. */ (row, index) => put(collection, row.key, { ...row.value, embedding: embeddings[index] }, row.version));
        for (let start = 0; start < mutations.length; start += 64) await this.commit(mutations.slice(start, start + 64), [{ collection: "owners", key: ownerKey, expectedVersion: owner.version }]);
        if (collection === "memories") count += rows.length;
        after = rows[rows.length - 1].key;
      }
    }
    return count;
  }

  /** Reads full owner records only for explicit backups and bulk imports. */
  async readMemorySpace(ownerKey: string): Promise<MemorySpace> {
    const owner = await this.owner(ownerKey), state: MemorySpace = { ...owner.value, memories: [], links: [], chunks: [], candidates: [], embeddings: [] };
    const before: Row[] = [{ collection: "owners", key: ownerKey, value: storedValue(owner.value), version: owner.version }];
    for (const collection of ["memories", "links", "chunks", "candidates"] as const) {
      const rows = await this.rows(collection, ownerKey); before.push(...rows);
      for (const row of rows) (state[collection] as object[]).push((row.value as unknown as Envelope<object>).record);
    }
    this.snapshots.set(state, clone(before)); return state;
  }

  /** Applies an explicit owner import while retaining unchanged source vectors. */
  async writeMemorySpace(ownerKey: string, state: MemorySpace): Promise<void> {
    assertMemorySpace(state);
    if (state.ownerKey !== ownerKey) throw new Error("Memory owner identity cannot change");
    const before = this.snapshots.get(state);
    if (before === undefined) throw new Error("Memory imports require their original database snapshot");
    const { memories, links, chunks, candidates, embeddings: _retiredCache, ...owner } = state;
    const originalOwner = before.find(/** Selects the original owner path guard. */ row => row.collection === "owners");
    if (originalOwner === undefined || (originalOwner.value as unknown as Owner).userDocumentPath !== owner.userDocumentPath) throw new Error("Memory USER.md path cannot change");
    const previous = new Map(before.map(/** Indexes source envelopes by their stable persisted identity. */ row => [row.collection + "\0" + row.key, row]));
    const after: { collection: string; key: string; value: Storage.Value }[] = [{ collection: "owners", key: ownerKey, value: storedValue(owner) }];
    const pending: { text: string; envelope: Envelope<object> }[] = [];
    const space = owner.settings.cloudEmbeddingEnabled ? embeddingSpace(owner.settings) : "";
    for (const [collection, records] of [["memories", memories], ["links", links], ["chunks", chunks], ["candidates", candidates]] as const) {
      for (const record of records) {
        const recordKey = key(ownerKey, record.id), original = previous.get(collection + "\0" + recordKey);
        const envelope: Envelope<typeof record> = { ownerKey, record };
        if (collection === "memories" || collection === "chunks") {
          const text = collection === "memories" && (record as Memory).isDocumentNode ? (record as Memory).title : (record as Memory | DocumentChunk).content;
          const existing = (original?.value as unknown as Envelope<object> | undefined)?.embedding;
          envelope.embedding = null;
          if (owner.settings.cloudEmbeddingEnabled) {
            if (existing !== undefined && existing !== null && existing.space === space && existing.text === text) envelope.embedding = existing;
            else pending.push({ text, envelope });
          }
        }
        after.push({ collection, key: recordKey, value: storedValue(envelope) });
      }
    }
    if (pending.length !== 0) {
      const computed = await embedTexts(pending.map(/** Batches only changed searchable inputs. */ item => item.text), owner.settings);
      for (let index = 0; index < pending.length; index += 1) pending[index].envelope.embedding = computed[index];
    }
    const mutations = this.diff(before, after).sort(/** Publishes owner configuration after its imported source records. */ (left, right) => Number(left.collection === "owners") - Number(right.collection === "owners"));
    for (let start = 0; start < mutations.length; start += 64) await this.commit(mutations.slice(start, start + 64), [{ collection: "owners", key: ownerKey, expectedVersion: originalOwner.version }]);
    this.snapshots.delete(state);
  }

  /** Builds the explicitly requested graph from this owner's sources and relationships. */
  async readMemoryGraph(ownerKey: string): Promise<MemoryGraph> {
    const memories = await this.listMemories(ownerKey), links = await this.listMemoryLinks(ownerKey), byId = new Map(memories.map(/** Indexes graph endpoint identities. */ memory => [memory.id, memory]));
    return { nodes: memories.map(/** Projects source identities for the graph view. */ memory => ({ id: memory.uuid, label: memory.title, color: memory.isDocumentNode ? 0xFF9575CD : 0xFFD3D3D3, metadata: { memoryId: memory.id } })),
      edges: links.map(/** Requires actual persisted graph endpoints. */ link => {
        const source = byId.get(link.sourceMemoryId), target = byId.get(link.targetMemoryId);
        if (source === undefined || target === undefined) throw new Error("Memory relationship has a missing endpoint");
        return { id: link.id, sourceId: source.uuid, targetId: target.uuid, label: link.type_, weight: link.weight, metadata: { description: link.description }, isCrossFolderLink: source.folderPath !== target.folderPath };
      }) };
  }

  /** Searches original document chunks using database-side lexical and vector ranking. */
  async searchMemoryChunks(ownerKey: string, memoryId: string, query: string, count: number): Promise<{ total: number; chunks: DocumentChunk[] }> {
    const owner = await this.owner(ownerKey), memory = (await this.entry<Memory>("memories", ownerKey, memoryId)).value.record;
    if (!memory.isDocumentNode) throw new Error("Document search requires a stored document memory");
    if (query.trim() === "") throw new Error("Document search query must not be blank");
    if (!Number.isSafeInteger(count) || count < 1) throw new Error("Chunk result count must be positive");
    const queryEmbedding = await prepareMemoryQuery(owner.value.settings, query, owner.value.searchConfig);
    return this.queryChunks(ownerKey, owner.value, memory, query, count, queryEmbedding);
  }

  /** Ranks document chunks with the query vector already prepared for this operation. */
  private async queryChunks(ownerKey: string, owner: Owner, memory: Memory, query: string, count: number, queryEmbedding: SourceEmbedding | null): Promise<{ total: number; chunks: DocumentChunk[] }> {
    return searchDocumentChunks(this.memoryDatabase, ownerKey, memory, query, count, owner.searchConfig, queryEmbedding);
  }

  /** Imports complete document-aware records, relationships, settings and USER text in this operation. */
  async importMemorySpace(ownerKey: string, imported: MemorySpace, strategy: "SKIP" | "UPDATE" | "CREATE_NEW"): Promise<MemoryImportResult> {
    assertMemorySpace(imported);
    const space = await this.readMemorySpace(ownerKey), result = { newMemories: 0, updatedMemories: 0, skippedMemories: 0, newLinks: 0 };
    const idMapping = new Map<string, string>();
    for (const memory of imported.memories) {
      const existing = space.memories.find(
        /** Matches the import's actual UUID, not a guessed title. */
        item => item.uuid === memory.uuid,
      );
      if (existing !== undefined && strategy === "SKIP") { result.skippedMemories += 1; idMapping.set(memory.id, existing.id); continue; }
      const value = clone(memory);
      if (existing !== undefined && strategy === "UPDATE") { value.id = existing.id; Object.assign(existing, value); result.updatedMemories += 1; }
      else {
        if (strategy === "CREATE_NEW") { const identity = uuid(); value.id = identity; value.uuid = identity; }
        else {
          if (space.memories.some(
            /** Rejects an occupied imported identity instead of remapping it silently. */
            item => item.id === value.id,
          )) throw new Error("Imported memory id is occupied: " + value.id);
        }
        space.memories.push(value); result.newMemories += 1;
      }
      idMapping.set(memory.id, value.id);
      space.chunks = space.chunks.filter(
        /** Replaces only the explicit updated document's prior chunks. */
        chunk => chunk.memoryUuid !== value.uuid,
      );
      for (const chunk of imported.chunks) if (chunk.memoryUuid === memory.uuid) {
        const incoming = { ...clone(chunk), memoryUuid: value.uuid, id: strategy === "CREATE_NEW" ? uuid() : chunk.id };
        if (space.chunks.some(
          /** Rejects duplicate chunk identities in the target space. */
          record => record.id === incoming.id,
        )) throw new Error("Imported chunk id is occupied: " + incoming.id);
        space.chunks.push(incoming);
      }
    }
    for (const link of imported.links) {
      const source = idMapping.get(link.sourceMemoryId), target = idMapping.get(link.targetMemoryId);
      if (source === undefined || target === undefined) throw new Error("Imported link has no mapped endpoint");
      const existing = space.links.find(
        /** Resolves only a matching relationship identity in non-copy imports. */
        record => record.id === link.id,
      );
      if (existing !== undefined && strategy === "SKIP") continue;
      const incoming = { ...clone(link), sourceMemoryId: source, targetMemoryId: target, id: strategy === "CREATE_NEW" ? uuid() : link.id };
      if (existing !== undefined && strategy === "UPDATE") Object.assign(existing, incoming);
      else { space.links.push(incoming); result.newLinks += 1; }
    }
    space.settings = clone(imported.settings); space.searchConfig = clone(imported.searchConfig);
    space.rebuildProgress = clone(imported.rebuildProgress); space.rebuildTask = clone(imported.rebuildTask); space.embeddings = [];
    for (const candidate of imported.candidates) {
      const incoming = { ...clone(candidate), id: strategy === "CREATE_NEW" ? uuid() : candidate.id };
      const existing = space.candidates.find(
        /** Resolves a candidate's actual persistent identity. */
        record => record.id === incoming.id,
      );
      if (existing === undefined) space.candidates.push(incoming);
      else if (strategy === "UPDATE") Object.assign(existing, incoming);
    }
    await this.writeMemorySpace(ownerKey, space);
    return result;
  }
}
