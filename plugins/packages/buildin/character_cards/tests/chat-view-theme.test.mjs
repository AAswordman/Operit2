import assert from "node:assert/strict";
import test from "node:test";
import { plain } from "./runtime.mjs";
import { selectorEnvironment } from "./selector-environment.mjs";
import { createSelectionSettingsFixture } from "./selection-settings-fixture.mjs";

/** Invokes the actual registered callback using the generic Core chat view event envelope. */
async function view(runtime, eventName, chatId) {
  assert.equal(runtime.chatViewHooks.length, 1);
  return runtime.chatViewHooks[0].function({ event: eventName, eventName, eventPayload: { chatId, viewId: "chat:" + chatId, runtime: "main" } });
}

/** Opens real plugin registrations and persists two roles with distinct independent theme references. */
async function environment(t) {
  const settings = createSelectionSettingsFixture(), runtime = await selectorEnvironment(t, settings.settings);
  const first = await runtime.api("character.create", { values: { name: "First view role", themeConfigId: "independent-theme-one" } });
  const second = await runtime.api("character.create", { values: { name: "Second view role", themeConfigId: "independent-theme-two", ttsConfigId: "tts-two" } });
  await runtime.seedBinding("first-chat", { version: 1, selection: "card:" + first.id });
  await runtime.seedBinding("second-chat", { version: 1, selection: "card:" + second.id });
  settings.clearCalls(); runtime.disk.clearCalls();
  return { runtime, settings, first, second };
}

/** Synchronizes the active actor and its configurations without changing either conversation's binding. */
test("registered view-open hook switches the active role and applies its bound configuration", async t => {
  const { runtime, settings, first, second } = await environment(t);
  const firstBinding = await runtime.binding("first-chat"), secondBinding = await runtime.binding("second-chat");
  for (const [chatId, card] of [["second-chat", second], ["first-chat", first]]) {
    await view(runtime, "view_opened", chatId);
    assert.deepEqual(await runtime.api("activePrompt.get", {}), { CharacterCard: { id: card.id } });
    assert.equal(settings.current.theme, card.themeConfigId);
    assert.equal(settings.current.tts, "tts-two");
  }
  assert.deepEqual(await runtime.binding("first-chat"), firstBinding);
  assert.deepEqual(await runtime.binding("second-chat"), secondBinding);
});

/** Reproduces choosing A, opening B's existing chat, then creating a chat from the actual active selection. */
test("new conversation belongs to the opened role instead of the previously selected role", async t => {
  const { runtime, second } = await environment(t);
  await view(runtime, "view_opened", "first-chat");
  await view(runtime, "view_opened", "second-chat");
  assert.equal(runtime.chatLifecycleHooks.length, 1);
  await runtime.chatLifecycleHooks[0].function({ event: "before_create", eventName: "before_create", eventPayload: {
    eventName: "before_create", creationKind: "new",
    chat: { id: "new-after-switch", title: "New conversation", workspaceId: null, parentChatId: null },
    sourceChatId: null, sourceMessageTimestamp: null, input: null, sourceExtension: null,
  } });
  runtime.disk.storage.inspectPreferences("chats",
    /** Verifies the actual draft persisted by the registered creation hook. */
    values => {
      assert.equal(values["draft_new-after-switch"].selection, "card:" + second.id);
    },
  );
});

/** Leaves background events and unassociated chats unchanged and restores an explicitly bound model selection. */
test("only opened bound chats synchronize the active selection", async t => {
  const { runtime, settings, first } = await environment(t);
  await runtime.api("activePrompt.setCard", { id: first.id });
  settings.clearCalls();
  await runtime.seedBinding("model-chat", { version: 1, selection: "model:chat" });
  for (const event of ["view_updated", "view_closed"]) await view(runtime, event, "second-chat");
  await view(runtime, "view_opened", "unbound-chat");
  assert.deepEqual(await runtime.api("activePrompt.get", {}), { CharacterCard: { id: first.id } });
  await view(runtime, "view_opened", "model-chat");
  assert.deepEqual(plain(await runtime.api("activePrompt.get", {})), { Model: { id: "chat" } });
  assert.deepEqual(settings.calls, []);
  assert.equal(await runtime.binding("unbound-chat"), null);
});

/** Applies only a group's own reference and leaves member themes independent. */
test("opening a group chat applies its own theme and an unbound role theme performs no SDK writes", async t => {
  const { runtime, settings, second } = await environment(t);
  const group = await runtime.api("group.create", { values: { name: "View group", themeConfigId: "independent-theme-one", members: [{ characterCardId: second.id, orderIndex: 0 }] } });
  const unbound = await runtime.api("character.create", { values: { name: "Unbound theme role", themeConfigId: null } });
  await runtime.seedBinding("group-chat", { version: 1, selection: "group:" + group.id });
  await runtime.seedBinding("null-theme-chat", { version: 1, selection: "card:" + unbound.id });
  await view(runtime, "view_opened", "second-chat");
  await view(runtime, "view_opened", "group-chat");
  assert.equal(settings.current.theme, "independent-theme-one");
  assert.deepEqual(await runtime.api("activePrompt.get", {}), { CharacterGroup: { id: group.id } });
  settings.clearCalls();
  await view(runtime, "view_opened", "null-theme-chat");
  assert.deepEqual(await runtime.api("activePrompt.get", {}), { CharacterCard: { id: unbound.id } });
  assert.deepEqual(settings.calls, []);
});

/** Preserves explicit failures from deleted references and real owner rejections. */
test("opening a chat propagates missing themes and owner failures without changing its binding", async t => {
  const { runtime, settings } = await environment(t), binding = await runtime.binding("second-chat");
  const before = await runtime.api("activePrompt.get", {});
  settings.themes.delete("independent-theme-two");
  await assert.rejects(view(runtime, "view_opened", "second-chat"), /主题配置不存在/);
  assert.deepEqual(settings.calls, [{ method: "Theme.listConfigs", args: [] }]);
  assert.deepEqual(await runtime.api("activePrompt.get", {}), before);
  const failure = new Error("Theme owner write rejected");
  settings.failAfterWrite("Theme.applyConfig", failure);
  await assert.rejects(view(runtime, "view_opened", "first-chat"), /Theme owner write rejected/);
  assert.deepEqual(await runtime.binding("second-chat"), binding);
});

/** Shares navigation and explicit selection ordering while waiting for the real theme owner's acknowledgement. */
test("view theme application waits for acknowledgement and serializes with explicit role switches", async t => {
  const { runtime, settings, first } = await environment(t), gate = settings.hold("Theme.applyConfig");
  let opened = false, selected = false;
  const opening = view(runtime, "view_opened", "second-chat").then(
    /** Records completion only after the independent theme owner acknowledges its exact write. */
    () => { opened = true; },
  );
  await gate.reached;
  const selecting = runtime.api("character.setActive", { id: first.id }).then(
    /** Records the later explicit selection only after the same shared application queue completes. */
    () => { selected = true; },
  );
  assert.equal(opened, false);
  assert.equal(selected, false);
  assert.deepEqual(settings.calls, [{ method: "Theme.listConfigs", args: [] }, { method: "Tts.listConfigs", args: [] }, { method: "Theme.applyConfig", args: ["independent-theme-two"] }]);
  gate.release();
  await Promise.all([opening, selecting]);
  assert.equal(opened, true);
  assert.equal(selected, true);
  assert.equal(settings.current.theme, "independent-theme-one");
  assert.deepEqual(await runtime.api("activePrompt.get", {}), { CharacterCard: { id: first.id } });
  assert.deepEqual(settings.calls, [
    { method: "Theme.listConfigs", args: [] }, { method: "Tts.listConfigs", args: [] },
    { method: "Theme.applyConfig", args: ["independent-theme-two"] }, { method: "Tts.setCurrentConfigId", args: ["tts-two"] },
    { method: "Tts.getCurrentConfigId", args: [] },
    { method: "Theme.listConfigs", args: [] }, { method: "Theme.applyConfig", args: ["independent-theme-one"] },
  ]);
});

/** Serializes model navigation behind an in-flight role application so late acknowledgements cannot restore a role. */
test("opening a model chat waits for an in-flight role switch and remains the final selection", async t => {
  const { runtime, settings, second } = await environment(t), gate = settings.hold("Theme.applyConfig");
  await runtime.seedBinding("model-chat", { version: 1, selection: "model:chat" });
  const openingRole = view(runtime, "view_opened", "second-chat");
  await gate.reached;
  const openingModel = view(runtime, "view_opened", "model-chat");
  await new Promise(setImmediate);
  assert.deepEqual(await runtime.api("activePrompt.get", {}), { CharacterCard: { id: second.id } });
  gate.release();
  await Promise.all([openingRole, openingModel]);
  assert.deepEqual(plain(await runtime.api("activePrompt.get", {})), { Model: { id: "chat" } });
});
