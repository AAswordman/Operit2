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
function commits(disk) { return disk.calls.filter(call => call.method === "storage.transaction"); }

test("new installation stores independent records and reopens without mutations", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk), state = plain(disk.readState());
  assert.equal(state.version, 3); assert.equal(state.cards.length, 1); assert.equal(state.cards[0].id, "default");
  assert.equal(commits(disk).length, 1);
  assert.equal((await disk.entries()).includes("state.json"), false);
  const document = await owner.run(session => session.readUser("character:default")); assert.equal(document.content, "");
  const bytes = disk.stateBytes(); disk.clearCalls();
  const reopened = await open(disk); assert.deepEqual(plain(await reopened.run(session => session.listCharacters())), state.cards);
  assert.deepEqual(disk.stateBytes(), bytes); assert.equal(commits(disk).length, 0);
  assert.equal(disk.calls.filter(call => ["write", "mkdir", "move", "deleteFile"].includes(call.method)).length, 0);
});

test("one memory edit publishes only that memory and the metadata guard", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk);
  const first = await owner.run(session => session.createMemory("character:default", memory("First")));
  await owner.run(session => session.createMemory("character:default", memory("Second")));
  disk.clearCalls();
  await owner.run(session => session.updateMemory("character:default", first.id, memory("Edited")));
  assert.equal(commits(disk).length, 1);
  const mutations = commits(disk)[0].args[0];
  assert.equal(mutations.filter(row => /^UPDATE "memories" /.test(row.sql)).length, 1);
  assert.equal(mutations.filter(row => /^UPDATE "meta" /.test(row.sql)).length, 1);
  assert.equal(mutations.filter(row => /^INSERT INTO "cards" /.test(row.sql)).length, 0);
  const reopened = await open(disk); assert.equal((await reopened.run(session => session.listMemories("character:default").then(rows => rows.find(row => row.id === first.id)))).title, "Edited");
});

test("a fresh operation sees writes committed by another repository execution", async t => {
  const disk = await createDiskHarness(t), first = await open(disk), second = await open(disk);
  const created = await second.run(session => session.createCharacter(card("External import")));
  assert.equal((await first.run(session => session.getCharacter(created.id))).name, "External import");
});

test("concurrent writers reject stale snapshots without overwriting an unrelated committed record", async t => {
  const disk = await createDiskHarness(t), first = await open(disk), second = await open(disk);
  let entered, proceed;
  const ready = new Promise(resolve => { entered = resolve; }), gate = new Promise(resolve => { proceed = resolve; });
  const pending = first.run(async session => { entered(); await gate; return session.updateCharacter({ ...(await session.getCharacter("default")), description: "Stale write" }); });
  await ready;
  await second.run(session => session.updateCharacter({ ...card("Actual committed name"), id: "default" }));
  proceed(); await assert.rejects(pending, /affected-row precondition/);
  assert.equal(disk.readState().cards[0].name, "Actual committed name");
});

test("transaction failures preserve original errors and authoritative database records", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk), before = disk.stateBytes(), failure = new Error("Exact storage failure");
  disk.failNext("storage.transaction", failure);
  await assert.rejects(owner.run(session => session.updateCharacter({ ...card("Rejected name"), id: "default" })), error => error === failure);
  assert.deepEqual(disk.stateBytes(), before);
  await assert.rejects(owner.run(session => session.listCharacters()), error => error === failure);
  const reopened = await open(disk); assert.equal((await reopened.run(session => session.getCharacter("default"))).name, "Operit");
});

test("interrupted migration metadata prevents plugin operations and is never replaced with defaults", async t => {
  const disk = await createDiskHarness(t); await open(disk);
  disk.storage.inspect(sql => sql.prepare('UPDATE "meta" SET "migrationStatus" = ?, "migrationStatus__state" = 2 WHERE "_key" = ?').run("writing", "state"));
  const before = disk.stateBytes.bind(disk); disk.clearCalls();
  await assert.rejects(open(disk), /migration has not completed/);
  assert.equal(commits(disk).length, 0); assert.equal(typeof before, "function");
});

test("owner pages preserve exact int64 identities and more than 1000 independently migrated records", async t => {
  const disk = await createDiskHarness(t); const owner = await open(disk);
  const created = plain(await owner.run(session => session.createMemory("character:default", memory("Template"))));
  const state = plain(disk.readState()); state.owners[0].memories = [];
  for (let index = 0; index < 1005; index += 1) state.owners[0].memories.push({ ...created, id: String(9007199254740993n + BigInt(index)), uuid: "source-" + index });
  state.nextId = String(9007199254740993n + 1005n); disk.replaceState(state);
  const reopened = await open(disk), retained = await reopened.run(session => session.listMemories("character:default"));
  assert.equal(retained.length, 1005); assert.equal(retained[0].id, "9007199254740993");
});

/** Verifies external profile staging does not publish a rejected database operation. */
test("rejected profile commit retains the actual published USER.md and original error", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk);
  await owner.run(session => session.writeUser("character:default", "Original profile"));
  const failure = new Error("Profile record commit rejected"); disk.failNext("storage.transaction", failure);
  await assert.rejects(owner.run(session => session.writeUser("character:default", "Rejected profile")), error => error === failure);
  const reopened = await open(disk);
  assert.equal((await reopened.run(session => session.readUser("character:default"))).content, "Original profile");
});

/** Verifies a file publication error stops the same repository without pretending the file write succeeded. */
test("profile publication failure remains explicit after its record commit", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk);
  const failure = new Error("USER.md rename rejected"); disk.failNext("move", failure);
  await assert.rejects(owner.run(session => session.writeUser("character:default", "Pending profile")), error => error === failure);
  await assert.rejects(owner.run(session => session.listCharacters()), error => error === failure);
  const reopened = await open(disk);
  assert.equal((await reopened.run(session => session.readUser("character:default"))).content, "");
});

/** Keeps long provider text in the stored value rather than exceeding the production Host key limit. */
test("embedding cache stores long multiline text with bounded independent keys", async t => {
  const disk = await createDiskHarness(t), owner = await open(disk);
  const text = "Actual source paragraph\n".repeat(200);
  await owner.run(async session => {
    const space = await session.readMemorySpace("character:default");
    space.embeddings.push({ endpoint: "https://embedding.invalid", model: "source-model", text, vector: [1, 2], updatedAt: 123 });
    await session.writeMemorySpace("character:default", space);
  });
  const reopened = await open(disk), retained = await reopened.run(session => session.readMemorySpace("character:default"));
  assert.equal(retained.embeddings[0].text, text);
  disk.storage.inspect(sql => { const row = sql.prepare('SELECT "_key", "record__vector", "record__text" FROM "embeddings"').get(); assert.equal(row._key, "character:default/00000000000000000000"); assert.equal(row.record__text, text); assert.equal(row.record__vector.byteLength, 16); });
});

/** Gates production collection reads without replacing the repository or stored records. */
async function gatedCollections(t, failure = null) {
  const disk = await createDiskHarness(t), originalOpen = disk.storage.sqlite.open;
  let armed = false, release, entered;
  const gate = new Promise(resolve => { release = resolve; });
  const allEntered = new Promise(resolve => { entered = resolve; });
  const started = new Set();
  disk.storage.sqlite.open = async options => {
    const database = await originalOpen(options), query = database.query;
    database.query = async (...args) => {
      const match = /^SELECT \* FROM "([A-Za-z]+)" ORDER BY/.exec(args[0]);
      if (armed && match !== null && match[1] !== "meta") {
        const name = match[1]; started.add(name);
        if (started.size === 11) entered();
        if (name === "cards" && failure !== null) throw failure;
        await gate;
      }
      return query(...args);
    };
    return database;
  };
  const owner = await open(disk);
  return { owner, started, allEntered, release, arm() { armed = true; }, disarm() { armed = false; } };
}

test("repository starts all collection reads before waiting on any collection", { timeout: 5000 }, async t => {
  const gate = await gatedCollections(t);
  gate.arm();
  const pending = gate.owner.run(session => session.listCharacters());
  try {
    await gate.allEntered;
    assert.equal(gate.started.size, 11);
  } finally { gate.release(); }
  assert.equal((await pending)[0].id, "default");
});

test("failed concurrent read settles all collections before another repository operation starts", { timeout: 5000 }, async t => {
  const failure = new Error("Diagnostic collection failure"), gate = await gatedCollections(t, failure);
  gate.arm();
  const failed = gate.owner.run(session => session.listCharacters()).then(
    () => { throw new Error("Expected the original read failure"); },
    error => error,
  );
  await gate.allEntered;
  gate.disarm();
  let executed = false;
  const next = gate.owner.run(async session => { executed = true; return session.listCharacters(); });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(executed, false, "The repository must retain ownership until every read settles");
  gate.release();
  assert.equal(await failed, failure);
  assert.equal((await next)[0].id, "default");
  assert.equal(executed, true);
});
