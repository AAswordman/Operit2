import type { CharacterMemoryDatabase } from "../storage/memory";
import type { Storage } from "../../../../../types/storage";
import type { Memory, DocumentChunk, MemoryLink, MemorySearchOptions } from "../model";
import type { SourceEmbedding } from "../memory-jobs/embeddings";
import { VectorIndex } from "./usearch";
import { TextIndex, TitleIndex } from "./text-index";

interface Envelope<T> { ownerKey: string; record: T; embedding?: SourceEmbedding | null }
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
  readonly links = new Map<string, MemoryLink>();
  readonly adjacency = new Map<string, Set<string>>();
  readonly parents = new Map<string, string>();
  readonly all = new Set<string>();
  readonly text = new TextIndex();
  readonly tags = new TextIndex();
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
      for (const collection of ["memories", "chunks", "links"]) {
        let after = "";
        for (;;) {
          const rows = await this.database.select<Envelope<Memory | DocumentChunk | MemoryLink>>(collection, this.ownerKey, "1", [], 100, after);
          for (const row of rows) await this.apply(collection, row.key, row);
          if (rows.length < 100) break;
          after = rows[rows.length - 1].key;
        }
      }
      this.initialized = true;
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
    this.text.remove(identity); removeFrom(this.units, old.parent, identity);
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
    if (collection !== "memories" && collection !== "chunks" && collection !== "links") return;
    const id = key.slice(this.ownerKey.length + 1), identity = collection + "/" + id;
    if (collection === "links") {
      const old = this.links.get(id);
      if (old) { removeFrom(this.adjacency, old.sourceMemoryId, id); removeFrom(this.adjacency, old.targetMemoryId, id); this.links.delete(id); }
      if (entry) { const link = entry.value.record as MemoryLink; this.links.set(id, link); addTo(this.adjacency, link.sourceMemoryId, id); addTo(this.adjacency, link.targetMemoryId, id); }
      return;
    }
    this.removeSource(identity);
    if (collection === "memories") {
      const old = this.memories.get(id);
      if (old) {
        this.tags.remove(id); this.titles.remove(id, old.title); this.parents.delete(old.uuid); this.memories.delete(id); this.all.delete(id);
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
      this.memories.set(id, memory); this.parents.set(parent, id);
      if (memory.title === ".folder_placeholder") return;
      this.all.add(id); for (const folder of folders(memory)) addTo(this.folderIds, folder, id);
      this.dates.splice(this.datePosition(memory.createdAt, id), 0, { time: memory.createdAt, id });
      this.tags.put(id, memory.tags.map(tag => tag.name).join("\0")); this.titles.put(id, memory.title);
      this.text.put(identity, memory.title + "\0" + memory.content);
    } else {
      this.chunks.set(id, record as DocumentChunk); addTo(this.chunkIds, parent, id); this.text.put(identity, record.content);
    }
    const expected = collection === "memories" && (record as Memory).isDocumentNode ? (record as Memory).title : record.content;
    const embedding = entry.value.embedding;
    const valid = embedding != null && embedding.text === expected && embedding.vector.length > 0 && embedding.vector.every(Number.isFinite);
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
  matching(terms: readonly string[], scope: Scope, keywordsEnabled = true, tagsEnabled = true): { keywords: Map<string, number>; tags: Map<string, number> } {
    const keywords = new Map<string, number>(), tags = new Map<string, number>();
    for (const term of terms) {
      const parents = new Set<string>();
      if (keywordsEnabled) for (const identity of this.text.search(term)) {
        const source = this.sources.get(identity); if (!source) continue;
        const id = this.parentId(source); if (id !== undefined && scope.ids.has(id)) parents.add(id);
      }
      for (const id of parents) keywords.set(id, (keywords.get(id) ?? 0) + 1);
      if (tagsEnabled) for (const id of this.tags.search(term)) if (scope.ids.has(id)) tags.set(id, (tags.get(id) ?? 0) + 1);
    }
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
