import schema from "./meta-schema.json";
import type { Storage } from "../../../../../types/storage";
import type { Card, GlobalMemoryPreferences, MemoryRecallSettings, MemorySpace } from "../model";
import { readCharacter } from "../drafts";
import { assertBoolean, assertObject, requireId } from "../validation";
import { CharacterStorage, configurationNames, recordPrefix } from "./preferences";
import { equal, storedValue } from "./records";
import { assertMemorySpace, createInitialState, normalizeMemoryRecallSettings, userDocumentPath } from "./state";

interface InitializationPlan {
  version: 1;
  card: Card;
  memory: Omit<MemorySpace, "ownerKey" | "userDocumentPath">;
  memoryPreferences: GlobalMemoryPreferences;
  memoryRecall: MemoryRecallSettings;
}

export interface InitializationRecoverySource {
  card: Card;
  owner: Storage.Value | null;
}

export type InitializationInspection =
  | { status: "ready" }
  | { status: "planned"; cardName: string }
  | { status: "repairable"; source: InitializationRecoverySource };

export type InitializationRecoveryRequest =
  | { action: "inspect" }
  | { action: "repair"; source: InitializationRecoverySource }
  | { action: "resume" };

/** Builds explicitly approved intent from an existing card and its actual owner settings. */
function recoveryPlan(source: InitializationRecoverySource): InitializationPlan {
  const plan = createPlan();
  plan.card = source.card;
  if (source.owner !== null) {
    assertObject(source.owner, "existing memory owner");
    const { ownerKey, userDocumentPath, ...memory } = source.owner;
    plan.memory = { ...memory, memories: [], links: [], chunks: [], candidates: [], embeddings: [] } as unknown as InitializationPlan["memory"];
  }
  assertPlan(plan);
  return plan;
}

/** Reads only the supported interrupted-first-install shape without matching default template content. */
async function recoverySource(storage: CharacterStorage): Promise<InitializationRecoverySource> {
  const entries = (await storage.configuration.cards.data()).entries();
  if (entries.length !== 1 || !entries[0][0].startsWith(recordPrefix))
    throw new Error("此修复仅处理首次安装中断，需要保留恰好一张角色卡；不会清空或接管其他数据。");
  const [key, value] = entries[0];
  const card = readCharacter(value, key.slice(recordPrefix.length));
  if (key !== recordPrefix + card.id || !equal(card, value)) throw new Error("角色卡格式不完整，不能作为首次初始化修复来源。");
  const owner = await storage.memory.get("owners", "character:" + card.id);
  const source = { card, owner: owner === null ? null : owner.value };
  await validateRecords(storage, recoveryPlan(source));
  return source;
}

/** Inspects failed initialization independently of the unavailable service without migrating records. */
export async function inspectCharacterInitialization(): Promise<InitializationInspection> {
  const storage = await CharacterStorage.openForRecovery();
  try {
    const metadata = await storage.metadata.data();
    if (metadata.contains("state")) return { status: "ready" };
    const pending = metadata.get("initialization");
    if (pending !== undefined) {
      assertPlan(pending);
      return { status: "planned", cardName: pending.card.name };
    }
    return { status: "repairable", source: await recoverySource(storage) };
  } finally { await storage.close(); }
}

/** Records approved recovery only while the inspected card and owner remain unchanged. */
export async function repairCharacterInitialization(expected: InitializationRecoverySource): Promise<void> {
  const storage = await CharacterStorage.openForRecovery();
  try {
    storage.metadata.withSchema(schema.version, schema.steps as Storage.MigrationRule[][]);
    const source = await recoverySource(storage);
    if (!equal(source, expected)) throw new Error("数据已在检查后发生变化，请重新检查再确认修复。");
    const plan = recoveryPlan(source);
    await storage.metadata.edit(preferences => {
      if (preferences.contains("state") || preferences.contains("initialization"))
        throw new Error("初始化状态已发生变化，请重新检查。");
      preferences.set("initialization", storedValue(plan));
    });
    await initializeCharacterStorage(storage, ToolPkg.getSpaceDataDir());
  } finally { await storage.close(); }
}

/** Fixes the complete installation payload before creating any business records. */
function createPlan(): InitializationPlan {
  const state = createInitialState(Date.now());
  const { ownerKey, userDocumentPath, ...memory } = state.owners[0];
  return { version: 1, card: state.cards[0], memory, memoryPreferences: state.memoryPreferences, memoryRecall: state.memoryRecall };
}

/** Derives owner identities from the sole persisted card identity, including after a space merge. */
function plannedOwner(plan: InitializationPlan): MemorySpace {
  const ownerKey = "character:" + plan.card.id;
  return { ...plan.memory, ownerKey, userDocumentPath: userDocumentPath(ownerKey) };
}

/** Projects the owner record without its independently stored empty collections. */
function plannedOwnerMetadata(plan: InitializationPlan) {
  const { memories, links, chunks, candidates, embeddings, ...metadata } = plannedOwner(plan);
  return metadata;
}

/** Rejects invalid plans without consulting the current default character template. */
function assertPlan(value: unknown): asserts value is InitializationPlan {
  assertObject(value, "character initialization plan");
  if (value.version !== 1) throw new Error("Unsupported character initialization plan");
  assertObject(value.card, "initial character");
  if (!equal(readCharacter(value.card, requireId(value.card.id, "initial character id")), value.card))
    throw new Error("Invalid character initialization card");
  assertObject(value.memory, "initial memory space");
  const owner = plannedOwner(value as unknown as InitializationPlan);
  assertMemorySpace(owner);
  for (const name of ["memories", "links", "chunks", "candidates", "embeddings"] as const)
    if (owner[name].length !== 0) throw new Error("Character initialization plan contains memory records");
  if (owner.rebuildTask !== null) throw new Error("Character initialization plan contains a memory job");
  assertObject(value.memoryPreferences, "initial memory preferences");
  assertBoolean(value.memoryPreferences.userDescriptionEnabled, "user description enabled");
  assertBoolean(value.memoryPreferences.autoUpdateEnabled, "memory auto update enabled");
  if (!equal(normalizeMemoryRecallSettings(value.memoryRecall), value.memoryRecall))
    throw new Error("Character initialization plan has incomplete recall settings");
}

/** Requires an empty store or the exact records authorized by a durable installation plan. */
async function validateRecords(storage: CharacterStorage, plan: InitializationPlan | undefined): Promise<void> {
  const configuration = await storage.data();
  for (const name of configurationNames) {
    const entries = configuration[name].entries();
    if (entries.length === 0) continue;
    if (plan === undefined || name !== "cards" || entries.length !== 1
      || entries[0][0] !== recordPrefix + plan.card.id || !equal(entries[0][1], plan.card))
      throw new Error("Character initialization found unplanned configuration: " + name);
  }
  const rows = await storage.memory.query("SELECT collection,key,value FROM plugin_objects LIMIT 2");
  if (rows.length === 0) return;
  if (plan === undefined) throw new Error("Character initialization found memory records without a plan");
  const expected = plannedOwnerMetadata(plan);
  if (rows.length !== 1 || rows[0].collection !== "owners" || rows[0].key !== expected.ownerKey
    || !equal(JSON.parse(String(rows[0].value)), expected))
    throw new Error("Character initialization found unplanned memory records");
}

/** Executes only persisted installation intent and atomically publishes its completion marker. */
export async function initializeCharacterStorage(storage: CharacterStorage, directory: string): Promise<void> {
  const metadata = await storage.metadata.data();
  if (metadata.contains("state")) {
    if (metadata.contains("initialization")) throw new Error("Character initialization plan remains beside completed metadata");
    return;
  }
  let persisted = metadata.get<Storage.Value>("initialization");
  if (persisted === undefined) {
    await validateRecords(storage, undefined);
    const plan = createPlan();
    await storage.metadata.edit(preferences => {
      if (preferences.contains("state") || preferences.contains("initialization"))
        throw new Error("Character initialization changed concurrently");
      preferences.set("initialization", storedValue(plan));
    });
    persisted = storedValue(plan);
  }
  assertPlan(persisted);
  const plan = persisted;
  await validateRecords(storage, plan);
  await storage.configuration.cards.edit(preferences => {
    const entries = preferences.entries();
    if (entries.length === 1 && entries[0][0] === recordPrefix + plan.card.id && equal(entries[0][1], plan.card)) return;
    if (entries.length !== 0) throw new Error("Character initialization card changed concurrently");
    preferences.set(recordPrefix + plan.card.id, storedValue(plan.card));
  });
  const owner = plannedOwner(plan);
  const existing = await storage.memory.get("owners", owner.ownerKey);
  const ownerMetadata = plannedOwnerMetadata(plan);
  if (existing === null) {
    await storage.memory.commit([{ op: "put", collection: "owners", key: owner.ownerKey, value: storedValue(ownerMetadata), expectedVersion: null }]);
  } else if (!equal(existing.value, ownerMetadata)) throw new Error("Character initialization owner changed concurrently");
  const path = directory + "/" + owner.userDocumentPath;
  const file = await Tools.Files.exists(path);
  if (file.exists) {
    if (file.isDirectory) throw new Error("Character initialization document path is a directory");
  } else {
    const created = await Tools.Files.mkdir(path.slice(0, path.lastIndexOf("/")), true);
    if (!created.successful) throw new Error(created.details);
    const { operation } = await Tools.Files.write(path, "", false);
    if (!operation.successful) throw new Error(operation.details);
  }
  await validateRecords(storage, plan);
  await storage.metadata.edit(preferences => {
    if (preferences.contains("state") || !equal(preferences.get("initialization"), plan))
      throw new Error("Character initialization changed concurrently");
    preferences.set("state", { version: 1, active: { CharacterCard: { id: plan.card.id } }, memoryPreferences: plan.memoryPreferences, memoryRecall: plan.memoryRecall });
    preferences.remove("initialization");
  });
}
