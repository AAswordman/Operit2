import type { CharacterRepository } from "../canonical";
import type { MemoryEmbedding, MemorySettings, MemorySpace } from "../model";
import { assertInteger, assertNumber, assertObject, assertString, requireName } from "../validation";

/** Validates one persisted real embedding rather than accepting a placeholder vector. */
export function assertEmbedding(value: unknown): asserts value is MemoryEmbedding {
  assertObject(value, "embedding"); requireName(value.endpoint, "embedding endpoint"); requireName(value.model, "embedding model"); assertString(value.text, "embedding text");
  assertInteger(value.updatedAt, "embedding timestamp", 0);
  if (!Array.isArray(value.vector) || value.vector.length === 0) throw new Error("Embedding must contain a nonempty vector");
  let norm = 0;
  for (const component of value.vector) { assertNumber(component, "embedding component", -Number.MAX_VALUE, Number.MAX_VALUE); norm += component * component; }
  if (!Number.isFinite(norm) || norm === 0) throw new Error("Embedding vector has an invalid norm");
}

/** Requires explicitly enabled configured cloud embedding over the existing generic HTTP capability. */
function embeddingSource(settings: MemorySettings): { endpoint: string; model: string } {
  if (!settings.cloudEmbeddingEnabled) throw new Error("Cloud embedding is disabled");
  const endpoint = requireName(settings.cloudEmbeddingEndpoint, "embedding endpoint").replace(/\/+$/, ""), model = requireName(settings.cloudEmbeddingModel, "embedding model");
  if (!/^https?:\/\//u.test(endpoint)) throw new Error("Embedding endpoint must be an absolute HTTP URL");
  return { endpoint, model };
}

/** Requests and validates an actual provider vector without retries or zero-vector substitution. */
async function requestEmbedding(text: string, settings: MemorySettings): Promise<MemoryEmbedding> {
  const source = embeddingSource(settings), headers: Record<string, string> = { "Content-Type": "application/json" };
  if (settings.cloudEmbeddingApiKey !== "") headers.Authorization = "Bearer " + settings.cloudEmbeddingApiKey;
  const response = await Tools.Net.http({
    url: source.endpoint.endsWith("/embeddings") ? source.endpoint : source.endpoint + "/embeddings",
    method: "POST", headers, body: JSON.stringify({ input: text, model: source.model, encoding_format: "float" }),
    connect_timeout: 30, read_timeout: 60, follow_redirects: true, ignore_ssl: false, responseType: "text", validateStatus: false,
  });
  if (response.statusCode < 200 || response.statusCode >= 300) throw new Error("Embedding request failed: HTTP " + response.statusCode + " " + response.content);
  assertString(response.content, "embedding response");
  const payload: unknown = JSON.parse(response.content); assertObject(payload, "embedding response");
  if (!Array.isArray(payload.data) || payload.data.length !== 1) throw new Error("Embedding response must contain exactly one input vector");
  const record = payload.data[0]; assertObject(record, "embedding response item");
  const embedding = { ...source, text, vector: record.embedding, updatedAt: Date.now() }; assertEmbedding(embedding); return embedding;
}

/** Reads an exact cache entry or performs its first real computation, never replacing a failed provider result. */
export async function embeddingFor(space: MemorySpace, text: string): Promise<number[]> {
  assertString(text, "embedding input"); const source = embeddingSource(space.settings);
  const matches = space.embeddings.filter(
    /** Matches the complete provider source and input without hashing away record provenance. */
    item => item.endpoint === source.endpoint && item.model === source.model && item.text === text,
  );
  if (matches.length > 1) throw new Error("Duplicate stored embedding input");
  if (matches.length === 1) { assertEmbedding(matches[0]); return matches[0].vector; }
  const computed = await requestEmbedding(text, space.settings); space.embeddings.push(computed); return computed.vector;
}

/** Computes a dimension-checked cosine similarity from two actual provider vectors. */
export function cosineSimilarity(left: number[], right: number[]): number {
  if (left.length === 0 || left.length !== right.length) throw new Error("Embedding dimensions do not match");
  let dot = 0, leftNorm = 0, rightNorm = 0;
  for (let index = 0; index < left.length; index += 1) { dot += left[index] * right[index]; leftNorm += left[index] ** 2; rightNorm += right[index] ** 2; }
  if (leftNorm === 0 || rightNorm === 0) throw new Error("Embedding norm must be positive");
  return Math.max(-1, Math.min(1, dot / Math.sqrt(leftNorm * rightNorm)));
}

/** Recomputes every unique full-record and document-chunk input, publishing only after all requests succeed. */
export async function rebuildMemoryEmbeddings(ownerKey: string, repository: CharacterRepository): Promise<number> {
  const space = await repository.readMemorySpace(ownerKey); embeddingSource(space.settings);
  const texts = new Set<string>();
  for (const memory of space.memories) texts.add(memory.isDocumentNode ? memory.title : memory.content);
  for (const chunk of space.chunks) texts.add(chunk.content);
  const computed: MemoryEmbedding[] = [];
  for (const text of texts) computed.push(await requestEmbedding(text, space.settings));
  space.embeddings = computed; await repository.writeMemorySpace(ownerKey, space); return space.memories.length;
}
