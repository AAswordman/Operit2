import type { MemoryChanges, MemoryRecord, LinkRecord } from "../api";
import type { DocumentChunk, MemorySpace } from "../model";
import { decodeMemoryBackup } from "../backup";
import { parseMemoryIdentifier } from "../domain";
import { assertEmbedding, cosineSimilarity } from "../memory-jobs/embeddings";
import type { CallerContext, DomainCall, MemoryToolName, ToolParameters, MemoryToolResult } from "./contract";
import { formatTime, limit, optionalNumber, optionalText, requiredText, score, timeBoundary } from "./contract";

/** Preserves the original query result fields and lossless plugin relationship identities. */
export interface MemoryInfo { ownerKey: string; title: string; content: string; source: string; tags: string[]; createdAt: string; chunkInfo: string | null; chunkIndices: number[] | null }
export interface QueryResult { memories: MemoryInfo[]; snapshotId: string | null; snapshotCreated: boolean; excludedBySnapshotCount: number }
export interface LinkInfo { linkId: string; sourceTitle: string; targetTitle: string; linkType: string; weight: number; description: string }
interface Snapshot { seen: Set<string>; access: number }
/** Owns ephemeral pagination only; all records and mutations remain in the main file repository. */
export class MemoryTools {
  private readonly snapshots = new Map<string, Map<string, Snapshot>>();
  private nextSnapshot = 0;
  private access = 0;
  /** Binds executable tools to the actual typed main-runtime dispatcher. */
  constructor(private readonly domain: DomainCall) {}

  /** Resolves one exact caller identity rather than interpreting an opaque chat context key. */
  private caller(input: ToolParameters, context: CallerContext): string {
    const explicit = optionalText(input, "caller_card_id");
    if (explicit !== undefined) {
      if (explicit.trim() === "") throw new Error("caller_card_id must not be blank");
      if (context.participantId !== null && explicit !== context.participantId) throw new Error("caller_card_id does not match the authenticated execution participant");
      return explicit;
    }
    if (context.participantId === null) throw new Error("caller_card_id or target_owner_key parameter is required");
    return context.participantId;
  }
  /** Resolves the actual persisted binding through the same domain service. */
  private async boundOwner(participantId: string): Promise<string> {
    const owner = await this.domain("memory.resolveOwner", { characterId: participantId });
    switch (owner.kind) {
      case "CHARACTER": return "character:" + owner.id;
      case "SHARED": return "shared:" + owner.id;
      default: throw new Error("Invalid memory owner kind");
    }
  }
  /** Validates an explicit owner, and enforces the selected participant's actual mount permissions. */
  private async owner(input: ToolParameters, context: CallerContext, write: boolean): Promise<string> {
    const explicit = optionalText(input, "target_owner_key");
    if (explicit !== undefined && !/^(character|shared):[^:\s]+$/.test(explicit)) throw new Error("Invalid target_owner_key");
    const selected = explicit === undefined ? await this.boundOwner(this.caller(input, context)) : explicit;
    if (context.participantId !== null) {
      const primary = await this.boundOwner(context.participantId);
      if (selected !== primary) {
        const card = await this.domain("character.get", { id: context.participantId });
        const permitted = card.sharedMemoryMounts.some(
          /** Tests the real persisted mount key and requested effect. */
          mount => "shared:" + mount.sharedMemoryId === selected && (write ? mount.writable : mount.readable),
        );
        if (!permitted) throw new Error("Memory owner access denied: " + selected);
      }
    }
    return selected;
  }
  /** Reads a validated complete repository export; no private copy of durable data is retained. */
  private async space(ownerKey: string): Promise<MemorySpace> {
    const exported = await this.domain("memory.export", { ownerKey });
    const space = decodeMemoryBackup(exported.content).space;
    if (space.ownerKey !== ownerKey) throw new Error("Memory export owner does not match the requested owner");
    return space;
  }
  /** Projects the historical model-facing result, keeping record identities internal to the plugin. */
  private info(ownerKey: string, memory: MemoryRecord): MemoryInfo {
    return { ownerKey, title: memory.title, content: memory.content, source: memory.source, tags: memory.tags.map(
      /** Retains actual tag names in the historical query projection. */
      tag => tag.name,
    ), createdAt: formatTime(memory.createdAt), chunkInfo: null, chunkIndices: null };
  }
  /** Formats the original five-chunk document result with one-based display indices. */
  private formatChunks(title: string, total: number, chunks: DocumentChunk[]): string {
    return "Document: " + title + "\n" + chunks.slice(0, 5).map(
      /** Uses real persisted chunk indices instead of reindexing the selected subset. */
      chunk => "Chunk " + (chunk.chunkIndex + 1) + "/" + total + ":\n" + chunk.content,
    ).join("\n---\n");
  }
  /** Reads only a committed main-service embedding; tool projections never mutate a detached export cache.
   * A missing or duplicate entry is a consistency error rather than permission to compute outside the main repository.
   */
  private committedEmbedding(space: MemorySpace, text: string): number[] {
    const endpoint = space.settings.cloudEmbeddingEndpoint.replace(/\/+$/, "");
    const model = space.settings.cloudEmbeddingModel;
    if (!space.settings.cloudEmbeddingEnabled) throw new Error("Cloud embedding is disabled");
    const matches = space.embeddings.filter(
      /** Selects the exact persisted provider and input provenance. */
      item => item.endpoint === endpoint && item.model === model && item.text === text,
    );
    if (matches.length !== 1) throw new Error("Embedding is missing or duplicated in the committed main-service cache: " + text);
    assertEmbedding(matches[0]);
    return matches[0].vector;
  }
  /** Combines lexical coverage and genuine committed embedding channels with the original chunk RRF weights. */
  private async searchChunks(space: MemorySpace, memory: MemoryRecord, query: string, count: number): Promise<DocumentChunk[]> {
    const chunks = space.chunks.filter(
      /** Selects the complete real document by UUID. */
      chunk => chunk.memoryUuid === memory.uuid,
    ).sort(
      /** Preserves the persisted document order. */
      (left, right) => left.chunkIndex - right.chunkIndex,
    );
    if (query.trim() === "*" || query.trim() === "") return chunks.slice(0, count);
    const keywords = query.split(query.indexOf("|") === -1 ? /\s+/u : /\|/u).map(
      /** Preserves the explicitly authored semantic channels. */
      word => word.trim(),
    ).filter(
      /** Rejects empty separators before requesting real embeddings. */
      word => word.length !== 0,
    );
    const patterns = [...new Set(keywords)].map(
      /** Implements authored wildcard spelling using escaped literal segments. */
      word => new RegExp(word.split("*").map(
        /** Escapes one exact authored keyword segment. */
        segment => segment.replace(/[.*+?^$\{\}()|[\]\\]/g, "\\$&"),
      ).join(".*"), "isu"),
    );
    let keywordMultiplier: number, vectorMultiplier: number;
    switch (space.searchConfig.scoreMode) {
      case "BALANCED": keywordMultiplier = 1; vectorMultiplier = 1; break;
      case "KEYWORD_FIRST": keywordMultiplier = 1.3; vectorMultiplier = 0.8; break;
      case "SEMANTIC_FIRST": keywordMultiplier = 0.8; vectorMultiplier = 1.3; break;
      default: throw new Error("Invalid memory scoring mode");
    }
    const scores = new Map<string, { chunk: DocumentChunk; score: number }>();
    if (space.searchConfig.keywordWeight > 0) {
      const lexical = chunks.map(
        /** Measures exact lexical coverage in each real persisted chunk. */
        chunk => ({ chunk, hits: patterns.filter(
          /** Matches document content against the declared lexical expression. */
          pattern => pattern.test(chunk.content),
        ).length }),
      ).filter(
        /** Includes only genuine lexical matches in the coverage channel. */
        item => item.hits > 0,
      ).sort(
        /** Orders coverage ties by original document position. */
        (left, right) => right.hits - left.hits || left.chunk.chunkIndex - right.chunk.chunkIndex,
      );
      for (let rank = 0; rank < lexical.length; rank += 1) {
        const item = lexical[rank];
        scores.set(item.chunk.id, { chunk: item.chunk, score: space.searchConfig.keywordWeight * keywordMultiplier * (1 + 0.6 * item.hits / Math.max(1, patterns.length)) / (61 + rank) });
      }
    }
    if (space.searchConfig.vectorWeight > 0) {
      const normalization = 1 / Math.sqrt(Math.max(1, keywords.length));
      for (const keyword of keywords) {
        await this.domain("memory.searchWithOptions", { ownerKey: space.ownerKey, query: keyword, folderPath: null, relevanceThreshold: 0, createdAtStartMs: null, createdAtEndMs: null });
        const committed = await this.space(space.ownerKey), queryVector = this.committedEmbedding(committed, keyword);
        const semantic: { chunk: DocumentChunk; similarity: number }[] = [];
        for (const chunk of chunks) semantic.push({ chunk, similarity: cosineSimilarity(queryVector, this.committedEmbedding(committed, chunk.content)) });
        semantic.sort(
          /** Ranks actual provider similarities with a stable persisted chunk tie breaker. */
          (left, right) => right.similarity - left.similarity || left.chunk.chunkIndex - right.chunk.chunkIndex,
        );
        for (let rank = 0; rank < semantic.length; rank += 1) {
          const item = semantic[rank], contribution = (1 / (61 + rank) + item.similarity * space.searchConfig.vectorWeight * vectorMultiplier) * normalization;
          const entry = scores.get(item.chunk.id);
          if (entry === undefined) scores.set(item.chunk.id, { chunk: item.chunk, score: contribution });
          else entry.score += contribution;
        }
      }
    }
    return [...scores.values()].sort(
      /** Preserves original RRF ordering and lossless numeric identity ties. */
      (left, right) => right.score - left.score || (BigInt(left.chunk.id) < BigInt(right.chunk.id) ? -1 : BigInt(left.chunk.id) > BigInt(right.chunk.id) ? 1 : 0),
    ).slice(0, count).map(
      /** Returns complete original chunk records rather than a synthetic projection. */
      item => item.chunk,
    );
  }
  /** Allocates or reuses owner-scoped pagination under the original snapshot contract. */
  private snapshot(ownerKey: string, requested: string | undefined): { id: string; state: Snapshot; created: boolean } {
    let snapshots = this.snapshots.get(ownerKey);
    if (snapshots === undefined) { snapshots = new Map(); this.snapshots.set(ownerKey, snapshots); }
    const id = requested === undefined ? "memory-query-" + (++this.nextSnapshot) : requested;
    if (id.trim() === "") throw new Error("snapshot_id must not be blank");
    let state = snapshots.get(id); const created = state === undefined;
    if (state === undefined) { state = { seen: new Set(), access: ++this.access }; snapshots.set(id, state); }
    state.access = ++this.access;
    while (snapshots.size > 32) {
      let oldestKey: string | undefined, oldest = Infinity;
      for (const [key, item] of snapshots) if (key !== id && item.access < oldest) { oldest = item.access; oldestKey = key; }
      if (oldestKey === undefined) throw new Error("Query snapshot index is inconsistent");
      snapshots.delete(oldestKey);
    }
    return { id, state, created };
  }
  /** Resolves a relationship by its exact lossless ID or an unambiguous title pair. */
  private link(space: MemorySpace, input: ToolParameters): LinkRecord {
    const id = optionalText(input, "link_id");
    let matches: LinkRecord[];
    if (id !== undefined) matches = space.links.filter(
      /** Matches the validated string identity without a floating-point conversion. */
      link => link.id === parseMemoryIdentifier(id, "link_id"),
    );
    else {
      const source = this.memory(space, requiredText(input, "source_title")), target = this.memory(space, requiredText(input, "target_title"));
      const type = optionalText(input, "link_type");
      matches = space.links.filter(
        /** Uses the exact directed title pair and optional relation type. */
        link => link.sourceMemoryId === source.id && link.targetMemoryId === target.id && (type === undefined || link.type_ === type),
      );
    }
    if (matches.length === 0) throw new Error("No matching link found");
    if (matches.length !== 1) throw new Error("Multiple links matched. Provide link_id or a more specific link_type.");
    return matches[0];
  }
  /** Requires an exact title and rejects ambiguous records instead of choosing one silently. */
  private memory(space: MemorySpace, title: string): MemoryRecord {
    const matches = space.memories.filter(
      /** Preserves exact title lookup semantics. */
      memory => memory.title === title,
    );
    if (matches.length === 0) throw new Error("Memory not found with title: " + title);
    if (matches.length !== 1) throw new Error("Multiple memories matched title: " + title);
    return matches[0];
  }
  /** Resolves complete relationship display fields from the authoritative same-owner records. */
  private linkInfo(space: MemorySpace, link: LinkRecord): LinkInfo {
    const source = space.memories.find(
      /** Resolves the real source identity. */
      memory => memory.id === link.sourceMemoryId,
    ), target = space.memories.find(
      /** Resolves the real target identity. */
      memory => memory.id === link.targetMemoryId,
    );
    if (source === undefined || target === undefined) throw new Error("Memory link refers to a missing record: " + link.id);
    return { linkId: link.id, sourceTitle: source.title, targetTitle: target.title, linkType: link.type_, weight: link.weight, description: link.description };
  }
  /** Executes all twelve migrated tools without a Core domain executor or persistence shortcut. */
  async execute<N extends MemoryToolName>(operation: N, input: ToolParameters, context: CallerContext): Promise<MemoryToolResult<N>>;
  /** Executes the selected tool using the actual shared memory domain service. */
  async execute(operation: MemoryToolName, input: ToolParameters, context: CallerContext): Promise<MemoryToolResult> {
    if (operation === "get_memory_owner_key") return this.boundOwner(this.caller(input, context));
    const write = !["query_memory", "get_memory_by_title", "query_memory_links"].some(
      /** Classifies only the declared read tools. */
      name => name === operation,
    );
    const ownerKey = await this.owner(input, context, write);
    switch (operation) {
      case "query_memory": return this.query(ownerKey, input);
      case "get_memory_by_title": return this.byTitle(ownerKey, input);
      case "create_memory": {
        const contentType = optionalText(input, "content_type"), source = optionalText(input, "source"), folderPath = optionalText(input, "folder_path"), tags = optionalText(input, "tags");
        const result = await this.domain("memory.create", { ownerKey, values: { title: requiredText(input, "title"), content: requiredText(input, "content"), contentType: contentType === undefined ? "text/plain" : contentType, source: source === undefined ? "ai_created" : source, folderPath: folderPath === undefined ? null : folderPath, tags: tags === undefined ? [] : this.tags(tags) } });
        return "Successfully created memory: '" + result.item.title + "' (UUID: " + result.item.uuid + ")";
      }
      case "update_memory": {
        const oldTitle = requiredText(input, "old_title"), current = await this.domain("memory.get", { ownerKey, title: oldTitle }), changes: MemoryChanges = {};
        for (const [parameter, field] of [["new_title", "title"], ["content", "content"], ["content_type", "contentType"], ["source", "source"], ["folder_path", "folderPath"]] as const) {
          const value = optionalText(input, parameter); if (value !== undefined) changes[field] = value;
        }
        for (const field of ["credibility", "importance"] as const) { const value = score(input, field); if (value !== undefined) changes[field] = value; }
        const tags = optionalText(input, "tags"); if (tags !== undefined) changes.tags = this.tags(tags);
        if (Object.keys(changes).length === 0) throw new Error("At least one memory change must be provided");
        const updated = await this.domain("memory.update", { ownerKey, originalTitle: oldTitle, expectedId: current.item.id, changes });
        return "Successfully updated memory from '" + oldTitle + "' to '" + updated.item.title + "'";
      }
      case "delete_memory": {
        const title = requiredText(input, "title"), memory = await this.domain("memory.get", { ownerKey, title });
        await this.domain("memory.delete", { ownerKey, id: memory.item.id }); return "Successfully deleted memory: '" + title + "'";
      }
      case "move_memory": return this.move(ownerKey, input);
      case "update_user_preferences": await this.domain("memory.user.write", { ownerKey, content: requiredText(input, "content") }); return "Successfully updated USER.md";
      case "link_memories": {
        const sourceTitle = requiredText(input, "source_title"), targetTitle = requiredText(input, "target_title"), linkType = optionalText(input, "link_type"), weight = score(input, "weight"), description = optionalText(input, "description");
        const result = await this.domain("memory.link.create", { ownerKey, sourceTitle, targetTitle, linkType: linkType === undefined ? "related" : linkType, weight: weight === undefined ? 0.5 : weight, description: description === undefined ? "" : description });
        return { sourceTitle, targetTitle, linkType: result.link.type_, weight: result.link.weight, description: result.link.description };
      }
      case "query_memory_links": return this.links(ownerKey, input);
      case "update_memory_link": {
        const space = await this.space(ownerKey), link = this.link(space, input), changes: { linkType?: string; weight?: number; description?: string } = {};
        const type = optionalText(input, "new_link_type"), weight = score(input, "weight"), description = optionalText(input, "description");
        if (type !== undefined) { if (type.trim() === "") throw new Error("new_link_type must not be blank"); changes.linkType = type; }
        if (weight !== undefined) changes.weight = weight; if (description !== undefined) changes.description = description;
        if (Object.keys(changes).length === 0) throw new Error("At least one of new_link_type, weight, description must be provided");
        const result = await this.domain("memory.link.update", { ownerKey, linkId: link.id, changes });
        return { totalCount: 1, links: [this.linkInfo(space, result.link)] };
      }
      case "delete_memory_link": {
        const link = this.link(await this.space(ownerKey), input); await this.domain("memory.link.delete", { ownerKey, linkId: link.id }); return "Successfully deleted memory link: " + link.id;
      }
      default: throw new Error("Unknown memory tool: " + operation);
    }
  }
  /** Splits the original comma-delimited tag contract without touching stored tag identities. */
  private tags(value: string): string[] { return value.split(",").map(
    /** Normalizes only delimiters and surrounding authored tag whitespace. */
    tag => tag.trim(),
  ).filter(
    /** Excludes empty tag entries as declared by the tool contract. */
    tag => tag !== "",
  ); }
  /** Performs filtered full-record search and commits pagination only after every document read succeeds. */
  private async query(ownerKey: string, input: ToolParameters): Promise<QueryResult> {
    const query = requiredText(input, "query"), count = limit(input, query.trim() === "*" ? Number.MAX_SAFE_INTEGER : 20), threshold = optionalNumber(input, "threshold"), folder = optionalText(input, "folder_path");
    if (threshold !== undefined && threshold < 0) throw new Error("Invalid threshold. Expected number >= 0.");
    const start = timeBoundary(input, "start_time"), end = timeBoundary(input, "end_time");
    if (start !== null && end !== null && start > end) throw new Error("start_time must not be after end_time");
    const result = await this.domain("memory.searchWithOptions", { ownerKey, query, folderPath: folder === undefined ? null : folder, relevanceThreshold: threshold === undefined ? 0 : threshold, createdAtStartMs: start, createdAtEndMs: end });
    if (result.ownerKey !== ownerKey) throw new Error("Memory search owner does not match the requested owner");
    const found = result.items, space = await this.space(ownerKey);
    const snapshot = this.snapshot(ownerKey, optionalText(input, "snapshot_id"));
    const unseen = found.filter(
      /** Excludes only records already delivered from this same owner and snapshot. */
      memory => !snapshot.state.seen.has(ownerKey + ":" + memory.id),
    ), returned = unseen.slice(0, count);
    const memories: MemoryInfo[] = [];
    for (const memory of returned) {
        const info = this.info(ownerKey, memory);
        if (memory.isDocumentNode) {
          const total = space.chunks.filter(
            /** Counts every real chunk belonging to this document. */
            chunk => chunk.memoryUuid === memory.uuid,
          ).length, chunks = await this.searchChunks(space, memory, query, Math.min(count, 20));
          info.content = query.trim() === "*" || count > 20 ? "Document: " + memory.title + " (" + total + " chunks)" : this.formatChunks(memory.title, total, chunks);
          if (chunks.length !== 0) { info.chunkInfo = "Chunks " + chunks.slice(0, 5).map(
            /** Displays selected indices using the historical one-based spelling. */
            chunk => chunk.chunkIndex + 1,
          ).join(", ") + "/" + total; info.chunkIndices = chunks.map(
            /** Returns actual zero-based persisted indices for subsequent tool calls. */
            chunk => chunk.chunkIndex,
          ); }
        }
        memories.push(info);
    }
    for (const memory of returned) snapshot.state.seen.add(ownerKey + ":" + memory.id);
    return { memories, snapshotId: snapshot.id, snapshotCreated: snapshot.created, excludedBySnapshotCount: found.length - unseen.length };
  }
  /** Reads an exact title or validates the original one-based document query/index/range parameters. */
  private async byTitle(ownerKey: string, input: ToolParameters): Promise<QueryResult | string> {
    const title = requiredText(input, "title"), result = await this.domain("memory.get", { ownerKey, title }), memory = result.item;
    if (!memory.isDocumentNode) return { memories: [this.info(ownerKey, memory)], snapshotId: null, snapshotCreated: false, excludedBySnapshotCount: 0 };
    const space = await this.space(ownerKey), chunks = space.chunks.filter(
      /** Selects only this exact real document. */
      chunk => chunk.memoryUuid === memory.uuid,
    ).sort(
      /** Retains stable document order. */
      (left, right) => left.chunkIndex - right.chunkIndex,
    ), total = chunks.length;
    const query = optionalText(input, "query"), index = optionalNumber(input, "chunk_index"), range = optionalText(input, "chunk_range");
    const supplied = [query !== undefined, index !== undefined, range !== undefined].filter(
      /** Counts explicitly supplied selectors instead of guessing precedence. */
      value => value,
    ).length;
    if (supplied === 0) return "Document: " + title + " (" + total + " chunks). Specify query, chunk_range (1-based start-end) or chunk_index (1-based).";
    if (supplied !== 1) throw new Error("Specify exactly one of query, chunk_range or chunk_index");
    let selected: DocumentChunk[];
    if (query !== undefined) { if (query.trim() === "") throw new Error("query must not be blank"); selected = await this.searchChunks(space, memory, query, limit(input, 20)); }
    else {
      let start: number, end: number;
      if (range !== undefined) { const parsed = /^(\d+)-(\d+)$/.exec(range); if (parsed === null) throw new Error("Invalid chunk_range. Expected start-end"); start = Number(parsed[1]); end = Number(parsed[2]); }
      else { if (index === undefined) throw new Error("Document selector is missing"); start = index; end = index; }
      if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 1 || end > total || start > end) throw new Error("Chunk range out of bounds. Valid range: 1-" + total);
      selected = chunks.filter(
        /** Selects the validated one-based range using persisted zero-based indices. */
        chunk => chunk.chunkIndex >= start - 1 && chunk.chunkIndex <= end - 1,
      );
    }
    if (selected.length === 0) throw new Error("No matching chunks found");
    return this.formatChunks(title, total, selected);
  }
  /** Moves the exact title/folder intersection through a single domain mutation. */
  private async move(ownerKey: string, input: ToolParameters): Promise<string> {
    const target = optionalText(input, "target_folder_path"), source = optionalText(input, "source_folder_path"), titles = optionalText(input, "titles");
    if (target === undefined) throw new Error("target_folder_path parameter is required");
    if (source === undefined && titles === undefined) throw new Error("Provide titles and/or source_folder_path to select memories to move");
    const names = titles === undefined ? null : new Set(titles.split(/[,\n|]/).map(
      /** Retains the original three title delimiters. */
      title => title.trim(),
    ).filter(
      /** Rejects delimiter-only selection instead of selecting all memories. */
      title => title !== "",
    ));
    if (names !== null && names.size === 0) throw new Error("titles must select at least one memory");
    const records = await this.domain("memory.list", { ownerKey }), selected = records.items.filter(
      /** Intersects both explicit selectors without silently changing selection strategy. */
      memory => (names === null || names.has(memory.title)) && (source === undefined || (memory.folderPath === null ? "" : memory.folderPath) === source),
    );
    if (selected.length === 0) throw new Error("No matching memories found to move");
    const ids = [...new Set(selected.map(
      /** Keeps each lossless memory identity once. */
      memory => memory.id,
    ))];
    const result = await this.domain("memory.move", { ownerKey, ids, folderPath: target });
    return "Successfully moved " + result.moved + " memories to '" + target + "'";
  }
  /** Queries actual directed relationships with precise filters and the original 200-result cap. */
  private async links(ownerKey: string, input: ToolParameters): Promise<{ totalCount: number; links: LinkInfo[] }> {
    const space = await this.space(ownerKey), id = optionalText(input, "link_id"), source = optionalText(input, "source_title"), target = optionalText(input, "target_title"), type = optionalText(input, "link_type");
    if (id !== undefined) parseMemoryIdentifier(id, "link_id");
    const sourceId = source === undefined ? null : this.memory(space, source).id, targetId = target === undefined ? null : this.memory(space, target).id;
    const links = space.links.filter(
      /** Applies every supplied filter to complete canonical relation records. */
      link => (id === undefined || link.id === id) && (sourceId === null || link.sourceMemoryId === sourceId) && (targetId === null || link.targetMemoryId === targetId) && (type === undefined || link.type_ === type),
    ).slice(0, Math.min(limit(input, 20), 200)).map(
      /** Includes real relationship descriptions and unchanged string identities. */
      link => this.linkInfo(space, link),
    );
    return { totalCount: links.length, links };
  }
}
