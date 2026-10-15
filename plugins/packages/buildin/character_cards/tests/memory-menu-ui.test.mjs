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

// Compare the actual registered plugin tree with main's group geometry, labels and persisted preferences.
test("plugin memory section retains main's exact collapsed and expanded controls and four summaries", async t => {
  const clock = timers(), runtime = await selectorEnvironment(t, {}, clock.globals);
  const entries = runtime.navigation.filter(entry => entry.surface === "chat_input_menu_sections");
  assert.equal(entries.length, 1); assert.equal(entries[0].route, "toolpkg:com.operit.character_cards:ui:chat-input-memory");
  const ui = menu(runtime, "selector-chat"); await ui.load(); let tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["记忆", "用户资料 · 记忆库更新"]);
  const collapsed = plain(tree);
  const header = keyedNode(tree, "memory-section-header");
  assert.equal(header.props.paddingHorizontal, 12);
  assert.deepEqual(plain(header.props.modifier.__modifierOps), [{ name: "heightIn", args: [{ minHeight: 36 }] }]);
  await ui.dispatch(header.props.onClick); tree = ui.render().tree;
  assert.deepEqual(texts(tree), ["记忆", "用户资料 · 记忆库更新", "提供用户资料", "开", "自动更新记忆库", "开", "手动更新记忆"]);
  assert.deepEqual(plain(keyedNode(tree, "memory-section-children").props.modifier.__modifierOps), [{ name: "padding", args: [12, 0, 8, 4] }]);
  for (const control of composeNodes(tree, node => node.type === "Switch")) {
    assert.equal(control.props.width, 40); assert.equal(control.props.height, 28); assert.equal(control.props.checked, true);
  }
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
  const clock = timers(), runtime = await selectorEnvironment(t, {}, clock.globals), ui = menu(runtime, "selector-chat"); await ui.load();
  const action = keyedNode(ui.render().tree, "memory-section-header").props.onClick;
  const info = keyedNode(ui.render().tree, "memory-info").props.onClick;
  assert.throws(() => ui.updateHostState({ chatId: "another-chat" }), /Memory menu chat owner changed/);
  assert.throws(() => ui.dispatch(action), /Memory menu chat owner changed/);
  await assert.rejects(ui.dispatch(info), /Memory menu chat owner changed/);
  assert.deepEqual(clock.pending, []);
});
