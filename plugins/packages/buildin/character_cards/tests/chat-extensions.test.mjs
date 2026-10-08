import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Rejects every undeclared host method instead of synthesizing an empty source or success. */
function strictMethods(name, methods) {
  return new Proxy(methods, {
    /** Allows only the exact declared controlled host capability. */
    get(target, key) {
      if (!Object.hasOwn(target, key)) throw new Error("Unprovided controlled capability: " + name + "." + String(key));
      return target[key];
    },
  });
}
/** Uses real plugin disk IO and an explicitly controlled owner-isolated generic chat record fixture. */
async function extensionHarness(t) {
  const disk = await createDiskHarness(t), records = new Map(), calls = [];
  for (const id of ["chat-a", "chat-b", "chat-unowned"]) records.set(id, {
    summary: { id, title: id, messageCount: 0, createdAt: "1700000000000", updatedAt: "1700000000000", isCurrent: false, inputTokens: 0, outputTokens: 0 },
    extension: id === "chat-unowned" ? null : { version: 1, selection: "card:default", notes: { text: "同namespace保留字段", items: [null, 3, true] } },
  });
  const missing = new Error("Controlled existing record is missing"), faults = new Map();
  /** Validates the exact locked chat target and refuses a missing host record before returning a namespace. */
  function targetRecord(target) {
    assert.deepEqual(Object.keys(target).sort(), ["chatId", "kind"]); assert.equal(target.kind, "chat");
    if (!records.has(target.chatId)) throw missing;
    return records.get(target.chatId);
  }
  /** Records generic calls and propagates scheduled original failures without mutating fixtures. */
  function attempted(method, ...args) {
    calls.push({ method, args: plain(args) });
    if (faults.has(method)) { const failure = faults.get(method); faults.delete(method); throw failure; }
  }
  const chat = strictMethods("Chat", {
    /** Enumerates every explicitly present host record, never deleted or cached file bindings. */
    async listAll() {
      attempted("listAll");
      return { totalCount: records.size, currentChatId: null, chats: [...records.values()].map(
        /** Returns the controlled actual host record summary. */
        record => plain(record.summary),
      ),
        /** Formats only the scheduled directory fixture. */
        toString() { return "Controlled generic records"; },
      };
    },
    /** Supplies the audited exact generic identity query needed for a management submission. */
    async findChat(options) {
      attempted("findChat", options); assert.deepEqual(plain(options), { query: options.query, match: "exact", index: 0 });
      if (!records.has(options.query)) throw missing;
      return { matchedCount: 1, chat: plain(records.get(options.query).summary),
        /** Formats this actual controlled record query. */
        toString() { return this.chat.title; },
      };
    },
    /** Returns null only for an existing record with no executing-owner namespace. */
    async readExtension(target) { attempted("readExtension", target); const record = targetRecord(target); return record.extension === null ? null : plain(record.extension); },
    /** Replaces only the executing-owner object on the exact existing record and returns its stored value. */
    async writeExtension(target, value) { attempted("writeExtension", target, value); const record = targetRecord(target); record.extension = plain(value); return plain(record.extension); },
    /** Removes only the executing-owner namespace and does not delete a host chat or plugin group. */
    async deleteExtension(target) { attempted("deleteExtension", target); const record = targetRecord(target), existed = record.extension !== null; record.extension = null; return existed; },
  });
  const globals = { ...disk.globals, Tools: strictMethods("Tools", { Files: disk.files, Chat: chat }) };
  const files = loadModule("src/storage/files.ts", globals), repository = await files.FileCharacterRepository.open();
  const service = loadModule("src/service.ts", globals).createCharacterCardsService(repository);
  const statePath = path.join(disk.directory, "character-memory", "state.json");
  return { disk, records, calls, missing, repository, service, statePath,
    /** Schedules exactly one controlled original host error. */
    failNext(method, failure) { assert.equal(faults.has(method), false); faults.set(method, failure); },
  };
}

/** Proves management APIs use only real record extensions and never publish a plugin-file binding mirror. */
test("binding read/write/delete use only owner-isolated generic extensions and preserve other namespace fields", async t => {
  const h = await extensionHarness(t), card = plain(await h.service.dispatchDomain("character.create", { values: { name: "真实角色", characterSetting: "正文" } }));
  const stateBefore = await readFile(h.statePath), original = plain(h.records.get("chat-a").extension);
  const written = plain(await h.service.dispatchDomain("chat.configuration.binding.write", { chatId: "chat-a", selection: "card:" + card.id }));
  assert.deepEqual(written, { chatId: "chat-a", selection: "card:" + card.id });
  assert.deepEqual(h.records.get("chat-a").extension, { ...original, selection: written.selection });
  assert.deepEqual(plain(await h.service.dispatchDomain("chat.configuration.binding.read", { chatId: "chat-a" })), written);
  assert.deepEqual(await readFile(h.statePath), stateBefore);
  const state = JSON.parse(stateBefore); assert.equal(state.version, 3); assert.equal(Object.hasOwn(state, "chatBindings"), false);
  assert.deepEqual(plain(await h.service.dispatchDomain("chat.configuration.binding.delete", { chatId: "chat-a" })), { chatId: "chat-a", deleted: true });
  assert.deepEqual(plain(await h.service.dispatchDomain("chat.configuration.binding.delete", { chatId: "chat-a" })), { chatId: "chat-a", deleted: false });
  assert.deepEqual(await readFile(h.statePath), stateBefore);
  await assert.rejects(() => h.service.dispatchDomain("chat.configuration.binding.read", { chatId: "chat-a" }), /no character selection/);
  const writes = h.calls.filter(
    /** Inspects only actual generic extension submissions. */
    call => call.method === "writeExtension",
  );
  assert.equal(writes.length, 1); assert.deepEqual(writes[0].args[0], { kind: "chat", chatId: "chat-a" });
});

/** Distinguishes legitimate null namespaces from absent records, corrupt markers and scheduled host failures. */
test("extension errors and malformed markers propagate without retry or synthesized bindings", async t => {
  const h = await extensionHarness(t), before = await readFile(h.statePath);
  await assert.rejects(() => h.service.dispatchDomain("chat.configuration.binding.read", { chatId: "not-persisted" }),
    /** Retains the exact host not-found error. */
    error => error === h.missing,
  );
  await assert.rejects(() => h.service.dispatchDomain("chat.configuration.binding.read", { chatId: "chat-unowned" }), /no character selection/);
  h.records.get("chat-a").extension = { version: 0, selection: "card:default" };
  await assert.rejects(() => h.service.dispatchDomain("chat.configuration.binding.write", { chatId: "chat-a", selection: "card:default" }), /Unsupported chat marker/);
  assert.equal(h.calls.filter(
    /** Ensures corrupt existing metadata was not silently replaced. */
    call => call.method === "writeExtension",
  ).length, 0);
  h.records.get("chat-a").extension = { version: 1, selection: "card:default" };
  const failure = new Error("Controlled extension write failure"); h.failNext("writeExtension", failure);
  await assert.rejects(() => h.service.dispatchDomain("chat.configuration.binding.write", { chatId: "chat-a", selection: "card:default" }),
    /** Retains the scheduled exact error instead of retrying or swallowing it. */
    error => error === failure,
  );
  assert.equal(h.calls.filter(
    /** Counts all attempted writes, including the failed submission. */
    call => call.method === "writeExtension",
  ).length, 1);
  assert.deepEqual(await readFile(h.statePath), before);
});

/** Checks referenced-selection deletion against actual extant host records rather than deleted file mirrors. */
test("selection deletion reads actual host records and blocks referenced roles without inventing deleted-chat cleanup", async t => {
  const h = await extensionHarness(t), card = plain(await h.service.dispatchDomain("character.create", { values: { name: "将删除角色" } }));
  await h.service.dispatchDomain("chat.configuration.binding.write", { chatId: "chat-a", selection: "card:" + card.id });
  const before = await readFile(h.statePath);
  await assert.rejects(() => h.service.dispatchDomain("character.delete", { id: card.id }), /Rebind or delete the chat extension/);
  assert.deepEqual(await readFile(h.statePath), before);
  h.records.delete("chat-a");
  assert.deepEqual(plain(await h.service.dispatchDomain("character.delete", { id: card.id })), { id: card.id, deleted: true });
  assert.equal(h.calls.filter(
    /** Confirms both deletion checks enumerated the real generic source. */
    call => call.method === "listAll",
  ).length, 2);
  assert.equal(Object.hasOwn(JSON.parse(await readFile(h.statePath, "utf8")), "chatBindings"), false);
});

/** Exercises memory-chat ownership and group metadata against the same service after file bindings were removed. */
test("memory chat lists use real extension owners and namespace deletion does not mutate manual group membership", async t => {
  const h = await extensionHarness(t), ownerKey = "character:default";
  const initial = plain(await h.service.dispatchDomain("memory.chat.list", { ownerKey }));
  assert.deepEqual(initial.map(
    /** Selects exact genuinely bound chat identities, excluding explicit null namespaces. */
    chat => chat.id,
  ), ["chat-a", "chat-b"]);
  const group = plain(await h.service.dispatchDomain("conversation-group.create", { ownerSelection: "card:default", name: "文件归属", pinned: false }));
  await h.service.dispatchDomain("conversation-group.moveChat", { ownerSelection: "card:default", groupId: group.id, chatId: "chat-a" });
  const stateBefore = await readFile(h.statePath);
  await h.service.dispatchDomain("chat.configuration.binding.delete", { chatId: "chat-a" });
  assert.deepEqual(await readFile(h.statePath), stateBefore);
  const listed = plain(await h.service.dispatchDomain("conversation-group.list", { ownerSelection: "card:default" }));
  assert.deepEqual(listed[0].chatIds, ["chat-a"]);
  const remaining = plain(await h.service.dispatchDomain("memory.chat.list", { ownerKey }));
  assert.deepEqual(remaining.map(
    /** Retains only the actual remaining record extension binding. */
    chat => chat.id,
  ), ["chat-b"]);
});
