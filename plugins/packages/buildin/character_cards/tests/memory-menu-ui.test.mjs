import assert from "node:assert/strict";
import test from "node:test";
import { readFile, writeFile } from "node:fs/promises";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, composeNodes, keyedNode } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";
import { createMemoryJobsHarness, CONTROLLED_CHAT_ID, CONTROLLED_OWNER } from "./memory-jobs-runtime.mjs";

/** Records timers so menu tests can reject background polling. */
function timers() {
  const pending = [];
  return { pending, globals: { setTimeout(callback, delay) { pending.push({ callback, delay }); return pending.length; }, clearTimeout() {} } };
}
/** Mounts the registered memory menu for one supplied chat. */
function menu(runtime, chatId) { return mountRegisteredComposeRoute(runtime, null, "chat-input-memory", { chatId, input: {} }); }
/** Collects text from the real rendered Compose tree. */
function texts(tree) { return composeNodes(tree, node => node.type === "Text").map(node => node.props.text); }
/** Requires exactly one Switch inside a keyed row and returns its real props. */
function rowSwitch(tree, key) {
  const found = [];
  (function visit(node) { if (node.type === "Switch") found.push(node); for (const child of node.children ?? []) visit(child); })(keyedNode(tree, key));
  assert.equal(found.length, 1, "Expected one switch row: " + key);
  return found[0];
}

// Compare the actual registered plugin tree with main's group geometry, labels and persisted preferences.
test("plugin memory section retains main's exact collapsed and expanded controls and four summaries", async t => {
  const clock = timers(), runtime = await selectorEnvironment(t, {}, clock.globals);
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  const entries = runtime.navigation.filter(entry => entry.surface === "chat_input_menu_sections");
  assert.equal(entries.length, 1); assert.equal(entries[0].route, "toolpkg:com.operit.character_cards:ui:chat-input-memory");
  const ui = menu(runtime, "selector-chat"); await ui.load(); let tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["记忆", "用户资料 · 记忆库更新"]);
  const collapsed = plain(tree);
  const header = keyedNode(tree, "memory-section-header");
  assert.equal(header.props.paddingHorizontal, 12);
  assert.deepEqual(plain(header.props.modifier.__modifierOps), [{ name: "heightIn", args: [{ minHeight: 36 }] }]);
  await ui.dispatch(header.props.onClick); tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["记忆", "用户资料 · 记忆库更新", "提供用户资料", "开", "自动更新记忆库", "开", "自动检索注入记忆", "开", "记忆注入条数", "允许重复命中", "关", "手动更新记忆"]);
  assert.deepEqual(plain(keyedNode(tree, "memory-section-children").props.modifier.__modifierOps), [{ name: "padding", args: [12, 0, 8, 4] }]);
  for (const control of composeNodes(tree, node => node.type === "Switch")) {
    assert.equal(control.props.width, 40); assert.equal(control.props.height, 28);
  }
  for (const key of ["memory-userDescriptionEnabled", "memory-autoUpdateEnabled", "memory-memoryRecall"]) assert.equal(rowSwitch(tree, key).props.checked, true);
  assert.equal(rowSwitch(tree, "memory-memoryRecallRepeat").props.checked, false);
  assert.equal(keyedNode(tree, "memory-recall-limit-input").props.value, "3");
  await ui.dispatch(keyedNode(tree, "memory-recall-limit-input").props.onValueChange, "5"); tree = ui.render().tree;
  assert.equal(keyedNode(tree, "memory-recall-limit-input").props.value, "5");
  assert.deepEqual(plain(await runtime.web({ action: "readMemoryRecallSettings" })), { enabled: true, limit: 5, allowRepeated: false });
  await ui.dispatch(keyedNode(tree, "memory-recall-limit-input").props.onValueChange, "abc"); tree = ui.render().tree;
  await ui.dispatch(keyedNode(tree, "memory-recall-limit-input").props.onValueChange, "51"); tree = ui.render().tree;
  assert.deepEqual(plain(await runtime.web({ action: "readMemoryRecallSettings" })), { enabled: true, limit: 5, allowRepeated: false });
  assert.equal(keyedNode(tree, "memory-recall-limit-input").props.value, "51");
  await ui.dispatch(keyedNode(tree, "memory-memoryRecallRepeat").props.onClick); tree = ui.render().tree;
  assert.deepEqual(plain(await runtime.web({ action: "readMemoryRecallSettings" })), { enabled: true, limit: 5, allowRepeated: true });
  await ui.dispatch(keyedNode(tree, "memory-memoryRecallRepeat").props.onClick); tree = ui.render().tree;
  await ui.dispatch(keyedNode(tree, "memory-recall-limit-input").props.onValueChange, "3"); tree = ui.render().tree;
  assert.deepEqual(plain(await runtime.web({ action: "readMemoryRecallSettings" })), { enabled: true, limit: 3, allowRepeated: false });
  if (process.env.OPERIT_EXPORT_MEMORY_MENU_FIXTURE === "1") {
    await writeFile(new URL("../../../../../apps/flutter/app/test/fixtures/native_memory_menu.json", import.meta.url), JSON.stringify({ collapsed, expanded: plain(tree) }, null, 2) + "\n");
  }
  await ui.dispatch(keyedNode(tree, "memory-autoUpdateEnabled").props.onClick);
  tree = ui.render().tree; assert.equal(texts(tree)[1], "用户资料");
  await ui.dispatch(keyedNode(tree, "memory-userDescriptionEnabled").props.onClick);
  tree = ui.render().tree; assert.equal(texts(tree)[1], "关");
  await ui.dispatch(keyedNode(tree, "memory-autoUpdateEnabled").props.onClick);
  tree = ui.render().tree; assert.equal(texts(tree)[1], "记忆库更新");
  assert.deepEqual(plain(await runtime.web({ action: "readGlobalMemoryPreferences" })), { userDescriptionEnabled: false, autoUpdateEnabled: true });
  const next = menu(runtime, "selector-chat"); await next.load(); assert.equal(texts(next.render().tree)[1], "记忆库更新");
  assert.deepEqual(clock.pending, []);
});

for (const selection of [null, "model:chat", "group:controlled-group"]) {
  /** Keeps conversations without a single character owner out of the role-memory controls. */
  test(`memory menu omits role-memory controls for selection ${selection}`, async t => {
    const clock = timers(), runtime = await selectorEnvironment(t, {}, clock.globals);
    if (selection !== null) await runtime.seedBinding("selector-chat", { version: 1, selection });
    const ui = menu(runtime, "selector-chat"); await ui.load();
    const tree = ui.render().tree;
    assert.equal(tree.props.key, "memory-menu-unavailable");
    assert.deepEqual(texts(tree), []);
    assert.deepEqual(composeNodes(tree,
      /** Finds actions that must not be exposed without a character memory owner. */
      node => node.props.onClick !== undefined || node.props.onLoad !== undefined,
    ), []);
    assert.deepEqual(clock.pending, []);
  });
}

/** Preserves genuine ownership errors instead of hiding a broken character binding. */
test("memory menu reports a missing selected character", async t => {
  const runtime = await selectorEnvironment(t);
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:missing-character" });
  const ui = menu(runtime, "selector-chat");
  await assert.rejects(ui.load(), /missing-character/);
});

test("memory info reads fresh queue status on demand without background polling", async t => {
  const clock = timers(), harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(clock.globals);
  await runtime.initialize(); await runtime.bind();
  const ui = menu(runtime, CONTROLLED_CHAT_ID); await ui.load();
  await runtime.enqueueReplies();
  const result = await ui.dispatch(keyedNode(ui.render().tree, "memory-info").props.onClick);
  const queue = await runtime.web({ action: "readMemoryAutoSaveStatus", ownerKey: CONTROLLED_OWNER });
  assert.equal(queue.pendingCandidates, 5); assert.equal(queue.pendingChats, 1);
  const description = `待处理 5 条 · 1 个聊天\n处理中 0 · 失败 0\n下次检查：约 ${queue.minutesUntilNextRun} 分钟后\n\n自动检查需要至少 5 个候选；不足时继续等待。手动更新不受此门槛限制。`;
  assert.deepEqual(plain(result.actionResult), { type: "toolpkg.ui.present", routeId: "toolpkg:com.operit.character_cards:ui:memory-status", input: { description } });
  const dialog = mountRegisteredComposeRoute(runtime, { requestId: "memory-status-test", input: result.actionResult.input }, "memory-status");
  const tree = dialog.render().tree;
  assert.equal(tree.type, "AlertDialog"); assert.equal(tree.props.title, "记忆状态"); assert.equal(tree.props.text, description);
  if (process.env.OPERIT_EXPORT_MEMORY_MENU_FIXTURE === "1") {
    const target = new URL("../../../../../apps/flutter/app/test/fixtures/native_memory_menu.json", import.meta.url);
    const fixture = JSON.parse(await readFile(target, "utf8")); fixture.status = plain(tree);
    await writeFile(target, JSON.stringify(fixture, null, 2) + "\n");
  }
  assert.equal(tree.props.confirmText, "关闭"); assert.equal(tree.props.dismissText, undefined);
  assert.deepEqual(plain((await dialog.dispatch(tree.props.onConfirm)).actionResult), { type: "toolpkg.presentation.cancel", requestId: "memory-status-test" });
  assert.deepEqual(clock.pending, []);
});

test("manual menu action runs real extraction while automatic updates are off and reports original snackbar text", async t => {
  const clock = timers(), harness = await createMemoryJobsHarness(t), runtime = harness.openRuntime(clock.globals);
  await runtime.initialize(); await runtime.bind();
  await runtime.web({ action: "writeGlobalMemoryPreference", key: "autoUpdateEnabled", value: false });
  const ui = menu(runtime, CONTROLLED_CHAT_ID); await ui.load(); await ui.dispatch(keyedNode(ui.render().tree, "memory-section-header").props.onClick);
  harness.queueAnalysis({ main: null, new: [], update: [], merge: [], links: [], profile_markdown: null });
  const result = await ui.dispatch(keyedNode(ui.render().tree, "memory-manual").props.onClick);
  assert.deepEqual(plain(result.actionResult), { type: "toolpkg.ui.message", message: "记忆提取完成" });
  assert.equal(harness.modelCalls.length, 1); harness.assertOutputsConsumed();
  assert.equal(texts(ui.render().tree).at(-1), "手动更新记忆");
  harness.queueExecutionError(new Error("controlled extraction failure"));
  const failed = await ui.dispatch(keyedNode(ui.render().tree, "memory-manual").props.onClick);
  assert.match(failed.actionResult.message, /^记忆提取失败：.*controlled extraction failure/);
  assert.equal(texts(ui.render().tree).at(-1), "手动更新记忆");
});

test("new-session memory menu keeps the original disabled manual action and no queue state", async t => {
  const clock = timers(), runtime = await selectorEnvironment(t, {}, clock.globals), ui = menu(runtime, null); await ui.load();
  await ui.dispatch(keyedNode(ui.render().tree, "memory-section-header").props.onClick);
  assert.equal(keyedNode(ui.render().tree, "memory-manual").props.onClick, undefined);
  const info = await ui.dispatch(keyedNode(ui.render().tree, "memory-info").props.onClick);
  assert.equal(info.actionResult.input.description, "暂无队列状态。自动提取会定期检查当前记忆库。");
});

test("memory section refuses retained actions and status reads after host chat ownership changes", async t => {
  const clock = timers(), runtime = await selectorEnvironment(t, {}, clock.globals);
  await runtime.seedBinding("selector-chat", { version: 1, selection: "card:default" });
  const ui = menu(runtime, "selector-chat"); await ui.load();
  const action = keyedNode(ui.render().tree, "memory-section-header").props.onClick;
  const info = keyedNode(ui.render().tree, "memory-info").props.onClick;
  assert.throws(() => ui.updateHostState({ chatId: "another-chat" }), /Memory menu chat owner changed/);
  assert.throws(() => ui.dispatch(action), /Memory menu chat owner changed/);
  await assert.rejects(ui.dispatch(info), /Memory menu chat owner changed/);
  assert.deepEqual(clock.pending, []);
});
