import assert from "node:assert/strict";
import test from "node:test";
import { uiProductionViolations, sidebarProductionViolations, sidebarViolationsFromSources } from "./ui-consumers.mjs";

const base = "apps/flutter/app/lib/ui/main/";
const paths = {
  main: base + "screens/OperitMainScreen.dart", phone: base + "layout/PhoneLayout.dart", tablet: base + "layout/TabletLayout.dart",
  drawer: base + "components/DrawerContent.dart", state: base + "components/DrawerConversationState.dart",
  host: "apps/flutter/app/lib/ui/common/contributions/ChatSidebarTabHost.dart",
};

/** Supplies source-text-only checker inputs, not running Flutter widgets, native hosts or a substitute registration catalog. */
function sourceFixture() {
  return new Map([
    [paths.main, `import '../layout/PhoneLayout.dart'; import '../layout/TabletLayout.dart';
      class _OperitMainScreenState { Widget build(context) { return children(PhoneLayout(drawerConversationState: _state, onConversationActivated: _activateConversationRoute), TabletLayout(drawerConversationState: _state, onConversationActivated: _activateConversationRoute)); } }`],
    [paths.phone, `import '../components/DrawerContent.dart'; class _PhoneLayoutState {
      Widget build(context) { return DrawerContent(histories: drawerState.histories, currentChatId: drawerState.currentChatId, activeStreamingChatIds: drawerState.activeStreamingChatIds, onConversationActivated: _handleConversationActivated); }
      void _handleConversationActivated() { widget.onConversationActivated(); } }`],
    [paths.tablet, `import '../components/DrawerContent.dart'; class _TabletLayoutState {
      Widget build(context) { return DrawerContent(histories: drawerState.histories, currentChatId: drawerState.currentChatId, activeStreamingChatIds: drawerState.activeStreamingChatIds, onConversationActivated: widget.onConversationActivated); } }`],
    [paths.drawer, `import '../../common/contributions/ChatSidebarTabHost.dart'; class _DrawerContentState {
      GeneratedCoreProxyClients get _clients => GeneratedCoreProxyClients(widget.bridge);
      Widget _workspaceContent(context) { final workspaceId = history.workspaceId; return workspace; }
      Future<void> _activateChat(String chatId) async { widget.onConversationActivated(); await core.switchChat(chatId: chatId); }
      Widget build(context) { return ChatSidebarTabHost(clients: _clients, chats: widget.histories, currentChatId: widget.currentChatId, activeStreamingChatIds: widget.activeStreamingChatIds, workspaceBuilder: _workspaceContent, onActivateChat: _activateChat); } }`],
    [paths.state, "class DrawerConversationState { final List<ChatHistoryListItem> histories; }"],
    [paths.host, `import '../../features/packages/screens/ToolPkgUiLauncherScreen.dart'; class _ChatSidebarTabHostState {
      Future<List> _readCatalog() async {
        final entries = await manager.getToolPkgNavigationEntries(useEnglish: true);
        final routes = await manager.getToolPkgUiRoutes(runtime: 'compose_dsl', useEnglish: true);
        for (final entry in entries) {
          if (entry.surface != 'chat_sidebar_tabs') continue;
          final matches = routes.where((route) => route.containerPackageName == entry.containerPackageName && route.routeId == entry.routeId);
          if (matches.length != 1 || entry.action != null) throw StateError('invalid route');
          final plugin = await manager.getToolPkgContainerRuntime(containerPackageName: entry.containerPackageName);
          if (plugin == null) throw StateError('disabled');
        }
        return tabs;
      }
      void _reloadCatalog() { _tabsFuture = _readCatalog(); }
      Future<void> _handleResult(raw, tab, generation) async {
        final chatId = chatSidebarActivation(raw); final catalog = await _readCatalog();
        final chats = await clients.chatRuntimeHolderMain.chatHistoryListItemsFlow().first;
        await widget.onActivateChat(chatId);
      }
      Widget _content(tabs) {
        if (workspaceSelected) return widget.workspaceBuilder(context);
        return ToolPkgUiLauncherScreen(clients: widget.clients, plugin: tab.plugin, initialRouteId: tab.route.routeId,
          embeddedScreenPath: tab.route.screen, showLauncherChrome: false, initialModuleSpec: tab.route.moduleSpec,
          initialState: {'input': tab.entry.params, 'chats': widget.chats.map(chatSidebarSummary).toList()});
      }
      Widget build(context) { return _content(tabs); }
    }`],
  ]);
}

/** Requires each explicit test source instead of fabricating empty source for an undeclared path. */
function inspect(sources) {
  return sidebarViolationsFromSources(
    /** Rejects any inspected path that the static checker fixture has not explicitly declared. */
    file => { assert.ok(sources.has(file), "Undeclared source fixture path: " + file); return sources.get(file); },
  );
}

/** Makes exact adversarial edits to one checker fixture without altering any production consumer. */
function replaced(sources, file, before, after) {
  const original = sources.get(file); assert.notEqual(original.indexOf(before), -1, "Missing exact static fixture edit");
  sources.set(file, original.replace(before, after)); return sources;
}

/** Keeps all mounted popup, attachment and sidebar business dependencies red, independent of deleted reference files. */
test("mounted production UI no longer launches old Core business consumers", () => {
  const violations = uiProductionViolations();
  assert.deepEqual(violations, [], "UI production extraction is incomplete:\n" + violations.join("\n"));
});

/** Audits the actual Main-to-layout-to-Drawer-to-tab-host chain with catalog discovery owned by the real tab host. */
test("real sidebar embeds registered Compose routes beside workspace without old role or group consumers", () => {
  const violations = sidebarProductionViolations();
  assert.deepEqual(violations, [], "Sidebar production extraction is incomplete:\n" + violations.join("\n"));
});

/** Does not require needless registeredTabs props when the mounted host genuinely owns catalog discovery. */
test("sidebar static checker accepts host-owned catalog loading without Main layout registeredTabs forwarding", () => {
  assert.deepEqual(inspect(sourceFixture()), []);
});

/** Detects exact old manager calls across every explicitly mounted file, including the actual catalog owner. */
test("sidebar static checker rejects legacy managers in main phone tablet drawer state and tab host", () => {
  for (const file of Object.values(paths)) {
    const sources = sourceFixture(); sources.set(file, sources.get(file) + " void oldPath() { clients.preferencesCharacterCardManager.getAllCharacterCards(); }");
    const violations = inspect(sources);
    assert.ok(violations.some(
      /** Requires the diagnostic to name the exact mounted file and manager member. */
      value => value.startsWith(file + ":") && /preferencesCharacterCardManager/.test(value),
    ), file + " must not escape mounted sidebar inspection");
  }
});

/** Ignores human labels and comments but rejects actual role interpretation and the old mounted group dialog. */
test("sidebar static checker distinguishes live role fields and group dialogs from inert reference text", () => {
  const inert = sourceFixture();
  inert.set(paths.drawer, inert.get(paths.drawer) + " // CreateGroupDialog() and history.characterGroupId are references only\n const label = 'CharacterCard characterCardName CreateGroupDialog()';");
  assert.deepEqual(inspect(inert), []);
  for (const statement of ["final id = history.characterGroupId;", "final name = history.characterCardName;", "CreateGroupDialog();"]) {
    const sources = sourceFixture(); sources.set(paths.drawer, sources.get(paths.drawer) + statement);
    assert.notDeepEqual(inspect(sources), [], "Live domain statement must remain red: " + statement);
  }
});

/** Requires real live constructor calls, so removing an import or placing the widget in an unrelated helper is not success. */
test("sidebar static checker rejects disconnected main layout drawer and tab host constructors", () => {
  for (const [file, before, after] of [
    [paths.main, "PhoneLayout(drawerConversationState:", "OtherLayout(drawerConversationState:"],
    [paths.main, "TabletLayout(drawerConversationState:", "OtherLayout(drawerConversationState:"],
    [paths.phone, "DrawerContent(histories:", "OtherDrawer(histories:"],
    [paths.tablet, "DrawerContent(histories:", "OtherDrawer(histories:"],
    [paths.drawer, "ChatSidebarTabHost(clients:", "OtherHost(clients:"],
    [paths.drawer, "import '../../common/contributions/ChatSidebarTabHost.dart';", "// import '../../common/contributions/ChatSidebarTabHost.dart';\n"],
  ]) assert.notDeepEqual(inspect(replaced(sourceFixture(), file, before, after)), [], "Broken actual mount must fail: " + file);
});

/** Keeps native clients, summaries and activation meaningful rather than accepting a mounted but unconnected tab host. */
test("sidebar static checker rejects missing clients empty summaries and disconnected activation", () => {
  for (const [file, before, after] of [
    [paths.main, "onConversationActivated: _activateConversationRoute", "onConversationActivated: unrelatedCallback"],
    [paths.phone, "histories: drawerState.histories", "histories: const []"],
    [paths.drawer, "GeneratedCoreProxyClients(widget.bridge)", "GeneratedCoreProxyClients(detachedBridge)"],
    [paths.drawer, "chats: widget.histories", "chats: const []"],
    [paths.drawer, "onActivateChat: _activateChat", "onActivateChat: unrelatedCallback"],
    [paths.drawer, "await core.switchChat(chatId: chatId)", "await unrelatedCall(chatId)"],
    [paths.host, "await widget.onActivateChat(chatId)", "await unrelatedCallback(chatId)"],
  ]) assert.notDeepEqual(inspect(replaced(sourceFixture(), file, before, after)), [], "Disconnected live data or activation must fail: " + file);
});

/** Requires actual enabled navigation and Compose routes plus current owner lookup inside the called catalog method. */
test("sidebar static checker rejects fake or uncalled catalog loading", () => {
  for (const [before, after] of [
    ["manager.getToolPkgNavigationEntries(useEnglish: true)", "fixtureEntries()"],
    ["manager.getToolPkgUiRoutes(runtime: 'compose_dsl', useEnglish: true)", "fixtureRoutes()"],
    ["manager.getToolPkgContainerRuntime(containerPackageName: entry.containerPackageName)", "fixtureOwner()"],
    ["entry.surface != 'chat_sidebar_tabs'", "entry.surface != 'main_sidebar_plugins'"],
    ["_tabsFuture = _readCatalog()", "_tabsFuture = fixtureTabs()"],
  ]) assert.notDeepEqual(inspect(replaced(sourceFixture(), paths.host, before, after)), [], "Fake catalog path must fail: " + before);
});

/** Rejects foreign routes, ambiguous owners, action callbacks and disabled packages instead of hiding consumer errors. */
test("sidebar static checker requires exact owner unique route action and disabled validation", () => {
  for (const [before, after] of [
    ["route.containerPackageName == entry.containerPackageName", "true"],
    ["route.routeId == entry.routeId", "true"],
    ["matches.length != 1", "matches.length == 0"],
    ["entry.action != null", "false"],
    ["plugin == null", "false"],
  ]) assert.notDeepEqual(inspect(replaced(sourceFixture(), paths.host, before, after)), [], "Missing registration guard must fail: " + before);
});

/** Verifies actual chrome-free route embedding and raw opaque input rather than an app-owned projected pseudo-tab. */
test("sidebar static checker rejects fake embedding launcher chrome discarded params and missing workspace", () => {
  for (const [before, after] of [
    ["return ToolPkgUiLauncherScreen(", "return FakeTab("],
    ["initialRouteId: tab.route.routeId", "initialRouteId: otherRoute"],
    ["showLauncherChrome: false", "showLauncherChrome: true"],
    ["'input': tab.entry.params", "'input': {}"],
    ["widget.chats.map(chatSidebarSummary)", "fixtureChatSummaries()"],
    ["return widget.workspaceBuilder(context)", "return fakeWorkspace()"],
    ["return _content(tabs)", "return fakeContent()"],
  ]) assert.notDeepEqual(inspect(replaced(sourceFixture(), paths.host, before, after)), [], "Nonembedded or disconnected content must fail: " + before);
});

/** Makes a dead helper and commented constructor insufficient to prove a real Main build mount. */
test("sidebar static checker does not accept disconnected same-name widget calls as production mounts", () => {
  const sources = replaced(sourceFixture(), paths.main, "PhoneLayout(drawerConversationState: _state, onConversationActivated: _activateConversationRoute)", "OtherLayout()");
  sources.set(paths.main, sources.get(paths.main) + " void unmountedExample() { PhoneLayout(drawerConversationState: _state, onConversationActivated: _activateConversationRoute); }\n // PhoneLayout(drawerConversationState: _state, onConversationActivated: _activateConversationRoute)");
  assert.notDeepEqual(inspect(sources), []);
});

/** Scans no retained migration-reference file unless it is in the explicit current production chain. */
test("sidebar static checker keeps unmounted migration reference files outside the production verdict", () => {
  const sources = sourceFixture(); sources.set(base + "reference/LegacyDrawer.dart", "CreateGroupDialog(); preferencesCharacterCardManager.getAllCharacterCards();");
  assert.deepEqual(inspect(sources), []);
});
