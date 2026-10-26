import type { Storage } from "../../../../../types/storage";
import { clearGoal, restoreGoal, type GoalRecord } from "./goal_mode_state.js";

let database: Promise<Storage.ObjectDatabase> | null = null;

/** Opens the goal package's synchronized database without consulting chat workspaces. */
function openGoalDatabase(): Promise<Storage.ObjectDatabase> {
  if (database === null) {
    database = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/goals.sqlite" });
  }
  return database;
}

/** Reads the synchronized one-message goal input flag for a chat. */
export async function isGoalInputSlotEnabled(chatId: string): Promise<boolean> {
  const entry = await (await openGoalDatabase()).collection("inputSlots").get(chatId);
  if (entry === null) return false;
  if (entry.value !== true) throw new Error("Persisted goal input flag is invalid.");
  return true;
}

/** Persists the chat's goal input flag in the plugin-owned database. */
export async function setGoalInputSlotEnabled(chatId: string, enabled: boolean): Promise<void> {
  const slots = (await openGoalDatabase()).collection("inputSlots");
  if (enabled) await slots.put(chatId, true);
  else await slots.remove(chatId);
}

/** Validates a structured goal record read from the owning plugin database. */
function parsePersistedGoal(parsed: unknown): GoalRecord {
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("Persisted goal must be an object.");
  }
  const value = parsed as Record<string, unknown>;
  if (
    typeof value.objective !== "string" ||
    (value.status !== "active" && value.status !== "paused" && value.status !== "completed") ||
    typeof value.updatedAt !== "number" || typeof value.startedAt !== "number" ||
    typeof value.pausedDurationMs !== "number"
  ) throw new Error("Persisted goal has invalid fields.");
  for (const field of ["pausedAt", "completedAt", "lastReviewedMessageTimestamp"]) {
    if (value[field] !== undefined && typeof value[field] !== "number") {
      throw new Error("Persisted goal has an invalid timestamp: " + field);
    }
  }
  return value as GoalRecord;
}

/** Reads the current chat goal and clears memory when its durable record was deleted. */
export async function readPersistedGoal(chatId: string): Promise<GoalRecord | null> {
  const entry = await (await openGoalDatabase()).collection("goals").get(chatId);
  if (entry === null) { clearGoal(chatId); return null; }
  return restoreGoal(chatId, parsePersistedGoal(entry.value));
}

/** Saves a goal as plain structured data with absent optional properties omitted. */
export async function writePersistedGoal(chatId: string, goal: GoalRecord): Promise<GoalRecord> {
  const value: Record<string, Storage.Value> = {
    objective: goal.objective, status: goal.status, updatedAt: goal.updatedAt,
    startedAt: goal.startedAt, pausedDurationMs: goal.pausedDurationMs,
  };
  if (goal.pausedAt !== undefined) value.pausedAt = goal.pausedAt;
  if (goal.completedAt !== undefined) value.completedAt = goal.completedAt;
  if (goal.lastReviewedMessageTimestamp !== undefined) value.lastReviewedMessageTimestamp = goal.lastReviewedMessageTimestamp;
  await (await openGoalDatabase()).collection("goals").put(chatId, value);
  return restoreGoal(chatId, goal);
}

/** Removes the chat goal from plugin storage and runtime memory. */
export async function deletePersistedGoal(chatId: string): Promise<void> {
  await (await openGoalDatabase()).collection("goals").remove(chatId);
  clearGoal(chatId);
}

/** Emits current chat state after local or remote synchronized goal transactions. */
export async function* watchPersistedGoal(chatId: string, signal: AbortSignal): AsyncGenerator<GoalRecord | null> {
  const database = await openGoalDatabase();
  for await (const entry of database.collection("goals").watch(chatId, { signal })) {
    if (entry === null) {
      clearGoal(chatId);
      yield null;
    } else {
      yield restoreGoal(chatId, parsePersistedGoal(entry.value));
    }
  }
}
