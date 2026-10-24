import type { Storage } from "../../../../../types/storage";

import merge from "./memory-merge.json";
import type { StorageMergeDescriptor } from "../../../../../types/storage.generated";

export const characterMerge = merge as StorageMergeDescriptor;

export type ObjectMutation = Storage.ObjectMutation;

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

  /** Reads complete JSON values in primary-key order using bounded host pages. */
  async read<T = Storage.Value>(collection: string): Promise<Storage.ListedEntry<T>[]> {
    const result: Storage.ListedEntry<T>[] = [];
    const source = this.database.collection<T>(collection);
    let after: string | null = null;
    for (;;) {
      const page = await source.list(after, 1000);
      result.push(...page);
      if (page.length < 1000) return result;
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
