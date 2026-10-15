import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

const codec = loadModule("src/group-execution/contracts.ts");
const chatId = "real-request-chat", userTimestamp = 1700000000111;
const input = { kind: "execute", submission: { chatId, text: "Original user text" }, turn: { requestKey: "plugin-local-only", configuration: {} } };

/** Constructs the actual Rust send-result wire shape, with no invented native execution token or locator from receivedAt. */
function native(outcome) { return { chatId, message: "Original user text", sentAt: 1700000000000, aiResponse: "Text is not an identity", receivedAt: 1700000000999, outcome }; }

/** Actual commit locators are preserved; local request correlation is supplied by the awaiting call, never accepted as native authority. */
test("receipt: maps actual committed outcome and retains nullable cancelled user/assistant fields without inventing timestamps", () => {
  const actual = native({ type: "committed", status: "completed", userMessageTimestamp: userTimestamp, assistant: { messageTimestamp: 1700000000222, variantIndex: 9 } });
  assert.deepEqual(plain(codec.decodeNativeGroupSendResult(actual, input, null)), { type: "committed", receipt: { status: "completed", chatId, requestKey: "plugin-local-only", userMessageTimestamp: userTimestamp,
    assistant: { messageTimestamp: 1700000000222, variantIndex: 9 } } });
  assert.equal(Object.hasOwn(actual, "requestKey"), false); assert.equal(Object.hasOwn(actual, "executionId"), false);
  assert.deepEqual(plain(codec.decodeNativeGroupSendResult(native({ type: "committed", status: "cancelled", userMessageTimestamp: null, assistant: null }), input, null)),
    { type: "committed", receipt: { status: "cancelled", chatId, requestKey: "plugin-local-only", userMessageTimestamp: null, assistant: null } });
});

/** Hook ownership and nonpersistent completion are distinct native states, not committed or completed group execution. */
test("receipt: blocked, consumed and not_persisted preserve their exact distinct native outcomes", () => {
  for (const outcome of [{ type: "blocked", message: null }, { type: "blocked", message: "Explicit refusal" }, { type: "consumed", metadata: { owner: "other-plugin", sequence: { token: "opaque-native-input" } } },
    { type: "not_persisted", status: "completed" }, { type: "not_persisted", status: "cancelled" }]) {
    assert.deepEqual(plain(codec.decodeNativeGroupSendResult(native(outcome), input, null)), outcome);
  }
});

/** The original timestamp is mandatory for a completed group commit and a continuation cannot acknowledge a different user row. */
test("receipt: malformed or unrelated receipts are rejected with no latest-message, sentAt or receivedAt inference", () => {
  const completed = { type: "committed", status: "completed", userMessageTimestamp: userTimestamp, assistant: { messageTimestamp: 1700000000222, variantIndex: 9 } };
  const invalids = [
    { ...native(completed), chatId: "another-chat" },
    { chatId, message: "Looks successful", sentAt: 1700000000000, receivedAt: 1700000000999 },
    native({ ...completed, assistant: null }), native({ ...completed, userMessageTimestamp: null }),
    native({ ...completed, assistant: { messageTimestamp: 1700000000222 } }), native({ ...completed, userMessageTimestamp: Number.MAX_SAFE_INTEGER + 1 }),
    native({ ...completed, assistant: { messageTimestamp: 1700000000222, variantIndex: 2147483648 } }),
    native({ type: "committed", status: "cancelled", assistant: null }), native({ type: "not_persisted", status: "done" }),
    native({ type: "blocked" }), native({ type: "consumed", metadata: null }), native({ type: "invented_completed" }),
  ];
  for (const value of invalids) assert.throws(() => codec.decodeNativeGroupSendResult(value, input, null));
  const continuation = { chatId, userMessageTimestamp: userTimestamp, turn: input.turn };
  assert.throws(() => codec.decodeNativeGroupSendResult(native({ ...completed, userMessageTimestamp: userTimestamp + 1 }), continuation, userTimestamp), /different user/);
  const recordOnly = { kind: "record_only", submission: input.submission, requestKey: "record-only-local-key" };
  assert.throws(() => codec.decodeNativeGroupSendResult(native(completed), recordOnly, null), /User-only/);
  assert.deepEqual(plain(codec.decodeNativeGroupSendResult(native({ ...completed, assistant: null }), recordOnly, null)), { type: "committed", receipt: { status: "completed", chatId,
    requestKey: "record-only-local-key", userMessageTimestamp: userTimestamp, assistant: null } });
});
