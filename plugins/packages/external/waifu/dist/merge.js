"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.mergeInput = mergeInput;
const mode_js_1 = require("./mode.js");
const MAX_BATCH_WAIT_MS = 15000;
const MAX_BATCH_MESSAGES = 20;
const MAX_BATCH_CHARS = 16000;
const batches = new Map();
const operations = new Map();
const deliveries = new Map();
/** Executes accepted requests in chat order outside the synchronous input-hook deadline. */
async function deliver(request) {
    const previous = deliveries.get(request.chatId);
    const operation = (async () => {
        // A failed earlier send must not discard a later accepted user message.
        if (previous !== undefined) {
            try {
                await previous;
            }
            catch { /* prior failure was already reported */ }
        }
        const receipt = await Tools.Chat.sendMessage(request);
        if (receipt.outcome.type !== "committed" || receipt.outcome.status !== "completed")
            throw new Error(`Waifu queued send did not complete: ${JSON.stringify(receipt.outcome)}`);
    })();
    deliveries.set(request.chatId, operation);
    try {
        await operation;
    }
    finally {
        if (deliveries.get(request.chatId) === operation)
            deliveries.delete(request.chatId);
    }
}
/** Reports the actual native error for one accepted delivery without retrying model execution. */
function submit(request) {
    void deliver(request).catch(error => Tools.System.toast(`Waifu 合并发送失败：${error instanceof Error ? error.message : String(error)}`));
}
/** Removes one timer and transfers the exact accepted text batch to native delivery. */
function flush(chatId) {
    const batch = batches.get(chatId);
    if (batch === undefined)
        return;
    batches.delete(chatId);
    clearTimeout(batch.timer);
    clearTimeout(batch.deadline);
    submit({ kind: "submit", chatId: batch.chatId, runtime: batch.runtime,
        input: { text: batch.text.join("\n"), attachments: [], replyToMessageTimestamp: null },
        turn: { kind: "execute" }, notifyReply: batch.notifyReply });
}
/** Preserves human submission data while batching only compatible plain-text input. */
async function accept(event) {
    const payload = event.eventPayload, chatId = payload.chatId;
    if (chatId === undefined)
        throw new Error("Waifu input requires a native chat identity");
    const mode = await (0, mode_js_1.modeForChat)(chatId);
    const pending = batches.has(chatId) || deliveries.has(chatId);
    const merging = mode !== null && mode.mergeSend;
    if (!merging && !pending)
        return;
    if (payload.attachments === null || payload.attachments === undefined)
        throw new Error("Waifu input requires actual attachment records");
    if (typeof payload.text !== "string")
        throw new Error("Waifu input requires the original text");
    if (payload.runtime !== "main" && payload.runtime !== "floating")
        throw new Error("Waifu input requires an actual native runtime");
    if (!merging || payload.attachments.length > 0 || payload.replyToMessageTimestamp !== null) {
        if (!pending)
            return;
        flush(chatId);
        submit({ kind: "submit", chatId, runtime: payload.runtime,
            input: { text: payload.text, attachments: payload.attachments, replyToMessageTimestamp: payload.replyToMessageTimestamp },
            turn: { kind: "execute" }, notifyReply: payload.notifyReply });
        return { action: "Consume", clearInput: true, metadata: { waifuQueued: true } };
    }
    if (payload.text.trim() === "")
        return;
    let batch = batches.get(chatId);
    if (batch !== undefined && (batch.runtime !== payload.runtime || batch.notifyReply !== payload.notifyReply)) {
        flush(chatId);
        batch = undefined;
    }
    if (batch === undefined) {
        batch = { chatId, runtime: payload.runtime, notifyReply: payload.notifyReply, text: [], timer: 0, deadline: 0, size: 0 };
        batches.set(chatId, batch);
        // A continuous stream of short messages must not postpone the first reply forever.
        batch.deadline = setTimeout(() => flush(chatId), MAX_BATCH_WAIT_MS);
    }
    clearTimeout(batch.timer);
    batch.text.push(payload.text);
    batch.size += payload.text.length;
    const queuedMessages = batch.text.length;
    if (queuedMessages >= MAX_BATCH_MESSAGES || batch.size >= MAX_BATCH_CHARS)
        flush(chatId);
    else
        batch.timer = setTimeout(() => flush(chatId), mode.mergeDelayMs);
    return { action: "Consume", clearInput: true, metadata: { waifuMerged: true, queuedMessages } };
}
/** Serializes human input edits while native API sends retain their explicit execution contract. */
async function mergeInput(event) {
    const { chatId, source, isProcessing } = event.eventPayload;
    // The host owns busy-chat queue, guide and interrupt actions. Consuming here would
    // convert a user's interruption into a later ordinary turn and hide their intent.
    if (chatId === undefined || event.event !== "submit_requested" || source === "Sequence" || source === "Tool" || isProcessing === true)
        return;
    const previous = operations.get(chatId);
    const operation = (async () => { if (previous !== undefined)
        await previous; return accept(event); })();
    operations.set(chatId, operation);
    try {
        return await operation;
    }
    finally {
        if (operations.get(chatId) === operation)
            operations.delete(chatId);
    }
}
