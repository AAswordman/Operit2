import type { CharacterDirectories } from "./model";
import { DatabaseCharacterRepository } from "./storage/database";
import { repairCharacterInitialization, type InitializationRecoverySource } from "./storage/initialization";

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

/** Serializes explicit repair with the failed opener so requests share the recovered owner. */
export async function recoverRepository(source: InitializationRecoverySource | null): Promise<DatabaseCharacterRepository> {
  const previous = repository;
  if (previous !== undefined) {
    let failed = false;
    try { await previous; } catch { failed = true; }
    if (!failed) throw new Error("角色卡服务已经正常运行，无需修复初始化。");
  }
  if (repository !== previous) throw new Error("已有初始化或修复请求正在执行。");
  repository = Promise.resolve().then(async () => {
    if (source !== null) await repairCharacterInitialization(source);
    return DatabaseCharacterRepository.open(directories);
  });
  return repository;
}
