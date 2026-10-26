import type { CharacterDirectories } from "./model";
import { DatabaseCharacterRepository } from "./storage/database";

let directories: CharacterDirectories | null = null;
let repository: Promise<DatabaseCharacterRepository> | undefined;

/** Connects external catalogs before any main-runtime caller opens the repository. */
export function connectDirectorySources(sources: CharacterDirectories): void {
  if (repository !== undefined || directories !== null) throw new Error("External directory readers must be connected exactly once before service initialization");
  directories = sources;
}

/** Owns migration, first-install initialization and all handles for this main runtime. */
export function getRepository(): Promise<DatabaseCharacterRepository> {
  return repository ??= Promise.resolve().then(() => DatabaseCharacterRepository.open(directories));
}
