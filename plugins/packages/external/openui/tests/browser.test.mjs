import assert from "node:assert/strict";
import { test, before, after } from "node:test";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { root, buildUpstreamReference } from "../scripts/build.mjs";

const FORM = 'root = Card([Form("survey", Buttons([Button("Send")]), [FormControl("Name", Input("name", "Your name", "text", {required: true}, $name))]), Buttons([Button("Clear", @Reset($name)), Button("Docs", @OpenUrl("https://example.org/"))])])\n$name = ""';
const REPORT = 'root = Card([CardHeader("Quarterly report", "Official OpenUI"), Table([Col("Team", ["Alpha", "Beta"]), Col("Count", [3, 5])]), BarChart(["Alpha", "Beta"], [Series("Count", [3, 5])])])';
let browser, server, origin;
const reference = await buildUpstreamReference();
const referenceHtml = '<!doctype html><html><head><meta charset="utf-8"><link rel="stylesheet" href="/openui.css"></head><body><main id="surface"><div id="root"></div><pre id="error" hidden></pre></main><script src="/reference.js"></script><script>const {createElement,createRoot,Renderer,ThemeProvider,openuiLibrary}=OpenUiReference;createRoot(document.getElementById("root")).render(createElement(ThemeProvider,{mode:"light"},createElement(Renderer,{response:window.referenceProgram,library:openuiLibrary,isStreaming:false,publishObservability:false})));</script></body></html>';

/** Serves the packaged assets and a separate, direct upstream reference page. */
before(async function startBrowser() {
  const files = new Map([
    ["/", { type: "text/html", body: await readFile(path.join(root, "dist/web/index.html")) }],
    ["/openui.js", { type: "application/javascript", body: await readFile(path.join(root, "dist/web/openui.js")) }],
    ["/openui.css", { type: "text/css", body: await readFile(path.join(root, "dist/web/openui.css")) }],
    ["/reference", { type: "text/html", body: referenceHtml }],
    ["/reference.js", { type: "application/javascript", body: reference.script }],
  ]);
  /** Serves only the explicitly declared test assets. */
  server = createServer(function serve(request, response) {
    const file = files.get(request.url);
    if (file === undefined) { response.writeHead(404); response.end(); return; }
    response.writeHead(200, { "Content-Type": file.type });
    response.end(file.body);
  });
  /** Binds a private ephemeral port for browser verification. */
  await new Promise(function listen(resolve) { server.listen(0, "127.0.0.1", resolve); });
  origin = "http://127.0.0.1:" + server.address().port;
  browser = await chromium.launch({ headless: true });
});

/** Releases Chromium and the local asset server after verification. */
after(async function stopBrowser() {
  if (browser !== undefined) await browser.close();
  /** Waits until the test asset listener is closed. */
  if (server !== undefined) await new Promise(function close(resolve) { server.close(resolve); });
});

/** Installs the real documented browser bridge shape and records its calls. */
async function pluginPage(program, streaming = false, formState = {}, width = 820) {
  const page = await browser.newPage({ viewport: { width, height: 1000 }, colorScheme: "light" });
  /** Exposes only the documented capabilities inside the isolated browser page. */
  await page.addInitScript(function installHost(input) {
    window.hostCalls = { actions: [], states: [], tools: [], heights: [] };
    window.OpenUiHost = {
      /** Hydrates this page from its owning screen snapshot. */
      async bootstrap() { return { input: { program: input.program, isStreaming: input.streaming, revision: 0 }, formState: input.formState, theme: { brightness: "light", colors: {} } }; },
      /** Captures the unmodified upstream field-state map. */
      async state(value) { window.hostCalls.states.push(value); },
      /** Captures document height notifications. */
      async resize(value) { window.hostCalls.heights.push(value); },
      /** Captures upstream actions without altering their payloads. */
      async action(value) { window.hostCalls.actions.push(value); },
      /** Executes only the explicit tools used by the browser fixture. */
      async callTool(request) {
        window.hostCalls.tools.push(request);
        if (request.name === "load_demo") return { title: "Loaded through host tools" };
        if (request.name === "save_demo") return { saved: true };
        throw new Error("Unexpected test tool: " + request.name);
      },
    };
  }, { program, streaming, formState });
  await page.goto(origin + "/");
  await page.waitForFunction(function mounted() { return document.querySelector("#root").childElementCount > 0; });
  return page;
}

/** Captures the upstream component tree and resolved styles without unstable generated identities. */
async function componentSnapshot(page) {
  return page.locator("#root").evaluate(function snapshot(root) {
    /** Serializes meaningful rendered output recursively. */
    function visit(node) {
      if (node.nodeType === Node.TEXT_NODE) return node.textContent;
      if (node.nodeType !== Node.ELEMENT_NODE) return null;
      const style = getComputedStyle(node);
      return {
        tag: node.tagName, className: node.getAttribute("class"), type: node.getAttribute("type"),
        color: style.color, font: style.fontSize, background: style.backgroundColor,
        children: Array.from(node.childNodes, visit),
      };
    }
    return visit(root);
  });
}

/** Compares component output and computed styles with direct upstream rendering. */
test("full-library forms, tables and charts match direct upstream rendering", async function parity() {
  assert.ok(reference.components.includes("EditableTable"));
  assert.ok(reference.components.includes("BarChart"));
  for (const program of [REPORT, FORM]) {
    const actual = await pluginPage(program);
    const expected = await browser.newPage({ viewport: { width: 820, height: 1000 }, colorScheme: "light" });
    await expected.addInitScript(function initialProgram(value) { window.referenceProgram = value; }, program);
    await expected.goto(origin + "/reference");
    await expected.waitForFunction(function mounted() { return document.querySelector("#root").childElementCount > 0; });
    await actual.evaluate(function layout() { return new Promise(requestAnimationFrame); });
    await expected.evaluate(function layout() { return new Promise(requestAnimationFrame); });
    assert.equal(await actual.locator("#error").isVisible(), false, await actual.locator("#error").textContent());
    assert.deepEqual(await componentSnapshot(actual), await componentSnapshot(expected));
    await actual.close();
    await expected.close();
  }
});

/** Verifies a completed stream preserves live fields, bindings and action payloads. */
test("stream completion and later revisions retain the same input element and form values", async function streamState() {
  const page = await pluginPage(FORM, true);
  await page.evaluate(function complete(program) { window.OpenUiPlugin.update({ program, isStreaming: false, revision: 1 }); }, FORM);
  const input = page.getByPlaceholder("Your name");
  await input.fill("Alice");
  await input.evaluate(function retainIdentity(element) { window.originalInput = element; });
  await page.evaluate(function extend(program) { window.OpenUiPlugin.update({ program: program + '\nunused = "later"', isStreaming: false, revision: 2 }); }, FORM);
  assert.equal(await input.inputValue(), "Alice");
  assert.equal(await input.evaluate(function sameElement(element) { return element === window.originalInput; }), true);
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await page.waitForFunction(function sent() { return window.hostCalls.actions.length === 1; });
  const action = await page.evaluate(function readAction() { return window.hostCalls.actions[0]; });
  assert.equal(action.type, "continue_conversation");
  assert.equal(action.formName, "survey");
  assert.ok(JSON.stringify(action.formState).includes("Alice"));
  await page.getByRole("button", { name: "Clear", exact: true }).click();
  await page.waitForFunction(function reset() { return document.querySelector('input[placeholder="Your name"]').value === ""; });
  await page.getByRole("button", { name: "Docs", exact: true }).click();
  await page.waitForFunction(function opened() { return window.hostCalls.actions.length === 2; });
  const link = await page.evaluate(function readLink() { return window.hostCalls.actions[1]; });
  assert.equal(link.type, "open_url");
  assert.equal(link.params.url, "https://example.org/");
  assert.equal(await page.locator("#error").isVisible(), false, await page.locator("#error").textContent());
  await page.close();
});

/** Verifies the official query runtime and theme provider cross the existing host bridge. */
test("Query and Mutation use the host tool provider and theme changes use the upstream provider", async function queryAndTheme() {
  const program = 'root = Card([TextContent(data.title), Buttons([Button("Save", @Run(save))])])\ndata = Query("load_demo", {id: 7}, {title: "Loading"})\nsave = Mutation("save_demo", {value: "kept"})';
  const page = await pluginPage(program);
  await page.getByText("Loaded through host tools", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await page.waitForFunction(function mutationRan() { return window.hostCalls.tools.some(function isSave(tool) { return tool.name === "save_demo"; }); });
  const calls = await page.evaluate(function readTools() { return window.hostCalls.tools; });
  assert.equal(calls.find(function isQuery(tool) { return tool.name === "load_demo"; }).arguments.id, 7);
  assert.equal(calls.find(function isMutation(tool) { return tool.name === "save_demo"; }).arguments.value, "kept");
  const light = await page.locator("#root").evaluate(function foreground(element) { return getComputedStyle(element.firstElementChild).getPropertyValue("--openui-background"); });
  await page.evaluate(function darkTheme() { window.OpenUiPlugin.theme({ brightness: "dark", colors: {} }); });
  await page.waitForFunction(function changedTheme(previous) { return getComputedStyle(document.querySelector("#root").firstElementChild).getPropertyValue("--openui-background") !== previous; }, light);
  assert.equal(await page.locator("#error").isVisible(), false, await page.locator("#error").textContent());
  await page.close();
});

/** Recreates the browser document from an opaque state map retained by its owning screen. */
test("document recreation hydrates the upstream field state on a narrow viewport", async function hydration() {
  const original = await pluginPage(FORM);
  await original.getByPlaceholder("Your name").fill("Alice");
  await original.waitForFunction(function capturedState() { return JSON.stringify(window.hostCalls.states).includes("Alice"); });
  const savedState = await original.evaluate(function latestState() { return window.hostCalls.states.at(-1); });
  await original.close();
  const restored = await pluginPage(FORM, false, savedState, 390);
  assert.equal(await restored.getByPlaceholder("Your name").inputValue(), "Alice");
  assert.equal(await restored.locator("#error").isVisible(), false, await restored.locator("#error").textContent());
  await restored.getByRole("button", { name: "Send", exact: true }).click();
  await restored.waitForFunction(function sent() { return window.hostCalls.actions.length === 1; });
  const action = await restored.evaluate(function readAction() { return window.hostCalls.actions[0]; });
  assert.ok(JSON.stringify(action.formState).includes("Alice"));
  const size = await restored.locator("#root").boundingBox();
  assert.ok(size.width > 0 && size.width <= 390);
  await restored.close();
});
