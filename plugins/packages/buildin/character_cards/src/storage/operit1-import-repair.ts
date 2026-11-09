import type { Storage } from "../../../../../types/storage";
import type { CharacterStorage } from "./preferences";
import { assertObject } from "../validation";
import { clone } from "./records";

type Document = Record<string, Storage.Value>;

/** Releases completed Operit1 imports from the retired conversion gate without reading memory records. */
export async function finishOperit1Import(storage: CharacterStorage): Promise<void> {
  const source = (await storage.metadata.data()).get<Document>("state");
  if (source === undefined || source.legacyImportPending !== true) return;
  if (source.migrationStatus === "writing") throw new Error("Operit1 import is still writing");
  if (typeof source.migrationSource !== "string" || !source.migrationSource.startsWith("operit1:"))
    throw new Error("Unsupported retired character migration; import the Operit1 snapshot directly");
  await storage.metadata.edit(preferences => {
    const current = preferences.get<Document>("state");
    assertObject(current, "Operit1 import metadata");
    if (current.migrationSource !== source.migrationSource || current.migrationStatus === "writing")
      throw new Error("Operit1 import changed during completion");
    const repaired = clone(current);
    delete repaired.legacyImportPending;
    preferences.set("state", repaired);
  });
}
