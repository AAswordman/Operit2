import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

test("native folder deletion retains partial failures without a plugin metadata transaction", async () => {
  const api = loadModule("src/sidebar-group-delete.ts"), calls = [];
  const result = await api.deleteConversationGroupMembers(["first","locked"], async chatId => {
    calls.push(chatId); if (chatId === "locked") throw new Error("Native lock refusal");
    return {chatId,deletedAt:1};
  });
  assert.deepEqual(calls,["first","locked"]);
  assert.deepEqual(plain(result),{deletedChatIds:["first"],failedChatIds:[{chatId:"locked",error:"Error: Native lock refusal"}]});
});
test("native folder deletion validates acknowledgements and does not retry", async () => {
  const api = loadModule("src/sidebar-group-delete.ts"), calls = [];
  const result = await api.deleteConversationGroupMembers(["first","second"], async chatId => {
    calls.push(chatId); return {chatId: chatId === "first" ? "wrong" : chatId,deletedAt:1};
  });
  assert.deepEqual(calls,["first","second"]);
  assert.deepEqual(Array.from(result.deletedChatIds),["second"]);
  assert.equal(result.failedChatIds[0].chatId,"first");
});
