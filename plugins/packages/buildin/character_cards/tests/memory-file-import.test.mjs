import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { writeFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";

/** Captures the production WebView interface with only the host picker and rendering capabilities supplied. */
async function importBridge(disk, chooseFile) {
  let bridge;
  const controller = { addJavascriptInterface(_name, value) { bridge = value; } };
  const node = (props, children) => ({ props, children });
  const ctx = {
    createWebViewController: () => controller,
    useState: (_key, initial) => [initial, () => {}],
    useRef: (_key, initial) => ({ current: initial }),
    UI: { Box: node, Text: node },
    Theme: { subscribe: () => () => {} },
    openFilePicker: chooseFile,
  };
  const editor = loadModule("src/ui-editor.ts", {
    ...disk.globals,
    ToolPkg: { ...disk.globals.ToolPkg, readResource: async () => "/package/character-memory.html" },
  });
  await editor.renderScreen(ctx, { title: "角色卡" }).props.onLoad();
  assert.ok(bridge, "The production renderer must register its interface");
  return bridge;
}

test("memory file import uses the selected host resource and imports its UTF-8 JSON through the production service", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const service = loadModule("src/service-runtime.ts", disk.globals);
  const ownerKey = "character:default";
  const created = await service.dispatchDomain("memory.create", { ownerKey, values: { title: "文件里的记忆", content: "完整中文内容" } });
  const exported = await service.dispatchDomain("memory.export", { ownerKey });
  await service.dispatchDomain("memory.delete", { ownerKey, id: created.item.id });
  const selectedPath = path.join(disk.directory, "selected-backup.json");
  await writeFile(selectedPath, "\uFEFF" + exported.content);
  const selections = [];
  const bridge = await importBridge(disk, async options => {
    selections.push(plain(options));
    return { cancelled: false, files: [{ path: selectedPath, uri: selectedPath, size: Buffer.byteLength(exported.content) + 3 }] };
  });
  const dialog = { type: "import", target: "memory", ownerKey, strategy: "UPDATE", content: "stale pasted text", busy: false, stage: "" };
  const graph = { data: null, searchIds: new Set(["stale"]), query: "stale", appliedQuery: "stale", selected: "stale", selectedEdge: "stale" };
  let closed = false;
  const notifications = [];
  const stages = [];
  const feature = loadModule("web/features/characters/transfers.ts", {
    window: { CharacterMemoryHost: { ...bridge, readImportFile: () => bridge.readImportFile([]) } },
  }).createCharactersTransfersFeature({
    state: {}, topDialog: () => dialog, popDialog: () => { closed = true; },
    renderDialogs: () => { stages.push({ stage: dialog.stage, busy: dialog.busy }); },
    request: operation => service.dispatch(operation), owningGraph: () => graph,
    toast: message => notifications.push(message),
  });
  await feature.handleAction("read-import", null);
  assert.deepEqual(selections, [{ picker: "document", allowMultiple: false, mimeTypes: ["application/json"] }]);
  const stored = (await service.dispatchDomain("memory.list", { ownerKey })).items;
  assert.equal(stored.length, 1);
  assert.equal(stored[0].content, "完整中文内容");
  assert.equal(graph.data.items[0].uuid, stored[0].uuid);
  assert.equal(graph.searchIds, null); assert.equal(graph.selected, null); assert.equal(graph.query, "");
  assert.equal(closed, true);
  assert.match(notifications[0], /记忆导入完成：新增 1/);
  assert.deepEqual(stages.map(value => value.busy), [true, true, true, false]);
  assert.match(stages[0].stage, /读取/); assert.match(stages[1].stage, /云端向量/); assert.match(stages[2].stage, /刷新/);
  assert.equal(dialog.stage, "");
});

test("cancelling memory file selection preserves import options and performs no read or import", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const bridge = await importBridge(disk, async () => ({ cancelled: true, files: [] }));
  const dialog = { type: "import", target: "memory", ownerKey: "character:default", strategy: "CREATE_NEW" };
  const unexpected = () => { throw new Error("Cancelled selection must not continue"); };
  const feature = loadModule("web/features/characters/transfers.ts", {
    window: { CharacterMemoryHost: { ...bridge, readImportFile: () => bridge.readImportFile([]) } },
  }).createCharactersTransfersFeature({ state: {}, topDialog: () => dialog, renderDialogs: () => {}, request: unexpected, popDialog: unexpected, owningGraph: unexpected, toast: unexpected });
  disk.clearCalls();
  await feature.handleAction("read-import", null);
  assert.equal(dialog.strategy, "CREATE_NEW");
  assert.equal(dialog.busy, false); assert.equal(dialog.stage, "");
  assert.equal(disk.calls.length, 0);
});

test("empty files and failed resource reads keep their errors at the import boundary", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const emptyPath = path.join(disk.directory, "empty.json");
  await writeFile(emptyPath, "\uFEFF  \n");
  let selectedPath = emptyPath;
  const bridge = await importBridge(disk, async () => ({ cancelled: false, files: [{ path: selectedPath }] }));
  await assert.rejects(bridge.readImportFile([]), /所选 JSON 文件为空/);
  selectedPath = path.join(disk.directory, "missing.json");
  await assert.rejects(bridge.readImportFile([]), /ENOENT/);
  await assert.rejects(bridge.readImportFile(["unexpected"]), /expects 0 arguments/);
});

/** Keeps the actual import pending at its HTTP boundary while observing the rendered dialog state. */
test("memory import shows its waiting stage and releases the dialog after a provider failure", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  let entered, rejectProvider;
  const ready = new Promise(resolve => { entered = resolve; });
  const provider = new Promise((_resolve, reject) => { rejectProvider = reject; });
  disk.globals.Tools.Net = { async http() { entered(); return provider; } };
  const service = loadModule("src/service-runtime.ts", disk.globals), ownerKey = "character:default";
  const created = await service.dispatchDomain("memory.create", { ownerKey, values: { title: "等待向量", content: "完整备份" } });
  const backup = JSON.parse((await service.dispatchDomain("memory.export", { ownerKey })).content);
  await service.dispatchDomain("memory.delete", { ownerKey, id: created.item.id });
  Object.assign(backup.space.settings, { cloudEmbeddingEnabled: true, cloudEmbeddingEndpoint: "https://provider.test", cloudEmbeddingModel: "model", cloudEmbeddingApiKey: "" });
  const selectedPath = path.join(disk.directory, "cloud-backup.json");
  await writeFile(selectedPath, JSON.stringify(backup));
  const bridge = await importBridge(disk, async () => ({ cancelled: false, files: [{ path: selectedPath }] }));
  const dialog = { type: "import", target: "memory", ownerKey, strategy: "UPDATE", content: "", busy: false, stage: "" };
  const rendered = [];
  const view = loadModule("web/features/characters/import-export-view.ts");
  const feature = loadModule("web/features/characters/transfers.ts", {
    window: { CharacterMemoryHost: { ...bridge, readImportFile: () => bridge.readImportFile([]) } },
  }).createCharactersTransfersFeature({
    state: {}, topDialog: () => dialog, request: operation => service.dispatch(operation),
    renderDialogs: () => rendered.push(view.importSpec(dialog).body),
    popDialog: () => { throw new Error("A failed import must remain open"); },
  });
  const failure = new Error("Embedding provider is unavailable");
  const pending = feature.handleAction("read-import", null);
  const rejected = assert.rejects(pending, error => error === failure);
  await ready;
  assert.equal(dialog.busy, true); assert.match(dialog.stage, /云端向量/);
  assert.match(rendered.at(-1), /role="status"/);
  assert.equal((await service.dispatchDomain("memory.list", { ownerKey })).items.length, 0);
  rejectProvider(failure); await rejected;
  assert.equal(dialog.busy, false); assert.equal(dialog.stage, "");
  assert.equal((await service.dispatchDomain("memory.list", { ownerKey })).items.length, 0);
});
