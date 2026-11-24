import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { plain } from "./runtime.mjs";

/** Calls the actual registered dependency boundary as the voice-call package. */
function identity(plugin, payload) {
  return plugin.apis.get("chat.identity").function({ callerPackage: "com.operit.voice_call", payload });
}

/** Commits a genuine creation binding without changing the globally active role. */
async function bind(plugin, chatId, selection) {
  await plugin.main.beforeChatCreate({ eventName: "before_create", eventPayload: {
    eventName: "before_create", creationKind: "new", chat: { id: chatId, title: "Call", workspaceId: null, parentChatId: null },
    sourceChatId: null, sourceMessageTimestamp: null, sourceExtension: null,
    input: { "com.operit.character_cards": { version: 1, selection } },
  } });
  await plugin.api("chat.configuration.draft.commit", { chatId });
}

/** Proves an existing call uses its committed role even when another role is selected globally. */
test("dependency identity reads the active preview and the call's distinct committed role", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), plugin = openPlugin(disk);
  const card = await plugin.api("character.create", { values: { name: "Custom caller", avatarUri: "vfs://shared/avatars/caller.png" } });
  await bind(plugin, "voice-call-chat", "card:" + card.id);
  assert.deepEqual(plain(await identity(plugin, { chatId: null })), { title: "Operit", avatarUri: null });
  assert.deepEqual(plain(await identity(plugin, { chatId: "voice-call-chat" })), { title: card.name, avatarUri: card.avatarUri });
  await plugin.api("character.update", { id: card.id, changes: { name: "Updated caller", avatarUri: "vfs://shared/avatars/updated.png" } });
  assert.deepEqual(plain(await identity(plugin, { chatId: "voice-call-chat" })), { title: "Updated caller", avatarUri: "vfs://shared/avatars/updated.png" });
});

/** Uses the group's authored portrait rather than choosing an arbitrary member. */
test("dependency identity shares group display rules and preserves an explicitly unset portrait", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), plugin = openPlugin(disk);
  const group = await plugin.api("group.create", { values: { name: "Call group", avatarUri: "vfs://shared/avatars/group.png", members: [{ characterCardId: "default", orderIndex: 0 }] } });
  await bind(plugin, "group-call", "group:" + group.id);
  assert.deepEqual(plain(await identity(plugin, { chatId: "group-call" })), { title: group.name, avatarUri: group.avatarUri });
  await bind(plugin, "no-portrait", "card:default");
  assert.deepEqual(plain(await identity(plugin, { chatId: "no-portrait" })), { title: "Operit", avatarUri: null });
  await bind(plugin, "model-call", "model:chat");
  assert.equal(await identity(plugin, { chatId: "model-call" }), null);
});

/** Invalid payloads and stale persisted references must stay visible to the dependency caller. */
test("dependency identity rejects malformed inputs and propagates deleted-role errors", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), plugin = openPlugin(disk);
  for (const payload of [null, {}, { chatId: "" }, { chatId: 7 }, { chatId: null, purpose: "execution" }]) {
    await assert.rejects(() => identity(plugin, payload));
  }
  assert.equal(disk.calls.length, 0);
  const card = await plugin.api("character.create", { values: { name: "Removed caller" } });
  await bind(plugin, "stale-call", "card:" + card.id);
  const state = disk.readState();
  state.cards = state.cards.filter(record => record.id !== card.id);
  disk.replaceState(state);
  await assert.rejects(() => identity(openPlugin(disk), { chatId: "stale-call" }));
});

/** The dependency display boundary includes every saved member, including those with no reply. */
test("dependency presentation returns the persisted ordered roster independently of active selection", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" }), plugin = openPlugin(disk);
  const alice = await plugin.api("character.create", { values: { name: "Alice", avatarUri: "alice.png" } });
  const bob = await plugin.api("character.create", { values: { name: "Bob" } });
  const group = await plugin.api("group.create", { values: { name: "Call group", avatarUri: "group.png", members: [
    { characterCardId: alice.id, orderIndex: 2 }, { characterCardId: "default", orderIndex: 1 }, { characterCardId: bob.id, orderIndex: 0 },
  ] } });
  await bind(plugin, "roster-call", "group:" + group.id);
  const presentation = payload => plugin.apis.get("chat.presentation").function({ callerPackage: "com.operit.voice_call", payload });
  assert.deepEqual(plain(await presentation({ chatId: "roster-call" })), {
    kind: "group", identity: { title: group.name, avatarUri: group.avatarUri },
    participants: [
      { id: bob.id, name: "Bob", avatarUri: null },
      { id: "default", name: "Operit", avatarUri: null },
      { id: alice.id, name: "Alice", avatarUri: "alice.png" },
    ],
  });
  await bind(plugin, "model-roster", "model:chat");
  assert.deepEqual(plain(await presentation({ chatId: "model-roster" })), { kind: "model", identity: null, participants: [] });
  const active = plain(await presentation({ chatId: null }));
  assert.equal(active.kind, "card");
  assert.equal(active.participants.length, 1);
  assert.equal(active.participants[0].id, "default");
  for (const payload of [null, {}, { chatId: "" }, { chatId: null, purpose: "execution" }]) {
    await assert.rejects(() => presentation(payload));
  }
  const state = disk.readState();
  state.cards = state.cards.filter(record => record.id !== bob.id);
  disk.replaceState(state);
  await assert.rejects(() => plugin.apis.get("chat.presentation").function({ callerPackage: "com.operit.voice_call", payload: { chatId: "roster-call" } }));
});
