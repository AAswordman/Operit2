import type { CharacterMemoryDatabase } from "../storage/memory";
import type { Storage } from "../../../../../types/storage";
import type { Memory, DocumentChunk, MemoryLink, MemorySearchOptions } from "../model";
import type { SourceEmbedding } from "../memory-jobs/embeddings";
import { VectorIndex } from "./usearch";
import { TitleIndex } from "./text-index";

interface Envelope<T> { ownerKey: string; record: T; embedding?: SourceEmbedding | null; embeddingTextValid?: boolean }
interface Source { identity: string; parent: string; label: bigint; group: string | null; collection: "memories" | "chunks"; version: string; id: string }
interface Group { index: VectorIndex; sources: Map<bigint, Source>; parents: Map<string, number> }
export interface Scope { ids: Set<string>; restricted: boolean }
const compare = (a: string, b: string) => a < b ? -1 : a > b ? 1 : 0;
const groupKey = (embedding: SourceEmbedding) => JSON.stringify([embedding.space, embedding.vector.length]);
function addTo(map: Map<string, Set<string>>, key: string, value: string): void {
  let values = map.get(key); if (!values) { values = new Set(); map.set(key, values); } values.add(value);
}
function removeFrom(map: Map<string, Set<string>>, key: string, value: string): void {
  const values = map.get(key); if (values) { values.delete(value); if (!values.size) map.delete(key); }
}
function folders(memory: Memory): string[] {
  if (memory.folderPath === null) return [];
  const parts = memory.folderPath.split("/"); return parts.map((_part, index) => parts.slice(0, index + 1).join("/"));
}

/** Derived, owner-local indexes. SQLite remains the synchronized source of truth. */
export class MemoryIndex {
  readonly memories = new Map<string, Memory>();
  readonly chunks = new Map<string, DocumentChunk>();
  readonly parents = new Map<string, string>();
  readonly all = new Set<string>();
  readonly titles = new TitleIndex();
  readonly sources = new Map<string, Source>();
  readonly units = new Map<string, Set<string>>();
  readonly chunkIds = new Map<string, Set<string>>();
  private readonly folderIds = new Map<string, Set<string>>();
  private readonly dates: { time: number; id: string }[] = [];
  private readonly groups = new Map<string, Group>();
  private readonly invalid = new Map<string, number>();
  private nextLabel = 1n;
  private cursor: string | null = null;
  private initialized = false;

  constructor(readonly database: CharacterMemoryDatabase, readonly ownerKey: string) {}

  close(): void { for (const group of this.groups.values()) group.index.close(); this.groups.clear(); }
  /** Captures the journal before initial paging, then catches writes made during the build. */
  async refresh(): Promise<boolean> {
    if (!this.initialized) {
      this.cursor = (await this.database.changes(null)).cursor;
      for (const collection of ["memories", "chunks"]) {
        let after = "";
        for (;;) {
          const raw = await this.database.query("SELECT key,version,json_remove(value,'$.record.content','$.record.tags','$.record.properties','$.embedding.text') AS value,json_extract(value,'$.embedding.text')=CASE WHEN json_extract(value,'$.record.isDocumentNode') THEN json_extract(value,'$.record.title') ELSE json_extract(value,'$.record.content') END AS valid FROM plugin_objects WHERE collection=? AND key>=? AND key<? AND key>? ORDER BY key LIMIT 1000", [collection, this.ownerKey + "/", this.ownerKey + "0", after]);
          const rows = raw.map(row => ({ key: String(row.key), version: String(row.version), value: { ...JSON.parse(String(row.value)), embeddingTextValid: row.valid === 1 } }));
          for (const row of rows) await this.apply(collection, row.key, row);
          if (rows.length < 1000) break;
          after = rows[rows.length - 1].key;
        }
      }
      this.dates.sort((a, b) => a.time - b.time || compare(a.id, b.id)); this.initialized = true;
    }
    for (;;) {
      const page = await this.database.changes(this.cursor);
      if (page.reset) return false;
      for (const change of page.changes) if (change.key.startsWith(this.ownerKey + "/")) {
        await this.apply(change.collection, change.key, change.entry as Storage.Entry<Envelope<Memory | DocumentChunk | MemoryLink>> | null);
      }
      this.cursor = page.cursor;
      if (!page.hasMore) return true;
    }
  }
  private datePosition(time: number, id: string): number {
    let low = 0, high = this.dates.length;
    while (low < high) {
      const middle = (low + high) >>> 1, item = this.dates[middle];
      if (item.time < time || (item.time === time && compare(item.id, id) < 0)) low = middle + 1; else high = middle;
    }
    return low;
  }
  private removeSource(identity: string): void {
    const old = this.sources.get(identity); if (!old) return;
    removeFrom(this.units, old.parent, identity);
    if (old.group !== null) {
      const group = this.groups.get(old.group)!; group.index.remove(old.label); group.sources.delete(old.label);
      const count = group.parents.get(old.parent)! - 1;
      if (count) group.parents.set(old.parent, count); else group.parents.delete(old.parent);
    } else {
      const count = this.invalid.get(old.parent)! - 1;
      if (count) this.invalid.set(old.parent, count); else this.invalid.delete(old.parent);
    }
    this.sources.delete(identity);
  }
  async apply(collection: string, key: string, entry: Storage.Entry<Envelope<Memory | DocumentChunk | MemoryLink>> | null): Promise<void> {
    if (collection !== "memories" && collection !== "chunks") return;
    const id = key.slice(this.ownerKey.length + 1), identity = collection + "/" + id;
    this.removeSource(identity);
    if (collection === "memories") {
      const old = this.memories.get(id);
      if (old) {
        this.titles.remove(id, old.title); this.parents.delete(old.uuid); this.memories.delete(id); this.all.delete(id);
        for (const folder of folders(old)) removeFrom(this.folderIds, folder, id);
        const offset = this.datePosition(old.createdAt, id);
        if (this.dates[offset]?.id === id) this.dates.splice(offset, 1);
      }
    } else {
      const old = this.chunks.get(id); if (old) { removeFrom(this.chunkIds, old.memoryUuid, id); this.chunks.delete(id); }
    }
    if (!entry) return;
    const record = entry.value.record as Memory | DocumentChunk;
    const parent = collection === "memories" ? (record as Memory).uuid : (record as DocumentChunk).memoryUuid;
    if (collection === "memories") {
      const memory = record as Memory;
      this.memories.set(id, { ...memory, content: "", tags: [], properties: [] }); this.parents.set(parent, id);
      if (memory.title === ".folder_placeholder") return;
      this.all.add(id); for (const folder of folders(memory)) addTo(this.folderIds, folder, id);
      const date = { time: memory.createdAt, id };
      if (this.initialized) this.dates.splice(this.datePosition(memory.createdAt, id), 0, date); else this.dates.push(date);
      this.titles.put(id, memory.title);
    } else {
      this.chunks.set(id, { ...record as DocumentChunk, content: "" }); addTo(this.chunkIds, parent, id);
    }
    const expected = collection === "memories" && (record as Memory).isDocumentNode ? (record as Memory).title : record.content;
    const embedding = entry.value.embedding;
    const valid = embedding != null && (entry.value.embeddingTextValid ?? embedding.text === expected) && embedding.vector.length > 0 && embedding.vector.every(Number.isFinite);
    const source: Source = { identity, parent, label: this.nextLabel++, group: valid ? groupKey(embedding) : null, collection, id, version: entry.version };
    if (valid) {
      let group = this.groups.get(source.group!);
      if (!group) { group = { index: await VectorIndex.open(embedding.vector.length), sources: new Map(), parents: new Map() }; this.groups.set(source.group!, group); }
      group.index.add(source.label, embedding.vector); group.sources.set(source.label, source); group.parents.set(parent, (group.parents.get(parent) ?? 0) + 1);
    } else this.invalid.set(parent, (this.invalid.get(parent) ?? 0) + 1);
    this.sources.set(identity, source); addTo(this.units, parent, identity);
  }
  scope(options: MemorySearchOptions): Scope {
    let ids = this.all, restricted = false;
    if (options.folderPath !== null) { ids = this.folderIds.get(options.folderPath) ?? new Set(); restricted = true; }
    if (options.createdAtStartMs !== null || options.createdAtEndMs !== null) {
      const start = options.createdAtStartMs === null ? 0 : this.datePosition(options.createdAtStartMs, "");
      const end = options.createdAtEndMs === null ? this.dates.length : this.datePosition(options.createdAtEndMs + 1, "");
      ids = new Set(this.dates.slice(start, end).filter(item => ids.has(item.id)).map(item => item.id)); restricted = true;
    }
    return { ids, restricted };
  }
  parentId(source: Source): string | undefined { return this.parents.get(source.parent); }
  /** Streams only relationships touching the selected retrieval seeds. */
  async *seedLinks(ids: string[]): AsyncGenerator<MemoryLink> {
    if (ids.length === 0) return;
    let after = "";
    for (;;) {
      const rows = await this.database.select<Envelope<MemoryLink>>("links", this.ownerKey, "(json_extract(value,'$.record.sourceMemoryId') IN (SELECT value FROM json_each(?)) OR json_extract(value,'$.record.targetMemoryId') IN (SELECT value FROM json_each(?)))", [JSON.stringify(ids), JSON.stringify(ids)], 1000, after);
      for (const row of rows) yield row.value.record;
      if (rows.length < 1000) return;
      after = rows[rows.length - 1].key;
    }
  }
  /** Streams only matching identities and term bitsets; source bodies remain in SQLite. */
  async textMatches(terms: readonly string[], collection: string, tags = false, parent?: string): Promise<Map<string, number>> {
    const result = new Map<string, number>();
    if (terms.length === 0) return result;
    const patterns = JSON.stringify(terms.map(term => "*" + [...term].map(char => { const upper = char.toUpperCase(); return upper !== char && [...upper].length === 1 ? "[" + char + upper + "]" : char; }).join("") + "*"));
    const fields = tags ? ["json_extract(tag.value,'$.name')"] : collection === "memories" ? ["json_extract(o.value,'$.record.title')", "json_extract(o.value,'$.record.content')"] : ["json_extract(o.value,'$.record.content')"];
    const match = fields.map(field => field + " GLOB term.value").join(" OR ");
    let after = "";
    for (;;) {
      const rows = await this.database.query("SELECT o.key,SUM(DISTINCT (1 << CAST(term.key AS INTEGER))) AS bits FROM plugin_objects o JOIN json_each(?) term" + (tags ? " JOIN json_each(json_extract(o.value,'$.record.tags')) tag" : "") + " WHERE o.collection=? AND o.key>=? AND o.key<? AND o.key>?" + (parent === undefined ? "" : " AND json_extract(o.value,'$.record.memoryUuid')=?") + " AND (" + match + ") GROUP BY o.key ORDER BY o.key LIMIT 1000", [patterns, collection, this.ownerKey + "/", this.ownerKey + "0", after, ...(parent === undefined ? [] : [parent])]);
      for (const row of rows) result.set(String(row.key).slice(this.ownerKey.length + 1), Number(row.bits) >>> 0);
      if (rows.length < 1000) return result;
      after = String(rows[rows.length - 1].key);
    }
  }
  /** Combines each term once per parent while preserving the existing keyword and tag ranks. */
  async matching(terms: readonly string[], scope: Scope, keywordsEnabled = true, tagsEnabled = true): Promise<{ keywords: Map<string, number>; tags: Map<string, number> }> {
    const keywords = new Map<string, number>(), tags = new Map<string, number>();
    for (const [collection, enabled, tagged] of [["memories", keywordsEnabled, false], ["chunks", keywordsEnabled, false], ["memories", tagsEnabled, true]] as const) {
      if (!enabled) continue;
      const target = tagged ? tags : keywords;
      for (const [sourceId, bits] of await this.textMatches(terms, collection, tagged)) {
        const id = collection === "chunks" ? this.parents.get(this.chunks.get(sourceId)!.memoryUuid) : sourceId;
        if (id !== undefined && scope.ids.has(id)) target.set(id, ((target.get(id) ?? 0) | bits) >>> 0);
      }
    }
    for (const counts of [keywords, tags]) for (const [id, bits] of counts) { let value = bits, count = 0; while (value) { value = (value & (value - 1)) >>> 0; count++; } counts.set(id, count); }
    return { keywords, tags };
  }
  validate(query: SourceEmbedding, scope: Scope, parent?: string): void {
    const relevant = (uuid: string) => parent === undefined ? scope.ids.has(this.parents.get(uuid) ?? "") : uuid === parent;
    for (const uuid of this.invalid.keys()) if (relevant(uuid)) throw new Error("Memory vector index is incomplete or stale; rebuild memory embeddings");
    const expected = groupKey(query);
    for (const [name, group] of this.groups) if (name !== expected) {
      for (const uuid of group.parents.keys()) if (relevant(uuid)) throw new Error("Memory vector provider dimensions changed; rebuild memory embeddings");
    }
  }
  semantic(query: SourceEmbedding, scope: Scope, count: number, excluded: Set<string>, parent?: string): Map<string, number> {
    const result = new Map<string, number>(), group = this.groups.get(groupKey(query)); if (!group) return result;
    let allowed: bigint[] | undefined;
    if (scope.restricted || parent !== undefined) {
      allowed = [];
      const uuids = parent === undefined ? [...scope.ids].map(id => this.memories.get(id)!.uuid) : [parent];
      for (const uuid of uuids) for (const identity of this.units.get(uuid) ?? []) {
        const source = this.sources.get(identity)!;
        if (source.group === groupKey(query) && (parent === undefined || source.collection === "chunks")) allowed.push(source.label);
      }
    }
    const excludedLabels: bigint[] = [];
    for (const id of excluded) {
      const uuid = this.memories.get(id)?.uuid; if (uuid === undefined) continue;
      for (const identity of this.units.get(uuid) ?? []) { const source = this.sources.get(identity)!; if (source.group === groupKey(query)) excludedLabels.push(source.label); }
    }
    // Increase the source budget when many chunks belong to the same document.
    const maximum = Math.min(group.sources.size, Math.max(512, count * 16));
    let budget = Math.min(maximum, Math.max(128, count * 4));
    for (;;) {
      const hits = group.index.search(query.vector, Math.max(1, budget), allowed, excludedLabels);
      for (const hit of hits) {
        const source = group.sources.get(hit.key)!;
        const id = parent === undefined ? this.parentId(source) : source.id;
        if (id !== undefined && (parent !== undefined || scope.ids.has(id))) result.set(id, Math.max(result.get(id) ?? -Infinity, hit.similarity));
      }
      if (result.size >= count || hits.length < budget || budget === maximum) break;
      budget = Math.min(maximum, budget * 2);
    }
    return result;
  }
  scoreCandidates(query: SourceEmbedding, ids: readonly string[], scores: Map<string, number>, chunks = false): void {
    const group = this.groups.get(groupKey(query)); if (!group) return;
    const labels: bigint[] = [], parents: string[] = [];
    for (const id of ids) {
      const identities = chunks ? ["chunks/" + id] : this.units.get(this.memories.get(id)!.uuid) ?? [];
      for (const identity of identities) { const source = this.sources.get(identity); if (source?.group === groupKey(query)) { labels.push(source.label); parents.push(id); } }
    }
    const exact = group.index.score(query.vector, labels);
    for (const id of new Set(parents)) scores.delete(id);
    exact.forEach((score, index) => scores.set(parents[index], Math.max(scores.get(parents[index]) ?? -Infinity, score)));
  }
}
interface Retained { tail: Promise<unknown>; owners: Map<string, MemoryIndex> }
const retained = new WeakMap<CharacterMemoryDatabase, Retained>();
/** Refresh and retrieval share a queue, so concurrent calls never observe a partially applied batch. */
export function withMemoryIndex<T>(database: CharacterMemoryDatabase, ownerKey: string, run: (index: MemoryIndex) => Promise<T> | T): Promise<T> {
  let state = retained.get(database);
  if (!state) { state = { tail: Promise.resolve(), owners: new Map() }; retained.set(database, state); }
  const current = state;
  const operation = current.tail.catch(() => {}).then(async () => {
    for (let attempt = 0; attempt < 3; attempt++) {
      let index = current.owners.get(ownerKey);
      if (!index) { index = new MemoryIndex(database, ownerKey); current.owners.set(ownerKey, index); }
      let ready: boolean;
      try { ready = await index.refresh(); }
      catch (error) { index.close(); current.owners.delete(ownerKey); throw error; }
      if (ready) return run(index);
      index.close(); current.owners.delete(ownerKey);
    }
    throw new Error("Memory change journal reset repeatedly during index construction");
  });
  current.tail = operation; return operation;
}
export async function closeMemoryIndexes(database: CharacterMemoryDatabase): Promise<void> {
  const state = retained.get(database); if (!state) return;
  await state.tail.catch(() => {});
  for (const index of state.owners.values()) index.close();
  retained.delete(database);
}
