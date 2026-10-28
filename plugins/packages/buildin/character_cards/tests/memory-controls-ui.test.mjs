import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { mkdir } from "node:fs/promises";
import http from "node:http";
import test from "node:test";
import { createHtmlDocument } from "../scripts/build.mjs";
import { fixture } from "./fixtures.mjs";
import { plain } from "./runtime.mjs";
import { CONTROLLED_CHAT_ID, CONTROLLED_OWNER, createMemoryJobsHarness } from "./memory-jobs-runtime.mjs";
const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { chromium } = require("playwright");
const html = await createHtmlDocument();
let browser, server, origin;
const output = fileURLToPath(new URL("../../../../../.tmp/character-cards/", import.meta.url));

test.before(async () => {
  server = http.createServer((_request, response) => { response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" }); response.end(html); });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  browser = await chromium.launch({ headless: true }); await mkdir(output, { recursive: true });
});
test.after(async () => { await browser?.close(); if (server) await new Promise(resolve => server.close(resolve)); });

/** Uses actual registered main IPC, repository transactions and temporary-disk persistence, not browser CRUD fixtures. */
async function controlsPage(t, width = 390, view = "memory") {
  const harness = await createMemoryJobsHarness(t), presentation = fixture();
  // Only readonly independent host directories are controlled; all domain settings and job state belong to production.
  Object.assign(harness.tools, {Models: {
    async listSummaries() { return { models: plain(presentation.snapshot.models) }; }
  }, Tts: {
    async listConfigs() { return { configs: plain(presentation.snapshot.ttsConfigs) }; }
  }, Catalog: {
    async readToolSources() { return plain(presentation.snapshot.toolCatalog); }
  }, Theme: {
    async listConfigs() { return { configs: [] }; }
  }});
  const runtime = harness.openRuntime(); await runtime.initialize(); await runtime.bind();
  const context = await browser.newContext({ viewport: { width, height: 844 }, timezoneId: "Asia/Shanghai" });
  t.after(() => context.close());
  const requests = [];
  await context.exposeFunction("dispatchRealMemoryControlRequest", async request => {
    requests.push(plain(request)); return plain(await runtime.web(request));
  });
  await context.exposeFunction("exportRealMemoryFile", async (path, content) => {
    await harness.tools.Files.write(path, content, false); return { cancelled: false, path };
  });
  await context.exposeFunction("readRealMemoryFile", async path => {
    return { cancelled: false, content: (await harness.tools.Files.read(path)).content };
  });
  await context.addInitScript(({ theme, view }) => {
    window.rejectMemoryAction = null;
    window.CharacterMemoryHost = {
      async loadScreen() { return { theme, screen: { requestId: null, input: { mode: "manage", view } }, snapshot: await this.request({ action: "snapshot" }) }; },
      async request(request) {
        if (window.rejectMemoryAction === request.action) throw new Error("Explicit test transport rejection");
        const result = await window.dispatchRealMemoryControlRequest(request);
        // Tests a lossy graph transport, never a substitute canonical relationship record.
        if (window.omitGraphDescription && request.action === "graph") for (const edge of result.graph.edges) edge.metadata = {};
        return result;
      },
      async avatarImage() { return null; },
      async openEntity() { throw new Error("No avatar picker configured for memory-controls test"); },
      /** Supplies only the host selection; the service imports actual file contents. */
      async readImportFile() {
        if (window.importSource === undefined) return { cancelled: true };
        return window.readRealMemoryFile(window.importSource);
      },
      /** Separates the host user's destination and cancellation from plugin preview state. */
      async exportFile(name, content) {
        if (window.rejectExport === true) throw new Error("Host save denied");
        if (window.cancelExport === true) return { cancelled: true, path: null };
        if (name !== "memory-backup.json") throw new Error("Unexpected export name");
        return window.exportRealMemoryFile(window.exportDestination, content);
      },
      async completeScreen() { throw new Error("Management has no presentation completion channel"); },
      async cancelScreen() { throw new Error("Management has no presentation cancellation channel"); },
    };
  }, { theme: presentation.theme, view });
  const page = await context.newPage(), errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(origin); await page.locator("#app h1, #app h2, #app [role=alert]").first().waitFor();
  assert.equal(await page.locator("#app [role=alert]").count(), 0, (await page.locator("#app [role=alert]").allTextContents()).join("\n"));
  await page.getByRole("heading", { name: view === "memory" ? "角色记忆库" : "角色卡", exact: true }).waitFor();
  const card = page.locator('[data-memory-card="default"]');
  async function open() {
    await card.getByRole("button", { name: "记忆设置", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "记忆设置", exact: true });
    await dialog.locator("input[data-field=interval], .dialog-error:not([hidden])").first().waitFor();
    assert.equal(await dialog.locator(".dialog-error:not([hidden])").count(), 0, await dialog.locator(".dialog-error").innerText());
    await dialog.getByLabel("检查间隔（分钟）", { exact: true }).waitFor();
    return dialog;
  }
  return { harness, runtime, context, page, requests, errors, card, open };
}

/** Reads authoritative stored bytes independently of the open browser and retained service. */
async function storedOwner(harness) {
  const state = await harness.readState(), owners = state.owners.filter(owner => owner.ownerKey === CONTROLLED_OWNER);
  assert.equal(owners.length, 1); return owners[0];
}

test("real main IPC: memory settings load/save atomically, retain scheduler state and survive reopening/runtime restart", async t => {
  const ui = await controlsPage(t);
  const shared = await ui.runtime.web({ action: "saveStore", store: { id: "", name: "Unaffected shared owner" }, create: true });
  const sharedKey = "shared:" + shared.stores[0].id;
  const sharedBefore = plain(await ui.runtime.web({ action: "readMemorySettings", ownerKey: sharedKey }));
  const settings = await ui.runtime.web({ action: "readMemorySettings", ownerKey: CONTROLLED_OWNER });
  await ui.runtime.web({ action: "writeMemorySettings", ownerKey: CONTROLLED_OWNER, settings: { ...settings, autoSaveIntervalMinutes: 9, memoryExtractionCustomRules: "Persisted rules", nextAutoSaveRunAtMs: 2000000000000 } });
  const config = await ui.runtime.web({ action: "readMemorySearchConfig", ownerKey: CONTROLLED_OWNER });
  await ui.runtime.web({ action: "writeMemorySearchConfig", ownerKey: CONTROLLED_OWNER, config: { ...config, scoreMode: "SEMANTIC_FIRST", keywordWeight: 3.5 } });
  const dialog = await ui.open();
  assert.equal(await dialog.getByLabel("检查间隔（分钟）").inputValue(), "9");
  assert.equal(await dialog.getByLabel("记忆提取自定义规则").inputValue(), "Persisted rules");
  assert.equal(await dialog.getByRole("button", { name: "保存", exact: true }).isEnabled(), true);
  assert.doesNotMatch(await dialog.innerText(), /未接入|尚未接线|假任务/);
  await dialog.getByLabel("检查间隔（分钟）").fill("13");
  await dialog.getByLabel("记忆提取自定义规则").fill("Only real owner-scoped extraction rules");
  await dialog.getByRole("switch", { name: "自动更新 USER.md", exact: true }).uncheck();
  await dialog.getByRole("switch", { name: "锁定 USER.md", exact: true }).check();
  await dialog.getByRole("tab", { name: "检索", exact: true }).click();
  assert.equal(await dialog.getByLabel("关键词权重").inputValue(), "3.5");
  assert.equal(await dialog.getByText("评分模式", { exact: true }).count(), 0);
  await dialog.getByLabel("关键词权重").fill("6.2");
  await dialog.getByLabel("标签权重").fill("1.1");
  // A scheduler update while the editor is open must not be overwritten by its old full-record draft.
  await ui.runtime.web({ action: "writeMemorySettings", ownerKey: CONTROLLED_OWNER, settings: { ...settings, nextAutoSaveRunAtMs: 2100000000000 } });
  ui.harness.disk.clearCalls();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await ui.page.getByText("记忆设置已保存", { exact: true }).waitFor();
  assert.equal(ui.harness.disk.calls.filter(call => call.method === "storage.commit").length, 1);
  const owner = await storedOwner(ui.harness);
  assert.equal(owner.settings.autoSaveIntervalMinutes, 13);
  assert.equal(owner.settings.memoryExtractionCustomRules, "Only real owner-scoped extraction rules");
  assert.equal(owner.settings.profileAutoUpdateEnabled, false); assert.equal(owner.settings.profileAutoUpdateLocked, true);
  assert.equal(owner.settings.nextAutoSaveRunAtMs, 2100000000000);
  assert.equal(owner.searchConfig.keywordWeight, 6.2); assert.equal(owner.searchConfig.tagWeight, 1.1);
  assert.equal(owner.searchConfig.scoreMode, "SEMANTIC_FIRST");
  assert.deepEqual(plain(await ui.runtime.web({ action: "readMemorySettings", ownerKey: sharedKey })), sharedBefore);
  await dialog.getByRole("button", { name: "关闭", exact: true }).last().click();
  const reopened = await ui.open();
  assert.equal(await reopened.getByLabel("检查间隔（分钟）").inputValue(), "13");
  await reopened.getByRole("button", { name: "关闭", exact: true }).last().click();
  // Reopen the page to read persisted owners; management no longer has a refresh header.
  await ui.page.reload();
  await ui.page.getByRole("heading", { name: "角色记忆库", exact: true }).waitFor();
  await ui.page.locator(`[data-action="owner-settings"][data-owner="${sharedKey}"]`).click();
  const sharedDialog = ui.page.getByRole("dialog", { name: "记忆设置", exact: true });
  await sharedDialog.getByLabel("检查间隔（分钟）").waitFor();
  assert.equal(await sharedDialog.getByLabel("检查间隔（分钟）").inputValue(), String(sharedBefore.autoSaveIntervalMinutes));
  await sharedDialog.getByLabel("检查间隔（分钟）").fill("14");
  await sharedDialog.getByRole("button", { name: "保存", exact: true }).click();
  assert.equal((await ui.runtime.web({ action: "readMemorySettings", ownerKey: sharedKey })).autoSaveIntervalMinutes, 14);
  assert.deepEqual((await storedOwner(ui.harness)).settings, owner.settings);
  const restarted = ui.harness.openRuntime(); await restarted.initialize();
  assert.deepEqual(plain(await restarted.web({ action: "readMemorySettings", ownerKey: CONTROLLED_OWNER })), owner.settings);
  assert.deepEqual(plain(await restarted.web({ action: "readMemorySearchConfig", ownerKey: CONTROLLED_OWNER })), owner.searchConfig);
  assert.deepEqual(ui.errors, []);
});

test("real main IPC: read failure cannot enable blank saves, retry loads actual settings and invalid combined saves publish nothing", async t => {
  const ui = await controlsPage(t);
  await ui.page.evaluate(() => { window.rejectMemoryAction = "readMemorySettings"; });
  const before = await ui.harness.readStateText();
  await ui.card.getByRole("button", { name: "记忆设置", exact: true }).click();
  const dialog = ui.page.getByRole("dialog", { name: "记忆设置", exact: true });
  await dialog.getByRole("button", { name: "重试", exact: true }).waitFor();
  assert.equal(await dialog.getByRole("button", { name: "保存", exact: true }).isDisabled(), true);
  assert.match(await dialog.getByRole("alert").innerText(), /Explicit test transport rejection/);
  assert.equal(await ui.harness.readStateText(), before);
  await ui.page.evaluate(() => { window.rejectMemoryAction = null; });
  await dialog.getByRole("button", { name: "重试", exact: true }).click();
  assert.equal(await dialog.getByLabel("检查间隔（分钟）").inputValue(), "5");
  const owner = await storedOwner(ui.harness), bytes = await ui.harness.readStateText();
  await assert.rejects(ui.runtime.web({ action: "writeMemoryOwnerSettings", ownerKey: CONTROLLED_OWNER, settings: { ...owner.settings, autoSaveIntervalMinutes: 12 }, config: { ...owner.searchConfig, keywordWeight: -1 } }));
  assert.equal(await ui.harness.readStateText(), bytes);
  await dialog.getByLabel("检查间隔（分钟）").fill("12");
  await ui.page.evaluate(() => { window.rejectMemoryAction = "writeMemoryOwnerSettings"; });
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  assert.match(await dialog.getByRole("alert").innerText(), /Explicit test transport rejection/);
  assert.equal(await ui.harness.readStateText(), bytes);
  assert.equal(await dialog.getByLabel("检查间隔（分钟）").inputValue(), "12");
  await dialog.getByRole("button", { name: "关闭", exact: true }).last().click();
  assert.equal(await ui.harness.readStateText(), bytes);
  assert.deepEqual(ui.errors, []);
});

test("real main IPC and registered interval: history rebuild executes actual extraction, polls without replacing drafts, cancels and completes durably", async t => {
  const ui = await controlsPage(t, 1100), dialog = await ui.open();
  await dialog.getByRole("tab", { name: "历史重建", exact: true }).click();
  assert.equal(await dialog.getByRole("button", { name: "开始重建", exact: true }).isDisabled(), true);
  await dialog.getByLabel("窗口消息数").fill("8");
  await dialog.locator(`[data-memory-chat="${CONTROLLED_CHAT_ID}"]`).check();
  await dialog.getByLabel("开始时间", { exact: true }).fill("2025-10-09");
  await dialog.getByLabel("结束时间", { exact: true }).fill("2025-10-09");
  await dialog.getByRole("button", { name: "开始重建", exact: true }).click();
  let owner = await storedOwner(ui.harness);
  assert.equal(owner.rebuildProgress.status, "preparing"); assert.ok(owner.rebuildTask.windows.length > 1);
  const submitted = ui.requests.find(request => request.action === "startMemoryRebuild");
  assert.equal(submitted.rebuild.fromInclusive, Date.parse("2025-10-09T00:00:00+08:00"));
  assert.equal(submitted.rebuild.toInclusive, Date.parse("2025-10-09T23:59:59.999+08:00"));
  assert.deepEqual(submitted.rebuild.chatIds, [CONTROLLED_CHAT_ID]);
  ui.harness.queueAnalysis({ main: null, new: [{ title: "Actual history extraction", content: "Committed by the production job", tags: [], folder_path: "History" }], update: [], merge: [], links: [], profile_markdown: "# Real extracted profile\n" });
  await ui.runtime.interval(); ui.harness.assertOutputsConsumed();
  const field = dialog.getByLabel("窗口消息数"); await field.fill("12"); await field.focus();
  await field.evaluate(element => { window.memoryPollingField = element; });
  await ui.page.waitForFunction(() => document.querySelector("[data-memory-rebuild]")?.textContent.includes("窗口 1/"), null, { timeout: 10000 });
  assert.equal(await field.inputValue(), "12");
  assert.equal(await field.evaluate(element => element === window.memoryPollingField && element === document.activeElement), true);
  owner = await storedOwner(ui.harness); assert.equal(owner.memories[0].title, "Actual history extraction");
  assert.equal(owner.rebuildProgress.completedWindows, 1);
  assert.equal((await ui.runtime.web({ action: "readUser", ownerKey: CONTROLLED_OWNER })).content, "# Real extracted profile\n");
  await dialog.getByRole("button", { name: "取消重建", exact: true }).click();
  assert.equal((await storedOwner(ui.harness)).rebuildProgress.status, "cancelled");
  const count = ui.harness.modelCalls.length; await ui.runtime.interval(); assert.equal(ui.harness.modelCalls.length, count);
  // A new whole-history plan uses one genuine window; no browser timer is allowed to claim completion.
  await dialog.getByLabel("窗口消息数").fill("32");
  await dialog.getByRole("button", { name: "开始重建", exact: true }).click();
  ui.harness.queueAnalysis({}); await ui.runtime.interval(); ui.harness.assertOutputsConsumed();
  await dialog.getByRole("button", { name: "刷新状态", exact: true }).click();
  assert.equal((await storedOwner(ui.harness)).rebuildProgress.status, "completed");
  assert.match(await dialog.locator("[data-memory-rebuild]").innerText(), /已完成/);
  await ui.page.screenshot({ path: output + "memory-controls-real-rebuild.png" });
  await dialog.getByRole("button", { name: "关闭", exact: true }).last().click();
  const pollCount = ui.requests.filter(request => request.action === "readMemoryRebuildProgress").length;
  await new Promise(resolve => setTimeout(resolve, 3300));
  assert.equal(ui.requests.filter(request => request.action === "readMemoryRebuildProgress").length, pollCount);
  assert.deepEqual(ui.errors, []);
});

test("real main IPC and local HTTP provider: saved embedding configuration sends actual requests, persists vectors and reports failures without replacing the cache", async t => {
  const ui = await controlsPage(t), received = [];
  let fail = false;
  const provider = http.createServer(async (request, response) => {
    let body = ""; for await (const chunk of request) body += chunk;
    received.push({ path: request.url, method: request.method, authorization: request.headers.authorization, body: JSON.parse(body) });
    response.writeHead(fail ? 503 : 200, { "Content-Type": "application/json" });
    response.end(fail ? JSON.stringify({ error: "Explicit local provider failure" }) : JSON.stringify({ data: JSON.parse(body).input.map((input, index) => ({ index, embedding: [1, 0.5, 0.25] })) }));
  });
  await new Promise(resolve => provider.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise(resolve => provider.close(resolve)));
  const endpoint = `http://127.0.0.1:${provider.address().port}/embeddings`;
  ui.harness.tools.Net = { async http(options) {
    const response = await fetch(options.url, { method: options.method, headers: options.headers, body: options.body });
    return { statusCode: response.status, content: await response.text() };
  } };
  await ui.runtime.api("memory.create", { ownerKey: CONTROLLED_OWNER, values: { title: "HTTP-backed memory", content: "Real embedding input" } });
  const dialog = await ui.open(); await dialog.getByRole("tab", { name: "检索", exact: true }).click();
  await dialog.getByRole("switch", { name: "云端 Embedding", exact: true }).check();
  await dialog.getByLabel("Embedding 完整请求地址").fill(endpoint);
  await dialog.getByLabel("Embedding 模型", { exact: true }).fill("actual-local-model");
  await dialog.getByLabel("API Key", { exact: true }).fill("explicit-provider-key");
  assert.equal(await dialog.getByLabel("API Key", { exact: true }).getAttribute("type"), "password");
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await dialog.getByRole("button", { name: "重建 Embedding", exact: true }).click();
  assert.equal(received.length, 1); assert.equal(received[0].path, "/embeddings"); assert.equal(received[0].method, "POST");
  assert.equal(received[0].authorization, "Bearer explicit-provider-key");
  assert.deepEqual(received[0].body, { input: ["Real embedding input"], model: "actual-local-model", encoding_format: "float" });
  const owner = await storedOwner(ui.harness);
  assert.equal(owner.settings.cloudEmbeddingEndpoint, endpoint); assert.equal(owner.embeddings.length, 0);
  /** Reads persisted source envelopes independently of the production search implementation. */
  function sourceVectors() { return ui.harness.disk.storage.inspect(sql => sql.prepare("SELECT value FROM records WHERE collection='memories' ORDER BY key").all().map(row => JSON.parse(row.value).embedding)); }
  assert.equal(sourceVectors().length, 1); assert.equal(sourceVectors()[0].text, "Real embedding input");
  await dialog.getByLabel("向量权重", { exact: true }).fill("1");
  await dialog.getByLabel("检索查询", { exact: true }).fill("Real provider query");
  await dialog.getByRole("button", { name: "使用当前权重检索", exact: true }).click();
  assert.equal(received.length, 2);
  assert.deepEqual(received[1].body, { input: ["Real provider query"], model: "actual-local-model", encoding_format: "float" });
  const debug = plain(await ui.runtime.web({ action: "searchMemoryDebug", ownerKey: CONTROLLED_OWNER, query: "Real provider query", config: { scoreMode: "BALANCED", keywordWeight: 0, tagWeight: 0, vectorWeight: 1, edgeWeight: 0 }, relevanceThreshold: 0 }));
  assert.ok(Math.abs(debug.candidates[0].vectorScore - 1) < 1e-12);
  assert.equal(received.length, 3, "Repeated retrieval generates one query vector and does not persist it"); assert.ok(debug.candidates[0].passedThreshold);
  assert.equal((await ui.runtime.web({ action: "readMemorySearchConfig", ownerKey: CONTROLLED_OWNER })).vectorWeight, 0);
  const cached = sourceVectors();
  fail = true; await dialog.getByRole("button", { name: "重建 Embedding", exact: true }).click();
  assert.match(await dialog.getByRole("alert").innerText(), /503.*Explicit local provider failure/);
  assert.deepEqual(sourceVectors(), cached);
  await dialog.getByLabel("检索查询", { exact: true }).fill("Actual failed provider query");
  await dialog.getByRole("button", { name: "使用当前权重检索", exact: true }).click();
  assert.match(await dialog.getByRole("alert").innerText(), /503.*Explicit local provider failure/);
  assert.deepEqual(sourceVectors(), cached);
  assert.deepEqual(ui.errors, []);
});

test("real persisted-message and interval hooks: saved extraction rules control queued memory and USER.md updates, and the modal reads real queue counts", async t => {
  const ui = await controlsPage(t);
  await ui.runtime.enqueueReplies();
  const dialog = await ui.open();
  assert.match(await dialog.locator("[data-memory-queue]").innerText(), /待处理 5 条 \/ 1 个聊天/);
  await dialog.getByLabel("检查间隔（分钟）").fill("1");
  await dialog.getByLabel("记忆提取自定义规则").fill("CUSTOM_REAL_EXTRACTION_RULE");
  await dialog.getByRole("switch", { name: "自动更新 USER.md", exact: true }).check();
  await dialog.getByRole("switch", { name: "锁定 USER.md", exact: true }).uncheck();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  await ui.runtime.makeDue();
  ui.harness.queueAnalysis({ main: { title: "Actual queued memory", content: "Stored by the real automatic extraction", tags: [], folder_path: "Queue" }, new: [], update: [], merge: [], links: [], profile_markdown: "# Queued user profile\n" });
  await ui.runtime.interval(); ui.harness.assertOutputsConsumed();
  assert.equal(ui.harness.modelCalls.length, 1);
  assert.match(JSON.stringify(ui.harness.modelCalls[0].turns), /CUSTOM_REAL_EXTRACTION_RULE/);
  const stored = await storedOwner(ui.harness);
  assert.equal(stored.candidates.length, 0); assert.equal(stored.memories[0].title, "Actual queued memory");
  assert.equal((await ui.runtime.web({ action: "readUser", ownerKey: CONTROLLED_OWNER })).content, "# Queued user profile\n");
  await ui.page.waitForFunction(() => document.querySelector("[data-memory-queue]")?.textContent.includes("待处理 0 条"), null, { timeout: 10000 });
  await ui.page.screenshot({ path: output + "memory-controls-real-auto-extraction.png" });
  const calls = ui.harness.modelCalls.length;
  await ui.runtime.enqueueReplies(); await ui.runtime.makeDue();
  await dialog.getByRole("switch", { name: "锁定 USER.md", exact: true }).check();
  await dialog.getByRole("button", { name: "保存", exact: true }).click();
  ui.harness.queueAnalysis({ main: null, new: [], update: [], merge: [], links: [], profile_markdown: "Must not overwrite locked profile" });
  await ui.runtime.interval(); ui.harness.assertOutputsConsumed();
  assert.equal(ui.harness.modelCalls.length, calls + 1);
  assert.equal((await ui.runtime.web({ action: "readUser", ownerKey: CONTROLLED_OWNER })).content, "# Queued user profile\n");
  assert.equal((await storedOwner(ui.harness)).candidates.filter(candidate => candidate.status === "failed").length, 5);
  await dialog.getByRole("button", { name: "关闭", exact: true }).last().click();
  const failedDialog = await ui.open();
  assert.match(await failedDialog.locator("[data-memory-queue] .error-banner").innerText(), /profile update forbidden by the owner settings/);
  assert.deepEqual(ui.errors, []);
});

/** Starts real records that make lexical, tag and graph channels independently observable. */
async function searchableMemories(runtime) {
  const aResult = await runtime.api("memory.create", { ownerKey: CONTROLLED_OWNER, values: { title: "Rust ownership", content: "Rust ownership lifetimes", tags: ["systems"] } });
  const bResult = await runtime.api("memory.create", { ownerKey: CONTROLLED_OWNER, values: { title: "Graph neighbor", content: "Other material", tags: ["rust"] } });
  const a = aResult.item, b = bResult.item;
  await runtime.web({ action: "createLink", ownerKey: CONTROLLED_OWNER, sourceTitle: a.title, targetTitle: b.title, linkType: "depends", weight: 0.8, description: "  canonical description\n" });
  return { a, b };
}

test("real search-debug UI: draft weights change genuine ranking without saving; scores match normal search and errors remain visible", async t => {
  for (const width of [390, 1100]) await t.test(`${width}px`, async t => {
    const ui = await controlsPage(t, width), { a, b } = await searchableMemories(ui.runtime);
    const saved = plain(await ui.runtime.web({ action: "readMemorySearchConfig", ownerKey: CONTROLLED_OWNER }));
    const dialog = await ui.open(); await dialog.getByRole("tab", { name: "检索", exact: true }).click();
    await dialog.getByLabel("检索查询", { exact: true }).fill("rust");
    await dialog.getByRole("button", { name: "使用当前权重检索", exact: true }).click();
    const rows = dialog.locator("[data-search-results] tbody tr");
    assert.equal(await rows.count(), 2); assert.match(await rows.first().innerText(), /Rust ownership/);
    const debug = plain(await ui.runtime.web({ action: "searchMemoryDebug", ownerKey: CONTROLLED_OWNER, query: "rust", config: saved, relevanceThreshold: 0 }));
    const normal = plain(await ui.runtime.web({ action: "searchMemory", ownerKey: CONTROLLED_OWNER, query: "rust" }));
    assert.deepEqual(debug.candidates.filter(hit => hit.passedThreshold).map(hit => hit.memory), normal);
    for (const hit of debug.candidates) assert.ok(Math.abs(hit.score - hit.keywordScore - hit.tagScore - hit.vectorScore - hit.edgeScore) < 1e-12);
    assert.ok(debug.candidates.find(hit => hit.memory.id === b.id).edgeScore > 0);
    await dialog.getByLabel("关键词权重", { exact: true }).fill("0");
    await dialog.getByLabel("标签权重", { exact: true }).fill("4");
    await dialog.getByLabel("关系权重", { exact: true }).fill("0");
    await dialog.getByRole("button", { name: "使用当前权重检索", exact: true }).click();
    assert.match(await rows.first().innerText(), /Graph neighbor/);
    assert.match(await rows.nth(1).innerText(), /未过阈值/);
    assert.deepEqual(plain(await ui.runtime.web({ action: "readMemorySearchConfig", ownerKey: CONTROLLED_OWNER })), saved);
    const operation = ui.requests.filter(request => request.action === "searchMemoryDebug").at(-1);
    assert.equal(operation.config.tagWeight, 4); assert.equal(operation.ownerKey, CONTROLLED_OWNER);
    assert.equal(debug.candidates[0].memory.id, a.id);
    await ui.page.screenshot({ path: output + `memory-real-search-${width}.png` });
    await dialog.getByLabel("检索查询", { exact: true }).fill("!!!");
    await dialog.getByRole("button", { name: "使用当前权重检索", exact: true }).click();
    assert.match(await dialog.getByRole("alert").innerText(), /no searchable tokens/);
    assert.equal(await dialog.locator("[data-search-results]").count(), 0);
    assert.deepEqual(ui.errors, []);
  });
});

test("global switches genuinely persist, gate new USER.md profiles and automatic jobs, while explicit rebuild remains available", async t => {
  const ui = await controlsPage(t);
  await ui.runtime.web({ action: "writeUser", ownerKey: CONTROLLED_OWNER, content: "# Private actual USER.md" });
  const resolve = () => ui.runtime.api("chat.configuration.resolve", { purpose: "execution", chatId: CONTROLLED_CHAT_ID, chatExtension: { version: 1, selection: "card:default" }, participantId: null, promptFunctionType: "CHAT", defaultModelBinding: { providerId: "dashscope", modelId: "qwen-plus" }, defaultTtsConfigId: "tts-one" });
  const initialExecution = plain(await resolve());
  assert.equal(initialExecution.profile.userPreferencesText, "# Private actual USER.md");
  const profile = ui.page.getByRole("switch", { name: "向模型提供用户资料", exact: true });
  await profile.click(); await ui.page.waitForFunction(() => !document.querySelector('[data-key="userDescriptionEnabled"]').checked);
  assert.equal((await resolve()).profile.userPreferencesText, "");
  const current = plain(await resolve());
  assert.deepEqual(current.profile, { ...initialExecution.profile, userPreferencesText: "" });
  assert.ok(current.participants.every(profile => profile.userPreferencesText === ""));
  await assert.rejects(() => ui.runtime.api("chat.configuration.resolve", { purpose: "execution", chatId: CONTROLLED_CHAT_ID, chatExtension: null, executionData: initialExecution.executionData, participantId: initialExecution.profile.id, promptFunctionType: "CHAT", defaultModelBinding: initialExecution.profile.modelBinding, defaultTtsConfigId: initialExecution.profile.ttsConfigId }), /not a supported field/);
  assert.equal((await ui.runtime.web({ action: "readUser", ownerKey: CONTROLLED_OWNER })).content, "# Private actual USER.md");
  const automatic = ui.page.getByRole("switch", { name: "自动更新记忆库", exact: true });
  await ui.runtime.enqueueReplies(); await ui.runtime.makeDue();
  await automatic.click(); await ui.page.waitForFunction(() => !document.querySelector('[data-key="autoUpdateEnabled"]').checked);
  const before = plain(await storedOwner(ui.harness));
  await ui.runtime.enqueueReplies(); await ui.runtime.interval();
  assert.deepEqual((await storedOwner(ui.harness)).candidates, before.candidates);
  assert.equal(ui.harness.modelCalls.length, 0);
  assert.deepEqual(plain((await ui.harness.openRuntime().web({ action: "snapshot" })).memoryPreferences), { userDescriptionEnabled: false, autoUpdateEnabled: false });
  await ui.runtime.startRebuild(); ui.harness.queueAnalysis({}); await ui.runtime.interval();
  assert.equal(ui.harness.modelCalls.length, 1); assert.equal((await storedOwner(ui.harness)).rebuildProgress.completedWindows, 1);
  await ui.runtime.web({ action: "cancelMemoryRebuild", ownerKey: CONTROLLED_OWNER });
  await ui.page.evaluate(() => { window.rejectMemoryAction = "writeGlobalMemoryPreference"; });
  await profile.click();
  assert.match(await ui.page.locator("#app [role=alert]").innerText(), /Explicit test transport rejection/);
  assert.equal(await profile.isChecked(), false);
  await ui.page.evaluate(() => { window.rejectMemoryAction = null; });
  await profile.click(); await ui.page.waitForFunction(() => document.querySelector('[data-key="userDescriptionEnabled"]').checked);
  assert.equal((await resolve()).profile.userPreferencesText, "# Private actual USER.md");
  assert.equal(await ui.page.locator("#app [role=alert]").count(), 0);
  await automatic.click(); await ui.page.waitForFunction(() => document.querySelector('[data-key="autoUpdateEnabled"]').checked);
  ui.harness.queueAnalysis({}); await ui.runtime.interval();
  assert.equal(ui.harness.modelCalls.length, 2); assert.equal((await storedOwner(ui.harness)).candidates.filter(candidate => candidate.status === "pending").length, 0);
  assert.deepEqual(ui.errors, []);
});

test("global preferences reject malformed IPC and failed transactions without publishing guessed flags", async t => {
  const ui = await controlsPage(t), before = await ui.harness.readStateText();
  for (const operation of [
    { action: "writeGlobalMemoryPreference", key: "unknown", value: true },
    { action: "writeGlobalMemoryPreference", key: "autoUpdateEnabled", value: "true" },
  ]) await assert.rejects(() => ui.runtime.web(operation), /Unknown global|必须是布尔值/);
  assert.equal(await ui.harness.readStateText(), before);
  ui.harness.disk.failNext("storage.preferences.edit", new Error("Actual global preference transaction failure"));
  await assert.rejects(() => ui.runtime.web({ action: "writeGlobalMemoryPreference", key: "autoUpdateEnabled", value: false }), /Actual global preference transaction failure/);
  assert.equal(await ui.harness.readStateText(), before);
  assert.equal((await ui.harness.openRuntime().web({ action: "snapshot" })).memoryPreferences.autoUpdateEnabled, true);
});

test("graph canonical relationship editing and full memory backup import/export run through actual main IPC and disk IO", async t => {
  const ui = await controlsPage(t); await searchableMemories(ui.runtime);
  await ui.runtime.web({ action: "writeUser", ownerKey: CONTROLLED_OWNER, content: "# Full backup USER.md\n  exact whitespace\n" });
  const source = await storedOwner(ui.harness), link = source.links[0];
  await ui.page.evaluate(() => { window.omitGraphDescription = true; });
  await ui.card.getByRole("button", { name: "记忆图谱", exact: true }).click();
  const graph = ui.page.getByRole("dialog").filter({ has: ui.page.locator("[data-graph-surface]") });
  const point = await graph.locator(".graph-edge").first().evaluate(line => {
    const matrix = line.getScreenCTM();
    const point = new DOMPoint((line.x1.baseVal.value + line.x2.baseVal.value) / 2, (line.y1.baseVal.value + line.y2.baseVal.value) / 2).matrixTransform(matrix);
    return { x: point.x, y: point.y };
  });
  await ui.page.mouse.click(point.x, point.y);
  await graph.getByRole("button", { name: "编辑关系", exact: true }).click();
  const editor = ui.page.getByRole("dialog", { name: "编辑记忆关系", exact: true });
  assert.equal(await editor.getByLabel("描述", { exact: true }).inputValue(), link.description);
  assert.ok(ui.requests.some(request => request.action === "readMemoryLink" && request.linkId === link.id));
  await editor.getByLabel("关系类型", { exact: true }).fill("canonical-edited");
  await editor.getByRole("button", { name: "保存", exact: true }).click();
  assert.equal((await storedOwner(ui.harness)).links[0].type_, "canonical-edited");
  assert.equal((await storedOwner(ui.harness)).links[0].description, link.description);
  await graph.getByRole("button", { name: "更多操作", exact: true }).click();
  await graph.getByRole("button", { name: "导出记忆 JSON", exact: true }).click();
  const exported = ui.page.getByRole("dialog", { name: "导出 JSON", exact: true });
  const content = await exported.getByLabel("JSON 内容").inputValue(), backup = JSON.parse(content);
  assert.equal(backup.space.memories.length, 2); assert.equal(backup.space.links[0].type_, "canonical-edited");
  assert.equal(backup.userMarkdown, "# Full backup USER.md\n  exact whitespace\n");
  const path = ui.harness.disk.directory + "/actual-memory-backup.json";
  assert.equal(await exported.locator("input").count(), 0);
  await ui.page.evaluate(path => { window.exportDestination = path; window.cancelExport = true; }, path);
  await exported.getByRole("button", { name: "保存文件", exact: true }).click();
  assert.equal(await exported.count(), 1);
  assert.equal(await exported.locator(".dialog-error:not([hidden])").count(), 0);
  await ui.page.evaluate(() => { window.cancelExport = false; });
  await ui.page.evaluate(() => { window.rejectExport = true; });
  await exported.getByRole("button", { name: "保存文件", exact: true }).click();
  assert.match(await exported.getByRole("alert").innerText(), /Host save denied/);
  assert.equal(await exported.count(), 1);
  await ui.page.evaluate(() => { window.rejectExport = false; });
  await exported.getByRole("button", { name: "保存文件", exact: true }).click();
  assert.equal((await ui.harness.disk.readBytes(path)).toString("utf8"), content);
  await graph.getByRole("button", { name: "更多操作", exact: true }).click();
  await graph.getByRole("button", { name: "导入记忆 JSON", exact: true }).click();
  const imported = ui.page.getByRole("dialog", { name: "导入记忆 JSON", exact: true });
  await imported.getByLabel("相同 UUID 记忆处理", { exact: true }).selectOption("UPDATE");
  backup.space.memories[0].content = "Restored through actual UI import";
  backup.userMarkdown = "# Restored profile";
  assert.equal(await imported.locator("textarea").count(), 0);
  await ui.page.evaluate(path => { window.importSource = path; }, path);
  await ui.harness.tools.Files.write(path, "{", false);
  await imported.getByRole("button", { name: "选择 JSON 文件并导入", exact: true }).click();
  await imported.getByRole("alert").waitFor();
  assert.equal(await imported.count(), 1);
  assert.notEqual((await storedOwner(ui.harness)).memories[0].content, "Restored through actual UI import");
  await ui.harness.tools.Files.write(path, JSON.stringify(backup), false);
  await imported.getByRole("button", { name: "选择 JSON 文件并导入", exact: true }).click();
  await imported.waitFor({ state: "hidden" });
  assert.equal((await storedOwner(ui.harness)).memories[0].content, "Restored through actual UI import");
  assert.equal((await ui.runtime.web({ action: "readUser", ownerKey: CONTROLLED_OWNER })).content, "# Restored profile");
  assert.equal((await storedOwner(ui.harness)).links[0].description, link.description);
  const shared = await ui.runtime.web({ action: "saveStore", store: { id: "", name: "Isolated restore target" }, create: true });
  const ownerKey = "shared:" + shared.stores[0].id;
  await ui.runtime.web({ action: "importMemory", ownerKey, content, strategy: "SKIP" });
  assert.equal((await ui.runtime.web({ action: "listMemories", ownerKey })).length, 2);
  assert.equal((await ui.runtime.web({ action: "readUser", ownerKey })).content, JSON.parse(content).userMarkdown);
  assert.deepEqual(ui.errors, []);
});

test("Tavern import selector delegates untouched input to genuine codecs; native export retains the canonical saved data", async t => {
  const ui = await controlsPage(t, 390, "characters");
  await ui.page.getByRole("button", { name: "导入", exact: true }).click();
  const dialog = ui.page.getByRole("dialog", { name: "导入角色 JSON", exact: true });
  await dialog.getByLabel("角色格式", { exact: true }).selectOption("tavern");
  const content = JSON.stringify({ spec: "chara_card_v2", spec_version: "2.0", data: { name: "Real Tavern import", description: "Actual Tavern text", first_mes: "Actual first message", system_prompt: "Actual system", tags: ["actual-tavern-tag"] } });
  await dialog.getByLabel("粘贴 Tavern JSON", { exact: true }).fill(content);
  await dialog.getByRole("button", { name: "导入", exact: true }).click();
  await ui.page.getByRole("button", { name: "编辑 Real Tavern import", exact: true }).waitFor();
  const snapshot = plain(await ui.harness.openRuntime().web({ action: "snapshot" }));
  const card = snapshot.cards.find(card => card.name === "Real Tavern import");
  assert.equal(card.openingStatement, "Actual first message"); assert.match(card.marks, /原始标签：actual-tavern-tag/);
  assert.deepEqual(ui.requests.find(request => request.action === "importCharacter"), { action: "importCharacter", format: "tavern", content });
  // Export now belongs to the single native renderer; browser coverage remains the genuine import codec.
  // The native export UI and real persisted bytes are exercised in entity-native.test.mjs.
  const tavern = JSON.parse(await ui.runtime.web({ action: "exportCharacter", id: card.id, format: "tavern" }));
  assert.equal(tavern.spec, "chara_card_v2"); assert.equal(tavern.data.name, card.name); assert.deepEqual(tavern.data.tags, []); assert.match(tavern.data.creator_notes, /actual-tavern-tag/);
  assert.deepEqual(ui.errors, []);
});

/** Rejects absent or malformed flags in complete stored JSON without changing user data. */
test("runtime rejects missing or corrupt memory preferences without writing defaults", async t => {
  const ui = await controlsPage(t);
  await ui.runtime.web({ action: "writeGlobalMemoryPreference", key: "autoUpdateEnabled", value: false });
  assert.equal((await ui.harness.openRuntime().web({ action: "snapshot" })).memoryPreferences.autoUpdateEnabled, false);
  ui.harness.disk.storage.inspectPreferences("meta", values => { delete values.state.memoryPreferences; });
  await assert.rejects(ui.runtime.web({ action: "snapshot" }), /global memory preferences/);
  assert.equal(ui.harness.disk.storage.inspectPreferences("meta", values => values.state.memoryPreferences), undefined);
  ui.harness.disk.storage.inspectPreferences("meta", values => { values.state.memoryPreferences = { userDescriptionEnabled: true, autoUpdateEnabled: "false" }; });
  await assert.rejects(ui.runtime.web({ action: "snapshot" }), /auto update enabled/);
  assert.equal(ui.harness.disk.storage.inspectPreferences("meta", values => values.state.memoryPreferences.autoUpdateEnabled), "false");
});

/** Keeps durable source failures visible while the production scheduler extracts the remaining valid candidates. */
test("real queue failures stay visible after partial extraction and reopening the settings dialog", async t => {
  const ui = await controlsPage(t), sources = await ui.runtime.enqueueReplies();
  ui.harness.deleteMessage(sources[0].timestamp); await ui.runtime.makeDue();
  ui.harness.queueAnalysis({}); await ui.runtime.interval(); ui.harness.assertOutputsConsumed();
  const dialog = await ui.open(), queue = dialog.locator("[data-memory-queue]");
  assert.match(await queue.innerText(), /待处理 0 条.*处理中 0.*失败 1/);
  assert.match(await queue.locator(".error-banner").innerText(), /提取失败：源消息已删除或所选版本已变化，已跳过该候选/);
  assert.equal(ui.harness.modelCalls.length, 1);
  await dialog.getByRole("button", { name: "关闭", exact: true }).last().click();
  const reopened = await ui.open();
  assert.match(await reopened.locator("[data-memory-queue] .error-banner").innerText(), /源消息已删除或所选版本已变化/);
  assert.deepEqual(ui.errors, []);
});

/** Distinguishes a completed rebuild with skipped windows from an entirely successful rebuild. */
test("real rebuild reports partial failure after continuing through the remaining windows", async t => {
  const ui = await controlsPage(t); await ui.runtime.startRebuild();
  const planned = await storedOwner(ui.harness), failure = new Error("Controlled visible skipped-window failure");
  ui.harness.queueExecutionError(failure); await ui.runtime.interval();
  for (let index = 1; index < planned.rebuildProgress.totalWindows; index += 1) {
    ui.harness.queueAnalysis({}); await ui.runtime.interval();
  }
  ui.harness.assertOutputsConsumed();
  const dialog = await ui.open(); await dialog.getByRole("tab", { name: "历史重建", exact: true }).click();
  const progress = dialog.locator("[data-memory-rebuild]");
  assert.match(await progress.innerText(), /已完成（部分失败）/);
  assert.match(await progress.innerText(), /失败 1/);
  assert.equal(await progress.locator(".error-banner").innerText(), failure.message);
  assert.equal(await dialog.getByRole("button", { name: "取消重建", exact: true }).isDisabled(), true);
  await dialog.locator('[data-memory-chat="' + CONTROLLED_CHAT_ID + '"]').check();
  assert.equal(await dialog.getByRole("button", { name: "开始重建", exact: true }).isEnabled(), true);
  assert.deepEqual(ui.errors, []);
});
