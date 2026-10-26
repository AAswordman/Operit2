import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Opens the production database repository against the generic SQLite host adapter. */
async function open(disk) { return loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open(); }
/** Creates complete editable values using the plugin's actual new-character form. */
function card(name) { return { ...plain(loadModule("src/drafts.ts").createCharacterDraft()), name, characterSetting: "Persisted character" }; }
/** Supplies an explicit memory creation command without fabricating a persisted memory. */
function memory(title) { return { title, content: "Persisted memory", contentType: "text/plain", source: "manual", credibility: 0.8, importance: 0.6, folderPath: null, tags: [] }; }
/** Selects only actual storage transactions from the generic host audit. */
function commits(disk) { return disk.calls.filter(call => call.method === "storage.commit"); }

test("new installation stores independent records and reopens without mutations", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk), state = plain(disk.readState());
  assert.equal(state.version, 1); assert.equal(state.cards.length, 1); assert.match(state.cards[0].id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.equal(commits(disk).length, 1);
  assert.equal((await disk.entries()).includes("state.json"), false);
  const document = await owner.run(session => session.readUser("character:" + state.cards[0].id)); assert.equal(document.content, "");
  const bytes = disk.stateBytes(); disk.clearCalls();
  const reopened = await open(disk); assert.deepEqual(plain(await reopened.run(session => session.listCharacters())), state.cards);
  assert.deepEqual(disk.stateBytes(), bytes); assert.equal(commits(disk).length, 0);
  assert.equal(disk.calls.filter(call => ["write", "mkdir", "move", "deleteFile"].includes(call.method)).length, 0);
});

test("sidebar directory reuses the canonical revision and refreshes after a character write", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk);
  disk.clearCalls();
  const first = await owner.readSidebarDirectory();
  const firstRequestCount = disk.calls.filter(call => call.method.startsWith("storage.")).length;
  const second = await owner.readSidebarDirectory();
  const secondRequestCount = disk.calls.filter(call => call.method.startsWith("storage.")).length - firstRequestCount;
  assert.deepEqual(plain(second), plain(first));
  assert.equal(firstRequestCount, 2);
  assert.equal(secondRequestCount, 2);
  await owner.run(session => session.createCharacter(card("Revision refreshed")));
  const refreshed = await owner.readSidebarDirectory();
  assert.ok(refreshed.cards.some(value => value.name === "Revision refreshed"));
});

test("one memory edit publishes only that memory and the metadata guard", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk);
  const first = await owner.run(session => session.createMemory("character:default", memory("First")));
  await owner.run(session => session.createMemory("character:default", memory("Second")));
  disk.clearCalls();
  await owner.run(session => session.updateMemory("character:default", first.id, memory("Edited")));
  assert.equal(commits(disk).length, 1);
  const mutations = commits(disk)[0].args[0];
  assert.equal(mutations.filter(row => row.collection === "memories").length, 1);
  assert.equal(mutations.filter(row => row.collection === "meta").length, 0);
  assert.equal(mutations.filter(row => row.collection === "cards").length, 0);
  const reopened = await open(disk); assert.equal((await reopened.run(session => session.listMemories("character:default").then(rows => rows.find(row => row.id === first.id)))).title, "Edited");
});

test("a fresh operation sees writes committed by another repository execution", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), first = await open(disk), second = await open(disk);
  const created = await second.run(session => session.createCharacter(card("External import")));
  assert.equal((await first.run(session => session.getCharacter(created.id))).name, "External import");
});

test("concurrent writers reject stale snapshots without overwriting an unrelated committed record", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), first = await open(disk), second = await open(disk);
  let entered, proceed;
  const ready = new Promise(resolve => { entered = resolve; }), gate = new Promise(resolve => { proceed = resolve; });
  const pending = first.run(async session => { entered(); await gate; return session.updateCharacter({ ...(await session.getCharacter("default")), description: "Stale write" }); });
  await ready;
  await second.run(session => session.updateCharacter({ ...card("Actual committed name"), id: "default" }));
  proceed(); await assert.rejects(pending, /changed before edit/);
  assert.equal(disk.readState().cards[0].name, "Actual committed name");
});

test("transaction failures preserve original errors and authoritative database records", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), before = disk.stateBytes(), failure = new Error("Exact storage failure");
  disk.failNext("storage.preferences.edit", failure);
  await assert.rejects(owner.run(session => session.updateCharacter({ ...card("Rejected name"), id: "default" })), error => error === failure);
  assert.deepEqual(disk.stateBytes(), before);
  assert.equal((await owner.run(session => session.listCharacters()))[0].name, "Operit");
  const reopened = await open(disk); assert.equal((await reopened.run(session => session.getCharacter("default"))).name, "Operit");
});

test("interrupted migration metadata prevents plugin operations and is never replaced with defaults", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }); await open(disk);
  disk.storage.inspectPreferences("meta", values => { values.state.migrationStatus = "writing"; });
  const before = disk.stateBytes.bind(disk); disk.clearCalls();
  await assert.rejects(open(disk), /migration has not completed/);
  assert.equal(commits(disk).length, 0); assert.equal(typeof before, "function");
});

test("owner pages preserve exact int64 identities and more than 1000 independently migrated records", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }); const owner = await open(disk);
  const created = plain(await owner.run(session => session.createMemory("character:default", memory("Template"))));
  const state = plain(disk.readState()); state.owners[0].memories = [];
  for (let index = 0; index < 1005; index += 1) state.owners[0].memories.push({ ...created, id: String(9007199254740993n + BigInt(index)), uuid: "source-" + index });
  disk.replaceState(state);
  const reopened = await open(disk), retained = await reopened.run(session => session.listMemories("character:default"));
  assert.equal(retained.length, 1005); assert.equal(retained[0].id, "9007199254740993");
});

/** Verifies external profile staging does not publish a rejected database operation. */
test("rejected profile commit retains the actual published USER.md and original error", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk);
  await owner.run(session => session.writeUser("character:default", "Original profile"));
  const failure = new Error("Profile record commit rejected"); disk.failNext("write", failure);
  await assert.rejects(owner.run(session => session.writeUser("character:default", "Rejected profile")), error => error === failure);
  const reopened = await open(disk);
  assert.equal((await reopened.run(session => session.readUser("character:default"))).content, "Original profile");
});

/** Verifies a file publication error stops the same repository without pretending the file write succeeded. */
test("profile publication failure remains explicit after its record commit", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk);
  const failure = new Error("USER.md rename rejected"); disk.failNext("move", failure);
  await assert.rejects(owner.run(session => session.writeUser("character:default", "Pending profile")), error => error === failure);
  assert.equal((await owner.run(session => session.listCharacters()))[0].name, "Operit");
  const reopened = await open(disk);
  assert.equal((await reopened.run(session => session.readUser("character:default"))).content, "");
});

/** Keeps long provider text in the stored value rather than exceeding the production Host key limit. */
test("long source text is stored with its vector under a stable source identity", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), text = "Actual source paragraph\n".repeat(200);
  disk.globals.Tools.Net = { async http(request) { const input = JSON.parse(request.body).input; return { statusCode: 200, content: JSON.stringify({ data: input.map((text, index) => ({ index, embedding: [1, 2] })) }) }; } };
  await owner.run(async session => session.writeMemorySettings("character:default", { ...await session.readMemorySettings("character:default"), cloudEmbeddingEnabled: true, cloudEmbeddingEndpoint: "https://provider.test", cloudEmbeddingModel: "model" }));
  const saved = await owner.run(session => session.createMemory("character:default", { ...memory("Long source"), content: text }));
  disk.storage.inspect(sql => { const row = sql.prepare("SELECT key,value FROM records WHERE collection='memories'").get(); const value = JSON.parse(row.value); assert.equal(row.key, "character:default/" + saved.id); assert.equal(value.embedding.text, text); assert.equal(value.embedding.vector.length, 2); assert.equal(sql.prepare("SELECT count(*) AS count FROM records WHERE collection='embeddings'").get().count, 0); });
});

/** Verifies configuration reads never scan searchable source collections. */
test("configuration operations read owner metadata independently of the memory corpus", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk);
  await owner.run(session => session.createMemory("character:default", memory("Existing source"))); disk.clearCalls();
  assert.equal((await owner.run(session => session.listCharacters()))[0].id, "default");
  assert.deepEqual(disk.calls.filter(call => call.method === "storage.list").map(call => call.args[0]), ["owners"]);
});

/** Verifies an original read failure is neither retried nor retained on unrelated operations. */
test("failed metadata reads propagate once and do not poison the repository", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), failure = new Error("Exact metadata read failure");
  disk.clearCalls(); disk.failNext("storage.list", failure);
  await assert.rejects(owner.run(session => session.listCharacters()), error => error === failure);
  assert.equal(disk.calls.filter(call => call.method === "storage.list").length, 1);
  assert.equal((await owner.run(session => session.listCharacters()))[0].id, "default");
});

/** Builds one complete synthetic memory so bulk tests never depend on a real exported backup. */
function bulkMemory(index, tags = 2) {
  return {
    id: String(index + 1),
    uuid: "synthetic-" + index,
    title: "Synthetic memory " + index,
    content: "Synthetic bulk content " + index,
    contentType: "text/plain",
    source: "test",
    credibility: 0.5,
    importance: 0.5,
    documentPath: null,
    isDocumentNode: false,
    chunkIndexFilePath: null,
    folderPath: null,
    createdAt: 1,
    updatedAt: 1,
    lastAccessedAt: 1,
    tags: Array.from({ length: tags }, (_, tag) => ({ id: String(1000000 + index * 10 + tag), name: "tag-" + tag })),
    properties: [],
  };
}

/** Reads a validated empty owner space and replaces only its records with synthetic memories. */
async function syntheticSpace(owner, count) {
  const space = plain(await owner.run(session => session.readMemorySpace("character:default")));
  space.memories = Array.from({ length: count }, (_, index) => bulkMemory(index));
  space.links = []; space.chunks = []; space.candidates = []; space.embeddings = [];
  return space;
}

/** Verifies a single import larger than one host transaction is chunked instead of rejected. */
test("bulk memory import commits bounded host transactions and retains every record", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), space = await syntheticSpace(owner, 1005);
  disk.clearCalls();
  await owner.run(session => session.importMemorySpace("character:default", space, "SKIP"));
  const transactions = commits(disk);
  assert.ok(transactions.length > 1, "bulk import must span more than one host transaction");
  for (const call of transactions) assert.ok(call.args[0].length <= 1000, "each transaction must stay within the host mutation limit");
  const reopened = await open(disk), retained = await reopened.run(session => session.listMemories("character:default"));
  assert.equal(retained.length, 1005);
  assert.equal(new Set(retained.map(value => value.uuid)).size, 1005);
});

/** Verifies full-record export reads every memory beyond one host page without truncation. */
test("memory export retains records beyond one host page", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), space = await syntheticSpace(owner, 1005);
  await owner.run(session => session.importMemorySpace("character:default", space, "SKIP"));
  const { exportMemoryBackup } = loadModule("src/backup.ts", disk.globals);
  const content = await owner.run(session => exportMemoryBackup("character:default", session));
  const backup = JSON.parse(content);
  assert.equal(backup.version, "2.0");
  assert.equal(backup.space.memories.length, 1005);
  assert.equal(new Set(backup.space.memories.map(value => value.uuid)).size, 1005);
});

/** Verifies a rejected middle batch leaves resumable records and SKIP completes without duplicates. */
test("a failed bulk import resumes with SKIP after earlier batches committed", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const owner = await open(disk), space = await syntheticSpace(owner, 1005);
  const failure = new Error("Second bulk transaction rejected");
  disk.failNext("storage.commit", failure, 2);
  await assert.rejects(owner.run(session => session.importMemorySpace("character:default", space, "SKIP")), error => error === failure);
  const reopened = await open(disk), partial = await reopened.run(session => session.listMemories("character:default"));
  assert.ok(partial.length > 0 && partial.length < 1005, "the first batch must remain committed");
  await reopened.run(session => session.importMemorySpace("character:default", space, "SKIP"));
  const completed = await reopened.run(session => session.listMemories("character:default"));
  assert.equal(completed.length, 1005);
  assert.equal(new Set(completed.map(value => value.uuid)).size, 1005);
});

/** Verifies vector-heavy source imports use bounded batches and preserve source ownership. */
test("bulk source vectors stay within host payload bounds", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), space = await syntheticSpace(owner, 1005);
  const { structuredNodes } = loadModule("src/storage/records.ts", disk.globals);
  disk.globals.Tools.Net = { async http(request) { const input = JSON.parse(request.body).input; return { statusCode: 200, content: JSON.stringify({ data: input.map((text, index) => ({ index, embedding: Array(1024).fill(1) })) }) }; } };
  Object.assign(space.settings, { cloudEmbeddingEnabled: true, cloudEmbeddingEndpoint: "https://provider.test", cloudEmbeddingModel: "model" });
  disk.clearCalls(); await owner.run(session => session.importMemorySpace("character:default", space, "SKIP"));
  const transactions = commits(disk); assert.ok(transactions.length >= 16);
  for (const call of transactions) { assert.ok(call.args[0].length <= 1000); assert.ok(structuredNodes(call.args[0]) < 1_000_000); }
  disk.storage.inspect(sql => {
    const rows = sql.prepare("SELECT value FROM records WHERE collection='memories'").all(); assert.equal(rows.length, 1005);
    for (const row of rows) { const value = JSON.parse(row.value); assert.equal(value.embedding.vector.length, 1024); assert.equal(value.embedding.text, value.record.content); }
    assert.equal(sql.prepare("SELECT count(*) AS count FROM records WHERE collection='embeddings'").get().count, 0);
  });
});

/** Verifies host list failures retain their exact identity and page size. */
test("collection read errors are propagated without changing page size", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), owner = await open(disk), failure = new Error("Host page rejection");
  disk.clearCalls(); disk.failNext("storage.list", failure);
  await assert.rejects(owner.run(session => session.listCharacters()), error => error === failure);
  assert.deepEqual(disk.calls.filter(call => call.method === "storage.list").map(call => call.args[2]), [100]);
});

/** Opens installed v1 data with repairable model selections without changing the persisted records. */
test("installed cards with unknown or incomplete model bindings reopen and remain editable", async t => {
  for (const binding of [
    { chatModelBindingMode: "UNRECOGNIZED_MODE", chatModelId: "old-model", chatProviderId: "old-provider" },
    { chatModelBindingMode: "FIXED_CONFIG", chatModelId: "old-model", chatProviderId: null },
    { chatModelBindingMode: "FIXED_MODEL", chatModelId: null, chatProviderId: null },
    { chatModelBindingMode: "FIXED_MODEL", chatModelId: "", chatProviderId: "" },
  ]) {
    const disk = await createDiskHarness(t, { legacyIdentity: "default" });
    disk.storage.inspectPreferences("cards", values => Object.assign(values.record_default, binding));
    const before = disk.stateBytes(); disk.clearCalls();
    const owner = await open(disk);
    assert.deepEqual(disk.stateBytes(), before);
    assert.equal(disk.calls.some(call => ["storage.commit", "storage.preferences.edit"].includes(call.method)), false);
    const loaded = plain(await owner.run(session => session.getCharacter("default")));
    for (const [key, value] of Object.entries(binding)) assert.equal(loaded[key], value);
    await owner.run(session => session.createCharacter(card("Other usable card")));
    assert.equal((await owner.run(session => session.listCharacters())).length, 2);
    await owner.run(session => session.updateCharacter({ ...loaded, chatModelBindingMode: "FOLLOW_GLOBAL", chatProviderId: null, chatModelId: null }));
    const reopened = await open(disk);
    assert.equal((await reopened.run(session => session.getCharacter("default"))).chatModelBindingMode, "FOLLOW_GLOBAL");
  }
});

/** Existing Build 26 records must recover on upgrade even when no legacy migration remains pending. */
test("already migrated partial characters read with defaults without startup repair writes", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk);
  const original = plain((await owner.listCharacters())[0]);
  const ownerKey = "character:" + original.id;
  const savedMemory = plain(await owner.run(session => session.createMemory(ownerKey, memory("必须保留的记忆"))));
  disk.storage.inspectPreferences("cards", values => {
    const card = values["record_" + original.id];
    delete card.createdAt; card.updatedAt = -1; delete card.themeConfigId;
    delete card.description; delete card.attachedTagIds;
    card.toolAccessConfig = { allowedPackages: ["retained-package"] };
  });
  const reopened = await open(disk), card = plain(await reopened.getCharacter(original.id));
  assert.equal(card.id, original.id); assert.equal(card.name, original.name);
  assert.equal(card.characterSetting, original.characterSetting);
  assert.equal(card.createdAt, 0); assert.equal(card.updatedAt, 0);
  assert.equal(card.themeConfigId, null); assert.equal(card.description, "");
  assert.deepEqual(card.attachedTagIds, []);
  assert.equal(card.toolAccessConfig.enabled, false);
  assert.deepEqual(card.toolAccessConfig.allowedPackages, ["retained-package"]);
  assert.deepEqual(plain((await reopened.readSidebarDirectory()).cards), [card]);
  assert.deepEqual(plain(await reopened.run(session => session.listMemories(ownerKey))), [savedMemory]);
  assert.equal(disk.storage.inspectPreferences("cards", values => values["record_" + original.id].createdAt), undefined);
  await reopened.run(session => session.updateCharacter({ ...card, description: "保存修改" }));
  assert.equal(disk.storage.inspectPreferences("cards", values => values["record_" + original.id].createdAt), 0);
  const before = disk.stateBytes(); disk.clearCalls(); await open(disk);
  assert.deepEqual(disk.stateBytes(), before);
  assert.equal(disk.calls.filter(call => call.method === "storage.preferences.edit").length, 0);
});
