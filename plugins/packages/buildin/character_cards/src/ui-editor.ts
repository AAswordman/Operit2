import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot } from "../../../../types/compose-dsl";
import type { Definition, Request, RequestOutput, Snapshot } from "./model";
import { createAvatarResources } from "./avatar-resources";
import { renderManagedEntityScreen, type ManagedEntityRequest } from "./ui-entity-native";
import { parseScreenResult, type ScreenBootstrap } from "./presentation";
import { record } from "./domain";
import { createUiScreenSession } from "./ui-contributions";
import type { UiScreenSession, UiPresentation } from "./ui-contributions";

/** Decodes the WebView controller's documented nested argument-array contract. */
function webArguments(value: readonly unknown[], expected: number): unknown[] {
  if (value.length !== 1 || !Array.isArray(value[0]) || value[0].length !== expected) throw new Error(`CharacterMemoryHost expects ${expected} arguments`);
  return value[0];
}

/** Uses the workflow plugin's offline WebView pattern with the application's Material palette. */
export function renderScreen(ctx: ComposeDslContext, definition: Definition, requiredMode: "memory-attachment" | null = null, managementView: "characters" | "memory" = "characters"): ComposeNode {
  const controller = ctx.createWebViewController("character-memory-web");
  const [path, setPath] = ctx.useState("character-memory-html", "");
  const [error, setError] = ctx.useState("character-memory-error", "");
  const ready = ctx.useRef("character-memory-ready", false);
  const unsubscribe = ctx.useRef<(() => void) | null>("character-memory-theme-subscription", null);
  const [presentation] = ctx.useState<UiPresentation | null>("presentation", null);
  const session = ctx.useRef<UiScreenSession | null>("character-memory-screen-session", null);
  if (session.current === null) session.current = createUiScreenSession(presentation);
  const screenSession = session.current;
  const [managed, setManaged] = ctx.useState<ManagedEntityRequest | null>("managed-entity", null);
  const managedLive = ctx.useRef<ManagedEntityRequest | null>("managed-entity-live", managed); managedLive.current = managed;
  const nextManaged = ctx.useRef("managed-entity-counter", 0);
  function finishEntity(result: { operation: "saved" | "deleted"; id: string } | null): void {
    managedLive.current = null; setManaged(null);
    void controller.evaluateJavascript("window.dispatchEvent(new CustomEvent('character-entity-closed', { detail: " + JSON.stringify(result) + " }));")
      .catch(error => setError(String(error)));
  }
  if (requiredMode !== null && screenSession.currentScreen().input.mode !== requiredMode) throw new Error("The attachment route requires an explicit attachment presentation");
  const origin = "https://characters.operit.local/";
  const avatars = ctx.useRef("character-memory-avatar-resources", createAvatarResources(origin)).current;
  /** Sends host theme changes through the plugin-owned document contract. */
  async function applyTheme(theme: ComposeThemeSnapshot): Promise<void> {
    if (ready.current) await controller.evaluateJavascript(`window.applyCharacterMemoryTheme(${JSON.stringify(theme)});`);
  }
  /** Publishes the sole HTML bridge before loading the package resource. */
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("CharacterMemoryHost", {
        /** Loads the initial route, palette and domain aggregate in one existing bridge invocation. */
        loadScreen: async (...args: unknown[]): Promise<ScreenBootstrap> => {
          webArguments(args, 0);
          const current = screenSession.currentScreen();
          const screen = current.input.mode === "manage" && managementView === "memory"
            ? { ...current, input: { mode: "manage" as const, view: "memory" as const } } : current;
          const snapshot = await ToolPkg.ipc.call<{ action: "snapshot" }, Snapshot>("character-memory.request", { action: "snapshot" }, { targetRuntime: "main" });
          ready.current = true;
          return { theme: ctx.Theme.getCurrent(), screen, snapshot };
        },
        /** Routes all editing operations through this package's main runtime. */
        request: async (...args: unknown[]) => {
          const [request] = webArguments(args, 1);
          const decoded = record(request, "character-memory.request") as Request;
          return ToolPkg.ipc.call<Request, RequestOutput>("character-memory.request", decoded, { targetRuntime: "main" });
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
        /** Returns a document resource reference without transporting image bytes through JS. */
        avatarImage: async (...args: unknown[]) => {
          const [uri] = webArguments(args, 1);
          if (uri !== null && typeof uri !== "string") throw new Error("头像路径必须是字符串或 null");
          return avatars.source(uri);
        },
        /** Opens the same native entity renderer used by chat avatars and the sidebar. */
        openEntity: (...args: unknown[]) => {
          const [mode, entity, id] = webArguments(args, 3);
          if (screenSession.currentScreen().input.mode !== "manage") throw new Error("Only management can open a local entity dialog");
          if ((mode !== "create" && mode !== "edit" && mode !== "preview") || (entity !== "card" && entity !== "group") || typeof id !== "string" || (mode === "create" ? id !== "" : id.trim() === "")) throw new Error("Invalid native entity dialog request");
          if (managedLive.current !== null) throw new Error("An entity dialog is already open");
          const request: ManagedEntityRequest = { requestId: "management-" + (++nextManaged.current), input: mode === "create" ? { mode, entity, id: "" } : { mode, entity, id } };
          managedLive.current = request; setManaged(request);
          return null;
        },
        /** Saves an export through the same host boundary used for file selection. */
        exportFile: async (...args: unknown[]) => {
          const [name, content] = webArguments(args, 2);
          if (typeof name !== "string" || name.trim() === "") throw new Error("导出文件名不能为空");
          if (typeof content !== "string") throw new Error("导出内容必须是字符串");
          return ctx.saveFile({ suggestedName: name, content, mimeType: "application/json", extensions: ["json"] });
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
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, [path === "" ? ctx.UI.Text({ text: error === "" ? `正在加载${managementView === "memory" ? "记忆" : definition.title}…` : error }) : ctx.UI.WebView({
    key: "character-memory-web", controller, fillMaxSize: true, url: origin,
    javaScriptEnabled: true, domStorageEnabled: true, supportZoom: false, useWideViewPort: true,
    /** Restricts top-level navigation to the package document. */
    onShouldOverrideUrlLoading: request => request.url === origin ? { action: "allow" } : { action: "cancel" },
    /** Serves the offline document and its registered images through the resource host. */
    onInterceptRequest: request => avatars.intercept(request) ?? (request.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" }),
    /** Handles disposal without retaining the screen in the host theme service. */
    onLifecycleEvent: event => { if (event.type === "Disposed") dispose(); },
  }), ...(managed === null ? [] : [renderManagedEntityScreen(ctx, managed, finishEntity)])]);
}
