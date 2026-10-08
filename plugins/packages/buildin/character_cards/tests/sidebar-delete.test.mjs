import assert from "node:assert/strict";
import test from "node:test";
import { loadModule, plain } from "./runtime.mjs";

/** Supplies explicit metadata for operation-order unit checks only; it is not a production group repository. */
function group(chatIds = ["chat-first", "chat-second"]) { return { id: "1", ownerSelection: "card:actual-scope", name: "Explicit deletion unit input", chatIds, displayOrder: 0, pinned: false, createdAt: 1, updatedAt: 1 }; }

/** Verifies an explicit native rejection is retained and successful members are not retried or rolled back. */
test("group deletion records partial native outcomes and never deletes metadata on any member failure", async () => {
  const api = loadModule("web/features/sidebar/groups/delete.ts"), calls = [], original = new Error("Native locked conversation");
  const result = await api.deleteConversationGroupMembers(group(), {
    /** Injects the labelled unit-boundary outcome for each exactly ordered native call. */
    async deleteChat(chatId) { calls.push(chatId); if (chatId === "chat-second") throw original; return { chatId, deletedAt: 1 }; },
    /** Rejects any attempt to inspect or mutate metadata after a known failed member call. */
    async readGroup() { throw new Error("Must not inspect metadata as if native deletions all succeeded"); },
    /** Rejects a false claim that a partially failed group can be deleted successfully. */
    async deleteMetadata() { throw new Error("Must not delete metadata after a native member failure"); },
  });
  assert.deepEqual(calls, ["chat-first", "chat-second"]);
  assert.deepEqual(plain(result), { deletedChatIds: ["chat-first"], failedChatIds: [{ chatId: "chat-second", error: String(original) }], metadataDeleted: false, metadataError: null });
});
/** Requires completion of each exact native deletion and the real empty membership state before metadata deletion. */
test("group deletion checks empty persisted membership only after all member calls complete", async () => {
  const api = loadModule("web/features/sidebar/groups/delete.ts"), calls = [];
  const result = await api.deleteConversationGroupMembers(group(), {
    /** Supplies the labelled successful native boundary result after its exact unit call. */
    async deleteChat(chatId) { calls.push("native:" + chatId); return { chatId, deletedAt: 1 }; },
    /** Supplies explicitly empty metadata only after both actual algorithm calls have occurred. */
    async readGroup(id) { calls.push("read:" + id); return group([]); },
    /** Records the metadata deletion as its own non-atomic unit-boundary operation. */
    async deleteMetadata(id) { calls.push("metadata:" + id); return { id, deleted: true, releasedChatIds: [] }; },
  });
  assert.deepEqual(calls, ["native:chat-first", "native:chat-second", "read:1", "metadata:1"]);
  assert.deepEqual(plain(result), { deletedChatIds: ["chat-first", "chat-second"], failedChatIds: [], metadataDeleted: true, metadataError: null });
});
/** Rejects a stale membership snapshot even when each native deletion reported success. */
test("group deletion does not erase metadata when backend membership cleanup has not actually committed", async () => {
  const api = loadModule("web/features/sidebar/groups/delete.ts"), calls = [];
  const result = await api.deleteConversationGroupMembers(group(), {
    /** Supplies an explicit unit success; this test deliberately leaves backend membership stale. */
    async deleteChat(chatId) { return { chatId, deletedAt: 1 }; },
    /** Returns the original nonempty input as the labelled stale metadata boundary. */
    async readGroup() { return group(); },
    /** Detects any illicit metadata deletion before membership cleanup. */
    async deleteMetadata(id) { calls.push(id); throw new Error("Unexpected metadata mutation"); },
  });
  assert.deepEqual(calls, []); assert.equal(result.metadataDeleted, false);
  assert.match(result.metadataError, /still has actual persisted members/);
});
/** Retains the original metadata failure after native success and never automatically repeats any operation. */
test("empty-group metadata deletion failures remain visible without retry or atomicity claims", async () => {
  const api = loadModule("web/features/sidebar/groups/delete.ts"), calls = [], original = new Error("Exact native file metadata failure");
  const result = await api.deleteConversationGroupMembers(group([]), {
    /** Rejects any manufactured member deletion for a genuinely empty group. */
    async deleteChat() { throw new Error("Empty group has no native chat deletion"); },
    /** Supplies the explicitly empty metadata boundary for this operation-order test. */
    async readGroup(id) { calls.push("read:" + id); return group([]); },
    /** Throws the original labelled metadata rejection exactly once. */
    async deleteMetadata(id) { calls.push("metadata:" + id); throw original; },
  });
  assert.deepEqual(calls, ["read:1", "metadata:1"]);
  assert.deepEqual(plain(result), { deletedChatIds: [], failedChatIds: [], metadataDeleted: false, metadataError: String(original) });
});
