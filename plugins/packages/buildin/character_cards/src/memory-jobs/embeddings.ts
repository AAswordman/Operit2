import type { CharacterRepository } from "../canonical";
import type { MemoryEmbedding, MemorySettings } from "../model";
import { assertInteger, assertNumber, assertObject, assertString, requireId, requireName } from "../validation";

/** Stores one normalized vector with its exact source input and provider space. */
export interface SourceEmbedding { space: string; text: string; vector: number[] }

/** Validates one persisted real embedding rather than accepting a placeholder vector. */
export function assertEmbedding(value: unknown): asserts value is MemoryEmbedding {
  assertObject(value, "embedding"); requireId(value.id, "embedding id"); requireName(value.endpoint, "embedding endpoint"); requireName(value.model, "embedding model"); assertString(value.text, "embedding text");
  assertInteger(value.updatedAt, "embedding timestamp", 0);
  if (!Array.isArray(value.vector) || value.vector.length === 0) throw new Error("Embedding must contain a nonempty vector");
  let norm = 0;
  for (const component of value.vector) { assertNumber(component, "embedding component", -Number.MAX_VALUE, Number.MAX_VALUE); norm += component * component; }
  if (!Number.isFinite(norm) || norm === 0) throw new Error("Embedding vector has an invalid norm");
}


/** Identifies a provider and normalization contract independently of API credentials. */
export function embeddingSpace(settings: MemorySettings): string {
  if (!settings.cloudEmbeddingEnabled) throw new Error("Cloud embedding is disabled");
  const endpoint = requireName(settings.cloudEmbeddingEndpoint, "embedding endpoint").replace(/\/+$/, "");
  if (!/^https?:\/\//u.test(endpoint)) throw new Error("Embedding endpoint must be an absolute HTTP URL");
  return JSON.stringify([endpoint, requireName(settings.cloudEmbeddingModel, "embedding model"), "unit-v1"]);
}

/** Validates finite provider components and normalizes once at the write boundary. */
function normalize(value: unknown): number[] {
  if (!Array.isArray(value) || value.length === 0) throw new Error("Embedding must contain a nonempty vector");
  let norm = 0;
  for (const component of value) { assertNumber(component, "embedding component", -Number.MAX_VALUE, Number.MAX_VALUE); norm += component * component; }
  if (!Number.isFinite(norm) || norm === 0) throw new Error("Embedding vector has an invalid norm");
  const length = Math.sqrt(norm);
  return value.map(/** Normalizes provider components for database dot-product search. */ component => component / length);
}

/** Requests indexed batches through the shared HTTP host with four bounded concurrent workers. */
export async function embedTexts(texts: readonly string[], settings: MemorySettings): Promise<SourceEmbedding[]> {
  const space = embeddingSpace(settings), [endpoint, model] = JSON.parse(space) as [string, string, string];
  const result: SourceEmbedding[] = new Array(texts.length);
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (settings.cloudEmbeddingApiKey !== "") headers.Authorization = "Bearer " + settings.cloudEmbeddingApiKey;
  let next = 0;
  /** Computes disjoint batches without holding any repository or SQLite transaction. */
  async function worker(): Promise<void> {
    while (next < texts.length) {
      const start = next; next += 64;
      const batch = texts.slice(start, start + 64);
      const response = await Tools.Net.http({
        url: endpoint.endsWith("/embeddings") ? endpoint : endpoint + "/embeddings",
        method: "POST", headers, body: JSON.stringify({ input: batch, model, encoding_format: "float" }),
        connect_timeout: 30, read_timeout: 60, follow_redirects: true, ignore_ssl: false, responseType: "text", validateStatus: false,
      });
      if (response.statusCode < 200 || response.statusCode >= 300) throw new Error("Embedding request failed: HTTP " + response.statusCode + " " + response.content);
      assertString(response.content, "embedding response");
      const payload: unknown = JSON.parse(response.content); assertObject(payload, "embedding response");
      if (!Array.isArray(payload.data) || payload.data.length !== batch.length) throw new Error("Embedding response count does not match the request");
      const seen = new Set<number>();
      for (const item of payload.data) {
        assertObject(item, "embedding response item"); assertInteger(item.index, "embedding response index", 0);
        if (item.index >= batch.length || seen.has(item.index)) throw new Error("Embedding response has duplicate or invalid input indices");
        seen.add(item.index);
        result[start + item.index] = { space, text: batch[item.index], vector: normalize(item.embedding) };
      }
    }
  }
  const workers = await Promise.allSettled(Array.from({ length: Math.min(4, Math.ceil(texts.length / 64)) }, worker));
  for (const worker of workers) if (worker.status === "rejected") throw worker.reason;
  const dimensions = result[0]?.vector.length;
  for (const embedding of result) if (embedding.vector.length !== dimensions) throw new Error("Embedding batch dimensions do not match");
  return result;
}

/** Rebuilds persisted source vectors through version-checked database pages. */
export function rebuildMemoryEmbeddings(ownerKey: string, repository: CharacterRepository): Promise<number> {
  return repository.rebuildMemoryEmbeddings(ownerKey);
}
