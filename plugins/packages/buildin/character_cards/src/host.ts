import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot } from "../../../../types/compose-dsl";
import type { Definition, Request, RequestOutput } from "./model";
import type { DomainOperation, DomainOutput } from "./api";
import { parseScreenResult } from "./presentation";
import { dispatch, dispatchDomain, initializeService } from "./service-runtime";
import { record } from "./domain";
import { createUiScreenSession } from "./ui-contributions";
import type { UiScreenSession, UiPresentation, DomainMessage } from "./ui-contributions";
import { parseDomainMessage } from "./ui-contributions";

/** Registers one package with character and memory views under a single navigation entry. */
export function register(definition: Definition, screen: (ctx: ComposeDslContext) => ComposeNode, attachmentScreen: (ctx: ComposeDslContext) => ComposeNode, sidebarScreen: (ctx: ComposeDslContext) => ComposeNode, selectionScreen: (ctx: ComposeDslContext) => ComposeNode, groupExecutionScreen: (ctx: ComposeDslContext) => ComposeNode): boolean {
  const route = `toolpkg:${definition.id}:ui:main`;
  ToolPkg.registerUiRoute({ id: "main", route, screen, runtime: "compose_dsl", keepAlive: true, title: { zh: definition.title, en: "Characters" } });
  const attachmentRoute = `toolpkg:${definition.id}:ui:memory-attachment`;
  ToolPkg.registerUiRoute({ id: "memory-attachment", route: attachmentRoute, screen: attachmentScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "记忆附件", en: "Memory attachment" } });
  const sidebarRoute = `toolpkg:${definition.id}:ui:chat-sidebar`;
  ToolPkg.registerUiRoute({ id: "chat-sidebar", route: sidebarRoute, screen: sidebarScreen, runtime: "compose_dsl", keepAlive: true, title: { zh: "会话侧边栏", en: "Chat sidebar" } });
  const selectionRoute = `toolpkg:${definition.id}:ui:selection`;
  ToolPkg.registerUiRoute({ id: "selection", route: selectionRoute, screen: selectionScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "切换角色卡", en: "Switch character" } });
  ToolPkg.registerUiRoute({ id: "group-execution", route: `toolpkg:${definition.id}:ui:group-execution`, screen: groupExecutionScreen, runtime: "compose_dsl", keepAlive: false, title: { zh: "群组执行", en: "Group execution" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar-characters", route: sidebarRoute, surface: "chat_sidebar_tabs", title: { zh: "角色分类", en: "Characters" }, icon: "Badge", order: definition.order, params: { view: "characters" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar-groups", route: sidebarRoute, surface: "chat_sidebar_tabs", title: { zh: "会话群组", en: "Conversation groups" }, icon: "Groups", order: definition.order + 1, params: { view: "groups" } });
  ToolPkg.registerNavigationEntry({ id: "sidebar", route, surface: "main_sidebar_plugins", title: { zh: definition.title, en: "Characters" }, icon: definition.icon, order: definition.order });
  ToolPkg.registerNavigationEntry({ id: "toolbox", route, surface: "toolbox", title: { zh: definition.title, en: "Characters" }, icon: definition.icon, order: definition.order });
  ToolPkg.registerNavigationEntry({ id: "memory-attachment", route: attachmentRoute, surface: "chat_attachments", title: { zh: "记忆附件", en: "Memory attachment" }, icon: "Memory", order: definition.order, params: { mode: "memory-attachment", ownerKey: null, folderPath: null } });
  return true;
}

/** Validates the finite UI envelope before delegating to the shared plugin service. */
export async function receiveUiRequest(value: unknown): Promise<RequestOutput> {
  const request = record(value, "character-memory.request");
  if (typeof request.action !== "string" || request.action.trim() === "") throw new Error("character-memory.request.action must be a nonblank string");
  return dispatch(request as Request);
}

/** Decodes the WebView controller's documented nested argument-array contract. */
function webArguments(value: readonly unknown[], expected: number): unknown[] {
  if (value.length !== 1 || !Array.isArray(value[0]) || value[0].length !== expected) throw new Error(`CharacterMemoryHost expects ${expected} arguments`);
  return value[0];
}

/** Validates memory-tool and UI domain IPC using the service's exact operation/input contract. */
export async function receiveDomainRequest(value: unknown): Promise<DomainOutput<DomainOperation>> {
  const message = parseDomainMessage(value);
  return dispatchDomain(message.operation, message.input);
}

/** Declares the UI operation channel without initializing the business store or migration. */
export function registerUiRequestChannel(): void {
  ToolPkg.ipc.on<Request, RequestOutput>("character-memory.request", receiveUiRequest);
  ToolPkg.ipc.on<DomainMessage, DomainOutput<DomainOperation>>("character-memory.domain", receiveDomainRequest);
}

/** Keeps the existing channel-registration export without performing business initialization. */
export const installService = registerUiRequestChannel;

/** Declares a lifecycle callback that initializes only when the host dispatches the event. */
export function registerServiceLifecycle(): void {
  ToolPkg.registerAppLifecycleHook({ id: "service-initialize", event: "application_on_create", function: onServiceInitialize });
}

/** Shares the retained first-call initializer with the actual application lifecycle. */
export async function onServiceInitialize(_event: ToolPkg.AppLifecycleHookEvent): Promise<void> {
  await initializeService();
}
/** Uses the workflow plugin's offline WebView pattern with the application's Material palette. */
export function renderScreen(ctx: ComposeDslContext, definition: Definition, requiredMode: "memory-attachment" | null = null): ComposeNode {
  const controller = ctx.createWebViewController("character-memory-web");
  const [path, setPath] = ctx.useState("character-memory-html", "");
  const [error, setError] = ctx.useState("character-memory-error", "");
  const ready = ctx.useRef("character-memory-ready", false);
  const unsubscribe = ctx.useRef<(() => void) | null>("character-memory-theme-subscription", null);
  const [presentation] = ctx.useState<UiPresentation | null>("presentation", null);
  const session = ctx.useRef<UiScreenSession | null>("character-memory-screen-session", null);
  if (session.current === null) session.current = createUiScreenSession(presentation);
  const screenSession = session.current;
  if (requiredMode !== null && screenSession.currentScreen().input.mode !== requiredMode) throw new Error("The attachment route requires an explicit attachment presentation");
  const origin = "https://characters.operit.local/";
  /** Sends host theme changes through the plugin-owned document contract. */
  async function applyTheme(theme: ComposeThemeSnapshot): Promise<void> {
    if (ready.current) await controller.evaluateJavascript(`window.applyCharacterMemoryTheme(${JSON.stringify(theme)});`);
  }
  /** Publishes the sole HTML bridge before loading the package resource. */
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("CharacterMemoryHost", {
        /** Marks the document ready and returns the actual host palette. */
        currentTheme: () => { ready.current = true; return ctx.Theme.getCurrent(); },
        /** Routes all editing operations through this package's main runtime. */
        request: async (...args: unknown[]) => {
          const [request] = webArguments(args, 1);
          const decoded = record(request, "character-memory.request") as Request;
          return ToolPkg.ipc.call<Request, RequestOutput>("character-memory.request", decoded, { targetRuntime: "main" });
        },
        /** Returns the readonly plugin screen input and its real host presentation request id. */
        currentScreen: (...args: unknown[]) => {
          webArguments(args, 0);
          return screenSession.currentScreen();
        },
        /** Returns an explicit completion action result for this exact presented screen. */
        completeScreen: (...args: unknown[]) => {
          const [result] = webArguments(args, 1);
          const current = screenSession.currentScreen();
          if (current.input.mode === "manage") throw new Error("Management has no presentation completion channel");
          return screenSession.completeScreen(parseScreenResult(result, current.input));
        },
        /** Returns an explicit cancellation action result without changing the selected identity. */
        cancelScreen: (...args: unknown[]) => {
          webArguments(args, 0);
          return screenSession.cancelScreen();
        },
        /** Saves an export to the VFS path explicitly supplied by the user. */
        exportFile: async (...args: unknown[]) => {
          const [path, content] = webArguments(args, 2);
          if (typeof path !== "string" || path.trim() === "") throw new Error("请输入导出文件的 VFS 路径");
          if (typeof content !== "string") throw new Error("导出内容必须是字符串");
          await Tools.Files.create(path, content);
          return true;
        },
      });
      unsubscribe.current = ctx.Theme.subscribe(applyTheme);
      setPath(await ToolPkg.readResource("character_memory_html", "character-memory.html"));
    } catch (failure) { setError(String(failure)); }
  }
  /** Releases this instance's subscription when its embedded surface is disposed. */
  function dispose(): void {
    ready.current = false;
    if (unsubscribe.current !== null) { unsubscribe.current(); unsubscribe.current = null; }
  }
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, path === "" ? ctx.UI.Text({ text: error === "" ? `正在加载${definition.title}…` : error }) : ctx.UI.WebView({
    key: "character-memory-web", controller, fillMaxSize: true, url: origin,
    javaScriptEnabled: true, domStorageEnabled: true, supportZoom: false, useWideViewPort: true,
    /** Restricts top-level navigation to the package document. */
    onShouldOverrideUrlLoading: request => request.url === origin ? { action: "allow" } : { action: "cancel" },
    /** Serves the single offline asset through the existing cross-platform resource host. */
    onInterceptRequest: request => request.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
    /** Handles disposal without retaining the screen in the host theme service. */
    onLifecycleEvent: event => { if (event.type === "Disposed") dispose(); },
  }));
}
