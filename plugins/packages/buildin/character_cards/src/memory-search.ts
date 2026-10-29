import type { MemorySearchConfig, MemorySearchOptions, MemorySearchDebug, MemorySearchHit, MemorySettings } from "./model";
import type { CharacterMemoryDatabase } from "./storage/memory";
import type { SourceEmbedding } from "./memory-jobs/embeddings";
import { embedTexts } from "./memory-jobs/embeddings";
import { withMemoryIndex } from "./search/state";
import type { MemoryIndex, Scope } from "./search/state";
export const MEMORY_SEARCH_THRESHOLD = 0.025;

/** Expands Unicode query words, Han bigrams and unigrams under a fixed lexical budget. */
export function lexicalTerms(query: string): string[] {
  const terms = new Set<string>();
  for (const match of query.toLowerCase().matchAll(/[\p{L}\p{N}_*]+/gu)) {
    const word = match[0]; terms.add(word);
    for (const han of word.matchAll(/\p{Script=Han}+/gu)) {
      const chars = [...han[0]];
      for (let index = 0; index < chars.length - 1; index += 1) terms.add(chars[index] + chars[index + 1]);
      for (const char of chars) terms.add(char);
    }
  }
  return [...terms].slice(0, 32);
}

/** The query vector is ephemeral and is shared by memory and document retrieval. */
export async function prepareMemoryQuery(settings: MemorySettings, query: string, config: MemorySearchConfig): Promise<SourceEmbedding | null> {
  return query.trim() !== "*" && settings.cloudEmbeddingEnabled && config.vectorWeight > 0 ? (await embedTexts([query], settings))[0] : null;
}
const compareId = (a: string, b: string) => a < b ? -1 : a > b ? 1 : 0;
function rank(index: MemoryIndex, counts: Map<string, number>, weight: number, termCount: number): Map<string, number> {
  if (weight === 0) return new Map();
  const ids = [...counts.keys()].sort((a, b) => {
    const first = index.memories.get(a)!, second = index.memories.get(b)!;
    return counts.get(b)! - counts.get(a)! || second.importance - first.importance || second.updatedAt - first.updatedAt || compareId(a, b);
  });
  return new Map(ids.map((id, offset) => [id, weight * index.memories.get(id)!.importance * (1 + 0.6 * counts.get(id)! / Math.max(1, termCount)) / (61 + offset)]));
}
function ranked(index: MemoryIndex, scope: Scope, options: MemorySearchOptions, config: MemorySearchConfig, query: SourceEmbedding | null, debug: boolean): { hits: MemorySearchHit[]; excluded: number } {
  const excluded = new Set(options.excludeIds ?? []), limit = options.limit ?? 100;
  if (options.query.trim() === "*") return { excluded: [...excluded].filter(id => scope.ids.has(id)).length,
    hits: [...scope.ids].filter(id => !excluded.has(id)).sort(compareId).slice(0, limit).map(id => ({ memory: index.memories.get(id)!, score: 1, keywordScore: 0, tagScore: 0, vectorScore: 0, edgeScore: 0, passedThreshold: true })) };
  const terms = lexicalTerms(options.query), matched = index.matching(terms, scope, config.keywordWeight > 0, config.tagWeight > 0);
  const lexicalBoost = config.scoreMode === "KEYWORD_FIRST" ? 1.3 : config.scoreMode === "SEMANTIC_FIRST" ? 0.8 : 1;
  const keywordWeight = config.keywordWeight * lexicalBoost, tagWeight = config.tagWeight * lexicalBoost;
  const vectorWeight = config.vectorWeight * (config.scoreMode === "SEMANTIC_FIRST" ? 1.3 : config.scoreMode === "KEYWORD_FIRST" ? 0.8 : 1);
  const edgeWeight = config.edgeWeight * (config.scoreMode === "KEYWORD_FIRST" ? 0.9 : config.scoreMode === "SEMANTIC_FIRST" ? 1.1 : 1);
  const keywords = rank(index, matched.keywords, keywordWeight, terms.length), tags = rank(index, matched.tags, tagWeight, terms.length);
  if (keywordWeight > 0) for (const id of index.titles.search(options.query)) if (scope.ids.has(id)) keywords.set(id, (keywords.get(id) ?? 0) + keywordWeight * index.memories.get(id)!.importance / 61);
  const lexical = new Set([...keywords.keys(), ...tags.keys()]);
  let semantic = new Map<string, number>();
  if (query !== null) {
    index.validate(query, scope);
    semantic = index.semantic(query, scope, Math.max(10, limit), excluded);
    // Exact reranking is bounded to the strongest lexical candidates and excluded identities.
    const candidates = [...lexical].sort((a, b) => (keywords.get(b) ?? 0) + (tags.get(b) ?? 0) - (keywords.get(a) ?? 0) - (tags.get(a) ?? 0));
    index.scoreCandidates(query, [...new Set([...candidates.slice(0, Math.max(256, limit * 4)), ...excluded].filter(id => scope.ids.has(id)))], semantic);
  }
  const ids = debug ? new Set(scope.ids) : new Set([...lexical, ...semantic.keys()]);
  const hits = new Map<string, MemorySearchHit>();
  const hit = (id: string) => {
    let item = hits.get(id);
    if (!item) {
      const keywordScore = keywords.get(id) ?? 0, tagScore = tags.get(id) ?? 0, vectorScore = (semantic.get(id) ?? 0) * vectorWeight;
      item = { memory: index.memories.get(id)!, keywordScore, tagScore, vectorScore, edgeScore: 0, score: keywordScore + tagScore + vectorScore, passedThreshold: false }; hits.set(id, item);
    }
    return item;
  };
  for (const id of ids) hit(id);
  const seeds = [...hits.values()].filter(item => item.score > 0).sort((a, b) => b.score - a.score || compareId(a.memory.id, b.memory.id)).slice(0, 10);
  if (edgeWeight > 0) {
    const targets = new Set<string>();
    for (const seed of seeds) for (const linkId of index.adjacency.get(seed.memory.id) ?? []) {
      const link = index.links.get(linkId)!, id = link.sourceMemoryId === seed.memory.id ? link.targetMemoryId : link.sourceMemoryId;
      if (scope.ids.has(id)) targets.add(id);
    }
    if (query !== null) index.scoreCandidates(query, [...targets], semantic);
    for (const id of targets) { const item = hit(id); item.vectorScore = (semantic.get(id) ?? 0) * vectorWeight; item.score = item.keywordScore + item.tagScore + item.vectorScore; }
    // Seed scores are captured before edge propagation; links never recursively amplify a query.
    const seedScores = seeds.map(seed => ({ id: seed.memory.id, score: seed.keywordScore + seed.tagScore + seed.vectorScore }));
    for (const seed of seedScores) for (const linkId of index.adjacency.get(seed.id) ?? []) {
      const link = index.links.get(linkId)!, id = link.sourceMemoryId === seed.id ? link.targetMemoryId : link.sourceMemoryId;
      if (!scope.ids.has(id)) continue;
      hit(id).edgeScore += (seed.score * link.weight + 0.03) * edgeWeight;
    }
  }
  let excludedCount = 0;
  for (const item of hits.values()) {
    item.score += item.edgeScore; item.passedThreshold = item.score > 0 && item.score >= options.relevanceThreshold;
    if (excluded.has(item.memory.id) && item.passedThreshold) excludedCount++;
  }
  return { excluded: excludedCount, hits: [...hits.values()].filter(item => !excluded.has(item.memory.id) && (debug || item.passedThreshold))
    .sort((a, b) => b.score - a.score || compareId(a.memory.id, b.memory.id)).slice(0, limit) };
}

/** Retrieval reads incremental changes, searches derived indexes, then hydrates only the page. */
export async function searchMemoryDatabase(database: CharacterMemoryDatabase, settings: MemorySettings, options: MemorySearchOptions, config: MemorySearchConfig, debug = false): Promise<{ result: MemorySearchDebug; queryEmbedding: SourceEmbedding | null }> {
  const limit = options.limit ?? 100, wildcard = options.query.trim() === "*";
  if (!Number.isInteger(limit) || limit < 1 || limit > 1000) throw new Error("Memory search limit must be between 1 and 1000");
  if (!wildcard) {
    if (options.query.trim() === "") throw new Error("Memory search query must not be blank");
    const vectorWeight = settings.cloudEmbeddingEnabled ? config.vectorWeight : 0;
    if (lexicalTerms(options.query).length === 0 && vectorWeight === 0) throw new Error("Memory search query has no searchable tokens");
    if (config.keywordWeight + config.tagWeight + vectorWeight + config.edgeWeight === 0) throw new Error("Memory search has no enabled scoring weights");
  }
  const queryEmbedding = await prepareMemoryQuery(settings, options.query, config);
  return withMemoryIndex(database, options.ownerKey, async index => {
    for (let attempt = 0; attempt < 3; attempt++) {
      const scope = index.scope(options), page = ranked(index, scope, options, config, queryEmbedding, debug);
      if (page.hits.length) {
        const entries = await database.getMany<{ record: MemorySearchHit["memory"] }>("memories", page.hits.map(item => options.ownerKey + "/" + item.memory.id));
        if (entries.some((entry, offset) => entry === null || entry.version !== index.sources.get("memories/" + page.hits[offset].memory.id)?.version)) {
          if (!await index.refresh()) throw new Error("Memory index changed during retrieval; retry the query");
          continue;
        }
        page.hits.forEach((item, offset) => { item.memory = entries[offset]!.value.record; });
      }
      return { queryEmbedding, result: { excludedBySnapshotCount: page.excluded, query: options.query, config, relevanceThreshold: options.relevanceThreshold, memoriesInScopeCount: scope.ids.size, candidates: page.hits } };
    }
    throw new Error("Memory records changed repeatedly during retrieval");
  });
}
