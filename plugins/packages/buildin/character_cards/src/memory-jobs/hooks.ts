import { dispatchDomain, runMemoryJobs } from "../service-runtime";
import { assertInteger, assertString, requireId } from "../validation";

/** Enqueues the persisted assistant revision whose saved message snapshot owns the memory destination. */
export async function onMemoryMessagePersisted(event: ToolPkg.ChatMessageHookEvent): Promise<void> {
  const payload = event.eventPayload;
  assertString(payload.sender, "persisted message sender");
  if (payload.sender !== "ai" && payload.sender !== "assistant") return;
  const chatId = requireId(payload.chatId, "persisted chat id"); assertInteger(payload.timestamp, "persisted message timestamp", 0);
  assertString(payload.content, "persisted message content");
  if (payload.content.trim() === "") return;
  assertInteger(payload.selectedVariantIndex, "persisted message selected variant index", 0);
  if (payload.selectedVariantIndex > 2147483647) throw new Error("Persisted message variant exceeds the host range");
  await dispatchDomain("memory.candidate.enqueue", { chatId, timestamp: payload.timestamp, variantIndex: payload.selectedVariantIndex, sourceType: "reply_finalized_auto", ownerKey: null });
}

/** Advances the actual durable plans and pending queue on the existing generic host interval. */
export async function onMemoryInterval(_event: ToolPkg.HostEventIntervalHookEvent<ToolPkg.JsonObject>): Promise<void> {
  await runMemoryJobs();
}

/** Registers real message and interval callbacks without opening storage during package registration. */
export function registerMemoryJobHooks(): void {
  ToolPkg.registerChatMessageHook({ id: "memory-candidate-enqueue", function: onMemoryMessagePersisted });
  ToolPkg.registerHostEventHook<ToolPkg.JsonObject>({ id: "memory-jobs-interval", source: "interval", trigger: { kind: "interval", intervalMs: 60000 }, function: onMemoryInterval });
}
