import type { Memory, MemorySearchConfig, MemorySearchOptions, MemorySpace } from "./model";
import { cosineSimilarity, embeddingFor } from "./memory-jobs/embeddings";

/** Tokenizes Unicode words and CJK unigrams/bigrams without substring guessing. */
function tokens(text: string): string[] {
  const words = text.toLocaleLowerCase().match(/[\p{L}\p{N}_]+/gu);
  const result: string[] = [];
  if (words === null) return result;
  for (const word of words) {
    if (/^[\p{Script=Han}]+$/u.test(word)) {
      const characters = [...word];
      for (let index = 0; index < characters.length; index += 1) {
        result.push(characters[index]);
        if (index + 1 < characters.length) result.push(characters[index] + characters[index + 1]);
      }
    } else result.push(word);
  }
  return result;
}
/** Counts exact token frequencies for the plugin's BM25 index. */
function counts(values: string[]): Map<string, number> {
  const result = new Map<string, number>();
  for (const value of values) {
    const previous = result.get(value);
    result.set(value, previous === undefined ? 1 : previous + 1);
  }
  return result;
}
/** Applies the complete explicit folder and timestamp filters to full memory records. */
function matchesFilters(memory: Memory, options: MemorySearchOptions): boolean {
  if (options.folderPath !== null && memory.folderPath !== options.folderPath && !(memory.folderPath !== null && memory.folderPath.startsWith(options.folderPath + "/"))) return false;
  if (options.createdAtStartMs !== null && memory.createdAt < options.createdAtStartMs) return false;
  if (options.createdAtEndMs !== null && memory.createdAt > options.createdAtEndMs) return false;
  return true;
}
/** Computes lexical, real provider-vector, tag and graph scoring over complete plugin records. */
export async function searchMemorySpace(space: MemorySpace, options: MemorySearchOptions): Promise<Memory[]> {
  const config: MemorySearchConfig = space.searchConfig;
  const memories = space.memories.filter(
    /** Keeps complete records under the caller's explicit filters. */
    memory => matchesFilters(memory, options),
  );
  if (options.query.trim() === "*") return memories;
  const query = [...new Set(tokens(options.query))];
  if (query.length === 0) throw new Error("Memory search query has no searchable tokens");
  const index = memories.map(
    /** Builds frequencies from the complete title, content and document chunks. */
    memory => {
      const chunks = space.chunks.filter(
        /** Selects chunks by the actual persisted UUID, never graph labels. */
        chunk => chunk.memoryUuid === memory.uuid,
      );
      const text = memory.title + "\n" + memory.content + "\n" + chunks.map(
        /** Retains every document chunk's searchable content. */
        chunk => chunk.content,
      ).join("\n");
      const values = tokens(text);
      return { memory, length: values.length, frequencies: counts(values), tags: new Set(memory.tags.flatMap(
        /** Indexes real memory tag names independently from content. */
        tag => tokens(tag.name),
      )) };
    },
  );
  if (index.length === 0) return [];
  const average = index.reduce(
    /** Accumulates actual token lengths for length normalization. */
    (total, document) => total + document.length, 0,
  ) / index.length;
  const lexical = new Map<string, number>(), tagScores = new Map<string, number>();
  let maximum = 0;
  for (const document of index) {
    let bm25 = 0, hits = 0;
    for (const token of query) {
      const df = index.filter(
        /** Counts documents with this exact token. */
        candidate => candidate.frequencies.has(token),
      ).length;
      const frequency = document.frequencies.get(token);
      if (frequency !== undefined) {
        const idf = Math.log(1 + (index.length - df + 0.5) / (df + 0.5));
        const lengthFactor = average === 0 ? 1 : document.length / average;
        bm25 += idf * frequency * 2.2 / (frequency + 1.2 * (0.25 + 0.75 * lengthFactor));
      }
      if (document.tags.has(token)) hits += 1;
    }
    lexical.set(document.memory.id, bm25); tagScores.set(document.memory.id, hits / query.length); maximum = Math.max(maximum, bm25);
  }
  const semantic = new Map<string, number>();
  if (config.vectorWeight > 0) {
    const queryVector = await embeddingFor(space, options.query);
    for (const document of index) {
      const memory = document.memory;
      const vector = await embeddingFor(space, memory.isDocumentNode ? memory.title : memory.content);
      let similarity = Math.max(0, cosineSimilarity(queryVector, vector));
      if (memory.isDocumentNode) for (const chunk of space.chunks) if (chunk.memoryUuid === memory.uuid) similarity = Math.max(similarity, cosineSimilarity(queryVector, await embeddingFor(space, chunk.content)));
      semantic.set(memory.id, similarity);
    }
  }
  let keywordMultiplier: number, semanticMultiplier: number, edgeMultiplier: number;
  switch (config.scoreMode) {
    case "BALANCED": keywordMultiplier = 1; semanticMultiplier = 1; edgeMultiplier = 1; break;
    case "KEYWORD_FIRST": keywordMultiplier = 1.3; semanticMultiplier = 0.8; edgeMultiplier = 0.9; break;
    case "SEMANTIC_FIRST": keywordMultiplier = 0.8; semanticMultiplier = 1.3; edgeMultiplier = 1.1; break;
    default: throw new Error("Invalid memory scoring mode");
  }
  const keywordWeight = config.keywordWeight * keywordMultiplier, tagWeight = config.tagWeight * keywordMultiplier,
    vectorWeight = config.vectorWeight * semanticMultiplier, edgeWeight = config.edgeWeight * edgeMultiplier;
  const scores: { memory: Memory; score: number }[] = [];
  for (const document of index) {
    const lexicalValue = lexical.get(document.memory.id), tagValue = tagScores.get(document.memory.id);
    if (lexicalValue === undefined || tagValue === undefined) throw new Error("Search index is inconsistent");
    const keyword = maximum === 0 ? 0 : lexicalValue / maximum;
    let edge = 0;
    for (const link of space.links) {
      let other: string;
      if (link.sourceMemoryId === document.memory.id) other = link.targetMemoryId;
      else if (link.targetMemoryId === document.memory.id) other = link.sourceMemoryId;
      else continue;
      const otherLexical = lexical.get(other);
      if (otherLexical !== undefined && maximum !== 0) edge = Math.max(edge, link.weight * otherLexical / maximum);
    }
    const denominator = keywordWeight + tagWeight + vectorWeight + edgeWeight;
    if (denominator === 0) throw new Error("Memory search has no enabled scoring weights");
    let vector = 0;
    if (vectorWeight > 0) { const value = semantic.get(document.memory.id); if (value === undefined) throw new Error("Semantic search index is inconsistent"); vector = value; }
    const score = (keyword * keywordWeight + tagValue * tagWeight + vector * vectorWeight + edge * edgeWeight) / denominator;
    if (score > 0 && score >= options.relevanceThreshold) scores.push({ memory: document.memory, score });
  }
  scores.sort(
    /** Orders actual ranked hits with a stable lossless identity tie breaker. */
    (left, right) => right.score - left.score || (BigInt(left.memory.id) < BigInt(right.memory.id) ? -1 : BigInt(left.memory.id) > BigInt(right.memory.id) ? 1 : 0),
  );
  return scores.map(
    /** Returns full records rather than a lossy search projection. */
    entry => entry.memory,
  );
}
