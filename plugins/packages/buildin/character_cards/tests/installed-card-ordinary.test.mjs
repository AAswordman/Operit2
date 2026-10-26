import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Opens the production repository over the real test object Host. */
async function open(disk) { return loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open(); }

test("installation creates an ordinary UUID card without a default flag", async t => {
  const first = await createDiskHarness(t), second = await createDiskHarness(t);
  await open(first); await open(second);
  const left = first.readState().cards[0], right = second.readState().cards[0];
  assert.match(left.id, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.notEqual(left.id, right.id);
  assert.equal(Object.hasOwn(left, "isDefault"), false);
});

test("the installed card can be edited and deleted under the same rules as another card", async t => {
  const disk = await createDiskHarness(t), repository = await open(disk), installed = plain(disk.readState().cards[0]);
  const changed = await repository.run(session => session.updateCharacter({ ...installed, name: "My own role" }));
  assert.equal(changed.id, installed.id);
  const created = await repository.run(session => session.createCharacter({ ...changed, id: "", name: "Second" }));
  await repository.run(session => session.writeActive({ CharacterCard: { id: created.id } }));
  await repository.run(session => session.deleteCharacter(installed.id));
  assert.deepEqual(plain(await (await open(disk)).listCharacters()).map(card => card.id), [created.id]);
});

test("deleting every card in explicit model mode never recreates an installation card", async t => {
  const disk = await createDiskHarness(t), repository = await open(disk), installed = disk.readState().cards[0];
  await repository.run(session => session.writeActive({ Model: { id: "chat" } }));
  await repository.run(session => session.deleteCharacter(installed.id));
  disk.clearCalls();
  assert.deepEqual(plain(await (await open(disk)).listCharacters()), []);
  assert.equal(disk.calls.some(call => call.method === "storage.commit"), false);
});
