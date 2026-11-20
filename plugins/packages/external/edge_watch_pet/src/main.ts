import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { connectWatch, readConfig, writeConfig, type WatchTarget } from "./driver";

const ROUTE = "toolpkg:com.operit.edge_watch_pet:ui:studio";
const ORIGIN = "https://watch-pet.operit.local/";
const TARGET_KEY = "watch_studio_target";
const TITLE = { zh: "手表桌宠", en: "Watch Pet" };

/** Loads the last verified watch from this plugin's own environment. */
function selectedWatch(ctx: ComposeDslContext): WatchTarget | null {
  const saved = ctx.getEnv(TARGET_KEY);
  if (saved === undefined) return null;
  const target: unknown = JSON.parse(saved);
  if (!target || typeof target !== "object") throw Error("保存的手表身份无效");
  const { nodeId, label } = target as Partial<WatchTarget>;
  if (typeof nodeId !== "string" || !nodeId || typeof label !== "string" || !label) {
    throw Error("保存的手表身份无效");
  }
  return { nodeId, label };
}

/** Allows only the settings operations exposed by this local studio. */
async function request(ctx: ComposeDslContext, args: unknown[]): Promise<unknown> {
  const [payload] = args;
  if (args.length !== 1 || !payload || typeof payload !== "object") throw Error("无效的设置请求");
  const operation = payload as Record<string, unknown>;
  switch (operation.action) {
    case "selected":
      return selectedWatch(ctx);
    case "connect": {
      const connected = await connectWatch(operation.reference);
      await ctx.setEnv(TARGET_KEY, JSON.stringify(connected.target));
      return connected;
    }
    case "read":
    case "save": {
      const target = selectedWatch(ctx);
      if (!target || operation.nodeId !== target.nodeId) throw Error("所选手表已变更，请重新连接");
      return operation.action === "read" ? readConfig(target) : writeConfig(target, operation.config);
    }
    default:
      throw Error("手表桌宠不支持此操作");
  }
}

/** Mounts one offline document and exposes its typed settings bridge. */
function studio(ctx: ComposeDslContext): ComposeNode {
  const controller = ctx.createWebViewController("watch-studio");
  const [file, setFile] = ctx.useState("watch-studio-file", "");
  const [error, setError] = ctx.useState("watch-studio-error", "");
  ctx.useEffect("watch-studio-cleanup", () => () => controller.removeJavascriptInterface("WatchStudio"), []);

  /** Prepares the packaged document before the WebView navigates to it. */
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("WatchStudio", {
        /** Returns operation errors through the same bridge as successful settings. */
        request: async (args) => {
          try {
            return { ok: true, data: await request(ctx, args) };
          } catch (problem) {
            return { ok: false, error: problem instanceof Error ? problem.message : String(problem) };
          }
        },
      });
      setFile(await ToolPkg.readResource("watch_studio", "watch-studio.html"));
    } catch (problem) {
      setError(String(problem));
    }
  }

  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, file ? ctx.UI.WebView({
    key: "watch-studio",
    controller,
    url: ORIGIN,
    fillMaxSize: true,
    javaScriptEnabled: true,
    domStorageEnabled: false,
    useWideViewPort: true,
    onShouldOverrideUrlLoading: (navigation) => navigation.url === ORIGIN ? { action: "allow" } : { action: "cancel" },
    onInterceptRequest: (incoming) => incoming.url === ORIGIN ? {
      action: "respond",
      response: { mimeType: "text/html", statusCode: 200, encoding: "utf-8", reasonPhrase: "OK", filePath: file },
    } : { action: "block" },
  }) : ctx.UI.Text({ text: error || "正在打开手表桌宠…", padding: 24 }));
}

/** Registers a normal ToolPkg route using the existing plugin sidebar. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "studio", route: ROUTE, runtime: "compose_dsl", screen: studio, title: TITLE });
  ToolPkg.registerNavigationEntry({ id: "watch_studio", route: ROUTE, surface: "main_sidebar_plugins", title: TITLE, icon: "Pets", order: 148 });
  return true;
}
