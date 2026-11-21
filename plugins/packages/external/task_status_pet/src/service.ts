import { DEFAULTS, PET_ROUTE, DETAILS_ROUTE, selectView, validateSettings, type Task, type Settings, type Mood, type Request, type Status, type View } from "./model";

export interface Port {
  load(): Promise<Settings>;
  save(settings: Settings): Promise<void>;
  snapshot(): Promise<Task[]>;
  approvals(): Promise<ToolPkg.ToolPermissionRequest[]>;
  resource(mood: Mood): Promise<string>;
  capabilities(): Promise<{ scope: string; permissionGranted: boolean }>;
  set(id: string, content: ToolPkg.FloatingWindowContent | null): Promise<string>;
  respond(chatId: string, requestId: string, decision: ToolPkg.ToolPermissionDecision): Promise<void>;
  switchChat(chatId: string): Promise<unknown>;
  schedule(work: () => void, delay: number): () => void;
}

/** Owns one serialized main-runtime state machine for hooks and all UI contexts. */
export class PetService {
  private settings: Settings = { ...DEFAULTS };
  private tasks = new Map<string, Task>();
  private approvals: ToolPkg.ToolPermissionRequest[] = [];
  private background = false;
  private manual = false;
  private dismissed = false;
  private details = false;
  private completed: string | null = null;
  private finish: (() => void) | null = null;
  private ready = false;
  private sequence = 0;
  private visual = "";
  private presentation = "closed";
  private tail: Promise<unknown> = Promise.resolve();
  constructor(private readonly port: Port) {}

  /** Serializes independent hooks and IPC requests without losing caller errors. */
  enqueue<T>(work: () => Promise<T>): Promise<T> {
    const result = this.tail.then(work);
    this.tail = result.then(() => undefined, () => undefined);
    return result;
  }

  /** Reads authoritative tasks once at activation, then consumes runtime events. */
  private async initialize(): Promise<void> {
    if (this.ready) return;
    this.settings = validateSettings(await this.port.load());
    for (const task of await this.port.snapshot()) if (task.active || task.state === "error") this.tasks.set(task.chatId, task);
    this.approvals = await this.port.approvals();
    this.ready = true;
  }

  /** Computes the current presentation without storing conversation contents. */
  private view(): View {
    return selectView([...this.tasks.values()], this.approvals, this.completed, this.sequence);
  }

  /** Reconciles plugin-owned windows while leaving geometry and playback with the host. */
  private async render(): Promise<void> {
    const view = this.view();
    const visual = `${view.mood}:${view.chatId}`;
    if (visual !== this.visual) { this.visual = visual; this.sequence++; view.sequence = this.sequence; }
    const visible = this.settings.enabled && !this.dismissed && (this.manual || !this.settings.backgroundOnly || this.background)
      && (view.mood !== "idle" || this.settings.keepIdle || this.manual);
    this.presentation = await this.port.set("cat", visible ? {
      route: PET_ROUTE,
      state: { view, sprite: await this.port.resource(view.mood) },
      options: { width: this.settings.width, height: this.settings.width + 32, alwaysOnTop: true, focusable: false, draggable: true },
    } : null);
    if (this.details && this.settings.enabled && !this.dismissed) {
      await this.port.set("details", { route: DETAILS_ROUTE, state: { view }, options: { width: 360, height: 460, alwaysOnTop: true, focusable: false, draggable: true } });
    } else { this.details = false; await this.port.set("details", null); }
  }

  /** Accepts structured runtime changes and never celebrates cancellation or idle transitions. */
  runtime(event: ToolPkg.ChatRuntimeEventPayload): Promise<void> {
    return this.enqueue(async () => {
      await this.initialize();
      for (const id of event.activeChatIds) if (!this.tasks.has(id)) this.tasks.set(id, { chatId: id, active: true, state: "processing", message: "正在处理" });
      for (const [id, task] of this.tasks) if (task.active && !event.activeChatIds.includes(id) && id !== event.chatId) this.tasks.delete(id);
      if (event.isActive || event.state === "error") this.tasks.set(event.chatId, {
        chatId: event.chatId, state: event.state, active: event.isActive, message: event.message ?? "",
      });
      else this.tasks.delete(event.chatId);
      if (event.isActive) { this.completed = null; this.finish?.(); this.finish = null; }
      if (event.state === "completed") {
        this.completed = event.chatId;
        this.finish?.();
        this.finish = this.port.schedule(() => {
          void this.enqueue(async () => { this.completed = null; this.finish = null; await this.render(); }).catch(error => console.error("[task-status-pet]", error));
        }, 3000);
      }
      await this.render();
    });
  }

  /** Replaces approvals with the broker's complete ordered snapshot. */
  permissions(requests: ToolPkg.ToolPermissionRequest[]): Promise<void> {
    return this.enqueue(async () => { await this.initialize(); this.approvals = requests; await this.render(); });
  }

  /** Tracks foreground visibility and closes every owned surface on termination. */
  lifecycle(event: ToolPkg.AppLifecycleEvent): Promise<void> {
    return this.enqueue(async () => {
      if (event === "application_on_terminate") {
        this.finish?.(); this.finish = null;
        await this.port.set("cat", null); await this.port.set("details", null);
        return;
      }
      await this.initialize();
      this.background = event === "application_on_background";
      this.manual = false; this.dismissed = false;
      if (!this.background) this.details = false;
      await this.render();
    });
  }

  /** Executes user controls only in the main runtime and returns the actual host status. */
  request(request: Request): Promise<Status> {
    return this.enqueue(async () => {
      await this.initialize();
      switch (request.kind) {
        case "save": {
          const settings = validateSettings(request.settings);
          await this.port.save(settings); this.settings = settings;
          break;
        }
        case "show": this.manual = true; this.dismissed = false; break;
        case "hide": this.manual = false; this.dismissed = true; this.details = false; break;
        case "details": this.details = true; break;
        case "closeDetails": this.details = false; break;
        case "switchChat": await this.port.switchChat(request.chatId); break;
        case "respond":
          if (!["allow", "deny", "allow_session"].includes(request.decision)) throw new Error("无效的审批决策");
          await this.port.respond(request.chatId, request.requestId, request.decision);
          this.approvals = await this.port.approvals();
          break;
        case "status": break;
        default: throw new Error("未知的任务猫操作");
      }
      await this.render();
      const caps = await this.port.capabilities();
      return { settings: this.settings, scope: caps.scope, permissionGranted: caps.permissionGranted, view: this.view(), presentation: this.presentation };
    });
  }
}
