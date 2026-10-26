import type { Storage } from "../../../../../types/storage";

export type PlanFileRecord = { chatId: string; content: string };
let database: Promise<Storage.ObjectDatabase> | null = null;

/** Opens the plan package's synchronized object database. */
export function openPlanDatabase(): Promise<Storage.ObjectDatabase> {
  if (database === null) database = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/plans.sqlite" });
  return database;
}

/** Normalizes plan text for stable comparison and persistence. */
export function normalizePlanText(content: string): string { return content.replace(/\r\n/g, "\n").trim(); }

/** Requires nonempty plan text and terminates it consistently. */
export function normalizePlanContent(content: string): string {
  const normalized = normalizePlanText(content);
  if (!normalized) throw new Error("plan content is empty");
  return normalized + "\n";
}

/** Reports whether the chat owns a persisted plan. */
export async function hasPlanFile(chatId: string): Promise<boolean> {
  return (await (await openPlanDatabase()).collection<Storage.Value>("plans").get(chatId)) !== null;
}

/** Reads a structured plan from plugin storage, independently of active views. */
export async function readPlanFile(chatId: string): Promise<PlanFileRecord | null> {
  const entry = await (await openPlanDatabase()).collection<Storage.Value>("plans").get(chatId);
  if (entry === null) return null;
  return parsePlanFile(chatId, entry.value);
}

/** Validates one structured plan record for both reads and live observation. */
function parsePlanFile(chatId: string, value: Storage.Value): PlanFileRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value) || typeof value.content !== "string") {
    throw new Error("Persisted plan has invalid content");
  }
  return { chatId, content: value.content };
}

/** Compares a candidate plan with this chat's durable plan. */
export async function planFileMatchesContent(chatId: string, content: string): Promise<boolean> {
  const normalized = normalizePlanText(content);
  if (!normalized) return false;
  const plan = await readPlanFile(chatId);
  return plan !== null && normalizePlanText(plan.content) === normalized;
}

/** Commits normalized plan content to the owning package's collection. */
export async function writePlanFile(chatId: string, content: string): Promise<PlanFileRecord> {
  const normalized = normalizePlanContent(content);
  await (await openPlanDatabase()).commit([
    { op: "put", collection: "plans", key: chatId, value: { content: normalized } },
    { op: "delete", collection: "modes", key: chatId },
  ]);
  return { chatId, content: normalized };
}

/** Removes the chat's plan with an exact read-version precondition. */
export async function deletePlanFile(chatId: string): Promise<{ chatId: string; deleted: boolean }> {
  const plans = (await openPlanDatabase()).collection("plans");
  const entry = await plans.get(chatId);
  if (entry !== null) await plans.remove(chatId, { expectedVersion: entry.version });
  return { chatId, deleted: entry !== null };
}

/** Streams current plan records when a local or remote transaction changes the chat. */
export async function* watchPlanFile(chatId: string, signal: AbortSignal): AsyncGenerator<PlanFileRecord | null> {
  const database = await openPlanDatabase();
  for await (const entry of database.collection<Storage.Value>("plans").watch(chatId, { signal })) {
    yield entry === null ? null : parsePlanFile(chatId, entry.value);
  }
}
