import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { buildMainScript, buildUiScreenScripts } from "../scripts/build.mjs";
import { packageRuntime, sdkScript } from "./package-runtime.mjs";

/** Runs actual main registration through the SDK; the only transport adapter records lazy IPC handlers. */
function registeredPackage(modules, registrationOnly = false) {
  const channels = new Map();
  const runtime = packageRuntime(modules, {}, registrationOnly);
  let main;
  runtime.context.__operitCurrentCallId = "compose-route-registration";
  runtime.context.__operitGetCallState = () => ({ params: { __operit_toolpkg_api_version: "2.0.0", toolPkgId: "com.operit.character_cards" } });
  runtime.context.__operitGetActiveModuleExports = () => main;
  runtime.context.__operitExpose = (name, value) => { runtime.context[name] = value; };
  vm.runInContext(sdkScript("ToolPkgApiRuntimeScript.rs"), runtime.context);
  vm.runInContext(sdkScript("ToolPkgRegistrationBridge.rs").replace("__OPERIT_TOOLPKG_REGISTRATION_ONLY__", "true"), runtime.context);
  runtime.context.ToolPkg.ipc = { on(name, callback) {
    assert.equal(channels.has(name), false, "Duplicate IPC registration: " + name);
    channels.set(name, callback);
  } };
  main = runtime.load("dist/main.js");
  assert.equal(main.registerToolPkg(), true);
  const routes = JSON.parse(vm.runInContext("JSON.stringify(__operitToolPkgRegistrationCapture.uiRoutes.map(JSON.parse))", runtime.context));
  return { ...runtime, main, routes };
}

/** Renders a captured path using both actual SDK scripts, not a direct call to a named main export. */
function renderRoute(modules, route, state) {
  const runtime = packageRuntime(modules);
  runtime.context.module = { exports: runtime.load(route.screen) };
  vm.runInContext(sdkScript("ToolPkgComposeDslBridge.rs"), runtime.context);
  const wrapper = sdkScript("ToolPkgComposeDslRuntimeScript.rs").replaceAll("{{", "{").replaceAll("}}", "}").replace("{script}", "");
  vm.runInContext(wrapper, runtime.context);
  return runtime.context.__operit_render_compose_dsl({ state, moduleSpec: route });
}

/** Traverses only serialized tree nodes to verify which actual screen was rendered. */
function nodes(node) { return [node, ...node.children.flatMap(nodes)]; }

/** Reproduces the native path-only registration boundary instead of directly calling main.screen in-process. */
test("registered Compose routes retain six independent executable module paths", async () => {
  const modules = { "dist/main.js": await buildMainScript(), ...await buildUiScreenScripts() };
  const plugin = registeredPackage(modules);
  assert.equal(plugin.routes.length, 6);
  assert.equal(new Set(plugin.routes.map(route => route.screen )).size, 6,
    "Path-only SDK registration must not collapse distinct named main exports to dist/main.js");
  for (const route of plugin.routes) {
    assert.equal(route.screen, `dist/ui/${route.id}/index.ui.js`);
    assert.ok(Object.hasOwn(modules, route.screen), route.screen);
    const exports = plugin.load(route.screen);
    assert.equal(typeof exports.default, "function", "Compose runtime requires a default screen entry: " + route.screen);
  }
});

/** Supplies explicit route inputs and checks each independent entry reaches the intended Compose tree. */
test("the SDK Compose wrapper renders every serialized character route with its own entry", async t => {
  const modules = { "dist/main.js": await buildMainScript(), ...await buildUiScreenScripts() };
  const plugin = registeredPackage(modules);
  const cases = {
    main: { state: {}, type: "Box", text: "正在加载角色卡…" },
    memory: { state: {}, type: "Box", text: "正在加载记忆…" },
    "memory-attachment": { state: { presentation: { requestId: "attachment-route-test", input: { mode: "memory-attachment", ownerKey: null, folderPath: null } } }, type: "Box", text: "正在加载角色卡…" },
    "chat-sidebar": { state: { input: { view: "characters" }, chatSidebar: { chats: [], currentChatId: null, activeStreamingChatIds: [] } }, type: "Column", key: "native-character-sidebar" },
    selection: { state: { presentation: { requestId: "selection-route-test", input: { mode: "select", kind: "all", selected: null, chatId: null } } }, type: "Dialog", key: "character-selection" },
    "group-execution": { state: { presentation: { requestId: "group-route-test", input: { mode: "group-execution", chatId: "explicit-chat-test" } } }, type: "Dialog", key: "group-execution-dialog" },
  };
  for (const route of plugin.routes) await t.test(route.id, async () => {
    const expected = cases[route.id];
    const result = await renderRoute(modules, route, expected.state);
    assert.equal(result.tree.type, expected.type);
    if (expected.key !== undefined) assert.equal(result.tree.props.key, expected.key);
    if (expected.text !== undefined) assert.ok(nodes(result.tree).some(node => node.props.text === expected.text));
  });
});

/** Checks installed/Core archives and each screen's bytes, not just an in-memory source bundle. */
test("installed and Core production archives contain the executable registered screen modules", async () => {
  const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
  const { unzipSync } = require("fflate");
  const packaged = await readFile(new URL("../dist/character_cards.toolpkg", import.meta.url));
  const production = await readFile(new URL("../../../../../core/crates/runtime/application/assets/plugins/buildin/character_cards.toolpkg", import.meta.url));
  assert.equal(Buffer.compare(packaged, production), 0, "Core must install the rebuilt package archive");
  const modules = unzipSync(packaged), plugin = registeredPackage(modules);
  const currentScreens = await buildUiScreenScripts();
  assert.equal(new Set(plugin.routes.map(route => route.screen )).size, 6);
  for (const route of plugin.routes) {
    assert.ok(Object.hasOwn(currentScreens, route.screen), "Undeclared installed screen: " + route.screen);
    const installed = await readFile(new URL("../" + route.screen, import.meta.url));
    assert.equal(Buffer.compare(Buffer.from(modules[route.screen]), installed), 0, route.screen);
    assert.equal(Buffer.compare(installed, Buffer.from(currentScreens[route.screen])), 0, "Stale installed screen: " + route.screen);
    assert.equal(typeof plugin.load(route.screen).default, "function");
  }
});

/** Exercises the engine's real registration-only placeholders without evaluating or even supplying UI bundles. */
test("registration-only main resolves independent UI placeholders without evaluating screens", async () => {
  const plugin = registeredPackage({ "dist/main.js": await buildMainScript() }, true);
  assert.equal(plugin.routes.length, 6);
  assert.equal(new Set(plugin.routes.map(route => route.screen )).size, 6);
  for (const route of plugin.routes) assert.equal(route.screen, `dist/ui/${route.id}/index.ui.js`);
});

/** Ensures standalone modal entries do not silently turn a missing presentation into the management editor. */
test("independent modal entries preserve explicit presentation validation", async () => {
  const modules = { "dist/main.js": await buildMainScript(), ...await buildUiScreenScripts() };
  const plugin = registeredPackage(modules);
  const errors = { "memory-attachment": /explicit attachment presentation/, selection: /explicit select presentation/, "group-execution": /group control presentation 必须是对象/ };
  for (const route of plugin.routes) if (Object.hasOwn(errors, route.id)) assert.throws(() => renderRoute(modules, route, {}), errors[route.id]);
});

/** UI runtimes only call main-runtime IPC; retaining its lazy factory dragged the entire backend into every screen. */
test("UI bundles omit the file repository and memory job backend", async () => {
  const screens = await buildUiScreenScripts();
  for (const [screen, bytes] of Object.entries(screens)) {
    const script = Buffer.from(bytes).toString("utf8");
    assert.doesNotMatch(script, /FileCharacterRepository|MemoryJobRunner|function createServiceRuntime/, screen);
  }
  assert.ok(screens["dist/ui/chat-sidebar/index.ui.js"].length < 100_000,
    "Keep the native sidebar lightweight instead of bundling the management backend");
});
