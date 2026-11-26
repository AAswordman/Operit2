import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Opens real plugin persistence. */
async function setup(t) {
  const disk = await createDiskHarness(t);
  const repository = await loadModule("src/storage/database.ts", disk.globals).DatabaseCharacterRepository.open();
  return { repository, card: plain(disk.readState().cards[0]) };
}
/** Creates a group containing the specified characters. */
async function createGroup(repository, cards) {
  return repository.run(session => session.createGroup({ id: "", name: "Group", description: "", themeConfigId: null, members: cards.map((card, orderIndex) => ({ characterCardId: card.id, orderIndex })), createdAt: 0, updatedAt: 0 }));
}

test("deleting an active card unbinds its chats and drafts while preserving other bindings", async t => {
  const { repository, card } = await setup(t);
  await repository.chatBindings.writeChatBinding({ chatId: "live", selection: "card:" + card.id });
  await repository.chatBindings.writeChatBinding({ chatId: "stale", selection: "card:" + card.id });
  await repository.chatBindings.writeChatBinding({ chatId: "other", selection: "model:chat" });
  await repository.chatBindings.prepareChatBinding({ chatId: "draft", selection: "card:" + card.id });
  await repository.run(session => session.deleteCharacter(card.id));
  assert.equal(await repository.chatBindings.findChatBinding("live"), null);
  assert.equal(await repository.chatBindings.findChatBinding("stale"), null);
  assert.equal((await repository.chatBindings.readChatBinding("other")).selection, "model:chat");
  await assert.rejects(repository.chatBindings.readDraftBinding("draft"), /unavailable/);
  assert.deepEqual(plain(await repository.readActive()), { Model: { id: "chat" } });
  assert.deepEqual(plain(await repository.listCharacters()), []);
});

test("deleting a member preserves a populated group binding and unbinds an empty group", async t => {
  const { repository, card } = await setup(t);
  const second = await repository.run(session => session.createCharacter({ ...card, name: "Second" }));
  const group = await createGroup(repository, [card, second]);
  await repository.chatBindings.writeChatBinding({ chatId: "group-chat", selection: "group:" + group.id });
  await repository.run(session => session.deleteCharacter(card.id));
  assert.equal((await repository.chatBindings.readChatBinding("group-chat")).selection, "group:" + group.id);
  assert.equal((await repository.listGroups())[0].members.length, 1);
  await repository.run(session => session.writeActive({ CharacterGroup: { id: group.id } }));
  await repository.run(session => session.deleteCharacter(second.id));
  assert.equal(await repository.chatBindings.findChatBinding("group-chat"), null);
  assert.deepEqual(plain(await repository.readActive()), { Model: { id: "chat" } });
});

test("deleting a group unbinds its chats and resets its active selection", async t => {
  const { repository, card } = await setup(t);
  const group = await createGroup(repository, [card]);
  await repository.chatBindings.writeChatBinding({ chatId: "group-chat", selection: "group:" + group.id });
  await repository.run(session => session.writeActive({ CharacterGroup: { id: group.id } }));
  await repository.run(session => session.deleteGroup(group.id));
  assert.equal(await repository.chatBindings.findChatBinding("group-chat"), null);
  assert.deepEqual(plain(await repository.readActive()), { Model: { id: "chat" } });
  assert.deepEqual(plain(await repository.listGroups()), []);
});
