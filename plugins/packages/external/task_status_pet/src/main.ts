import { CHANNEL, DEFAULTS, PET_ROUTE, DETAILS_ROUTE, SETTINGS_ROUTE, type Mood, type Request, type Status, type Settings, type Task } from "./model";
import { PetService } from "./service";
import { petScreen, detailsScreen, settingsScreen } from "./ui";

let service: PetService | null = null;
const resources = new Map<Mood, Promise<string>>();

/** Reads saved controls exclusively in the retained main runtime. */
async function configuration(): Promise<{ settings: Settings }> {
  return PluginConfig.use("task-status-pet", { settings: { ...DEFAULTS } });
}

/** Reads real processing status at activation without inferring it from message text. */
async function snapshot(): Promise<Task[]> {
  const chats = (await Tools.Chat.listAll()).chats;
  const tasks: Task[] = [];
  for (let offset = 0; offset < chats.length; offset += 8) {
    const batch = await Promise.all(chats.slice(offset, offset + 8).map(async chat => {
      const status = await Tools.Chat.agentStatus(chat.id);
      return { chatId: chat.id, state: status.state, active: status.isProcessing, message: status.message ?? "" };
    }));
    tasks.push(...batch);
  }
  return tasks;
}

/** Creates the plugin's one state owner lazily, outside registration capture. */
function owner(): PetService {
  if (service === null) service = new PetService({
    /** Loads durable controls without persisting transient task data. */
    async load() { return (await configuration()).settings; },
    /** Commits settings before applying window changes. */
    async save(settings) { const config = await configuration(); config.settings = settings; await PluginConfig.flush(config); },
    snapshot,
    /** Queries the shared approval broker using the authenticated plugin identity. */
    approvals() { return ToolPkg.toolPermissions.list(); },
    /** Resolves each of the six packaged animations only once. */
    resource(mood) {
      let promise = resources.get(mood);
      if (!promise) { promise = ToolPkg.readResource(mood); resources.set(mood, promise); }
      return promise;
    },
    /** Reports the actual system or application scope. */
    capabilities() { return ToolPkg.ui.floatingWindows.capabilities(); },
    /** Reads measured drag coordinates before the cat's host surface is released. */
    windows() { return ToolPkg.ui.floatingWindows.list(); },
    /** Delegates window ownership, drag coordinates and platform differences to the host. */
    set(id, content) { return ToolPkg.ui.floatingWindows.set(id, content); },
    /** Submits the exact selected approval; stale requests are rejected by the broker. */
    respond(chatId, requestId, decision) { return ToolPkg.toolPermissions.respond(chatId, requestId, decision); },
    /** Selects the user's requested conversation without sending a message. */
    switchChat(chatId) { return Tools.Chat.switchTo(chatId); },
    /** Schedules only the completion transition, never animation frames. */
    schedule(work, delay) { const timer = setTimeout(work, delay); return () => clearTimeout(timer); },
  });
  return service;
}

/** Routes controls from floating and settings UI sessions to the main owner. */
export function receive(request: Request): Promise<Status> { return owner().request(request); }

/** Receives authoritative typed runtime notifications in the main runtime. */
export function onRuntime(event: ToolPkg.ChatRuntimeHookEvent): Promise<void> { return owner().runtime(event.eventPayload); }

/** Receives the complete approval list, including activation and foreground replays. */
export function onPermissions(event: ToolPkg.ToolPermissionHookEvent): Promise<void> { return owner().permissions(event.eventPayload.requests); }

/** Applies application lifecycle notifications from every supported host. */
export function onLifecycle(event: ToolPkg.AppLifecycleHookEvent): Promise<void> { return owner().lifecycle(event.eventName); }

/** Applies Flutter visibility broadcasts without treating transient inactive focus as background. */
export function onVisibility(event: ToolPkg.HostEventBroadcastHookEvent<"app.lifecycle.resumed" | "app.lifecycle.paused" | "app.lifecycle.hidden" | "app.lifecycle.detached">): Promise<void> {
  const topic = event.eventPayload.payload.topic;
  return owner().lifecycle(topic === "app.lifecycle.resumed" ? "application_on_foreground" : topic === "app.lifecycle.detached" ? "application_on_terminate" : "application_on_background");
}

ToolPkg.ipc.on<Request, Status>(CHANNEL, receive);

/** Registers settings as the standalone entry, plus floating routes and typed hooks. */
export function registerToolPkg(): boolean {
  for (const [id, route, screen, title] of [
    ["settings", SETTINGS_ROUTE, settingsScreen, "任务状态猫设置"],
    ["pet", PET_ROUTE, petScreen, "任务猫悬浮窗"], ["details", DETAILS_ROUTE, detailsScreen, "任务猫详情"],
  ] as const) ToolPkg.registerUiRoute({ id, route, screen, runtime: "compose_dsl", title: { zh: title, en: "Task Status Cat" } });
  ToolPkg.registerNavigationEntry({ id: "settings", route: SETTINGS_ROUTE, surface: "toolbox", icon: "Pets", title: { zh: "任务状态猫", en: "Task Status Cat" } });
  ToolPkg.registerChatRuntimeHook({ id: "tasks", function: onRuntime });
  ToolPkg.registerToolPermissionHook({ id: "approvals", function: onPermissions });
  for (const event of ["application_on_create", "application_on_foreground", "application_on_background", "application_on_terminate"] as const)
    ToolPkg.registerAppLifecycleHook({ id: event, event, function: onLifecycle });
  for (const topic of ["app.lifecycle.resumed", "app.lifecycle.paused", "app.lifecycle.hidden", "app.lifecycle.detached"] as const)
    ToolPkg.registerHostEventHook({ id: topic, source: "broadcast", trigger: { kind: "broadcast", topic }, function: onVisibility });
  return true;
}
