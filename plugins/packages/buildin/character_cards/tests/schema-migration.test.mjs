import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";
import { fixture } from "./fixtures.mjs";

const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

/** Reads exact plugin-relative resource bytes within the isolated disk fixture. */
function bytes(disk, relative) { return disk.readBytes(disk.directory + "/" + relative); }

/** Reads the actual migration marker independently of the TS migration implementation. */
function metadata(disk) {
  return disk.storage.inspectPreferences("meta", values => values.state);
}

/** Supplies the generic Chat host with exact revision extensions and a separate foreign namespace. */
function chatHost() {
  const extensions = new Map([[0, { participantId: "operit" }], [1, { participantId: "travel" }], [2, null]]);
  const foreign = { participantId: "unrelated-plugin-identity" }, writes = [];
  return {
    extensions, foreign, writes,
    /** Lists Core conversations without reading character binding projections. */
    async listAll() { return { chats: [{ id: "core-chat", messageCount: 1 }] }; },
    /** Supplies all revision counts through the existing paged message contract. */
    async getMessagesRange(chatId, options) {
      assert.equal(chatId, "core-chat"); assert.equal(options.start, 0);
      return { messages: [{ timestamp: 987, variantIndex: 1, variantCount: 3 }] };
    },
    /** Reads only the authenticated plugin namespace at the exact Core revision. */
    async readExtension(target) {
      assert.equal(target.chatId, "core-chat"); assert.equal(target.messageTimestamp, 987);
      assert.ok(extensions.has(target.variantIndex));
      return { extension: structuredClone(extensions.get(target.variantIndex)) };
    },
    /** Writes only the authenticated namespace while retaining foreign extensions. */
    async writeExtension(target, value) {
      writes.push(structuredClone({ target, value })); extensions.set(target.variantIndex, structuredClone(value));
      return { extension: structuredClone(value) };
    },
  };
}

/** Opens production plugin storage with generic host fixtures only. */
async function open(disk, chat) {
  const globals = { ...disk.globals, Tools: { Files: disk.files, Storage: disk.storage, Chat: chat } };
  return loadModule("src/storage/preferences.ts", globals).CharacterStorage.open();
}

/** Seeds released-shape version-one JSON records and exact owner resource bytes on real disk. */
async function seed(t, count = 3) {
  const disk = await createDiskHarness(t, { legacyIdentity: "operit" });
  const source = fixture(), model = loadModule("src/storage/state.ts");
  const state = {
    version: 1, memoryPreferences: source.snapshot.memoryPreferences,
    cards: source.snapshot.cards, groups: source.snapshot.groups, tags: source.snapshot.tags, stores: source.snapshot.stores,
    active: source.snapshot.active,
    owners: ["character:operit", "character:travel", "shared:shared-main"].map(key => plain(model.createMemorySpace(key))),
    conversationGroups: [{ id: "manual-1", ownerSelection: "group:group-one", name: "Manual group", chatIds: ["core-chat"], pinned: false, displayOrder: 0, createdAt: 1, updatedAt: 2 }],
  };
  state.cards[0].isDefault = true;
  state.cards[0].chatModelId = "external-model";
  state.cards[0].chatProviderId = "external-provider";
  state.cards[0].ttsConfigId = "external-tts";
  state.cards[0].themeConfigId = "external-theme";
  state.cards[0].sharedMemoryMounts = [{ sharedMemoryId: "shared-main", readable: true, writable: false }];
  state.owners[0].memories = Array.from({ length: count }, (_, index) => ({ ...structuredClone(source.graph.items[index % 3]), id: String(index + 1), uuid: "memory-" + (index + 1) }));
  state.owners[0].memories[0].isDocumentNode = true;
  state.owners[0].memories[0].documentPath = "owners/character%3Aoperit/resources/source.txt";
  state.owners[0].memories[0].chunkIndexFilePath = "owners/character%3Aoperit/resources/index.bin";
  state.owners[0].links = source.links;
  state.owners[0].chunks = [{ id: "1", memoryUuid: "memory-1", chunkIndex: 0, content: "Exact document chunk" }];
  state.owners[0].candidates = [{ id: "1", chatId: "core-chat", triggerMessageTimestamp: 987, triggerVariantIndex: 1, createdAt: 1, updatedAt: 2, status: "processing", attemptCount: 3, lastError: "original status", sourceType: "reply_finalized_auto" }];
  state.owners[0].embeddings = [{ id: "1", endpoint: "https://fixture.invalid", model: "model", text: "input", vector: [1, 0], updatedAt: 12 }];
  state.owners[0].rebuildTask = { id: "task-1", windows: [{ chatId: "core-chat", chatTitle: "Original chat", sourceMessageCount: 1, messages: [{ sender: "ai", content: "Original source", timestamp: 987, variantIndex: 1, variantCount: 3, provider: "original-provider", modelName: "original-model" }] }], nextWindow: 0 };
  Object.assign(state.owners[0].rebuildProgress, { status: "running", totalChats: 1, totalWindows: 1, totalSourceMessages: 1 });
  state.owners[2].memories = [structuredClone(source.graph.items[0])];
  state.owners[2].memories[0].documentPath = "owners/character%3Aoperit/resources/source.txt";
  disk.replaceState(state);
  for (const owner of state.owners) {
    const path = disk.directory.replaceAll("\\", "/") + "/" + owner.userDocumentPath;
    await disk.files.mkdir(path.slice(0, path.lastIndexOf("/")), true);
    await disk.files.write(path, "# Original USER " + owner.ownerKey + "\nUnicode 用户资料");
  }
  const root = disk.directory.replaceAll("\\", "/") + "/owners/character%3Aoperit/resources";
  await disk.files.mkdir(root, true);
  await disk.files.write(root + "/source.txt", "Original document 用户\u0000bytes");
  await disk.files.write(root + "/index.bin", "Exact index bytes");
  const raw = await disk.storage.dataStore.open({ path: disk.directory + "/chats.preferences.json" });
  await raw.edit(preferences => {
    preferences.set("binding_core-chat", { version: 1, selection: "card:operit" });
    preferences.set("draft_core-draft", { version: 1, selection: "group:group-one" });
  });
  await raw.close();
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; values.state.legacyImportPending = true; });
  return { disk, state, chat: chatHost() };
}

/** Preserves independently queued requests that map to the same exact revision during released-data import. */
test("released-data import retains repeated candidate sources with independent IDs and original states", async t => {
  const { disk, state, chat } = await seed(t), original = state.owners[0].candidates[0];
  state.owners[0].candidates.push({ ...original, id: "2", status: "failed", updatedAt: 3, attemptCount: 5, lastError: "Original extraction failure" });
  const expected = state.owners[0].candidates.map(({ id, ...record }) => record);
  disk.replaceState(state);
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; values.state.legacyImportPending = true; });
  await open(disk, chat);
  const saved = plain(disk.readState()), card = saved.cards.find(value => value.name === state.cards[0].name);
  const owner = saved.owners.find(value => value.ownerKey === "character:" + card.id);
  assert.equal(owner.candidates.length, 2);
  assert.equal(new Set(owner.candidates.map(value => value.id)).size, 2);
  for (const record of owner.candidates) assert.match(record.id, uuidPattern);
  assert.deepEqual(owner.candidates.map(({ id, ...record }) => record).sort((left, right) => left.attemptCount - right.attemptCount), expected);
  assert.equal(owner.memories.length, state.owners[0].memories.length);
  assert.equal(metadata(disk).legacyImportPending, undefined);
  const before = disk.stateBytes(); await open(disk, chat);
  assert.deepEqual(disk.stateBytes(), before);
});

test("TS released-data import into v1 converts every owned identity and reference and preserves user content", async t => {
  const { disk, state: old, chat } = await seed(t), database = await open(disk, chat), state = plain(disk.readState());
  assert.equal(state.version, 1); assert.equal(disk.storage.inspectPreferences("migration", values => values.plan), undefined);
  for (const collection of ["cards", "groups", "tags", "stores", "conversationGroups"])
    for (const record of state[collection]) assert.match(record.id, uuidPattern);
  const first = state.cards.find(card => card.name === old.cards[0].name), second = state.cards.find(card => card.name === old.cards[1].name);
  assert.equal(first.isDefault, undefined);
  for (const key of ["chatModelId", "chatProviderId", "ttsConfigId", "themeConfigId", "characterSetting", "createdAt", "updatedAt"]) assert.deepEqual(first[key], old.cards[0][key]);
  assert.deepEqual(first.attachedTagIds, [state.tags.find(tag => tag.name === old.tags[0].name).id]);
  assert.equal(first.sharedMemoryMounts[0].sharedMemoryId, state.stores[0].id);
  assert.equal(second.sharedMemoryId, state.stores[0].id);
  assert.deepEqual(state.groups[0].members.map(member => member.characterCardId), [first.id, second.id]);
  assert.deepEqual(state.active, { CharacterCard: { id: first.id } });
  assert.equal(state.conversationGroups[0].ownerSelection, "group:" + state.groups[0].id);
  assert.deepEqual(state.conversationGroups[0].chatIds, ["core-chat"]);
  assert.equal((await database.chats.data()).get("binding_core-chat").selection, "card:" + first.id);
  assert.equal((await database.chats.data()).get("draft_core-draft").selection, "group:" + state.groups[0].id);
  const owner = state.owners.find(owner => owner.ownerKey === "character:" + first.id);
  for (const collection of ["memories", "links", "chunks", "candidates", "embeddings"])
    for (const record of owner[collection]) assert.match(record.id, uuidPattern);
  for (const memory of owner.memories) {
    assert.match(memory.uuid, uuidPattern);
    for (const record of [...memory.tags, ...memory.properties]) assert.match(record.id, uuidPattern);
  }
  const document = owner.memories.find(memory => memory.isDocumentNode);
  assert.equal(owner.chunks[0].memoryUuid, document.uuid);
  assert.equal(owner.links[0].sourceMemoryId, document.id);
  assert.equal(owner.memories.some(memory => memory.id === owner.links[0].targetMemoryId), true);
  assert.match(owner.rebuildTask.id, uuidPattern); assert.equal(owner.rebuildTask.nextWindow, 0);
  assert.deepEqual({ ...owner.candidates[0], id: "1" }, old.owners[0].candidates[0]);
  const shared = state.owners.find(owner => owner.ownerKey === "shared:" + state.stores[0].id);
  assert.notEqual(shared.memories[0].id, document.id);
  assert.equal(shared.memories[0].documentPath, document.documentPath);
  assert.equal((await bytes(disk, document.documentPath)).toString(), "Original document 用户\u0000bytes");
  assert.equal((await bytes(disk, document.chunkIndexFilePath)).toString(), "Exact index bytes");
  assert.equal((await bytes(disk, owner.userDocumentPath)).toString(), "# Original USER character:operit\nUnicode 用户资料");
  assert.equal((await disk.files.exists(disk.directory + "/owners/character%3Aoperit")).exists, false);
  assert.deepEqual(chat.extensions.get(0), { participantId: first.id });
  assert.deepEqual(chat.extensions.get(1), { participantId: second.id });
  assert.equal(chat.extensions.get(2), null); assert.equal(chat.writes.length, 2);
  assert.deepEqual(chat.foreign, { participantId: "unrelated-plugin-identity" });
  const snapshotBytes = disk.stateBytes(); disk.clearCalls(); await open(disk, chat);
  assert.deepEqual(disk.stateBytes(), snapshotBytes); assert.equal(disk.calls.some(call => call.method === "storage.commit"), false);
});

test("released-data conversion commits bounded atomic key swaps and resumes with the persisted UUIDs", async t => {
  const { disk, chat } = await seed(t, 1100), originalOpen = disk.storage.objects.open;
  let batches = 0;
  const failure = new Error("Injected second record-page failure");
  /** Interrupts the actual generic transaction boundary after the first committed record page. */
  disk.storage.objects.open = async options => {
    const handle = await originalOpen(options), commit = handle.commit;
    return { ...handle,
      /** Rejects one record page without mutating SQLite or inventing a receipt. */
      async commit(mutations) {
        if (mutations.some(mutation => mutation.collection === "memories") && ++batches === 2) throw failure;
        return commit(mutations);
      },
    };
  };
  await assert.rejects(open(disk, chat), error => error === failure);
  const pending = disk.storage.inspectPreferences("migration", values => values.plan), identities = structuredClone(pending.identities);
  assert.equal(metadata(disk).version, 1); assert.equal(pending.phase, "records"); assert.ok(pending.cursor > 0);
  disk.storage.objects.open = originalOpen;
  await open(disk, chat);
  const state = plain(disk.readState()), cardId = identities[JSON.stringify(["cards", "operit"])];
  const owner = state.owners.find(owner => owner.ownerKey === "character:" + cardId);
  assert.equal(owner.memories.length, 1100);
  assert.ok(owner.memories.some(memory => memory.id === identities[JSON.stringify([owner.ownerKey + "/memories", "1"])]));
  assert.equal(chat.writes.length, 2);
  for (const call of disk.calls.filter(call => call.method === "storage.commit")) {
    const mutations = call.args[0]; assert.ok(mutations.length <= 1000);
    for (const mutation of mutations.filter(mutation => mutation.op === "delete" && mutation.collection === "memories"))
      assert.ok(mutations.some(other => other.op === "put" && other.collection === mutation.collection));
  }
});

test("owner copy interruption retains source bytes and resumes with the same UUID map", async t => {
  const { disk, chat } = await seed(t), failure = new Error("Injected owner copy failure");
  disk.failNext("copy", failure);
  await assert.rejects(open(disk, chat), error => error === failure);
  const pending = disk.storage.inspectPreferences("migration", values => values.plan), id = pending.identities[JSON.stringify(["cards", "operit"])];
  assert.equal(pending.phase, "files");
  assert.equal((await bytes(disk, "owners/character%3Aoperit/USER.md")).toString(), "# Original USER character:operit\nUnicode 用户资料");
  await open(disk, chat); assert.ok(disk.readState().cards.some(card => card.id === id));
});

test("conflicting owner destination bytes stop migration and retain the original records", async t => {
  const { disk, chat } = await seed(t);
  disk.failNext("copy", new Error("Pause after persisted plan")); await assert.rejects(open(disk, chat));
  const pending = disk.storage.inspectPreferences("migration", values => values.plan), file = pending.files[0], target = disk.directory.replaceAll("\\", "/") + "/" + file.destination;
  await disk.files.mkdir(target.slice(0, target.lastIndexOf("/")), true); await disk.files.write(target, "Conflicting data");
  await assert.rejects(open(disk, chat), /resource verification failed/);
  assert.equal(metadata(disk).version, 1);
  assert.notEqual((await bytes(disk, file.source)).toString(), "Conflicting data");
  assert.ok(disk.storage.inspectPreferences("cards", values => values.record_operit));
});

test("an interrupted message-author write resumes without rewriting the already converted revision", async t => {
  const { disk, chat } = await seed(t), originalWrite = chat.writeExtension;
  let writes = 0;
  /** Simulates a process interruption after a real extension write but before its progress marker. */
  chat.writeExtension = async (target, value) => {
    await originalWrite(target, value);
    if (++writes === 1) throw new Error("Author write response interrupted");
    return { extension: value };
  };
  await assert.rejects(open(disk, chat), /Author write response interrupted/);
  chat.writeExtension = originalWrite;
  await open(disk, chat); assert.equal(chat.writes.length, 2); assert.equal(disk.readState().version, 1);
});

for (const version of [0, 2, 3]) test("unsupported schema " + version + " fails before any mutation", async t => {
  const { disk, chat } = await seed(t);
  disk.storage.inspectPreferences("meta", values => { values.state.version = version; });
  disk.clearCalls(); await assert.rejects(open(disk, chat), /Unsupported character schema version/);
  assert.equal(disk.calls.some(call => call.method === "storage.commit"), false);
  assert.equal(chat.writes.length, 0);
});

test("existing UUIDs and the Core model selection survive a released-data import unchanged", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "a8f78f9b-3fe5-467a-8b54-3269b78a246a" });
  const state = plain(disk.readState()); state.version = 1; state.active = { Model: { id: "chat" } }; disk.replaceState(state);
  await open(disk, null);
  assert.deepEqual(plain(disk.readState()), { ...state, version: 1 });
  assert.equal(disk.calls.some(call => call.method === "copy" || call.method === "deleteFile"), false);
});

test("v1 storage adopts another released source using the same import boundary", async t => {
  const { disk, state, chat } = await seed(t), existingId = "42f09266-5477-4d3b-80e8-bf1fc3a34930";
  const existing = { ...structuredClone(state.cards[0]), id: existingId, name: "Existing UUID character", attachedTagIds: [], sharedMemoryMounts: [] };
  delete existing.isDefault;
  state.cards.push(existing);
  const owner = plain(loadModule("src/storage/state.ts").createMemorySpace("character:" + existingId));
  state.owners.push(owner); state.version = 1; disk.replaceState(state);
  const path = disk.directory.replaceAll("\\", "/") + "/" + owner.userDocumentPath;
  await disk.files.mkdir(path.slice(0, path.lastIndexOf("/")), true); await disk.files.write(path, "Existing v1 USER.md");
  disk.storage.inspectPreferences("meta", values => { values.state.legacyImportPending = true; });
  disk.failNext("copy", new Error("Interrupted legacy import conversion"));
  await assert.rejects(open(disk, chat), /Interrupted legacy import conversion/);
  assert.equal(metadata(disk).version, 1); assert.equal(metadata(disk).legacyImportPending, true);
  await open(disk, chat);
  const current = plain(disk.readState());
  assert.deepEqual(current.cards.find(card => card.id === existingId), existing);
  assert.equal(current.cards.length, 3);
  assert.equal(current.cards.some(card => card.id === "operit" || card.id === "travel"), false);
  assert.equal(metadata(disk).legacyImportPending, undefined);
  assert.equal((await bytes(disk, owner.userDocumentPath)).toString(), "Existing v1 USER.md");
});

test("file-copy response interruption verifies the existing destination without copying it twice", async t => {
  const { disk, chat } = await seed(t), originalCopy = disk.files.copy;
  let copies = 0;
  /** Interrupts after an actual file copy while its durable progress cursor still names that file. */
  disk.files.copy = async (...args) => {
    await originalCopy(...args);
    if (++copies === 1) throw new Error("Copy response interrupted");
    return { successful: true, details: "Native copy completed" };
  };
  await assert.rejects(open(disk, chat), /Copy response interrupted/);
  const plan = disk.storage.inspectPreferences("migration", values => values.plan), file = plan.files[0]; assert.equal(plan.cursor, 0);
  assert.deepEqual(await bytes(disk, file.source), await bytes(disk, file.destination));
  disk.files.copy = originalCopy; disk.clearCalls(); await open(disk, chat);
  assert.equal(disk.calls.some(call => call.method === "copy" && call.args[0].endsWith("/" + file.source)), false);
});

test("cleanup response interruption resumes after the already retired source owner directory", async t => {
  const { disk, chat } = await seed(t), originalDelete = disk.files.deleteFile;
  /** Interrupts after a real verified source directory removal and before its progress commit. */
  disk.files.deleteFile = async (...args) => {
    await originalDelete(...args); throw new Error("Cleanup response interrupted");
  };
  await assert.rejects(open(disk, chat), /Cleanup response interrupted/);
  assert.equal(disk.storage.inspectPreferences("migration", values => values.plan).phase, "cleanup");
  disk.files.deleteFile = originalDelete; await open(disk, chat);
  assert.equal(disk.readState().version, 1); assert.equal(disk.storage.inspectPreferences("migration", values => values.plan), undefined);
});

/** Keeps released cards usable after one of their group's members was deleted. */
test("released-data migration removes a deleted group member and retains its group", async t => {
  const { disk, state, chat } = await seed(t);
  state.groups[0].members[0].characterCardId = "missing-character"; disk.replaceState(state);
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; values.state.legacyImportPending = true; });
  await open(disk, chat);
  const saved = plain(disk.readState());
  assert.equal(saved.groups.length, state.groups.length);
  assert.equal(saved.groups[0].members.length, state.groups[0].members.length - 1);
  assert.equal(saved.cards.length, state.cards.length);
  assert.deepEqual(
    saved.owners.flatMap(owner => owner.memories.map(memory => memory.content)).sort(),
    state.owners.flatMap(owner => owner.memories.map(memory => memory.content)).sort(),
  );
  assert.equal(metadata(disk).legacyImportPending, undefined);
});

test("malformed persisted migration work fails before filesystem or record changes", async t => {
  const { disk, chat } = await seed(t);
  disk.storage.inspectPreferences("migration", values => { values.plan = { from: "released", to: 1, phase: "records", cursor: 99, identities: {}, files: [], directories: [], authors: [], changes: [] }; });
  disk.clearCalls(); await assert.rejects(open(disk, chat), /cursor exceeds its work/);
  assert.equal(disk.calls.some(call => call.method === "storage.commit" || call.method === "deleteFile"), false);
});

/** Completes pending released-data adoption while preserving an unresolved binding for the installed editor. */
test("pending released-data adoption is not blocked by an unknown model binding", async t => {
  const { disk, chat } = await seed(t);
  disk.storage.inspectPreferences("cards", values => Object.assign(values.record_operit, {
    chatModelBindingMode: "UNRECOGNIZED_MODE", chatProviderId: null, chatModelId: null,
  }));
  await open(disk, chat);
  const state = plain(disk.readState()), card = state.cards.find(value => value.name === "Operit");
  assert.match(card.id, uuidPattern);
  assert.equal(card.chatModelBindingMode, "UNRECOGNIZED_MODE");
  assert.equal(card.chatProviderId, null); assert.equal(card.chatModelId, null);
  assert.equal(metadata(disk).legacyImportPending, undefined);
  const before = disk.stateBytes(); await open(disk, chat);
  assert.deepEqual(disk.stateBytes(), before);
});

/** Keeps unresolved primary selections exact while converting actual libraries and their references. */
test("released-data import preserves missing primary libraries through UUID adoption and reopen", async t => {
  for (const mode of ["SHARED", "CHARACTER"]) {
    for (const sharedId of [null, "", "  ", "deleted-store"]) {
      const { disk, state, chat } = await seed(t);
      disk.storage.inspectPreferences("cards", values => Object.assign(values.record_operit, {
        memoryBindingMode: mode, sharedMemoryId: sharedId,
      }));
      await open(disk, chat);
      const saved = plain(disk.readState()), card = saved.cards.find(value => value.name === "Operit");
      const other = saved.cards.find(value => value.name === state.cards[1].name);
      assert.match(card.id, uuidPattern);
      assert.equal(card.memoryBindingMode, mode); assert.equal(card.sharedMemoryId, sharedId);
      assert.equal(saved.stores.length, state.stores.length);
      assert.match(saved.stores[0].id, uuidPattern);
      assert.equal(other.sharedMemoryId, saved.stores[0].id);
      assert.equal(card.sharedMemoryMounts[0].sharedMemoryId, saved.stores[0].id);
      const owner = saved.owners.find(value => value.ownerKey === "character:" + card.id);
      assert.equal(owner.memories.length, state.owners[0].memories.length);
      assert.equal((await bytes(disk, owner.userDocumentPath)).toString(), "# Original USER character:operit\nUnicode 用户资料");
      assert.equal(metadata(disk).legacyImportPending, undefined);
      const before = disk.stateBytes(); disk.clearCalls(); await open(disk, chat);
      assert.deepEqual(disk.stateBytes(), before);
      assert.equal(disk.calls.some(call => call.method === "storage.commit" || call.method === "storage.preferences.edit"), false);
    }
  }
});

/** Preserves historical authors after their character is deleted while converting surviving authors. */
test("released-data audit retains a deleted message author during UUID adoption", async t => {
  const { disk, chat } = await seed(t);
  chat.extensions.set(2, { participantId: "deleted-character" });
  await open(disk, chat);
  const saved = plain(disk.readState());
  assert.deepEqual(chat.extensions.get(2), { participantId: "deleted-character" });
  assert.deepEqual(chat.extensions.get(0), { participantId: saved.cards.find(card => card.name === "Operit").id });
  assert.equal(chat.writes.length, 2);
  assert.equal(metadata(disk).legacyImportPending, undefined);
  const before = disk.stateBytes(); await open(disk, chat);
  assert.deepEqual(disk.stateBytes(), before); assert.equal(chat.writes.length, 2);
});

/** Retains explicit missing targets in chat bindings, drafts and archived conversation-group scopes. */
test("released-data audit retains deleted chat targets without replacing their selection", async t => {
  for (const selection of ["card:deleted-character", "group:deleted-group"]) {
    const { disk, chat } = await seed(t);
    disk.storage.inspectPreferences("conversationGroups", values => { values["record_manual-1"].ownerSelection = selection; });
    const handle = await disk.storage.dataStore.open({ path: disk.directory + "/chats.preferences.json" });
    await handle.edit(preferences => {
      preferences.set("binding_core-chat", { version: 1, selection });
      preferences.set("draft_core-draft", { version: 1, selection });
    });
    await handle.close();
    const database = await open(disk, chat);
    assert.equal((await database.chats.data()).get("binding_core-chat").selection, selection);
    assert.equal((await database.chats.data()).get("draft_core-draft").selection, selection);
    assert.equal(plain(disk.readState()).conversationGroups[0].ownerSelection, selection);
    assert.equal(metadata(disk).legacyImportPending, undefined);
    const before = disk.stateBytes(); await open(disk, chat);
    assert.deepEqual(disk.stateBytes(), before);
  }
});

/** Keeps the original unavailable active identity without selecting another role or model. */
test("released-data audit retains a deleted active character or group", async t => {
  for (const active of [{ CharacterCard: { id: "deleted-character" } }, { CharacterGroup: { id: "deleted-group" } }]) {
    const { disk, chat } = await seed(t);
    disk.storage.inspectPreferences("meta", values => { values.state.active = structuredClone(active); });
    await open(disk, chat);
    assert.deepEqual(plain(disk.readState()).active, active);
    assert.deepEqual(metadata(disk).active, active);
    assert.equal(metadata(disk).legacyImportPending, undefined);
    const before = disk.stateBytes(); await open(disk, chat);
    assert.deepEqual(disk.stateBytes(), before);
  }
});

/** Keeps host-owned opaque section tokens out of the character identity conversion. */
test("released-data audit retains an opaque conversation-group scope token", async t => {
  const { disk, chat } = await seed(t), scope = "section-scope-opaque-token";
  disk.storage.inspectPreferences("conversationGroups", values => { values["record_manual-1"].ownerSelection = scope; });
  await open(disk, chat);
  assert.equal(plain(disk.readState()).conversationGroups[0].ownerSelection, scope);
  assert.equal(metadata(disk).legacyImportPending, undefined);
});

/** Rejects malformed chat tokens before publishing a migration plan or changing owner files. */
test("released-data audit still rejects malformed chat selections before migration writes", async t => {
  for (const selection of ["card:", "group:", "model:other", "unknown:target"]) {
    const { disk, chat } = await seed(t);
    const handle = await disk.storage.dataStore.open({ path: disk.directory + "/chats.preferences.json" });
    await handle.edit(preferences => { preferences.set("binding_core-chat", { version: 1, selection }); });
    await handle.close(); disk.clearCalls();
    await assert.rejects(open(disk, chat), /Invalid.*selection/);
    assert.equal(disk.storage.inspectPreferences("migration", values => values.plan), undefined);
    assert.equal(disk.calls.some(call => ["storage.commit", "copy", "deleteFile"].includes(call.method)), false);
  }
});

/** Keeps absolute Files API resource references aligned with the verified owner-directory move. */
test("released-data audit rewrites absolute document paths before retiring the old owner", async t => {
  const { disk, state, chat } = await seed(t);
  const directory = disk.globals.ToolPkg.getSpaceDataDir();
  const source = state.owners[0].memories[0];
  const oldDocument = directory + "/" + source.documentPath;
  const oldIndex = directory + "/" + source.chunkIndexFilePath;
  source.documentPath = oldDocument; source.chunkIndexFilePath = oldIndex;
  const external = directory + "/external-document.txt";
  await disk.files.write(external, "Externally owned document");
  state.owners[0].memories[1].documentPath = external;
  disk.replaceState(state);
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; values.state.legacyImportPending = true; });
  await open(disk, chat);
  const saved = plain(disk.readState()), card = saved.cards.find(value => value.name === "Operit");
  const owner = saved.owners.find(value => value.ownerKey === "character:" + card.id);
  const document = owner.memories.find(memory => memory.isDocumentNode);
  assert.equal((await disk.files.exists(document.documentPath)).exists, true);
  assert.equal((await disk.files.exists(document.chunkIndexFilePath)).exists, true);
  assert.equal((await disk.readBytes(document.documentPath)).toString(), "Original document 用户\u0000bytes");
  assert.equal((await disk.readBytes(document.chunkIndexFilePath)).toString(), "Exact index bytes");
  assert.equal(document.documentPath, directory + "/owners/" + encodeURIComponent(owner.ownerKey) + "/resources/source.txt");
  assert.equal((await disk.files.exists(oldDocument)).exists, false);
  assert.equal((await disk.files.exists(oldIndex)).exists, false);
  assert.equal(owner.memories.filter(memory => memory.documentPath === external).length, 1);
  assert.equal((await disk.readBytes(external)).toString(), "Externally owned document");
  const before = disk.stateBytes(); await open(disk, chat);
  assert.deepEqual(disk.stateBytes(), before);
});

/** Repairs stale released lists before UUID conversion while retaining the exact missing primary binding. */
test("released-data migration cleans deleted references without losing cards or memory text", async t => {
  const { disk, state, chat } = await seed(t);
  state.cards[0].attachedTagIds.push("deleted-tag", "");
  state.cards[0].sharedMemoryMounts.push({ sharedMemoryId: "deleted-store", readable: true, writable: true });
  state.cards[0].memoryBindingMode = "SHARED"; state.cards[0].sharedMemoryId = "deleted-primary";
  const owner = state.owners[0];
  owner.links.push({ ...owner.links[0], id: "stale-link", targetMemoryId: "deleted-memory" });
  owner.chunks.push({ ...owner.chunks[0], id: "stale-chunk", memoryUuid: "deleted-document" });
  disk.replaceState(state);
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; values.state.legacyImportPending = true; });
  await open(disk, chat);
  const saved = plain(disk.readState()), card = saved.cards.find(card => card.name === state.cards[0].name);
  const memory = saved.owners.find(owner => owner.ownerKey === "character:" + card.id);
  assert.equal(card.sharedMemoryId, "deleted-primary"); assert.equal(card.memoryBindingMode, "SHARED");
  assert.equal(card.attachedTagIds.length, state.cards[0].attachedTagIds.length - 2);
  assert.equal(card.sharedMemoryMounts.length, state.cards[0].sharedMemoryMounts.length - 1);
  assert.equal(memory.links.length, owner.links.length - 1); assert.equal(memory.chunks.length, owner.chunks.length - 1);
  assert.deepEqual(memory.memories.map(memory => memory.content).sort(), owner.memories.map(memory => memory.content).sort());
  const before = disk.stateBytes(); await open(disk, chat); assert.deepEqual(disk.stateBytes(), before);
});

/** Retains self relationships that were accepted and saved by the released repository. */
test("released-data migration preserves a historical self relationship", async t => {
  const { disk, state, chat } = await seed(t);
  state.owners[0].links[0].targetMemoryId = state.owners[0].links[0].sourceMemoryId;
  disk.replaceState(state);
  disk.storage.inspectPreferences("meta", values => { values.__operit_preferences_schema_version = 0; values.state.legacyImportPending = true; });
  await open(disk, chat);
  const saved = plain(disk.readState()), card = saved.cards.find(card => card.name === state.cards[0].name);
  const owner = saved.owners.find(owner => owner.ownerKey === "character:" + card.id);
  assert.equal(owner.links.length, state.owners[0].links.length);
  assert.equal(owner.links[0].sourceMemoryId, owner.links[0].targetMemoryId);
  const before = disk.stateBytes(); await open(disk, chat); assert.deepEqual(disk.stateBytes(), before);
});

/** Keeps memory text when an optional document or its index no longer exists. */
test("released-data migration clears missing attachment paths and preserves memory content", async t => {
  const { disk, state, chat } = await seed(t), root = disk.globals.ToolPkg.getSpaceDataDir();
  await disk.files.deleteFile(root + "/owners/character%3Aoperit/resources/source.txt", false);
  await disk.files.deleteFile(root + "/owners/character%3Aoperit/resources/index.bin", false);
  await open(disk, chat);
  const saved = plain(disk.readState()), card = saved.cards.find(card => card.name === state.cards[0].name);
  const owner = saved.owners.find(owner => owner.ownerKey === "character:" + card.id);
  assert.deepEqual(owner.memories.map(memory => memory.content).sort(), state.owners[0].memories.map(memory => memory.content).sort());
  const document = owner.memories.find(memory => memory.isDocumentNode);
  assert.equal(document.documentPath, null); assert.equal(document.chunkIndexFilePath, null);
  const before = disk.stateBytes(); await open(disk, chat); assert.deepEqual(disk.stateBytes(), before);
});

/** Starts a fresh editable owner document after the old file was lost. */
test("released-data migration recreates a missing USER document without changing stored memory text", async t => {
  for (const missingDirectory of [false, true]) {
    const { disk, state, chat } = await seed(t), owner = state.owners[1], root = disk.globals.ToolPkg.getSpaceDataDir();
    await disk.files.deleteFile(root + "/" + (missingDirectory ? owner.userDocumentPath.slice(0, -8) : owner.userDocumentPath), missingDirectory);
    await open(disk, chat);
    const saved = plain(disk.readState()), card = saved.cards.find(card => card.name === state.cards[1].name);
    const memory = saved.owners.find(owner => owner.ownerKey === "character:" + card.id);
    assert.equal((await bytes(disk, memory.userDocumentPath)).toString(), "");
    assert.deepEqual(memory.memories.map(memory => memory.content).sort(), owner.memories.map(memory => memory.content).sort());
    const before = disk.stateBytes(); await open(disk, chat); assert.deepEqual(disk.stateBytes(), before);
  }
});

/** Reproduces partial released cards before the UUID migration without losing their surviving content. */
test("released-data migration completes missing character fields before reading references", async t => {
  const { disk, state, chat } = await seed(t);
  const original = structuredClone(state.cards[0]);
  disk.storage.inspectPreferences("cards", values => {
    const card = values.record_operit;
    for (const field of ["createdAt", "updatedAt", "avatarUri", "themeConfigId", "memoryBindingMode", "attachedTagIds", "sharedMemoryMounts"]) delete card[field];
    card.toolAccessConfig = { enabled: true, allowedPackages: ["retained-package"] };
  });
  await open(disk, chat);
  const migrated = plain(disk.readState());
  const card = migrated.cards.find(card => card.name === original.name);
  assert.equal(migrated.cards.length, state.cards.length);
  assert.equal(card.characterSetting, original.characterSetting);
  assert.equal(card.openingStatement, original.openingStatement);
  assert.equal(card.chatModelId, original.chatModelId);
  assert.equal(card.createdAt, 0); assert.equal(card.updatedAt, 0);
  assert.equal(card.avatarUri, null); assert.equal(card.themeConfigId, null);
  assert.equal(card.memoryBindingMode, "CHARACTER");
  assert.deepEqual(card.attachedTagIds, []); assert.deepEqual(card.sharedMemoryMounts, []);
  assert.deepEqual(card.toolAccessConfig.allowedPackages, ["retained-package"]);
  assert.deepEqual(card.toolAccessConfig.allowedSkills, []);
  assert.deepEqual(migrated.owners.flatMap(owner => owner.memories.map(memory => memory.content)).sort(),
    state.owners.flatMap(owner => owner.memories.map(memory => memory.content)).sort());
  assert.equal(metadata(disk).legacyImportPending, undefined);
  const before = disk.stateBytes(); await open(disk, chat);
  assert.deepEqual(disk.stateBytes(), before);
});
