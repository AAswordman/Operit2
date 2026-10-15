import assert from "node:assert/strict";
import test from "node:test";
import http from "node:http";
import { createRequire } from "node:module";
import { createHtmlDocument } from "../scripts/build.mjs";
import { fixture, installBrowserFixture } from "./fixtures.mjs";

const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const browserType = require("playwright")[process.env.CHARACTER_CARDS_UI_BROWSER ?? "chromium"];
let browser, server, origin;

test.before(async () => {
  const html = await createHtmlDocument();
  server = http.createServer((_request, response) => {
    response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" }); response.end(html);
  });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
  browser = await browserType.launch({ headless: true });
});

test.after(async () => {
  if (browser) await browser.close();
  if (server) await new Promise(resolve => server.close(resolve));
});

async function open(view = "characters", malformed = false) {
  const context = await browser.newContext();
  await context.addInitScript({ content: `(${installBrowserFixture.toString()})(${JSON.stringify(fixture())});` });
  await context.addInitScript(({ view, malformed }) => {
    window.startupCalls = [];
    window.unhandledStartupErrors = [];
    window.addEventListener("unhandledrejection", event => { window.unhandledStartupErrors.push(String(event.reason)); });
    window.startupGate = new Promise((resolve, reject) => { window.releaseStartup = resolve; window.failStartup = reject; });
    const loadScreen = window.CharacterMemoryHost.loadScreen;
    window.CharacterMemoryHost.loadScreen = async () => {
      window.startupCalls.push("loadScreen"); await window.startupGate;
      const initial = await loadScreen.call(window.CharacterMemoryHost);
      initial.screen = { requestId: malformed ? "unowned-management" : null, input: { mode: "manage", view } };
      return initial;
    };
    window.CharacterMemoryHost.completeScreen = async () => { throw new Error("not a popup"); };
    window.CharacterMemoryHost.cancelScreen = async () => { throw new Error("not a popup"); };
    // Images deliberately never complete; they cannot gate the content metric.
    window.CharacterMemoryHost.avatarImage = async () => new Promise(() => {});
  }, { view, malformed });
  const page = await context.newPage(); page.setDefaultTimeout(5000);
  await page.goto(origin);
  await page.waitForFunction(() => window.startupCalls.length === 1);
  return { page, context };
}

for (const view of ["characters", "memory"]) {
  test(`${view} renders one initial screen response with no second snapshot read or avatar wait`, async () => {
    const { page, context } = await open(view);
    try {
      assert.equal(await page.locator('[data-avatar-uri]').count(), 0);
      assert.equal(await page.evaluate(() => performance.getEntriesByName("character-memory.content-ready").length), 0);
      await page.evaluate(() => window.releaseStartup());
      await page.waitForFunction(() => performance.getEntriesByName("character-memory.content-ready").length === 1);
      assert.equal(await page.locator('[data-avatar-uri]').count(), 2);
      assert.deepEqual(await page.evaluate(() => window.startupCalls), ["loadScreen"]);
      assert.deepEqual(await page.evaluate(() => window.testCalls), [{ action: "snapshot" }]);
      assert.equal(await page.locator('[role="alert"]').count(), 0);
      assert.deepEqual(await page.evaluate(() => window.unhandledStartupErrors), []);
    } finally { await context.close(); }
  });
}

test("initial screen rejection displays the failure without publishing content readiness", async () => {
  const { page, context } = await open();
  try {
    await page.evaluate(() => window.failStartup(new Error("STARTUP_READ_FAILED")));
    await page.getByRole("alert").filter({ hasText: "STARTUP_READ_FAILED" }).waitFor();
    assert.deepEqual(await page.evaluate(() => window.unhandledStartupErrors), []);
    assert.equal(await page.evaluate(() => performance.getEntriesByName("character-memory.content-ready").length), 0);
    assert.equal(await page.locator('[data-avatar-uri]').count(), 0);
  } finally { await context.close(); }
});

test("initial screen response still rejects an unowned management presentation", async () => {
  const { page, context } = await open("characters", true);
  try {
    await page.evaluate(() => window.releaseStartup());
    await page.getByRole("alert").filter({ hasText: "管理视图不能包含 presentation requestId" }).waitFor();
    assert.equal(await page.evaluate(() => performance.getEntriesByName("character-memory.content-ready").length), 0);
    assert.equal(await page.locator('[data-avatar-uri]').count(), 0);
  } finally { await context.close(); }
});
