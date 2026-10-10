import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot } from "../../../../types/compose-dsl";
import type { Definition, Request, RequestOutput } from "./model";
import { imageSource, pickedImagePath } from "./image-source";
import { parseScreenResult } from "./presentation";
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
          const current = screenSession.currentScreen();
          return current.input.mode === "manage" && managementView === "memory"
            ? { ...current, input: { mode: "manage", view: "memory" } } : current;
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
        /** Reads an image through Files instead of exposing VFS paths to Image.file. */
        avatarImage: async (...args: unknown[]) => {
          const [uri] = webArguments(args, 1);
          if (uri !== null && typeof uri !== "string") throw new Error("头像路径必须是字符串或 null");
          return imageSource(uri === null ? await ToolPkg.readResource("character_default_avatar", "operit-avatar.png") : uri);
        },
        /** Imports only a user-selected image to persistent package storage. */
        chooseAvatar: async (...args: unknown[]) => {
          webArguments(args, 0);
          const selected = await ctx.openFilePicker({ picker: "image", allowMultiple: false, mimeTypes: ["image/*"] });
          if (selected.cancelled) return null;
          if (selected.files.length !== 1) throw new Error("请选择一张头像图片");
          const platform = (await Tools.System.terminal.info()).platform;
          const source = await imageSource(pickedImagePath(selected.files[0].path, platform));
          const directory = ToolPkg.getConfigDir().replace(/\/$/, "") + "/avatars";
          const created = await Tools.Files.mkdir(directory, true);
          if (!created.successful) throw new Error(created.details);
          const uri = directory + "/" + Date.now() + "-" + Math.random().toString(36).slice(2) + ".image";
          const saved = await Tools.Files.writeBinary(uri, source.slice(source.indexOf(",") + 1));
          if (!saved.successful) throw new Error(saved.details);
          return { uri, source };
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
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, path === "" ? ctx.UI.Text({ text: error === "" ? `正在加载${managementView === "memory" ? "记忆" : definition.title}…` : error }) : ctx.UI.WebView({
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
