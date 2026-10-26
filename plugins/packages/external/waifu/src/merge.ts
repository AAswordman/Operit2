import { modeForChat } from "./mode.js";

type Request = Parameters<typeof Tools.Chat.sendMessage>[0];
interface Batch { chatId: string; runtime: Request["runtime"]; notifyReply: boolean; text: string[]; timer: number }
const batches = new Map<string, Batch>();
const operations = new Map<string, Promise<unknown>>();
const deliveries = new Map<string, Promise<void>>();

/** Executes accepted requests in chat order outside the synchronous input-hook deadline. */
async function deliver(request: Request): Promise<void> {
  const previous = deliveries.get(request.chatId);
  const operation = (async () => {
    if (previous !== undefined) await previous;
    const receipt = await Tools.Chat.sendMessage(request);
    if (receipt.outcome.type !== "committed" || receipt.outcome.status !== "completed") throw new Error(`Waifu queued send did not complete: ${JSON.stringify(receipt.outcome)}`);
  })();
  deliveries.set(request.chatId, operation);
  try { await operation; }
  finally { if (deliveries.get(request.chatId) === operation) deliveries.delete(request.chatId); }
}

/** Reports the actual native error for one accepted delivery without retrying model execution. */
function submit(request: Request): void {
  void deliver(request).catch(error => Tools.System.toast(`Waifu 合并发送失败：${error instanceof Error ? error.message : String(error)}`));
}

/** Removes one timer and transfers the exact accepted text batch to native delivery. */
function flush(chatId: string): void {
  const batch = batches.get(chatId);
  if (batch === undefined) return;
  batches.delete(chatId); clearTimeout(batch.timer);
  submit({ kind: "submit", chatId: batch.chatId, runtime: batch.runtime,
    input: { text: batch.text.join("\n"), attachments: [], replyToMessageTimestamp: null },
    turn: { kind: "execute" }, notifyReply: batch.notifyReply });
}

/** Preserves human submission data while batching only compatible plain-text input. */
async function accept(event: ToolPkg.ChatInputHookEvent): Promise<ToolPkg.ChatInputHookObjectResult | void> {
  const payload = event.eventPayload, chatId = payload.chatId;
  if (chatId === undefined) throw new Error("Waifu input requires a native chat identity");
  const mode = await modeForChat(chatId);
  const pending = batches.has(chatId) || deliveries.has(chatId);
  const merging = mode !== null && mode.mergeSend;
  if (!merging && !pending) return;
  if (payload.attachments === null || payload.attachments === undefined) throw new Error("Waifu input requires actual attachment records");
  if (typeof payload.text !== "string") throw new Error("Waifu input requires the original text");
  if (payload.runtime !== "main" && payload.runtime !== "floating") throw new Error("Waifu input requires an actual native runtime");
  if (!merging || payload.attachments.length > 0 || payload.replyToMessageTimestamp !== null) {
    if (!pending) return;
    flush(chatId);
    submit({ kind: "submit", chatId, runtime: payload.runtime,
      input: { text: payload.text, attachments: payload.attachments, replyToMessageTimestamp: payload.replyToMessageTimestamp },
      turn: { kind: "execute" }, notifyReply: payload.notifyReply });
    return { action: "Consume", clearInput: true, metadata: { waifuQueued: true } };
  }
  if (payload.text.trim() === "") return;
  let batch = batches.get(chatId);
  if (batch !== undefined && (batch.runtime !== payload.runtime || batch.notifyReply !== payload.notifyReply)) { flush(chatId); batch = undefined; }
  if (batch === undefined) {
    batch = { chatId, runtime: payload.runtime, notifyReply: payload.notifyReply, text: [], timer: 0 };
    batches.set(chatId, batch);
  }
  clearTimeout(batch.timer); batch.text.push(payload.text);
  batch.timer = setTimeout(() => flush(chatId), mode!.mergeDelayMs);
  return { action: "Consume", clearInput: true, metadata: { waifuMerged: true, queuedMessages: batch.text.length } };
}

/** Serializes human input edits while native API sends retain their explicit execution contract. */
export async function mergeInput(event: ToolPkg.ChatInputHookEvent): Promise<ToolPkg.ChatInputHookObjectResult | void> {
  const { chatId, source } = event.eventPayload;
  if (chatId === undefined || event.event !== "submit_requested" || source === "Sequence" || source === "Tool") return;
  const previous = operations.get(chatId);
  const operation = (async () => { if (previous !== undefined) await previous; return accept(event); })();
  operations.set(chatId, operation);
  try { return await operation; }
  finally { if (operations.get(chatId) === operation) operations.delete(chatId); }
}
