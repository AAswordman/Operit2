import type { ComposeDslContext, ComposeNode, ComposeThemeSnapshot } from "../../../../types/compose-dsl";
import type { MemoryGraphResult } from "./model";

export interface MemoryGraphDisplay { data: MemoryGraphResult; selected: string | null; selectedEdge: string | null; theme: ComposeThemeSnapshot }

/** Hosts only the graph canvas; all persistence, forms and navigation stay in DSL. */
export function renderMemoryGraph(ctx: ComposeDslContext, key: string, display: MemoryGraphDisplay, select: (kind: "memory" | "edge", id: string) => void, fail: (error: string) => void): ComposeNode {
  const controller = ctx.createWebViewController(key + ":controller");
  const [path, setPath] = ctx.useState(key + ":path", "");
  const current = ctx.useRef(key + ":display", display); current.current = display;
  const ready = ctx.useRef(key + ":ready", false);
  const active = ctx.useRef(key + ":active", true);
  const selectLive = ctx.useRef(key + ":select", select); selectLive.current = select;
  const origin = "https://memory-graph.operit.local/";
  ctx.useEffect(key + ":lifetime", () => { active.current = true; return () => { active.current = false; ready.current = false; }; }, []);
  /** Registers the finite display-only bridge before loading the offline canvas. */
  async function initialize(): Promise<void> {
    ready.current = false;
    try {
      controller.addJavascriptInterface("MemoryGraphHost", {
        /** Returns an already-authorized graph projection without exposing service requests. */
        load: (...args: unknown[]) => { if (!active.current) throw new Error("Graph display is no longer active"); if (args.length !== 1 || !Array.isArray(args[0]) || args[0].length !== 0) throw new Error("Graph load expects no arguments"); ready.current = true; return current.current; },
        /** Validates canvas selections against the exact graph currently displayed. */
        select: (...args: unknown[]) => {
          if (!active.current) throw new Error("Graph display is no longer active");
          if (args.length !== 1 || !Array.isArray(args[0]) || args[0].length !== 2) throw new Error("Graph select expects kind and id");
          const [kind, id] = args[0];
          if (typeof id !== "string" || kind !== "memory" && kind !== "edge") throw new Error("Invalid graph selection");
          const data = current.current.data;
          if (kind === "memory" ? !data.graph.nodes.some(node => node.id === id) : !data.graph.edges.some(edge => edge.id === id)) throw new Error("Graph selection no longer exists");
          selectLive.current(kind, id); return null;
        },
      });
      const resource = await ToolPkg.readResource("character_memory_html", "character-memory.html");
      if (active.current) setPath(resource);
    } catch (error) { fail(String(error)); }
  }
  ctx.useEffect(key + ":projection", () => {
    if (ready.current) void controller.evaluateJavascript("window.updateMemoryGraph(" + JSON.stringify(display) + ");").catch(error => fail(String(error)));
    return () => {};
  }, [JSON.stringify(display)]);
  /** Sends canvas-only camera commands from native toolbar buttons. */
  async function camera(command: "in" | "out" | "fit"): Promise<void> {
    if (!ready.current) return;
    try { await controller.evaluateJavascript("window.memoryGraphCamera(" + JSON.stringify(command) + ");"); }
    catch (error) { fail(String(error)); }
  }
  return ctx.UI.Box({ key: "memory-graph-display", fillMaxSize: true }, [

    ctx.UI.Box({ key: "memory-graph-loader", fillMaxSize: true, onLoad: initialize }, path === "" ? ctx.UI.Text({ text: "正在加载记忆图谱…" }) : ctx.UI.WebView({ key, controller, fillMaxSize: true, url: origin, javaScriptEnabled: true, domStorageEnabled: false, supportZoom: false,
      /** Allows only the offline graph document to navigate this view. */
      onShouldOverrideUrlLoading: request => ({ action: request.url === origin ? "allow" : "cancel" }),
      /** Serves a single local resource and blocks all other requests. */
      onInterceptRequest: request => request.url === origin ? { action: "respond", response: { mimeType: "text/html", encoding: "utf-8", statusCode: 200, reasonPhrase: "OK", filePath: path } } : { action: "block" },
      /** Stops updates when the display leaves the native composition. */
      onLifecycleEvent: event => { if (event.type === "Disposed") ready.current = false; },
    })),
    ctx.UI.Box({ modifier: ctx.Modifier.align("bottomStart"), padding: 12 }, ctx.UI.Surface({ shape: { cornerRadius: 12 }, color: ctx.MaterialTheme.colorScheme.surfaceContainerHigh }, ctx.UI.Row({}, [
      ...([ ["in", "放大", "Add"], ["out", "缩小", "Remove"], ["fit", "适应画布", "FitScreen"] ] as const).map(([command, label, icon]) => ctx.UI.IconButton({ key: "graph-" + command, tooltip: label, width: 36, height: 36, icon, onClick: () => camera(command) })),
    ]))),
  ]);
}
