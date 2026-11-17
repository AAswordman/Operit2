import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { keyedNode, mountRegisteredComposeRoute, mountRegisteredSelector } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";
import { buildUiScreenScripts } from "../scripts/build.mjs";

/** Opens the selector through the actual quick-switch row, retaining its navigation intent. */
async function quickSelector(runtime, chatId = "selector-chat") {
  const menu = mountRegisteredComposeRoute(runtime, null, "chat-input-menu", { chatId, input: {} });
  await menu.load();
  const action = (await menu.dispatch(keyedNode(menu.render().tree, "current-character").props.onClick)).actionResult;
  assert.equal(action.input.openLatestChat, true);
  const selector = mountRegisteredSelector(runtime, { requestId: "quick-switch", input: action.input });
  await selector.load();
  return selector;
}

/** Seeds only the host's metadata and the persisted plugin binding, leaving production sorting and lookup intact. */
async function history(runtime, id, selection, updatedAt) {
  runtime.records.set(id, { extension: null, summary: { updatedAt, messageCount: 5 } });
  await runtime.seedBinding(id, { version: 1, selection });
}

async function select(ui, key) {
  return plain((await ui.dispatch(keyedNode(ui.render().tree, key).props.onClick)).actionResult);
}

function activated(chatId) {
  return { type: "toolpkg.presentation.complete", requestId: "quick-switch", value: { type: "toolpkg.chat.activate", chatId } };
}

test("quick switch opens the target role's latest updated conversation and preserves all bindings", async t => {
  const runtime = await selectorEnvironment(t);
  const card = await runtime.api("character.create", { values: { name: "Target" } });
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  await history(runtime, "target-old", "card:" + card.id, "2026-11-01T00:00:00Z");
  await history(runtime, "target-latest", "card:" + card.id, String(Date.parse("2026-11-17T00:00:00Z")));
  await history(runtime, "other-newer", "card:default", "2026-11-18T00:00:00Z");
  const ui = await quickSelector(runtime), before = await runtime.disk.stateBytes();
  assert.deepEqual(await select(ui, "card:" + card.id), activated("target-latest"));
  assert.deepEqual(await runtime.disk.stateBytes(), before);
  assert.equal(runtime.calls.filter(call => call.method === "createNew").length, 0);
});

test("quick switch without history creates an explicitly bound chat without inheriting the current chat", async t => {
  const runtime = await selectorEnvironment(t);
  const card = await runtime.api("character.create", { values: { name: "First conversation" } });
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  // A stale binding whose Core conversation was deleted must not count as history.
  await runtime.seedBinding("deleted-chat", { version: 1, selection: "card:" + card.id });
  const ui = await quickSelector(runtime);
  const row = keyedNode(ui.render().tree, "card:" + card.id).props.onClick;
  assert.deepEqual(plain((await ui.dispatch(row)).actionResult), activated("created-chat-1"));
  assert.deepEqual(await runtime.binding("selector-chat"), { version: 1, selection: "card:default" });
  assert.deepEqual(await runtime.binding("created-chat-1"), { version: 1, selection: "card:" + card.id });
  assert.deepEqual(runtime.calls.filter(call => call.method === "createNew").map(call => call.args[0]), [{
    setAsCurrentChat: false, sourceChatId: null,
    input: { "com.operit.character_cards": { version: 1, selection: "card:" + card.id } },
  }]);
  await assert.rejects(async () => ui.dispatch(row), /already finished|compose action not found/);
  assert.equal(runtime.calls.filter(call => call.method === "createNew").length, 1);
});

test("quick switch from a new-session screen also reuses the target role's history", async t => {
  const runtime = await selectorEnvironment(t);
  const card = await runtime.api("character.create", { values: { name: "Existing role" } });
  await history(runtime, "existing-role-chat", "card:" + card.id, "2026-11-17T00:00:00Z");
  const ui = await quickSelector(runtime, null), before = await runtime.disk.stateBytes();
  assert.deepEqual(await select(ui, "card:" + card.id), activated("existing-role-chat"));
  assert.deepEqual(await runtime.disk.stateBytes(), before);
});

test("selecting the current role opens its latest chat without creating another conversation", async t => {
  const runtime = await selectorEnvironment(t);
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  const ui = await quickSelector(runtime);
  assert.deepEqual(await select(ui, "card:default"), activated("selector-chat"));
  assert.equal(runtime.calls.filter(call => call.method === "createNew").length, 0);
});

test("model-only quick switch reuses unbound model history instead of removing the current role", async t => {
  const runtime = await selectorEnvironment(t);
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  runtime.records.set("model-chat", { extension: null, summary: { updatedAt: "2026-11-17T00:00:00Z" } });
  const ui = await quickSelector(runtime);
  assert.deepEqual(await select(ui, "model:chat"), activated("model-chat"));
  assert.equal((await runtime.binding("selector-chat")).selection, "card:default");
});

test("group navigation keeps its own history separate from member card conversations", async t => {
  const runtime = await selectorEnvironment(t);
  const group = await runtime.api("group.create", { values: { name: "Group", members: [{ characterCardId: "default", orderIndex: 0 }] } });
  await history(runtime, "group-history", "group:" + group.id, "2026-11-16T00:00:00Z");
  await history(runtime, "member-history", "card:default", "2026-11-17T00:00:00Z");
  const ui = mountRegisteredSelector(runtime, { requestId: "quick-switch", input: {
    mode: "select", chatId: "selector-chat", openLatestChat: true, kind: "group", selected: null,
  } });
  await ui.load();
  assert.deepEqual(await select(ui, "group:" + group.id), activated("group-history"));
  assert.equal(await runtime.binding("selector-chat"), null);
});

test("quick switch serializes pending navigation and keeps lookup failures retryable", async t => {
  const runtime = await selectorEnvironment(t), ui = await quickSelector(runtime);
  const listAll = runtime.tools.Chat.listAll;
  let release, reached;
  const pendingRead = new Promise(resolve => { release = resolve; });
  const started = new Promise(resolve => { reached = resolve; });
  runtime.tools.Chat.listAll = async () => { reached(); await pendingRead; throw new Error("History lookup failed"); };
  const row = keyedNode(ui.render().tree, "card:default").props.onClick;
  const pending = ui.dispatch(row);
  await started;
  assert.equal(keyedNode(ui.render().tree, "selector-close").props.enabled, false);
  await assert.rejects(async () => ui.dispatch(row), /already running|compose action not found/);
  release();
  await assert.rejects(pending, /History lookup failed/);
  assert.match(keyedNode(ui.render().tree, "selector-commit-error").props.text, /History lookup failed/);
  assert.equal(runtime.calls.filter(call => call.method === "createNew").length, 0);
  runtime.tools.Chat.listAll = listAll;
  assert.deepEqual(await select(ui, "model:chat"), activated("selector-chat"));
});

test("packaged quick switch preserves the original conversation when creation fails", async t => {
  const runtime = await selectorEnvironment(t, {}, {}, await buildUiScreenScripts());
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  const card = await runtime.api("character.create", { values: { name: "New role" } });
  const ui = await quickSelector(runtime), before = await runtime.disk.stateBytes();
  runtime.tools.Chat.createNew = async () => { throw new Error("Creation failed"); };
  await assert.rejects(async () => select(ui, "card:" + card.id), /Creation failed/);
  assert.deepEqual(await runtime.disk.stateBytes(), before);
  assert.match(keyedNode(ui.render().tree, "selector-commit-error").props.text, /Creation failed/);
  assert.equal((await ui.dispatch(keyedNode(ui.render().tree, "selector-close").props.onClick)).actionResult.type, "toolpkg.presentation.cancel");
});
