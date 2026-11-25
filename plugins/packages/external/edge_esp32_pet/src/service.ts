import type { Esp32Target } from "./driver";

export type Mood = "working" | "thinking" | "approval" | "completed" | "idle" | "error";
export interface Task { chatId: string; state: string; active: boolean; }
export interface Port {
  load(): Promise<Esp32Target | null>;
  save(target: Esp32Target | null): Promise<void>;
  snapshot(): Promise<Task[]>;
  approvals(): Promise<ToolPkg.ToolPermissionRequest[]>;
  send(target: Esp32Target, mood: Mood): Promise<void>;
  schedule(work: () => void, delay: number): () => void;
}

/** Owns task synchronization independently of the studio's WebView lifetime. */
export class Esp32Service {
  private target: Esp32Target | null = null;
  private tasks = new Map<string, Task>();
  private pending: ToolPkg.ToolPermissionRequest[] = [];
  private completed = false;
  private finish: (() => void) | null = null;
  private ready = false;
  private stopped = false;
  private sent = "";
  private tail: Promise<unknown> = Promise.resolve();

  /** Binds native calls and timers without starting work during registration. */
  constructor(private readonly port: Port) {}

  /** Serializes hook updates and selection changes while preserving caller errors. */
  private enqueue<T>(work: () => Promise<T>): Promise<T> {
    const result = this.tail.then(work);
    this.tail = result.then(() => undefined, () => undefined);
    return result;
  }

  /** Reads real tasks and pending approvals once at plugin activation. */
  private async initialize(): Promise<void> {
    if (this.ready) return;
    this.target = await this.port.load();
    for (const task of await this.port.snapshot()) if (task.active || task.state === "error") this.tasks.set(task.chatId, task);
    this.pending = await this.port.approvals();
    this.ready = true;
  }

  /** Gives approvals and failures priority over active work and completion. */
  private mood(): Mood {
    if (this.pending.length) return "approval";
    const tasks = [...this.tasks.values()];
    if (tasks.some(task => task.state === "error")) return "error";
    const active = tasks.find(task => task.active);
    if (active) return active.state === "connecting" || active.state === "processing" ? "thinking" : "working";
    return this.completed ? "completed" : "idle";
  }

  /** Sends only changed moods and records success after the device acknowledges. */
  private async publish(): Promise<void> {
    if (!this.target || this.stopped) return;
    const mood = this.mood(), key = `${this.target.nodeId}:${mood}`;
    if (key === this.sent) return;
    await this.port.send(this.target, mood);
    this.sent = key;
  }

  /** Restores the persisted selection or migrates the studio's previous selection. */
  selected(previous: Esp32Target | null): Promise<Esp32Target | null> {
    return this.enqueue(async () => {
      await this.initialize();
      if (!this.target && previous) { await this.port.save(previous); this.target = previous; }
      return this.target;
    });
  }

  /** Commits a verified selection and immediately sends the current task mood. */
  select(target: Esp32Target | null): Promise<void> {
    return this.enqueue(async () => {
      await this.initialize();
      await this.port.save(target);
      this.target = target;
      this.sent = "";
      await this.publish();
    });
  }

  /** Applies the authoritative active-chat list without parsing message text. */
  runtime(event: ToolPkg.ChatRuntimeEventPayload): Promise<void> {
    return this.enqueue(async () => {
      if (this.stopped) return;
      await this.initialize();
      for (const id of event.activeChatIds) if (!this.tasks.has(id)) this.tasks.set(id, { chatId: id, state: "processing", active: true });
      for (const [id, task] of this.tasks) if (task.active && id !== event.chatId && !event.activeChatIds.includes(id)) this.tasks.delete(id);
      if (event.isActive || event.state === "error") this.tasks.set(event.chatId, { chatId: event.chatId, state: event.state, active: event.isActive });
      else this.tasks.delete(event.chatId);
      if (event.isActive) { this.completed = false; this.finish?.(); this.finish = null; }
      if (event.state === "completed" && ![...this.tasks.values()].some(task => task.active)) {
        this.completed = true;
        this.finish?.();
        this.finish = this.port.schedule(() => {
          void this.enqueue(async () => { this.completed = false; this.finish = null; await this.publish(); })
            .catch(error => console.error("[esp32-pet] completion", error));
        }, 3000);
      }
      await this.publish();
    });
  }

  /** Replaces the complete pending list without transmitting approval contents. */
  permissions(requests: ToolPkg.ToolPermissionRequest[]): Promise<void> {
    return this.enqueue(async () => {
      if (this.stopped) return;
      await this.initialize(); this.pending = requests; await this.publish();
    });
  }

  /** Refreshes on activation and foreground, and clears the remote mood on shutdown. */
  lifecycle(event: ToolPkg.AppLifecycleEvent): Promise<void> {
    return this.enqueue(async () => {
      if (event === "application_on_terminate") {
        this.stopped = true; this.finish?.(); this.finish = null;
        this.ready = false; this.tasks.clear(); this.pending = []; this.completed = false; this.sent = "";
        if (this.target) await this.port.send(this.target, "idle");
        return;
      }
      this.stopped = false;
      await this.initialize();
      this.sent = "";
      await this.publish();
    });
  }
}
