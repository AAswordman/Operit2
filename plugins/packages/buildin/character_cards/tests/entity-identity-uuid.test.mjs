import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Opens the production database repository against the generic SQLite host adapter. */
async function open(disk) { return loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open(); }
/** Selects only actual storage transactions from the generic host audit. */
function commits(disk) { return disk.calls.filter(call => call.method === "storage.commit"); }
/** Requires the exact device-independent identity shape minted for every new entity. */
const ENTITY_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
/** Builds one valid new-character draft from the installation card without reusing its identity. */
function draft(state, name) {
  const base = plain(state.cards[0]);
  return { ...base, id: "", name };
}
/** Supplies an explicit memory creation command without fabricating a persisted memory. */
function memory(title) { return { title, content: "Persisted memory", contentType: "text/plain", source: "manual", credibility: 0.8, importance: 0.6, folderPath: null, tags: [] }; }

test("new entity identities are device-independent UUIDs", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), repository = await open(disk);
  const state = plain(disk.readState());
  const card = plain(await repository.run(session => session.createCharacter(draft(state, "Created"))));
  const group = plain(await repository.run(session => session.createGroup({ id: "", name: "Created group", description: "", themeConfigId: null, members: [], createdAt: 0, updatedAt: 0 })));
  const tag = plain(await repository.run(session => session.createTag({ name: "Created tag", description: "", promptContent: "", tagType: "CUSTOM" })));
  const store = plain(await repository.run(session => session.createStore("Created library")));
  assert.match(card.id, ENTITY_ID);
  assert.match(group.id, ENTITY_ID);
  assert.match(tag.id, ENTITY_ID);
  assert.match(store.id, ENTITY_ID);
  const created = plain(await repository.run(session => session.createMemory("character:default", { ...memory("Device-independent"), tags: ["Independent tag"] })));
  assert.match(created.id, ENTITY_ID);
  assert.equal(created.uuid, created.id);
  assert.match(created.tags[0].id, ENTITY_ID);
  const updated = plain(await repository.run(session => session.updateMemory("character:default", created.id, { ...memory("Device-independent"), tags: ["Replacement tag"] })));
  assert.match(updated.tags[0].id, ENTITY_ID);
});

test("two devices mint disjoint identities whose records merge as separate entities", async t => {
  const first = await createDiskHarness(t, { legacyIdentity: "default" }), second = await createDiskHarness(t, { legacyIdentity: "default" });
  const one = await open(first), two = await open(second);
  const left = plain(await one.run(session => session.createCharacter(draft(plain(first.readState()), "Device card"))));
  const right = plain(await two.run(session => session.createCharacter(draft(plain(second.readState()), "Device card"))));
  assert.match(left.id, ENTITY_ID);
  assert.match(right.id, ENTITY_ID);
  assert.notEqual(left.id, right.id);
  const oneState = plain(first.readState()), merged = plain(second.readState());
  merged.cards.push(plain(oneState.cards.find(card => card.id === left.id)));
  merged.owners.push(plain(oneState.owners.find(owner => owner.ownerKey === "character:" + left.id)));
  const legacy = { ...plain(merged.cards.find(card => card.id === "default")), id: "character-7", name: "Legacy" };
  merged.cards.push(legacy);
  merged.owners.push(plain(loadModule("src/storage/state.ts").createMemorySpace("character:character-7")));
  second.replaceState(merged);
  const reopened = await open(second);
  const cards = plain(await reopened.run(session => session.listCharacters()));
  assert.deepEqual(cards.map(card => card.id).sort(), ["character-7", "default", left.id, right.id].sort());
  assert.equal("nextId" in plain(second.readState()), false);
  second.clearCalls();
  await open(second);
  assert.equal(commits(second).length, 0);
});

test("memories minted on two devices keep distinct identities after merge", async t => {
  const first = await createDiskHarness(t, { legacyIdentity: "default" }), second = await createDiskHarness(t, { legacyIdentity: "default" });
  const one = await open(first), two = await open(second);
  const left = plain(await one.run(session => session.createMemory("character:default", memory("Left device"))));
  const right = plain(await two.run(session => session.createMemory("character:default", memory("Right device"))));
  assert.match(left.id, ENTITY_ID); assert.match(right.id, ENTITY_ID); assert.notEqual(left.id, right.id);
  const source = plain(first.readState()), merged = plain(second.readState());
  merged.owners[0].memories.push(plain(source.owners[0].memories.find(item => item.id === left.id)));
  second.replaceState(merged);
  const reopened = await open(second);
  const memories = plain(await reopened.run(session => session.listMemories("character:default")));
  assert.deepEqual(memories.map(item => item.id).sort(), [left.id, right.id].sort());
});

test("legacy decimal ids coexist with device-independent identities and their references", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), repository = await open(disk);
  const created = plain(await repository.run(session => session.createMemory("character:default", memory("New identity"))));
  const state = plain(disk.readState());
  state.owners[0].memories.push({ ...plain(created), id: "7", uuid: "legacy-seven", title: "Legacy identity" });
  state.owners[0].links.push({ id: "3", sourceMemoryId: created.id, targetMemoryId: "7", type_: "related", weight: 0.5, description: "" });
  disk.replaceState(state);
  const reopened = await open(disk);
  const memories = plain(await reopened.run(session => session.listMemories("character:default")));
  assert.deepEqual(memories.map(item => item.id).sort(), [created.id, "7"].sort());
  const graph = plain(await reopened.run(session => session.readMemoryGraph("character:default")));
  assert.equal(graph.edges.length, 1);
  assert.equal(graph.edges[0].id, "3");
});
