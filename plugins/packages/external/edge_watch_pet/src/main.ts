import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { connectWatch, discoverWatches, readConfig, writeConfig, sendMood, type WatchTarget } from "./driver";
import { WatchService, type Task } from "./service";

const ROUTE = "toolpkg:com.operit.edge_watch_pet:ui:studio";
const ORIGIN = "https://watch-pet.operit.local/";
const TARGET_KEY = "watch_studio_target";
const TITLE = { zh: "手表桌宠", en: "Watch Pet" };
const CHANNEL = "watch-pet-selection";
const MOODS = ["working", "thinking", "approval", "completed", "idle", "error"] as const;
let service: WatchService | null = null;

/** Reads the plugin-owned selection shared by hooks and studio sessions. */
async function configuration(): Promise<{ target: WatchTarget | null }> {
  return PluginConfig.use("watch-pet", { target: null as WatchTarget | null });
}

/** Creates the retained task owner only when a hook or IPC operation runs. */
function owner(): WatchService {
  if (service === null) service = new WatchService({
    /** Loads only the device identity, without retaining task text. */
    async load() { return targetIdentity((await configuration()).target); },
    /** Persists the selected device before sending task updates. */
    async save(target) { const config = await configuration(); config.target = target; await PluginConfig.flush(config); },
    /** Reads current processing state when the plugin starts. */
    async snapshot() {
      const tasks: Task[] = [];
      const chats = (await Tools.Chat.listAll()).chats;
      for (let offset = 0; offset < chats.length; offset += 8) tasks.push(...await Promise.all(chats.slice(offset, offset + 8).map(async chat => {
        const status = await Tools.Chat.agentStatus(chat.id);
        return { chatId: chat.id, state: status.state, active: status.isProcessing };
      })));
      return tasks;
    },
    /** Reads approval presence without sending details to the watch. */
    approvals() { return ToolPkg.toolPermissions.list(); },
    send: sendMood,
    /** Schedules the single completion transition in the plugin host. */
    schedule(work, delay) { const timer = setTimeout(work, delay); return () => clearTimeout(timer); },
  });
  return service;
}

/** Validates a device identity crossing a studio or persisted-selection boundary. */
function targetIdentity(value: unknown): WatchTarget | null {
  if (value === null) return null;
  if (!value || typeof value !== "object") throw Error("保存的手表身份无效");
  const { nodeId, label } = value as Partial<WatchTarget>;
  if (typeof nodeId !== "string" || !nodeId || typeof label !== "string" || !label) throw Error("保存的手表身份无效");
  return { nodeId, label };
}

/** Handles selection in the main runtime so closing settings keeps task sync alive. */
export async function selection(request: { action: string; previous?: WatchTarget | null; nodeId?: unknown }): Promise<unknown> {
  if (request.action === "selected") return owner().selected(targetIdentity(request.previous));
  if (request.action === "discover") {
    const discovered = await discoverWatches();
    const previous = await owner().selected(targetIdentity(request.previous));
    const retained = discovered.watches.find(watch => watch.target.nodeId === previous?.nodeId);
    const connected = retained ?? (discovered.watches.length === 1 ? discovered.watches[0] : null);
    await owner().select(connected ? connected.target : null);
    return { ...discovered, connected };
  }
  if (request.action === "connect") {
    const connected = await connectWatch(request.nodeId);
    await owner().select(connected.target);
    return connected;
  }
  throw Error("无效的设备选择操作");
}

ToolPkg.ipc.on(CHANNEL, selection);

/** Receives actual task changes in the retained main runtime. */
export function onRuntime(event: ToolPkg.ChatRuntimeHookEvent): Promise<void> { return owner().runtime(event.eventPayload); }

/** Receives the broker's complete approval snapshot. */
export function onPermissions(event: ToolPkg.ToolPermissionHookEvent): Promise<void> { return owner().permissions(event.eventPayload.requests); }

/** Restores task synchronization on activation and releases it on termination. */
export function onLifecycle(event: ToolPkg.AppLifecycleHookEvent): Promise<void> { return owner().lifecycle(event.eventName); }

/** Loads the last verified watch from this plugin's own environment. */
function selectedWatch(ctx: ComposeDslContext): WatchTarget | null {
  const saved = ctx.getEnv(TARGET_KEY);
  if (saved === undefined) return null;
  return targetIdentity(JSON.parse(saved));
}

/** Allows only the settings operations exposed by this local studio. */
async function request(ctx: ComposeDslContext, args: unknown[]): Promise<unknown> {
  const [payload] = args;
  if (args.length !== 1 || !payload || typeof payload !== "object") throw Error("无效的设置请求");
  const operation = payload as Record<string, unknown>;
  switch (operation.action) {
    case "discover":
      return ToolPkg.ipc.call(CHANNEL, { action: "discover", previous: selectedWatch(ctx) }, { targetRuntime: "main" });
    case "connect": {
      const connected = await ToolPkg.ipc.call(CHANNEL,
        { action: "connect", nodeId: operation.nodeId }, { targetRuntime: "main" });
      return connected;
    }
    case "read":
    case "save": {
      const target = await ToolPkg.ipc.call<unknown, WatchTarget | null>(CHANNEL,
        { action: "selected", previous: selectedWatch(ctx) }, { targetRuntime: "main" });
      if (!target || operation.nodeId !== target.nodeId) throw Error("所选手表已变更，请重新连接");
      return operation.action === "read" ? readConfig(target) : writeConfig(target, operation.config);
    }
    default:
      throw Error("手表桌宠不支持此操作");
  }
}

/** Mounts one offline document and exposes its typed settings bridge. */
export function studio(ctx: ComposeDslContext): ComposeNode {
  const controller = ctx.createWebViewController("watch-studio");
  const [file, setFile] = ctx.useState("watch-studio-file", "");
  const [error, setError] = ctx.useState("watch-studio-error", "");
  const [images, setImages] = ctx.useState<Record<string, string>>("watch-studio-images", {});
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
      const paths: Record<string, string> = {};
      for (const mood of MOODS) paths[`${ORIGIN}${mood}.webp`] = await ToolPkg.readResource(mood);
      setImages(paths);
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
    } : images[incoming.url] ? {
      action: "respond", response: { mimeType: "image/webp", statusCode: 200, reasonPhrase: "OK", filePath: images[incoming.url] },
    } : { action: "block" },
  }) : ctx.UI.Text({ text: error || "正在打开手表桌宠…", padding: 24 }));
}

/** Registers a normal ToolPkg route using the existing plugin sidebar. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "studio", route: ROUTE, runtime: "compose_dsl", screen: studio, title: TITLE });
  ToolPkg.registerNavigationEntry({ id: "watch_studio", route: ROUTE, surface: "main_sidebar_plugins", title: TITLE, icon: "Pets", order: 148 });
  ToolPkg.registerChatRuntimeHook({ id: "watch_tasks", function: onRuntime });
  ToolPkg.registerToolPermissionHook({ id: "watch_approvals", function: onPermissions });
  for (const event of ["application_on_create", "application_on_foreground", "application_on_terminate"] as const)
    ToolPkg.registerAppLifecycleHook({ id: event, event, function: onLifecycle });
  return true;
}
