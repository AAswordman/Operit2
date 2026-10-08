import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile, writeFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { loadModule, plain } from "./runtime.mjs";

const operations = ["list", "create", "update", "delete", "moveChat", "reorder"];
const roleScope = "card:default", workspaceScope = "workspace:/opaque/project";

/** Rejects every undeclared host capability instead of manufacturing successful domain results. */
function strictHost(name, methods) {
  return new Proxy(methods, {
    /** Exposes only the explicitly controlled generic host methods. */
    get(target, key) {
      if (!Object.hasOwn(target, key)) throw new Error("Unprovided controlled capability: " + name + "." + String(key));
      return target[key];
    },
  });
}

/** Opens the actual main registrations and real disk repository with only controlled exact chat lookup. */
async function groupHarness(t) {
  const disk = await createDiskHarness(t), lookups = [];
  const chats = new Map(["chat-a", "chat-b", "chat-c"].map(
    /** Supplies explicit existing host identities without group or character fields. */
    id => [id, { id, title: id, messageCount: 0, createdAt: "1720000000000", updatedAt: "1720000000000", isCurrent: false, inputTokens: 0, outputTokens: 0, characterCardName: null }],
  ));
  const missing = new Error("Controlled host chat is missing");
  const chat = strictHost("Chat", {
    /** Implements the production exact-ID query with the real generic SDK result shape. */
    async findChat(options) {
      assert.deepEqual(plain(options), { query: options.query, match: "exact", index: 0 });
      lookups.push(options.query);
      if (!chats.has(options.query)) throw missing;
      return { matchedCount: 1, chat: plain(chats.get(options.query)),
        /** Formats this explicit controlled host lookup result. */
        toString() { return this.chat.title; },
      };
    },
  });
  const tools = strictHost("Tools", { Files: disk.files, Chat: chat });
  const statePath = path.join(disk.directory, "character-memory", "state.json");
  /** Creates a fresh production runtime on the same disk without retaining a second service or cached state. */
  function open() {
    const plugin = openPlugin({ ...disk, globals: { ...disk.globals, Tools: tools } });
    assert.equal(plugin.toolLifecycleHooks.length, 1); assert.equal(plugin.toolPromptHooks.length, 1);
    return plugin;
  }
  /** Reads authoritative native file bytes independently from the production repository. */
  async function state() { return JSON.parse(await readFile(statePath, "utf8")); }
  /** Counts only actual publication of the authoritative snapshot, not independent USER.md files. */
  function publications() {
    return disk.calls.filter(
      /** Requires the exact destination path declared by the plugin. */
      call => call.method === "move" && path.resolve(call.args[1]) === statePath,
    );
  }
  return { disk, chats, lookups, missing, tools, statePath, open, state, publications };
}

/** Calls one actual registered public provider and copies only its cross-realm JSON result. */
async function api(plugin, operation, input) { return plain(await plugin.api(operation, input)); }

/** Creates a manual group through the sole registered domain service, never by writing fixture records. */
async function create(plugin, name, ownerSelection = null, pinned = false) {
  return api(plugin, "conversation-group.create", { ownerSelection, name, pinned });
}

/** Lists one explicitly supplied scope through the actual public API. */
async function list(plugin, ownerSelection) { return api(plugin, "conversation-group.list", { ownerSelection }); }

/** Moves an explicitly present host chat through the actual generic identity check and file service. */
async function move(plugin, chatId, groupId, ownerSelection) {
  return api(plugin, "conversation-group.moveChat", { chatId, groupId, ownerSelection });
}

/** Proves rejected input causes no authoritative publication and preserves the original snapshot bytes. */
async function rejectsWithoutWrite(harness, invoke, expectation) {
  const before = await readFile(harness.statePath), count = harness.publications().length;
  await assert.rejects(invoke, expectation);
  assert.deepEqual(await readFile(harness.statePath), before);
  assert.equal(harness.publications().length, count);
}

/** Exercises real main registration and typed dependency wrappers before any repository read. */
test("six manual-group APIs register once without storage IO and wrappers/Web use the same file service", async t => {
  const h = await groupHarness(t), plugin = h.open();
  assert.equal(h.disk.calls.length, 0, "Registration must not initialize business storage");
  for (const operation of operations) assert.ok(plugin.apis.has("conversation-group." + operation));
  const dependencies = [];
  const wrappers = loadModule("src/api.ts", { ToolPkg: {
    /** Delivers a real typed dependency request to its actually registered provider. */
    callDependency(packageId, operation, payload) {
      assert.equal(packageId, "com.operit.character_cards"); dependencies.push(operation);
      return plugin.api(operation, payload);
    },
  } }).characterCards.conversationGroups;
  assert.deepEqual(plain(await wrappers.list({ ownerSelection: null })), []);
  const state = await h.state();
  assert.equal(state.version, 3); assert.deepEqual(state.conversationGroups, []);
  assert.equal(Object.hasOwn(state, "chatBindings"), false);
  const first = plain(await wrappers.create({ ownerSelection: null, name: "全局", pinned: false }));
  const updated = plain(await wrappers.update({ id: first.id, changes: { pinned: true } }));
  assert.equal(updated.pinned, true);
  assert.deepEqual(plain(await wrappers.moveChat({ chatId: "chat-a", groupId: first.id, ownerSelection: null })), { chatId: "chat-a", previousGroupId: null, groupId: first.id });
  const ordered = plain(await wrappers.reorder({ ownerSelection: null, ids: [first.id] }));
  assert.deepEqual(plain(await plugin.web({ action: "listConversationGroups", ownerSelection: null })), ordered);
  const removed = plain(await wrappers.delete({ id: first.id }));
  assert.deepEqual(removed, { id: first.id, deleted: true, releasedChatIds: ["chat-a"] });
  assert.deepEqual(dependencies, ["list", "create", "update", "moveChat", "reorder", "delete"].map(
    /** Matches the exact six wrapper operations in actual invocation order. */
    operation => "conversation-group." + operation,
  ));
});

/** Verifies independent scopes, complete records, metadata edits and actual runtime restart persistence. */
test("scope-local CRUD preserves complete records and accepts identical names across role/workspace/global scopes", async t => {
  const h = await groupHarness(t), plugin = h.open();
  const global = await create(plugin, "同名"), role = await create(plugin, "同名", roleScope), workspace = await create(plugin, "同名", workspaceScope, true);
  assert.notEqual(global.id, role.id); assert.notEqual(role.id, workspace.id);
  for (const group of [global, role, workspace]) {
    assert.match(group.id, /^[1-9][0-9]*$/); assert.equal(group.displayOrder, 0);
    assert.ok(Number.isSafeInteger(group.createdAt)); assert.equal(group.updatedAt, group.createdAt); assert.deepEqual(group.chatIds, []);
  }
  await rejectsWithoutWrite(h, () => create(plugin, "同名", roleScope), /Name already exists/);
  const role2 = await create(plugin, "第二组", roleScope);
  await rejectsWithoutWrite(h, () => plugin.api("conversation-group.update", { id: role2.id, changes: { name: role.name } }), /Name already exists/);
  await move(plugin, "chat-a", role.id, roleScope);
  const before = (await list(plugin, roleScope))[0];
  const changed = await api(plugin, "conversation-group.update", { id: role.id, changes: { name: "重命名", pinned: true } });
  assert.deepEqual(changed, { ...before, name: "重命名", pinned: true, updatedAt: changed.updatedAt });
  assert.ok(changed.updatedAt >= before.updatedAt);
  const diskState = await h.state(), stateBytes = await readFile(h.statePath); h.disk.clearCalls();
  const reopened = h.open();
  assert.deepEqual(await list(reopened, roleScope), [changed, role2]);
  assert.deepEqual(await list(reopened, null), [global]); assert.deepEqual(await list(reopened, workspaceScope), [workspace]);
  assert.deepEqual((await h.state()).conversationGroups, diskState.conversationGroups);
  assert.deepEqual(await readFile(h.statePath), stateBytes); assert.equal(h.publications().length, 0);
});

/** Checks single membership across every scope, explicit ungrouping and metadata-only deletion. */
test("membership transfers are unique across scopes and deleting a group releases chats without host deletion", async t => {
  const h = await groupHarness(t), plugin = h.open();
  const role = await create(plugin, "角色组", roleScope), workspace = await create(plugin, "工作区组", workspaceScope), next = await create(plugin, "后续组", roleScope);
  const original = await h.state(), userPath = path.join(h.disk.directory, "character-memory", original.owners[0].userDocumentPath);
  const userBytes = await readFile(userPath);
  assert.deepEqual(await move(plugin, "chat-a", role.id, roleScope), { chatId: "chat-a", previousGroupId: null, groupId: role.id });
  const writes = h.publications().length;
  assert.deepEqual(await move(plugin, "chat-a", role.id, roleScope), { chatId: "chat-a", previousGroupId: role.id, groupId: role.id });
  assert.equal(h.publications().length, writes, "Idempotent membership must not duplicate or republish");
  await rejectsWithoutWrite(h, () => move(plugin, "chat-a", workspace.id, roleScope), /different scope/);
  await rejectsWithoutWrite(h, () => move(plugin, "chat-a", null, workspaceScope), /another scope/);
  assert.deepEqual(await move(plugin, "chat-a", workspace.id, workspaceScope), { chatId: "chat-a", previousGroupId: role.id, groupId: workspace.id });
  assert.deepEqual((await list(plugin, roleScope))[0].chatIds, []);
  assert.deepEqual(await move(plugin, "chat-a", null, workspaceScope), { chatId: "chat-a", previousGroupId: workspace.id, groupId: null });
  await move(plugin, "chat-a", role.id, roleScope); await move(plugin, "chat-b", role.id, roleScope);
  assert.deepEqual(await api(plugin, "conversation-group.delete", { id: role.id }), { id: role.id, deleted: true, releasedChatIds: ["chat-a", "chat-b"] });
  assert.deepEqual(await list(plugin, roleScope), [{ ...next, displayOrder: 0, updatedAt: (await list(plugin, roleScope))[0].updatedAt }]);
  assert.equal(h.chats.size, 3, "Metadata deletion must not delete host chats");
  const final = await h.state();
  assert.deepEqual(final.groups, original.groups); assert.deepEqual(final.active, original.active);
  assert.equal(Object.hasOwn(final, "chatBindings"), false); assert.deepEqual(await readFile(userPath), userBytes);
  await rejectsWithoutWrite(h, () => move(plugin, "unknown-chat", workspace.id, workspaceScope),
    /** Requires the exact host error rather than an empty result or fabricated successful move. */
    failure => failure === h.missing,
  );
});

/** Ensures complete scoped permutations reject duplicate, missing or cross-scope IDs without partial mutation. */
test("manual ordering is a complete scoped permutation and persists after a fresh main runtime", async t => {
  const h = await groupHarness(t), plugin = h.open();
  const a = await create(plugin, "A", roleScope), b = await create(plugin, "B", roleScope), c = await create(plugin, "C", roleScope), global = await create(plugin, "A");
  await rejectsWithoutWrite(h, () => plugin.api("conversation-group.reorder", { ownerSelection: roleScope, ids: [a.id, b.id] }), /all conversation groups/);
  await rejectsWithoutWrite(h, () => plugin.api("conversation-group.reorder", { ownerSelection: roleScope, ids: [a.id, a.id, c.id] }), /duplicate/);
  await rejectsWithoutWrite(h, () => plugin.api("conversation-group.reorder", { ownerSelection: roleScope, ids: [a.id, global.id, c.id] }), /another scope/);
  const ordered = await api(plugin, "conversation-group.reorder", { ownerSelection: roleScope, ids: [c.id, a.id, b.id] });
  assert.deepEqual(ordered.map(
    /** Reads the actual persisted scoped order. */
    group => [group.id, group.displayOrder],
  ), [[c.id, 0], [a.id, 1], [b.id, 2]]);
  assert.deepEqual(await list(plugin, null), [global]);
  assert.deepEqual(await list(h.open(), roleScope), ordered);
});

/** Rejects malformed API input rather than supplying implicit scope, pin flags or editable fields. */
test("required scope and full input validation reject malformed group requests without publishing", async t => {
  const h = await groupHarness(t), plugin = h.open(), group = await create(plugin, "真实组");
  const invalid = [
    ["list", {}], ["list", { ownerSelection: " " }], ["create", { name: "缺scope", pinned: false }],
    ["create", { ownerSelection: null, name: "缺pin" }], ["update", { id: group.id, changes: {} }],
    ["update", { id: group.id, changes: { chatIds: ["chat-a"] } }], ["update", { id: group.id, changes: { ownerSelection: roleScope } }],
    ["moveChat", { chatId: "chat-a", groupId: group.id }], ["delete", { id: Number(group.id) }],
    ["reorder", { ownerSelection: null, ids: [group.id], unexpected: true }],
  ];
  for (const [operation, payload] of invalid) await rejectsWithoutWrite(h, () => plugin.api("conversation-group." + operation, payload), /./);
});

/** Opens deliberately corrupted authoritative files without repairing, migrating or writing missing schema data. */
test("schema v3 rejects old versions, file chat bindings, absent collections and corrupt membership/order records on real disk", async t => {
  const cases = [
    /** Uses the explicitly unsupported former file version. */
    state => { state.version = 1; },
    /** Uses the removed dual-source schema rather than migrating file bindings. */
    state => { state.version = 2; },
    /** Rejects the removed file-binding collection even when the version claims the new schema. */
    state => { state.chatBindings = []; },
    /** Removes the mandatory collection to ensure no empty collection is synthesized. */
    state => { delete state.conversationGroups; },
    /** Removes the explicit global scope field from a persisted full record. */
    state => { delete state.conversationGroups[0].ownerSelection; },
    /** Duplicates a chat across distinct valid groups. */
    state => { state.conversationGroups[0].chatIds = ["chat-a"]; state.conversationGroups[1].chatIds = ["chat-a"]; },
    /** Creates a discontinuous scope-local display order. */
    state => { state.conversationGroups[1].displayOrder = 5; },
    /** Creates a duplicate same-scope name. */
    state => { state.conversationGroups[1].name = state.conversationGroups[0].name; },
    /** Persists a numeric identity instead of a lossless decimal string. */
    state => { state.conversationGroups[0].id = 9007199254740992; },
  ];
  for (const corrupt of cases) {
    const h = await groupHarness(t), plugin = h.open();
    await create(plugin, "A"); await create(plugin, "B");
    const state = await h.state(); corrupt(state);
    const bytes = JSON.stringify(state); await writeFile(h.statePath, bytes, "utf8"); h.disk.clearCalls();
    await assert.rejects(() => list(h.open(), null), /./);
    assert.equal(await readFile(h.statePath, "utf8"), bytes);
    assert.equal(h.disk.calls.filter(
      /** Inspects every native mutation attempt, including USER.md and staging files. */
      call => ["write", "move", "mkdir", "deleteFile"].indexOf(call.method) !== -1,
    ).length, 0);
  }
});

/** Checks original publication errors, unchanged disk bytes, fail-stop and explicit unfinished-file rejection. */
test("failed group publication propagates the original error, never retries and never publishes partial metadata", async t => {
  const h = await groupHarness(t), plugin = h.open(); await create(plugin, "初始");
  const before = await readFile(h.statePath), failure = new Error("Controlled state rename failure");
  h.disk.clearCalls(); h.disk.failNext("move", failure);
  await assert.rejects(() => create(plugin, "未发布"),
    /** Preserves the exact filesystem failure object. */
    error => error === failure,
  );
  assert.deepEqual(await readFile(h.statePath), before); assert.equal(h.publications().length, 1);
  const calls = h.disk.calls.length;
  await assert.rejects(() => create(plugin, "禁止重试"),
    /** Retains the same fail-stop error for later operations. */
    error => error === failure,
  );
  assert.equal(h.disk.calls.length, calls, "Failed runtime must not attempt another publication");
  await assert.rejects(() => list(h.open(), null), /Unfinished plugin snapshot publication/);
  assert.deepEqual(await readFile(h.statePath), before);
});

/** Exercises complete cross-scope backup records, lossless IDs and unchanged memory/real USER.md data. */
test("character backup round-trips full manual groups atomically while preserving memory and the real USER.md", async t => {
  const h = await groupHarness(t), plugin = h.open();
  const role = await create(plugin, "同名", roleScope, true), global = await create(plugin, "同名");
  await move(plugin, "chat-a", role.id, roleScope); await move(plugin, "chat-b", global.id, null);
  const ownerKey = "character:default";
  await plugin.api("memory.create", { ownerKey, values: { title: "完整记忆", content: "保留正文", contentType: "text/plain", source: "manual", credibility: 0.8, importance: 0.6, folderPath: "documents/notes", tags: ["真实标签"] } });
  const memoryBackup = JSON.parse((await api(plugin, "memory.export", { ownerKey })).content);
  await plugin.api("memory.delete", { ownerKey, id: memoryBackup.space.memories[0].id });
  memoryBackup.space.memories[0].id = "9007199254740993";
  memoryBackup.space.memories[0].isDocumentNode = true; memoryBackup.space.memories[0].documentPath = "resources/full-document.md";
  memoryBackup.userMarkdown = "# USER\n真实文件正文\n";
  await plugin.api("memory.import", { ownerKey, content: JSON.stringify(memoryBackup), strategy: "UPDATE" });
  const stateBefore = await h.state(), userPath = path.join(h.disk.directory, "character-memory", stateBefore.owners[0].userDocumentPath), userBytes = await readFile(userPath);
  const exported = JSON.parse((await api(plugin, "character.exportBackup", {})).content);
  assert.equal(exported.version, 2); assert.deepEqual(exported.conversationGroups, stateBefore.conversationGroups);
  exported.conversationGroups[0].id = "9007199254741000";
  exported.conversationGroups[0].createdAt = 1700000000123; exported.conversationGroups[0].updatedAt = 1700000000456;
  await create(plugin, "将被完整替换", workspaceScope);
  const count = h.publications().length;
  await plugin.api("character.importBackup", { content: JSON.stringify(exported) });
  assert.equal(h.publications().length, count + 1, "Characters/tags/groups must share exactly one snapshot publication");
  const restored = await h.state(); assert.deepEqual(restored.conversationGroups, exported.conversationGroups);
  assert.deepEqual(restored.owners, stateBefore.owners); assert.deepEqual(await readFile(userPath), userBytes);
  assert.equal((await api(plugin, "memory.list", { ownerKey })).items[0].id, "9007199254740993");
  assert.equal((await api(plugin, "memory.user.path", { ownerKey })).path, userPath.replaceAll("\\", "/"));
  assert.deepEqual(await list(h.open(), roleScope), [exported.conversationGroups[0]]);
  const invalid = plain(exported); invalid.conversationGroups[1].chatIds.push("chat-a");
  await rejectsWithoutWrite(h, () => plugin.api("character.importBackup", { content: JSON.stringify(invalid) }), /more than one conversation group/);
  const created = await create(plugin, "高位ID之后"); assert.ok(BigInt(created.id) > 9007199254741000n);
});
