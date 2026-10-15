import assert from "node:assert/strict";
import test from "node:test";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { createHostDirectoriesFixture } from "./host-directories-fixture.mjs";
import { loadModule, plain } from "./runtime.mjs";

const directories = plain(loadModule("tests/host-directories-input.ts").hostDirectoriesTestInput), markers = loadModule("src/chat-markers.ts");

/** Builds the actual generic host request without a persisted draft query or Core-owned selection token. */
function request(purpose, selection, participantId = null) {
  return { purpose, chatId: "unpersisted-creation-draft", chatExtension: { version: 1, selection }, messageExtension: null, participantId, promptFunctionType: "CHAT",
    defaultModelBinding: { providerId: "fixture-provider-token", modelId: "fixture-model-token" }, defaultTtsConfigId: "fixture-tts-config" };
}

/** Prepares actual plugin draft data before calling its authenticated display resolver. */
async function resolveDraft(plugin, input) {
  await plugin.main.beforeChatCreate({ eventName: "before_create", eventPayload: {
    eventName: "before_create", creationKind: "new", chat: { id: input.chatId, title: "Draft", workspaceId: null, parentChatId: null },
    sourceChatId: null, sourceMessageTimestamp: null, sourceExtension: null,
    input: { "com.operit.character_cards": input.chatExtension },
  } });
  return plugin.api("chat.configuration.draft.resolve", input);
}

/** Calls only a real main-registered API and real disk service; the explicit directory values remain a controlled SDK boundary. */
test("registered configuration: actual main handler resolves a multi-group display without profiles or directory reads", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  assert.equal(settings.calls.length, 0); assert.equal(disk.calls.length, 0);
  assert.equal(plugin.apis.get("chat.configuration.resolve").function, plugin.main.chatConfigurationResolveApi);
  const a = await plugin.api("character.create", { values: { name: "Registered participant A", openingStatement: "Not group initialization" } });
  const b = await plugin.api("character.create", { values: { name: "Registered participant B", avatarUri: "actual-b.png" } });
  const group = await plugin.api("group.create", { values: { name: "Registered group", members: [{ characterCardId: a.id, orderIndex: 8 }, { characterCardId: b.id, orderIndex: 1 }] } });
  const selection = "group:" + group.id, result = plain(await resolveDraft(plugin, request("display", selection)));
  assert.deepEqual(result, { contextKey: selection, identity: { title: "Registered group", avatarUri: null },
    participants: [{ id: b.id, name: b.name, avatarUri: b.avatarUri }, { id: a.id, name: a.name, avatarUri: a.avatarUri }], initialMessages: [] });
  assert.equal(settings.calls.length, 0, "Group presentation must not read model/TTS/tool directories");
  await assert.rejects(() => plugin.api("chat.configuration.draft.resolve", request("execution", selection)), /display identity/);
});

/** A real initial message is authored by an explicit single participant and stores that participant's exact execution snapshot. */
test("registered configuration: an actual card opening has an explicit author snapshot, never a group member default", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  const card = await plugin.api("character.create", { values: { name: "Explicit opening author", avatarUri: "actual-author.png", openingStatement: "Actual opening content" } });
  const result = plain(await resolveDraft(plugin, request("display", "card:" + card.id)));
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

/** Prepares and commits the real provider-owned binding before resolving an execution. */
async function committedConfiguration(plugin, selection, participantId = null, chatId = "unpersisted-creation-draft") {
  await resolveDraft(plugin, { ...request("display", selection), chatId });
  await plugin.api("chat.configuration.draft.commit", { chatId });
  return plain(await plugin.api("chat.configuration.resolve", { ...request("execution", selection, participantId), chatId }));
}

/** The actual registered resolver authors group constraints before native storage freezes the execution snapshot. */
test("registered configuration: group prompts preserve each speaker and saved member order across historical execution", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  const a = await plugin.api("character.create", { values: { name: "Prompt speaker A", characterSetting: "Exact A setting" } });
  const b = await plugin.api("character.create", { values: { name: "Prompt speaker B", characterSetting: "Exact B setting" } });
  const group = await plugin.api("group.create", { values: { name: "Prompt group", members: [{ characterCardId: a.id, orderIndex: 9 }, { characterCardId: b.id, orderIndex: 2 }] } });
  const selection = "group:" + group.id, result = await committedConfiguration(plugin, selection, a.id);
  assert.equal(result.profile.id, a.id);
  assert.match(result.profile.introPrompt, /Exact A setting/);
  assert.ok(result.profile.introPrompt.includes("Stay in role as " + a.name));
  assert.ok(result.profile.introPrompt.includes("Group participants: " + b.name + ", " + a.name));
  for (const participant of result.participants) assert.ok(participant.introPrompt.includes("Stay in role as " + participant.name));
  const saved = plain(markers.decodeMessageMarker(result.messageExtension));
  assert.deepEqual(saved.profile, result.profile); assert.deepEqual(saved.participants, result.participants);
  await plugin.api("character.update", { id: a.id, changes: { name: "Changed current name", characterSetting: "Changed current setting" } });
  const historical = plain(await plugin.api("chat.configuration.resolve", { ...request("execution", selection, a.id), messageExtension: result.messageExtension }));
  assert.deepEqual(historical.profile, saved.profile);
  assert.deepEqual(historical.messageExtension, result.messageExtension);
});

/** Card execution carries only its authored prompt, while a one-member group retains explicit group semantics. */
test("registered configuration: single cards and single-member groups retain their own prompt scope", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  const card = await plugin.api("character.create", { values: { name: "Solo prompt speaker", characterSetting: "Exact solo setting" } });
  const single = await committedConfiguration(plugin, "card:" + card.id);
  assert.doesNotMatch(single.profile.introPrompt, /Role response plan hint|Group participants/);
  const group = await plugin.api("group.create", { values: { name: "One member", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  const grouped = await committedConfiguration(plugin, "group:" + group.id, card.id, "single-member-group-chat");
  assert.ok(grouped.profile.introPrompt.includes("Stay in role as " + card.name));
  assert.ok(grouped.profile.introPrompt.includes("Group participants: " + card.name));
});

/** Authenticated generic deletion removes the real binding and leaves historical message snapshots usable. */
test("registered configuration: generic deletion checks its owner, acknowledges real cleanup and preserves history", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  const card = await plugin.api("character.create", { values: { name: "Deletion speaker" } });
  const selection = "card:" + card.id, executed = await committedConfiguration(plugin, selection);
  const registered = plugin.apis.get("chat.configuration.delete");
  assert.equal(registered.function, plugin.main.chatConfigurationDeleteApi);
  await assert.rejects(() => registered.function({ callerPackage: "other-owner", payload: { chatId: "unpersisted-creation-draft" } }), /host caller/);
  assert.equal((await plugin.api("chat.configuration.binding.read", { chatId: "unpersisted-creation-draft" })).selection, selection);
  assert.deepEqual(plain(await plugin.api("chat.configuration.delete", { chatId: "unpersisted-creation-draft" })), { deleted: true });
  await assert.rejects(() => plugin.api("chat.configuration.binding.read", { chatId: "unpersisted-creation-draft" }));
  const historical = plain(await plugin.api("chat.configuration.resolve", { ...request("execution", selection, card.id), messageExtension: executed.messageExtension }));
  assert.deepEqual(historical.profile, executed.profile);
  assert.deepEqual(plain(await plugin.api("chat.configuration.delete", { chatId: "unpersisted-creation-draft" })), { deleted: true });
});

/** Existing chats without role associations use a real model profile and keep their own immutable history. */
test("registered configuration: no role association executes the configured model and provider identity", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  const input = request("execution", "model:chat");
  const result = plain(await plugin.api("chat.configuration.resolve", input));
  assert.equal(result.profile.name, input.defaultModelBinding.modelId);
  assert.equal(result.profile.avatarUri, "provider-logo:fixture-provider-type");
  assert.equal(result.profile.introPrompt, ""); assert.equal(result.profile.userPreferencesText, "");
  assert.deepEqual(result.profile.resources, []); assert.equal(result.messageExtension.primaryOwnerKey, "");
  assert.deepEqual(result.profile.modelBinding, input.defaultModelBinding);
  const display = plain(await plugin.api("chat.configuration.resolve", { ...input, purpose: "display" }));
  assert.deepEqual(display.identity, { title: result.profile.name, avatarUri: result.profile.avatarUri });
  assert.deepEqual(display.participants, [{ id: "model:chat", name: result.profile.name, avatarUri: result.profile.avatarUri }]);
  const reference = await plugin.api("chat.configuration.snapshot.store", result.messageExtension);
  const saved = await plugin.api("chat.configuration.snapshot.read", reference);
  await plugin.api("activePrompt.setCard", { id: "default" });
  settings.clearCalls();
  const historical = plain(await plugin.api("chat.configuration.resolve", { ...input, messageExtension: saved }));
  assert.deepEqual(historical.profile, result.profile); assert.equal(settings.calls.length, 0);
  const corrupt = plain(saved); corrupt.profile.introPrompt = "Unexpected role prompt"; corrupt.participants[0].introPrompt = corrupt.profile.introPrompt;
  await assert.rejects(() => plugin.api("chat.configuration.resolve", { ...input, messageExtension: corrupt }), /model-only message snapshot/);
  settings.failNext("Models.listSummaries", new Error("Configured model directory rejected"));
  await assert.rejects(() => plugin.api("chat.configuration.resolve", input), /Configured model directory rejected/);
});

/** The genuine global model selection persists without referencing any stored character or group. */
test("registered configuration: explicit model selection persists and produces role-free drafts", async t => {
  const disk = await createDiskHarness(t), settings = createHostDirectoriesFixture(disk, directories), plugin = openPlugin(settings.harness);
  assert.deepEqual(plain(await plugin.api("activePrompt.setModel", {})), { type: "model", id: "chat", active: true });
  assert.deepEqual(plain(await plugin.api("activePrompt.get", {})), { Model: { id: "chat" } });
  const result = await committedConfiguration(plugin, "model:chat");
  assert.equal(result.profile.id, "model:chat"); assert.equal(result.profile.name, "fixture-model-token");
  assert.deepEqual(result.profile.resources, []);
  const reopened = openPlugin(settings.harness);
  assert.deepEqual(plain(await reopened.api("activePrompt.get", {})), { Model: { id: "chat" } });
  await assert.rejects(() => reopened.api("chat.configuration.resolve", request("execution", "model:chat", "default")), /Invalid model-only execution participant/);
});
