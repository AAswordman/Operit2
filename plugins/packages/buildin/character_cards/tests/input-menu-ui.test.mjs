import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, mountRegisteredSelector, composeNodes, keyedNode } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";

/** Mounts the actually registered input-menu DSL module with the same neutral host state as Flutter. */
function menu(runtime, chatId) { return mountRegisteredComposeRoute(runtime, null, "chat-input-menu", { chatId, input: {} }); }

/** Extracts actual SDK-rendered text values instead of recreating a menu in the fixture. */
function texts(tree) { return composeNodes(tree,
  /** Selects native DSL Text nodes from the real serialized tree. */
  node => node.type === "Text",
).map(node => node.props.text); }

/** Verifies one real registered surface and the historical row's exact native geometry and route action. */
test("registered TS input menu reproduces one avatar and two-line character row above statistics", async t => {
  const runtime = await selectorEnvironment(t);
  const card = await runtime.api("character.create", { values: { name: "DSL current role", avatarUri: "https://example.invalid/actual-avatar.png" } });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:" + card.id });
  const registrations = runtime.navigation.filter(
    /** Requires the exact shared navigation surface rendered directly by the menu host. */
    entry => entry.surface === "chat_input_menu",
  );
  assert.equal(registrations.length, 1); assert.equal(registrations[0].route, "toolpkg:com.operit.character_cards:ui:chat-input-menu");
  const ui = menu(runtime, "selector-chat"); await ui.load(); const tree = ui.render().tree;
  assert.equal(tree.type, "Row"); assert.equal(tree.props.paddingHorizontal, 12); assert.equal(tree.props.paddingVertical, 6);
  assert.deepEqual(plain(tree.props.modifier.__modifierOps), [{ name: "heightIn", args: [{ minHeight: 48 }] }]);
  assert.deepEqual(texts(tree), ["当前角色卡", card.name]);
  const avatars = composeNodes(tree,
    /** Requires the persisted avatar and circular crop in the actual native menu row. */
    node => node.type === "Image",
  );
  assert.equal(avatars.length, 1); assert.equal(avatars[0].props.width, 32); assert.equal(avatars[0].props.height, 32);
  assert.equal(avatars[0].props.uri, card.avatarUri); assert.equal(avatars[0].props.contentScale, "crop");
  const response = await ui.dispatch(keyedNode(tree, "current-character").props.onClick);
  assert.deepEqual(plain(response.actionResult), { type: "toolpkg.ui.present", routeId: "toolpkg:com.operit.character_cards:ui:selection",
    input: { mode: "select", chatId: "selector-chat", kind: "card", selected: { CharacterCard: { id: card.id } } } });
  const selector = mountRegisteredSelector(runtime, { requestId: "menu-click", input: response.actionResult.input }); await selector.load();
  assert.equal(selector.render().tree.type, "Dialog");
});

/** Keeps an unbound chat explicit and commits only after the user selects an actual record in the TS dialog. */
test("unbound TS menu shows its original row and opens a real selector without borrowing global active", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "User-selected role" } });
  const ui = menu(runtime, "selector-chat"); await ui.load(); const tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["当前角色卡", "未绑定"]);
  assert.equal(runtime.records.get("selector-chat").extension, null);
  const response = await ui.dispatch(keyedNode(tree, "current-character").props.onClick);
  assert.equal(response.actionResult.input.selected, null);
  const selector = mountRegisteredSelector(runtime, { requestId: "menu-bind", input: response.actionResult.input }); await selector.load();
  await selector.dispatch(keyedNode(selector.render().tree, "card:" + card.id).props.onClick);
  assert.equal(runtime.records.get("selector-chat").extension.selection, "card:" + card.id);
});

/** Ensures a group binding never expands the single original menu slot into extra group controls. */
test("group-bound TS menu retains one original row and only the character-switch action", async t => {
  const runtime = await selectorEnvironment(t), participant = await runtime.api("character.create", { values: { name: "Actual participant" } });
  const group = await runtime.api("group.create", { values: { name: "Actual group", members: [{ characterCardId: participant.id, orderIndex: 0 }] } });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "group:" + group.id });
  const ui = menu(runtime, "selector-chat"); await ui.load(); const tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["当前角色卡", group.name]);
  assert.equal(composeNodes(tree,
    /** Counts genuine native menu rows that can issue a presentation request. */
    node => node.props.onClick !== undefined,
  ).length, 1);
  const response = await ui.dispatch(tree.props.onClick);
  assert.equal(response.actionResult.input.kind, "card"); assert.equal(response.actionResult.input.selected, null);
});

/** Preserves the explicit global new-session mode while keeping all rendering in the plugin DSL. */
test("new-session TS menu reads global active and requests a selector with an explicit null chat", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "New-session role" } });
  await runtime.api("activePrompt.setCard", { id: card.id });
  const ui = menu(runtime, null); await ui.load(); const tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["当前角色卡", card.name]);
  const response = await ui.dispatch(tree.props.onClick);
  assert.equal(response.actionResult.input.chatId, null);
  assert.equal(runtime.records.get("selector-chat").extension, null);
});

/** Prevents an old menu callback from opening a selector for a replaced chat context. */
test("retained TS menu action rejects a changed host chat without modifying bindings", async t => {
  const runtime = await selectorEnvironment(t), ui = menu(runtime, "selector-chat"); await ui.load();
  const action = ui.render().tree.props.onClick;
  assert.throws(
    /** Changes the actual retained SDK state rather than calling a test-side ownership implementation. */
    () => ui.updateHostState({ chatId: "another-chat" }),
    /Input menu chat owner changed/,
  );
  assert.throws(
    /** Dispatches the retained synchronous menu callback through the actual SDK. */
    () => ui.dispatch(action), /Input menu chat owner changed/,
  );
  assert.equal(runtime.records.get("selector-chat").extension, null);
});
