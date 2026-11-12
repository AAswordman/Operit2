import assert from "node:assert/strict";
import test from "node:test";
import { selectorEnvironment } from "./selector-environment.mjs";
import { mountRegisteredComposeRoute, composeNodes, keyedNode } from "./selector-render.mjs";
import { buildUiScreenScripts } from "../scripts/build.mjs";
import { plain } from "./runtime.mjs";
import { fixture } from "./fixtures.mjs";
const modules = await buildUiScreenScripts();

/** Opens real plugin storage and controls only independent host directories. */
async function environment(t, runtimeGlobals = {}) {
  const plugin = await selectorEnvironment(t, { Theme: { async listConfigs() { return { configs: [] }; } } }, runtimeGlobals, modules);
  plugin.tools.Chat.listAll = async () => ({ totalCount: plugin.records.size, chats: [...plugin.records.keys()].map(id => ({ id, title: id, messageCount: 0, createdAt: "2026-10-08T12:00:00Z", updatedAt: "2026-10-08T12:00:00Z", isCurrent: id === "selector-chat", inputTokens: 0, outputTokens: 0 })) });
  plugin.tools.Chat.findChat = async options => ({ matchedCount: plugin.records.has(options.query) ? 1 : 0, chat: plugin.records.has(options.query) ? { id: options.query, title: options.query, messageCount: 0, createdAt: "2026-10-08T12:00:00Z", updatedAt: "2026-10-08T12:00:00Z", isCurrent: options.query === "selector-chat", inputTokens: 0, outputTokens: 0 } : null });
  return plugin;
}
/** Dispatches the SDK's actual callback on a keyed native control. */
async function action(ui, key, prop = "onClick", payload) {
  if (key === "import-card" || key === "create-card") return ui.dispatch(keyedNode(ui.render().tree, "add-card").props.onSelect, key === "create-card" ? 0 : 1);
  return ui.dispatch(keyedNode(ui.render().tree, key).props[prop], payload);
}
/** Checks that forms and management routes have no platform view underneath. */
function nativeOnly(ui) { assert.equal(composeNodes(ui.render().tree, node => node.type === "WebView").length, 0); }
/** Enters the actual memory workspace through its native management action. */
async function memory(t, plugin, owner = "character:default") {
  let bridge; const ui = mountRegisteredComposeRoute(plugin, null, "memory", {}, command => { if (command.command === "addJavascriptInterface") bridge = command.payload.object; }); ui.graphBridge = () => bridge; await ui.load(); nativeOnly(ui);
  await ui.dispatch({ __actionId: "__operit_theme_changed" }, fixture().theme); await action(ui, "library-open:" + owner); await action(ui, "memory-workspace-root", "onLoad"); return ui;
}
/** Opens the original overflow menu before invoking one of its four actions. */
async function more(ui, key) { return action(ui, "memory-more", "onSelect", ["memory-settings-open", "memory-refresh", "memory-import", "memory-export"].indexOf(key)); }
/** Submits complete memory values through the real service. */
async function seed(plugin, ownerKey, title, folderPath = "") {
  return plugin.web({ action: "saveMemory", ownerKey, originalTitle: null, title, content: "正文 " + title, folderPath, tags: "日常", contentType: "text/plain", source: "manual", credibility: 0.7, importance: 0.8 });
}

test("management native lists expose card/group editing, activation, memory, imports and batch binding", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "原生角色", description: "可搜索的说明" } });
  const group = await plugin.api("group.create", { values: { name: "原生群组", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  const ui = mountRegisteredComposeRoute(plugin, null, "main"); await ui.load(); nativeOnly(ui);
  for (const key of ["create-card", "import-card", "bind-chats", "create-group", "import-group", "card:" + card.id + "-open", "card-memory:" + card.id, "card-user:" + card.id, "group:" + group.id + "-open"]) keyedNode(ui.render().tree, key);
  await action(ui, "card-activate:" + card.id); assert.equal((await plugin.web({ action: "snapshot" })).active.CharacterCard.id, card.id);
  await action(ui, "group-activate:" + group.id); assert.equal((await plugin.web({ action: "snapshot" })).active.CharacterGroup.id, group.id);
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "management-query").length, 0);

});

test("native batch binding preserves hidden selections, writes only on submit and leaves global role unchanged", async t => {
  const plugin = await environment(t), card = await plugin.api("character.create", { values: { name: "批量角色" } });
  plugin.records.set("other-chat", { extension: null });
  const ui = mountRegisteredComposeRoute(plugin, null, "main"); await ui.load(); const active = plain((await plugin.web({ action: "snapshot" })).active);
  await action(ui, "card-bind:" + card.id); nativeOnly(ui);
  await action(ui, "binding-check:selector-chat", "onCheckedChange", true);
  await action(ui, "binding-query", "onValueChange", "other-chat"); await action(ui, "binding-check:other-chat", "onCheckedChange", true);
  assert.equal(await plugin.binding(), null); assert.equal(await plugin.binding("other-chat"), null);
  await action(ui, "binding-submit");
  assert.equal((await plugin.binding()).selection, "card:" + card.id); assert.equal((await plugin.binding("other-chat")).selection, "card:" + card.id);
  assert.deepEqual(plain((await plugin.web({ action: "snapshot" })).active), active);
});

test("native memory libraries persist global switches and shared store CRUD", async t => {
  const plugin = await environment(t), ui = mountRegisteredComposeRoute(plugin, null, "memory"); await ui.load(); nativeOnly(ui);
  const initial = (await plugin.web({ action: "snapshot" })).memoryPreferences;
  await action(ui, "memory-preference:userDescriptionEnabled", "onCheckedChange", !initial.userDescriptionEnabled);
  assert.equal((await plugin.web({ action: "readGlobalMemoryPreferences" })).userDescriptionEnabled, !initial.userDescriptionEnabled);
  await action(ui, "create-store"); await action(ui, "store-name", "onValueChange", "共享测试库"); await action(ui, "store-save");
  const store = (await plugin.web({ action: "listStores" })).find(store => store.name === "共享测试库"); assert.ok(store);
  await action(ui, "store-edit:" + store.id); await action(ui, "store-name", "onValueChange", "共享改名"); await action(ui, "store-save");
  assert.equal((await plugin.web({ action: "listStores" })).find(value => value.id === store.id).name, "共享改名");
  await action(ui, "store-delete:" + store.id); assert.ok((await plugin.web({ action: "listStores" })).some(value => value.id === store.id)); await action(ui, "store-delete-confirm"); assert.equal((await plugin.web({ action: "listStores" })).some(value => value.id === store.id), false);
});

test("memory native forms save all fields, preserve drafts on rejection and cancel without writes", async t => {
  const plugin = await environment(t), ui = await memory(t, plugin);
  await action(ui, "memory-create"); nativeOnly(ui);
  for (const [key, value] of [["memory-title", "迁移记忆"], ["memory-content", "第一行\n第二行"], ["memory-content-type", "text/markdown"], ["memory-source", "test"], ["memory-credibility", 0.9], ["memory-importance", 0.6], ["memory-folder", "生活/日常"], ["memory-tags", "标签一,标签二"]]) await action(ui, key, "onValueChange", value);
  await action(ui, "memory-save"); const items = await plugin.web({ action: "listMemories", ownerKey: "character:default" }); const item = items.find(item => item.title === "迁移记忆"); assert.ok(item);
  assert.equal(item.content, "第一行\n第二行"); assert.equal(item.contentType, "text/markdown"); assert.equal(item.credibility, 0.9); assert.equal(item.importance, 0.6); assert.equal(item.folderPath, "生活/日常"); assert.deepEqual(plain(item.tags.map(tag => tag.name)).sort(), ["标签一", "标签二"]);
  await action(ui, "memory-select:" + item.uuid); await action(ui, "memory-edit-selected"); await action(ui, "memory-title", "onValueChange", ""); await action(ui, "memory-save"); keyedNode(ui.render().tree, "memory-workspace-error"); assert.equal(keyedNode(ui.render().tree, "memory-importance").props.value, 0.6);
  await action(ui, "memory-title", "onValueChange", "取消的草稿"); await action(ui, "memory-form-cancel"); assert.equal((await plugin.web({ action: "listMemories", ownerKey: "character:default" }))[0].title, "迁移记忆");
  await action(ui, "memory-delete-selected"); assert.equal((await plugin.web({ action: "listMemories", ownerKey: "character:default" })).length, 1); await action(ui, "memory-delete-confirm"); assert.equal((await plugin.web({ action: "listMemories", ownerKey: "character:default" })).length, 0);
});

test("native USER.md edits use the selected shared owner and preserve private documents", async t => {
  const plugin = await environment(t), snapshot = await plugin.web({ action: "saveStore", create: true, store: { id: "", name: "共享资料" } }), store = snapshot.stores[0];
  await plugin.web({ action: "createMemoryLibrary", ownerKey: "shared:" + store.id });
  const before = await plugin.web({ action: "readUser", ownerKey: "character:default" });
  const ui = mountRegisteredComposeRoute(plugin, null, "memory"); await ui.load(); await action(ui, "library-user:shared:" + store.id); await action(ui, "memory-workspace-root", "onLoad"); nativeOnly(ui); await action(ui, "memory-profile", "onValueChange", "共享用户资料"); await action(ui, "memory-profile-save");
  assert.equal((await plugin.web({ action: "readUser", ownerKey: "shared:" + store.id })).content, "共享用户资料"); assert.deepEqual(plain(await plugin.web({ action: "readUser", ownerKey: "character:default" })), plain(before));
});

test("native relationship editor creates, edits and deletes real links by lossless ids", async t => {
  const plugin = await environment(t), ownerKey = "character:default"; await seed(plugin, ownerKey, "源"); await seed(plugin, ownerKey, "目标");
  const ui = await memory(t, plugin); await action(ui, "memory-graph-link-mode");
  const initial = await plugin.web({ action: "graph", ownerKey }); await action(ui, "memory-select:" + initial.items.find(item => item.title === "源").uuid); await action(ui, "memory-select:" + initial.items.find(item => item.title === "目标").uuid);
  await action(ui, "link-source", "onClick", initial.items.findIndex(item => item.title === "源")); await action(ui, "link-target", "onClick", initial.items.findIndex(item => item.title === "目标")); await action(ui, "link-type", "onValueChange", "related"); await action(ui, "link-description", "onValueChange", "第一版"); await action(ui, "link-save");
  const graph = await plugin.web({ action: "graph", ownerKey }); assert.equal(graph.graph.edges.length, 1); const edge = graph.graph.edges[0], first = graph.items.find(item => item.title === "源");
  await action(ui, "memory-graph-loader", "onLoad"); await ui.dispatch(ui.graphBridge().select, ["edge", edge.id]); await action(ui, "link-edit-selected"); nativeOnly(ui); assert.equal(keyedNode(ui.render().tree, "link-description").props.value, "第一版");
  await action(ui, "link-weight", "onValueChange", "0.4"); await action(ui, "link-description", "onValueChange", "第二版"); await action(ui, "link-save");
  const saved = await plugin.web({ action: "readMemoryLink", ownerKey, linkId: edge.id }); assert.equal(saved.description, "第二版"); assert.equal(saved.weight, 0.4);
  await action(ui, "memory-graph-loader", "onLoad"); await ui.dispatch(ui.graphBridge().select, ["edge", edge.id]); await action(ui, "link-delete-selected"); await action(ui, "link-delete-confirm"); assert.equal((await plugin.web({ action: "graph", ownerKey })).graph.edges.length, 0);
});

test("settings load authoritative values, validate numeric drafts and persist one combined save", async t => {
  const plugin = await environment(t), ownerKey = "character:default", ui = await memory(t, plugin);
  await more(ui, "memory-settings-open"); await action(ui, "memory-settings-root", "onLoad"); nativeOnly(ui);
  await action(ui, "memory-interval", "onValueChange", ""); await action(ui, "memory-settings-save"); assert.match(keyedNode(ui.render().tree, "memory-settings-error").props.text, /检查间隔/);
  await action(ui, "memory-interval", "onValueChange", "7"); await action(ui, "memory-extraction-rules", "onValueChange", "保留用户明确说出的偏好"); await action(ui, "memory-settings-tabs:search");
  await action(ui, "memory-keywordWeight", "onValueChange", "2.5"); await action(ui, "memory-tagWeight", "onValueChange", "0.8"); await action(ui, "memory-settings-save");
  assert.equal((await plugin.web({ action: "readMemorySettings", ownerKey })).autoSaveIntervalMinutes, 7); assert.equal((await plugin.web({ action: "readMemorySearchConfig", ownerKey })).keywordWeight, 2.5);
  await action(ui, "memory-settings-back"); await more(ui, "memory-settings-open"); await action(ui, "memory-settings-root", "onLoad"); assert.equal(keyedNode(ui.render().tree, "memory-interval").props.value, "7"); await action(ui, "memory-settings-back");
});

test("attachment owner/folder selection uses exact persisted records and emits the existing result", async t => {
  const plugin = await environment(t), ownerKey = "character:default"; await seed(plugin, ownerKey, "根目录"); await seed(plugin, ownerKey, "工作记忆", "工作"); await seed(plugin, ownerKey, "子目录记忆", "工作/子目录");
  const ui = mountRegisteredComposeRoute(plugin, { requestId: "attachment-native", input: { mode: "memory-attachment", ownerKey: null, folderPath: null } }, "memory-attachment"); await ui.load(); nativeOnly(ui);
  await action(ui, "attachment-owner:" + ownerKey); await action(ui, "attachment-folder:工作"); const result = await action(ui, "attachment-confirm"); assert.equal(result.actionResult.type, "toolpkg.presentation.complete"); assert.match(result.actionResult.value.content, /工作记忆/); assert.doesNotMatch(result.actionResult.value.content, /子目录记忆|根目录/);
  const locked = mountRegisteredComposeRoute(plugin, { requestId: "attachment-locked", input: { mode: "memory-attachment", ownerKey, folderPath: "工作" } }, "memory-attachment"); await locked.load(); assert.equal(composeNodes(locked.render().tree, node => node.props.key === "attachment-back").length, 0); nativeOnly(locked);
  assert.throws(() => locked.retarget({ requestId: "other", input: { mode: "memory-attachment", ownerKey: null, folderPath: null } }), /owner changed/);
});


test("graph keeps the original modal, toolbar and inspector; forms unmount its platform view", async t => {
  const plugin = await environment(t), ownerKey = "character:default"; await seed(plugin, ownerKey, "图谱源"); await seed(plugin, ownerKey, "图谱目标");
  const ui = await memory(t, plugin); await action(ui, "memory-graph-loader", "onLoad");
  assert.equal(composeNodes(ui.render().tree, node => node.type === "WebView").length, 1);
  const bridge = ui.graphBridge(); assert.deepEqual(Object.keys(bridge).sort(), ["load", "select"]);
  const loaded = await ui.dispatch(bridge.load, []), graph = loaded.actionResult.data; assert.equal(graph.items.length, 2);
  assert.throws(() => ui.dispatch(bridge.select, ["memory", "nonexistent"]), /no longer exists/);
  await ui.dispatch(bridge.select, ["memory", graph.items[0].uuid]); keyedNode(ui.render().tree, "memory-inspector"); assert.equal(composeNodes(ui.render().tree, node => node.type === "WebView").length, 1);
  await action(ui, "memory-edit-selected"); nativeOnly(ui); await action(ui, "memory-form-cancel"); await action(ui, "memory-graph-loader", "onLoad");
  await action(ui, "memory-graph-link-mode"); await ui.dispatch(ui.graphBridge().select, ["memory", graph.items[0].uuid]); await ui.dispatch(ui.graphBridge().select, ["memory", graph.items[1].uuid]); nativeOnly(ui); keyedNode(ui.render().tree, "link-save");
  await action(ui, "memory-form-cancel"); await more(ui, "memory-settings-open"); await action(ui, "memory-settings-root", "onLoad"); nativeOnly(ui); await action(ui, "memory-settings-back");
});

test("failed profile/settings reads cannot enable blank writes and explicit retry reads canonical values", async t => {
  const plugin = await environment(t), ownerKey = "character:default", ui = mountRegisteredComposeRoute(plugin, null, "memory"); await ui.load();
  await plugin.web({ action: "writeUser", ownerKey, content: "应当保留的用户资料" });
  plugin.failNext("read", new Error("Controlled user read failure")); await action(ui, "library-user:" + ownerKey); await action(ui, "memory-workspace-root", "onLoad");
  assert.ok(composeNodes(ui.render().tree, node => node.props.key === "memory-profile-save").length === 0); keyedNode(ui.render().tree, "memory-profile-retry");
  await action(ui, "memory-profile-retry"); assert.equal(keyedNode(ui.render().tree, "memory-profile").props.value, "应当保留的用户资料"); await action(ui, "memory-form-cancel");
  const before = plain(await plugin.web({ action: "readMemorySettings", ownerKey }));
  plugin.failNext("storage.get", new Error("Controlled settings read failure")); await action(ui, "library-settings:" + ownerKey); await action(ui, "memory-settings-root", "onLoad");
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "memory-settings-save").length, 0);
  keyedNode(ui.render().tree, "memory-settings-retry"); await action(ui, "memory-settings-retry");
  assert.equal(keyedNode(ui.render().tree, "memory-interval").props.value, String(before.autoSaveIntervalMinutes)); await action(ui, "memory-settings-back");
});

test("native memory import/export uses selected VFS files and keeps drafts after picker cancellation", async t => {
  let plugin, selected = null, outcome = "cancel", destination = "", saved;
  plugin = await environment(t, {
    async __operitNativeComposeFilePickerCommand() { return selected === null ? { cancelled: true, files: [] } : { cancelled: false, files: [{ path: selected, uri: selected, size: 100 }] }; },
    async __operitNativeComposeFileSaveCommand(payload) { saved = plain(payload.options); if (outcome === "cancel") return { cancelled: true, path: null }; await plugin.disk.files.write(destination, payload.options.content, false); return { cancelled: false, path: destination }; },
  });
  const ownerKey = "character:default"; await seed(plugin, ownerKey, "真实文件导入"); const backup = await plugin.web({ action: "exportMemory", ownerKey });
  await plugin.web({ action: "deleteMemory", ownerKey, title: "真实文件导入" });
  const ui = await memory(t, plugin); await more(ui, "memory-import"); await action(ui, "memory-import-strategy", "onClick", 1); await action(ui, "memory-import-submit"); keyedNode(ui.render().tree, "memory-import-submit");
  selected = plugin.disk.directory + "/selected.json"; await plugin.disk.files.write(selected, "\uFEFF" + backup.content, false); await action(ui, "memory-import-submit");
  assert.ok((await plugin.web({ action: "listMemories", ownerKey })).some(item => item.title === "真实文件导入"));
  await more(ui, "memory-export"); await action(ui, "memory-export-save"); keyedNode(ui.render().tree, "memory-export-save");
  destination = plugin.disk.directory + "/export.json"; outcome = "save"; await action(ui, "memory-export-save"); assert.equal((await plugin.disk.files.read(destination)).content, saved.content); assert.equal(saved.mimeType, "application/json");
});

test("original action catalog retains independent owner entries, dialog forms and native settings tabs", async t => {
  const plugin = await environment(t);
  await plugin.api("character.update", { id: "default", changes: { name: "Operit", description: "默认角色卡" } });
  const stores = await plugin.web({ action: "saveStore", create: true, store: { id: "", name: "共享记忆" } }), store = stores.stores[0];
  await plugin.web({ action: "createMemoryLibrary", ownerKey: "shared:" + store.id });
  const card = await plugin.api("character.create", { values: { name: "旅行助理", description: "一起规划行程，记录沿途的故事。", memoryBindingMode: "SHARED", sharedMemoryId: store.id } });
  await plugin.api("group.create", { values: { name: "日常讨论", members: [{ characterCardId: "default", orderIndex: 0 }, { characterCardId: card.id, orderIndex: 1 }] } });
  const role = mountRegisteredComposeRoute(plugin, null, "main"); await role.load();
  await action(role, "bind-chats");
  keyedNode(role.render().tree, "character-section"); keyedNode(role.render().tree, "management-dialog");
  for (const key of ["binding-card", "binding-query", "binding-filter:all", "binding-filter:unbound", "binding-select-visible", "binding-submit"]) keyedNode(role.render().tree, key);
  await action(role, "management-back");
  await action(role, "import-card"); assert.equal(keyedNode(role.render().tree, "import-format").type, "DropdownMenu");
  await action(role, "management-back");
  const ui = mountRegisteredComposeRoute(plugin, null, "memory"); await ui.load();
  for (const owner of ["character:default", "shared:" + store.id]) for (const entry of ["library-open:", "library-user:", "library-settings:"]) keyedNode(ui.render().tree, entry + owner);
  await action(ui, "create-store"); keyedNode(ui.render().tree, "shared-memory-section");
  await action(ui, "management-back");
  await action(ui, "library-user:shared:" + store.id); await action(ui, "memory-workspace-root", "onLoad");
  nativeOnly(ui); keyedNode(ui.render().tree, "shared-memory-section");
  await action(ui, "memory-form-cancel");
  await action(ui, "library-settings:shared:" + store.id); await action(ui, "memory-settings-root", "onLoad");
  assert.equal(keyedNode(ui.render().tree, "memory-settings-tabs").type, "PrimaryTabRow");
  await action(ui, "memory-settings-tabs:search");
  assert.equal(composeNodes(ui.render().tree, node => node.props.key === "memory-score-mode").length, 0);
  for (const key of ["memory-keywordWeight", "memory-tagWeight", "memory-vectorWeight", "memory-edgeWeight", "memory-embedding", "memory-debug-search"]) keyedNode(ui.render().tree, key);
  await action(ui, "memory-settings-tabs:rebuild");
  for (const key of ["memory-rebuild-window", "memory-rebuild-from", "memory-rebuild-to", "memory-rebuild-start", "memory-rebuild-cancel", "memory-status-refresh"]) keyedNode(ui.render().tree, key);
  await action(ui, "memory-settings-back");
  await ui.dispatch({ __actionId: "__operit_theme_changed" }, fixture().theme);
  await action(ui, "library-open:character:default"); await action(ui, "memory-workspace-root", "onLoad");
  for (const key of ["memory-workspace-back", "memory-more", "memory-search", "memory-search-clear", "memory-folder-filter", "memory-graph-link-mode", "memory-inspector", "memory-create"]) keyedNode(ui.render().tree, key);
  await action(ui, "memory-create");
  assert.equal(keyedNode(ui.render().tree, "memory-credibility").type, "Slider");
  assert.equal(keyedNode(ui.render().tree, "memory-importance").type, "Slider");
  nativeOnly(ui); await action(ui, "memory-form-cancel");
});
