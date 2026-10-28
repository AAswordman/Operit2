import { getSettings as waifuSettings, stream as waifuStream } from "./dependencies/waifu/api.js";
import { attachments } from "./attachments.js";
import { QQGateway } from "./gateway.js";
import { QQOpenApi } from "./openapi.js";
import { CHANNEL, MAX_EVENTS, defaultConfig, errorText, integer, nonblank, text, type AutoReplyParams, type Config, type ConfigureParams, type InboundEvent, type ReceiveParams, type Request, type Response, type ReplyRole, type Status } from "./model.js";

/** Owns configuration, Gateway events and AI work in the package main runtime. */
class QQBotService {
  private api: QQOpenApi;
  private gateway: QQGateway;
  private readonly events: InboundEvent[] = [];
  private readonly seen = new Set<string>();
  private autoTimer: number | null = null;
  private autoTask: Promise<void> | null = null;
  private autoGeneration = 0;
  private activeChat: string | null = null;
  private autoRunning = false;
  private autoError = "";
  private repliedCount = 0;

  /** Creates one native network service around a durable plugin configuration. */
  constructor(private readonly config: Config) {
    this.api = this.createApi();
    this.gateway = this.createGateway();
  }

  /** Binds all HTTP calls to the ordinary cross-platform networking SDK. */
  private createApi(config: Config = this.config): QQOpenApi { return new QQOpenApi(config, { request: options => Tools.Net.http(options) }); }

  /** Binds socket handles and ordered dispatches to the current main engine. */
  private createGateway(): QQGateway {
    return new QQGateway(this.api, { request: request => Tools.Net.webSocket(request) }, event => this.enqueue(event));
  }

  /** Exposes operational state without returning stored application credentials. */
  status(): Status {
    return { waifuProfile: this.config.waifuProfile, configured: this.config.appId !== "" && this.config.appSecret !== "", appId: this.config.appId, useSandbox: this.config.useSandbox, listenerEnabled: this.config.listenerEnabled,
      service: { status: this.gateway.status, sessionId: this.gateway.sessionId, lastError: this.gateway.lastError, receivedCount: this.gateway.receivedCount },
      queue: { size: this.events.length, capacity: MAX_EVENTS },
      autoReply: { config: { ...this.config.autoReply }, running: this.autoRunning, lastError: this.autoError, bindings: { ...this.config.bindings }, repliedCount: this.repliedCount } };
  }

  /** Restores explicitly enabled services after application creation or foreground entry. */
  async restore(): Promise<void> {
    if (!this.config.listenerEnabled) return;
    await this.start(false);
  }

  /** Starts the listener and awaits its authenticated READY event. */
  private async start(restart: boolean): Promise<void> {
    nonblank(this.config.appId, "AppID"); nonblank(this.config.appSecret, "AppSecret");
    if (restart) await this.stopRuntime();
    if (this.gateway.status === "ready") {
      if (this.config.autoReply.enabled && !this.autoRunning) this.startAuto();
      return;
    }
    if (this.gateway.status !== "connecting") {
      this.api = this.createApi(); this.gateway = this.createGateway();
      await this.gateway.start();
    }
    await this.gateway.waitReady();
    if (this.config.autoReply.enabled) this.startAuto();
  }

  /** Stops pending AI work and the socket before releasing the listener runtime. */
  async stopRuntime(): Promise<void> { await this.stopAuto(); await this.gateway.stop(); }

  /** Validates a configuration patch before durably committing it and restarting its affected runtime. */
  private async configure(params: ConfigureParams): Promise<void> {
    const next = { appId: this.config.appId, appSecret: this.config.appSecret, useSandbox: this.config.useSandbox, listenerEnabled: this.config.listenerEnabled };
    if (params.app_id !== undefined) next.appId = text(params.app_id, "app_id").trim();
    if (params.app_secret !== undefined) next.appSecret = text(params.app_secret, "app_secret").trim();
    if (params.use_sandbox !== undefined) { if (typeof params.use_sandbox !== "boolean") throw new Error("use_sandbox must be boolean"); next.useSandbox = params.use_sandbox; }
    if (params.listener_enabled !== undefined) { if (typeof params.listener_enabled !== "boolean") throw new Error("listener_enabled must be boolean"); next.listenerEnabled = params.listener_enabled; }
    if (next.listenerEnabled) { nonblank(next.appId, "AppID"); nonblank(next.appSecret, "AppSecret"); }
    if (params.test_connection === true) await this.createApi({ ...this.config, ...next }).test();
    const changed = next.appId !== this.config.appId || next.appSecret !== this.config.appSecret || next.useSandbox !== this.config.useSandbox;
    if (changed || !next.listenerEnabled || params.restart_service === true) await this.stopRuntime();
    Object.assign(this.config, next);
    if (changed) { this.api = this.createApi(); this.gateway = this.createGateway(); }
    await PluginConfig.flush(this.config);
    if (next.listenerEnabled) await this.start(false);
  }

  /** Deduplicates actual remote identities and rejects an overfull event queue explicitly. */
  private enqueue(event: InboundEvent): void {
    const key = event.messageId === "" ? `${event.eventType}:${event.eventId}:${event.seq}` : `${event.scene}:${event.messageId}`;
    if (this.seen.has(key)) return;
    if (this.events.length >= MAX_EVENTS) throw new Error("QQ event queue is full; consume or clear events before restarting the listener");
    this.seen.add(key);
    if (this.seen.size > 512) this.seen.delete(this.seen.values().next().value!);
    this.events.push(event);
  }

  /** Returns and optionally consumes only events matching explicit filters. */
  private receive(params: ReceiveParams): Response {
    const limit = params.limit === undefined ? 20 : integer(params.limit, "limit", 1, 100);
    if (params.scene !== undefined && params.scene !== "c2c" && params.scene !== "group" && params.scene !== "unknown") throw new Error("Invalid scene");
    const selected = this.events.filter(event => (params.scene === undefined || params.scene === event.scene) && (params.event_type === undefined || params.event_type === event.eventType)).slice(0, limit);
    if (params.consume !== false) for (const event of selected) this.events.splice(this.events.indexOf(event), 1);
    const events = selected.map(event => {
      if (params.include_raw === true) return { ...event };
      const { rawPayload: _raw, ...summary } = event;
      return summary;
    });
    return { success: true, returnedCount: events.length, remainingCount: this.events.length, events };
  }

  /** Commits automatic reply settings using the v2 conversation creation contract. */
  private async configureAuto(params: AutoReplyParams): Promise<void> {
    const next = { ...this.config.autoReply };
    for (const [field, value] of [["enabled", params.enabled], ["c2cEnabled", params.c2c_enabled], ["groupEnabled", params.group_enabled]] as const) {
      if (value !== undefined) { if (typeof value !== "boolean") throw new Error(`${field} must be boolean`); next[field] = value; }
    }
    if (params.ai_timeout_ms !== undefined) next.aiTimeoutMs = integer(params.ai_timeout_ms, "ai_timeout_ms", 1000, 600000);
    if (params.chat_group !== undefined) next.chatGroup = text(params.chat_group, "chat_group").trim();
    if (params.character_selection !== undefined) next.characterSelection = text(params.character_selection, "character_selection").trim();
    if (params.assistant_instruction !== undefined) next.assistantInstruction = text(params.assistant_instruction, "assistant_instruction");
    if (next.enabled && params.start_now !== false && !this.config.listenerEnabled) throw new Error("Enable the QQ listener before starting automatic replies");
    const profile = params.waifu_profile === undefined ? this.config.waifuProfile : nonblank(params.waifu_profile, "waifu_profile");
    if (next.enabled) await waifuSettings(profile);
    await this.stopAuto();
    this.config.waifuProfile = profile;
    this.config.autoReply = next;
    await PluginConfig.flush(this.config);
    if (next.enabled && params.start_now !== false) {
      await this.start(false);
    }
  }

  /** Starts one sequential AI consumer without opening a second receive connection. */
  private startAuto(): void {
    if (this.autoRunning) return;
    this.autoRunning = true; this.autoError = "";
    this.scheduleAuto(this.autoGeneration);
  }

  /** Schedules the next automatic reply after the preceding one has completely settled. */
  private scheduleAuto(generation: number): void {
    if (!this.autoRunning || generation !== this.autoGeneration) return;
    this.autoTimer = setTimeout(() => {
      if (this.autoTask !== null) { this.scheduleAuto(generation); return; }
      this.autoTask = this.autoTick(generation);
      void this.autoTask.then(() => { this.autoTask = null; this.scheduleAuto(generation); });
    }, 300);
  }

  /** Cancels the authenticated in-flight generation before allowing stop or reconfiguration to finish. */
  private async stopAuto(): Promise<void> {
    this.autoRunning = false; ++this.autoGeneration;
    if (this.autoTimer !== null) { clearTimeout(this.autoTimer); this.autoTimer = null; }
    if (this.activeChat !== null) await Tools.Chat.cancel(this.activeChat);
    if (this.autoTask !== null) await this.autoTask;
  }

  /** Consumes one supported inbound message and records any real delivery or model error. */
  private async autoTick(generation: number): Promise<void> {
    try {
      if (generation !== this.autoGeneration) return;
      if (this.gateway.status === "error" || this.gateway.status === "stopped") {
        this.autoRunning = false;
        throw new Error(`QQ listener is ${this.gateway.status}: ${this.gateway.lastError}`);
      }
      const config = this.config.autoReply;
      const index = this.events.findIndex(event => event.messageId !== "" && ((event.scene === "c2c" && config.c2cEnabled) || (event.scene === "group" && config.groupEnabled)));
      if (index === -1) return;
      const event = this.events.splice(index, 1)[0];
      const key = `${event.scene}:${event.messageId}`;
      if (this.config.processed.indexOf(key) !== -1) return;
      this.config.processed = [...this.config.processed, key].slice(-512);
      await PluginConfig.flush(this.config);
      const scene = event.scene;
      if (scene !== "c2c" && scene !== "group") throw new Error("Automatic replies require a C2C or group event");
      const target = scene === "c2c" ? event.userOpenId : event.groupOpenId;
      const binding = `${scene}:${target}`;
      let chatId = this.config.bindings[binding];
      if (chatId === undefined) {
        const input = config.characterSelection === "" ? null : { "com.operit.character_cards": { version: 1, selection: config.characterSelection } };
        chatId = (await Tools.Chat.createNew({ setAsCurrentChat: false, input })).chatId;
        await Tools.Chat.updateTitle(chatId, `QQ ${scene} ${target}`);
        if (config.chatGroup !== "") await Tools.Chat.updateGroup([chatId], config.chatGroup);
        this.config.bindings = { ...this.config.bindings, [binding]: chatId };
        await PluginConfig.flush(this.config);
      }
      if (generation !== this.autoGeneration) return;
      const files = await attachments(event);
      if (generation !== this.autoGeneration) return;
      this.activeChat = chatId;
      let timedOut = false;
      const timer = setTimeout(() => {
        timedOut = true;
        void Tools.Chat.cancel(chatId).catch(error => { console.error("[qqbot.auto.timeout]", error); });
      }, config.aiTimeoutMs);
      let delivered = 0;
      try {
        const request = { kind: "submit" as const, chatId, runtime: "main" as const, input: {
          text: `${event.content}\n\n[QQ ${scene}; sender=${event.userOpenId}; message=${event.messageId}]\n${config.assistantInstruction}`,
          attachments: files, replyToMessageTimestamp: null }, turn: { kind: "execute" as const }, notifyReply: false };
        for await (const output of waifuStream(request, this.config.waifuProfile)) {
          if (generation !== this.autoGeneration) return;
          if (timedOut) throw new Error("QQ automatic reply exceeded its AI deadline");
          if (output.type === "segment") {
            if (output.segment.kind === "text") await this.api.send(scene, target, { content: output.segment.text, msg_id: event.messageId });
            else await this.api.sendImage(scene, target, output.segment.source, event.messageId);
            delivered++;
          }
        }
      } catch (error) {
        if (timedOut) throw new Error("QQ automatic reply exceeded its AI deadline");
        throw error;
      } finally { clearTimeout(timer); this.activeChat = null; }
      if (generation !== this.autoGeneration) return;
      if (delivered === 0) throw new Error("Waifu response contained no deliverable segments");
      this.repliedCount++; this.autoError = "";
    } catch (error) { this.autoError = errorText(error); console.error("[qqbot.auto]", error); }
  }

  /** Handles every UI and tool operation in the single package owner. */
  async request(request: Request): Promise<Response> {
    switch (request.action) {
      case "configure": await this.configure(request.params); break;
      case "status": break;
      case "reply_roles": {
        const [cards, groups] = await Promise.all([
          ToolPkg.callDependency<Record<string, never>, { id: string; name: string }[]>("com.operit.character_cards", "character.list", {}),
          ToolPkg.callDependency<Record<string, never>, { id: string; name: string }[]>("com.operit.character_cards", "group.list", {}),
        ]);
        const roles: ReplyRole[] = [
          ...cards.map(card => ({ selection: `card:${card.id}`, name: card.name })),
          ...groups.map(group => ({ selection: `group:${group.id}`, name: `${group.name}（角色组）` })),
        ];
        return { success: true, roles };
      }
      case "test": return { success: true, response: await this.api.test() };
      case "start":
        nonblank(this.config.appId, "AppID"); nonblank(this.config.appSecret, "AppSecret");
        this.config.listenerEnabled = true; await PluginConfig.flush(this.config); await this.start(request.restart === true); break;
      case "stop": await this.stopRuntime(); this.config.listenerEnabled = false; await PluginConfig.flush(this.config); break;
      case "receive": if (request.params.auto_start === true) await this.start(false); return this.receive(request.params);
      case "clear": { const count = this.events.length; this.events.splice(0); return { success: true, clearedCount: count }; }
      case "send_c2c": return { success: true, response: await this.api.send("c2c", request.params.openid, request.params) };
      case "send_group": return { success: true, response: await this.api.send("group", request.params.group_openid, request.params) };
      case "upload_image": return { success: true, response: await this.api.uploadImage(request.scene, request.target, request.source) };
      case "auto_configure": await this.configureAuto(request.params); break;
      case "auto_start": await this.configureAuto({ enabled: true, start_now: true }); break;
      case "auto_stop": await this.configureAuto({ enabled: false }); break;
      case "auto_once":
        if (this.autoTask !== null) throw new Error("An automatic reply is already in progress");
        if (this.gateway.status !== "ready") throw new Error("The QQ listener must be ready");
        this.autoError = "";
        this.autoTask = this.autoTick(this.autoGeneration);
        try { await this.autoTask; } finally { this.autoTask = null; }
        if (this.autoError !== "") throw new Error(this.autoError);
        break;
      default: throw new Error("Unknown QQ Bot operation");
    }
    return { success: true, status: this.status() };
  }
}

let pendingService: Promise<QQBotService> | null = null;
let commandTail: Promise<void> = Promise.resolve();

/** Initializes exactly one service and retains its asynchronous configuration load. */
async function service(): Promise<QQBotService> {
  if (pendingService === null) pendingService = PluginConfig.use("qqbot", defaultConfig()).then(config => new QQBotService(config));
  return pendingService;
}

/** Serializes administrative work while keeping protocol heartbeats independent. */
function command<T>(operation: (owner: QQBotService) => Promise<T>): Promise<T> {
  const result = commandTail.then(async () => operation(await service()));
  commandTail = result.then(() => {}, error => { console.error("[qqbot.request]", error); });
  return result;
}

/** Routes tools and UI commands through the single administrative queue. */
export function receive(request: Request): Promise<Response> { return command(owner => owner.request(request)); }

/** Registers the authoritative main-runtime IPC channel. */
export function registerService(): void { ToolPkg.ipc.on<Request, Response>(CHANNEL, receive); }

/** Restores user-enabled networking when the application starts or returns to the foreground. */
export function restore(): Promise<void> { return command(owner => owner.restore()); }

/** Releases model and networking resources during application termination. */
export function terminate(): Promise<void> { return command(owner => owner.stopRuntime()); }
