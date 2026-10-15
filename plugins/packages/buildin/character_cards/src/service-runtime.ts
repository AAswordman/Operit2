import type { DomainInput, DomainOperation, DomainOutput } from "./api";
import type { CharacterDirectories, Request, RequestOutput } from "./model";
import type { CharacterCardsService } from "./service";
import { createCharacterCardsService } from "./service";
import { DatabaseCharacterRepository } from "./storage/database";

/** Owns one retained initialization result for a single JavaScript runtime owner. */
export interface CharacterCardsServiceRuntime {
  /** Resolves the same initialized business service, including the same rejection. */
  getService(): Promise<CharacterCardsService>;
}

/** Defers business initialization until first use and never resets a rejected Promise. */
export function createServiceRuntime(initialize: () => Promise<CharacterCardsService>): CharacterCardsServiceRuntime {
  let initialization: Promise<CharacterCardsService> | null = null;
  return {
    /** Captures the initialization Promise before invoking the supplied business initializer. */
    getService(): Promise<CharacterCardsService> {
      if (initialization === null) initialization = Promise.resolve().then(initialize);
      return initialization;
    },
  };
}

let directorySources: CharacterDirectories | null = null;
let opening = false;

/** Connects real directory readers before database initialization without evaluating them during registration. */
export function connectDirectorySources(sources: CharacterDirectories): void {
  if (opening || directorySources !== null) throw new Error("External directory readers must be connected exactly once before service initialization");
  directorySources = sources;
}

/** Creating the lazy owner has no side effects; UI-only bundles can omit the entire backend. */
const runtime = /* @__PURE__ */ createServiceRuntime(
  /** Opens only this plugin's record database on first use, never during registration. */
  async (): Promise<CharacterCardsService> => {
    opening = true;
    return createCharacterCardsService(await DatabaseCharacterRepository.open(directorySources));
  },
);

/** Initializes the same service used by first calls when the application lifecycle is dispatched. */
export async function initializeService(): Promise<void> {
  await runtime.getService();
}

/** Returns the business service shared by commands, public APIs, UI requests, and providers. */
export function getService(): Promise<CharacterCardsService> {
  return runtime.getService();
}

/** Dispatches a UI operation only after the retained initialization has succeeded. */
export async function dispatch<R extends Request>(request: R): Promise<RequestOutput<R>> {
  return (await getService()).dispatch(request);
}

/** Executes a typed domain method on the same initialized business service. */
export async function dispatchDomain<K extends DomainOperation>(operation: K, input: DomainInput<K>): Promise<DomainOutput<K>> {
  return (await getService()).dispatchDomain(operation, input);
}

/** Advances genuine memory jobs through the one retained main-runtime business service. */
export async function runMemoryJobs(): Promise<{ owners: number; rebuildWindows: number; candidateBatches: number }> {
  return (await getService()).runMemoryJobs();
}
