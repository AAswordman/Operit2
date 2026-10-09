import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../../../../../", import.meta.url));
const ui = "apps/flutter/app/lib/ui";

/** Reads an exact inspected Dart production path instead of searching retained reference UI. */
function source(relative) { return readFileSync(path.join(root, relative), "utf8"); }

/** Removes comments and string bodies before checking actual named Dart invocations. */
function code(relative) {
  return source(relative).replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
    /** Keeps line numbers while preventing explanatory text from being treated as a live call. */
    token => token.replace(/[^\r\n]/g, " "),
  );
}

/** Returns the first actual exact-symbol invocation in the inspected Dart source. */
function invocation(text, symbol) {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(symbol)) throw new Error("Expected an exact Dart identifier");
  const escaped = symbol;
  const match = new RegExp("\\b" + escaped + "\\s*\\(").exec(text);
  return match === null ? null : text.slice(0, match.index).split("\n").length;
}

/** Reports only old UI business calls reachable through the named actual chat-screen invocation chain. */
export function uiProductionViolations() {
  const violations = [], factory = ui + "/main/screens/OperitScreens.dart";
  const sidebar = sidebarViolationsFromSources(source);
  if (invocation(code(factory), "AIChatScreen") === null) return sidebar;
  const chat = ui + "/features/chat", screen = chat + "/screens/AIChatScreen.dart";
  const attachment = chat + "/components/attachments/MemoryAttachmentDialog.dart";
  if (/import\s+['"][^'"]*MemoryAttachmentDialog\.dart['"]/.test(source(screen)) && invocation(code(screen), "MemoryAttachmentDialog") !== null) {
    const text = code(attachment), hits = [];
    for (const method of ["getAllCharacterCards", "getAllSharedMemoryStores", "repositoryMemoryRepositoryForOwner"]) {
      const line = invocation(text, method);
      if (line !== null) hits.push(method + " at line " + line);
    }
    if (hits.length > 0) violations.push(attachment + ": mounted by " + screen + ", attachment still reads old Core character/memory sources: " + hits.join(", "));
  }
  const popup = chat + "/components/style/input/agent/AgentInputMenuPopup.dart";
  const sections = [chat + "/components/style/input/agent/AgentChatInputSection.dart", chat + "/components/style/input/classic/ClassicChatInputSection.dart"];
  const popupMounted = sections.some(
    /** Requires the existing section to import and instantiate this exact popup widget. */
    file => /import\s+['"][^'"]*AgentInputMenuPopup\.dart['"]/.test(source(file)) && invocation(code(file), "AgentInputMenuPopup") !== null,
  );
  if (popupMounted && invocation(code(popup), "_CharacterCardSelectorDialog") !== null) {
    const text = code(popup), hits = [];
    for (const method of ["chatCharacterCards", "switchChatCharacterCardTarget"]) {
      const line = invocation(text, method);
      if (line !== null) hits.push(method + " at line " + line);
    }
    if (hits.length > 0) violations.push(popup + ": mounted agent/classic popup still launches the old selector and Core domain bindings: " + hits.join(", "));
  }
  return [...violations, ...sidebar];
}

const sidebarPaths = {
  main: ui + "/main/screens/OperitMainScreen.dart",
  phone: ui + "/main/layout/PhoneLayout.dart",
  tablet: ui + "/main/layout/TabletLayout.dart",
  drawer: ui + "/main/components/DrawerContent.dart",
  state: ui + "/main/components/DrawerConversationState.dart",
  host: ui + "/common/contributions/ChatSidebarTabHost.dart",
};

/** Strips comment and string bodies while preserving offsets for exact live Dart-symbol checks. */
function semanticCode(text) {
  return text.replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g,
    /** Preserves positions without treating labels, imports or explanatory comments as business execution. */
    token => token.replace(/[^\r\n]/g, " "),
  );
}

/** Finds an exact live identifier without classifying human-readable labels or substrings as domain code. */
function identifier(text, name) {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) throw new Error("Expected an exact Dart identifier");
  const match = new RegExp("\\b" + name + "\\b").exec(text);
  return match === null ? null : text.slice(0, match.index).split("\n").length;
}

/** Reads balanced delimiters in comment-free source so nested actual calls cannot satisfy an unrelated signature. */
function closingDelimiter(text, opening, left, right) {
  let depth = 1;
  for (let index = opening + 1; index < text.length; index += 1) {
    if (text[index] === left) depth += 1;
    if (text[index] === right) depth -= 1;
    if (depth === 0) return index;
  }
  throw new Error("Unbalanced inspected Dart source at offset " + opening);
}

/** Extracts exact named call argument spans and preserves raw values for locked literal contract checks. */
function calls(raw, symbol) {
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(symbol)) throw new Error("Expected an exact Dart call identifier");
  const text = semanticCode(raw), pattern = new RegExp("\\b" + symbol + "\\s*\\(", "g"), result = [];
  let match;
  while ((match = pattern.exec(text)) !== null) {
    const opening = match.index + match[0].length - 1, end = closingDelimiter(text, opening, "(", ")");
    result.push({ raw: raw.slice(opening + 1, end), text: text.slice(opening + 1, end), end });
  }
  return result;
}

/** Reads a real method body rather than accepting call-shaped text in a declaration or an unmounted helper. */
function methodBody(raw, name) {
  const text = semanticCode(raw), bodies = [];
  for (const call of calls(raw, name)) {
    const rest = text.slice(call.end + 1), prefix = /^\s*(?:async\s*)?\{/.exec(rest);
    if (prefix === null) continue;
    const opening = call.end + prefix[0].length, end = closingDelimiter(text, opening, "{", "}");
    bodies.push(raw.slice(opening + 1, end));
  }
  if (bodies.length !== 1) throw new Error("Expected one actual Dart method body for " + name + ", found " + bodies.length);
  return bodies[0];
}

/** Restricts mounted methods to their actual widget State class so unrelated local widgets cannot satisfy the chain. */
function classBody(raw, name) {
  const text = semanticCode(raw), match = new RegExp("\\bclass\\s+" + name + "\\b").exec(text);
  if (match === null) throw new Error("Missing actual mounted Dart State class: " + name);
  const opening = text.indexOf("{", match.index), end = closingDelimiter(text, opening, "{", "}");
  return raw.slice(opening + 1, end);
}

/** Requires the mounted widget call to carry each exact live argument, never a null or empty fixture collection. */
function wiredCall(raw, name, required) {
  return calls(raw, name).some(
    /** Checks all required arguments on the same actual call instead of unrelated declarations. */
    call => required.every(
      /** Uses contract-specific exact named expressions without interpreting opaque registration fields. */
      ([field, expression]) => new RegExp("\\b" + field + "\\s*:\\s*(?:" + expression + ")\\s*(?:,|$)").test(call.text),
    ),
  );
}

/** Requires an actual import for the mounted constructor, ignoring commented-out import examples. */
function imports(raw, name) {
  const withoutComments = raw.replace(/\/\/[^\r\n]*|\/\*[\s\S]*?\*\//g, "");
  return new RegExp("import\\s+['\"][^'\"]*" + name + "\\.dart['\"]").test(withoutComments);
}

/** Follows the explicit live main/layout/drawer/tab-host source chain; fixture readers prove only this static checker. */
export function sidebarViolationsFromSources(readSource) {
  if (typeof readSource !== "function") throw new Error("Sidebar checker requires an explicit source reader");
  const files = new Map(Object.entries(sidebarPaths).map(
    /** Reads each exact production consumer, including the actual catalog owner rather than expecting needless prop forwarding. */
    ([role, file]) => { const raw = readSource(file); return [role, { file, raw, text: semanticCode(raw) }]; },
  ));
  const violations = [], forbidden = [
    "preferencesCharacterCardManager", "preferencesCharacterGroupCardManager", "preferencesActivePromptManager", "preferencesSharedMemoryStoreManager",
    "getAllCharacterCards", "getAllCharacterGroupCards", "allCharacterGroupCardsFlow", "characterCardListFlow", "getCharacterCardFlow", "getCharacterCard",
    "readActivePrompt", "activePromptFlow", "CreateGroupDialog", "CharacterAvatar", "CharacterCard", "CharacterGroupCard", "ActivePromptCharacterCard", "ActivePromptCharacterGroup",
    "characterCardName", "characterCardId", "characterGroupId", "characterGroupNamesById", "characterCardAvatarUrisByName",
  ];
  for (const entry of files.values()) {
    for (const name of forbidden) {
      const line = identifier(entry.text, name);
      if (line !== null) violations.push(entry.file + ": mounted main sidebar still consumes " + name + " at line " + line);
    }
  }
  const main = files.get("main"), drawer = files.get("drawer"), host = files.get("host");
  let mainBuild, drawerBuild, catalog, content, activation;
  try {
    mainBuild = methodBody(classBody(main.raw, "_OperitMainScreenState"), "build"); drawerBuild = methodBody(classBody(drawer.raw, "_DrawerContentState"), "build");
    catalog = methodBody(host.raw, "_readCatalog"); content = methodBody(host.raw, "_content"); activation = methodBody(host.raw, "_handleResult");
  } catch (error) {
    violations.push("Sidebar production rendering/catalog method is missing or ambiguous: " + error.message);
    return violations;
  }
  const liveReference = "[A-Za-z_][A-Za-z0-9_]*(?:\\.[A-Za-z_][A-Za-z0-9_]*)*";
  for (const [role, widget] of [["phone", "PhoneLayout"], ["tablet", "TabletLayout"]]) {
    const layout = files.get(role);
    if (!imports(main.raw, widget) || !wiredCall(mainBuild, widget, [["drawerConversationState", liveReference], ["onConversationActivated", "_activateConversationRoute"]])) {
      violations.push(main.file + ": actual build must mount " + widget + " with conversation state and activation callback");
    }
    let layoutBuild;
    try { layoutBuild = methodBody(classBody(layout.raw, role === "phone" ? "_PhoneLayoutState" : "_TabletLayoutState"), "build"); }
    catch (error) { violations.push(layout.file + ": " + error.message); continue; }
    if (!imports(layout.raw, "DrawerContent") || !wiredCall(layoutBuild, "DrawerContent", [
      ["histories", "drawerState\\.histories"], ["currentChatId", "drawerState\\.currentChatId"],
      ["activeStreamingChatIds", "drawerState\\.activeStreamingChatIds"], ["onConversationActivated", role === "phone" ? "_handleConversationActivated" : "widget\\.onConversationActivated"],
    ])) violations.push(layout.file + ": actual build must mount DrawerContent with generic chat summaries and activation");
    if (role === "phone") {
      try {
        if (!/\bwidget\.onConversationActivated\s*\(/.test(semanticCode(methodBody(layout.raw, "_handleConversationActivated")))) {
          violations.push(layout.file + ": drawer activation wrapper must invoke the supplied main activation callback");
        }
      } catch (error) { violations.push(layout.file + ": " + error.message); }
    }
  }
  if (!imports(drawer.raw, "ChatSidebarTabHost") || !wiredCall(drawerBuild, "ChatSidebarTabHost", [
    ["clients", "_clients"], ["chats", "widget\\.histories"], ["currentChatId", "widget\\.currentChatId"],
    ["activeStreamingChatIds", "widget\\.activeStreamingChatIds"], ["workspaceBuilder", "_workspaceContent"], ["onActivateChat", "_activateChat"],
  ])) violations.push(drawer.file + ": actual build must mount ChatSidebarTabHost with runtime clients, generic chats, workspace and activation");
  if (!calls(drawer.raw, "GeneratedCoreProxyClients").some(
    /** Requires the receiving runtime bridge, not a detached client or successful test host. */
    call => /^\s*widget\.bridge\s*$/.test(call.text),
  )) violations.push(drawer.file + ": tab clients must use the actual receiving runtime bridge");
  if (identifier(drawer.text, "workspaceId") === null) violations.push(drawer.file + ": workspace projection must retain real generic workspace identity");
  try {
    const activate = semanticCode(methodBody(drawer.raw, "_activateChat"));
    if (invocation(activate, "switchChat") === null || !/\bwidget\.onConversationActivated\s*\(/.test(activate)) {
      violations.push(drawer.file + ": tab activation must execute the existing switchChat and navigation/close chain");
    }
  } catch (error) { violations.push(drawer.file + ": " + error.message); }
  const catalogCode = semanticCode(catalog), contentCode = semanticCode(content), activationCode = semanticCode(activation);
  if (invocation(catalogCode, "getToolPkgNavigationEntries") === null || invocation(catalogCode, "getToolPkgUiRoutes") === null || invocation(catalogCode, "getToolPkgContainerRuntime") === null) {
    violations.push(host.file + ": _readCatalog must resolve real enabled navigation, Compose routes and their current owner");
  }
  if (!/\bentry\.surface\s*!=\s*'chat_sidebar_tabs'/.test(catalog) || !/\bruntime\s*:\s*'compose_dsl'/.test(catalog)) {
    violations.push(host.file + ": catalog must select exact chat_sidebar_tabs and compose_dsl contracts");
  }
  for (const guard of [
    /\broute\.containerPackageName\s*==\s*entry\.containerPackageName/, /\broute\.routeId\s*==\s*entry\.routeId/,
    /\bmatches\.length\s*!=\s*1/, /\bentry\.action\s*!=\s*null/, /\bplugin\s*==\s*null/,
  ]) if (!guard.test(catalogCode)) violations.push(host.file + ": actual catalog is missing owner/route/action/disabled validation: " + guard.source);
  if (invocation(catalogCode, "StateError") === null) violations.push(host.file + ": invalid registered owners and routes must produce explicit errors");
  if (!imports(host.raw, "ToolPkgUiLauncherScreen") || !wiredCall(content, "ToolPkgUiLauncherScreen", [
    ["clients", "widget\\.clients"], ["plugin", "tab\\.plugin"], ["initialRouteId", "tab\\.route\\.routeId"],
    ["embeddedScreenPath", "tab\\.route\\.screen"], ["showLauncherChrome", "false"], ["initialModuleSpec", "tab\\.route\\.moduleSpec"],
  ])) violations.push(host.file + ": registered routes must embed the actual ToolPkgUiLauncherScreen with showLauncherChrome:false");
  if (!/['"]input['"]\s*:\s*tab\.entry\.params/.test(content) || !/\bwidget\.chats\.map\s*\(\s*chatSidebarSummary\s*\)/.test(contentCode)) {
    violations.push(host.file + ": embedded route input must preserve opaque params and the actual generic chat summary callback");
  }
  if (!/\bwidget\.workspaceBuilder\s*\(\s*context\s*\)/.test(contentCode)) violations.push(host.file + ": the native workspace tab must invoke its supplied workspace builder");
  if (invocation(activationCode, "chatSidebarActivation") === null || invocation(activationCode, "_readCatalog") === null || invocation(activationCode, "chatHistoryListItemsFlow") === null || !/\bawait\s+widget\.onActivateChat\s*\(\s*chatId\s*\)/.test(activationCode)) {
    violations.push(host.file + ": activation must validate the protocol, current catalog and chat existence before awaiting the host callback");
  }
  try {
    const reload = semanticCode(methodBody(host.raw, "_reloadCatalog"));
    let catalogIsLoaded = invocation(reload, "_readCatalog") !== null;
    if (!catalogIsLoaded && invocation(reload, "_loadCatalog") !== null) {
      const loader = semanticCode(methodBody(host.raw, "_loadCatalog"));
      catalogIsLoaded = invocation(loader, "_readCatalog") !== null;
    }
    if (!catalogIsLoaded || invocation(semanticCode(methodBody(classBody(host.raw, "_ChatSidebarTabHostState"), "build")), "_content") === null) {
      violations.push(host.file + ": catalog and embedded route helpers must be called by the actual host rendering path");
    }
  } catch (error) { violations.push(host.file + ": " + error.message); }
  return violations;
}

/** Audits the real sidebar independently of the deleted attachment dialog or whether AIChatScreen is mounted. */
export function sidebarProductionViolations() { return sidebarViolationsFromSources(source); }
