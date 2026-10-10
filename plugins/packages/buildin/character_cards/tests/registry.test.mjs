import assert from "node:assert/strict";
import test from "node:test";
import { fixture } from "./fixtures.mjs";
import { interfaceMembers, loadModule, plain } from "./runtime.mjs";

const roots = ["character", "group", "tag", "active-prompt", "memory"];

/** Keeps input-menu layout out of public display metadata now that the actual DSL route owns rendering. */
test("chat context projection publishes identity only while its menu renders through DSL", () => {
  const api = loadModule("src/ui-contributions.ts"), directory = fixture().snapshot;
  const routes = { editor: "editor-route", selection: "selection-route", execution: "execution-route" };
  for (const selection of [null, "card:travel", "group:group-one"]) {
    const context = plain(api.contextActions(routes, { chatId: "explicit-chat", selection }, directory));
    assert.deepEqual(Object.keys(context).sort(), ["backgroundUri", "identity"]);
    if (selection === null) assert.equal(context.identity, null);
    else assert.equal(context.identity.action.routeId, routes.editor);
  }
});

/** Builds a real SDK-shaped command event for parser/help checks without business persistence. */
function commandEvent(name, args) {
  return { event: "core_command", eventName: "core_command", eventPayload: { commandId: name, commandName: name, args, json: true } };
}

/** Captures provider registrations only; it implements no production business operations. */
test("plugin registers exactly the transferred provider roots using the actual generic SDK signature", async () => {
  const definitions = [];
  const registry = { registerCoreCommand:
    /** Records only the actual registration definition published by the plugin. */
    definition => definitions.push(definition),
  };
  const api = loadModule("src/commands.ts", { ToolPkg: registry });
  api.registerDomainCommands();
  assert.deepEqual(definitions.map(
    /** Compares declared root names without constructing caller command arrays. */
    definition => definition.name,
  ), roots);
  const fields = interfaceMembers("../../../../types/toolpkg.d.ts", "CoreCommandRegistration");
  for (const definition of definitions) {
    assert.deepEqual(Object.keys(definition).sort(), fields.sort());
    assert.equal(definition.id, definition.name); assert.equal(typeof definition.function, "function");
    const result = await definition.function(commandEvent(definition.name, ["help"]));
    assert.ok(result.stdout.length > 0); assert.ok(result.json.usage.length > 0);
    assert.equal(Object.hasOwn(result.json, "ok"), false);
  }
});

/** Requires public registrations to cover the typed plugin domain catalog without calling dependency APIs. */
test("every declared domain method is published as a provider registerApi callback", () => {
  const definitions = [], registry = { registerApi:
    /** Records a provider callback without emulating its business service. */
    definition => definitions.push(definition),
  };
  const api = loadModule("src/public-api.ts", { ToolPkg: registry });
  api.registerDomainApis();
  const declared = [...interfaceMembers("../src/api.ts", "DomainOperations"), "memory.query"].sort();
  const names = definitions.map(
    /** Preserves each stable published method name. */
    definition => definition.name,
  );
  assert.equal(new Set(names).size, names.length);
  assert.deepEqual([...names].sort(), declared);
  for (const definition of definitions) assert.deepEqual(Object.keys(definition).sort(), ["function", "name"]);
  for (const definition of definitions) assert.equal(typeof definition.function, "function");
});

/** Verifies command-provider parsing retains the complete record and creates a domain invocation, not a caller. */
test("full-record character update parses to one typed domain invocation", () => {
  const api = loadModule("src/commands.ts"), record = fixture().snapshot.cards[1];
  record.ttsConfigId = "tts-one";
  record.toolAccessConfig = { enabled: true, allowedBuiltinTools: ["builtin"], allowedPackages: ["package"], allowedSkills: ["skill"], allowedMcpServers: ["mcp"] };
  record.sharedMemoryMounts = [{ sharedMemoryId: "shared-main", readable: true, writable: false }];
  const parsed = api.parseCharacterCommand(["update", record.id, "--record", JSON.stringify(record)]);
  const { id, ...changes } = record;
  assert.equal(parsed.kind, "domain"); assert.equal(parsed.operation, "character.update");
  assert.deepEqual(plain(parsed.payload), { id, changes }); assert.equal(typeof parsed.execute, "function");
});

/** Verifies memory provider parsing keeps the exact selected owner and title-addressed repository query. */
test("memory command provider retains exact shared owner scope", () => {
  const api = loadModule("src/commands.ts");
  const parsed = api.parseMemoryCommand(["shared", "shared-main", "item", "search", "不需要子串命中"]);
  assert.equal(parsed.kind, "domain"); assert.equal(parsed.operation, "memory.search");
  assert.deepEqual(plain(parsed.payload), { ownerKey: "shared:shared-main", query: "不需要子串命中" });
  assert.throws(() => api.parseMemoryCommand(["character", "wrong:id", "graph"]), /character:<id> or shared:<id>/);
});

/** Verifies invalid host command events remain explicit failures in text and structured outputs. */
test("invalid command events report errors instead of empty successful results", async () => {
  const api = loadModule("src/commands.ts");
  const result = await api.onCharacterCommand(commandEvent("memory", ["list"]));
  assert.equal(result.json.ok, false); assert.equal(result.json.error.code, "invalid_arguments");
  assert.match(result.stderr, /does not match character/);
  const args = await api.onMemoryCommand(commandEvent("memory", ["shared", "shared-main", "graph", "trailing"]));
  assert.equal(args.json.ok, false); assert.equal(args.json.error.code, "invalid_arguments");
  assert.equal(Object.hasOwn(args.json, "items"), false);
});

/** Verifies four distinct route handlers and exact registered surfaces without launching UI or opening storage. */
test("one character-and-memory package registers native selection, embedded sidebar, management and attachment routes", () => {
  const routes = [], navigation = [], ipc = [];
  const registry = {
    /** Captures the route without constructing a browser host. */
    registerUiRoute: definition => routes.push(definition),
    /** Captures the exact host navigation surface. */
    registerNavigationEntry: definition => navigation.push(definition),
    ipc: {
      /** Captures the package-owned operation channel. */
      on: (name, handler) => ipc.push({ name, handler }),
    },
  };
  const api = loadModule("src/host.ts", { ToolPkg: registry });
  /** Provides only a named management screen callback for registration metadata inspection. */
  const screen = () => { throw new Error("Registration tests do not render production UI"); };
  /** Supplies a distinct attachment callback so its registration cannot accidentally reuse the management screen. */
  const attachmentScreen = () => { throw new Error("Registration tests do not render production attachment UI"); };
  /** Supplies the independently embedded sidebar handler instead of reusing the editor. */
  const sidebarScreen = () => { throw new Error("Registration tests do not render the embedded sidebar"); };
  /** Supplies the compact native selector handler instead of a management or attachment WebView. */
  const selectionScreen = () => { throw new Error("Registration tests do not render the native selector"); };
  /** Supplies a distinct native execution-controls screen for its actual route registration. */
  const groupExecutionScreen = () => { throw new Error("Registration tests do not render group controls"); };
  const memoryScreen = () => { throw new Error("Registration tests do not render memory UI"); };
  /** Supplies the independent embedded input-menu renderer. */
  const inputMenuScreen = () => { throw new Error("Registration tests do not render input-menu UI"); };
  assert.equal(api.register({ id: "com.operit.character_cards", title: "角色卡", icon: "Badge", order: 150 }, screen, attachmentScreen, sidebarScreen, selectionScreen, groupExecutionScreen, memoryScreen, inputMenuScreen), true);
  api.registerUiRequestChannel();
  const managementRoute = "toolpkg:com.operit.character_cards:ui:main";
  const attachmentRoute = "toolpkg:com.operit.character_cards:ui:memory-attachment";
  assert.equal(routes.length, 7);
  assert.deepEqual(plain(routes), [
    { id: "main", route: managementRoute, runtime: "compose_dsl", keepAlive: true, title: { zh: "角色卡", en: "Characters" } },
    { id: "memory-attachment", route: attachmentRoute, runtime: "compose_dsl", keepAlive: false, title: { zh: "记忆附件", en: "Memory attachment" } },
    { id: "chat-sidebar", route: "toolpkg:com.operit.character_cards:ui:chat-sidebar", runtime: "compose_dsl", keepAlive: true, title: { zh: "会话侧边栏", en: "Chat sidebar" } },
    { id: "chat-input-menu", route: "toolpkg:com.operit.character_cards:ui:chat-input-menu", runtime: "compose_dsl", keepAlive: false, title: { zh: "当前角色卡", en: "Current character" } },
    { id: "selection", route: "toolpkg:com.operit.character_cards:ui:selection", runtime: "compose_dsl", keepAlive: false, title: { zh: "切换角色卡", en: "Switch character" } },
    { id: "group-execution", route: "toolpkg:com.operit.character_cards:ui:group-execution", runtime: "compose_dsl", keepAlive: false, title: { zh: "群组执行", en: "Group execution" } },
    { id: "memory", route: "toolpkg:com.operit.character_cards:ui:memory", runtime: "compose_dsl", keepAlive: true, title: { zh: "记忆", en: "Memory" } },
  ]);
  assert.equal(routes[0].screen, screen); assert.equal(routes[1].screen, attachmentScreen);
  assert.equal(routes[2].screen, sidebarScreen); assert.equal(routes[3].screen, inputMenuScreen); assert.equal(routes[4].screen, selectionScreen); assert.equal(routes[5].screen, groupExecutionScreen); assert.equal(routes[6].screen, memoryScreen);
  assert.equal(new Set(routes.map(
    /** Requires five independent real screen callbacks, not aliases of the management page. */
    entry => entry.screen,
  )).size, 7);
  assert.equal(navigation.length, 6);
  assert.deepEqual(plain(navigation.filter(
    /** Requires exactly one directly embedded input-menu registration. */
    entry => entry.surface === "chat_input_menu",
  )), [{ id: "current-character", route: "toolpkg:com.operit.character_cards:ui:chat-input-menu", surface: "chat_input_menu", title: { zh: "当前角色卡", en: "Current character" }, order: 150, params: {} }]);
  assert.deepEqual(plain(navigation.filter(
    /** Validates the single legacy role tab and their exact opaque input on the shared registered sidebar route. */
    entry => entry.surface === "chat_sidebar_tabs",
  )), [
    { id: "sidebar-characters", route: "toolpkg:com.operit.character_cards:ui:chat-sidebar", surface: "chat_sidebar_tabs", title: { zh: "角色卡", en: "Characters" }, icon: "Badge", order: 150, params: { view: "characters" } },
  ]);
  const sidebar = navigation.filter(
    /** Requires exactly the host's sidebar extension surface. */
    definition => definition.surface === "main_sidebar_plugins",
  );
  assert.equal(sidebar.length, 2);
  assert.deepEqual(plain(sidebar[0]), { id: "sidebar", route: managementRoute, surface: "main_sidebar_plugins", title: { zh: "角色卡", en: "Characters" }, icon: "Badge", order: 150 });
  assert.deepEqual(plain(sidebar[1]), { id: "memory-settings", route: "toolpkg:com.operit.character_cards:ui:memory", surface: "main_sidebar_plugins", title: { zh: "记忆", en: "Memory" }, icon: "Memory", order: 151 });
  const toolbox = navigation.filter(
    /** Keeps the existing toolbox entry separate from attachment registration. */
    definition => definition.surface === "toolbox",
  );
  assert.equal(toolbox.length, 1); assert.equal(toolbox[0].route, managementRoute);
  const attachments = navigation.filter(
    /** Selects the formal plus-menu registration rather than a public API source. */
    definition => definition.surface === "chat_attachments",
  );
  assert.equal(attachments.length, 1);
  assert.deepEqual(plain(attachments[0]), { id: "memory-attachment", route: attachmentRoute, surface: "chat_attachments", title: { zh: "记忆附件", en: "Memory attachment" }, icon: "Memory", order: 150, params: { mode: "memory-attachment", ownerKey: null, folderPath: null } });
  assert.equal(Object.hasOwn(attachments[0], "action"), false);
  assert.deepEqual(ipc.map(
    /** Requires both current UI and domain channels exactly once, without a separate memory plugin. */
    entry => entry.name,
  ), ["character-memory.request", "character-memory.domain"]);
  assert.equal(ipc[0].handler, api.receiveUiRequest);
  assert.equal(ipc[1].handler, api.receiveDomainRequest);
});

/** Executes registered negative paths without installing production data, business globals, or a successful host. */
test("unconnected registered business providers surface dependency failures rather than fixture records", async () => {
  const definitions = [], registry = { registerApi:
    /** Captures genuine provider functions without implementing the missing persistence boundary. */
    definition => definitions.push(definition),
  };
  const api = loadModule("src/public-api.ts", { ToolPkg: registry });
  api.registerDomainApis();
  const provider = definitions.find(
    /** Selects the exact full-record catalog method registered by the actual plugin. */
    definition => definition.name === "character.list",
  );
  assert.notEqual(provider, undefined);
  await assert.rejects(provider.function({ payload: {} }),
    /** Requires a real dependency failure and never treats it as successful application loading. */
    error => typeof error.message === "string" && error.message.length > 0,
  );
  const commands = loadModule("src/commands.ts");
  const result = await commands.onCharacterCommand(commandEvent("character", ["list"]));
  assert.equal(result.json.ok, false);
  assert.equal(result.json.error.code, "domain_operation_failed");
  assert.equal(result.stderr.trim(), result.json.error.message);
  assert.ok(result.json.error.message.length > 0);
  assert.equal(Object.hasOwn(result.json, "items"), false);
  assert.equal(Object.hasOwn(result.json, "cards"), false);
});
