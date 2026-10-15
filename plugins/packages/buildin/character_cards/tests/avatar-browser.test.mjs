import assert from "node:assert/strict";
import test from "node:test";
import http from "node:http";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { createHtmlDocument } from "../scripts/build.mjs";
import { fixture, installBrowserFixture } from "./fixtures.mjs";
import { loadModule } from "./runtime.mjs";

const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const browserType = require("playwright")[process.env.CHARACTER_CARDS_UI_BROWSER ?? "chromium"];
let browser, server, origin, resources;
const counts = new Map();

test.before(async () => {
  const html = await createHtmlDocument();
  const installed = await readFile(new URL("../resources/character-memory.html", import.meta.url), "utf8");
  const avatar = await readFile(new URL("../resources/operit-avatar.png", import.meta.url));
  const { createAvatarResources } = loadModule("src/avatar-resources.ts", {
    ToolPkg: { async readResource() { return "/app/default.png"; } },
  });
  resources = createAvatarResources("https://characters.operit.local/");
  // Explicit HTTP/browser adapter: native VFS and MIME inference have separate Flutter tests.
  server = http.createServer((request, response) => {
    if (request.url === "/" || request.url === "/installed") {
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" }); response.end(request.url === "/installed" ? installed : html); return;
    }
    const decision = resources.intercept({ url: "https://characters.operit.local" + request.url, method: "GET", isMainFrame: false, headers: {} });
    if (decision?.action !== "respond") { response.writeHead(404); response.end(); return; }
    const path = decision.response.filePath;
    counts.set(path, (counts.get(path) ?? 0) + 1);
    if (path === "/app/missing.image") { response.writeHead(404); response.end(); return; }
    response.writeHead(200, { "Content-Type": "image/png", "Cache-Control": "no-store" }); response.end(avatar);
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  browser = await browserType.launch({ headless: true });
});

test.after(async () => {
  if (browser) await browser.close();
  if (server) await new Promise(resolve => server.close(resolve));
});

async function open({ view = "characters", reject = false, installed = false, missing = false } = {}) {
  const data = fixture();
  data.snapshot.cards = Array.from({ length: 20 }, (_, index) => ({ ...data.snapshot.cards[0], id: "card-" + index, name: "Visible character " + index, avatarUri: index === 19 ? missing ? "/app/missing.image" : "/app/custom image.image" : null }));
  data.snapshot.groups = [];
  data.snapshot.active = { CharacterCard: { id: "card-0" } };
  const context = await browser.newContext({ viewport: { width: 800, height: 600 } });
  await context.exposeFunction("testAvatarReference", path => resources.source(path));
  await context.addInitScript({ content: `(${installBrowserFixture.toString()})(${JSON.stringify(data)});` });
  await context.addInitScript(({ view, reject }) => {
    window.avatarCalls = [];
    window.rejectAvatar = reject;
    window.avatarGate = new Promise(resolve => { window.releaseAvatars = resolve; });
    const loadScreen = window.CharacterMemoryHost.loadScreen;
    window.CharacterMemoryHost.loadScreen = async () => ({ ...await loadScreen.call(window.CharacterMemoryHost), screen: { requestId: null, input: { mode: "manage", view } } });
    window.CharacterMemoryHost.completeScreen = async () => { throw new Error("not a presented screen"); };
    window.CharacterMemoryHost.cancelScreen = async () => { throw new Error("not a presented screen"); };
    window.CharacterMemoryHost.avatarImage = async path => {
      window.avatarCalls.push(path); await window.avatarGate;
      if (window.rejectAvatar) throw new Error("image lookup unavailable");
      return window.testAvatarReference(path);
    };
  }, { view, reject });
  const page = await context.newPage();
  page.setDefaultTimeout(5000);

  await page.goto(origin + (installed ? "/installed" : "/"));
  return { page, context };
}

for (const view of ["characters", "memory"]) {
  test(`${view} content paints before avatars; completion patches images without replacing the page`, async () => {
    const { page, context } = await open({ view });
    try {
      await page.getByText("Visible character 0", { exact: true }).waitFor();
      assert.equal(await page.locator("[data-avatar-uri]").count(), 20);
      assert.equal(await page.locator("[data-avatar-uri] img").count(), 0);
      await page.waitForFunction(() => window.avatarCalls.length === 2);
      assert.equal(await page.locator('[role="alert"]').count(), 0);
      await page.evaluate(() => { window.originalCard = document.querySelector("[data-avatar-uri]"); window.originalPage = document.querySelector(".page"); window.releaseAvatars(); });
      await page.waitForFunction(() => {
        const images = [...document.querySelectorAll("[data-avatar-uri] img")];
        return images.length === 20 && images.every(image => image.complete && image.naturalWidth === 1024);
      });
      assert.equal(await page.evaluate(() => window.originalCard === document.querySelector("[data-avatar-uri]") && window.originalPage === document.querySelector(".page")), true);
      assert.equal(await page.evaluate(() => new Set([...document.querySelectorAll("[data-avatar-uri] img")].map(image => image.getAttribute("src"))).size), 2);
      assert.deepEqual(await page.evaluate(() => window.avatarCalls), [null, "/app/custom image.image"]);
    } finally { await context.close(); }
  });
}

test("avatar bridge failure leaves all cards visible and usable", async () => {
  const { page, context } = await open({ reject: true });
  try {
    await page.getByText("Visible character 0", { exact: true }).waitFor();
    await page.evaluate(() => window.releaseAvatars());
    await page.waitForFunction(() => window.avatarCalls.length === 2);
    assert.equal(await page.locator('[data-action="edit-card"]').count(), 20);
    assert.equal(await page.locator('[role="alert"]').count(), 0);
    assert.equal(await page.locator("[data-avatar-uri] > svg").count(), 20);
    await page.getByRole("button", { name: "编辑 Visible character 0", exact: true }).click();
    assert.equal(await page.evaluate(() => window.testCalls.some(call => call.action === "openEntity")), true);
  } finally { await context.close(); }
});


test("failed avatar lookup retries on a later refresh without blocking either render", async () => {
  const { page, context } = await open({ reject: true });
  try {
    await page.getByText("Visible character 0", { exact: true }).waitFor();
    await page.evaluate(() => window.releaseAvatars());
    await page.waitForFunction(() => window.avatarCalls.length === 2);
    await page.evaluate(() => {
      window.rejectAvatar = false;
      window.dispatchEvent(new CustomEvent("character-entity-closed"));
    });
    await page.waitForFunction(() => document.querySelectorAll("[data-avatar-uri] img").length === 20);
    assert.equal(await page.locator('[role="alert"]').count(), 0);
    assert.equal(await page.locator('[data-action="edit-card"]').count(), 20);
    assert.equal(await page.evaluate(() => window.avatarCalls.length), 4);
  } finally { await context.close(); }
});


test("rebuilt installed document loads its same-origin avatar resources", async () => {
  const { page, context } = await open({ installed: true });
  try {
    await page.getByText("Visible character 0", { exact: true }).waitFor();
    await page.evaluate(() => window.releaseAvatars());
    await page.waitForFunction(() => document.querySelectorAll("[data-avatar-uri] img").length === 20);
    assert.equal(await page.locator('[role="alert"]').count(), 0);
  } finally { await context.close(); }
});

test("unreadable image keeps its placeholder while other avatars load", async () => {
  const before = counts.get("/app/missing.image") ?? 0;
  const { page, context } = await open({ missing: true });
  try {
    await page.getByText("Visible character 0", { exact: true }).waitFor();
    await page.evaluate(() => window.releaseAvatars());
    await page.waitForFunction(() => document.querySelectorAll("[data-avatar-uri] img").length === 19);
    assert.ok((counts.get("/app/missing.image") ?? 0) > before);
    assert.equal(await page.locator('[data-avatar-uri="/app/missing.image"] > svg').count(), 1);
    assert.equal(await page.locator('[data-action="edit-card"]').count(), 20);
  } finally { await context.close(); }
});
