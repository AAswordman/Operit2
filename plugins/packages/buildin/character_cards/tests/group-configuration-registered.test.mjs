import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { createSoftwareSettingsFixture } from "./software-settings-fixture.mjs";
import { loadModule, plain } from "./runtime.mjs";

const directories = plain(loadModule("tests/software-settings-input.ts").softwareSettingsTestInput), markers = loadModule("src/chat-markers.ts");

/** Builds the actual generic host request without a persisted draft query or Core-owned selection token. */
function request(purpose, selection, participantId = null) {
  return { purpose, chatId: "unpersisted-creation-draft", chatExtension: { version: 1, selection }, messageExtension: null, participantId, promptFunctionType: "CHAT",
    defaultModelBinding: { providerId: "fixture-provider-token", modelId: "fixture-model-token" }, defaultTtsConfigId: "fixture-tts-config" };
}

/** Calls only a real main-registered API and real disk service; the explicit directory values remain a controlled SDK boundary. */
test("registered configuration: actual main handler resolves a multi-group display without profiles or directory reads", async t => {
  const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directories), plugin = openPlugin(settings.harness);
  assert.equal(settings.calls.length, 0); assert.equal(disk.calls.length, 0);
  assert.equal(plugin.apis.get("chat.configuration.resolve").function, plugin.main.chatConfigurationResolveApi);
  const a = await plugin.api("character.create", { values: { name: "Registered participant A", openingStatement: "Not group initialization" } });
  const b = await plugin.api("character.create", { values: { name: "Registered participant B", avatarUri: "actual-b.png" } });
  const group = await plugin.api("group.create", { values: { name: "Registered group", members: [{ characterCardId: a.id, orderIndex: 8 }, { characterCardId: b.id, orderIndex: 1 }] } });
  const selection = "group:" + group.id, result = plain(await plugin.api("chat.configuration.resolve", request("display", selection)));
  assert.deepEqual(result, { contextKey: selection, identity: { title: "Registered group", avatarUri: null },
    participants: [{ id: b.id, name: b.name, avatarUri: b.avatarUri }, { id: a.id, name: a.name, avatarUri: a.avatarUri }], initialMessages: [] });
  assert.equal(settings.calls.length, 0, "Group presentation must not read model/TTS/tool directories");
  await assert.rejects(() => plugin.api("chat.configuration.resolve", request("execution", selection)), /explicit execution participant/);
});

/** A real initial message is authored by an explicit single participant and stores that participant's exact execution snapshot. */
test("registered configuration: an actual card opening has an explicit author snapshot, never a group member default", async t => {
  const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directories), plugin = openPlugin(settings.harness);
  const card = await plugin.api("character.create", { values: { name: "Explicit opening author", avatarUri: "actual-author.png", openingStatement: "Actual opening content" } });
  const result = plain(await plugin.api("chat.configuration.resolve", request("display", "card:" + card.id)));
  assert.equal(Object.hasOwn(result, "profile"), false); assert.equal(result.initialMessages.length, 1);
  const initial = result.initialMessages[0], saved = plain(markers.decodeMessageMarker(initial.messageExtension));
  assert.equal(initial.content, card.openingStatement); assert.equal(initial.displayName, card.name);
  assert.equal(saved.profile.id, card.id); assert.equal(saved.profile.name, card.name); assert.equal(saved.selection, "card:" + card.id);
  assert.equal(saved.primaryOwnerKey, "character:" + card.id); assert.equal(saved.participants.length, 1);
  assert.ok(settings.calls.length > 0, "Only an explicitly authored initial message invokes strict execution resolution");
});

/** The existing registered resolver keeps its actual authenticated host boundary rather than creating an unguarded parallel provider. */
test("registered configuration: rejects non-host callers before file/domain reads", async t => {
  const disk = await createDiskHarness(t), plugin = openPlugin(disk), registered = plugin.apis.get("chat.configuration.resolve");
  await assert.rejects(() => registered.function({ callerPackage: "untrusted-plugin", payload: request("display", "group:unread") }), /authenticated host caller/);
  assert.equal(disk.calls.length, 0);
});
