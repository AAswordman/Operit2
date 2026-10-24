import type { Storage } from "../../../../../types/storage";
import type { CharacterStorage } from "./preferences";
import { assertObject } from "../validation";
import { migrateLegacyToV1 } from "./migrations/legacy-to-v1";

export const CURRENT_CHARACTER_SCHEMA_VERSION = 1;

/** Converts released Core data directly into the first plugin-owned schema. */
export async function migrateCharacterSchema(storage: CharacterStorage, preferences: Storage.MutablePreferences, directory: string): Promise<void> {
  const metadata = preferences.get<Record<string, Storage.Value>>("state");
  if (metadata === undefined) return;
  assertObject(metadata, "character schema metadata");
  if (metadata.migrationStatus === "writing") throw new Error("Character migration has not completed: legacy import is still writing");
  if (metadata.version !== 1) throw new Error("Unsupported character schema version: " + metadata.version);
  await migrateLegacyToV1(storage, preferences, directory, metadata);
}
