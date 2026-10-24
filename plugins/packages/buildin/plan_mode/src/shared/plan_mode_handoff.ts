import { normalizePlanText, openPlanDatabase } from "./plan_mode_plan_file.js";

/** Preserves existing valid identities and hashes plans that exceed storage key limits. */
function handoffKey(chatId: string, content: string): string {
  const identity = chatId.length + ":" + chatId + normalizePlanText(content);
  let byteLength = 0;
  for (const character of identity) {
    const code = character.codePointAt(0)!;
    byteLength += code <= 0x7f ? 1 : code <= 0x7ff ? 2 : code <= 0xffff ? 3 : 4;
    if (code <= 0x1f || (code >= 0x7f && code <= 0x9f) || byteLength > 1024) {
      // The prefix cannot collide with legacy identities, which always start with a chat-id length.
      return "hashed:" + CryptoJS.MD5(identity).toString();
    }
  }
  return identity;
}

/** Reads the durable handoff marker shared by all devices and UI runtimes. */
export async function isPlanStarted(chatId: string, content: string): Promise<boolean> {
  return (await (await openPlanDatabase()).collection("startedPlans").get(handoffKey(chatId, content))) !== null;
}

/** Claims a plan handoff with an absence precondition rather than a runtime-local flag. */
export async function claimPlanStart(chatId: string, content: string): Promise<boolean> {
  const plan = normalizePlanText(content);
  if (!plan) return false;
  const collection = (await openPlanDatabase()).collection("startedPlans");
  const key = handoffKey(chatId, plan);
  if (await collection.get(key) !== null) return false;
  await collection.put(key, { chatId, content: plan }, { expectedVersion: null });
  return true;
}

/** Removes a handoff marker when starting implementation fails. */
export async function forgetPlanStart(chatId: string, content: string): Promise<void> {
  await (await openPlanDatabase()).collection("startedPlans").remove(handoffKey(chatId, content));
}

/** Streams synchronized handoff changes for one exact chat and plan identity. */
export async function* watchPlanStarted(chatId: string, content: string, signal: AbortSignal): AsyncGenerator<boolean> {
  const key = handoffKey(chatId, content);
  const database = await openPlanDatabase();
  for await (const entry of database.collection("startedPlans").watch(key, { signal })) {
    yield entry !== null;
  }
}
