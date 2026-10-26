import assert from "node:assert/strict";
import test from "node:test";
import { clock, load, plain, provider, settle } from "./harness.mjs";

/** Models one native observation with deterministic pending pulls and explicit disposal. */
function channel() {
  const queue = []; let waiting = null, ended = false, closes = 0, pulls = 0;
  return {
    /** Implements a native stream as a single-consumer async iterator. */
    [Symbol.asyncIterator]() { return this; },
    /** Pulls the next authoritative snapshot. */
    next() { pulls++; if (queue.length) return Promise.resolve({ done: false, value: queue.shift() }); if (ended) return Promise.resolve({ done: true }); return new Promise(resolve => { waiting = resolve; }); },
    /** Detaches observation and wakes the outstanding native pull. */
    async return() { closes++; ended = true; if (waiting !== null) { waiting({ done: true }); waiting = null; } return { done: true }; },
    /** Publishes a semantic event without depending on the delivery consumer. */
    push(value) { if (waiting !== null) { const next = waiting; waiting = null; next({ done: false, value }); } else queue.push(value); },
    stats() { return { closes, pulls }; },
  };
}

/** Creates the production dependency client and real provider around deterministic Chat observation. */
async function fixture(charDelayMs = 0) {
  const time = clock(), source = channel(), p = provider({ Date: time.Date }), calls = [], cancellations = [];
  const settings = await p.call("settings.get", { profile: "default" });
  await p.call("settings.set", { profile: "default", settings: { ...settings, charDelayMs, enableEmoticons: true } });
  const Tools = { Chat: { sendMessageStreaming() { return source; }, async cancel(chatId) { cancellations.push(chatId); return { cancelRequested: true }; } } };
  const client = load("api", { ...time, Tools, ToolPkg: { async callDependency(packageName, method, payload) { assert.equal(packageName, "com.operit.waifu"); calls.push(method); return p.call(method, payload); } } });
  const request = { kind: "submit", chatId: "chat", runtime: "main", input: { text: "hello", attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute" }, notifyReply: false };
  const stream = client.stream(request);
  /** Publishes a real semantic snapshot containing text plus hidden thinking and tool content. */
  function part(text, messageTimestamp = 10) { source.push({ type: "part", messageTimestamp, parts: [{ kind: "thinking", content: "private" }, { kind: "tool_result", content: "secret" }, { kind: "markdown", content: text }] }); }
  /** Publishes only the originating turn's terminal receipt. */
  function complete(status = "completed") { source.push({ type: "completed", result: { chatId: "chat", aiResponse: "raw markup must not be sent", outcome: { type: "committed", status } } }); }
  return { time, source, p, calls, cancellations, stream, part, complete };
}

test("a sentence is yielded before model completion and final output never duplicates the receipt", async () => {
  const f = await fixture(), first = f.stream.next(); await settle();
  f.part("你好。后"); await settle();
  assert.deepEqual(plain((await first).value), { type: "segment", index: 0, segment: { kind: "text", text: "你好。" } });
  assert.equal(f.calls.at(-1), "session.feed");
  f.part("你好。后面！<emotion>happy</emotion>"); f.complete(); await settle();
  const rest = []; for await (const event of f.stream) rest.push(event);
  assert.deepEqual(rest.filter(event => event.type === "segment").map(event => event.segment.kind), ["text", "image"]);
  assert.equal(rest.at(-1).type, "completed");
  assert.equal(f.cancellations.length, 0); assert.equal(f.calls.at(-1), "session.close");
});

test("typing delay is capped at three seconds while model collection continues eagerly", async () => {
  const f = await fixture(250), first = f.stream.next(); await settle();
  f.part("第一句。后"); await settle(); await first;
  f.part("第一句。第二句话很长很长很长很长。后"); f.complete(); await settle();
  const second = f.stream.next(); let ready = false; second.then(() => { ready = true; }); await settle();
  await f.time.advance(2999); assert.equal(ready, false);
  await f.time.advance(1); assert.equal((await second).value.segment.text, "第二句话很长很长很长很长。");
  assert.ok(f.source.stats().pulls >= 3);
  await f.stream.return(); assert.equal(f.cancellations.length, 0);
});

test("consumer early exit cancels its actual Chat and releases the provider session", async () => {
  const f = await fixture(), first = f.stream.next(); await settle();
  f.part("第一句。后"); await settle(); await first;
  await f.stream.return();
  assert.deepEqual(f.cancellations, ["chat"]); assert.equal(f.calls.at(-1), "session.close"); assert.equal(f.source.stats().closes, 1);
});

test("failed native receipts reject instead of fabricating a final response", async () => {
  const f = await fixture(), next = f.stream.next(); await settle(); f.complete("cancelled");
  await assert.rejects(next, /did not complete/); assert.equal(f.calls.at(-1), "session.close");
});

test("a provider failure cancels still-running generation and propagates the real error", async () => {
  const f = await fixture(), first = f.stream.next(); await settle(); f.part("先发送。后"); await settle(); await first;
  const next = f.stream.next(); f.part("被更改。后");
  await assert.rejects(next, /already delivered/); assert.deepEqual(f.cancellations, ["chat"]);
});
