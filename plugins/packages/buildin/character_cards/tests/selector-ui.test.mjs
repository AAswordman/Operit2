import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { selectorEnvironment } from "./selector-environment.mjs";
import { composeNodes, keyedNode, mountRegisteredSelector } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";

/** Creates the explicit plugin request used by the existing generic modal host, including global-null semantics. */
function presentation(requestId, chatId, kind = "card", selected = null) { return { requestId, input: { mode: "select", chatId, kind, selected } }; }
/** Counts actual host writes rather than declaring any successful plugin operation in a fixture. */
function writes(runtime) { return runtime.calls.filter(
  /** Audits the exact generic record-extension method. */
  call => call.method === "writeExtension",
); }
/** Reads genuine file-owned state for cancellation and persistence assertions independently from the service. */
async function stateBytes(runtime) { return readFile(path.join(runtime.disk.directory, "character-memory", "state.json")); }
/** Opens one actually registered SDK-rendered selector and loads its actual records through main IPC. */
async function ready(runtime, input) { const ui = mountRegisteredSelector(runtime, input); await ui.load(); return ui; }

/** Verifies the real registered handler creates one native Dialog and reproduces the original compact geometry and rows. */
test("registered native selector loads real records, avatars, current checks and one 360x420 modal", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Native role", description: "Actual saved description", avatarUri: "runtime/data/user_assets/character_avatars/native.png" } });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:" + card.id });
  const ui = mountRegisteredSelector(runtime, presentation("native-shape", "selector-chat", "card", { CharacterCard: { id: "default" } }));
  assert.equal(ui.first.tree.type, "Dialog"); assert.equal(composeNodes(ui.first.tree,
    /** Checks genuine native loading rather than a static text marker. */
    node => node.type === "CircularProgressIndicator",
  ).length, 1);
  await ui.load(); const tree = ui.render().tree;
  assert.equal(composeNodes(tree,
    /** Requires exactly one actual Dialog in the entire SDK-rendered tree. */
    node => node.type === "Dialog",
  ).length, 1);
  assert.equal(composeNodes(tree,
    /** The selector must never construct a WebView or management page. */
    node => node.type === "WebView",
  ).length, 0);
  assert.equal(tree.props.closeOnDismissRequest, false); assert.equal(tree.props.properties.usePlatformDefaultWidth, false);
  const body = tree.children[0]; assert.equal(body.type, "Column"); assert.equal(body.props.height, 420);
  assert.deepEqual(plain(body.props.modifier.__modifierOps), [{ name: "widthIn", args: [{ maxWidth: 360 }] }, { name: "heightIn", args: [{ minHeight: 420, maxHeight: 420 }] }]);
  const option = keyedNode(tree, "card:" + card.id);
  assert.equal(option.type, "Row"); assert.equal(typeof option.props.onClick.__actionId, "string");
  assert.equal(composeNodes(option,
    /** Requires the actual saved URI and crop scale on the native Image node. */
    node => node.type === "Image" && node.props.uri === card.avatarUri && node.props.contentScale === "crop",
  ).length, 1);
  assert.equal(composeNodes(option,
    /** Uses a fresh actual extension marker, not the deliberately stale caller check mark. */
    node => node.type === "Icon" && node.props.name === "Check",
  ).length, 1);
  assert.equal(composeNodes(tree,
    /** Displays the native list's actual description without substituting a sample value. */
    node => node.type === "Text" && node.props.text === card.description,
  ).length, 1);
  assert.equal(composeNodes(tree,
    /** Verifies the real declared static image resource for genuinely avatar-free records. */
    node => node.type === "Image" && typeof node.props.path === "string",
  ).length, 1);
});

/** Exercises the actual SDK callback dispatch and waits for the one real namespace write before V1 completion. */
test("one native row click commits the current chat exactly once before real SDK JSON completion", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Click commits native role" } });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:default" });
  runtime.records.get("selector-chat").extension.notes = { text: "Unrelated own-namespace value" };
  const active = plain(await runtime.api("activePrompt.get", {})), ui = await ready(runtime, presentation("native-click", "selector-chat"));
  const before = writes(runtime).length, row = keyedNode(ui.render().tree, "card:" + card.id);
  const response = await ui.dispatch(row.props.onClick);
  assert.deepEqual(plain(response.actionResult), { type: "toolpkg.presentation.complete", requestId: "native-click", value: { selection: "card:" + card.id, contextKey: "card:" + card.id } });
  assert.equal(writes(runtime).length, before + 1);
  assert.deepEqual(runtime.records.get("selector-chat").extension, { version: 1, selection: "card:" + card.id, notes: { text: "Unrelated own-namespace value" } });
  assert.deepEqual(plain(await runtime.api("activePrompt.get", {})), active);
  await assert.rejects(ui.dispatch(row.props.onClick), /already finished/); assert.equal(writes(runtime).length, before + 1);
});

/** Selects a genuine stored role group with the same immediate-click flow and no editor/confirmation substitution. */
test("native group selector shows genuine groups and commits the selected group immediately", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Group member" } });
  const group = await runtime.api("group.create", { values: { name: "Native role group", description: "Saved group description", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  const ui = await ready(runtime, presentation("native-group", "selector-chat", "group"));
  const tree = ui.render().tree; assert.equal(keyedNode(tree, "group:" + group.id).type, "Row");
  assert.equal(composeNodes(tree,
    /** Requires the group glyph and does not fabricate a character image for a group. */
    node => node.type === "Icon" && node.props.name === "Groups",
  ).length, 1);
  assert.equal(composeNodes(tree,
    /** The original UX contains no newly invented confirmation button. */
    node => node.type === "Button" || node.type === "TextButton",
  ).length, 0);
  const response = await ui.dispatch(keyedNode(tree, "group:" + group.id).props.onClick);
  assert.deepEqual(plain(response.actionResult), { type: "toolpkg.presentation.complete", requestId: "native-group", value: { selection: "group:" + group.id, contextKey: "group:" + group.id } });
  assert.equal(runtime.records.get("selector-chat").extension.selection, "group:" + group.id);
});

/** Preserves the explicit null target's old global-active mode and proves later source-free initialization consumes it. */
test("native null-chat selector persists real global active without writing a chat namespace", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "First-create active choice" } });
  const menu = await runtime.api("chat.context.actions", { chatId: null });
  assert.equal(menu.selectors[0].routeId, "toolpkg:com.operit.character_cards:ui:selection"); assert.equal(menu.selectors[0].input.chatId, null);
  runtime.calls.length = 0; const ui = await ready(runtime, { requestId: "native-global", input: menu.selectors[0].input });
  const response = await ui.dispatch(keyedNode(ui.render().tree, "card:" + card.id).props.onClick);
  assert.deepEqual(plain(response.actionResult), { type: "toolpkg.presentation.complete", requestId: "native-global", value: { selection: "card:" + card.id, contextKey: "card:" + card.id } });
  assert.deepEqual(plain(await runtime.api("activePrompt.get", {})), { CharacterCard: { id: card.id } });
  assert.deepEqual(runtime.calls, []);
  const initialized = await runtime.main.beforeChatCreate({ eventName: "before_create", eventPayload: { eventName: "before_create", creationKind: "new", chat: { id: "fresh-native-chat", title: "Source-free", workspaceId: null, parentChatId: null }, sourceChatId: null, sourceMessageTimestamp: null, input: null, sourceExtension: null } });
  assert.deepEqual(plain(initialized), { extension: { version: 1, selection: "card:" + card.id } });
});

/** Keeps a real unowned namespace selectable without borrowing global active or interpreting a legacy role field. */
test("unbound existing chat menu stays reachable with null identity and an initially unchecked native selector", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Explicit user binds unowned chat" } });
  const active = plain(await runtime.api("activePrompt.get", {})); const menu = await runtime.api("chat.context.actions", { chatId: "selector-chat" });
  assert.equal(menu.identity, null); assert.equal(menu.selectors.length, 2); assert.equal(menu.selectors[0].input.selected, null); assert.equal(menu.selectors[1].input.selected, null);
  const ui = await ready(runtime, { requestId: "native-unbound", input: menu.selectors[0].input });
  assert.equal(composeNodes(ui.render().tree,
    /** Only an actual namespace marker may supply the check; global active cannot check an unbound chat row. */
    node => node.type === "Icon" && node.props.name === "Check",
  ).length, 0);
  await ui.dispatch(keyedNode(ui.render().tree, "card:" + card.id).props.onClick);
  assert.equal(runtime.records.get("selector-chat").extension.selection, "card:" + card.id); assert.deepEqual(plain(await runtime.api("activePrompt.get", {})), active);
});

/** Verifies both explicit target modes cancel with V1 and no stored role or namespace changes. */
test("native close cancels global and existing-chat selectors without any binding or active writes", async t => {
  const runtime = await selectorEnvironment(t);
  await runtime.api("character.create", { values: { name: "Cancel boundary saved role" } });
  for (const chatId of [null, "selector-chat"]) {
    const ui = await ready(runtime, presentation(chatId === null ? "cancel-global" : "cancel-chat", chatId));
    const before = await stateBytes(runtime), count = writes(runtime).length;
    const response = await ui.dispatch(keyedNode(ui.render().tree, "selector-close").props.onClick);
    assert.deepEqual(plain(response.actionResult), { type: "toolpkg.presentation.cancel", requestId: chatId === null ? "cancel-global" : "cancel-chat" });
    assert.deepEqual(await stateBytes(runtime), before); assert.equal(writes(runtime).length, count); assert.equal(runtime.records.get("selector-chat").extension, null);
  }
});

/** Holds the actual host write and verifies every native row and dismissal affordance refuses concurrent completion. */
test("native selector disables clicks and close during the one actual pending binding write", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Busy row role" } });
  const ui = await ready(runtime, presentation("native-busy", "selector-chat")), row = keyedNode(ui.render().tree, "card:" + card.id), close = keyedNode(ui.render().tree, "selector-close");
  const gate = runtime.holdWrite(), pending = ui.dispatch(row.props.onClick);
  for (let turn = 0; turn < 100 && writes(runtime).length === 0; turn += 1) await new Promise(
    /** Lets the genuine backend operation reach the explicitly controlled native write barrier. */
    resolve => setTimeout(resolve, 1),
  );
  assert.equal(writes(runtime).length, 1); const busyTree = ui.render().tree;
  assert.equal(Object.hasOwn(keyedNode(busyTree, "card:" + card.id).props, "onClick"), false); assert.equal(keyedNode(busyTree, "selector-close").props.enabled, false);
  assert.equal(busyTree.props.properties.dismissOnBackPress, false); assert.equal(busyTree.props.properties.dismissOnClickOutside, false);
  await assert.rejects(ui.dispatch(row.props.onClick), /already running/);
  assert.throws(
    /** The real SDK dispatch propagates this synchronous stale-close rejection before returning any action result. */
    () => ui.dispatch(close.props.onClick), /already running/,
  );
  assert.equal(writes(runtime).length, 1); gate.release(); const result = await pending;
  assert.equal(result.actionResult.type, "toolpkg.presentation.complete"); assert.equal(writes(runtime).length, 1);
});

/** Propagates a real native failure and leaves the same request open, actionable and explicitly in error. */
test("failed native binding commit stays in the same modal and permits explicit cancellation", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Failure visible role" } });
  const ui = await ready(runtime, presentation("native-error", "selector-chat")), original = new Error("Controlled original native namespace write failure");
  runtime.failNext("writeExtension", original);
  await assert.rejects(ui.dispatch(keyedNode(ui.render().tree, "card:" + card.id).props.onClick),
    /** Requires original error identity rather than replacing it with a successful or lossy response. */
    failure => failure === original,
  );
  const tree = ui.render().tree; assert.equal(tree.type, "Dialog"); assert.equal(keyedNode(tree, "selector-close").props.enabled, true);
  assert.match(keyedNode(tree, "selector-commit-error").props.text, /Controlled original native namespace write failure/); assert.equal(runtime.records.get("selector-chat").extension, null);
  const cancelled = await ui.dispatch(keyedNode(tree, "selector-close").props.onClick); assert.equal(cancelled.actionResult.type, "toolpkg.presentation.cancel");
});

/** A malformed own marker is an error, not an unbound state, and neither initial load nor rerender silently reads active. */
test("invalid real chat marker is not repaired from active and load failures are not automatically retried", async t => {
  const runtime = await selectorEnvironment(t); runtime.records.get("selector-chat").extension = { version: 77, selection: "card:default" };
  await assert.rejects(runtime.api("chat.context.actions", { chatId: "selector-chat" }), /version/);
  const ui = mountRegisteredSelector(runtime, presentation("native-bad-marker", "selector-chat")), before = runtime.calls.length;
  await assert.rejects(ui.load(), /version/); const tree = ui.render().tree;
  assert.equal(tree.type, "Dialog"); assert.match(keyedNode(tree, "selector-load-error").props.text, /version/);
  await assert.rejects(ui.load(), /version/); assert.equal(runtime.calls.length, before + 1);
  assert.equal(writes(runtime).length, 0); const result = await ui.dispatch(keyedNode(tree, "selector-close").props.onClick); assert.equal(result.actionResult.type, "toolpkg.presentation.cancel");
});

/** Requires a real select presentation and refuses missing, malformed or retargeted requests before any implicit initialization. */
test("native selector rejects malformed targets and stale presentation replacement without data writes", async t => {
  const runtime = await selectorEnvironment(t);
  for (const value of [null, { requestId: null, input: { mode: "manage" } }, { requestId: "missing-chat", input: { mode: "select", kind: "card", selected: null } }, presentation("empty-chat", ""), presentation("bad-kind", "selector-chat", "not-a-kind")]) {
    assert.throws(
      /** Mounts the actual registered route, not only a helper parser. */
      () => mountRegisteredSelector(runtime, value),
    );
  }
  const ui = mountRegisteredSelector(runtime, presentation("stale-native", "selector-chat"));
  assert.throws(
    /** Changes the real SDK-retained request and verifies the original modal cannot take its result. */
    () => ui.retarget(presentation("other-native", null)),
    /cannot change its owning presentation request/,
  );
  assert.equal(writes(runtime).length, 0); assert.equal(runtime.disk.calls.length, 0);
});

/** Rejects retained real SDK actions after host request replacement, not only the resulting rerender. */
test("stale native selector row load and cancel actions cannot write or complete a replacement request", async t => {
  const runtime = await selectorEnvironment(t), card = await runtime.api("character.create", { values: { name: "Request-owned row" } });
  const ui = await ready(runtime, presentation("stale-actions", "selector-chat")), tree = ui.render().tree;
  const row = keyedNode(tree, "card:" + card.id), close = keyedNode(tree, "selector-close"), count = runtime.calls.length;
  assert.throws(
    /** Replaces the actual SDK-retained generic presentation while preserving the old encoded callback identities. */
    () => ui.retarget(presentation("replacement-actions", null)), /cannot change its owning presentation request/,
  );
  await assert.rejects(ui.dispatch(row.props.onClick), /cannot change its owning presentation request/);
  assert.throws(
    /** A synchronous retained cancellation must reject before producing any V1 terminal value. */
    () => ui.dispatch(close.props.onClick), /cannot change its owning presentation request/,
  );
  await assert.rejects(ui.load(), /cannot change its owning presentation request/);
  assert.equal(runtime.calls.length, count); assert.equal(runtime.records.get("selector-chat").extension, null);
});
