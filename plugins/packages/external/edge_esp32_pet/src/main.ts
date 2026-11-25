import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import { connectDevice, discoverDevices, releaseScene, sendMood, type Esp32Target } from "./driver";
import { Esp32Service, type Task } from "./service";

const ROUTE = "toolpkg:com.operit.edge_esp32_pet:ui:studio";
const ORIGIN = "https://esp32-pet.operit.local/";
const TARGET_KEY = "esp32_studio_target";
const TITLE = { zh: "ESP32 任务猫", en: "ESP32 Task Cat" };
const CHANNEL = "esp32-pet-selection";

let service: Esp32Service | null = null;

/** Reads the plugin-owned selection shared by hooks and studio sessions. */
async function configuration(): Promise<{ target: Esp32Target | null }> {
  return PluginConfig.use("esp32-pet", { target: null as Esp32Target | null });
}

/** Creates the retained task owner only when a hook or IPC operation runs. */
function owner(): Esp32Service {
  if (service === null) service = new Esp32Service({
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
    /** Reads approval presence without sending details to the ESP32. */
    approvals() { return ToolPkg.toolPermissions.list(); },
    send: sendMood,
    /** Schedules the single completion transition in the plugin host. */
    schedule(work, delay) { const timer = setTimeout(work, delay); return () => clearTimeout(timer); },
  });
  return service;
}

/** Validates a device identity crossing a studio or persisted-selection boundary. */
function targetIdentity(value: unknown): Esp32Target | null {
  if (value == null) return null;
  if (!value || typeof value !== "object") throw Error("保存的ESP32身份无效");
  const { nodeId, label } = value as Partial<Esp32Target>;
  if (typeof nodeId !== "string" || !nodeId || typeof label !== "string" || !label) throw Error("保存的ESP32身份无效");
  return { nodeId, label };
}

/** Handles selection in the main runtime so closing settings keeps task sync alive. */
export async function selection(request: { action: string; previous?: Esp32Target | null; nodeId?: unknown }): Promise<unknown> {
  if (request.action === "selected") return owner().selected(targetIdentity(request.previous));
  if (request.action === "discover") {
    const discovered = await discoverDevices();
    const previous = await owner().selected(targetIdentity(request.previous));
    const retained = discovered.devices.find(device => device.target.nodeId === previous?.nodeId);
    const connected = retained ?? (discovered.devices.length === 1 ? discovered.devices[0] : null);
    if (connected) await owner().select(connected.target);
    return { ...discovered, connected };
  }
  if (request.action === "disconnect") {
    const previous = await owner().selected(null);
    await owner().select(null);
    if (previous) await releaseScene(previous);
    return null;
  }
  if (request.action === "connect") {
    const connected = await connectDevice(request.nodeId);
    const previous = await owner().selected(null);
    await owner().select(connected.target);
    if (previous && previous.nodeId !== connected.target.nodeId) await releaseScene(previous);
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

/** Loads the last verified ESP32 from this plugin's own environment. */
function selectedDevice(ctx: ComposeDslContext): Esp32Target | null {
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
      return ToolPkg.ipc.call(CHANNEL, { action: "discover", previous: selectedDevice(ctx) }, { targetRuntime: "main" });
    case "connect": {
      const connected = await ToolPkg.ipc.call(CHANNEL,
        { action: "connect", nodeId: operation.nodeId }, { targetRuntime: "main" });
      return connected;
    }
    case "disconnect": {
      await ToolPkg.ipc.call(CHANNEL, { action: "disconnect" }, { targetRuntime: "main" });
      return null;
    }
    default:
      throw Error("ESP32 任务猫不支持此操作");
  }
}

/** Mounts one offline document and exposes its typed settings bridge. */
export function studio(ctx: ComposeDslContext): ComposeNode {
  const controller = ctx.createWebViewController("esp32-studio");
  const [file, setFile] = ctx.useState("esp32-studio-file", "");
  const [error, setError] = ctx.useState("esp32-studio-error", "");
  ctx.useEffect("esp32-studio-cleanup", () => () => controller.removeJavascriptInterface("Esp32Studio"), []);

  /** Prepares the packaged document before the WebView navigates to it. */
  async function initialize(): Promise<void> {
    try {
      controller.addJavascriptInterface("Esp32Studio", {
        /** Returns operation errors through the same bridge as successful settings. */
        request: async (args) => {
          try {
            return { ok: true, data: await request(ctx, args) };
          } catch (problem) {
            return { ok: false, error: problem instanceof Error ? problem.message : String(problem) };
          }
        },
      });
      setFile(await ToolPkg.readResource("esp32_studio", "esp32-studio.html"));
    } catch (problem) {
      setError(String(problem));
    }
  }

  return ctx.UI.Box({ fillMaxSize: true, onLoad: initialize }, file ? ctx.UI.WebView({
    key: "esp32-studio",
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
  }) : ctx.UI.Text({ text: error || "正在打开ESP32 任务猫…", padding: 24 }));
}

/** Registers a normal ToolPkg route using the existing plugin sidebar. */
export function registerToolPkg(): boolean {
  ToolPkg.registerUiRoute({ id: "studio", route: ROUTE, runtime: "compose_dsl", screen: studio, title: TITLE });
  ToolPkg.registerNavigationEntry({ id: "esp32_studio", route: ROUTE, surface: "main_sidebar_plugins", title: TITLE, icon: "Pets", order: 148 });
  ToolPkg.registerChatRuntimeHook({ id: "esp32_tasks", function: onRuntime });
  ToolPkg.registerToolPermissionHook({ id: "esp32_approvals", function: onPermissions });
  for (const event of ["application_on_create", "application_on_foreground", "application_on_terminate"] as const)
    ToolPkg.registerAppLifecycleHook({ id: event, event, function: onLifecycle });
  return true;
}
