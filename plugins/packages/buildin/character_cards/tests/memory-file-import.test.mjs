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
  const dialog = { type: "import", target: "memory", ownerKey, strategy: "UPDATE", content: "stale pasted text" };
  const graph = { data: null, searchIds: new Set(["stale"]), query: "stale", appliedQuery: "stale", selected: "stale", selectedEdge: "stale" };
  let closed = false;
  const notifications = [];
  const feature = loadModule("web/features/characters/transfers.ts", {
    window: { CharacterMemoryHost: { ...bridge, readImportFile: () => bridge.readImportFile([]) } },
  }).createCharactersTransfersFeature({
    state: {}, topDialog: () => dialog, popDialog: () => { closed = true; },
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
});

test("cancelling memory file selection preserves import options and performs no read or import", async t => {
  const disk = await createDiskHarness(t, { legacyIdentity: "default" });
  const bridge = await importBridge(disk, async () => ({ cancelled: true, files: [] }));
  const dialog = { type: "import", target: "memory", ownerKey: "character:default", strategy: "CREATE_NEW" };
  const unexpected = () => { throw new Error("Cancelled selection must not continue"); };
  const feature = loadModule("web/features/characters/transfers.ts", {
    window: { CharacterMemoryHost: { ...bridge, readImportFile: () => bridge.readImportFile([]) } },
  }).createCharactersTransfersFeature({ state: {}, topDialog: () => dialog, request: unexpected, popDialog: unexpected, owningGraph: unexpected, toast: unexpected });
  disk.clearCalls();
  await feature.handleAction("read-import", null);
  assert.equal(dialog.strategy, "CREATE_NEW");
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
