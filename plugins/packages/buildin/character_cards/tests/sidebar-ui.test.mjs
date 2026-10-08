import assert from "node:assert/strict";
import test from "node:test";
import { createRequire } from "node:module";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { fixture } from "./fixtures.mjs";
import { loadModule, plain } from "./runtime.mjs";

const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const { chromium } = require("playwright"), { build } = require("esbuild");
const plugin = fileURLToPath(new URL("../", import.meta.url));
let browser, script, css;

/** Supplies explicit projected UI-only generic chat summaries; these are not production records or host responses. */
function summaries() {
  return [
    { id: "ui-chat-one", title: "完整第一条对话", updatedAt: "2026-10-08T12:00:00Z", displayOrder: 0, workspaceId: "workspace-one", workspaceName: "工作区一", locked: false, pinned: true },
    { id: "ui-chat-two", title: "另一条锁定对话", updatedAt: "2026-10-08T12:00:01Z", displayOrder: 1, workspaceId: "workspace-one", workspaceName: "工作区一", locked: true, pinned: false },
  ];
}
/** Builds only the actual pure renderer for read-only DOM verification without writing installed artifacts or inventing a production bridge. */
test.before(async () => {
  const result = await build({ absWorkingDir: plugin, entryPoints: [fileURLToPath(new URL("../web/features/sidebar/controller.ts", import.meta.url))], tsconfig: fileURLToPath(new URL("../tsconfig.web.json", import.meta.url)), bundle: true, format: "iife", globalName: "PureSidebarUnderTest", platform: "browser", target: "es2020", write: false });
  assert.equal(result.outputFiles.length, 1); script = result.outputFiles[0].text;
  css = await readFile(new URL("../web/style.css", import.meta.url), "utf8") + await readFile(new URL("../web/features/sidebar/style.css", import.meta.url), "utf8");
  browser = await chromium.launch({ headless: true });
});
/** Closes the browser used exclusively by explicitly labelled pure renderer tests. */
test.after(async () => { if (browser !== undefined) await browser.close(); });

/** Opens the actual typed DOM renderer with explicitly labelled projected test inputs and an intent recorder, not a successful backend. */
async function purePage(t, view, data) {
  const context = await browser.newContext({ viewport: { width: 360, height: 720 } });
  t.after(
    /** Releases the isolated pure UI context. */
    async () => context.close(),
  );
  const page = await context.newPage();
  await page.setContent('<!doctype html><html><head><style>' + css + '</style></head><body><main id="app"></main></body></html>');
  await page.addScriptTag({ content: script });
  await page.evaluate(
    /** Mounts only the pure controller and records intent; this test does not install or simulate any production host bridge. */
    ({ view, data, chats }) => {
      window.pureSidebarIntents = [];
      const current = { input: { view }, chatSidebar: { chats, currentChatId: "ui-chat-one", activeStreamingChatIds: ["ui-chat-one"] } };
      const actions = {
        /** Records an exact pure UI intent without creating a chat, changing metadata or returning a successful CRUD result. */
        async dispatch(intent) { window.pureSidebarIntents.push(intent); },
      };
      window.pureSidebar = PureSidebarUnderTest.createSidebarController(document.getElementById("app"), current, actions);
      window.pureSidebar.setData(data);
    },
    { view, data, chats: summaries() },
  );
  return page;
}

/** Validates the exact neutral host-state shape and rejects old group or role fields rather than inspecting them. */
test("sidebar context parser requires explicit input and generic summary fields with no presentation inference", () => {
  const parser = loadModule("src/ui-sidebar.ts");
  const expected = { input: { view: "characters" }, chatSidebar: { chats: summaries(), currentChatId: "ui-chat-one", activeStreamingChatIds: ["ui-chat-one"] } };
  assert.deepEqual(plain(parser.parseCurrentSidebar(expected)), expected);
  for (const invalid of [null, {}, { input: { mode: "manage" }, chatSidebar: expected.chatSidebar }, { input: expected.input, chatSidebar: null }, { input: expected.input, chatSidebar: { ...expected.chatSidebar, chats: [{ ...summaries()[0], group: "old-native-group" }] } }]) assert.throws(
    /** Passes each exact malformed host boundary to the real parser. */
    () => parser.parseCurrentSidebar(invalid),
  );
  assert.deepEqual(plain(parser.parseChatActivateAction({ type: "toolpkg.chat.activate", chatId: "ui-chat-one" })), { type: "toolpkg.chat.activate", chatId: "ui-chat-one" });
  assert.throws(
    /** Rejects attaching opaque selection or owner fields to the generic host action. */
    () => parser.parseChatActivateAction({ type: "toolpkg.chat.activate", chatId: "ui-chat-one", selection: "card:foreign" }),
  );
});
/** Keeps every explicit namespace-derived selection state mandatory, including an explicitly unbound chat. */
test("character categories require explicit selection states and preserve actual empty role-group categories", () => {
  const projection = loadModule("src/ui-sidebar.ts"), directory = fixture().snapshot;
  const chats = summaries(), selections = [{ chatId: chats[0].id, selection: "card:travel" }, { chatId: chats[1].id, selection: null }];
  const sections = plain(projection.characterSidebarSections(directory, selections, chats));
  assert.equal(sections.find(
    /** Finds the actual persisted role group from the labelled directory input. */
    section => section.id === "group:group-one",
  ).chats.length, 0);
  assert.deepEqual(sections.find(
    /** Locates explicitly unbound state rather than assigning the active character. */
    section => section.id === "unbound",
  ).chats, [chats[1]]);
  assert.throws(
    /** Rejects a missing state instead of treating it as an implicitly unbound record. */
    () => projection.characterSidebarSections(directory, selections.slice(0, 1), chats), /no explicit namespace selection state/,
  );
});
/** Exercises actual SVG-originated DOM clicks and native generic status rendering in the independent character tab. */
test("pure character sidebar renders active pinned locked streaming chats and dispatches exact SVG click intent", async t => {
  const projection = loadModule("src/ui-sidebar.ts"), chats = summaries();
  const sections = plain(projection.characterSidebarSections(fixture().snapshot, [{ chatId: chats[0].id, selection: "card:travel" }, { chatId: chats[1].id, selection: "card:travel" }], chats));
  const page = await purePage(t, "characters", { view: "characters", sections });
  assert.equal(await page.getByRole("heading", { name: "角色分类", exact: true }).count(), 1);
  assert.equal(await page.getByRole("tab").count(), 0, "Plugin sidebar is not a set of manager-page tab buttons");
  const active = page.locator('.sidebar-chat-main[data-chat-id="ui-chat-one"]');
  assert.equal(await active.getAttribute("aria-current"), "true");
  assert.equal(await page.getByRole("status", { name: "正在生成" }).count(), 1);
  assert.equal(await page.locator('.sidebar-chat-row[data-chat-id="ui-chat-two"] .sidebar-chat-status svg').count(), 1);
  await active.locator("svg").first().click();
  assert.deepEqual(await page.evaluate(
    /** Reads only the explicit pure intent recorder, never a manufactured host activation result. */
    () => window.pureSidebarIntents,
  ), [{ type: "activate-chat", chatId: "ui-chat-one" }]);
  await page.getByLabel("搜索对话或分组", { exact: true }).fill("锁定");
  assert.equal(await page.locator('.sidebar-chat-main[data-chat-id="ui-chat-one"]').count(), 0);
  assert.equal(await page.locator('.sidebar-chat-main[data-chat-id="ui-chat-two"]').count(), 1);
});
/** Verifies scope-separated legal duplicate names and persisted-empty-group operations using explicit projected UI-only inputs. */
test("pure group sidebar retains duplicate names across scopes and supports empty-group create rename pin reorder intents", async t => {
  const scopes = [
    { id: "card:first", title: "角色一", groups: [{ id: "group-first", name: "相同名称", pinned: false, displayOrder: 0, chats: summaries() }], ungrouped: [] },
    { id: "card:second", title: "角色二", groups: [{ id: "group-second", name: "相同名称", pinned: true, displayOrder: 0, chats: [] }], ungrouped: [] },
  ];
  const page = await purePage(t, "groups", { view: "groups", scopes });
  assert.equal(await page.locator(".sidebar-group-header strong").filter({ hasText: "相同名称" }).count(), 2);
  const group = page.locator('.sidebar-conversation-group[data-group-id="group-second"]');
  assert.match(await group.innerText(), /暂无对话/);
  await page.locator('[data-scope-id="card:second"]').getByRole("button", { name: "新建分组", exact: true }).click();
  await group.getByLabel("分组操作", { exact: true }).click();
  await group.getByRole("button", { name: "重命名分组", exact: true }).click();
  await group.getByRole("button", { name: "取消置顶分组", exact: true }).click();
  await group.getByRole("button", { name: "下移分组", exact: true }).click();
  assert.deepEqual(await page.evaluate(
    /** Returns finite UI intent only; no group record was created or changed in this test. */
    () => window.pureSidebarIntents,
  ), [{ type: "create-group", scopeId: "card:second" }, { type: "rename-group", groupId: "group-second" }, { type: "pin-group", groupId: "group-second" }, { type: "reorder-group", groupId: "group-second", direction: "down" }]);
});
/** Matches the original explicit deletion prompt and displays partial failure without a hidden retry or a metadata-success claim. */
test("pure delete dialog confirms all member chats and retains exact inspectable partial outcomes", async t => {
  const page = await purePage(t, "groups", { view: "groups", scopes: [] });
  await page.evaluate(
    /** Supplies explicit local dialog projection solely to verify its real Material markup. */
    () => { window.pureSidebar.state.dialog = { type: "delete-group", groupId: "group-one", name: "原分组", memberCount: 2, progress: null }; window.pureSidebar.render(); },
  );
  const dialog = page.getByRole("dialog", { name: "删除分组", exact: true });
  assert.match(await dialog.innerText(), /确定要删除分组“原分组”及其中的 2 条对话吗？/);
  await page.evaluate(
    /** Displays labelled operation outcomes without invoking or mocking a production deletion bridge. */
    () => { window.pureSidebar.state.dialog.progress = { deletedChatIds: ["ui-chat-one"], failedChatIds: [{ chatId: "ui-chat-two", error: "Exact locked native deletion failure" }], metadataDeleted: false, metadataError: null }; window.pureSidebar.render(); },
  );
  assert.match(await dialog.innerText(), /Exact locked native deletion failure/);
  assert.match(await dialog.innerText(), /分组记录尚未删除/);
  assert.equal(await dialog.getByRole("button", { name: "删除", exact: true }).count(), 0);
  await dialog.getByRole("button", { name: "关闭", exact: true }).click();
  assert.deepEqual(await page.evaluate(
    /** Requires explicit closure, not an automatic retry or a fake successful result. */
    () => window.pureSidebarIntents,
  ), [{ type: "cancel-dialog" }]);
});
