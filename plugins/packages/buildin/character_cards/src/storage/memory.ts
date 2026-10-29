import type { Storage } from "../../../../../types/storage";
import merge from "./memory-merge.json";
import type { StorageRecordGuard, StorageMergeDescriptor } from "../../../../../types/storage.generated";

import { closeMemoryIndexes } from "../search/state";

export const characterMerge = merge as StorageMergeDescriptor;
export type ObjectMutation = Storage.ObjectMutation;
const HOST_READ_PAGE_LIMIT = 100;

/** Stores synchronized memory records and executes bounded database queries. */
export class CharacterMemoryDatabase {
  /** Retains one authenticated storage handle. */
  private constructor(private readonly database: Storage.ObjectDatabase) {}

  /** Opens the sole synchronized character namespace through the shared host. */
  static async open(): Promise<CharacterMemoryDatabase> {
    const directory = ToolPkg.getSpaceDataDir();
    if (typeof directory !== "string" || directory.trim() === "") throw new Error("Plugin data directory is unavailable");
    return new CharacterMemoryDatabase(await Tools.Storage.objects.open({ path: directory + "/memory.sqlite", merge: characterMerge }));
  }

  /** Closes the retained memory database handle. */
  async close(): Promise<void> { await closeMemoryIndexes(this); return this.database.close(); }

  /** Reads explicit backup or migration pages without hiding storage failures. */
  async read<T = Storage.Value>(collection: string): Promise<Storage.ListedEntry<T>[]> {
    const result: Storage.ListedEntry<T>[] = [];
    const source = this.database.collection<T>(collection);
    let after: string | null = null;
    for (;;) {
      const page = await source.list(after, HOST_READ_PAGE_LIMIT);
      result.push(...page);
      if (page.length < HOST_READ_PAGE_LIMIT) return result;
      after = page[page.length - 1].key;
    }
  }

  /** Reads the local change journal; synchronized replays and other handles are included. */
  changes(after: string | null, limit = 100) { return this.database.changes(after, limit); }

  /** Executes simple database filtering and bounded projections. */
  query(sql: string, params: Storage.SqlValue[] = []): Promise<Record<string, Storage.SqlValue>[]> {
    return this.database.query(sql, params);
  }

  /** Reads one owner's records using the indexed collection and primary-key range. */
  async select<T>(collection: string, ownerKey: string, condition = "1", params: Storage.SqlValue[] = [], limit = 100, after = ""): Promise<Storage.ListedEntry<T>[]> {
    const rows = await this.query("SELECT key,value,version FROM plugin_objects WHERE collection=? AND key>=? AND key<? AND key>? AND (" + condition + ") ORDER BY key LIMIT ?",
      [collection, ownerKey + "/", ownerKey + "0", after, ...params, limit]);
    return rows.map(/** Decodes only database-selected records. */ row => ({ key: String(row.key), value: JSON.parse(String(row.value)) as T, version: String(row.version) }));
  }

  /** Pages an explicitly requested owner export without loading other owners. */
  async all<T>(collection: string, ownerKey: string, condition = "1", params: Storage.SqlValue[] = []): Promise<Storage.ListedEntry<T>[]> {
    const result: Storage.ListedEntry<T>[] = [];
    let after = "";
    for (;;) {
      const page = await this.select<T>(collection, ownerKey, condition, params, HOST_READ_PAGE_LIMIT, after);
      result.push(...page);
      if (page.length < HOST_READ_PAGE_LIMIT) return result;
      after = page[page.length - 1].key;
    }
  }

  /** Removes the retired text cache after schema imports without reading vector payloads. */
  async retireLegacyEmbeddings(): Promise<void> {
    for (;;) {
      const rows = await this.query("SELECT key,version FROM plugin_objects WHERE collection='embeddings' ORDER BY key LIMIT 100");
      if (rows.length === 0) return;
      await this.commit(rows.map(/** Deletes only exact cache versions selected by this migration. */ row => ({ op: "delete", collection: "embeddings", key: String(row.key), expectedVersion: String(row.version) })));
    }
  }

  /** Reads one exact record and its host-issued concurrency version. */
  get<T = Storage.Value>(collection: string, key: string): Promise<Storage.Entry<T> | null> {
    return this.database.collection<T>(collection).get(key);
  }

  /** Reads selected identities in one host snapshot. */
  getMany<T = Storage.Value>(collection: string, keys: readonly string[]): Promise<(Storage.Entry<T> | null)[]> {
    return this.database.collection<T>(collection).getMany([...keys]);
  }

  /** Publishes guarded source records and vectors in one synchronized transaction. */
  commit(mutations: ObjectMutation[], guards: StorageRecordGuard[] = []): Promise<Storage.Receipt> { return this.database.commit(mutations, guards); }
}
