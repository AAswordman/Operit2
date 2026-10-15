import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, composeNodes, keyedNode } from "./selector-render.mjs";
import { plain } from "./runtime.mjs";
import { buildUiScreenScripts } from "../scripts/build.mjs";

const composeModules = await buildUiScreenScripts();

function mount(plugin, mode, entity, id, requestId = "native-entity") {
  return mountRegisteredComposeRoute(plugin, { requestId, input: { mode, entity, id } }, "main");
}
function oneSurface(tree) {
  assert.equal(tree.type, "Dialog");
  assert.equal(composeNodes(tree, node => node.type === "Dialog" || node.type === "AlertDialog").length, 1);
  assert.equal(composeNodes(tree, node => node.type === "WebView").length, 0);
  assert.equal(tree.props.title, undefined);
  assert.equal(tree.props.closeOnDismissRequest, false);
}
async function click(ui, key, prop = "onClick", payload) { return ui.dispatch(keyedNode(ui.render().tree, key).props[prop], payload); }
/** Supplies only explicitly controlled independent catalogues/native chat summaries, never entity CRUD. */
async function environment(t, themes = [], runtimeGlobals = {}) {
  const plugin = await selectorEnvironment(t, {Theme: { async listConfigs() { return { configs: themes }; }
  }}, runtimeGlobals, composeModules);
  plugin.tools.Chat.listAll = async () => ({ totalCount: plugin.records.size, chats: [...plugin.records.keys()].map(id => ({ id, title: "Controlled native conversation", messageCount: 0, createdAt: "2026-10-08T12:00:00Z", updatedAt: "2026-10-08T12:00:00Z", isCurrent: true, inputTokens: 0, outputTokens: 0 })) });
  return plugin;
}

test("avatar preview uses one native surface without loading HTML, and can cover the whole window", async t => {
  const plugin = await environment(t);
  const card = await plugin.api("character.create", { values: { name: "原生角色预览", characterSetting: "真实完整设定" } });
  const actions = await plugin.api("chat.context.actions", { chatId: null });
  assert.equal(actions.identity.avatarAction.input.mode, "preview");
  const ui = mount(plugin, "preview", "card", card.id);
  oneSurface(ui.first.tree); await ui.load(); const tree = ui.render().tree; oneSurface(tree);
  assert.ok(composeNodes(tree, node => node.props.text === "真实完整设定").length > 0);
  assert.equal(tree.props.properties.fullscreen, false);
  await click(ui, "entity-primary"); await click(ui, "entity-expand-characterSetting"); const full = ui.render().tree; oneSurface(full);
  assert.equal(full.props.properties.fullscreen, true);
  assert.equal(keyedNode(full, "native-entity-panel").props.fillMaxSize, true);
  assert.equal(keyedNode(full, "native-entity-panel").props.width, undefined);
  await click(ui, "entity-cancel"); assert.equal(ui.render().tree.props.properties.fullscreen, false);
  await click(ui, "entity-cancel");
  const result = await click(ui, "entity-close");
  assert.deepEqual(plain(result.actionResult), { type: "toolpkg.presentation.complete", requestId: "native-entity", value: { mode: "preview", entity: "card", id: card.id } });
});

test("native preview reads saved content in one page and expands named configuration without writes", async t => {
  const plugin = await environment(t);
  const tag = await plugin.api("tag.create", { values: { name: "温和可靠", description: "", promptContent: "耐心回答", tagType: "TONE" } });
  const store = await plugin.api("memory.shared.create", { name: "日常对话记忆" });
  const card = await plugin.api("character.create", { values: {
    name: "温和的伙伴", description: "一个温和、可靠的伙伴，陪你整理思路，把想法变成行动。",
    characterSetting: "你是一位可靠、温和的助手。\n认真倾听，给出清楚、具体的回答；遇到不确定的事情，坦诚说明。",
    openingStatement: "你好，今天有什么想一起聊聊的？", otherContentChat: "保持自然的交流节奏，用简洁的语言表达。",
    otherContentVoice: "", advancedCustomPrompt: "", marks: "记录每次对话中值得记住的细节。", attachedTagIds: [tag.id],
    chatModelBindingMode: "FIXED_MODEL", chatModelId: "qwen-plus", ttsConfigId: "tts-one", memoryBindingMode: "SHARED", sharedMemoryId: store.id,
    sharedMemoryMounts: [{ sharedMemoryId: store.id, readable: true, writable: false }],
    toolAccessConfig: { enabled: true, allowedBuiltinTools: ["fixture-builtin"], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] },
  } });
  const before = await plugin.disk.stateBytes(), ui = mount(plugin, "preview", "card", card.id); await ui.load(); const tree = ui.render().tree;
  oneSurface(tree); assert.equal(keyedNode(tree, "entity-preview-body").type, "LazyColumn");
  assert.equal(keyedNode(tree, "entity-avatar").props.width, 72);
  assert.equal(keyedNode(tree, "entity-preview-name").props.text, "温和的伙伴");
  assert.equal(composeNodes(tree, node => ["PrimaryTabRow", "HorizontalPager", "TextField"].includes(node.type)).length, 0);
  assert.ok(composeNodes(tree, node => node.props.text === "温和可靠").length);
  assert.equal(composeNodes(tree, node => node.props.key === "entity-preview-config-details").length, 0);
  assert.equal(composeNodes(tree, node => ["entity-preview-voice", "entity-preview-prompt"].includes(node.props.key)).length, 0);
  const sections = composeNodes(tree, node => ["entity-preview-opening", "entity-preview-setting", "entity-preview-chat", "entity-preview-notes"].includes(node.props.key));
  assert.deepEqual(sections.map(node => node.props.key), ["entity-preview-opening", "entity-preview-setting", "entity-preview-chat", "entity-preview-notes"]);
  if (process.env.CHARACTER_CARDS_WRITE_NATIVE_FIXTURE === "1") {
    const { writeFile } = await import("node:fs/promises");
    await writeFile(new URL("../../../../../apps/flutter/app/test/fixtures/native_character_preview.json", import.meta.url), JSON.stringify(plain(tree), null, 2) + "\n");
  }
  await click(ui, "entity-preview-config-toggle"); const expanded = ui.render().tree;
  assert.equal(keyedNode(expanded, "entity-preview-config-details").type, "Column");
  const texts = composeNodes(expanded, node => node.type === "Text").map(node => node.props.text);
  for (const value of ["DashScope · qwen-plus", "测试语音", "日常对话记忆", "日常对话记忆（可读）"]) assert.ok(texts.includes(value), value);
  assert.ok(!texts.includes(store.id)); assert.ok(!texts.some(value => typeof value === "string" && value.includes('"allowedBuiltinTools"')));
  assert.deepEqual(await plugin.disk.stateBytes(), before);
  if (process.env.CHARACTER_CARDS_WRITE_NATIVE_FIXTURE === "1") {
    const { writeFile } = await import("node:fs/promises");
    await writeFile(new URL("../../../../../apps/flutter/app/test/fixtures/native_character_preview_expanded.json", import.meta.url), JSON.stringify(plain(expanded), null, 2) + "\n");
  }
  await click(ui, "entity-preview-config-toggle"); assert.equal(composeNodes(ui.render().tree, node => node.props.key === "entity-preview-config-details").length, 0);
  await click(ui, "entity-primary"); assert.equal(keyedNode(ui.render().tree, "entity-pager").type, "HorizontalPager");
  await click(ui, "entity-cancel"); assert.equal(composeNodes(ui.render().tree, node => node.type === "HorizontalPager").length, 0);
});

test("preview configuration resolves the independent theme name only when expanded", async t => {
  const plugin = await environment(t, [{ id: "preview-independent-theme", name: "柔和日间主题" }]);
  for (const entity of ["card", "group"]) {
    const api = entity === "card" ? "character" : "group";
    const record = await plugin.api(api + ".create", { values: { name: "主题预览-" + entity, themeConfigId: "preview-independent-theme" } });
    const before = await plugin.disk.stateBytes(), ui = mount(plugin, "preview", entity, record.id);
    await ui.load(); await click(ui, "entity-preview-config-toggle");
    const deadline = Date.now() + 2000;
    while (!composeNodes(ui.render().tree, node => node.props.text === "柔和日间主题").length && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 5));
    }
    assert.ok(composeNodes(ui.render().tree, node => node.props.text === "柔和日间主题").length);
    assert.equal(composeNodes(ui.render().tree, node => node.props.text === "preview-independent-theme").length, 0);
    assert.deepEqual(await plugin.disk.stateBytes(), before);
    assert.equal((await click(ui, "entity-close")).actionResult.type, "toolpkg.presentation.complete");
  }
});

test("native preview hides empty reading sections and preserves long multiline content", async t => {
  const plugin = await environment(t), content = "长篇设定，完整保留。\n".repeat(120);
  const card = await plugin.api("character.create", { values: { name: "长篇角色", description: "", openingStatement: "", characterSetting: content, otherContentChat: "", otherContentVoice: "", advancedCustomPrompt: "", marks: "" } });
  const ui = mount(plugin, "preview", "card", card.id); await ui.load(); const tree = ui.render().tree;
  assert.equal(composeNodes(tree, node => node.props.text === content).length, 1);
  for (const key of ["description", "opening", "chat", "voice", "prompt", "notes"]) assert.equal(composeNodes(tree, node => node.props.key === "entity-preview-" + key).length, 0);
  assert.equal(composeNodes(tree, node => node.props.text === "未设置").length, 0);
});

test("native group preview shows ordered member portraits and descriptions in the reading layout", async t => {
  const plugin = await environment(t), first = await plugin.api("character.create", { values: { name: "温和的倾听者", description: "认真听你说，也帮你梳理情绪。" } });
  const second = await plugin.api("character.create", { values: { name: "理性的思考者", description: "提供不同角度，一起寻找清晰的答案。" } });
  const group = await plugin.api("group.create", { values: { name: "一起想想", description: "不同的视角，同一个话题。", members: [{ characterCardId: second.id, orderIndex: 1 }, { characterCardId: first.id, orderIndex: 0 }] } });
  const ui = mount(plugin, "preview", "group", group.id); await ui.load(); const tree = ui.render().tree;
  oneSurface(tree); assert.equal(composeNodes(tree, node => node.type === "PrimaryTabRow" || node.type === "TextField").length, 0);
  const rows = composeNodes(tree, node => String(node.props.key ?? "").startsWith("entity-preview-member-"));
  assert.deepEqual(rows.map(node => node.props.key), ["entity-preview-member-" + first.id, "entity-preview-member-" + second.id]);
  assert.equal(composeNodes(tree, node => node.type === "Image").length, 2);
  for (const value of ["2 位角色", first.description, second.description]) assert.ok(composeNodes(tree, node => node.props.text === value).length);
  if (process.env.CHARACTER_CARDS_WRITE_NATIVE_FIXTURE === "1") {
    const { writeFile } = await import("node:fs/promises");
    await writeFile(new URL("../../../../../apps/flutter/app/test/fixtures/native_group_preview.json", import.meta.url), JSON.stringify(plain(tree), null, 2) + "\n");
  }
});

test("native preview edits genuine records in the same surface and completes only when preview closes", async t => {
  const plugin = await environment(t);
  const card = await plugin.api("character.create", { values: { name: "编辑原生预览", description: "旧描述", otherContentVoice: "保留语音", marks: "保留备注" } });
  const active = plain(await plugin.api("activePrompt.get", {})), ui = mount(plugin, "preview", "card", card.id);
  await ui.load(); await click(ui, "entity-primary"); oneSurface(ui.render().tree);
  await click(ui, "entity-field-description", "onValueChange", "原生字段实际保存");
  const saved = await click(ui, "entity-primary"); assert.equal(saved.actionResult, null);
  const record = plain(await plugin.api("character.get", { id: card.id }));
  assert.equal(record.description, "原生字段实际保存"); assert.equal(record.otherContentVoice, "保留语音"); assert.equal(record.marks, "保留备注");
  oneSurface(ui.render().tree);
  assert.deepEqual(plain(await plugin.api("activePrompt.get", {})), active);
  await click(ui, "entity-primary"); await click(ui, "entity-field-name", "onValueChange", "不保存的名称");
  assert.equal((await click(ui, "entity-cancel")).actionResult, null);
  assert.equal((await plugin.api("character.get", { id: card.id })).name, "编辑原生预览");
  const closed = await ui.dispatch(ui.render().tree.props.onDismissRequest);
  assert.equal(closed.actionResult.type, "toolpkg.presentation.complete");
});

test("native edit roots save/cancel/delete through the real generic terminal contract", async t => {
  const plugin = await environment(t);
  const card = await plugin.api("character.create", { values: { name: "独立原生编辑" } });
  const cancel = mount(plugin, "edit", "card", card.id, "edit-cancel"); await cancel.load();
  await click(cancel, "entity-field-name", "onValueChange", "取消草稿");
  assert.deepEqual(plain((await click(cancel, "entity-close")).actionResult), { type: "toolpkg.presentation.cancel", requestId: "edit-cancel" });
  assert.equal((await plugin.api("character.get", { id: card.id })).name, "独立原生编辑");
  const save = mount(plugin, "edit", "card", card.id, "edit-save"); await save.load();
  await click(save, "entity-field-name", "onValueChange", "保存草稿");
  assert.deepEqual(plain((await click(save, "entity-primary")).actionResult), { type: "toolpkg.presentation.complete", requestId: "edit-save", value: { mode: "edit", entity: "card", id: card.id, operation: "saved" } });
  const remove = mount(plugin, "edit", "card", card.id, "edit-delete"); await remove.load();
  await click(remove, "entity-delete"); oneSurface(remove.render().tree);
  assert.equal((await plugin.api("character.get", { id: card.id })).name, "保存草稿");
  await click(remove, "entity-cancel"); await click(remove, "entity-delete");
  const deleted = await click(remove, "entity-primary");
  assert.equal(composeNodes(remove.render().tree, node => node.props.key === "entity-error").length, 0, composeNodes(remove.render().tree, node => node.props.key === "entity-error").map(node => node.props.text).join("\n"));
  assert.deepEqual(plain(deleted.actionResult), { type: "toolpkg.presentation.complete", requestId: "edit-delete", value: { mode: "edit", entity: "card", id: card.id, operation: "deleted" } });
  assert.equal((await plugin.api("character.list", {})).some(value => value.id === card.id), false);
});

test("invalid native drafts remain visible and cancellable without writes or premature completion", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "有效角色" } });
  const ui = mount(plugin, "edit", "card", card.id); await ui.load();
  const before = await plugin.disk.stateBytes();
  await click(ui, "entity-field-name", "onValueChange", ""); assert.equal((await click(ui, "entity-primary")).actionResult, null);
  assert.match(keyedNode(ui.render().tree, "entity-error").props.text, /角色名称不能为空/);
  assert.deepEqual(await plugin.disk.stateBytes(), before); oneSurface(ui.render().tree);
  assert.equal((await click(ui, "entity-cancel")).actionResult.type, "toolpkg.presentation.cancel");
});

test("native group preview and editor preserve actual ordered member identities", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "组内角色" } });
  const group = await plugin.api("group.create", { values: { name: "原生群组", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  const ui = mount(plugin, "preview", "group", group.id); await ui.load(); oneSurface(ui.render().tree);
  assert.ok(composeNodes(ui.render().tree, node => node.props.text === "组内角色").length > 0);
  await click(ui, "entity-primary"); await click(ui, "entity-field-description", "onValueChange", "实际群组描述");
  await click(ui, "entity-primary");
  const saved = plain(await plugin.api("group.get", { id: group.id }));
  assert.equal(saved.description, "实际群组描述"); assert.deepEqual(saved.members, [{ characterCardId: card.id, orderIndex: 0 }]);
});

test("a retained native callback cannot mutate a retargeted presentation", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "不可重定向" } });
  const ui = mount(plugin, "edit", "card", card.id); await ui.load();
  const stale = keyedNode(ui.render().tree, "entity-primary").props.onClick;
  assert.throws(() => ui.retarget({ requestId: "another", input: { mode: "edit", entity: "card", id: card.id } }), /cannot change its owning request/);
  await assert.rejects(async () => ui.dispatch(stale), /cannot change its owning request/);
  assert.equal((await plugin.api("character.get", { id: card.id })).name, "不可重定向");
});

test("native label editing remains staged until the complete character is saved", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "原生标签事务" } });
  const before = await plugin.disk.stateBytes(), preview = mount(plugin, "preview", "card", card.id); await preview.load();
  await click(preview, "entity-primary"); await click(preview, "entity-tags-manage"); await click(preview, "entity-tag-create"); oneSurface(preview.render().tree);
  await click(preview, "entity-field-tag-name", "onValueChange", "取消的新标签");
  await click(preview, "entity-field-tag-promptContent", "onValueChange", "取消的提示词"); await click(preview, "entity-primary");
  assert.deepEqual(await plugin.disk.stateBytes(), before);
  await click(preview, "entity-cancel"); await click(preview, "entity-cancel"); assert.deepEqual(await plugin.disk.stateBytes(), before);
  const editor = mount(plugin, "edit", "card", card.id); await editor.load();
  await click(editor, "entity-tags-manage"); await click(editor, "entity-tag-create"); await click(editor, "entity-field-tag-name", "onValueChange", "实际原生标签");
  await click(editor, "entity-field-tag-promptContent", "onValueChange", "实际完整标签提示词"); await click(editor, "entity-primary");
  assert.deepEqual(await plugin.disk.stateBytes(), before);
  await click(editor, "entity-cancel");
  assert.equal((await click(editor, "entity-primary")).actionResult.type, "toolpkg.presentation.complete");
  const tags = await plugin.web({ action: "listTags" }), saved = await plugin.api("character.get", { id: card.id });
  const tag = tags.find(tag => tag.name === "实际原生标签"); assert.notEqual(tag, undefined);
  assert.equal(tag.promptContent, "实际完整标签提示词"); assert.deepEqual(plain(saved.attachedTagIds), [tag.id]);
});

test("native editor geometry follows main's scaffold field tabs and binding section contracts", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "原生角色卡", description: "来自 main 的对照布局", characterSetting: "角色设定正文", openingStatement: "你好。", otherContentChat: "聊天附加正文" } });
  const ui = mount(plugin, "edit", "card", card.id); await ui.load(); const tree = ui.render().tree;
  const panel = keyedNode(tree, "native-entity-panel"); assert.equal(panel.props.width, 760); assert.equal(panel.props.height, 820);
  assert.equal(tree.props.shape.cornerRadius, 28); assert.equal(tree.props.containerColor.__colorToken, "surfaceContainerHigh");
  const header = keyedNode(tree, "entity-header").props;
  assert.deepEqual([header.paddingStart, header.paddingTop, header.paddingEnd, header.paddingBottom], [24, 22, 16, 12]);
  const footer = keyedNode(tree, "entity-footer"); assert.equal(footer.type, "FlowRow");
  assert.deepEqual([footer.props.paddingStart, footer.props.paddingTop, footer.props.paddingEnd, footer.props.paddingBottom], [24, 0, 24, 20]);
  assert.equal(footer.props.spacing, 8); assert.equal(footer.props.runSpacing, 8);
  assert.equal(keyedNode(tree, "entity-tab").type, "PrimaryTabRow"); assert.equal(keyedNode(tree, "entity-pager").type, "HorizontalPager");
  for (const i of [0, 1, 2]) {
    const list = keyedNode(tree, "entity-body-" + i);
    assert.equal(list.type, "LazyColumn");
    for (const prop of ["paddingStart", "paddingTop", "paddingEnd", "paddingBottom"]) assert.equal(list.props[prop], undefined);
    const items = [...list.children, ...Object.values(list.slots ?? {}).flat()];
    assert.ok(items.length > 0);
    for (const [index, item] of items.entries()) {
      assert.equal(item.type, "Box");
      assert.deepEqual([item.props.paddingStart, item.props.paddingTop, item.props.paddingEnd, item.props.paddingBottom],
        [18, index === 0 ? 10 : 0, 18, index === items.length - 1 ? 10 : 0]);
    }
  }
  assert.equal(keyedNode(tree, "entity-field-description").props.maxLines, 1);
  assert.equal(keyedNode(tree, "entity-field-characterSetting").props.maxLines, 6);
  assert.equal(keyedNode(tree, "entity-field-openingStatement").props.maxLines, 3);
  assert.equal(keyedNode(tree, "entity-field-otherContentChat").props.maxLines, 4);
  for (const [key, lines] of [["characterSetting", 6], ["openingStatement", 3], ["otherContentChat", 4]]) {
    assert.equal(keyedNode(tree, "entity-field-" + key).props.minLines, lines);
  }
  assert.equal(keyedNode(tree, "entity-expand-characterSetting").props.width, 28);
  assert.deepEqual(plain(keyedNode(tree, "entity-expand-characterSetting").props.contentPadding), { horizontal: 0, vertical: 0 });
  assert.equal(composeNodes(tree, node => node.type === "InputDecorator" && node.props.label === "角色头像").length, 1);
  assert.equal(composeNodes(tree, node => node.type === "Image" && node.props.key === "entity-edit-avatar")[0].props.width, 44);
  for (const key of ["entity-model", "entity-theme", "entity-tts", "entity-memory", "entity-tool"]) assert.equal(keyedNode(tree, key + "-switch").type, "Switch");
  assert.equal(composeNodes(tree, node => node.type === "HorizontalDivider").length, 0);
  if (process.env.CHARACTER_CARDS_WRITE_NATIVE_FIXTURE === "1") {
    const { writeFile } = await import("node:fs/promises");
    await writeFile(new URL("../../../../../apps/flutter/app/test/fixtures/native_character_editor.json", import.meta.url), JSON.stringify(plain(tree), null, 2) + "\n");
  }
});

test("expanded native text is staged and native page swipes update the selected tab", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "完整交互", characterSetting: "原始正文" } });
  const ui = mount(plugin, "edit", "card", card.id); await ui.load();
  await click(ui, "entity-expand-characterSetting");
  await click(ui, "entity-expanded-input", "onValueChange", "取消展开草稿"); await click(ui, "entity-cancel");
  assert.equal(keyedNode(ui.render().tree, "entity-field-characterSetting").props.value, "原始正文");
  await click(ui, "entity-expand-characterSetting"); await click(ui, "entity-expanded-input", "onValueChange", "确认展开正文"); await click(ui, "entity-primary");
  assert.equal(keyedNode(ui.render().tree, "entity-field-characterSetting").props.value, "确认展开正文");
  await click(ui, "entity-pager", "onPageChanged", 2); assert.equal(keyedNode(ui.render().tree, "entity-tab").props.selectedTabIndex, 2);
  await click(ui, "entity-tab-1"); assert.equal(keyedNode(ui.render().tree, "entity-pager").props.currentPage, 1);
  const saved = await click(ui, "entity-primary"); assert.equal(saved.actionResult.type, "toolpkg.presentation.complete");
  assert.equal((await plugin.api("character.get", { id: card.id })).characterSetting, "确认展开正文");
});

function management(plugin) {
  const commands = []; let methods;
  const ui = mountRegisteredComposeRoute(plugin, null, "main", {}, command => {
    commands.push(plain(command));
    if (command.command === "addJavascriptInterface") methods = command.payload.object;
    return null;
  });
  return { ...ui, commands, async open(mode, entity, id) {
    assert.ok(methods.openEntity); return ui.dispatch(methods.openEntity, [mode, entity, id]);
  } };
}
async function loadManagementModal(ui) {
  const nodes = composeNodes(ui.render().tree, node => node.type === "Dialog");
  assert.equal(nodes.length, 1);
  await ui.dispatch(nodes[0].props.onLoad);
  return nodes[0];
}

test("management reuses the sole native entity surface for edit preview and reopen without replacing its WebView", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "管理页原生入口", description: "原始描述" } });
  const ui = management(plugin); await ui.load();
  const firstWeb = composeNodes(ui.render().tree, node => node.type === "WebView")[0];
  await ui.open("edit", "card", card.id); await loadManagementModal(ui);
  assert.equal(composeNodes(ui.render().tree, node => node.type === "WebView").length, 1);
  assert.equal(composeNodes(ui.render().tree, node => node.type === "WebView")[0].props.key, firstWeb.props.key);
  await click(ui, "entity-field-description", "onValueChange", "管理页保存到真实数据库");
  const saved = await click(ui, "entity-primary"); assert.equal(saved.actionResult, null, "Local management must not invent a host terminal event");
  assert.equal(composeNodes(ui.render().tree, node => node.type === "Dialog").length, 0);
  assert.equal((await plugin.api("character.get", { id: card.id })).description, "管理页保存到真实数据库");
  assert.match(ui.commands.at(-1).payload.script, /character-entity-closed/);
  assert.match(ui.commands.at(-1).payload.script, /saved/);
  await ui.open("preview", "card", card.id); await loadManagementModal(ui);
  assert.ok(composeNodes(ui.render().tree, node => node.props.text === "管理页保存到真实数据库").length);
  await click(ui, "entity-close");
  await ui.open("edit", "card", card.id); await loadManagementModal(ui);
  await click(ui, "entity-field-name", "onValueChange", "取消的草稿"); await click(ui, "entity-cancel");
  assert.equal((await plugin.api("character.get", { id: card.id })).name, "管理页原生入口");
});

test("management creation is a native unsaved draft and commits only on save for cards and groups", async t => {
  const plugin = await environment(t), ui = management(plugin); await ui.load();
  for (const entity of ["card", "group"]) {
    const api = entity === "card" ? "character" : "group", before = plain(await plugin.api(api + ".list", {}));
    await ui.open("create", entity, ""); await loadManagementModal(ui);
    assert.equal(composeNodes(ui.render().tree, node => node.props.key === "entity-delete" || node.props.key === "entity-export").length, 0);
    await click(ui, "entity-field-name", "onValueChange", "取消创建"); await click(ui, "entity-cancel");
    assert.deepEqual(plain(await plugin.api(api + ".list", {})), before);
    await ui.open("create", entity, ""); await loadManagementModal(ui);
    await click(ui, "entity-field-name", "onValueChange", entity === "card" ? "原生新角色" : "原生新群组");
    await click(ui, "entity-primary");
    assert.equal(composeNodes(ui.render().tree, node => node.type === "Dialog").length, 0);
    const after = plain(await plugin.api(api + ".list", {})); assert.equal(after.length, before.length + 1);
    assert.ok(after.find(value => value.name.startsWith("原生新")).id);
    assert.match(ui.commands.at(-1).payload.script, /saved/);
  }
});

test("native export saves canonical Operit and Tavern data through the host selector and preserves cancelled or failed previews", async t => {
  let saveOutcome = "cancel", destination, plugin;
  const saves = [];
  plugin = await environment(t, [], {
    /** Models only the external host save selector; serialization and actions use production code. */
    async __operitNativeComposeFileSaveCommand(payload) {
      saves.push(plain(payload));
      assert.equal(payload.executionContextKey, "selector-test");
      assert.equal(payload.routeInstanceId, "native-selector-test");
      if (saveOutcome === "error") throw new Error("Host save denied");
      if (saveOutcome === "cancel") return { cancelled: true, path: null };
      await plugin.disk.files.write(destination, payload.options.content, false);
      return { cancelled: false, path: destination };
    },
  });
  const card = await plugin.api("character.create", { values: { name: "唯一原生导出", characterSetting: "原始正文", marks: "导出保留备注" } });
  const ui = management(plugin); await ui.load(); await ui.open("edit", "card", card.id); await loadManagementModal(ui);
  await click(ui, "entity-field-characterSetting", "onValueChange", "不能导出未保存的草稿");
  for (const format of ["json", "tavern"]) {
    await click(ui, "entity-export"); await click(ui, "entity-export-" + format);
    assert.equal(composeNodes(ui.render().tree, node => node.type === "TextField").length, 0);
    saveOutcome = "cancel";
    await click(ui, "entity-export-save");
    assert.ok(keyedNode(ui.render().tree, "entity-export-save"));
    assert.equal(composeNodes(ui.render().tree, node => node.props.key === "entity-error").length, 0);
    saveOutcome = "error";
    await click(ui, "entity-export-save");
    assert.ok(composeNodes(ui.render().tree, node => node.props.key === "entity-error").length);
    const path = plugin.disk.directory + "/export-" + format + ".json";
    destination = path; saveOutcome = "save";
    await click(ui, "entity-export-save");
    assert.equal(saves.at(-1).options.suggestedName, "唯一原生导出-" + (format === "json" ? "operit" : "tavern") + ".json");
    assert.equal(saves.at(-1).options.mimeType, "application/json");
    assert.deepEqual(saves.at(-1).options.extensions, ["json"]);
    assert.equal((await plugin.disk.readBytes(path)).toString(), saves.at(-1).options.content);
    const data = JSON.parse((await plugin.disk.readBytes(path)).toString());
    if (format === "tavern") { assert.equal(data.spec, "chara_card_v2"); assert.equal(data.data.name, card.name); assert.equal(data.data.system_prompt, "原始正文"); }
    else { assert.equal(data.name, card.name); assert.equal(data.characterSetting, "原始正文"); }
    assert.equal(composeNodes(ui.render().tree, node => node.type === "Dialog").length, 1);
  }
  await click(ui, "entity-cancel");
  assert.equal((await plugin.api("character.get", { id: card.id })).characterSetting, "原始正文");
  assert.equal(saves.length, 6);
  const group = await plugin.api("group.create", { values: { name: "宿主保存群组", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  await ui.open("edit", "group", group.id); await loadManagementModal(ui);
  await click(ui, "entity-export"); await click(ui, "entity-export-json");
  destination = plugin.disk.directory + "/group.json";
  await click(ui, "entity-export-save");
  assert.equal(saves.at(-1).options.suggestedName, "宿主保存群组-operit.json");
  assert.equal((await plugin.disk.readBytes(destination)).toString(), (await plugin.api("group.export", { id: group.id })).content);
});

test("the sole native editor retains model TTS shared-memory and four-category tool bindings on the real save transaction", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "原生全部绑定" } });
  const readCatalog = plugin.tools.Catalog.readToolSources;
  plugin.tools.Catalog.readToolSources = async () => {
    const catalog = await readCatalog(); catalog.builtinTools.push({ name: "use_package", displayName: "启用外部工具", description: "真实工具策略必需的显式能力" }); return catalog;
  };
  const store = await plugin.api("memory.shared.create", { name: "原生共享库" });
  const ui = management(plugin); await ui.load(); await ui.open("edit", "card", card.id); await loadManagementModal(ui);
  await click(ui, "entity-model-switch", "onCheckedChange", true); await click(ui, "entity-model-picker"); await click(ui, "entity-picker-qwen-plus");
  await click(ui, "entity-tts-switch", "onCheckedChange", true); await click(ui, "entity-picker-tts-one");
  await click(ui, "entity-memory-switch", "onCheckedChange", true); await click(ui, "entity-store-picker"); await click(ui, "entity-picker-" + store.id);
  await click(ui, "entity-tool-switch", "onCheckedChange", true); await click(ui, "entity-tool-configure");
  const keys = ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"], names = ["fixture-builtin", "fixture-package", "fixture-skill", "fixture-mcp"];
  for (let tab = 0; tab < 4; tab++) {
    await click(ui, "entity-tool-tabs-" + tab);
    const row = keyedNode(ui.render().tree, "entity-tool-" + keys[tab] + "-" + names[tab]);
    await ui.dispatch(composeNodes(row, node => node.type === "Checkbox")[0].props.onCheckedChange, true);
  }
  await click(ui, "entity-tool-tabs-0");
  const activation = keyedNode(ui.render().tree, "entity-tool-allowedBuiltinTools-use_package");
  await ui.dispatch(composeNodes(activation, node => node.type === "Checkbox")[0].props.onCheckedChange, true);
  await click(ui, "entity-primary");
  assert.equal((await plugin.api("character.get", { id: card.id })).ttsConfigId, null);
  await click(ui, "entity-primary");
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "entity-error").map(node => node.props.text).join(" "), "");
  const saved = plain(await plugin.api("character.get", { id: card.id }));
  assert.equal(saved.chatModelBindingMode, "FIXED_MODEL"); assert.equal(saved.chatModelId, "qwen-plus"); assert.equal(saved.ttsConfigId, "tts-one");
  assert.equal(saved.memoryBindingMode, "SHARED"); assert.equal(saved.sharedMemoryId, store.id); assert.equal(saved.toolAccessConfig.enabled, true);
  for (let tab = 0; tab < 4; tab++) assert.deepEqual(saved.toolAccessConfig[keys[tab]], tab === 0 ? [names[tab], "use_package"] : [names[tab]]);
});

test("native entity content and cancellation are available while its default avatar is pending", async t => {
  const plugin = await environment(t);
  const card = await plugin.api("character.create", { values: { name: "Content before image", characterSetting: "Visible without waiting for the avatar" } });
  const open = plugin.openComposeRoute;
  let complete;
  const gate = new Promise(resolve => { complete = resolve; });
  plugin.openComposeRoute = route => {
    const page = open(route);
    const original = page.context.ToolPkg.readResource;
    page.context.ToolPkg.readResource = async (...args) => { await gate; return original(...args); };
    return page;
  };
  const ui = mount(plugin, "preview", "card", card.id);
  const loaded = await ui.load();
  assert.ok(composeNodes(loaded.tree, node => node.props.text === "Visible without waiting for the avatar").length > 0);
  assert.equal(composeNodes(loaded.tree, node => node.type === "LoadingIndicator").length, 0);
  const result = await click(ui, "entity-close");
  assert.equal(result.actionResult.type, "toolpkg.presentation.complete");
  complete();
});
