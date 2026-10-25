import type { Storage } from "../../../../../types/storage";

import merge from "./memory-merge.json";
import type { StorageMergeDescriptor } from "../../../../../types/storage.generated";

export const characterMerge = merge as StorageMergeDescriptor;

export type ObjectMutation = Storage.ObjectMutation;

/** Bounds one host list page under the structured bridge budget; 1024-dimension vectors make a 1000-record page exceed it. */
const HOST_READ_PAGE_LIMIT = 200;

/** Keeps shrinking a rejected page down to this floor before rethrowing the read failure. */
const HOST_READ_PAGE_FLOOR = 32;

/** Stores memory owners and memory objects independently of JSON configuration preferences. */
export class CharacterMemoryDatabase {
  /** Retains one authenticated object handle without declaring business SQL tables. */
  private constructor(private readonly database: Storage.ObjectDatabase) {}

  /** Opens the sole synchronized character namespace independently of plugin installation scope. */
  static async open(): Promise<CharacterMemoryDatabase> {
    const directory = ToolPkg.getSpaceDataDir();
    if (typeof directory !== "string" || directory.trim() === "")
      throw new Error("Plugin data directory is unavailable");
    const database = new CharacterMemoryDatabase(await Tools.Storage.objects.open({ path: directory + "/memory.sqlite", merge: characterMerge }));
    return database;
  }

  /** Closes the retained memory engine handle. */
  close(): Promise<void> { return this.database.close(); }

  /**
   * Reads complete JSON values in primary-key order using bounded host pages. Vector
   * records push one page past the host structured bridge budget, so a rejected page
   * shrinks toward the floor instead of failing the whole collection read.
   */
  async read<T = Storage.Value>(collection: string): Promise<Storage.ListedEntry<T>[]> {
    const result: Storage.ListedEntry<T>[] = [];
    const source = this.database.collection<T>(collection);
    let after: string | null = null, pageSize = HOST_READ_PAGE_LIMIT;
    for (;;) {
      let page: Storage.ListedEntry<T>[];
      try {
        page = await source.list(after, pageSize);
      } catch (error) {
        if (pageSize <= HOST_READ_PAGE_FLOOR) throw error;
        pageSize = Math.max(HOST_READ_PAGE_FLOOR, Math.floor(pageSize / 2));
        continue;
      }
      result.push(...page);
      if (page.length < pageSize) return result;
      after = page[page.length - 1].key;
    }
  }

  /** Reads one exact document and its host-issued concurrency version. */
  get<T = Storage.Value>(collection: string, key: string): Promise<Storage.Entry<T> | null> {
    return this.database.collection<T>(collection).get(key);
  }

  /** Reads selected identities in one host snapshot without scanning their collections. */
  getMany<T = Storage.Value>(collection: string, keys: readonly string[]): Promise<(Storage.Entry<T> | null)[]> {
    return this.database.collection<T>(collection).getMany([...keys]);
  }

  /** Publishes complete changed JSON documents and memory objects in one guarded transaction. */
  commit(mutations: ObjectMutation[]): Promise<Storage.Receipt> {
    return this.database.commit(mutations);
  }
}
