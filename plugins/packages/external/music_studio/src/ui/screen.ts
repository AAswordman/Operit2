import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import type { Request } from "../shared/model";
const URL = "https://music.operit.local/";
/** Only the bundled page gets a bridge; remote navigation and resources are rejected. */
export default function screen(ctx: ComposeDslContext): ComposeNode {
  const controller = ctx.createWebViewController("music-studio");
  const [path, setPath] = ctx.useState("music-path", ""); const [error, setError] = ctx.useState("music-error", "");
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("MusicHost", {
        request: async (...args: unknown[]) => { const [request] = args[0] as [Request]; return ToolPkg.ipc.call<Request, unknown>("music.service", request, { targetRuntime: "main" }); },
        saveFile: async (...args: unknown[]) => {
          const [name, base64] = args[0] as [string, string];
          if (typeof name !== "string" || !/^[a-zA-Z0-9_-]+\.(wav|operitmusic\.json)$/.test(name) || typeof base64 !== "string" || base64.length > 24000000 || !/^[A-Za-z0-9+/]*={0,2}$/.test(base64)) throw new Error("Invalid music export");
          const directory = getPluginConfigDir("com.operit.music_studio") + "/exports";
          const made = await Tools.Files.mkdir(directory, true); if (!made.successful) throw new Error(made.details || "Cannot create export directory");
          const destination = directory + "/" + name; const result = await Tools.Files.writeBinary(destination, base64); if (!result.successful) throw new Error(result.details || "Export write failed");
          return destination;
        },
      });
      setPath(await ToolPkg.readResource("music_web", "music-studio.html"));
    } catch (e) { setError(String(e)); }
  }
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, path ? ctx.UI.WebView({
    key: "music-studio", controller, fillMaxSize: true, url: URL, javaScriptEnabled: true, domStorageEnabled: true,
    supportZoom: false, useWideViewPort: true, mediaPlaybackRequiresUserGesture: true,
    onShouldOverrideUrlLoading: r => r.url === URL ? { action: "allow" } : { action: "cancel" },
    onInterceptRequest: r => r.url === URL ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
  }) : ctx.UI.Text({ text: error || "正在加载音乐工作台…" }));
}
