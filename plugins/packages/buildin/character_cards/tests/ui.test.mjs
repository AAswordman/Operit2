import assert from "node:assert/strict";
import test from "node:test";
import http from "node:http";
import { createRequire } from "node:module";
import { mkdir } from "node:fs/promises";
import { createHtmlDocument } from "../scripts/build.mjs";
import { fixture } from "./fixtures.mjs";
const require = createRequire(import.meta.url);
const browserType = require("playwright")[process.env.CHARACTER_CARDS_UI_BROWSER ?? "chromium"];
const html = await createHtmlDocument();
let browser, server, origin;
const output = "D:/Code/prog/assistance2/target/character-dialog-device-test";

/** Serves only the actual offline graph artifact for browser verification. */
test.before(async () => {
  server = http.createServer((_request, response) => { response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" }); response.end(html); });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve)); origin = `http://127.0.0.1:${server.address().port}`;
  browser = await browserType.launch({ headless: true }); await mkdir(output, { recursive: true });
});
/** Releases browser and server resources after display tests. */
test.after(async () => { await browser?.close(); if (server) await new Promise(resolve => server.close(resolve)); });
/** Installs the display-only bridge with no CRUD service or browser-owned dialogs. */
async function page(t, width = 390, dark = false) {
  const data = fixture(), display = { data: data.graph, selected: null, selectedEdge: null, theme: data.theme };
  if (dark) { display.theme.brightness = "dark"; Object.assign(display.theme.colors, { surface: "#101417ff", onSurface: "#e0e3e5ff", primaryContainer: "#173847ff", onPrimaryContainer: "#c6e7ffff" }); }
  const context = await browser.newContext({ viewport: { width, height: 720 }, hasTouch: true }); t.after(() => context.close());
  await context.addInitScript(value => {
    window.selections = [];
    window.MemoryGraphHost = { async load() { return value; }, async select(kind, id) { window.selections.push({ kind, id }); } };
  }, display);
  const page = await context.newPage(); await page.goto(origin); await page.locator(".node").first().waitFor(); return page;
}

for (const width of [390, 1100]) for (const dark of [false, true]) test(`read-only graph fits ${width}px ${dark ? "dark" : "light"} without management controls`, async t => {
  const p = await page(t, width, dark);
  assert.equal(await p.locator(".node").count(), 3); assert.equal(await p.locator(".edge-hit").count(), 1);
  assert.equal(await p.locator("input, textarea, select, dialog, form").count(), 0);
  assert.deepEqual(await p.evaluate(() => Object.keys(window.MemoryGraphHost).sort()), ["load", "select"]);
  const box = await p.locator("svg").boundingBox(); assert.equal(box.width, width); assert.equal(box.height, 720);
  await p.screenshot({ path: `${output}/memory-graph-${width}-${dark ? "dark" : "light"}.png` });
});

test("graph selections preserve opaque node and relationship identifiers", async t => {
  const p = await page(t); await p.locator('[data-memory="memory-one"]').click();
  await p.locator('[data-memory="memory-two"]').focus(); await p.keyboard.press("Enter");
  await p.locator('[data-edge="1"]').focus(); await p.keyboard.press("Enter");
  assert.deepEqual(await p.evaluate(() => window.selections), [{ kind: "memory", id: "memory-one" }, { kind: "memory", id: "memory-two" }, { kind: "edge", id: "1" }]);
});

test("wheel zoom preserves its focal point and native camera controls fit the graph", async t => {
  const p = await page(t, 1100), transform = await p.locator("svg > g").getAttribute("transform");
  await p.mouse.move(550, 300); await p.mouse.wheel(0, -200); await p.waitForTimeout(80);
  assert.notEqual(await p.locator("svg > g").getAttribute("transform"), transform);
  await p.evaluate(() => window.memoryGraphCamera("fit")); assert.equal(await p.locator("svg > g").getAttribute("transform"), transform);
});

test("graph projection updates theme and escaped labels without adding browser mutation APIs", async t => {
  const p = await page(t), data = fixture(); data.graph.graph.nodes[0].label = '<img src=x onerror="window.injected=true">';
  await p.evaluate(value => window.updateMemoryGraph(value), { data: data.graph, selected: "memory-one", selectedEdge: null, theme: data.theme });
  assert.equal(await p.locator("img, script[src], input, form").count(), 0); assert.equal(await p.evaluate(() => window.injected), undefined);
  assert.equal(await p.locator('[data-memory="memory-one"]').getAttribute("class"), "node selected");
  assert.doesNotMatch(html, /CharacterMemoryHost|saveMemory|writeUser|saveCharacter|openEntity|createEditor/);
});


test("node dragging changes its position without panning or selecting, and canvas panning preserves nodes", async t => {
  const p = await page(t, 1100), node = p.locator('[data-memory="memory-one"]');
  const before = await node.getAttribute("transform"), camera = await p.locator("svg > g").getAttribute("transform"), box = await node.boundingBox();
  await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2); await p.mouse.down(); await p.mouse.move(box.x + box.width / 2 + 70, box.y + box.height / 2 + 40, { steps: 10 }); await p.mouse.up();
  assert.notEqual(await node.getAttribute("transform"), before); assert.equal(await p.locator("svg > g").getAttribute("transform"), camera); assert.deepEqual(await p.evaluate(() => window.selections), []);
  const moved = await node.getAttribute("transform"); await p.mouse.move(10, 10); await p.mouse.down(); await p.mouse.move(90, 50, { steps: 15 }); await p.mouse.up();
  assert.equal(await node.getAttribute("transform"), moved); assert.notEqual(await p.locator("svg > g").getAttribute("transform"), camera);
});

test("wheel units normalize pixel, line and page deltas", async t => {
  const p = await page(t, 1100), values = [];
  for (const [deltaMode, deltaY] of [[0, 160], [1, 10], [2, 160 / 720]]) {
    await p.evaluate(() => window.memoryGraphCamera("fit"));
    await p.locator("svg").dispatchEvent("wheel", { deltaMode, deltaY, clientX: 550, clientY: 360 });
    values.push(await p.locator("svg > g").getAttribute("transform"));
  }
  assert.equal(values[0], values[1]); assert.equal(values[0], values[2]);
});

test("two-pointer pinch changes camera without changing node positions or selecting", async t => {
  const p = await page(t, 1100), svg = p.locator("svg"), camera = await p.locator("svg > g").getAttribute("transform"), node = await p.locator('.node').first().getAttribute("transform");
  await svg.dispatchEvent("pointerdown", { pointerId: 11, pointerType: "touch", clientX: 100, clientY: 100 });
  await svg.dispatchEvent("pointerdown", { pointerId: 12, pointerType: "touch", clientX: 200, clientY: 100 });
  await svg.dispatchEvent("pointermove", { pointerId: 12, pointerType: "touch", clientX: 260, clientY: 100 });
  await svg.dispatchEvent("pointerup", { pointerId: 12, pointerType: "touch", clientX: 260, clientY: 100 }); await svg.dispatchEvent("pointerup", { pointerId: 11, pointerType: "touch", clientX: 100, clientY: 100 });
  assert.notEqual(await p.locator("svg > g").getAttribute("transform"), camera); assert.equal(await p.locator('.node').first().getAttribute("transform"), node); assert.deepEqual(await p.evaluate(() => window.selections), []);
});
