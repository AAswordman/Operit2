import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";
import { createDiskHarness } from "./disk-files.mjs";

const submitted = { submissionId: "submission", chatId: "chat", selection: "group:33", runtime: "main", notifyReply: true,
  text: "Original full input", attachments: [{ filePath: "vfs://a", nodeId: "node", fileName: "a.txt", mimeType: "text/plain", fileSize: 7, content: "payload" }], replyToMessageTimestamp: 9 };

/** Supplies only the author Core attaches to a committed message. */
function marker(id) { return { participantId: id }; }

/** Supplies one explicit controlled native receipt with no timestamp or latest-history heuristic. */
function receipt(message, timestamp, status = "completed", user = 10) {
  return { chatId: "chat", message, aiResponse: null, receivedAt: null, sentAt: 1,
    outcome: { type: "committed", status, userMessageTimestamp: user, assistant: timestamp === null ? null : { messageTimestamp: timestamp, variantIndex: 0 } } };
}

/** Loads the real transport with only the declared current Chat capabilities. */
function transport(chat, globals = {}) {
  const namespace = new Proxy(chat, {
    /** Rejects every retired method or undeclared host capability immediately. */
    get(target, name) { assert.equal(Object.hasOwn(target, name), true, "Undeclared native capability: " + String(name)); return target[name]; },
  });
  const module = loadModule("src/group-execution/native-transport.ts", { ...globals, Tools: { ...globals.Tools, Chat: namespace } });
  return new module.NativeGroupTurnTransport();
}

/** The real plugin transport submits once, continues the precise user row and never asks for native handles or finish methods. */
test("native send transport: complete input, exact continuations and host-authored identities", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const calls = [], snapshots = new Map(); let count = 0;
  const native = transport({
    /** Acknowledges the explicitly planned send and stores its controlled author identity. */
    async sendMessage(request) {
      const input = plain(request); calls.push(input); count++;
      if (input.kind === "submit") return receipt(input.input.text, null);
      snapshots.set(20 + count, marker(input.participantId));
      return receipt("", 20 + count);
    },
    /** Reads only the exact committed message locator, never current character state. */
    async readExtension(target) { assert.deepEqual(Object.keys(target).sort(), ["chatId", "kind", "messageTimestamp", "variantIndex"]); assert.equal(target.kind, "message"); assert.equal(target.chatId, "chat"); assert.equal(target.variantIndex, 0); assert.equal(snapshots.has(target.messageTimestamp), true); return { extension: snapshots.get(target.messageTimestamp) }; },
  }, disk.globals);
  const initial = await native.submit({ kind: "record_only", submission: submitted, requestKey: "local-input" });
  const first = await native.continue({ submissionId: "submission", chatId: "chat", runtime: "main", userMessageTimestamp: 10,
    turn: { requestKey: "local-1", selection: "group:33", participantId: "1", notifyReply: false, summaryPolicy: "defer" } });
  const continued = await native.continue({ submissionId: "submission", chatId: "chat", runtime: "main", userMessageTimestamp: 10,
    turn: { requestKey: "local-2", selection: "group:33", participantId: "2", notifyReply: true, summaryPolicy: "after_reply" } });
  assert.deepEqual(calls[0], { kind: "submit", chatId: "chat", runtime: "main", notifyReply: false,
    input: { text: submitted.text, attachments: submitted.attachments, replyToMessageTimestamp: 9 }, turn: { kind: "record_only" } });
  assert.deepEqual(calls[1], { kind: "continue", chatId: "chat", runtime: "main", userMessageTimestamp: 10, participantId: "1", notifyReply: false, summaryPolicy: "defer" });
  assert.deepEqual(calls[2], { kind: "continue", chatId: "chat", runtime: "main", userMessageTimestamp: 10, participantId: "2", notifyReply: true, summaryPolicy: "after_reply" });
  await native.finish({ submissionId: "submission", chatId: "chat", runtime: "main", userMessageTimestamp: 10,
    receipts: [{ status: "completed", chatId: "chat", requestKey: "local-input", userMessageTimestamp: 10, assistant: initial.outcome.assistant },
      { status: "completed", chatId: "chat", requestKey: "local-1", userMessageTimestamp: 10, assistant: first.outcome.assistant },
      { status: "completed", chatId: "chat", requestKey: "local-2", userMessageTimestamp: 10, assistant: continued.outcome.assistant }] });
  assert.equal(calls.length, 3);
  await assert.rejects(native.cancel({ chatId: "chat", requestKey: "local-2", runtime: "main" }), /matching active send/);
});

/** Record-only input must not obtain a participant or request any model/profile capability. */
test("native send transport: user-only persistence sends no arbitrary participant", async () => {
  const calls = [], native = transport({
    /** Acknowledges only the genuine controlled user-only record. */
    async sendMessage(request) { calls.push(plain(request)); return receipt(request.input.text, null); },
  });
  await native.submit({ kind: "record_only", submission: submitted, requestKey: "record-only" });
  assert.deepEqual(calls[0].turn, { kind: "record_only" }); assert.equal(calls[0].notifyReply, false);
  await native.finish({ submissionId: "submission", chatId: "chat", runtime: "main", userMessageTimestamp: 10,
    receipts: [{ status: "completed", chatId: "chat", requestKey: "record-only", userMessageTimestamp: 10, assistant: null }] });
  assert.equal(calls.length, 1);
});

/** Cancellation invokes only the public owner/chat operation while the real send Promise is still pending. */
test("native send transport: cancellation correlates a live attempt but exposes no executionId", async () => {
  let settle; const calls = [];
  const native = transport({
    /** Holds the explicit native send completion so the test can request cancellation during its real await. */
    sendMessage(request) {
      calls.push({ method: "send", input: plain(request) });
      return new Promise(
        /** Exposes only settlement of this controlled native send. */
        resolve => { settle = resolve; },
      );
    },
    /** Reports the real owner/chat acknowledgement and settles the separately observed originating receipt. */
    async cancel(chatId) { calls.push({ method: "cancel", chatId }); settle(receipt(submitted.text, null, "cancelled", null)); return { chatId, cancelRequested: true }; },
  });
  const sending = native.submit({ kind: "record_only", submission: submitted, requestKey: "pending" });
  await native.cancel({ chatId: "chat", requestKey: "pending", runtime: "main" });
  assert.equal((await sending).outcome.status, "cancelled");
  assert.deepEqual(calls.map(call => call.method), ["send", "cancel"]);
  await assert.rejects(native.cancel({ chatId: "chat", requestKey: "pending", runtime: "main" }), /matching active send/);
});

console.log("SCOPE: real plugin transport execution with controlled native receipts/snapshots; no live model, Core lease or cross-platform integration acceptance");
