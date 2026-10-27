import { installScopedHostRuntime, installGeneratedToolsRuntime, domainToolResult } from "../../../../../tools/tests/fixtures/scoped_host_runtime.mjs";
import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";

const submitted = { kind: "submit", chatId: "chat-a", runtime: "main", input: { text: "Original", attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute", participantId: "participant-a" }, notifyReply: false };
const receipt = { chatId: "chat-a", message: "Original", aiResponse: "Visible text", receivedAt: 30, sentAt: 10,
  outcome: { type: "committed", status: "completed", userMessageTimestamp: 11, assistant: { messageTimestamp: 22, variantIndex: 0 } } };

/** Copies values out of the isolated production runtime without changing their wire fields. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Loads the actual SDK runtime with an explicitly controlled native ABI, not another stream implementation. */
function runtime(native) {
  const calls = [], context = vm.createContext({ console });
  const registry = installScopedHostRuntime(context);
  context.__operitNativeCallToolStructured = registry.binding(
    /** Submits host-owned structured values and settles the exact production Promise. */
    function(id, type, name, parameters) {
      assert.equal(type, 'default');
      const method = name === 'chat_observe' ? parameters.observation.op : name === 'chat_send_message' ? 'send' : name === 'chat_cancel' ? 'cancel' : name;
      const payload = name === 'chat_observe' ? parameters.observation.op === 'open' ? parameters.observation.request : parameters.observation.handle : name === 'chat_send_message' ? parameters.request : parameters.chatId;
      const value = structuredClone(payload); calls.push({ method, value });
      native(method, value,
        /** Delivers the planned host result without JSON text or public callback slots. */
        result => registry.settle(id, registry.snapshotResult({
          success: true, data: domainToolResult(name, result, parameters),
        }), false),
        /** Preserves the planned host rejection through the same scoped channel. */
        message => registry.settle(id, message, true));
    });
  installGeneratedToolsRuntime(context);
  return { context, calls,
    /** Evaluates caller code in the same JavaScript realm as the strict production JSON validator. */
    run(code) { return vm.runInContext(code, context); },
    /** Creates the complete original request in the actual caller realm. */
    request() { return vm.runInContext("(" + JSON.stringify(submitted) + ")", context); },
  };
}

/** Requires every host operation to release its owning call reference. */
function assertCallReleased(context) { assert.equal(context.refs, 0); }

/** The non-streaming call returns the actual receipt and does not split admission/wait/finalization into author calls. */
test("send: exactly one current native send returns its real terminal receipt", async () => {
  const env = runtime(
    /** Supplies only the declared single-send capability for this scenario. */
    (method, request, resolve) => { assert.equal(method, "send"); assert.deepEqual(request, submitted); resolve(receipt); },
  );
  assert.deepEqual(plain(await env.context.Tools.Chat.sendMessage(env.request())), receipt);
  assert.deepEqual(env.calls.map(call => call.method), ["send"]); assertCallReleased(env.context);
});

/** A streamed replacement carries already parsed thinking/tool/status blocks and the final real receipt on one channel. */
test("stream: ordered native pulls deliver atomic semantic replacements and one completion", async () => {
  const parts = [
    { partId: "thinking", sequence: 0, kind: "thinking", content: "Reasoning", toolCallId: null, toolName: null, attributes: {} },
    { partId: "tool", sequence: 1, kind: "tool_call", content: "", toolCallId: "call-1", toolName: "read", attributes: { path: "exact" } },
    { partId: "removed", sequence: 2, kind: "markdown", content: "Uncommitted text", toolCallId: null, toolName: null, attributes: {} },
  ];
  const expected = [
    { type: "part", chatId: "chat-a", messageTimestamp: 22, revision: 1, parts },
    { type: "part", chatId: "chat-a", messageTimestamp: 22, revision: 2, parts: parts.slice(0, 2) },
    { type: "completed", result: receipt },
  ];
  let next = 0;
  const env = runtime(
    /** Delivers only explicitly declared pulls; unexpected methods or extra pulls are failures. */
    (method, value, resolve) => {
      switch (method) {
        case "open": assert.deepEqual(value, submitted); resolve("private-observation"); break;
        case "next": assert.equal(value, "private-observation"); assert.ok(next < expected.length); resolve(expected[next++]); break;
        case "close": assert.equal(value, "private-observation"); resolve(null); break;
        default: assert.fail("Unexpected native method: " + method);
      }
    },
  );
  const stream = env.context.Tools.Chat.sendMessageStreaming(env.request());
  assert.equal(typeof stream.then, "undefined"); assert.equal(env.calls.length, 0);
  const actual = [];
  for await (const event of stream) actual.push(plain(event));
  assert.deepEqual(actual, expected); assert.equal(next, 3);
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "next", "next", "close"]); assertCallReleased(env.context);
});

/** Breaking a loop disposes the native observation but cannot cancel an accepted model request. */
test("stream: break disposes once without issuing cancel", async () => {
  const env = runtime(
    /** Exposes only admission, one semantic pull and disposal. */
    (method, _value, resolve) => {
      if (method === "open") resolve("observation");
      else if (method === "next") resolve({ type: "part", chatId: "chat-a", messageTimestamp: 22, revision: 1, parts: [] });
      else { assert.equal(method, "close"); resolve(null); }
    },
  );
  const stream = env.context.Tools.Chat.sendMessageStreaming(env.request());
  for await (const _event of stream) break;
  await stream.return();
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "close"]); assertCallReleased(env.context);
});

/** Disposal must wake an outstanding native pull instead of waiting indefinitely for another provider chunk. */
test("stream: return closes a pending pull and concurrent next is rejected", async () => {
  let pending = null, signal;
  const started = new Promise(
    /** Exposes only the deterministic native next rendezvous. */
    resolve => { signal = resolve; },
  );
  const env = runtime(
    /** Holds the actual native next completion until its explicit close operation. */
    (method, _value, resolve) => {
      if (method === "open") resolve("observation");
      else if (method === "next") { pending = resolve; signal(); }
      else { assert.equal(method, "close"); assert.notEqual(pending, null); pending(null); resolve(null); }
    },
  );
  const stream = env.context.Tools.Chat.sendMessageStreaming(env.request()), next = stream.next();
  await started;
  await assert.rejects(stream.next(), /only one pending next/);
  assert.equal((await stream.return()).done, true); assert.equal((await next).done, true);
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "close"]); assertCallReleased(env.context);
});

/** A failed observation retains its original error and removes all its host Promise and stream resources. */
test("stream: original native errors propagate and cleanup failures retain both causes", async () => {
  const env = runtime(
    /** Plans two independent original errors rather than creating a successful completion. */
    (method, _value, resolve, reject) => {
      if (method === "open") resolve("observation");
      else if (method === "next") reject(" original generation error ");
      else { assert.equal(method, "close"); reject(" original disposal error "); }
    },
  );
  await assert.rejects(env.context.Tools.Chat.sendMessageStreaming(env.request()).next(),
    /** Verifies both errors without manufacturing a receipt or retrying. */
    error => error.cause.message === " original generation error " && error.cleanupError.message === " original disposal error ");
  assertCallReleased(env.context);
});

/** The request is captured once, so later caller mutation cannot change the accepted send. */
test("stream: snapshots the request and closing an unopened iterator does not send", async () => {
  const env = runtime(
    /** Accepts only the captured original request followed by explicit disposal. */
    (method, value, resolve) => {
      if (method === "open") { assert.deepEqual(value, submitted); resolve("observation"); }
      else if (method === "next") resolve({ type: "completed", result: receipt });
      else { assert.equal(method, "close"); resolve(null); }
    },
  );
  const request = env.request(), stream = env.context.Tools.Chat.sendMessageStreaming(request); request.input.text = "Changed";
  assert.equal((await stream.next()).value.type, "completed");
  const unopened = env.context.Tools.Chat.sendMessageStreaming(env.request()); await unopened.return();
  assert.equal((await unopened.next()).done, true);
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "close"]); assertCallReleased(env.context);
});

/** Invalid argument counts and non-JSON objects are rejected before scheduling any native operation. */
test("send: rejects malformed caller data without a native call", async () => {
  const env = runtime(
    /** Any native invocation is forbidden in this validation-only scenario. */
    () => assert.fail("Invalid data reached native Chat"),
  );
  await assert.rejects(env.run("Tools.Chat.sendMessage()"), /exactly (?:one request|1 arguments)/);
  await assert.rejects(env.run("Tools.Chat.sendMessage({ value: () => 1 })"), /non-JSON/);
  assert.throws(() => env.run("Tools.Chat.sendMessageStreaming({ value: NaN })"), /non-JSON/);
  await assert.rejects(env.run("Tools.Chat.cancel('')"), /Identifier must be exact nonblank text/);
  assert.equal(env.calls.length, 0); assertCallReleased(env.context);
});

/** Chat cancellation uses only the public chat identity; the native host captures execution ownership internally. */
test("cancel: returns the exact native owner/chat cancellation acknowledgement", async () => {
  const actual = { chatId: "chat-a", cancelRequested: true };
  const env = runtime(
    /** Provides only cancellation for the declared chat. */
    (method, value, resolve) => { assert.equal(method, "cancel"); assert.equal(value, "chat-a"); resolve(actual); },
  );
  assert.deepEqual(plain(await env.context.Tools.Chat.cancel("chat-a")), actual);
  assert.deepEqual(env.calls, [{ method: "cancel", value: "chat-a" }]); assertCallReleased(env.context);
});

console.log("SCOPE: actual JavaScript iterator execution with a controlled native ABI; no Rust compilation or live AI generation");
