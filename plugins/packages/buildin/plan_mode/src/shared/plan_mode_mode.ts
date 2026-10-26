import { openPlanDatabase } from "./plan_mode_plan_file.js";

/** Reads the planning flag from synchronized plugin state. */
export async function isPlanModeEnabledForChat(chatId: string): Promise<boolean> {
  const entry = await (await openPlanDatabase()).collection("modes").get(chatId);
  if (entry === null) return false;
  if (entry.value !== true) throw new Error("Persisted plan mode flag is invalid");
  return true;
}

/** Enables planning for a chat across the device space. */
export async function enablePlanModeForChat(chatId: string): Promise<void> {
  await (await openPlanDatabase()).collection("modes").put(chatId, true);
}

/** Removes the synchronized planning flag for the chat. */
export async function disablePlanMode(chatId: string): Promise<void> {
  await (await openPlanDatabase()).collection("modes").remove(chatId);
}
