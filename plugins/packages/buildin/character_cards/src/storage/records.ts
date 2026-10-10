import type { Storage } from "../../../../../types/storage";
import type { CharacterState, MemorySpace } from "../model";

export const collections = ["cards", "groups", "tags", "stores", "conversationGroups", "owners", "memories", "links", "chunks", "candidates", "embeddings"] as const;
export const ownerCollections = ["memories", "links", "chunks", "candidates", "embeddings"] as const;
export interface StoredRecord { collection: string; key: string; value: Storage.Value; version: string | null }

/** Copies supported structured domain values without using JSON text codecs. */
export function clone<T>(value: T): T {
  if (Array.isArray(value)) return value.map(clone) as T;
  if (value !== null && typeof value === "object") return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, clone(item)])) as T;
  return value;
}

/** Compares structured records without serializing the full character and memory state. */
export function equal(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (left === null || right === null || typeof left !== "object" || typeof right !== "object") return false;
  const keys = Object.keys(left), other = Object.keys(right);
  return Array.isArray(left) === Array.isArray(right) && keys.length === other.length && keys.every(key => Object.prototype.hasOwnProperty.call(right, key) && equal((left as Record<string, unknown>)[key], (right as Record<string, unknown>)[key]));
}

/** Uses domain identities and bounded ordered slots for derived embeddings whose full input stays in the value. */
function recordKey(collection: string, value: unknown, index: number): string {
  const record = value as Record<string, unknown>;
  if (collection !== "embeddings") return record.id as string;
  return String(index).padStart(20, "0");
}

/** Projects individual domain records into the host object collections shared with migration. */
export function records(state: CharacterState): StoredRecord[] {
  const result: StoredRecord[] = [];
  for (const collection of ["cards", "groups", "tags", "stores", "conversationGroups"] as const) {
    for (const record of state[collection]) result.push({ collection, key: record.id, value: record as unknown as Storage.Value, version: null });
  }
  for (const owner of state.owners) {
    const { memories, links, chunks, candidates, embeddings, ...metadata } = owner;
    result.push({ collection: "owners", key: owner.ownerKey, value: metadata as unknown as Storage.Value, version: null });
    for (const collection of ownerCollections) for (const [index, record] of owner[collection].entries()) result.push({ collection, key: owner.ownerKey + "/" + recordKey(collection, record, index), value: { ownerKey: owner.ownerKey, record } as unknown as Storage.Value, version: null });
  }
  return result;
}

/** Reconstructs a validated operation snapshot from separately persisted host records. */
export function stateFromRecords(metadata: Storage.Value, rows: StoredRecord[]): CharacterState {
  if (metadata === null || typeof metadata !== "object" || Array.isArray(metadata)) throw new Error("Invalid character database metadata");
  if (metadata.migrationStatus === "writing") throw new Error("Character migration has not completed");
  const state = { version: metadata.version, nextId: metadata.nextId, active: metadata.active, cards: [], groups: [], tags: [], stores: [], conversationGroups: [], owners: [] } as unknown as CharacterState;
  const owners = new Map<string, MemorySpace>();
  for (const row of rows) {
    if (row.collection === "owners") {
      const owner = { ...(row.value as unknown as MemorySpace), memories: [], links: [], chunks: [], candidates: [], embeddings: [] };
      if (owner.ownerKey !== row.key || owners.has(row.key)) throw new Error("Invalid memory owner record: " + row.key);
      owners.set(row.key, owner); state.owners.push(owner);
    } else if (!ownerCollections.some(collection => collection === row.collection)) {
      (state[row.collection as "cards"] as unknown[]).push(row.value);
    }
  }
  for (const row of rows) {
    if (!ownerCollections.some(collection => collection === row.collection)) continue;
    const envelope = row.value as unknown as { ownerKey: string; record: Record<string, unknown> }, owner = owners.get(envelope.ownerKey);
    if (owner === undefined || row.key !== envelope.ownerKey + "/" + recordKey(row.collection, envelope.record, owner[row.collection as "memories"].length)) throw new Error("Invalid owner-scoped record: " + row.key);
    (owner[row.collection as "memories"] as unknown[]).push(envelope.record);
  }
  return state;
}
