import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Supplies Core metadata only; plugin binding and snapshot behavior runs against real SQLite. */
async function environment(t, count = 3) {
  const disk = await createDiskHarness(t), chats = new Map(), calls = [];
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
    /** Returns exactly the requested bounded batch without any plugin-owned business data. */
    async listChats(options) {
      calls.push({ method: "listChats", options: plain(options) });
      assert.ok(options.chat_ids.length <= 200);
      return { totalCount: options.chat_ids.length, currentChatId: null, chats: options.chat_ids.filter(id => chats.has(id)).map(id => plain(chats.get(id))) };
    },
  };
  const globals = { ...disk.globals, Tools: { Files: disk.files, Storage: disk.storage, Chat: chat } };
  const repository = await loadModule("src/storage/database.ts", globals).DatabaseCharacterRepository.open();
  const service = loadModule("src/service.ts", globals).createCharacterCardsService(repository);
  const storage = loadModule("src/chat-storage.ts", globals);
  return { disk, chats, calls, repository, service, storage };
}

/** Constructs a complete immutable send snapshot with an explicit memory permission. */
function snapshot() {
  const profile = { id: "default", name: "Original role", avatarUri: null, introPrompt: "Original prompt", userPreferencesText: "Original USER", openingStatement: "Hello", modelBinding: { providerId: "provider", modelId: "model" }, ttsConfigId: "voice", toolAccess: { enabled: false, allowedBuiltinTools: [], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] }, resources: [{ key: "character:default", readable: true, writable: true }] };
  return { version: 1, selection: "card:default", promptFunctionType: "CHAT", primaryOwnerKey: "character:default", profile, participants: [plain(profile)] };
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
  h.disk.storage.inspect(sql => assert.equal(sql.prepare('SELECT COUNT(*) AS n FROM "chat_bindings"').get().n, 0));
});

test("425 bound chats use three bounded metadata calls without listAll or extension reads", async t => {
  const h = await environment(t, 425), database = await h.storage.chatDatabase();
  await database.commit([...h.chats.keys()].map(key => ({ op: "put", collection: "chat_bindings", key, value: { version: 1, selection: "card:default" }, expectedVersion: null })));
  h.disk.clearCalls();
  const rows = plain(await h.service.dispatchDomain("memory.chat.list", { ownerKey: "character:default" }));
  assert.equal(rows.length, 425);
  assert.equal(rows[0].id, "chat-424");
  assert.deepEqual(h.calls.map(call => call.options.chat_ids.length), [200, 200, 25]);
  for (const query of h.disk.calls.filter(call => call.method === "storage.query")) assert.doesNotMatch(query.args[0], /FROM "(?:memories|embeddings|candidates)"/);
  const bindings = await h.storage.bindingsForChats([...h.chats.keys()]);
  assert.equal(bindings.size, 425);
  h.chats.delete("chat-0");
  await assert.rejects(h.service.dispatchDomain("memory.chat.list", { ownerKey: "character:default" }), /complete|missing|incomplete/i);
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

test("generic references retain immutable historical profiles after current role edits", async t => {
  const h = await environment(t), saved = snapshot();
  const reference = await h.storage.storeSnapshot(saved);
  assert.deepEqual(Object.keys(reference).sort(), ["recordId", "schemaVersion"]);
  await h.service.dispatchDomain("character.update", { id: "default", changes: { name: "Changed role", characterSetting: "Changed prompt" } });
  assert.deepEqual(plain(await h.storage.readStoredSnapshot(reference)), saved);
  await assert.rejects(h.storage.readStoredSnapshot({ recordId: "missing", schemaVersion: 1 }), /unavailable/);
});

test("draft commit is atomic and a stale deletion rolls back the entire SQL batch", async t => {
  const h = await environment(t), binding = { chatId: "chat-0", selection: "card:default" };
  await h.storage.prepareChatBinding(binding);
  assert.equal(await h.storage.findChatBinding(binding.chatId), null);
  await h.storage.commitDraftBinding(binding.chatId);
  assert.deepEqual(plain(await h.storage.readChatBinding(binding.chatId)), binding);
  const database = await h.storage.chatDatabase(), original = await database.get("chat_bindings", binding.chatId);
  await h.storage.writeChatBinding(binding);
  await assert.rejects(database.commit([
    { op: "put", collection: "chat_drafts", key: "other", value: { version: 1, selection: binding.selection }, expectedVersion: null },
    { op: "delete", collection: "chat_bindings", key: binding.chatId, expectedVersion: original.version },
  ]), /NOT NULL|precondition/);
  assert.equal(await database.get("chat_drafts", "other"), null);
  await h.storage.prepareChatBinding({ ...binding, chatId: "aborted" });
  await h.storage.abortDraftBinding("aborted");
  assert.equal(await database.get("chat_drafts", "aborted"), null);
});

test("memory settings and USER reads never query bodies or vectors", async t => {
  const h = await environment(t);
  h.disk.clearCalls();
  await h.service.dispatchDomain("memory.settings.read", { ownerKey: "character:default" });
  await h.service.dispatchDomain("memory.searchConfig.read", { ownerKey: "character:default" });
  await h.service.dispatchDomain("memory.user.read", { ownerKey: "character:default" });
  await h.service.dispatchDomain("memory.user.path", { ownerKey: "character:default" });
  const queries = h.disk.calls.filter(call => call.method === "storage.query");
  assert.equal(queries.length, 4);
  for (const query of queries) assert.match(query.args[0], /SELECT o\.\* FROM "owners"/);
  h.disk.clearCalls();
  assert.deepEqual(plain(await h.service.readGlobalMemoryPreferences()), { userDescriptionEnabled: true, autoUpdateEnabled: true });
  const flagQueries = h.disk.calls.filter(call => call.method === "storage.query");
  assert.equal(flagQueries.length, 1);
  assert.match(flagQueries[0].args[0], /SELECT \* FROM "meta"/);
});
