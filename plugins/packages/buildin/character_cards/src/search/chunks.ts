import type { CharacterMemoryDatabase } from "../storage/memory";
import type { Memory, DocumentChunk, MemorySearchConfig } from "../model";
import type { SourceEmbedding } from "../memory-jobs/embeddings";
import { lexicalTerms } from "../memory-search";
import { withMemoryIndex } from "./state";
/** Uses the same source vectors and tokenization as parent-memory retrieval. */
export function searchDocumentChunks(database: CharacterMemoryDatabase, ownerKey: string, memory: Memory, query: string, count: number, config: MemorySearchConfig, embedding: SourceEmbedding | null): Promise<{ total: number; chunks: DocumentChunk[] }> {
  return withMemoryIndex(database, ownerKey, async index => {
    const ids = index.chunkIds.get(memory.uuid) ?? new Set<string>(), scores = new Map<string, number>();
    const wildcard = query.trim() === "*";
    if (wildcard) for (const id of ids) scores.set(id, 1);
    else {
      if (config.keywordWeight > 0) for (const term of lexicalTerms(query)) for (const identity of index.text.search(term)) {
        if (identity.startsWith("chunks/") && ids.has(identity.slice(7))) scores.set(identity.slice(7), config.keywordWeight);
      }
      if (embedding !== null) {
        const scope = { ids: new Set([memory.id]), restricted: true };
        index.validate(embedding, scope, memory.uuid);
        const semantic = index.semantic(embedding, scope, count, new Set(), memory.uuid);
        index.scoreCandidates(embedding, [...scores.keys()].slice(0, Math.max(256, count * 4)), semantic, true);
        for (const [id, similarity] of semantic) scores.set(id, (scores.get(id) ?? 0) + config.vectorWeight * similarity);
      }
    }
    const selected = [...scores.keys()].filter(id => scores.get(id)! > 0).sort((a, b) => scores.get(b)! - scores.get(a)! || index.chunks.get(a)!.chunkIndex - index.chunks.get(b)!.chunkIndex).slice(0, count);
    const entries = selected.length ? await database.getMany<{ record: DocumentChunk }>("chunks", selected.map(id => ownerKey + "/" + id)) : [];
    if (entries.some((entry, offset) => entry === null || entry.version !== index.sources.get("chunks/" + selected[offset])?.version)) throw new Error("Document changed during retrieval; retry the query");
    return { total: ids.size, chunks: entries.map(entry => entry!.value.record) };
  });
}
