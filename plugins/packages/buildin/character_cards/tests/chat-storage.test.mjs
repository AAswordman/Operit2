import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Supplies Core metadata only; plugin binding and snapshot behavior runs against real SQLite. */
async function environment(t, count = 3) {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), chats = new Map(), messages = new Map(), calls = [];
  for (let index = 0; index < count; index++) {
    const id = "chat-" + index;
    chats.set(id, { id, title: id, messageCount: 0, createdAt: "1700000000000", updatedAt: String(1700000000000 + index), isCurrent: false, inputTokens: 0, outputTokens: 0 });
  }
  const chat = {
    /** Validates an explicitly existing Core identity for binding submissions. */
    async findChat(options) {
      calls.push({ method: "findChat", options: plain(options) });
      const record = chats.get(options.query);
      if (record === undefined) throw new Error("Core chat missing");
      return { matchedCount: 1, chat: plain(record) };
    },
    async listAll() {
      calls.push({ method: "listAll" });
      return { totalCount: chats.size, currentChatId: null, chats: [...chats.values()].map(plain) };
    },
    /** Returns exactly the requested bounded batch without any plugin-owned business data. */
    async listChats(options) {
      calls.push({ method: "listChats", options: plain(options) });
      assert.ok(options.chat_ids.length <= 200);
      return { totalCount: options.chat_ids.length, currentChatId: null, chats: options.chat_ids.filter(id => chats.has(id)).map(id => plain(chats.get(id))) };
    },
    /** Serves the stored inclusive range so orphan checks can match real message revisions. */
    async getMessagesRange(chatId, options) {
      calls.push({ method: "getMessagesRange", options: plain(options) });
      const list = messages.get(chatId) ?? [];
      return { chatId, order: "asc", start: options.start, end: options.end, limit: options.end - options.start + 1, messages: list.slice(options.start, options.end + 1).map(plain) };
    },
  };
  const globals = { ...disk.globals, Tools: { Files: disk.files, Storage: disk.storage, Chat: chat } };
  const repository = await loadModule("src/storage/database.ts", globals).DatabaseCharacterRepository.open();
  const service = loadModule("src/service.ts", globals).createCharacterCardsService(repository);
  const storage = loadModule("src/chat-storage.ts", globals);
  return { disk, chats, messages, calls, repository, service, storage };
}

test("binding CRUD persists solely in SQL and role deletion respects explicit bindings", async t => {
  const h = await environment(t);
  const role = await h.service.dispatchDomain("character.create", { values: { name: "Bound role" } });
  const binding = { chatId: "chat-0", selection: "card:" + role.id };
  assert.deepEqual(plain(await h.service.dispatchDomain("chat.configuration.binding.write", binding)), binding);
  assert.deepEqual(plain(await h.storage.readChatBinding("chat-0")), binding);
  await assert.rejects(h.service.dispatchDomain("character.delete", { id: role.id }), /Rebind or delete/);
  assert.equal(h.calls.filter(call => call.method !== "findChat").length, 0);
  assert.deepEqual(plain(await h.storage.deleteChatBinding("chat-0")), { chatId: "chat-0", deleted: true });
  assert.equal(await h.storage.findChatBinding("chat-0"), null);
  assert.deepEqual(plain(await h.service.dispatchDomain("character.delete", { id: role.id })), { id: role.id, deleted: true });
  h.disk.storage.inspectPreferences("chats", values => assert.equal(Object.keys(values).filter(key => key.startsWith("binding_")).length, 0));
});

test("425 bound chats use three bounded metadata calls without listAll or extension reads", async t => {
  const h = await environment(t, 425), database = await h.storage.chatDatabase();
  await database.chats.edit(preferences => { for (const key of h.chats.keys()) preferences.set("binding_" + key, { version: 1, selection: "card:default" }); });
  h.disk.clearCalls();
  const listing = plain(await h.service.dispatchDomain("memory.chat.list", { ownerKey: "character:default" }));
  assert.equal(listing.chats.length, 425);
  assert.equal(listing.missingChatIds.length, 0);
  assert.equal(listing.chats[0].id, "chat-424");
  assert.deepEqual(h.calls.map(call => call.options.chat_ids.length), [200, 200, 25]);
  for (const query of h.disk.calls.filter(call => call.method === "storage.get")) assert.doesNotMatch(query.args[0], /FROM "(?:memories|embeddings|candidates)"/);
  const bindings = await h.storage.bindingsForChats([...h.chats.keys()]);
  assert.equal(bindings.size, 425);
  h.chats.delete("chat-0");
  const degraded = plain(await h.service.dispatchDomain("memory.chat.list", { ownerKey: "character:default" }));
  assert.deepEqual(degraded.missingChatIds, ["chat-0"]);
  assert.equal(degraded.chats.length, 424);
  assert.equal((await h.storage.bindingsForChats(["chat-0"])).size, 1);
});

test("pruning missing bindings deletes only the dangling ones and keeps the rest", async t => {
  const h = await environment(t, 3), database = await h.storage.chatDatabase();
  await database.chats.edit(preferences => {
    for (const key of h.chats.keys()) preferences.set("binding_" + key, { version: 1, selection: "card:default" });
  });
  h.chats.delete("chat-1");
  assert.deepEqual(plain(await h.service.dispatchDomain("memory.chat.pruneMissing", { ownerKey: "character:default" })), { removedChatIds: ["chat-1"], removedCandidateIds: [] });
  assert.equal(await h.storage.findChatBinding("chat-1"), null);
  assert.deepEqual(plain(await h.storage.findChatBinding("chat-0")), { chatId: "chat-0", selection: "card:default" });
  assert.deepEqual(plain(await h.service.dispatchDomain("memory.chat.pruneMissing", { ownerKey: "character:default" })), { removedChatIds: [], removedCandidateIds: [] });
});

test("pruneMissing also drops extraction candidates whose chat or source message disappeared", async t => {
  const h = await environment(t, 3);
  h.chats.set("chat-0", { ...h.chats.get("chat-0"), messageCount: 1 });
  h.messages.set("chat-0", [{ sender: "ai", content: "Controlled reply", timestamp: 10, variantIndex: 0, variantCount: 1, provider: "p", modelName: "m" }]);
  h.chats.delete("chat-1");
  await h.repository.run(async repository => {
    const space = await repository.readMemorySpace("character:default");
    space.candidates.push(
      { id: "keep-1", chatId: "chat-0", triggerMessageTimestamp: 10, triggerVariantIndex: 0, createdAt: 1, updatedAt: 1, status: "pending", attemptCount: 0, lastError: "", sourceType: "reply_finalized_auto" },
      { id: "orphan-message", chatId: "chat-0", triggerMessageTimestamp: 99, triggerVariantIndex: 0, createdAt: 1, updatedAt: 1, status: "pending", attemptCount: 0, lastError: "", sourceType: "reply_finalized_auto" },
      { id: "orphan-chat", chatId: "chat-1", triggerMessageTimestamp: 5, triggerVariantIndex: 0, createdAt: 1, updatedAt: 1, status: "failed", attemptCount: 1, lastError: "stale", sourceType: "reply_finalized_auto" },
    );
    await repository.writeMemorySpace("character:default", space);
  });
  assert.deepEqual(plain(await h.service.dispatchDomain("memory.chat.pruneMissing", { ownerKey: "character:default" })),
    { removedChatIds: [], removedCandidateIds: ["orphan-chat", "orphan-message"] });
  await h.repository.run(async repository => {
    const after = await repository.readMemorySpace("character:default");
    assert.deepEqual(plain(after.candidates.map(candidate => candidate.id)), ["keep-1"]);
  });
});

test("group and writable mount ownership is filtered through relation tables", async t => {
  const h = await environment(t);
  const store = await h.service.dispatchDomain("memory.shared.create", { name: "Shared" });
  const role = await h.service.dispatchDomain("character.create", { values: { name: "Mounted role", sharedMemoryMounts: [{ sharedMemoryId: store.id, readable: true, writable: false }] } });
  const group = await h.service.dispatchDomain("group.create", { values: { name: "Group", members: [{ characterCardId: role.id, orderIndex: 0 }] } });
  await h.storage.writeChatBinding({ chatId: "chat-1", selection: "group:" + group.id });
  assert.deepEqual(plain(await h.storage.chatsForMemoryOwner("shared:" + store.id)), []);
  await h.service.dispatchDomain("character.update", { id: role.id, changes: { sharedMemoryMounts: [{ sharedMemoryId: store.id, readable: true, writable: true }] } });
  assert.deepEqual(plain(await h.storage.chatsForMemoryOwner("shared:" + store.id)), ["chat-1"]);
  assert.deepEqual(plain(await h.storage.chatsForMemoryOwner("character:" + role.id)), ["chat-1"]);
});

test("message author reads have no configuration storage and never write SQL", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  let extension = { participantId: "default" };
  const storage = loadModule("src/chat-storage.ts", { ...disk.globals, Tools: { ...disk.globals.Tools, Chat: { async readExtension() { return { extension }; } } } });
  assert.equal(await storage.readMessageAuthor("chat-1", 10, 0), "default");
  assert.equal(disk.calls.length, 0);
  assert.equal(storage.storeSnapshot, undefined); assert.equal(storage.readStoredSnapshot, undefined);
  extension = { participantId: "default", profile: { introPrompt: "forbidden" } };
  await assert.rejects(() => storage.readMessageAuthor("chat-1", 10, 0), /Invalid message author identity/);
  extension = null;
  assert.equal(await storage.readMessageAuthor("chat-1", 10, 0), null);
});

test("draft commit is atomic and a stale edit publishes neither the new draft nor the deletion", async t => {
  const h = await environment(t), binding = { chatId: "chat-0", selection: "card:default" };
  await h.storage.prepareChatBinding(binding);
  assert.equal(await h.storage.findChatBinding(binding.chatId), null);
  await h.storage.commitDraftBinding(binding.chatId);
  assert.deepEqual(plain(await h.storage.readChatBinding(binding.chatId)), binding);
  const database = await h.storage.chatDatabase();
  let entered, release;
  const ready = new Promise(resolve => { entered = resolve; }), gate = new Promise(resolve => { release = resolve; });
  const pending = database.chats.edit(async preferences => {
    preferences.set("draft_other", { version: 1, selection: binding.selection });
    entered(); await gate;
    preferences.remove("binding_" + binding.chatId);
  });
  await ready;
  const other = await h.disk.storage.dataStore.open({ path: h.disk.directory + "/chats.preferences.json" });
  await other.edit(preferences => { preferences.set("binding_" + binding.chatId, { version: 1, selection: "model:chat" }); });
  release(); await assert.rejects(pending, /edit conflict/); await other.close();
  assert.equal((await database.chats.data()).get("draft_other"), undefined);
  await h.storage.prepareChatBinding({ ...binding, chatId: "aborted" });
  await h.storage.abortDraftBinding("aborted");
  assert.equal((await database.chats.data()).get("draft_aborted"), undefined);
});

test("memory settings and USER reads never query bodies or vectors", async t => {
  const h = await environment(t);
  h.disk.clearCalls();
  await h.service.dispatchDomain("memory.settings.read", { ownerKey: "character:default" });
  await h.service.dispatchDomain("memory.searchConfig.read", { ownerKey: "character:default" });
  await h.service.dispatchDomain("memory.user.read", { ownerKey: "character:default" });
  await h.service.dispatchDomain("memory.user.path", { ownerKey: "character:default" });
  const queries = h.disk.calls.filter(call => call.method === "storage.get");
  assert.equal(queries.filter(call => call.args[0] === "owners").length, 4);
  assert.equal(queries.some(call => ["memories", "embeddings", "candidates"].includes(call.args[0])), false);
  h.disk.clearCalls();
  assert.deepEqual(plain(await h.service.readGlobalMemoryPreferences()), { userDescriptionEnabled: true, autoUpdateEnabled: true });
  const flagQueries = h.disk.calls.filter(call => call.method === "storage.preferences.data");
  assert.equal(flagQueries.length, 1);
  assert.ok(flagQueries[0].args[0].endsWith("/meta.preferences.json"));
});


/** Requires a single persisted preference snapshot for a catalog larger than the former SQL page limit. */
test("batch binding catalog includes every bound and unbound host conversation using one preference snapshot", async t => {
  const h = await environment(t, 525);
  await h.storage.writeChatBinding({ chatId: "chat-1", selection: "card:default" });
  await h.storage.writeChatBinding({ chatId: "chat-2", selection: "model:chat" });
  h.disk.clearCalls();
  const rows = plain(await h.service.dispatch({ action: "listBindableChats" }));
  assert.equal(rows.length, 525);
  assert.equal(rows.find(chat => chat.id === "chat-0").selection, null);
  assert.equal(rows.find(chat => chat.id === "chat-1").selection, "card:default");
  assert.equal(rows.find(chat => chat.id === "chat-2").selection, "model:chat");
  assert.equal(rows[0].id, "chat-524");
  assert.deepEqual(h.calls.map(call => call.method), ["listAll"]);
  const reads = h.disk.calls.filter(call => call.method === "storage.preferences.data" && call.args[0].endsWith("/chats.preferences.json"));
  assert.equal(reads.length, 1);
  assert.equal(h.disk.calls.some(call => call.method === "storage.query"), false);
  assert.equal(h.disk.calls.filter(call => call.method === "storage.transaction").length, 0);
});


test("batch picker rejects duplicate histories and binds through the existing validated service", async t => {
  const h = await environment(t);
  const role = await h.service.dispatchDomain("character.create", { values: { name: "Batch target role" } });
  for (const chatId of ["chat-0", "chat-1"]) {
    assert.deepEqual(plain(await h.service.dispatch({ action: "writeChatBinding", chatId, selection: "card:" + role.id })), { chatId, selection: "card:" + role.id });
  }
  await assert.rejects(h.service.dispatch({ action: "writeChatBinding", chatId: "missing-chat", selection: "card:" + role.id }), /missing/);
  const rows = plain(await h.service.dispatch({ action: "listBindableChats" }));
  assert.equal(rows.filter(row => row.selection === "card:" + role.id).length, 2);
  assert.equal(rows.find(row => row.id === "chat-2").selection, null);
  h.chats.set("chat-2", h.chats.get("chat-0"));
  await assert.rejects(h.service.dispatch({ action: "listBindableChats" }), /重复/);
});
