import type { Storage } from "../../../../../types/storage";
import declarations from "./merge.json";
import type { StoragePreferencesMergeDescriptor } from "../../../../../types/storage.generated";
const merge = declarations as Record<keyof typeof declarations, StoragePreferencesMergeDescriptor>;
import { clone } from "./records";
import { migrateCharacterStorage } from "./migrations/legacy-to-v1";
import { assertObject } from "../validation";
import { CharacterMemoryDatabase } from "./memory";

export const configurationNames = ["cards", "groups", "tags", "stores", "conversationGroups"] as const;
export type ConfigurationName = typeof configurationNames[number];
export type ConfigurationSnapshot = Record<ConfigurationName, Storage.Preferences>;
export const recordPrefix = "record_";

/** Reads complete configuration documents from an immutable native preference snapshot. */
export function documents<T>(preferences: Storage.Preferences): T[] {
  return preferences.entries().filter(([key]) => key.startsWith(recordPrefix)).map(([, value]) => clone(value) as T);
}

/** Selects one complete configuration document without a collection adapter or revision counter. */
export function document<T>(preferences: Storage.Preferences, id: string): T | undefined {
  return clone(preferences.get<T>(recordPrefix + id));
}

/** Retains the native preference owners independently of the memory object engine. */
export class CharacterStorage {
  /** Retains explicit storage owners without emulating a cross-engine transaction. */
  private constructor(
    readonly configuration: Record<ConfigurationName, Storage.DataStore>,
    readonly metadata: Storage.DataStore,
    readonly chats: Storage.DataStore,
    readonly migration: Storage.DataStore,
    readonly memory: CharacterMemoryDatabase,
  ) {}

  /** Opens JSON configuration owners and upgrades the plugin schema before business reads. */
  static async open(): Promise<CharacterStorage> {
    const directory = ToolPkg.getSpaceDataDir();
    if (typeof directory !== "string" || directory.trim() === "") throw new Error("Plugin data directory is unavailable");
    const handles: Storage.DataStore[] = [];
    let memory: CharacterMemoryDatabase | undefined;
    try {
      /** Opens a declared JSON owner through the native preferences interface. */
      async function open(name: keyof typeof merge): Promise<Storage.DataStore> {
        const store = await Tools.Storage.dataStore.open({ path: directory + "/" + name + ".preferences.json", merge: merge[name] });
        handles.push(store);
        return store;
      }
      const configuration = {} as Record<ConfigurationName, Storage.DataStore>;
      for (const name of configurationNames) configuration[name] = await open(name);
      const metadata = await open("meta"), chats = await open("chats");
      const migration = await Tools.Storage.dataStore.open({ path: directory + "/migration.preferences.json" });
      handles.push(migration);
      memory = await CharacterMemoryDatabase.open();
      const storage = new CharacterStorage(configuration, metadata, chats, migration, memory);
      // The schema callback only marks the format upgrade. File and record work belongs
      // to the repository's initialization, outside any pending preferences edit.
      metadata.withSchema(1, (version, preferences) => {
        if (version !== 0) throw new Error("Missing character schema migration: " + version);
        const state = preferences.get<Record<string, Storage.Value>>("state");
        if (state !== undefined) {
          assertObject(state, "character schema metadata");
          preferences.set("state", { ...state, legacyImportPending: true });
        }
      });
      await metadata.data();
      await migrateCharacterStorage(storage, directory);
      await memory.retireLegacyEmbeddings();
      return storage;
    } catch (error) {
      const closed = await Promise.allSettled([...handles.map(handle => handle.close()), ...(memory === undefined ? [] : [memory.close()])]);
      const failures = closed.filter((result): result is PromiseRejectedResult => result.status === "rejected").map(result => result.reason);
      if (failures.length !== 0) throw Object.assign(new Error("Character storage open and handle cleanup failed"), { cause: error, cleanupErrors: failures });
      throw error;
    }
  }

  /** Reads each configuration owner's immutable snapshot without loading memory objects. */
  async data(): Promise<ConfigurationSnapshot> {
    const result = {} as ConfigurationSnapshot;
    for (const name of configurationNames) result[name] = await this.configuration[name].data();
    return result;
  }
}
