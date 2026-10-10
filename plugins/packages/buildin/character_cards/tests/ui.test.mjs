import { packageRuntime } from "./package-runtime.mjs";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import vm from "node:vm";
import http from "node:http";
import assert from "node:assert/strict";
import test from "node:test";
import { fixture, installBrowserFixture } from "./fixtures.mjs";
import { buildMainScript, buildUiScreenScripts, createHtmlDocument } from "../scripts/build.mjs";
import { createDiskHarness } from "./disk-files.mjs";
import { importedCallCount } from "./runtime.mjs";
import { keyedNode, mountRegisteredSelector } from "./selector-render.mjs";
import { createNativeChatHost } from "./native-chat-host.mjs";
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { chromium } = require("playwright");
const html = await createHtmlDocument();
const mainScript = await buildMainScript();
const installedHtml = await readFile(new URL("../resources/character-memory.html", import.meta.url), "utf8");
const output = fileURLToPath(new URL("../../../../../.tmp/character-cards/", import.meta.url));
let browser, server, origin;

/** Starts an isolated page server and headless browser for UI checks only. */
test.before(async () => {
  server = http.createServer(
    /** Serves explicitly selected current-source or installed HTML without substituting a missing artifact. */
    (request, response) => {
      if (request.url !== "/" && request.url !== "/installed" && request.url !== "/memory") { response.writeHead(404); response.end("Unknown test document"); return; }
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      response.end(request.url === "/installed" ? installedHtml : html);
    },
  );
  await new Promise(
    /** Resolves the server's real ephemeral listening port. */
    resolve => server.listen(0, "127.0.0.1", resolve),
  );
  origin = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ headless: true });
  await mkdir(output, { recursive: true });
});

/** Closes all browser and HTTP resources after verification. */
test.after(async () => { if (browser) await browser.close(); if (server) await new Promise(
  /** Resolves after the isolated test server releases its listening socket. */
  resolve => server.close(resolve),
); });

/** Installs an explicit management presentation and rejects results without a presented caller. */
function installManagementPresentation() {
  const bridge = window.CharacterMemoryHost;
  if (bridge === undefined) throw new Error("Management test requires its explicitly installed browser fixture");
  /** Supplies the exact legal management input rather than inferring a missing screen. */
  bridge.currentScreen = async () => ({ requestId: null, input: window.location.pathname === "/memory" ? { mode: "manage", view: "memory" } : { mode: "manage" } });
  /** Rejects completion because a management page has no presentation request. */
  bridge.completeScreen = async () => { throw new Error("Management has no presentation completion channel"); };
  /** Rejects cancellation because a management page has no presentation request. */
  bridge.cancelScreen = async () => { throw new Error("Management has no presentation cancellation channel"); };
}

/** Creates a page with its complete explicitly declared management host before document execution. */
async function page(width = 1100, height = 950, data = fixture()) {
  const context = await browser.newContext({ viewport: { width, height } });
  await context.addInitScript({ content: `(${installBrowserFixture.toString()})(${JSON.stringify(data)}); (${installManagementPresentation.toString()})();` });
  const page = await context.newPage();
  await page.goto(origin);
  await page.locator('#app h1, #app h2, #app [role="alert"]').first().waitFor();
  const startupErrors = await page.locator('#app [role="alert"]').allTextContents();
  assert.ok(await page.getByRole("heading", { name: "角色卡", exact: true }).count() > 0, `Current UI did not initialize: ${startupErrors.join("\n")}`);
  return { page, context };
}

/** Opens the existing character rather than a newly fabricated draft. */
async function openCharacter(page) { await page.getByRole("button", { name: "编辑 旅行助理", exact: true }).click(); }

/** Verifies original Material structure and draft preservation across all role tabs. */
test("character editor reproduces exactly basic/content/binding; memory is an independent page", async () => {
  const ui = await page();
  const errors = [];
  ui.page.on("pageerror", error => errors.push(error.message));
  await openCharacter(ui.page);
  const dialog = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  assert.equal(await dialog.getByRole("tab").count(), 3);
  assert.equal(await dialog.getByLabel("角色名称 *").inputValue(), "旅行助理");
  await dialog.getByLabel("角色名称 *").fill("我的旅行助手");
  await dialog.getByRole("tab", { name: "内容", exact: true }).click();
  await dialog.getByLabel("聊天附加内容").fill("尽量给出详细路线。");
  await dialog.getByRole("tab", { name: "绑定", exact: true }).click();
  assert.equal(await dialog.getByRole("switch", { name: "TTS 配置", exact: true }).isEnabled(), true);
  await dialog.getByRole("tab", { name: "基础", exact: true }).click();
  assert.equal(await dialog.getByLabel("角色名称 *").inputValue(), "我的旅行助手");
  await ui.page.screenshot({ path: output + "character-editor-light.png" });
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await ui.page.getByRole("button", { name: "编辑 我的旅行助手", exact: true }).waitFor();
  const calls = await ui.page.evaluate(() => window.testCalls);
  const saved = calls.find(call => call.action === "saveCharacter");
  assert.equal(saved.card.otherContentChat, "尽量给出详细路线。");
  assert.deepEqual(errors, []);
  await ui.context.close();
});

/** Opens the separate memory settings page and the exact bound role-library row. */
async function openCharacterMemory(page, name = "旅行助理") {
  if (await page.getByRole("heading", { name: "记忆", exact: true }).count() === 0) await page.goto(origin + "/memory");
  await page.getByRole("heading", { name: "角色记忆库", exact: true }).waitFor();
  assert.equal(await page.getByRole("dialog", { name: "编辑角色卡", exact: true }).count(), 0);
  return page.locator("[data-memory-card]").filter({ has: page.locator(".entity-title", { hasText: name }) });
}

/** Opens the real SVG graph using the selected character's stored memory binding. */
async function openCharacterGraph(page) {
  const editor = await openCharacterMemory(page);
  await editor.getByRole("button", { name: "记忆图谱", exact: true }).click();
  const graph = page.getByRole("dialog", { name: "旅行助理 的记忆图谱", exact: true });
  await graph.locator(".graph-node").first().waitFor();
  return graph;
}

/** Verifies shared-bound memory operations retain the card's actual owner identity. */
test("independent memory page USER.md and graph use the selected character shared binding", async () => {
  const ui = await page();
  const editor = await openCharacterMemory(ui.page);
  await ui.page.screenshot({ path: output + "character-memory-tab.png" });
  await editor.getByRole("button", { name: "用户资料 · USER.md", exact: true }).click();
  const profile = ui.page.getByRole("dialog", { name: "旅行助理 的用户资料", exact: true });
  await profile.getByLabel("用户资料", { exact: true }).fill("更新后的真实用户资料");
  await profile.getByRole("button", { name: "保存", exact: true }).click();
  await editor.getByRole("button", { name: "记忆图谱", exact: true }).click();
  const graph = ui.page.getByRole("dialog", { name: "旅行助理 的记忆图谱", exact: true });
  await graph.getByRole("button", { name: "创建记忆", exact: true }).waitFor();
  await ui.page.screenshot({ path: output + "memory-graph-light.png" });
  await graph.getByRole("button", { name: "创建记忆", exact: true }).click();
  const edit = ui.page.getByRole("dialog", { name: "创建记忆", exact: true });
  await edit.getByLabel("标题 *").fill("新记忆");
  await edit.getByLabel("内容", { exact: true }).fill("来自当前角色的记忆内容");
  await edit.getByRole("button", { name: "保存", exact: true }).click();
  await graph.locator(".graph-sidebar").getByRole("button", { name: /^新记忆/ }).waitFor();
  const calls = await ui.page.evaluate(() => window.testCalls);
  for (const call of calls) if (["readUser", "writeUser", "graph", "saveMemory"].indexOf(call.action) >= 0) assert.equal(call.ownerKey, "shared:shared-main");
  await ui.context.close();
});

/** Ensures profile read failures cannot turn into an enabled blank overwrite form. */
test("failed USER.md read stays visible and disables save", async () => {
  const ui = await page();
  const editor = await openCharacterMemory(ui.page);
  await ui.page.evaluate(() => { window.testFailure = "readUser"; });
  await editor.getByRole("button", { name: "用户资料 · USER.md", exact: true }).click();
  const profile = ui.page.getByRole("dialog", { name: "旅行助理 的用户资料", exact: true });
  await profile.getByRole("alert").waitFor();
  assert.match(await profile.getByRole("alert").innerText(), /TEST_HOST_REJECTED/);
  assert.equal(await profile.getByRole("button", { name: "保存", exact: true }).isEnabled(), false);
  await ui.context.close();
});

/** Checks live theme updates and the full mobile editor without horizontal overflow. */
test("mobile layout and dark host theme preserve draft and fit the viewport", async () => {
  const ui = await page(390, 844);
  await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByLabel("角色名称 *").fill("保留编辑中的名称");
  await ui.page.evaluate(theme => {
    theme.brightness = "dark";
    Object.assign(theme.colors, { primary: "#d0bcffff", onPrimary: "#381e72ff", primaryContainer: "#4f378bff", onPrimaryContainer: "#eaddffff", onSurface: "#e6e0e9ff", onSurfaceVariant: "#cac4d0ff", surface: "#141218ff", surfaceContainer: "#211f26ff", surfaceContainerHigh: "#2b2930ff", surfaceContainerHighest: "#36343bff", outline: "#938f99ff", outlineVariant: "#49454fff", error: "#f2b8b5ff" });
    window.applyCharacterMemoryTheme(theme);
  }, fixture().theme);
  assert.equal(await editor.getByLabel("角色名称 *").inputValue(), "保留编辑中的名称");
  const bounds = await editor.boundingBox();
  assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= 390);
  assert.ok(bounds.y >= 0 && bounds.y + bounds.height <= 844);
  await ui.page.screenshot({ path: output + "character-editor-mobile-dark.png" });
  await editor.getByRole("button", { name: "取消", exact: true }).click();
  await openCharacterMemory(ui.page);
  await ui.page.screenshot({ path: output + "character-memory-mobile-dark.png" });
  await ui.context.close();
});

/** Verifies tag CRUD remains draft-only and cancels with its enclosing native character editor. */
test("tag management stages edits and deletion without writing until character save", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("button", { name: "管理标签", exact: true }).click();
  const manager = ui.page.getByRole("dialog", { name: "管理标签", exact: true });
  await manager.getByRole("button", { name: "创建标签", exact: true }).click();
  const tag = ui.page.getByRole("dialog", { name: "创建标签", exact: true });
  await tag.getByLabel("标签名称 *").fill("新标签");
  await tag.getByLabel("提示词内容").fill("<script>不执行</script>");
  await tag.getByRole("button", { name: "保存", exact: true }).click();
  await manager.getByRole("button", { name: "编辑 新标签", exact: true }).waitFor();
  assert.equal(await ui.page.locator("script").count(), 1);
  await manager.getByRole("button", { name: "删除 旅行", exact: true }).click();
  const confirmation = ui.page.getByRole("dialog", { name: "删除标签", exact: true });
  await confirmation.getByRole("button", { name: "删除", exact: true }).click();
  await manager.getByRole("button", { name: "完成", exact: true }).click();
  assert.equal(await editor.getByRole("button", { name: "新标签", exact: true }).getAttribute("class"), "chip selected");
  assert.equal(await editor.getByRole("button", { name: "旅行", exact: true }).count(), 0);
  const before = await ui.page.evaluate(
    /** Reads only the test host's explicit request history. */
    () => window.testCalls,
  );
  assert.deepEqual(before.map(
    /** Records the finite actions performed before character save. */
    call => call.action,
  ), ["snapshot", "listThemeChoices"]);
  await editor.getByRole("button", { name: "取消", exact: true }).click();
  await openCharacter(ui.page);
  assert.equal(await editor.getByRole("button", { name: "新标签", exact: true }).count(), 0);
  assert.equal(await editor.getByRole("button", { name: "旅行", exact: true }).count(), 1);
  await ui.context.close();
});

/** Verifies staged tag changes and real selected ids travel with character save. */
test("character save sends staged tags in the native change-set shape", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("button", { name: "管理标签", exact: true }).click();
  const manager = ui.page.getByRole("dialog", { name: "管理标签", exact: true });
  await manager.getByRole("button", { name: "编辑 友好", exact: true }).click();
  const tag = ui.page.getByRole("dialog", { name: "编辑标签", exact: true });
  await tag.getByLabel("标签名称 *").fill("修改后的友好");
  await tag.getByRole("button", { name: "保存", exact: true }).click();
  await manager.getByRole("button", { name: "完成", exact: true }).click();
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  const calls = await ui.page.evaluate(
    /** Reads the test bridge's exact payload after the parent was saved. */
    () => window.testCalls,
  );
  const saved = calls.find(
    /** Selects the sole character commit. */
    call => call.action === "saveCharacter",
  );
  assert.equal(saved.tagChanges.updated[0].id, "tag-persona");
  assert.equal(saved.tagChanges.updated[0].tagType, "TONE");
  assert.equal(saved.tagChanges.updated[0].name, "修改后的友好");
  await ui.context.close();
});

/** Verifies export is native Operit JSON of the saved record, not a plugin envelope or draft. */
test("native JSON export preserves original saved record fields", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByLabel("角色名称 *").fill("尚未保存");
  await editor.getByRole("button", { name: "导出", exact: true }).click();
  const exported = ui.page.getByRole("dialog", { name: "导出 JSON", exact: true });
  const record = JSON.parse(await exported.getByLabel("JSON 内容").inputValue());
  assert.deepEqual(record, fixture().snapshot.cards[1]);
  assert.equal(Object.hasOwn(record, "format"), false);
  await ui.context.close();
});

/** Verifies memory edits preserve every field provided by the original native editor. */
test("isolated memory editor forwards type source credibility and importance to its bound-owner request", async () => {
  const ui = await page(); const editor = await openCharacterMemory(ui.page);
  await editor.getByRole("button", { name: "记忆图谱", exact: true }).click();
  const graph = ui.page.getByRole("dialog", { name: "旅行助理 的记忆图谱", exact: true });
  await graph.getByRole("button", { name: "喜欢安静的旅行", exact: true }).click();
  await graph.getByRole("button", { name: "编辑", exact: true }).click();
  const memory = ui.page.getByRole("dialog", { name: "编辑记忆", exact: true });
  assert.equal(await memory.getByLabel("来源", { exact: true }).inputValue(), "manual");
  await memory.getByLabel("内容类型", { exact: true }).fill("text/markdown");
  await memory.getByLabel("来源", { exact: true }).fill("用户笔记");
  await memory.getByLabel("可信度", { exact: true }).focus();
  for (let index = 0; index < 15; index++) await memory.getByLabel("可信度", { exact: true }).press("ArrowLeft");
  await memory.getByLabel("重要性", { exact: true }).focus();
  for (let index = 0; index < 15; index++) await memory.getByLabel("重要性", { exact: true }).press("ArrowLeft");
  await memory.getByRole("button", { name: "保存", exact: true }).click();
  const calls = await ui.page.evaluate(
    /** Captures the test host's native memory-update request. */
    () => window.testCalls,
  );
  const saved = calls.find(
    /** Finds the selected memory's sole update operation. */
    call => call.action === "saveMemory",
  );
  assert.equal(saved.ownerKey, "shared:shared-main"); assert.equal(saved.contentType, "text/markdown");
  assert.equal(saved.source, "用户笔记"); assert.equal(saved.credibility, 0.75); assert.equal(saved.importance, 0.65);
  await ui.context.close();
});

/** Verifies native backend search results expand to one-hop neighbors and folder subtrees. */
test("isolated graph search forwards the plugin query and expands only direct neighbors", async () => {
  const ui = await page(); const editor = await openCharacterMemory(ui.page);
  await editor.getByRole("button", { name: "记忆图谱", exact: true }).click();
  const graph = ui.page.getByRole("dialog", { name: "旅行助理 的记忆图谱", exact: true });
  await graph.getByLabel("搜索记忆", { exact: true }).fill("不需要子串命中");
  await graph.getByRole("button", { name: "搜索", exact: true }).click();
  assert.equal(await graph.locator(".graph-node").count(), 2);
  assert.equal(await graph.locator(".graph-sidebar .memory-item").count(), 1);
  const calls = await ui.page.evaluate(
    /** Reads the finite repository search submitted by the HTML graph. */
    () => window.testCalls,
  );
  const search = calls.find(
    /** Selects the native memory search rather than any text-inspection shortcut. */
    call => call.action === "searchMemory",
  );
  assert.equal(search.query, "不需要子串命中"); assert.equal(search.ownerKey, "shared:shared-main");
  await graph.getByRole("button", { name: "清除搜索", exact: true }).click();
  assert.equal(await graph.locator(".graph-node").count(), 3);
  await ui.context.close();
});

/** Verifies a real node press retains its SVG target and selects exactly on release. */
test("SVG node press does not select or replace its target before the actual click", async () => {
  const ui = await page(), graph = await openCharacterGraph(ui.page);
  const node = graph.locator('.graph-node[data-id="memory-one"]');
  const bounds = await node.boundingBox();
  assert.notEqual(bounds, null);
  await node.evaluate(
    /** Retains the exact pressed SVG element to detect premature redraws. */
    element => { window.testPressedNode = element; },
  );
  await ui.page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await ui.page.mouse.down();
  assert.equal(await graph.locator(".graph-node.selected").count(), 0);
  assert.equal(await ui.page.evaluate(
    /** Confirms pointer-down did not detach the future click target. */
    () => window.testPressedNode.isConnected,
  ), true);
  await ui.page.mouse.move(bounds.x + bounds.width / 2 + 2, bounds.y + bounds.height / 2 + 1);
  assert.equal(await graph.locator(".graph-node.selected").count(), 0);
  await ui.page.mouse.up();
  assert.equal(await graph.locator('.graph-node.selected[data-id="memory-one"]').count(), 1);
  assert.equal(await graph.locator(".graph-detail h3").innerText(), "喜欢安静的旅行");
  await ui.context.close();
});

/** Verifies cumulative small pointer moves pan without consuming the next native node click. */
test("slow graph pan preserves node positions and the next real SVG click", async () => {
  const ui = await page(), graph = await openCharacterGraph(ui.page);
  const surface = graph.locator("[data-graph-surface]"), bounds = await surface.boundingBox();
  assert.notEqual(bounds, null);
  const origin = { x: bounds.x + 20, y: bounds.y + 20 };
  assert.equal(await ui.page.evaluate(
    /** Requires the gesture to start on genuine graph background, not a guessed hit target. */
    point => document.elementFromPoint(point.x, point.y).matches("[data-graph-surface]"), origin,
  ), true);
  const transform = await surface.locator(":scope > g").getAttribute("transform");
  const nodeTransform = await graph.locator('.graph-node[data-id="memory-one"]').getAttribute("transform");
  await ui.page.mouse.move(origin.x, origin.y); await ui.page.mouse.down();
  for (let step = 1; step <= 12; step++) await ui.page.mouse.move(origin.x + step * 3, origin.y + step * 2);
  await ui.page.mouse.up();
  assert.notEqual(await surface.locator(":scope > g").getAttribute("transform"), transform);
  assert.equal(await graph.locator('.graph-node[data-id="memory-one"]').getAttribute("transform"), nodeTransform);
  assert.equal(await graph.locator(".graph-node.selected").count(), 0);
  await graph.locator('.graph-node[data-id="memory-one"]').click();
  assert.equal(await graph.locator(".graph-detail h3").innerText(), "喜欢安静的旅行");
  await graph.locator('.graph-node[data-id="memory-two"]').focus(); await ui.page.keyboard.press("Enter");
  assert.equal(await graph.locator(".graph-detail h3").innerText(), "饮食偏好");
  await ui.context.close();
});

/** Verifies node dragging updates only that node and never invokes click selection or link mode. */
test("SVG node drag does not select a link source and the next click still does", async () => {
  const ui = await page(), graph = await openCharacterGraph(ui.page);
  await graph.getByRole("button", { name: "链接", exact: true }).click();
  const node = graph.locator('.graph-node[data-id="memory-one"]');
  await node.waitFor({ state: "visible" }); await node.scrollIntoViewIfNeeded();
  const bounds = await node.boundingBox();
  assert.notEqual(bounds, null);
  const transform = await node.getAttribute("transform");
  const camera = await graph.locator("[data-graph-surface] > g").getAttribute("transform");
  await ui.page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await ui.page.mouse.down();
  await ui.page.mouse.move(bounds.x + bounds.width / 2 + 48, bounds.y + bounds.height / 2 + 30, { steps: 16 });
  await ui.page.mouse.up();
  assert.notEqual(await node.getAttribute("transform"), transform);
  assert.equal(await graph.locator("[data-graph-surface] > g").getAttribute("transform"), camera);
  assert.equal(await graph.locator(".graph-node.selected,.graph-node.link-source").count(), 0);
  assert.equal(await ui.page.getByRole("dialog", { name: "创建记忆链接", exact: true }).count(), 0);
  await node.click();
  assert.equal(await graph.locator('.graph-node.link-source[data-id="memory-one"]').count(), 1);
  await graph.locator('.graph-node[data-id="memory-two"]').click();
  const link = ui.page.getByRole("dialog", { name: "创建记忆链接", exact: true });
  assert.equal(await link.getByLabel("源记忆", { exact: true }).inputValue(), "喜欢安静的旅行");
  await ui.context.close();
});

/** Verifies an SVG edge remains a native mouse click target after the pointer gesture repair. */
test("SVG relationship click opens the actual edge inspector", async () => {
  const ui = await page(), graph = await openCharacterGraph(ui.page);
  const point = await graph.locator(".graph-edge-hit").first().evaluate(
    /** Projects the visible relationship midpoint through the SVG camera transform. */
    line => {
      const matrix = line.getScreenCTM();
      const point = new DOMPoint((line.x1.baseVal.value + line.x2.baseVal.value) / 2, (line.y1.baseVal.value + line.y2.baseVal.value) / 2).matrixTransform(matrix);
      return { x: point.x, y: point.y };
    },
  );
  await ui.page.mouse.move(point.x, point.y); await ui.page.mouse.down();
  assert.equal(await graph.locator(".graph-edge.selected").count(), 0);
  await ui.page.mouse.up();
  assert.equal(await graph.locator(".graph-edge.selected").count(), 1);
  await graph.getByRole("button", { name: "编辑关系", exact: true }).click();
  const editor = ui.page.getByRole("dialog", { name: "编辑记忆关系", exact: true });
  await ui.page.waitForFunction(
    /** Waits for the real terminal dialog or explicit graph error, not a guessed successful edit. */
    () => document.querySelector('dialog[aria-label="编辑记忆关系"]') !== null || document.querySelector('dialog[aria-label="旅行助理 的记忆图谱"] [role="alert"]:not([hidden])') !== null,
  );
  const failures = await graph.getByRole("alert").allTextContents();
  assert.equal(await editor.count(), 1, `Complete relationship editing is not available: ${failures.join("\n")}`);
  assert.equal(await editor.getByLabel("关系类型", { exact: true }).inputValue(), "影响");
  assert.equal(await editor.getByLabel("描述", { exact: true }).inputValue(), fixture().links[0].description);
  await ui.context.close();
});

/** Verifies link mode uses two actual SVG clicks and the original default strength. */
test("native link mode selects source then target and sends the bound owner", async () => {
  const ui = await page(), graph = await openCharacterGraph(ui.page);
  await graph.getByRole("button", { name: "链接", exact: true }).click();
  await graph.locator('.graph-node[data-id="memory-one"]').click();
  await graph.locator('.graph-node[data-id="memory-two"]').click();
  const link = ui.page.getByRole("dialog", { name: "创建记忆链接", exact: true });
  assert.equal(await link.getByLabel("源记忆", { exact: true }).inputValue(), "喜欢安静的旅行");
  assert.equal(await link.getByLabel("目标记忆", { exact: true }).inputValue(), "饮食偏好");
  assert.equal(await link.getByLabel("强度（0–1）", { exact: true }).inputValue(), "1");
  await link.getByRole("button", { name: "创建", exact: true }).click();
  const calls = await ui.page.evaluate(
    /** Reads only the bridge request emitted by the native-style link gesture. */
    () => window.testCalls,
  );
  const creations = calls.filter(
    /** Requires exactly one write for the two actual node clicks and the dialog submit. */
    call => call.action === "createLink",
  );
  assert.equal(creations.length, 1);
  const created = creations[0];
  assert.equal(created.ownerKey, "shared:shared-main"); assert.equal(created.weight, 1);
  assert.equal(await graph.locator(".graph-edge-action").count(), 2);
  await ui.context.close();
});

/** Verifies the current import request and a finite isolated response; it does not execute production import logic. */
test("isolated UI import delegates untouched input and renders the declared snapshot response", async () => {
  const ui = await page();
  await ui.page.getByRole("button", { name: "导入", exact: true }).click();
  const dialog = ui.page.getByRole("dialog", { name: "导入角色 JSON", exact: true });
  const record = fixture().snapshot.cards[0];
  await dialog.getByLabel("粘贴 Operit JSON", { exact: true }).fill(JSON.stringify(record));
  await dialog.getByRole("button", { name: "导入", exact: true }).click();
  const calls = await ui.page.evaluate(
    /** Reads the exact native import payload submitted by the document. */
    () => window.testCalls,
  );
  const imports = calls.filter(
    /** Identifies the actual declared import provider instead of an obsolete client-side create. */
    call => call.action === "importCharacter",
  );
  assert.equal(imports.length, 1);
  assert.deepEqual(imports[0], { action: "importCharacter", format: "operit", content: JSON.stringify(record) });
  assert.equal(calls.filter(
    /** Rejects a separate browser-owned mutation path for imported records. */
    call => call.action === "saveCharacter",
  ).length, 0);
  await dialog.locator(".busy-bar").waitFor({ state: "hidden" });
  const failures = await dialog.getByRole("alert").allTextContents();
  assert.equal(await dialog.count(), 0, `Successful import response was not observed: ${failures.join("\n")}`);
  await ui.page.getByRole("button", { name: "编辑 隔离 UI 导入返回角色", exact: true }).waitFor();
  await ui.context.close();
});

/** Verifies graph read failures never expose a guessed empty library or write controls. */
test("failed graph read reports the host failure and prevents memory writes", async () => {
  const ui = await page();
  const editor = await openCharacterMemory(ui.page);
  await ui.page.evaluate(() => { window.testFailure = "graph"; });
  await editor.getByRole("button", { name: "记忆图谱", exact: true }).click();
  const graph = ui.page.getByRole("dialog", { name: "旅行助理 的记忆图谱", exact: true });
  assert.match(await graph.getByRole("alert").innerText(), /TEST_HOST_REJECTED/);
  assert.equal(await graph.getByRole("button", { name: "创建记忆", exact: true }).count(), 0);
  await ui.context.close();
});

/** Copies explicit protocol data across isolated VM and browser realms without changing fields. */
function plain(value) { return JSON.parse(JSON.stringify(value)); }

/** Runs the current main bundle with native file IO and explicit readonly directory fixtures, never fixture storage. */
async function fileBackedPlugin(t, bundle, uiScripts = undefined) {
  const disk = await createDiskHarness(t), readonly = fixture(), native = createNativeChatHost(new Map([["chat-dom", { extension: null }]])), hostCalls = native.calls, records = native.records;
  const apis = new Map(), channels = new Map(), commands = new Map(), routes = [], navigation = [], appHooks = [], messageHooks = [], hostHooks = [], chatLifecycleHooks = [], toolLifecycleHooks = [], toolPromptHooks = [];
  const tools = { Files: disk.files, Storage: disk.storage, SoftwareSettings: {
    /** Supplies only the explicitly labelled readonly model directory required by the actual service. */
    async listModelSummaries() { return plain(readonly.snapshot.models); },
    /** Declares this isolated host's empty saved-theme catalog without fabricating a role binding or successful apply. */
    async listThemeConfigs() { return []; },
    /** Supplies only the explicitly labelled readonly TTS directory, not business records or persistence. */
    async listTtsConfigs() { return plain(readonly.snapshot.ttsConfigs); },
    /** Supplies the complete labelled readonly tool catalog without faking a store or CRUD response. */
    async readToolSourceCatalog() { return plain(readonly.snapshot.toolCatalog); },
  }, Chat: {
    ...native.chat,
    /** Supplies an explicit existing host chat for the service's exact identity validation, not a plugin binding. */
    async findChat(params) {
      native.observe("findChat", params); assert.deepEqual(plain(params), { query: "chat-dom", match: "exact", index: 0 });
      return { matchedCount: 1, chat: { id: "chat-dom", title: "Explicit host chat", messageCount: 0, createdAt: "2026-10-08T12:00:00Z", updatedAt: "2026-10-08T12:00:00Z", isCurrent: true, inputTokens: 0, outputTokens: 0 },
        /** Formats only the explicitly declared readonly host directory entry. */
        toString() { return "Explicit host chat"; },
      };
    },
  } };
  const registry = { ...disk.globals.ToolPkg,
    /** Captures an actual route callback from the current production main bundle. */
    registerUiRoute(value) { routes.push(value); },
    /** Captures formal navigation entries and preserves their opaque parameters exactly. */
    registerNavigationEntry(value) { navigation.push(value); },
    /** Requires each real public provider to be registered once. */
    registerApi(value) { assert.equal(apis.has(value.name), false, "Duplicate public API: " + value.name); apis.set(value.name, value.function); },
    /** Captures each real command declaration without invoking any domain command. */
    registerCoreCommand(value) { assert.equal(commands.has(value.name), false); commands.set(value.name, value); },
    /** Captures the actual lazy lifecycle callback without starting memory jobs in a UI test. */
    registerAppLifecycleHook(value) { appHooks.push(value); },
    /** Captures the actual message hook registration without running extraction or constructing a queue. */
    registerChatMessageHook(value) { messageHooks.push(value); },
    /** Captures the actual interval hook declaration without starting a local timer or simulated task. */
    registerHostEventHook(value) { hostHooks.push(value); },
    /** Captures the genuine pre-create hook without creating a chat or providing a replacement initializer. */
    registerChatLifecycleHook(value) { chatLifecycleHooks.push(value); },
    /** Requires the actual group input callback to remain exported and uniquely registered. */
    registerChatInputHook(value) { assert.equal(value.id, "group-input-submit"); assert.equal(value.function.name, "onGroupInputSubmit"); },
    /** Captures the real tool-execution policy without calling it during registration. */
    registerToolLifecycleHook(value) { toolLifecycleHooks.push(value); },
    /** Captures the real model-visible-tool policy without synthesizing descriptors or responses. */
    registerToolPromptComposeHook(value) { toolPromptHooks.push(value); },
    /** Resolves only a real manifest-declared resource, including the native selector's packaged default avatar. */
    async readResource(key, name) {
      const manifest = JSON.parse(await readFile(new URL("../manifest.json", import.meta.url), "utf8"));
      const matches = manifest.resources.filter(
        /** Looks up the exact declared identity without substituting a missing resource. */
        resource => resource.key === key,
      );
      assert.equal(matches.length, 1, "Undeclared plugin resource: " + key);
      assert.equal(path.basename(matches[0].path), name);
      const file = fileURLToPath(new URL("../" + matches[0].path, import.meta.url));
      assert.ok((await readFile(file)).length > 0, "Declared resource must exist on disk: " + matches[0].path);
      const destination = disk.directory + "/" + name;
      await writeFile(destination, await readFile(file));
      return destination;
    },
    ipc: {
      /** Retains actual service handlers and rejects duplicate main-runtime channels. */
      on(name, callback) { assert.equal(channels.has(name), false); channels.set(name, callback); },
      /** Calls the real registered main-runtime service handler rather than returning a mocked record. */
      async call(name, input, options) { assert.equal(options.targetRuntime, "main"); assert.equal(channels.has(name), true, name); return channels.get(name)(input); },
    },
  };
  const runtime = packageRuntime({ "dist/main.js": bundle, ...(uiScripts ?? await buildUiScreenScripts()) }, { Tools: tools, ToolPkg: registry });
  const module = { exports: runtime.load("dist/main.js") };
  assert.equal(module.exports.registerToolPkg(), true);
  assert.equal(disk.calls.length, 0, "Registration must not read directories or initialize business storage");
  assert.equal(messageHooks.length, 1); assert.equal(messageHooks[0].id, "memory-candidate-enqueue");
  assert.equal(messageHooks[0].function, module.exports.onMemoryMessagePersisted);
  assert.equal(hostHooks.length, 1); assert.equal(hostHooks[0].id, "memory-jobs-interval");
  assert.equal(hostHooks[0].source, "interval"); assert.deepEqual(plain(hostHooks[0].trigger), { kind: "interval", intervalMs: 60000 });
  assert.equal(hostHooks[0].function, module.exports.onMemoryInterval);
  assert.equal(chatLifecycleHooks.length, 1); assert.equal(chatLifecycleHooks[0].id, "chat-initialization");
  assert.equal(chatLifecycleHooks[0].function, module.exports.beforeChatCreate);
  assert.equal(toolLifecycleHooks.length, 1); assert.equal(toolLifecycleHooks[0].id, "participant-tool-execution");
  assert.equal(toolLifecycleHooks[0].function, module.exports.toolCallPolicy);
  assert.equal(toolPromptHooks.length, 1); assert.equal(toolPromptHooks[0].id, "participant-tool-visibility");
  assert.equal(toolPromptHooks[0].function, module.exports.toolPromptPolicy);
  return { disk, readonly, routes, navigation, main: module.exports, tools, hostCalls, records, chatLifecycleHooks, toolLifecycleHooks, toolPromptHooks,
    /** Delegates only the real registered main-runtime operation channel, not a fixture domain implementation. */
    domain: registry.ipc.call,
    /** Invokes an actual registered API with the authenticated generic host request envelope. */
    async api(name, payload) { assert.equal(apis.has(name), true, name); return plain(await apis.get(name)({ callerPackage: "host", payload })); },
  };
}

/** Extracts the real Rust-owned SDK registration script without replacing callback serialization. */
async function policyRegistrationScript(file, functionName) {
  const source = await readFile(new URL("../../../../../core/crates/plugin/sdk/src/toolpkg/" + file, import.meta.url), "utf8");
  const signature = "pub fn " + functionName + "(";
  assert.equal(source.split(signature).length, 2, "Expected exactly one current SDK registration script");
  const start = source.indexOf('r#"', source.indexOf(signature)), end = source.indexOf('"#', start + 3);
  assert.ok(start >= 0 && end > start, "The real SDK must supply its current raw JavaScript bootstrap");
  return source.slice(start + 3, end);
}

/** Serializes the actual main-registered policies through the real SDK's exported-function resolver. */
async function captureRegisteredPolicies(plugin) {
  const context = vm.createContext({
    __operitCurrentCallId: "character-policy-registration",
    /** Supplies authenticated API-version metadata only, never storage or tool results. */
    __operitGetCallState(callId) { assert.equal(callId, "character-policy-registration"); return { params: { toolPkgId: "com.operit.character_cards", __operit_toolpkg_api_version: "2.0.0" } }; },
    /** Exposes the genuine evaluated main exports to the existing SDK callback resolver. */
    __operitGetActiveModuleExports() { return plugin.main; },
    lifecycle: plugin.toolLifecycleHooks, prompt: plugin.toolPromptHooks, chatLifecycle: plugin.chatLifecycleHooks,
  });
  vm.runInContext("/** Publishes the existing SDK namespaces into the isolated registration realm. */ globalThis.__operitExpose = function(name, value) { globalThis[name] = value; };", context);
  vm.runInContext(await policyRegistrationScript("ToolPkgApiRuntimeScript.rs", "buildToolPkgApiRuntimeScript"), context);
  const registration = await policyRegistrationScript("ToolPkgRegistrationBridge.rs", "buildToolPkgRegistrationBridgeScript");
  assert.equal(registration.split("__OPERIT_TOOLPKG_REGISTRATION_ONLY__").length, 2);
  vm.runInContext(registration.replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", "true"), context);
  vm.runInContext("for (const definition of lifecycle) ToolPkg.registerToolLifecycleHook(definition); for (const definition of prompt) ToolPkg.registerToolPromptComposeHook(definition); for (const definition of chatLifecycle) ToolPkg.registerChatLifecycleHook(definition);", context);
  const captured = JSON.parse(vm.runInContext("JSON.stringify({lifecycle:__operitToolPkgRegistrationCapture.toolLifecycleHooks.map(JSON.parse),prompt:__operitToolPkgRegistrationCapture.toolPromptComposeHooks.map(JSON.parse),chatLifecycle:__operitToolPkgRegistrationCapture.chatLifecycleHooks.map(JSON.parse)})", context));
  assert.throws(
    /** Requires the actual durable-reference resolver to reject a callback absent from all main exports. */
    () => vm.runInContext("ToolPkg.registerToolLifecycleHook({id:'unexported',function:/** Must never run; its unexported identity is rejected at registration. */ function unexportedPolicy() { throw new Error('An unexported policy must not run'); }});", context),
    /function must be exported from a toolpkg module/,
  );
  return captured;
}

/** Requires unique main bootstrap calls and resolvable real SDK captures without invoking any business hook. */
test("actual SDK capture resolves unique chat initialization and both tool policies from real main exports without business IO", async t => {
  assert.equal(importedCallCount("../src/main.ts", "./runtime-tools/policy", "registerToolPolicies"), 1);
  assert.equal(importedCallCount("../src/main.ts", "./chat-lifecycle", "registerChatInitialization"), 1);
  const plugin = await fileBackedPlugin(t, mainScript), captured = await captureRegisteredPolicies(plugin);
  assert.deepEqual(captured, { lifecycle: [{ id: "participant-tool-execution", function: "toolCallPolicy" }], prompt: [{ id: "participant-tool-visibility", function: "toolPromptPolicy" }], chatLifecycle: [{ id: "chat-initialization", function: "beforeChatCreate" }] });
  assert.equal(plugin.main[captured.lifecycle[0].function], plugin.toolLifecycleHooks[0].function);
  assert.equal(plugin.main[captured.prompt[0].function], plugin.toolPromptHooks[0].function);
  assert.equal(plugin.main[captured.chatLifecycle[0].function], plugin.chatLifecycleHooks[0].function);
  assert.equal(plugin.disk.calls.length, 0, "Policy capture/export resolution must not open the plugin store or any host directory");
});

/** Opens an actual registered screen and delegates browser messages to its real WebView bridge and file service. */
async function registeredPage(t, plugin, routeId, presentation, documentUrl) {
  const context = await browser.newContext({ viewport: { width: 1100, height: 950 } });
  t.after(
    /** Releases this test's browser context without changing any application or fixture state. */
    async () => context.close(),
  );
  const states = new Map([["presentation", presentation], ["chatId", "chat-dom"]]), refs = new Map();
  const completed = [], calls = [], errors = [];
  let bridge = null;
  const controller = {
    /** Captures the actual interface object published by the production route renderer. */
    addJavascriptInterface(name, value) { assert.equal(name, "CharacterMemoryHost"); bridge = value; },
    /** Rejects unexpected document evaluation instead of inventing an evaluation result. */
    evaluateJavascript() { throw new Error("Unexpected theme event during this isolated UI test"); },
  };
  const ctx = {
    /** Returns the documented WebView controller used by the actual renderer. */
    createWebViewController() { return controller; },
    /** Retains state requested by the existing Compose route contract. */
    useState(key, initial) { if (!states.has(key)) states.set(key, initial); return [states.get(key),
      /** Assigns only an explicit state update from the renderer. */
      value => states.set(key, value),
    ]; },
    /** Retains only references declared by the actual route. */
    useRef(key, initial) { if (!refs.has(key)) refs.set(key, { current: initial }); return refs.get(key); },
    Theme: {
      /** Supplies only an explicit isolated palette; no business data is sourced here. */
      getCurrent() { return plain(plugin.readonly.theme); },
      /** Retains the disposer without emitting an invented theme event. */
      subscribe() { return /** Disposes the isolated subscription. */ () => {}; },
    },
    UI: {
      /** Retains the production onLoad callback without interpreting opaque input. */
      Box(props, children) { return { props, children }; },
      /** Captures the real loading text node. */
      Text(props) { return { props }; },
      /** Captures existing WebView options without adding a new renderer or transport. */
      WebView(props) { return { props }; },
    },
  };
  const route = plugin.routes.find(
    /** Requires the exact registered route identifier requested by the test. */
    value => value.id === routeId,
  );
  assert.notEqual(route, undefined); await route.screen(ctx).props.onLoad(); assert.notEqual(bridge, null);
  await context.exposeFunction("invokeActualCharacterBridge",
    /** Routes each browser call through the actual nested-argument ABI and records only terminal V1 actions. */
    async (method, args) => {
      assert.equal(Object.hasOwn(bridge, method), true);
      calls.push({ method, args: plain(args) });
      const result = plain(await bridge[method](args));
      if (method === "completeScreen" || method === "cancelScreen") completed.push(result);
      return result;
    },
  );
  await context.addInitScript(
    /** Installs only transport delegates to the actual registered bridge, never successful CRUD implementations. */
    () => { window.CharacterMemoryHost = {
      /** Delegates the palette request to the renderer. */
      currentTheme: () => window.invokeActualCharacterBridge("currentTheme", []),
      /** Delegates the exact presented input to its real session. */
      currentScreen: () => window.invokeActualCharacterBridge("currentScreen", []),
      /** Delegates CRUD to the actual main IPC handler and native disk repository. */
      request: value => window.invokeActualCharacterBridge("request", [value]),
      /** Delegates completion to the actual plugin result adapter. */
      completeScreen: value => window.invokeActualCharacterBridge("completeScreen", [value]),
      /** Delegates explicit cancellation without an identity mutation. */
      cancelScreen: () => window.invokeActualCharacterBridge("cancelScreen", []),
      /** Delegates requested export IO to the actual existing host method. */
      exportFile: (file, content) => window.invokeActualCharacterBridge("exportFile", [file, content]),
      avatarImage: uri => window.invokeActualCharacterBridge("avatarImage", [uri]),
      chooseAvatar: () => window.invokeActualCharacterBridge("chooseAvatar", []),
    }; },
  );
  const page = await context.newPage();
  page.on("pageerror",
    /** Captures real browser exceptions for terminal assertions. */
    error => errors.push(error.message),
  );
  await page.goto(documentUrl); await page.locator("#app h1, #app h2, #app [role=alert], dialog").first().waitFor();
  assert.equal(await page.locator("#app [role=alert]").count(), 0, (await page.locator("#app [role=alert]").allTextContents()).join("\n"));
  return { page, context, bridge, completed, calls, errors };
}

/** Requires an explicit management input and rejects terminal actions that have no presentation caller. */
test("management fixture explicitly declares its screen and rejects completion and cancellation", async () => {
  const ui = await page();
  assert.deepEqual(await ui.page.evaluate(
    /** Reads the legal management presentation through its complete explicit test bridge. */
    () => window.CharacterMemoryHost.currentScreen(),
  ), { requestId: null, input: { mode: "manage" } });
  await assert.rejects(ui.page.evaluate(
    /** Verifies that an ordinary management view cannot invent a completion recipient. */
    () => window.CharacterMemoryHost.completeScreen({ mode: "preview", entity: "card", id: "travel" }),
  ), /Management has no presentation completion channel/);
  await assert.rejects(ui.page.evaluate(
    /** Verifies that an ordinary management view cannot invent a cancellation recipient. */
    () => window.CharacterMemoryHost.cancelScreen(),
  ), /Management has no presentation cancellation channel/);
  await ui.context.close();
});

/** Exercises actual editor DOM and picker dispatch with clearly labelled presentation-only directory inputs. */
test("theme picker DOM searches exact independent IDs, cancels without writes, and stages only the character reference", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("tab", { name: "绑定", exact: true }).click();
  await editor.getByRole("button", { name: "选择主题配置", exact: true }).click();
  let picker = ui.page.getByRole("dialog", { name: "选择主题配置", exact: true });
  await picker.getByLabel("搜索主题配置", { exact: true }).fill("夜间");
  assert.equal(await picker.getByRole("button", { name: "选择主题 测试独立主题", exact: true }).count(), 0);
  assert.equal(await picker.getByRole("button", { name: "选择主题 测试夜间主题", exact: true }).count(), 1);
  await picker.getByRole("button", { name: "取消", exact: true }).click();
  assert.equal(await ui.page.evaluate(
    /** Reads only the explicit browser fixture audit; opening and cancelling must not invoke save or activation. */
    () => window.testCalls.filter(value => value.action !== "snapshot" && value.action !== "listThemeChoices").length,
  ), 0);
  await editor.getByRole("button", { name: "选择主题配置", exact: true }).click();
  picker = ui.page.getByRole("dialog", { name: "选择主题配置", exact: true });
  await picker.getByRole("button", { name: "选择主题 测试夜间主题", exact: true }).click();
  assert.match(await editor.textContent(), /测试夜间主题/);
  assert.equal(await ui.page.evaluate(
    /** Confirms the chooser has edited only its local draft, not the saved isolated fixture record. */
    () => window.testRecords.cards.find(value => value.id === "travel").themeConfigId,
  ), null);
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await ui.page.waitForFunction(
    /** Waits for the explicit save response to be installed, not a screenshot or a predetermined success indicator. */
    () => window.testRecords.cards.find(value => value.id === "travel").themeConfigId === "independent-theme-two",
  );
  const calls = await ui.page.evaluate(
    /** Copies the actual business requests dispatched by the production browser module. */
    () => window.testCalls,
  );
  const saved = calls.filter(value => value.action === "saveCharacter"); assert.equal(saved.length, 1);
  assert.equal(saved[0].card.themeConfigId, "independent-theme-two"); assert.equal(saved[0].card.ttsConfigId, null);
  assert.equal(calls.filter(value => value.action === "activate" || value.action === "writeChatBinding").length, 0);
  await ui.context.close();
});

/** Uses the actual group editor while making every response and UI-only directory source explicit in the fixture. */
test("group theme picker DOM saves its own reference without changing members or adding participant TTS", async () => {
  const data = fixture(); data.groupSaveSnapshot = plain(data.snapshot); data.groupSaveSnapshot.groups[0].themeConfigId = "independent-theme-one";
  const ui = await page(1100, 950, data);
  await ui.page.getByRole("button", { name: "编辑群组 日常讨论", exact: true }).click();
  const editor = ui.page.getByRole("dialog", { name: "编辑群组", exact: true });
  await editor.getByRole("button", { name: "选择主题配置", exact: true }).click();
  const picker = ui.page.getByRole("dialog", { name: "选择主题配置", exact: true });
  await picker.getByRole("button", { name: "选择主题 测试独立主题", exact: true }).click();
  assert.match(await editor.textContent(), /测试独立主题/);
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await ui.page.waitForFunction(
    /** Waits for the fully declared group response rather than emulating a group repository in the browser. */
    () => window.testRecords.groups[0].themeConfigId === "independent-theme-one",
  );
  const saved = await ui.page.evaluate(
    /** Reads exactly one actual group-save request from the production dispatcher. */
    () => window.testCalls.filter(value => value.action === "saveGroup"),
  );
  assert.equal(saved.length, 1); assert.equal(saved[0].group.themeConfigId, "independent-theme-one");
  assert.deepEqual(saved[0].group.members, data.snapshot.groups[0].members); assert.equal(Object.hasOwn(saved[0].group, "ttsConfigId"), false);
  await ui.context.close();
});

/** Missing directory access stays an explicit visible error; no implicit empty picker, save, retry or success is permitted. */
test("theme picker directory rejection stays in the same editor and performs zero reference saves", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("tab", { name: "绑定", exact: true }).click();
  await ui.page.evaluate(
    /** Injects an explicitly planned original directory failure at the test transport boundary only. */
    () => { window.testFailure = "listThemeChoices"; },
  );
  await editor.getByRole("button", { name: "选择主题配置", exact: true }).click();
  await editor.getByRole("alert").filter({ hasText: "TEST_HOST_REJECTED" }).waitFor();
  assert.equal(await ui.page.getByRole("dialog", { name: "选择主题配置", exact: true }).count(), 0);
  await editor.getByRole("button", { name: "取消", exact: true }).click();
  const calls = await ui.page.evaluate(
    /** Audits exact directory attempts and zero mutations after the failed reader and explicit cancellation. */
    () => window.testCalls,
  );
  assert.equal(calls.filter(value => value.action === "listThemeChoices").length, 2);
  assert.equal(calls.filter(value => value.action === "saveCharacter" || value.action === "activate" || value.action === "writeChatBinding").length, 0);
  await ui.context.close();
});

/** Keeps the plugin settings entry within the pre-migration character/group surface. */
test("character settings remove the whole page title and retain theme binding", async () => {
  const data = fixture(); data.snapshot.cards[1].themeConfigId = "saved-independent-theme";
  const ui = await page(1100, 950, data);
  assert.equal(await ui.page.locator('#app [data-action="refresh"], #app [data-action="preview-group"]').count(), 0);
  assert.equal(await ui.page.locator("#app .appbar, #app h1").count(), 0);
  assert.equal((await ui.page.locator("#app .scroll").boundingBox()).y, 0);
  await ui.page.screenshot({ path: output + "character-settings-no-title.png" });
  assert.equal(await ui.page.getByRole("button", { name: "编辑群组", exact: true }).count(), 0);
  await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("tab", { name: "绑定", exact: true }).click();
  assert.equal(await editor.locator(".binding").count(), 5);
  assert.deepEqual(await editor.getByRole("switch").evaluateAll(nodes => nodes.map(node => node.getAttribute("aria-label"))), ["聊天模型", "TTS 配置", "记忆绑定", "工具访问"]);
  assert.equal(await editor.getByRole("button", { name: "选择主题配置", exact: true }).count(), 1);
  await editor.getByRole("tab", { name: "基础", exact: true }).click();
  await editor.getByLabel("角色名称 *", { exact: true }).fill("原有角色编辑");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await ui.page.getByRole("button", { name: "编辑 原有角色编辑", exact: true }).waitFor();
  const calls = await ui.page.evaluate(() => window.testCalls);
  assert.equal(calls.filter(call => call.action === "listThemeChoices").length, 1);
  assert.equal(calls.find(call => call.action === "saveCharacter").card.themeConfigId, "saved-independent-theme");
  await ui.context.close();
});

/** Removes the added group UI without deleting data already stored on that group. */
test("group settings retain theme picker, fields and members without duplicate row actions", async () => {
  const data = fixture(); data.snapshot.groups[0].themeConfigId = "saved-group-theme";
  data.groupSaveSnapshot = plain(data.snapshot);
  const ui = await page(1100, 950, data);
  const tile = ui.page.locator(".entity").filter({ has: ui.page.getByRole("button", { name: "编辑群组 日常讨论", exact: true }) });
  assert.equal(await tile.locator(".entity-subtitle").textContent(), "Operit、旅行助理");
  assert.equal(await tile.locator(".badge").count(), 1);
  assert.equal(await tile.locator(".entity-actions button").count(), 1);
  await tile.getByRole("button", { name: "编辑群组 日常讨论", exact: true }).click();
  const editor = ui.page.getByRole("dialog", { name: "编辑群组", exact: true });
  assert.equal(await editor.locator(".field").count(), 2);
  assert.equal(await editor.getByRole("checkbox").count(), data.snapshot.cards.length);
  assert.equal(await editor.getByRole("button", { name: "选择主题配置", exact: true }).count(), 1);
  assert.equal(await editor.locator(".notice").count(), 0);
  assert.equal(await editor.locator(".binding").count(), 1);
  assert.equal(await editor.getByRole("button", { name: "导出 JSON", exact: true }).count(), 1);
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  await editor.waitFor({ state: "detached" });
  const calls = await ui.page.evaluate(() => window.testCalls);
  const saved = calls.find(call => call.action === "saveGroup");
  assert.equal(saved.group.themeConfigId, "saved-group-theme");
  assert.deepEqual(saved.group.members, data.snapshot.groups[0].members);
  assert.equal(calls.filter(call => call.action === "listThemeChoices").length, 1);
  await ui.context.close();
});

/** Deleted refs remain explicit until the user chooses a real independent ID or intentionally removes the reference. */
test("deleted theme and TTS references remain visible in picker DOM and cancellation does not replace them", async () => {
  const data = fixture(); data.snapshot.cards[1].themeConfigId = "deleted-theme"; data.snapshot.cards[1].ttsConfigId = "deleted-tts";
  const ui = await page(1100, 950, data); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("tab", { name: "绑定", exact: true }).click();
  assert.match(await editor.getByRole("alert").filter({ hasText: "已绑定 TTS" }).textContent(), /已绑定 TTS 配置不存在：deleted-tts/);
  await editor.getByRole("button", { name: "选择主题配置", exact: true }).click();
  const theme = ui.page.getByRole("dialog", { name: "选择主题配置", exact: true });
  assert.match(await theme.getByRole("alert").textContent(), /已绑定主题配置不存在：deleted-theme/);
  await theme.getByRole("button", { name: "取消", exact: true }).click();
  await editor.getByRole("button", { name: "选择 TTS 配置", exact: true }).click();
  const tts = ui.page.getByRole("dialog", { name: "选择 TTS 配置", exact: true });
  assert.match(await tts.getByRole("alert").textContent(), /已绑定 TTS 配置不存在：deleted-tts/);
  await tts.getByRole("button", { name: "取消", exact: true }).click();
  await editor.getByRole("button", { name: "取消", exact: true }).click();
  const retained = await ui.page.evaluate(
    /** Reads the unchanged declared record and mutation audit, not a default or synthesized unbound value. */
    () => ({ card: window.testRecords.cards.find(value => value.id === "travel"), saves: window.testCalls.filter(value => value.action === "saveCharacter").length }),
  );
  assert.equal(retained.card.themeConfigId, "deleted-theme"); assert.equal(retained.card.ttsConfigId, "deleted-tts"); assert.equal(retained.saves, 0);
  await ui.context.close();
});

/** An explicitly empty independent speech directory must not trap a saved deleted reference behind a disabled unbind control. */
test("empty TTS directory permits only explicit user unbinding and never chooses a replacement config", async () => {
  const data = fixture(); data.snapshot.ttsConfigs = []; data.snapshot.cards[1].ttsConfigId = "deleted-tts";
  const ui = await page(1100, 950, data); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("tab", { name: "绑定", exact: true }).click();
  const binding = editor.getByRole("switch", { name: "TTS 配置", exact: true }); assert.equal(await binding.isEnabled(), true);
  assert.match(await editor.getByRole("alert").textContent(), /deleted-tts/);
  await binding.uncheck(); await editor.getByRole("button", { name: "保存", exact: true }).click();
  await ui.page.waitForFunction(
    /** Confirms the actual explicit save submitted null, not an automatic response to the missing record. */
    () => window.testRecords.cards.find(value => value.id === "travel").ttsConfigId === null,
  );
  const saved = await ui.page.evaluate(
    /** Retains the actual staged reference field and zero global-configuration operations. */
    () => window.testCalls.filter(value => value.action === "saveCharacter"),
  );
  assert.equal(saved.length, 1); assert.equal(saved[0].card.ttsConfigId, null); await ui.context.close();
});

/** Keeps binding controls editable and proves they remain staged until the one complete character save. */
test("TTS and four tool-source categories stage real directory choices and save their complete binding fields", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  await editor.getByRole("tab", { name: "绑定", exact: true }).click();
  await editor.getByRole("switch", { name: "TTS 配置", exact: true }).check();
  await editor.getByRole("button", { name: "选择 TTS 配置", exact: true }).click();
  const tts = ui.page.getByRole("dialog", { name: "选择 TTS 配置", exact: true });
  await tts.getByLabel("搜索 TTS 配置", { exact: true }).fill("测试语音");
  await tts.getByRole("button", { name: "选择 TTS 测试语音", exact: true }).click();
  await editor.getByRole("switch", { name: "工具访问", exact: true }).check();
  await editor.getByRole("button", { name: "配置允许使用的工具", exact: true }).click();
  const policy = ui.page.getByRole("dialog", { name: "自定义允许使用的工具", exact: true });
  for (const name of ["fixture-builtin", "fixture-package", "fixture-skill", "fixture-mcp"]) await policy.locator(`[data-tool-source="${name}"]`).check();
  await policy.getByRole("button", { name: "确定", exact: true }).click();
  const before = await ui.page.evaluate(
    /** Reads only emitted requests to prove nested directory picks have not persisted the parent draft. */
    () => window.testCalls,
  );
  assert.equal(before.some(
    /** Requires the single authoritative save to remain uncalled while picks are staged. */
    call => call.action === "saveCharacter",
  ), false);
  await editor.getByRole("button", { name: "保存", exact: true }).click(); await editor.waitFor({ state: "hidden" });
  const calls = await ui.page.evaluate(
    /** Reads the actual full-record request emitted by the one explicit save. */
    () => window.testCalls,
  );
  const saves = calls.filter(
    /** Counts only complete character writes, never a nested chooser action. */
    call => call.action === "saveCharacter",
  );
  assert.equal(saves.length, 1); assert.equal(saves[0].card.ttsConfigId, "tts-one");
  assert.deepEqual(saves[0].card.sharedMemoryMounts, fixture().snapshot.cards[1].sharedMemoryMounts);
  assert.deepEqual(saves[0].card.toolAccessConfig, { enabled: true, allowedBuiltinTools: ["fixture-builtin"], allowedPackages: ["fixture-package"], allowedSkills: ["fixture-skill"], allowedMcpServers: ["fixture-mcp"] });
  await ui.context.close();
});

/** Exercises source-built routes, real host completion and native file persistence without fixture storage. */
test("current plugin routes connect selector preview and memory attachment to the actual file service", async t => {
  const plugin = await fileBackedPlugin(t, mainScript);
  const card = await plugin.api("character.create", { values: { name: "真实选择角色", description: "Native disk card", characterSetting: "Persisted setting", openingStatement: "Persisted opening" } });
  const group = await plugin.api("group.create", { values: { name: "真实选择组", description: "Native disk group", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  const ownerKey = "character:" + card.id;
  const first = await plugin.api("memory.create", { ownerKey, values: { title: "真实附件第一条", content: "真实文件中的完整记忆内容", folderPath: "附件根", contentType: "text/plain", source: "manual", tags: ["native-test"] } });
  const second = await plugin.api("memory.create", { ownerKey, values: { title: "真实附件第二条", content: "第二条完整内容", folderPath: "附件根", contentType: "text/plain", source: "manual" } });
  await plugin.api("memory.create", { ownerKey, values: { title: "子目录不应混入", content: "Descendant content", folderPath: "附件根/child" } });
  await plugin.api("chat.configuration.binding.write", { chatId: "chat-dom", selection: "card:default" });
  const active = await plugin.api("activePrompt.get", {});
  /** Cancels a staged selection and verifies both persisted chat and global identities. */
  await t.test("selector cancellation never changes the stored binding or global active prompt", async s => {
    const menu = await plugin.api("chat.context.actions", { chatId: "chat-dom" });
    assert.equal(Object.hasOwn(menu, "selectors"), false);
    const ui = mountRegisteredSelector(plugin, { requestId: "selector-cancel", input: { mode: "select", chatId: "chat-dom", kind: "card", selected: null } }); await ui.load();
    assert.equal(keyedNode(ui.render().tree, "card:" + card.id).type, "Row");
    const before = plain(plugin.records.get("chat-dom").extension), count = plugin.hostCalls.length;
    const cancelled = await ui.dispatch(keyedNode(ui.render().tree, "selector-close").props.onClick);
    assert.deepEqual(plain(cancelled.actionResult), { type: "toolpkg.presentation.cancel", requestId: "selector-cancel" });
    assert.deepEqual(plugin.records.get("chat-dom").extension, before); assert.equal(plugin.hostCalls.length, count);
    assert.deepEqual(await plugin.api("chat.configuration.binding.read", { chatId: "chat-dom" }), { chatId: "chat-dom", selection: "card:default" });
    assert.deepEqual(await plugin.api("activePrompt.get", {}), active);
  });
  /** Commits through actual main IPC and rejects a second completion before any repeated write. */
  await t.test("card selector commits actual main IPC before emitting the opaque V1 value exactly once", async s => {
    const menu = await plugin.api("chat.context.actions", { chatId: "chat-dom" });
    assert.equal(Object.hasOwn(menu, "selectors"), false);
    const ui = mountRegisteredSelector(plugin, { requestId: "selector-card", input: { mode: "select", chatId: "chat-dom", kind: "card", selected: null } }); await ui.load();
    const row = keyedNode(ui.render().tree, "card:" + card.id), committed = await ui.dispatch(row.props.onClick);
    assert.deepEqual(plain(committed.actionResult), { type: "toolpkg.presentation.complete", requestId: "selector-card", value: { selection: "card:" + card.id, contextKey: "card:" + card.id } });
    const file = path.join(plugin.disk.directory, "character-memory/state.json"), before = await plugin.disk.stateBytes();
    const stored = JSON.parse(before.toString("utf8")), binding = plain(plugin.records.get("chat-dom").extension), count = plugin.hostCalls.length;
    assert.equal(Object.hasOwn(stored, "chatBindings"), false, "Bindings belong to the native record namespace, not plugin files");
    assert.deepEqual(binding, { version: 1, selection: "card:" + card.id });
    await assert.rejects(ui.dispatch(row.props.onClick), /already finished/);
    assert.equal(plugin.hostCalls.length, count, "Repeated native action must not issue another extension read or write");
    assert.deepEqual(plugin.records.get("chat-dom").extension, binding); assert.deepEqual(await plugin.disk.stateBytes(), before);
    assert.deepEqual(await plugin.api("activePrompt.get", {}), active);
  });
  /** Confirms a real group binding while preserving the global active identity. */
  await t.test("group selector persists its actual identity without globally activating it", async s => {
    const ui = mountRegisteredSelector(plugin, { requestId: "selector-group", input: { mode: "select", chatId: "chat-dom", kind: "group", selected: null } }); await ui.load();
    const committed = await ui.dispatch(keyedNode(ui.render().tree, "group:" + group.id).props.onClick);
    assert.deepEqual(plain(committed.actionResult), { type: "toolpkg.presentation.complete", requestId: "selector-group", value: { selection: "group:" + group.id, contextKey: "group:" + group.id } });
    assert.deepEqual(plugin.records.get("chat-dom").extension, { version: 1, selection: "group:" + group.id });
    assert.equal((await plugin.api("chat.configuration.binding.read", { chatId: "chat-dom" })).selection, "group:" + group.id);
    assert.deepEqual(await plugin.api("activePrompt.get", {}), active);
  });
  /** Saves a nested preview editor without ending the caller until the root preview closes. */
  await t.test("identity preview opens the actual plugin popup and its nested editor saves genuine records", async s => {
    await plugin.api("chat.configuration.binding.write", { chatId: "chat-dom", selection: "card:" + card.id });
    const menu = await plugin.api("chat.context.actions", { chatId: "chat-dom" });
    const ui = await registeredPage(s, plugin, "main", { requestId: "preview-card", input: menu.identity.action.input }, origin);
    const preview = ui.page.getByRole("dialog", { name: "角色卡预览", exact: true }); await preview.waitFor();
    assert.equal(await preview.getByText("Persisted setting", { exact: true }).count(), 1);
    await preview.getByRole("button", { name: "编辑", exact: true }).click();
    const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
    await editor.getByLabel("描述", { exact: true }).fill("真实 popup 提交后的描述");
    await editor.getByRole("button", { name: "保存", exact: true }).click(); await editor.waitFor({ state: "hidden" });
    assert.equal((await plugin.api("character.get", { id: card.id })).description, "真实 popup 提交后的描述");
    assert.deepEqual(ui.completed, []); await preview.locator("[data-action=close-dialog]").first().click();
    await ui.page.getByText("当前视图已结束。", { exact: true }).waitFor();
    assert.deepEqual(ui.completed, [{ type: "toolpkg.presentation.complete", requestId: "preview-card", value: { mode: "preview", entity: "card", id: card.id } }]);
    assert.deepEqual(ui.errors, []);
  });
  /** Exercises the formal attachment entry, exact complete records and generic terminal payload. */
  await t.test("formal plus-menu entry picks genuine owner folder records and emits only V1 generic text", async s => {
    const entry = plugin.navigation.find(
      /** Selects the exact official attachment registration rather than a toolbox or API source. */
      value => value.surface === "chat_attachments",
    );
    assert.notEqual(entry, undefined); assert.equal(entry.route, "toolpkg:com.operit.character_cards:ui:memory-attachment");
    assert.deepEqual(plain(entry.params), { mode: "memory-attachment", ownerKey: null, folderPath: null });
    const ui = await registeredPage(s, plugin, "memory-attachment", { requestId: "attachment-native", input: plain(entry.params) }, origin);
    assert.deepEqual(plain(ui.bridge.currentScreen([])), { requestId: "attachment-native", input: plain(entry.params) });
    await ui.page.getByRole("button", { name: "真实选择角色", exact: true }).click();
    await ui.page.getByRole("button", { name: "附件根", exact: true }).click();
    const content = await ui.page.getByLabel("记忆附件正文", { exact: true }).textContent();
    assert.equal(content, "# 真实选择角色 / 附件根\n\n## 真实附件第一条\n\n真实文件中的完整记忆内容\n\n## 真实附件第二条\n\n第二条完整内容");
    await ui.page.getByRole("button", { name: "添加记忆附件", exact: true }).click(); await ui.page.getByText("当前视图已结束。", { exact: true }).waitFor();
    assert.deepEqual(ui.completed, [{ type: "toolpkg.presentation.complete", requestId: "attachment-native", value: { type: "text", name: "记忆附件", content, mediaType: "text/plain" } }]);
    assert.equal(ui.calls.some(
      /** Requires a real full-record request scoped to the selected private owner. */
      call => call.method === "request" && call.args[0].action === "listMemories" && call.args[0].ownerKey === ownerKey,
    ), true);
    assert.deepEqual(ui.errors, []);
  });
  /** Cancels the actual independent attachment route without publishing a text value or touching stored bindings. */
  await t.test("attachment cancellation returns explicit V1 cancel and does not mutate native files", async s => {
    const ui = await registeredPage(s, plugin, "memory-attachment", { requestId: "attachment-cancel", input: { mode: "memory-attachment", ownerKey, folderPath: "附件根" } }, origin);
    await ui.page.getByLabel("记忆附件正文", { exact: true }).waitFor();
    const file = path.join(plugin.disk.directory, "character-memory/state.json"), before = await plugin.disk.stateBytes();
    await ui.page.getByRole("button", { name: "取消", exact: true }).click();
    await ui.page.getByText("当前视图已结束。", { exact: true }).waitFor();
    assert.deepEqual(ui.completed, [{ type: "toolpkg.presentation.cancel", requestId: "attachment-cancel" }]);
    assert.deepEqual(await plugin.disk.stateBytes(), before);
    await assert.rejects(async () => ui.bridge.completeScreen([{ mode: "memory-attachment", ownerKey, folderPath: "附件根", content: "Invalid post-cancel content" }]), /already finished/);
    assert.deepEqual(await plugin.disk.stateBytes(), before); assert.deepEqual(ui.errors, []);
  });
  /** Enforces caller scope in the actual plugin adapter before permitting any terminal result. */
  await t.test("locked attachment input cannot navigate outside its scope or complete another owner or folder", async s => {
    const ui = await registeredPage(s, plugin, "memory-attachment", { requestId: "attachment-locked", input: { mode: "memory-attachment", ownerKey, folderPath: "附件根" } }, origin);
    await ui.page.getByLabel("记忆附件正文", { exact: true }).waitFor();
    assert.equal(await ui.page.getByRole("button", { name: "返回", exact: true }).count(), 0);
    await assert.rejects(async () => ui.bridge.completeScreen([{ mode: "memory-attachment", ownerKey: "character:default", folderPath: "附件根", content: "Wrong owner" }]), /attachment scope/);
    await assert.rejects(async () => ui.bridge.completeScreen([{ mode: "memory-attachment", ownerKey, folderPath: "附件根/child", content: "Wrong folder" }]), /attachment scope/);
    assert.deepEqual(ui.completed, []);
    await ui.page.getByRole("button", { name: "取消", exact: true }).click();
    await ui.page.getByText("当前视图已结束。", { exact: true }).waitFor();
    assert.deepEqual(ui.completed, [{ type: "toolpkg.presentation.cancel", requestId: "attachment-locked" }]);
    assert.deepEqual(ui.errors, []);
  });
  /** Performs real SVG gestures and confirms the relationship in the authoritative file-backed graph. */
  await t.test("native SVG click drag and link submission use genuine persisted memory records", async s => {
    const ui = await registeredPage(s, plugin, "memory", null, origin);
    const editor = await openCharacterMemory(ui.page, "真实选择角色"); await editor.getByRole("button", { name: "记忆图谱", exact: true }).click();
    const graph = ui.page.getByRole("dialog", { name: "真实选择角色 的记忆图谱", exact: true });
    const node = graph.locator(`.graph-node[data-id="${first.item.uuid}"]`); await node.waitFor();
    const box = await node.locator("rect").boundingBox(); assert.notEqual(box, null);
    await ui.page.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await ui.page.mouse.down();
    assert.equal(await graph.locator(".graph-node.selected").count(), 0); await ui.page.mouse.up();
    assert.equal(await graph.locator(".graph-node.selected").count(), 1);
    const before = await node.getAttribute("transform"); await ui.page.mouse.down();
    await ui.page.mouse.move(box.x + box.width / 2 + 30, box.y + box.height / 2 + 20, { steps: 4 }); await ui.page.mouse.up();
    assert.notEqual(await node.getAttribute("transform"), before);
    await graph.getByRole("button", { name: "链接", exact: true }).click(); await node.click();
    await graph.locator(`.graph-node[data-id="${second.item.uuid}"]`).click();
    const link = ui.page.getByRole("dialog", { name: "创建记忆链接", exact: true });
    await link.getByLabel("关系类型", { exact: true }).fill("真实图谱关系");
    await link.getByLabel("描述", { exact: true }).fill("Actual SVG input committed to native files");
    await link.getByRole("button", { name: "创建", exact: true }).click(); await graph.locator(".graph-edge-action").waitFor();
    const native = await plugin.api("memory.graph", { ownerKey });
    assert.equal(native.graph.edges.length, 1); assert.equal(native.graph.edges[0].label, "真实图谱关系");
    assert.deepEqual(ui.errors, []);
  });
});

/** Runs the installed main and offline HTML artifacts, separately from current-source transpilation checks. */
test("installed plugin artifacts launch management and the independent V1 text attachment UI", async t => {
  const installedMain = await readFile(new URL("../dist/main.js", import.meta.url));
  assert.equal(createHash("sha256").update(installedMain).digest("hex"), createHash("sha256").update(mainScript).digest("hex"), "Installed main bytes must match current-source SHA256; rebuild the single plugin after source freeze");
  assert.equal(createHash("sha256").update(installedHtml).digest("hex"), createHash("sha256").update(html).digest("hex"), "Installed HTML must match current-source SHA256; stale resources are not UI acceptance");
  const installedScreens = {};
  for (const file of Object.keys(await buildUiScreenScripts())) installedScreens[file] = await readFile(new URL("../" + file, import.meta.url));
  const plugin = await fileBackedPlugin(t, installedMain, installedScreens);
  const card = await plugin.api("character.create", { values: { name: "产物角色", description: "Installed artifact native record" } });
  const ownerKey = "character:" + card.id;
  await plugin.api("memory.create", { ownerKey, values: { title: "产物记忆", content: "生成 HTML 读取到的完整文件内容", folderPath: "产物目录" } });
  /** Opens management from the actual installed resource and rejects unowned terminal results. */
  await t.test("installed management explicitly exposes its true CurrentScreen and full typed editor", async s => {
    const ui = await registeredPage(s, plugin, "main", null, origin + "/installed");
    assert.deepEqual(plain(ui.bridge.currentScreen([])), { requestId: null, input: { mode: "manage" } });
    await ui.page.getByRole("button", { name: "编辑 产物角色", exact: true }).click();
    const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
    assert.equal(await editor.getByRole("tab").count(), 3);
    await editor.getByRole("tab", { name: "绑定", exact: true }).click();
    assert.equal(await editor.getByRole("switch", { name: "TTS 配置", exact: true }).isEnabled(), true);
    await assert.rejects(async () => ui.bridge.completeScreen([{ mode: "preview", entity: "card", id: card.id }]), /Management has no presentation completion channel/);
    assert.throws(
      /** Verifies actual management has no cancellation receiver rather than mocking a successful closure. */
      () => ui.bridge.cancelScreen([]), /management route has no presentation result channel/,
    );
    assert.deepEqual(ui.errors, []);
  });
  /** Reads real persisted memory through the generated browser bundle and completes its actual attachment route. */
  await t.test("installed attachment resource completes with exact generic text and no owner fields", async s => {
    const ui = await registeredPage(s, plugin, "memory-attachment", { requestId: "installed-attachment", input: { mode: "memory-attachment", ownerKey, folderPath: "产物目录" } }, origin + "/installed");
    await ui.page.getByLabel("记忆附件正文", { exact: true }).waitFor();
    const content = "# 产物角色 / 产物目录\n\n## 产物记忆\n\n生成 HTML 读取到的完整文件内容";
    assert.equal(await ui.page.getByLabel("记忆附件正文", { exact: true }).textContent(), content);
    await ui.page.getByRole("button", { name: "添加记忆附件", exact: true }).click();
    await ui.page.getByText("当前视图已结束。", { exact: true }).waitFor();
    assert.deepEqual(ui.completed, [{ type: "toolpkg.presentation.complete", requestId: "installed-attachment", value: { type: "text", name: "记忆附件", content, mediaType: "text/plain" } }]);
    assert.deepEqual(ui.errors, []);
  });
});

/** The character page has no inner title; the independent memory header remains stationary. */
test("character scroll uses the full viewport without a title while memory keeps its fixed header", async () => {
  const data = fixture();
  for (let i = 0; i < 24; i++) data.snapshot.cards.push({ ...data.snapshot.cards[1], id: "scroll-" + i, name: "滚动角色 " + i, isDefault: false });
  const ui = await page(800, 500, data);
  for (const document of [origin, origin + "/memory"]) {
    await ui.page.goto(document);
    await ui.page.locator(".entity").first().waitFor();
    assert.equal(await ui.page.locator('[data-action="memory-settings"], [data-action="characters-settings"]').count(), 0);
    const memory = document.endsWith("/memory");
    assert.equal(await ui.page.locator(".appbar").count(), memory ? 1 : 0);
    const before = memory ? await ui.page.locator(".appbar").boundingBox() : null;
    const metrics = await ui.page.locator(".scroll").evaluate(el => ({ client: el.clientHeight, content: el.scrollHeight }));
    assert.ok(metrics.content > metrics.client);
    if (memory) assert.ok(metrics.client < 500);
    else assert.equal(metrics.client, 500);
    await ui.page.locator(".scroll").hover(); await ui.page.mouse.wheel(0, 700);
    await ui.page.waitForFunction(() => document.querySelector(".scroll").scrollTop > 0);
    if (memory) assert.deepEqual(await ui.page.locator(".appbar").boundingBox(), before);
    else assert.equal(await ui.page.locator(".appbar, h1").count(), 0);
  }
  await ui.context.close();
});

/** Checks the original flexible avatar row, explicit picker transport and the authored persistent URI. */
test("avatar editor keeps old 44px preview and right-aligned choose/clear controls", async () => {
  const ui = await page(); await openCharacter(ui.page);
  const editor = ui.page.getByRole("dialog", { name: "编辑角色卡", exact: true });
  const row = editor.locator(".avatar-editor");
  const image = await row.locator(".avatar").boundingBox(), bounds = await row.boundingBox();
  assert.equal(image.width, 44); assert.equal(image.height, 44);
  const choose = await row.getByRole("button", { name: "选择", exact: true }).boundingBox();
  assert.ok(bounds.x + bounds.width - choose.x - choose.width <= 14);
  await ui.page.evaluate(() => {
    window.CharacterMemoryHost.chooseAvatar = async () => ({ uri: "/app/data/plugin/avatars/selected.png", source: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a3ioAAAAASUVORK5CYII=" });
  });
  await row.getByRole("button", { name: "选择", exact: true }).click();
  await row.getByRole("button", { name: "清除", exact: true }).waitFor();
  assert.ok(await row.locator("img").getAttribute("src"));
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  const saved = await ui.page.evaluate(() => window.testCalls.find(call => call.action === "saveCharacter"));
  assert.equal(saved.card.avatarUri, "/app/data/plugin/avatars/selected.png");
  await ui.context.close();
});
