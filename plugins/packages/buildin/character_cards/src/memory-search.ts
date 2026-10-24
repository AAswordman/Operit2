import type { DocumentChunk, Memory, MemorySearchConfig, MemorySearchOptions, MemorySpace, MemorySearchDebug, MemorySearchHit } from "./model";
import { cosineSimilarity, embeddingConfigured, embeddingFor } from "./memory-jobs/embeddings";

export const MEMORY_SEARCH_THRESHOLD = 0.025;
interface SearchDocument { id: string; texts: string[]; importance: number; updatedAt: number }
interface RankedScore { id: string; score: number }

/** Escapes regex syntax in exact authored text. */
function escapeLiteral(text: string): string { return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }

/** Escapes literal query text while retaining explicitly authored wildcard segments. */
function pattern(text: string): RegExp { return new RegExp(text.split("*").map(escapeLiteral).join(".*"), "isu"); }

/** Matches a complete literal title even when it contains wildcard punctuation. */
function literalPattern(text: string): RegExp { return new RegExp(escapeLiteral(text), "isu"); }

/** Splits the public whitespace or pipe-delimited semantic keyword contract. */
function keywords(query: string): string[] {
  return query.split(query.indexOf("|") === -1 ? /\s+/u : /\|/u).map(
    /** Normalizes authored term boundaries without rewriting their contents. */
    term => term.trim(),
  ).filter(
    /** Excludes empty separators from scoring and provider calls. */
    term => term !== "",
  );
}

/** Produces bounded Unicode lexical expressions without substring-guessing loops. */
function lexicalPatterns(query: string): RegExp[] {
  const terms = new Set<string>();
  for (const match of query.toLowerCase().matchAll(/[\p{L}\p{N}_*]+/gu)) {
    const term = match[0];
    if (term !== "*") terms.add(term);
    if (/^\p{Script=Han}+$/u.test(term)) {
      const characters = [...term];
      for (let index = 0; index < characters.length; index += 1) {
        terms.add(characters[index]);
        if (index + 1 < characters.length) terms.add(characters[index] + characters[index + 1]);
      }
    }
  }
  return [...terms].sort(
    /** Gives longer lexical expressions the same bounded query budget as Kotlin. */
    (left, right) => right.length - left.length,
  ).slice(0, 32).map(pattern);
}

/** Resolves the Kotlin score-mode multipliers for both nodes and document chunks. */
function weights(config: MemorySearchConfig): MemorySearchConfig {
  let keyword: number, vector: number, edge: number;
  switch (config.scoreMode) {
    case "BALANCED": keyword = 1; vector = 1; edge = 1; break;
    case "KEYWORD_FIRST": keyword = 1.3; vector = 0.8; edge = 0.9; break;
    case "SEMANTIC_FIRST": keyword = 0.8; vector = 1.3; edge = 1.1; break;
    default: throw new Error("Invalid memory scoring mode");
  }
  return { scoreMode: config.scoreMode, keywordWeight: config.keywordWeight * keyword, tagWeight: config.tagWeight * keyword, vectorWeight: config.vectorWeight * vector, edgeWeight: config.edgeWeight * edge };
}

/** Ranks lexical coverage using RRF, stored importance, and stable record order. */
function lexicalScores(documents: SearchDocument[], patterns: RegExp[], weight: number): RankedScore[] {
  if (weight === 0 || patterns.length === 0) return [];
  return documents.map(
    /** Counts each query expression once regardless of repeated occurrences. */
    document => ({ document, hits: patterns.filter(
      /** Matches an explicit expression against the selected title, tags, or chunk. */
      expression => document.texts.some(text => expression.test(text)),
    ).length }),
  ).filter(
    /** Requires actual lexical evidence before contributing a rank score. */
    entry => entry.hits > 0,
  ).sort(
    /** Matches Kotlin's coverage, importance, and update-time tie ordering. */
    (left, right) => right.hits - left.hits || right.document.importance - left.document.importance || right.document.updatedAt - left.document.updatedAt,
  ).map(
    /** Uses the original reciprocal-rank constant and coverage bonus. */
    (entry, rank) => ({ id: entry.document.id, score: weight * entry.document.importance * (1 + 0.6 * entry.hits / patterns.length) / (61 + rank) }),
  );
}

/** Scores real provider vectors with the Kotlin semantic rank and keyword normalization. */
async function semanticScores(space: MemorySpace, documents: SearchDocument[], terms: string[], weight: number): Promise<RankedScore[]> {
  const scores: RankedScore[] = [];
  if (weight === 0 || documents.length === 0) return scores;
  const vectors: number[][][] = [];
  for (const document of documents) {
    const inputs: number[][] = [];
    for (const text of document.texts) inputs.push(await embeddingFor(space, text));
    vectors.push(inputs);
  }
  for (const term of terms) {
    const query = await embeddingFor(space, term);
    const ranked = documents.map(
      /** Keeps each similarity attached to its exact stored record. */
      (document, index) => {
        const similarities = vectors[index];
        if (similarities.length === 0) throw new Error("Memory search record has no embedding inputs");
        let similarity = -Infinity;
        for (const vector of similarities) similarity = Math.max(similarity, cosineSimilarity(query, vector));
        return { document, similarity };
      },
    ).sort(
      /** Preserves source order when actual provider similarities tie. */
      (left, right) => right.similarity - left.similarity,
    );
    for (let rank = 0; rank < ranked.length; rank += 1) {
      const entry = ranked[rank];
      scores.push({ id: entry.document.id, score: (Math.sqrt(entry.document.importance) / (61 + rank) + entry.similarity * weight) / Math.sqrt(terms.length) });
    }
  }
  return scores;
}

/** Applies folder descendants and inclusive creation-time boundaries before every scoring channel. */
function matchesFilters(memory: Memory, options: MemorySearchOptions): boolean {
  if (options.folderPath !== null && memory.folderPath !== options.folderPath && !(memory.folderPath !== null && memory.folderPath.startsWith(options.folderPath + "/"))) return false;
  if (options.createdAtStartMs !== null && memory.createdAt < options.createdAtStartMs) return false;
  if (options.createdAtEndMs !== null && memory.createdAt > options.createdAtEndMs) return false;
  return memory.title !== ".folder_placeholder";
}

/** Returns full records using the same ranking and threshold decisions as the search debugger. */
export async function searchMemorySpace(space: MemorySpace, options: MemorySearchOptions): Promise<Memory[]> {
  if (options.query.trim() === "*") return space.memories.filter(
    /** Lists only the explicitly selected scope without calling a provider. */
    memory => matchesFilters(memory, options),
  );
  return (await searchMemorySpaceDebug(space, options, space.searchConfig)).candidates.filter(
    /** Selects exactly the debugger's accepted full records. */
    hit => hit.passedThreshold,
  ).map(
    /** Retains every stored memory field in the public search result. */
    hit => hit.memory,
  );
}

/** Computes node RRF channels and one-hop graph expansion without corpus-dependent normalization. */
export async function searchMemorySpaceDebug(space: MemorySpace, options: MemorySearchOptions, config: MemorySearchConfig): Promise<MemorySearchDebug> {
  if (options.query.trim() === "*") throw new Error("检索调试请输入具体查询文本");
  const expressions = lexicalPatterns(options.query), terms = keywords(options.query), resolved = weights(config);
  // An unconfigured embedding provider cannot serve the semantic channel; skipping it keeps keyword,
  // tag and edge retrieval working instead of failing every search for a weight the owner cannot use.
  const vectorWeight = embeddingConfigured(space.settings) ? resolved.vectorWeight : 0;
  if (expressions.length === 0) throw new Error("Memory search query has no searchable tokens");
  if (resolved.keywordWeight + resolved.tagWeight + vectorWeight + resolved.edgeWeight === 0) throw new Error("Memory search has no enabled scoring weights");
  const candidates: MemorySearchHit[] = space.memories.filter(
    /** Restricts all channels and graph expansion to the requested scope. */
    memory => matchesFilters(memory, options),
  ).map(
    /** Initializes explicit channel accumulators for each stored record. */
    memory => ({ memory, score: 0, keywordScore: 0, tagScore: 0, vectorScore: 0, edgeScore: 0, passedThreshold: false }),
  );
  const byId = new Map(candidates.map(
    /** Indexes scoped records for constant-time channel accumulation. */
    hit => [hit.memory.id, hit],
  ));
  /** Adds one channel contribution to the scoped record that produced it. */
  function addChannel(id: string, channel: "keywordScore" | "tagScore" | "vectorScore", score: number): void {
    const hit = byId.get(id);
    if (hit === undefined) throw new Error("Memory score refers to an out-of-scope record: " + id);
    hit[channel] += score;
  }
  const chunkTexts = new Map<string, string[]>();
  for (const hit of candidates) if (hit.memory.isDocumentNode) chunkTexts.set(hit.memory.uuid, []);
  for (const chunk of space.chunks) {
    const texts = chunkTexts.get(chunk.memoryUuid);
    if (texts !== undefined) texts.push(chunk.content);
  }
  const documents = candidates.map(
    /** Searches node titles and retains full-text recall for explicitly imported documents. */
    hit => {
      const texts = [hit.memory.title], chunks = chunkTexts.get(hit.memory.uuid);
      if (chunks !== undefined) texts.push(...chunks);
      return { id: hit.memory.id, texts, importance: hit.memory.importance, updatedAt: hit.memory.updatedAt };
    },
  );
  for (const contribution of lexicalScores(documents, expressions, resolved.keywordWeight)) addChannel(contribution.id, "keywordScore", contribution.score);
  const tagDocuments = documents.map(
    /** Uses the same lexical ranking contract for exact stored tag names. */
    (document, index) => ({ ...document, texts: candidates[index].memory.tags.map(tag => tag.name) }),
  );
  for (const contribution of lexicalScores(tagDocuments, expressions, resolved.tagWeight)) addChannel(contribution.id, "tagScore", contribution.score);
  let reverseRank = 0;
  for (const hit of candidates) if (literalPattern(hit.memory.title).test(options.query)) {
    hit.keywordScore += resolved.keywordWeight * hit.memory.importance / (61 + reverseRank);
    reverseRank += 1;
  }
  const semanticDocuments = documents.map(
    /** Uses document titles and complete non-document content as provider inputs. */
    (document, index) => ({ ...document, texts: candidates[index].memory.isDocumentNode ? document.texts : [candidates[index].memory.content] }),
  );
  for (const contribution of await semanticScores(space, semanticDocuments, terms, vectorWeight)) addChannel(contribution.id, "vectorScore", contribution.score);
  for (const hit of candidates) hit.score = hit.keywordScore + hit.tagScore + hit.vectorScore;
  const seeds = candidates.filter(
    /** Expands only real matched seeds, never unrelated zero-score nodes. */
    hit => hit.score > 0,
  ).sort(
    /** Selects the Kotlin ten-seed graph expansion budget. */
    (left, right) => right.score - left.score,
  ).slice(0, 10);
  if (resolved.edgeWeight > 0) for (const seed of seeds) for (const link of space.links) {
    const other = link.sourceMemoryId === seed.memory.id ? link.targetMemoryId : link.targetMemoryId === seed.memory.id ? link.sourceMemoryId : null;
    if (other === null) continue;
    const target = byId.get(other);
    if (target !== undefined) target.edgeScore += (seed.score * link.weight + 0.03) * resolved.edgeWeight;
  }
  for (const hit of candidates) {
    hit.score += hit.edgeScore;
    hit.passedThreshold = hit.score > 0 && hit.score >= options.relevanceThreshold;
  }
  candidates.sort(
    /** Keeps stable persisted identity ties after all four channels are accumulated. */
    (left, right) => right.score - left.score || left.memory.id.localeCompare(right.memory.id),
  );
  return { query: options.query, config: { ...config }, relevanceThreshold: options.relevanceThreshold, memoriesInScopeCount: candidates.length, candidates };
}

/** Searches a document with the same lexical and semantic ranking used by memory nodes. */
export async function searchMemoryChunks(space: MemorySpace, memoryId: string, query: string, count: number): Promise<{ total: number; chunks: DocumentChunk[] }> {
  const memory = space.memories.find(
    /** Resolves the exact stored document rather than a display title. */
    item => item.id === memoryId,
  );
  if (memory === undefined || !memory.isDocumentNode) throw new Error("Document search requires a stored document memory");
  const chunks = space.chunks.filter(
    /** Selects this document's original persisted chunks. */
    chunk => chunk.memoryUuid === memory.uuid,
  ).sort(
    /** Preserves one-based selector presentation over stored zero-based indices. */
    (left, right) => left.chunkIndex - right.chunkIndex,
  );
  if (query.trim() === "*") return { total: chunks.length, chunks: chunks.slice(0, count) };
  if (query.trim() === "") throw new Error("Document search query must not be blank");
  const documents = chunks.map(
    /** Gives chunks unit importance and stable document-position ties. */
    chunk => ({ id: chunk.id, texts: [chunk.content], importance: 1, updatedAt: 0 }),
  ), resolved = weights(space.searchConfig), scores = new Map<string, number>();
  // Same unconfigured-provider degradation as node search: skip the semantic channel entirely.
  const contributions = [...lexicalScores(documents, lexicalPatterns(query), resolved.keywordWeight), ...await semanticScores(space, documents, keywords(query), embeddingConfigured(space.settings) ? resolved.vectorWeight : 0)];
  for (const contribution of contributions) {
    const previous = scores.get(contribution.id);
    scores.set(contribution.id, previous === undefined ? contribution.score : previous + contribution.score);
  }
  const selected = chunks.filter(
    /** Retains only chunks that contributed to at least one enabled channel. */
    chunk => scores.has(chunk.id),
  ).sort(
    /** Resolves score ties by actual document position. */
    (left, right) => {
      const leftScore = scores.get(left.id), rightScore = scores.get(right.id);
      if (leftScore === undefined || rightScore === undefined) throw new Error("Ranked chunk score is missing");
      return rightScore - leftScore || left.chunkIndex - right.chunkIndex;
    },
  ).slice(0, count);
  return { total: chunks.length, chunks: selected };
}
