import { DEFAULTS, PET_ROUTE, DETAILS_ROUTE, selectView, validateSettings, type Task, type Settings, type Mood, type Request, type Status, type View } from "./model";

export interface Port {
  log(level: "info" | "debug" | "error", event: string, fields: Record<string, unknown>): void;
  load(): Promise<Settings>;
  save(settings: Settings): Promise<void>;
  snapshot(): Promise<Task[]>;
  approvals(): Promise<ToolPkg.ToolPermissionRequest[]>;
  resource(mood: Mood): Promise<string>;
  capabilities(): Promise<{ scope: string; permissionGranted: boolean }>;
  windows(): Promise<ToolPkg.FloatingWindowInfo[]>;
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
  private terminated = false;
  private sequence = 0;
  private visual = "";
  private presentation = "closed";
  private position: Pick<ToolPkg.FloatingWindowOptions, "x" | "y"> | null = null;
  private tail: Promise<unknown> = Promise.resolve();
  private serial = 0;
  private pending = 0;
  private operation = 0;
  private decision = "";
  /** Binds the host port without performing storage or window work at construction. */
  constructor(private readonly port: Port) {}

  /** Serializes independent hooks and IPC requests without losing caller errors. */
  enqueue<T>(work: () => Promise<T>, cause = "internal", fields: Record<string, unknown> = {}, level: "info" | "debug" = "debug"): Promise<T> {
    const operation = ++this.serial, receivedAt = Date.now();
    this.pending++;
    this.port.log(level, "received", { operation, cause, ...fields, pending: this.pending });
    const result = this.tail.then(async () => {
      this.operation = operation;
      const startedAt = Date.now();
      this.port.log(level, "begin", { operation, cause, queueWaitMs: startedAt - receivedAt, pending: this.pending });
      try {
        const value = await work();
        this.port.log(level, "done", { operation, cause, elapsedMs: Date.now() - startedAt, totalMs: Date.now() - receivedAt, presentation: this.presentation });
        return value;
      } catch (error) {
        this.port.log("error", "failed", { operation, cause, elapsedMs: Date.now() - startedAt, totalMs: Date.now() - receivedAt,
          error: error instanceof Error ? error.message : String(error) });
        throw error;
      } finally { this.pending--; }
    });
    this.tail = result.then(() => undefined, () => undefined);
    return result;
  }

  /** Separates startup queries, resource reads and native window waits in diagnostics. */
  private async measure<T>(stage: string, work: () => Promise<T>, level: "info" | "debug" = "info"): Promise<T> {
    const startedAt = Date.now();
    this.port.log(level, "stage.begin", { operation: this.operation, stage });
    try {
      const result = await work();
      this.port.log(level, "stage.done", { operation: this.operation, stage, elapsedMs: Date.now() - startedAt });
      return result;
    } catch (error) {
      this.port.log("error", "stage.failed", { operation: this.operation, stage, elapsedMs: Date.now() - startedAt,
        error: error instanceof Error ? error.message : String(error) });
      throw error;
    }
  }

  /** Reads authoritative tasks once at activation, then consumes runtime events. */
  private async initialize(): Promise<void> {
    if (this.ready) return;
    this.settings = validateSettings(await this.measure("load-settings", () => this.port.load()));
    for (const task of await this.measure("task-snapshot", () => this.port.snapshot())) if (task.active || task.state === "error") this.tasks.set(task.chatId, task);
    this.approvals = await this.measure("approval-snapshot", () => this.port.approvals());
    this.ready = true;
    this.port.log("info", "initialized", { operation: this.operation, settings: this.settings, tasks: this.tasks.size, approvals: this.approvals.length });
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
    const reason = this.terminated ? "terminated" : !this.settings.enabled ? "disabled" : this.dismissed ? "dismissed"
      : !this.manual && this.settings.backgroundOnly && !this.background ? "foreground"
      : view.mood === "idle" && !this.settings.keepIdle && !this.manual ? "idle-disabled"
      : this.manual ? "manual" : this.settings.backgroundOnly ? "background" : "always";
    const visible = ["manual", "background", "always"].includes(reason);
    const decision = { visible, reason, background: this.background, manual: this.manual, dismissed: this.dismissed, terminated: this.terminated,
      settings: this.settings, mood: view.mood, activeTasks: view.count, approvals: this.approvals.length, details: this.details };
    const key = JSON.stringify(decision);
    if (key !== this.decision) {
      this.decision = key;
      this.port.log("info", "visibility", { operation: this.operation, ...decision, previousPresentation: this.presentation });
    }
    if (!visible && this.presentation === "shown" && !this.terminated) {
      const cat = (await this.port.windows()).find(window => window.id === "cat");
      if (cat) this.position = { x: cat.options.x, y: cat.options.y };
    }
    this.presentation = await this.measure(visible ? "cat.set" : "cat.close", async () => this.port.set("cat", visible ? {
      route: PET_ROUTE,
      state: { view, sprite: await this.measure(`resource.${view.mood}`, () => this.port.resource(view.mood), this.presentation === "shown" ? "debug" : "info") },
      options: { ...(this.presentation !== "shown" ? this.position : null), width: this.settings.width, height: this.settings.width + 32, alwaysOnTop: true, focusable: false, draggable: true },
    } : null), (visible ? this.presentation === "shown" : this.presentation === "closed") ? "debug" : "info");
    if (this.details && !this.terminated && this.settings.enabled && !this.dismissed) {
      await this.port.set("details", { route: DETAILS_ROUTE, state: { view }, options: { width: 360, height: 460, alwaysOnTop: true, focusable: false, draggable: true } });
    } else { this.details = false; await this.port.set("details", null); }
  }

  /** Accepts structured runtime changes and never celebrates cancellation or idle transitions. */
  runtime(event: ToolPkg.ChatRuntimeEventPayload): Promise<void> {
    return this.enqueue(async () => {
      await this.initialize();
      for (const id of event.activeChatIds) if (!this.tasks.has(id)) this.tasks.set(id, { chatId: id, active: true, state: "processing", message: "正在处理" });
      for (const [id, task] of this.tasks) if ((task.active && !event.activeChatIds.includes(id) && id !== event.chatId)
        || (event.isActive && task.state === "completed")) this.tasks.delete(id);
      if (event.isActive || event.state === "error" || event.state === "completed") this.tasks.set(event.chatId, {
        chatId: event.chatId, state: event.state, active: event.isActive, message: event.message ?? "",
        title: event.taskTitle || this.tasks.get(event.chatId)?.title, round: event.currentRound, toolName: event.toolName,
      });
      else this.tasks.delete(event.chatId);
      if (event.isActive) { this.completed = null; this.finish?.(); this.finish = null; }
      if (event.state === "completed") {
        this.completed = event.chatId;
        this.finish?.();
        this.finish = this.port.schedule(() => {
          void this.enqueue(async () => {
            if (this.tasks.get(event.chatId)?.state === "completed") this.tasks.delete(event.chatId);
            this.completed = null; this.finish = null; await this.render();
          }).catch(error => console.error("[task-status-pet]", error));
        }, 3000);
      }
      await this.render();
    }, "runtime", { state: event.state, activeTasks: event.activeChatIds.length });
  }

  /** Replaces approvals with the broker's complete ordered snapshot. */
  permissions(requests: ToolPkg.ToolPermissionRequest[]): Promise<void> {
    return this.enqueue(async () => { await this.initialize(); this.approvals = requests; await this.render(); }, "permissions", { count: requests.length });
  }

  /** Tracks foreground visibility and closes every owned surface on termination. */
  lifecycle(event: ToolPkg.AppLifecycleEvent, origin: Record<string, unknown> = {}): Promise<void> {
    return this.enqueue(async () => {
      if (event === "application_on_terminate") {
        this.terminated = true; this.details = false;
        this.position = null;
        this.finish?.(); this.finish = null;
        await this.port.set("cat", null); await this.port.set("details", null);
        this.presentation = "closed";
        return;
      }
      if (event === "application_on_create") this.terminated = false;
      await this.initialize();
      const background = event === "application_on_background";
      this.port.log("info", "lifecycle", { operation: this.operation, event, ...origin, previousBackground: this.background, background,
        unchanged: background === this.background, manual: this.manual, dismissed: this.dismissed });
      if (background !== this.background) { this.manual = false; this.dismissed = false; }
      this.background = background;
      if (!this.background) this.details = false;
      await this.render();
    }, "lifecycle", { event, ...origin }, "info");
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
    }, "request", { kind: request.kind }, request.kind === "status" ? "debug" : "info");
  }
}
