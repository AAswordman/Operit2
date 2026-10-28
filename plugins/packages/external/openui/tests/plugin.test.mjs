import assert from "node:assert/strict";
import { test } from "node:test";
import vm from "node:vm";
import { readFile } from "node:fs/promises";
import { build } from "esbuild";
import { root } from "../scripts/build.mjs";
import path from "node:path";

/** Evaluates a readable CommonJS ToolPkg module with its actual host globals. */
function evaluateModule(source, globals, require) {
  const module = { exports: {} };
  vm.runInNewContext(source, { module, exports: module.exports, console, Promise, Map, ...globals, require });
  return module.exports;
}
const screenSource = await readFile(path.join(root, "dist/ui/index.ui.js"), "utf8");
const mainSource = await readFile(path.join(root, "dist/main.js"), "utf8");
const protocolBuild = await build({ absWorkingDir: root, entryPoints: ["src/protocol.ts"], bundle: true, platform: "node", format: "cjs", write: false });
const protocol = evaluateModule(protocolBuild.outputFiles[0].text, {}, undefined);

/** Models the public Compose lifecycle and captures each real package capability request. */
function createScreenHarness(xmlContent) {
  const states = new Map([["xmlContent", xmlContent], ["chatId", "owning-chat"]]);
  const refs = new Map(), effects = new Map(), interfaces = new Map();
  const scripts = [], reads = [], messages = [], tools = [], listeners = new Set();
  const controller = {
    /** Registers one named privileged interface. */
    addJavascriptInterface(name, methods) { interfaces.set(name, methods); },
    /** Removes a disposed interface. */
    removeJavascriptInterface(name) { interfaces.delete(name); },
    /** Captures scripts sent to the retained browser document. */
    async evaluateJavascript(script) { scripts.push(script); },
  };
  /** Creates a concrete descriptor for each generic Compose component. */
  function node(type) {
    return function createNode(props, children) { return { type, props, children }; };
  }
  const ctx = {
    UI: Object.fromEntries(["Column", "Text", "WebView", "Dialog", "Button"].map(
      /** Exposes only the component factories used by the actual screen. */
      type => [type, node(type)],
    )),
    /** Retains keyed Compose state across screen renders. */
    useState(key, initialValue) {
      if (!states.has(key)) states.set(key, initialValue);
      /** Replaces one retained state value. */
      return [states.get(key), function set(value) { states.set(key, value); }];
    },
    /** Retains keyed mutable references across screen renders. */
    useRef(key, initialValue) {
      if (!refs.has(key)) refs.set(key, { current: initialValue });
      return refs.get(key);
    },
    /** Runs keyed effects only when their dependencies change. */
    useEffect(key, setup, dependencies) {
      const previous = effects.get(key);
      if (previous !== undefined && previous.dependencies.length === dependencies.length && dependencies.every(
        /** Compares dependencies using the public keyed effect contract. */
        (value, index) => Object.is(value, previous.dependencies[index]),
      )) return;
      if (previous !== undefined) previous.cleanup();
      effects.set(key, { dependencies, cleanup: setup() });
    },
    /** Returns the stable controller for this browser instance. */
    createWebViewController() { return controller; },
    Theme: {
      /** Returns the host's effective theme snapshot. */
      getCurrent() { return { brightness: "light", colors: {} }; },
      /** Registers a theme listener with an explicit cleanup callback. */
      subscribe(listener) { listeners.add(listener); return function unsubscribe() { listeners.delete(listener); }; },
    },
    /** Captures normal tool-host requests. */
    async callTool(name, parameters) { tools.push({ name, parameters }); return { result: "tool-value" }; },
  };
  const globals = {
    ToolPkg: {
      /** Resolves only the requested package resource. */
      async readResource(key, name) { reads.push({ key, name }); return "/resources/" + name; },
    },
    Tools: { Chat: {
      /** Captures the exact owning conversation and submitted action payload. */
      async sendMessage(request) { messages.push(request); return { chatId: request.chatId, message: request.input.text, sentAt: 1, outcome: { kind: "completed" } }; },
    } },
  };
  const screen = evaluateModule(screenSource, globals, undefined).default;
  return { states, refs, effects, interfaces, controller, scripts, reads, messages, tools, listeners, ctx,
    /** Renders one revision of the actual packaged screen. */
    render() { return screen(ctx); },
  };
}

/** Finds a concrete component in the generic Compose descriptor tree. */
function findNode(node, type) {
  if (node.type === type) return node;
  if (!Array.isArray(node.children)) return undefined;
  for (const child of node.children) {
    const match = findNode(child, type);
    if (match !== undefined) return match;
  }
  return undefined;
}

/** Ensures XML framing never rewrites the upstream language body. */
test("the envelope preserves OpenUI code, dollars and every partial closing delimiter", function envelope() {
  const program = '\nroot = Card([TextContent("$x$ and <b>literal</b>")])\n$steps = 3\n';
  const closing = "</openui>";
  for (let length = 0; length < closing.length; length++) {
    const parsed = protocol.parseOpenUi("<openui>" + program + closing.slice(0, length));
    assert.equal(parsed.program, program);
    assert.equal(parsed.isStreaming, true);
  }
  assert.equal(protocol.parseOpenUi("<openui>" + program + closing).isStreaming, false);
  assert.throws(function rejectUnwrappedCode() { protocol.parseOpenUi(program); }, /envelope/);
});

/** Verifies public hook registration and prompt ownership. */
test("registration and model instructions belong entirely to the optional plugin", function registrations() {
  const registered = { xml: [], prompts: [] };
  const main = evaluateModule(mainSource, {
    ToolPkg: {
      /** Captures the plugin's XML render contribution. */
      registerXmlRenderPlugin(item) { registered.xml.push(item); },
      /** Captures the plugin's system-prompt contribution. */
      registerSystemPromptComposeHook(item) { registered.prompts.push(item); },
    },
  }, function requireScreen(specifier) {
    assert.equal(specifier, "./ui/index.ui.js");
    return evaluateModule(screenSource, {}, undefined);
  });
  assert.equal(main.registerToolPkg(), true);
  assert.equal(registered.xml.length, 1);
  assert.equal(registered.xml[0].tag, "openui");
  assert.equal(registered.prompts.length, 1);
  assert.equal(main.onSystemPrompt({ eventName: "before_compose_system_prompt", eventPayload: { systemPrompt: "base" } }), undefined);
  const prompt = main.onSystemPrompt({ eventName: "after_compose_system_prompt", eventPayload: { systemPrompt: "base" } }).systemPrompt;
  assert.ok(prompt.startsWith("base\n\nUse OpenUI Lang inside an <openui>"));
  assert.ok(prompt.includes("EditableTable("));
  assert.ok(prompt.includes("@Set("));
  const rendered = main.onXmlRender({ eventPayload: { tagName: "openui", xmlContent: "<openui>root = Card([])</openui>", chatId: "target" } });
  assert.equal(rendered.composeDsl.state.chatId, "target");
  assert.equal(typeof rendered.composeDsl.screen, "function");
});

/** Verifies retained browser identity and cleanup through the Compose lifecycle. */
test("stream updates preserve the WebView, state and resource ownership", async function lifecycle() {
  const harness = createScreenHarness('<openui>root = Card([TextContent("one")])');
  await harness.render().props.onLoad();
  const firstWebView = findNode(harness.render(), "WebView");
  const bridge = harness.interfaces.get("OpenUiHost");
  const bootstrap = bridge.bootstrap([]);
  assert.equal(bootstrap.input.isStreaming, true);
  assert.equal(harness.reads.length, 3);
  bridge.state([{ survey: { name: { value: "Alice", componentType: "Input" } } }]);
  harness.states.set("xmlContent", '<openui>root = Card([TextContent("one more")])</openui>');
  const secondWebView = findNode(harness.render(), "WebView");
  assert.equal(firstWebView.props.controller, secondWebView.props.controller);
  assert.equal(harness.reads.length, 3);
  assert.equal(bridge.bootstrap([]).formState.survey.name.value, "Alice");
  assert.ok(harness.scripts.some(function hasCompletedRevision(script) { return script.includes('"isStreaming":false'); }));
  assert.equal(secondWebView.props.onInterceptRequest({ url: "https://openui.operit.local/openui.js" }).response.filePath, "/resources/openui.js");
  assert.equal(secondWebView.props.onInterceptRequest({ url: "https://openui.operit.local/missing" }).action, "block");
  assert.equal(secondWebView.props.onInterceptRequest({ url: "https://images.example.org/image.png" }).action, "allow");
  for (const effect of harness.effects.values()) effect.cleanup();
  assert.equal(harness.interfaces.size, 0);
  assert.equal(harness.listeners.size, 0);
});

/** Verifies conversation identity, tool dispatch and unprivileged URL dialogs. */
test("upstream actions keep the exact owning chat and original form payload", async function actions() {
  const harness = createScreenHarness("<openui>root = Card([])</openui>");
  await harness.render().props.onLoad();
  const bridge = harness.interfaces.get("OpenUiHost");
  const action = { type: "continue_conversation", params: { message: "send" }, humanFriendlyMessage: "Submit", formState: { survey: { name: { value: "Alice" } } }, formName: "survey" };
  await bridge.action([action]);
  assert.equal(harness.messages.length, 1);
  assert.equal(harness.messages[0].chatId, "owning-chat");
  assert.deepEqual(JSON.parse(harness.messages[0].input.text.split("OpenUI action:\n")[1]), action);
  await bridge.action([{ type: "open_url", params: { url: "https://example.org/" }, humanFriendlyMessage: "Docs" }]);
  const dialog = findNode(harness.render(), "Dialog");
  const browser = findNode(dialog, "WebView");
  assert.equal(browser.props.url, "https://example.org/");
  assert.equal(browser.props.controller, undefined);
  assert.equal(harness.interfaces.size, 1);
  await bridge.callTool([{ name: "get_example", arguments: { id: 1 } }]);
  await bridge.callTool([{ name: "no_arguments" }]);
  assert.equal(harness.tools[0].name, "get_example");
  assert.equal(harness.tools[0].parameters.id, 1);
  assert.equal(harness.tools[1].parameters, undefined);
  await assert.rejects(bridge.action([{ type: "invented", params: {}, humanFriendlyMessage: "Invalid" }]), /Unsupported/);
  harness.states.set("xmlContent", "<openui>root = Card([])");
  harness.render();
  await assert.rejects(bridge.action([action]), /completed block/);
});
