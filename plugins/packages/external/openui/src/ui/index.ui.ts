import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot, ComposeWebViewResourceDecision, ComposeWebViewResourceRequest } from "../../../../../types/compose-dsl";
import type { JsonObject } from "../../../../../types/core";
import { browserArguments, ORIGIN, parseOpenUi, requireAction, requireObject, requireWebUrl, type ProgramInput } from "../protocol";

const RESOURCES = [
  { key: "openui_html", path: "", name: "index.html", mime: "text/html" },
  { key: "openui_js", path: "openui.js", name: "openui.js", mime: "application/javascript" },
  { key: "openui_css", path: "openui.css", name: "openui.css", mime: "text/css" },
];

/** Hosts the upstream renderer with plugin-owned streaming, state and capability bridges. */
export default function OpenUiScreen(ctx: ComposeDslContext): ComposeNode {
  const [xmlContent] = ctx.useState("xmlContent", "");
  const [chatId] = ctx.useState("chatId", "");
  const [loaded, setLoaded] = ctx.useState("openui-loaded", false);
  const [error, setError] = ctx.useState("openui-error", "");
  const [height, setHeight] = ctx.useState("openui-height", 120);
  const [link, setLink] = ctx.useState<string | null>("openui-link", null);
  const [formState, setFormState] = ctx.useState<JsonObject>("openui-form-state", {});
  const savedState = ctx.useRef("openui-current-form-state", formState);
  savedState.current = formState;
  const parsed = parseOpenUi(xmlContent);
  const current = ctx.useRef<ProgramInput>("openui-input", { ...parsed, revision: 0 });
  if (current.current.program !== parsed.program || current.current.isStreaming !== parsed.isStreaming) {
    current.current = { ...parsed, revision: current.current.revision + 1 };
  }
  const ready = ctx.useRef("openui-ready", false);
  const files = ctx.useRef("openui-resource-files", new Map<string, { path: string; mime: string }>());
  const controller = ctx.createWebViewController("openui-renderer");

  /** Surfaces a capability or transport failure without replacing the program. */
  function reportFailure(failure: unknown): void {
    setError(String(failure));
  }

  /** Sends one monotonic program revision into the existing React root. */
  function publishInput(): void {
    if (ready.current) {
      void controller.evaluateJavascript("window.OpenUiPlugin.update(" + JSON.stringify(current.current) + ");").catch(reportFailure);
    }
  }

  /** Applies the host's effective brightness through the upstream theme provider. */
  async function publishTheme(theme: ComposeThemeSnapshot): Promise<void> {
    if (ready.current) await controller.evaluateJavascript("window.OpenUiPlugin.theme(" + JSON.stringify(theme) + ");");
  }

  /** Releases the theme subscription and privileged interface when this composition is disposed. */
  function subscribeTheme(): () => void {
    const unsubscribe = ctx.Theme.subscribe(publishTheme);
    return function dispose(): void {
      ready.current = false;
      unsubscribe();
      controller.removeJavascriptInterface("OpenUiHost");
    };
  }
  ctx.useEffect("openui-theme-subscription", subscribeTheme, []);

  /** Publishes stream revisions without remounting the WebView or resetting form state. */
  function updateInput(): () => void {
    publishInput();
    return function releaseInputEffect(): void {};
  }
  ctx.useEffect("openui-input-revision", updateInput, [current.current.revision]);

  /** Registers the sole privileged bridge before loading the offline package document. */
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("OpenUiHost", {
        /** Hydrates this exact block from the current stream, form state and host theme. */
        bootstrap: (...args: unknown[]) => {
          browserArguments(args, 0);
          ready.current = true;
          return { input: current.current, formState: savedState.current, theme: ctx.Theme.getCurrent() };
        },
        /** Retains upstream form changes in this XML render instance. */
        state: (...args: unknown[]) => {
          const [value] = browserArguments(args, 1);
          const state = requireObject(value) as JsonObject;
          savedState.current = state;
          setFormState(state);
        },
        /** Sizes the embedded WebView to its rendered content in CSS pixels. */
        resize: (...args: unknown[]) => {
          const [value] = browserArguments(args, 1);
          if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) throw new Error("Invalid OpenUI height");
          setHeight(Math.ceil(value));
        },
        /** Executes explicit upstream Query and Mutation requests through the normal tool host. */
        callTool: async (...args: unknown[]) => {
          const [value] = browserArguments(args, 1);
          const request = requireObject(value);
          if (typeof request.name !== "string" || request.name.trim() === "") throw new Error("OpenUI tool name is required");
          if (request.arguments !== undefined) requireObject(request.arguments);
          return ctx.callTool<unknown>(request.name, request.arguments as Record<string, unknown> | undefined);
        },
        /** Handles only upstream conversation and URL actions; local bindings stay inside OpenUI. */
        action: async (...args: unknown[]) => {
          const [value] = browserArguments(args, 1);
          const action = requireAction(value);
          if (current.current.isStreaming) throw new Error("OpenUI actions require a completed block");
          switch (action.type) {
            case "continue_conversation": {
              if (chatId.trim() === "") throw new Error("OpenUI action has no owning conversation");
              const result = await Tools.Chat.sendMessage({
                kind: "submit", chatId, runtime: "main", notifyReply: false, turn: { kind: "execute" },
                input: { text: action.humanFriendlyMessage + "\n\nOpenUI action:\n" + JSON.stringify(action), attachments: [], replyToMessageTimestamp: null },
              });
              return result;
            }
            case "open_url":
              setLink(requireWebUrl(action.params.url));
              return;
            default:
              throw new Error("Unsupported OpenUI host action: " + action.type);
          }
        },
      });
      const resources = await Promise.all(RESOURCES.map(
        /** Resolves each declared package asset through the shared resource host. */
        async resource => ({ url: ORIGIN + resource.path, path: await ToolPkg.readResource(resource.key, resource.name), mime: resource.mime }),
      ));
      for (const resource of resources) files.current.set(resource.url, resource);
      setLoaded(true);
    } catch (failure) {
      reportFailure(failure);
    }
  }

  /** Serves registered package files and permits explicit HTTP resources used by upstream components. */
  function intercept(request: ComposeWebViewResourceRequest): ComposeWebViewResourceDecision {
    if (/^https?:\/\//i.test(request.url) && !request.url.startsWith(ORIGIN)) return { action: "allow" };
    const resource = files.current.get(request.url);
    if (resource === undefined) return { action: "block" };
    return { action: "respond", response: { mimeType: resource.mime, encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: resource.path } };
  }

  /** Closes the separate browser dialog, which never receives the OpenUI capability bridge. */
  function closeLink(): void {
    setLink(null);
  }

  const children: ComposeNode[] = [];
  if (error !== "") children.push(ctx.UI.Text({ text: error }));
  if (loaded) children.push(ctx.UI.WebView({
    key: "openui-renderer", controller, url: ORIGIN, fillMaxWidth: true, height,
    javaScriptEnabled: true, domStorageEnabled: false, supportZoom: false, useWideViewPort: true,
    onInterceptRequest: intercept,
    /** Keeps the privileged document fixed and opens HTTP links in an unprivileged browser dialog. */
    onShouldOverrideUrlLoading: request => {
      if (request.url === ORIGIN) return { action: "allow" };
      if (/^https?:\/\//i.test(request.url)) setLink(requireWebUrl(request.url));
      return { action: "cancel" };
    },
    /** Marks a disposed page as unable to receive further stream or theme updates. */
    onLifecycleEvent: event => { if (event.type === "Disposed") ready.current = false; },
  }));
  if (link !== null) children.push(ctx.UI.Dialog({ onDismissRequest: closeLink }, [
    ctx.UI.Column({ fillMaxWidth: true, height: 600 }, [
      ctx.UI.Button({ onClick: closeLink }, [ctx.UI.Text({ text: "Close" })]),
      ctx.UI.WebView({ key: "openui-link-browser", url: link, fillMaxWidth: true, height: 540, javaScriptEnabled: true }),
    ]),
  ]));
  return ctx.UI.Column({ fillMaxWidth: true, onLoad: initialize }, children);
}
