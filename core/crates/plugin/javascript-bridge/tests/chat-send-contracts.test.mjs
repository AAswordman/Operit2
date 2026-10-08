import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import vm from "node:vm";

const root = new URL("../../../../../", import.meta.url);
/** Reads current production code, not an emitted runtime artifact. */
function source(path) { return readFileSync(new URL(path, root), "utf8"); }
const generator = source("core/crates/plugin/codegen/src/runtime_bindings.rs");
const validatorStart = generator.indexOf("function __operitRequireChatJsonObject(value) {");
const validatorEnd = generator.indexOf("/** Invokes only the narrow typed native record binding", validatorStart);
assert.ok(validatorStart >= 0 && validatorEnd > validatorStart);
const runtimeScript = generator.slice(validatorStart, validatorEnd) + source("core/crates/plugin/sdk/src/chat_runtime.js");
const submitted = { kind: "submit", chatId: "chat-a", runtime: "main", input: { text: "Original", attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute", participantId: "participant-a" }, notifyReply: false };
const receipt = { chatId: "chat-a", message: "Original", aiResponse: "Visible text", receivedAt: 30, sentAt: 10,
  outcome: { type: "committed", status: "completed", userMessageTimestamp: 11, assistant: { messageTimestamp: 22, variantIndex: 0 } } };

/** Copies values out of the isolated production runtime without changing their wire fields. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Loads the actual SDK runtime with an explicitly controlled native ABI, not another stream implementation. */
function runtime(native) {
  const calls = [], context = vm.createContext({ console });
  context.__operitNativeChatAsync =
    /** Audits exactly one current native invocation and exposes its real private response callback. */
    function(id, method, payload) {
      const value = JSON.parse(payload); calls.push({ method, value });
      native(method, value,
        /** Delivers the explicitly planned native value through the actual production callback slot. */
        result => context[id](JSON.stringify(result), false),
        /** Preserves the explicitly planned native rejection message. */
        message => context[id](JSON.stringify({ message }), true));
    };
  vm.runInContext(runtimeScript, context);
  return { context, calls,
    /** Evaluates caller code in the same JavaScript realm as the strict production JSON validator. */
    run(code) { return vm.runInContext(code, context); },
    /** Creates the complete original request in the actual caller realm. */
    request() { return vm.runInContext("(" + JSON.stringify(submitted) + ")", context); },
  };
}

/** Requires callback cleanup after every actual transport response. */
function noCallbacks(context) { assert.deepEqual(Object.keys(context).filter(key => /^__operit_chat_send_\d+$/.test(key)), []); }

/** The non-streaming call returns the actual receipt and does not split admission/wait/finalization into author calls. */
test("send: exactly one current native send returns its real terminal receipt", async () => {
  const env = runtime(
    /** Supplies only the declared single-send capability for this scenario. */
    (method, request, resolve) => { assert.equal(method, "send"); assert.deepEqual(request, submitted); resolve(receipt); },
  );
  assert.deepEqual(plain(await env.context.__operitChatSend(env.request())), receipt);
  assert.deepEqual(env.calls.map(call => call.method), ["send"]); noCallbacks(env.context);
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
  const stream = env.context.__operitChatStream(env.request());
  assert.equal(typeof stream.then, "undefined"); assert.equal(env.calls.length, 0);
  const actual = [];
  for await (const event of stream) actual.push(plain(event));
  assert.deepEqual(actual, expected); assert.equal(next, 3);
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "next", "next", "close"]); noCallbacks(env.context);
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
  const stream = env.context.__operitChatStream(env.request());
  for await (const _event of stream) break;
  await stream.return();
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "close"]); noCallbacks(env.context);
});

/** Disposal must wake an outstanding native pull instead of waiting indefinitely for another provider chunk. */
test("stream: return closes a pending pull and concurrent next is rejected", async () => {
  let pending = null, signal;
  const started = new Promise(
    /** Exposes only the deterministic native next rendezvous. */
    resolve => { signal = resolve; },
  );
  const env = runtime(
    /** Holds the actual native next callback until its explicit close operation. */
    (method, _value, resolve) => {
      if (method === "open") resolve("observation");
      else if (method === "next") { pending = resolve; signal(); }
      else { assert.equal(method, "close"); assert.notEqual(pending, null); pending(null); resolve(null); }
    },
  );
  const stream = env.context.__operitChatStream(env.request()), next = stream.next();
  await started;
  await assert.rejects(stream.next(), /only one pending next/);
  assert.equal((await stream.return()).done, true); assert.equal((await next).done, true);
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "close"]); noCallbacks(env.context);
});

/** A failed observation retains its original error and removes all its private callback and stream resources. */
test("stream: original native errors propagate and cleanup failures retain both causes", async () => {
  const env = runtime(
    /** Plans two independent original errors rather than creating a successful completion. */
    (method, _value, resolve, reject) => {
      if (method === "open") resolve("observation");
      else if (method === "next") reject(" original generation error ");
      else { assert.equal(method, "close"); reject(" original disposal error "); }
    },
  );
  await assert.rejects(env.context.__operitChatStream(env.request()).next(),
    /** Verifies both errors without manufacturing a receipt or retrying. */
    error => error.cause.message === " original generation error " && error.cleanupError.message === " original disposal error ");
  noCallbacks(env.context);
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
  const request = env.request(), stream = env.context.__operitChatStream(request); request.input.text = "Changed";
  assert.equal((await stream.next()).value.type, "completed");
  const unopened = env.context.__operitChatStream(env.request()); await unopened.return();
  assert.equal((await unopened.next()).done, true);
  assert.deepEqual(env.calls.map(call => call.method), ["open", "next", "close"]); noCallbacks(env.context);
});

/** Invalid argument counts and non-JSON objects are rejected before scheduling any native operation. */
test("send: rejects malformed caller data without a native call", async () => {
  const env = runtime(
    /** Any native invocation is forbidden in this validation-only scenario. */
    () => assert.fail("Invalid data reached native Chat"),
  );
  await assert.rejects(env.run("__operitChatSend()"), /exactly one request/);
  await assert.rejects(env.run("__operitChatSend({ value: () => 1 })"), /non-JSON/);
  assert.throws(() => env.run("__operitChatStream({ value: NaN })"), /non-JSON/);
  await assert.rejects(env.run("__operitChatCancel('')"), /nonempty chatId/);
  assert.equal(env.calls.length, 0); noCallbacks(env.context);
});

/** Chat cancellation uses only the public chat identity; the native host captures execution ownership internally. */
test("cancel: returns the exact native owner/chat cancellation acknowledgement", async () => {
  const actual = { chatId: "chat-a", cancelRequested: true };
  const env = runtime(
    /** Provides only cancellation for the declared chat. */
    (method, value, resolve) => { assert.equal(method, "cancel"); assert.equal(value, "chat-a"); resolve(actual); },
  );
  assert.deepEqual(plain(await env.context.__operitChatCancel("chat-a")), actual);
  assert.deepEqual(env.calls, [{ method: "cancel", value: "chat-a" }]); noCallbacks(env.context);
});

/** Audits the mounted native/SDK path without claiming this source assertion is a Rust execution test. */
test("static: only three send controls are public and native snapshots use the existing parser", () => {
  const sdk = source("core/crates/plugin/sdk/src/js_sdk/chat.rs");
  const transport = source("core/crates/runtime/application/src/services/core/ChatTurnTransport.rs");
  const runtime = source("core/crates/runtime/application/src/services/ToolRuntimeSupportService.rs");
  const parser = source("core/crates/runtime/application/src/services/core/MessageProcessingDelegate.rs");
  const observation = source("core/crates/runtime/application/src/services/core/ChatSendStream.rs");
  const engine = source("core/crates/plugin/javascript-bridge/src/javascript/JsEngine.rs");
  assert.match(sdk, /fn sendMessage\(&self, request: ChatSendRequest\) -> JsFuture<MessageSendResultData>/);
  assert.match(sdk, /fn sendMessageStreaming\(&self, request: ChatSendRequest\) -> JsAsyncIterable<ChatSendEvent>/);
  assert.match(sdk, /fn cancel\(&self, chatId: String\)/);
  assert.doesNotMatch(sdk, /fn (startTurn|waitTurn|cancelTurn|finishSequence)\(|onIntermediateResult|ChatSendMessageStreamingOptions/);
  assert.match(transport, /sequence\.owner == owner && sequence\.chatId == chatId/);
  assert.match(runtime, /ownedActiveExecution\(owner, chatId\)/);
  assert.match(runtime, /cancelNativeChatExecution\(&capturedChatId, &executionId\)/);
  assert.match(parser, /parser\.resetToSnapshot\(&snapshot\)/);
  assert.match(parser, /parser\.pushSnapshot\(&snapshot\)/);
  assert.match(parser, /observer\.snapshot\(workerAiMessage\.timestamp, &parts\)/);
  assert.match(observation, /MAX_CHAT_SNAPSHOT_BYTES: usize = 8 \* 1024 \* 1024/);
  assert.match(observation, /watch::channel\(ObservationState::default\(\)\)/);
  assert.doesNotMatch(observation, /unbounded|VecDeque|cancelNativeChatExecution/);
  assert.match(engine, /"__operitNativeChatAsync"/);
  assert.match(engine, /let cancel = host\.cancel\(chatId\)/);
});

console.log("SCOPE: actual JavaScript iterator execution with a controlled native ABI plus source checks; no Rust compilation or live AI generation");
