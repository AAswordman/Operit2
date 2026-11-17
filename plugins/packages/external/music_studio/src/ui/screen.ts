import type { ComposeDslContext, ComposeNode } from "../../../../../types/compose-dsl";
import type { Request } from "../shared/model";
const URL = "https://music.operit.local/";
/** Only the bundled page gets a bridge; remote navigation and resources are rejected. */
export default function screen(ctx: ComposeDslContext): ComposeNode {
  const controller = ctx.createWebViewController("music-studio");
  const themeReady = ctx.useRef("music-theme-ready", false);
  ctx.useEffect("music-theme-subscription", () => {
    const unsubscribe = ctx.Theme.subscribe(async theme => {
      if (themeReady.current) await controller.evaluateJavascript("window.MusicStudioTheme?.apply(" + JSON.stringify({ brightness: theme.brightness }) + ");");
    });
    return () => { themeReady.current = false; unsubscribe(); controller.removeJavascriptInterface("MusicHost"); };
  }, []);
  const [path, setPath] = ctx.useState("music-path", ""); const [error, setError] = ctx.useState("music-error", "");
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("MusicHost", {
        getTheme: () => { themeReady.current = true; return { brightness: ctx.Theme.getCurrent().brightness }; },
        request: async (...args: unknown[]) => { const [request] = args[0] as [Request]; return ToolPkg.ipc.call<Request, unknown>("music.service", request, { targetRuntime: "main" }); },
        saveFile: async (...args: unknown[]) => {
          const [name, contentBase64] = args[0] as [string, string];
          // 240 seconds of stereo PCM plus the effect tail fits within 64 MiB.
          const maxBase64 = 4 * Math.ceil(64 * 1024 * 1024 / 3);
          if (typeof name !== "string" || !/^[a-zA-Z0-9_-]+\.(wav|operitmusic\.json)$/.test(name)
            || typeof contentBase64 !== "string" || contentBase64.length > maxBase64
            || contentBase64.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(contentBase64)) throw new Error("无效的音乐导出文件或文件超过 64 MiB");
          const result = await ctx.saveFile({
            suggestedName: name, contentBase64,
            mimeType: name.endsWith(".wav") ? "audio/wav" : "application/json",
            extensions: name.endsWith(".wav") ? ["wav"] : ["json"],
          });
          if (!result || typeof result.cancelled !== "boolean") throw new Error("宿主未返回保存结果，请更新软件后重试");
          if (!result.cancelled && (typeof result.path !== "string" || !result.path.trim())) throw new Error("宿主未返回保存路径，文件导出未确认");
          return result;
        },
      });
      setPath(await ToolPkg.readResource("music_web", "music-studio.html"));
    } catch (e) { setError(String(e)); }
  }
  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, path ? ctx.UI.WebView({
    key: "music-studio", controller, fillMaxSize: true, url: URL, javaScriptEnabled: true, domStorageEnabled: true,
    supportZoom: false, useWideViewPort: true, mediaPlaybackRequiresUserGesture: true,
    onPageStarted: () => { themeReady.current = false; },
    onShouldOverrideUrlLoading: r => r.url === URL ? { action: "allow" } : { action: "cancel" },
    onInterceptRequest: r => r.url === URL ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
  }) : ctx.UI.Text({ text: error || "正在加载音乐工作台…" }));
}
