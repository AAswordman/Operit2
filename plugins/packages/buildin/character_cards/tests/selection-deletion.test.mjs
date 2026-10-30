import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Uses real plugin persistence while controlling only Core's generic conversation directory. */
async function setup(t, listChats) {
  const disk = await createDiskHarness(t);
  const globals = { ...disk.globals, Tools: { Files: disk.files, Storage: disk.storage, Chat: { listChats } } };
  const repository = await loadModule("src/storage/database.ts", globals).DatabaseCharacterRepository.open();
  const card = plain(disk.readState().cards[0]);
  await repository.run(session => session.writeActive({ Model: { id: "chat" } }));
  return { repository, card };
}

test("a deleted Core conversation's residual binding does not block deleting its character", async t => {
  const calls = [];
  const { repository, card } = await setup(t, async input => {
    calls.push(plain(input));
    return { totalCount: 0, chats: [] };
  });
  await repository.chatBindings.writeChatBinding({ chatId: "deleted-chat", selection: "card:" + card.id });
  await repository.run(session => session.deleteCharacter(card.id));
  assert.deepEqual(plain(await repository.listCharacters()), []);
  assert.deepEqual(calls, [{ chat_ids: ["deleted-chat"], limit: 200 }]);
});

test("a real conversation still blocks deletion when another binding is stale", async t => {
  const { repository, card } = await setup(t, async input => {
    assert.deepEqual(plain(input), { chat_ids: ["deleted-chat", "live-chat"], limit: 200 });
    return { totalCount: 1, chats: [{ id: "live-chat" }] };
  });
  for (const chatId of ["deleted-chat", "live-chat"])
    await repository.chatBindings.writeChatBinding({ chatId, selection: "card:" + card.id });
  await assert.rejects(repository.run(session => session.deleteCharacter(card.id)), /before deleting its selection: live-chat/);
  assert.equal((await repository.listCharacters()).length, 1);
});

test("residual group bindings do not block deleting a group or its member character", async t => {
  const { repository, card } = await setup(t, async () => ({ totalCount: 0, chats: [] }));
  const group = await repository.run(session => session.createGroup({
    id: "", name: "Group", description: "", themeConfigId: null,
    members: [{ characterCardId: card.id, orderIndex: 0 }], createdAt: 0, updatedAt: 0,
  }));
  await repository.chatBindings.writeChatBinding({ chatId: "deleted-group-chat", selection: "group:" + group.id });
  await repository.run(session => session.deleteCharacter(card.id));
  assert.deepEqual(plain((await repository.listGroups())[0].members), []);
  await repository.run(session => session.deleteGroup(group.id));
  assert.deepEqual(plain(await repository.listGroups()), []);
});

test("an existing group conversation protects both the group and its member character", async t => {
  const { repository, card } = await setup(t, async () => ({ totalCount: 1, chats: [{ id: "live-group-chat" }] }));
  const group = await repository.run(session => session.createGroup({
    id: "", name: "Group", description: "", themeConfigId: null,
    members: [{ characterCardId: card.id, orderIndex: 0 }], createdAt: 0, updatedAt: 0,
  }));
  await repository.chatBindings.writeChatBinding({ chatId: "live-group-chat", selection: "group:" + group.id });
  await assert.rejects(repository.run(session => session.deleteCharacter(card.id)), /before deleting its selection: live-group-chat/);
  await assert.rejects(repository.run(session => session.deleteGroup(group.id)), /before deleting its selection: live-group-chat/);
  assert.equal((await repository.listCharacters()).length, 1);
  assert.equal((await repository.listGroups()).length, 1);
});

test("deletion checks every binding batch instead of stopping after 200 stale conversations", async t => {
  const calls = [];
  const { repository, card } = await setup(t, async input => {
    calls.push(plain(input));
    const chats = input.chat_ids.includes("live-chat") ? [{ id: "live-chat" }] : [];
    return { totalCount: chats.length, chats };
  });
  for (let index = 0; index < 200; index++)
    await repository.chatBindings.writeChatBinding({ chatId: "deleted-" + String(index).padStart(3, "0"), selection: "card:" + card.id });
  await repository.chatBindings.writeChatBinding({ chatId: "live-chat", selection: "card:" + card.id });
  await assert.rejects(repository.run(session => session.deleteCharacter(card.id)), /before deleting its selection: live-chat/);
  assert.deepEqual(calls.map(call => call.chat_ids.length), [200, 1]);
});

for (const [label, lookup, expected] of [
  ["lookup failure", async () => { throw new Error("Core unavailable"); }, /Core unavailable/],
  ["incomplete lookup", async () => ({ totalCount: 1, chats: [] }), /对话列表不完整/],
  ["unrelated identity", async () => ({ totalCount: 1, chats: [{ id: "unrelated" }] }), /unrelated or duplicate/],
]) {
  test(label + " preserves the character and its binding", async t => {
    const { repository, card } = await setup(t, lookup);
    const binding = { chatId: "unknown-chat", selection: "card:" + card.id };
    await repository.chatBindings.writeChatBinding(binding);
    await assert.rejects(repository.run(session => session.deleteCharacter(card.id)), expected);
    assert.equal((await repository.listCharacters()).length, 1);
    assert.deepEqual(plain(await repository.chatBindings.findChatBinding(binding.chatId)), binding);
  });
}
