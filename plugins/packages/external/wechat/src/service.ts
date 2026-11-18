import { ChannelStore, conversationKey } from "./channel-store.js";
import { ChannelInbox, messageKey } from "./channel-inbox.js";
import type { ChannelRequest, ChannelResponses, ChannelMessage, ChannelConversation } from "./channel-api.js";
import {
  getSettings as waifuSettings,
  stream as waifuStream,
} from "./dependencies/waifu/api.js";
import { WeChatApi, hostHttp, WeChatApiError, WeChatTransportError } from "./api.js";
import { downloadMedia, downloadMediaContent, uploadMedia, mediaReference } from "./media.js";
import {
  API_BASE,
  CHANNEL,
  MAX_EVENTS,
  defaultConfig,
  errorText,
  identifier,
  inbound,
  integer,
  nonblank,
  object,
  text,
  wechatUrl,
  type AutoReplyParams,
  type Config,
  type InboundEvent,
  type JsonObject,
  type LoginStatus,
  type MediaParams,
  type ReceiveParams,
  type Request,
  type ReplyRole,
  type Response,
  type Status,
} from "./model.js";

/** Owns one device-local WeChat account, long poll and ordered automatic replies. */
class WeChatService {
  private readonly sessions = new ChannelStore();
  private readonly opening = new Map<string, Promise<ChannelConversation>>();
  private readonly channelInbox = new ChannelInbox();
  private readonly http = hostHttp();
  private api: WeChatApi;
  private readonly loginApi = new WeChatApi(this.http);
  private state: Status["service"]["state"] = "stopped";
  private lastError = "";
  private receivedCount = 0;
  private pollTask: Promise<void> | null = null;
  private pollGeneration = 0;
  private loginTask: Promise<void> | null = null;
  private loginGeneration = 0;
  private loginSession: {
    qrcode: string;
    baseUrl: string;
    verifyCode: string;
    startOnConfirm: boolean;
  } | null = null;
  private loginDelay: { timer: number; resolve: () => void } | null = null;
  private pollDelay: { timer: number; resolve: () => void } | null = null;
  private login: LoginStatus = {
    state: "idle",
    qrContent: "",
    expiresAt: 0,
    error: "",
  };
  private readonly autoTasks = new Map<string, Promise<void>>();
  private autoTimer: number | null = null;
  private autoGeneration = 0;
  private readonly activeChats = new Map<string, string>();
  private readonly activeContexts = new Map<string, {chatId: string; accountId: string; conversationId: string; authorId: string}>();
  private autoError = "";
  private repliedCount = 0;

  /** Binds a stored account to the ordinary cross-platform host network port. */
  constructor(private readonly config: Config) {
    this.api = new WeChatApi(this.http, config.account, this.sessions.recordSend("wechat"));
    config.failedDeliveries ??= [];
    config.activeDeliveries ??= [];
    const interrupted = [...config.activeDeliveries, ...(config.delivery ? [config.delivery] : [])];
    if (interrupted.length > 0) {
      const recovered = interrupted.map((item) => item.state === "processing"
        ? { ...item, state: "failed" as const, error: "上次运行在回复完成前结束，请确认已发送内容后选择重试或跳过。" }
        : item);
      config.failedDeliveries = [...config.failedDeliveries, ...recovered];
      config.activeDeliveries = [];
      config.delivery = null;
      this.autoError = recovered[0].error;
    }
  }

  /** Exposes operational state while never returning the bearer token. */
  status(): Status {
    const account = this.config.account;
    const allDeliveries = [...this.config.failedDeliveries, ...this.config.activeDeliveries];
    const delivery = allDeliveries[0] ?? null;
    return {
      account:
        account === null
          ? null
          : { botId: account.botId, userId: account.userId },
      login: { ...this.login },
      listenerEnabled: this.config.listenerEnabled,
      service: {
        state: this.state,
        error: this.lastError,
        receivedCount: this.receivedCount,
      },
      queue: { size: this.config.queue.length, capacity: MAX_EVENTS },
      autoReply: {
        config: { ...this.config.autoReply },
        running: this.autoTasks.size > 0 || this.autoTimer !== null,
        error: this.autoError,
        repliedCount: this.repliedCount,
        bindingCount: Object.keys(this.config.bindings).length,
        deliveries: allDeliveries.map((item) => ({ messageId: item.event.messageId, userId: item.event.userId, state: item.state, delivered: item.delivered, error: item.error })),
        delivery:
          delivery === null
            ? null
            : {
                messageId: delivery.event.messageId,
                userId: delivery.event.userId,
                state: delivery.state,
                delivered: delivery.delivered,
                error: delivery.error,
              },
      },
    };
  }

  /** Starts QR acquisition and returns as soon as a displayable QR value exists. */
  async loginStart(startOnConfirm = false): Promise<void> {
    await this.cancelLogin();
    await this.stopRuntime();
    this.config.listenerEnabled = false;
    await PluginConfig.flush(this.config);
    this.login = {
      state: "wait",
      qrContent: "",
      expiresAt: Date.now() + 300000,
      error: "",
    };
    try {
      const response = await this.loginApi.loginStart();
      this.login.qrContent = nonblank(
        response.qrcode_img_content,
        "qrcode_img_content",
      );
      this.loginSession = {
        qrcode: nonblank(response.qrcode, "qrcode"),
        baseUrl: API_BASE,
        verifyCode: "", startOnConfirm,
      };
      this.loginTask = this.pollLogin(++this.loginGeneration);
    } catch (error) {
      this.login.state = "error";
      this.login.error = this.diagnostic(error);
      throw new Error(this.login.error);
    }
  }

  /** Polls official QR status, handles verification and stores only confirmed credentials. */
  private async pollLogin(generation: number): Promise<void> {
    try {
      while (
        generation === this.loginGeneration &&
        Date.now() < this.login.expiresAt
      ) {
        const session = this.loginSession;
        if (session === null) throw new Error("微信二维码会话已取消。");
        const result = await this.loginApi.loginPoll(
          session.baseUrl,
          session.qrcode,
          session.verifyCode,
        );
        if (generation !== this.loginGeneration) return;
        const state = text(
          result.status,
          "qrcode status",
        ) as LoginStatus["state"];
        this.login.state = state;
        if (state === "wait" || state === "scaned") {
          await new Promise<void>((resolve) => {
            const timer = setTimeout(() => {
              this.loginDelay = null;
              resolve();
            }, 500);
            this.loginDelay = { timer, resolve };
          });
          continue;
        }
        if (state === "need_verifycode") {
          this.login.error =
            "微信已要求验证码，请在设置页填写手机上显示的验证码。";
          return;
        }
        if (state === "scaned_but_redirect") {
          session.baseUrl = wechatUrl(
            `https://${nonblank(result.redirect_host, "redirect_host")}`,
            "redirect_host",
          );
          this.login.state = "scaned_but_redirect";
          continue;
        }
        if (state === "expired") {
          this.login.error = "二维码已过期，请重新获取。";
          return;
        }
        if (state === "verify_code_blocked" || state === "binded_redirect") {
          this.login.error =
            state === "verify_code_blocked"
              ? "验证码尝试次数过多，请重新获取二维码。"
              : "此 Bot 已绑定当前实例，未返回新的凭据。";
          if (state === "binded_redirect" && this.config.account !== null) {
            this.login.state = "confirmed";
            this.login.qrContent = "";
            this.loginSession = null;
          }
          return;
        }
        if (state === "confirmed") {
          const account = {
            botId: nonblank(result.ilink_bot_id, "ilink_bot_id"),
            userId: nonblank(result.ilink_user_id, "ilink_user_id"),
            token: nonblank(result.bot_token, "bot_token"),
            baseUrl: wechatUrl(result.baseurl, "baseurl"),
          };
          if (this.config.account?.botId !== account.botId) {
            this.config.cursor = "";
            this.config.queue = [];
            this.config.seen = [];
            this.config.contexts = {};
            this.config.history = [];
            this.config.delivery = null;
            this.config.activeDeliveries = [];
            this.config.failedDeliveries = [];
          }
          this.config.account = account;
          this.api = new WeChatApi(this.http, account, this.sessions.recordSend("wechat"));
          this.login.qrContent = "";
          this.login.error = "扫码登录成功。";
          this.loginSession = null;
          await PluginConfig.flush(this.config);
          if (session.startOnConfirm && generation === this.loginGeneration) {
            await this.start().catch(error => { this.lastError = this.diagnostic(error); });
          }
          return;
        }
        throw new Error(`未知微信二维码状态：${state}`);
      }
      if (
        generation === this.loginGeneration &&
        this.login.state !== "confirmed"
      ) {
        this.login.state = "expired";
        this.login.error = "二维码轮询已超时，请重新获取。";
      }
    } catch (error) {
      if (generation === this.loginGeneration) {
        this.login.state = "error";
        this.login.error = this.diagnostic(error);
      }
    } finally {
      if (generation === this.loginGeneration) this.loginTask = null;
    }
  }

  /** Cancels only the QR poll and leaves an already confirmed account untouched. */
  async cancelLogin(): Promise<void> {
    ++this.loginGeneration;
    if (this.loginDelay !== null) {
      clearTimeout(this.loginDelay.timer);
      this.loginDelay.resolve();
      this.loginDelay = null;
    }
    if (this.loginTask !== null) await this.loginTask;
    this.loginTask = null;
    this.loginSession = null;
    if (this.login.state !== "confirmed")
      this.login = { state: "idle", qrContent: "", expiresAt: 0, error: "" };
  }

  /** Submits the exact verification code required by the currently scanned QR. */
  async loginVerify(code: string): Promise<void> {
    if (this.login.state !== "need_verifycode" || this.loginTask !== null)
      throw new Error("当前没有等待验证码的微信登录。");
    if (this.loginSession === null)
      throw new Error("微信二维码会话已结束，请重新获取。");
    this.loginSession.verifyCode = nonblank(code, "verify code");
    this.login.error = "";
    this.login.state = "scaned";
    this.loginTask = this.pollLogin(++this.loginGeneration);
  }

  /** Starts one serialized long-poll loop and optional automatic reply worker. */
  async start(restart = false): Promise<void> {
    if (this.config.account === null)
      throw new Error("请先扫码登录微信机器人。");
    if (restart) await this.stopRuntime();
    if (this.pollTask !== null) return;
    this.state = "connecting";
    this.lastError = "";
    try {
      await this.api.post("msg/notifystart");
    } catch (error) {
      if (error instanceof WeChatApiError && error.code === -14) {
        this.state = "error";
        this.lastError = this.diagnostic(error);
        throw new Error(this.lastError);
      }
      // Lifecycle notifications are best-effort in the iLink protocol.
      this.lastError = this.diagnostic(error);
    }
    this.config.listenerEnabled = true;
    await PluginConfig.flush(this.config);
    const generation = ++this.pollGeneration;
    this.pollTask = this.poll(generation).catch((error) => {
      this.state = "error";
      this.lastError = this.diagnostic(error);
    });
    if (this.config.autoReply.enabled) this.startAuto();
    await Promise.resolve();
  }

  /** Repeats getUpdates with the server cursor and persists it atomically with events. */
  private async poll(generation: number): Promise<void> {
    try {
      let timeoutSeconds = 45, failures = 0;
      while (
        generation === this.pollGeneration &&
        this.config.listenerEnabled
      ) {
        let response: JsonObject;
        try { response = await this.api.updates(this.config.cursor, timeoutSeconds); }
        catch (error) {
          const retryable = error instanceof WeChatTransportError || (error instanceof WeChatApiError && error.code !== -14);
          if (!retryable || generation !== this.pollGeneration) throw error;
          this.state = "connecting";
          this.lastError = `网络中断，正在重连：${this.diagnostic(error)}`;
          await this.waitPoll(Math.min(30000, 1000 * 2 ** Math.min(failures++, 5)));
          continue;
        }
        failures = 0;
        if (generation !== this.pollGeneration) return;
        const messages = response.msgs === undefined ? [] : response.msgs;
        if (!Array.isArray(messages))
          throw new Error("微信 getUpdates 的 msgs 必须是数组。");
        const cursor = response.get_updates_buf === undefined ? "" : text(response.get_updates_buf, "get_updates_buf");
        const seen = new Set(this.config.seen);
        const incoming: InboundEvent[] = [];
        for (const value of messages) {
          let event: InboundEvent;
          try {
            const raw = object(value, "WeChat message");
            if (raw.message_type !== 1 || (raw.message_state !== undefined && raw.message_state !== 2) || (typeof raw.group_id === "string" && raw.group_id)) continue;
            event = inbound(raw);
          } catch (error) {
            console.warn("[wechat.inbound]", this.diagnostic(error));
            continue;
          }
          // The same remote message ID in another conversation is a different
          // message. Legacy plain IDs remain conservative during migration.
          const identity = JSON.stringify([this.api.requireAccount().botId, event.userId, event.messageId]);
          if (seen.has(identity) || seen.has(event.messageId)) continue;
          seen.add(identity);
          incoming.push(event);
        }
        // Route explicitly selected maintenance messages to the durable channel inbox
        // alone when automatic replies are disabled. Other messages retain the normal queue.
        const capture = this.config.channelCapture === true;
        const ids = this.config.channelConversationIds;
        const selected = (event: InboundEvent): boolean => capture && (!Array.isArray(ids) || ids.includes(event.userId));
        const channelOnly = this.config.channelOnlyWhenNoAuto === true && !this.config.autoReply.enabled;
        const queued = channelOnly ? incoming.filter(event => !selected(event)) : incoming;
        if (this.config.autoReply.enabled && this.config.queue.length + queued.length > MAX_EVENTS)
          throw new Error(
            "微信事件队列已满，请先消费或清空事件，再启动机器人。",
          );
        const accountId=nonblank(this.config.account?.botId,"botId");
        for(const event of incoming){
          const message:ChannelMessage={key:messageKey(["wechat",accountId,event.userId,event.messageId]),channel:"wechat",accountId,
            messageId:event.messageId,conversationId:event.userId,authorId:event.userId,text:event.text,timestamp:String(event.timestamp),
            autoReplyEnabled:this.config.autoReply.enabled,context:JSON.parse(JSON.stringify({contextToken:event.contextToken,items:event.items,raw:event.raw}))};
          await this.sessions.observe(message,"c2c",`微信私聊 ${event.userId}`,this.config.bindings[`${accountId}:${event.userId}`]);
          if(selected(event))await this.channelInbox.record(message);
        }
        this.config.queue.push(...queued);
        if (!this.config.autoReply.enabled) this.config.queue = this.config.queue.slice(-MAX_EVENTS);
        this.config.seen = [...seen].slice(-512);
        this.config.history = [...this.config.history, ...incoming].slice(-512);
        for (const event of incoming)
          this.config.contexts[event.userId] = event.contextToken;
        if (cursor !== "") this.config.cursor = cursor;
        this.receivedCount += incoming.length;
        await PluginConfig.flush(this.config);
        this.state = "ready";
        this.lastError = "";
        if (response.longpolling_timeout_ms !== undefined) {
          const duration = integer(
            response.longpolling_timeout_ms,
            "longpolling_timeout_ms",
            1000,
            120000,
          );
          timeoutSeconds = Math.ceil(duration / 1000) + 10;
        }
        await this.waitPoll(100);
      }
    } catch (error) {
      if (generation === this.pollGeneration) {
        this.state = "error";
        this.lastError = this.diagnostic(error);
        if (error instanceof WeChatApiError && error.code === -14) {
          this.config.listenerEnabled = false;
          this.lastError = "微信登录已失效，请重新扫码登录。";
          await PluginConfig.flush(this.config);
        }
        await this.stopAuto();
      }
    } finally {
      if (generation === this.pollGeneration) this.pollTask = null;
    }
  }

  private waitPoll(ms: number): Promise<void> {
    return new Promise(resolve => {
      const timer = setTimeout(() => { this.pollDelay = null; resolve(); }, ms);
      this.pollDelay = { timer, resolve };
    });
  }

  /** Stops the poll, model generation and typing state before releasing the runtime. */
  async stopRuntime(): Promise<void> {
    ++this.pollGeneration;
    if (this.pollDelay !== null) {
      clearTimeout(this.pollDelay.timer);
      this.pollDelay.resolve();
      this.pollDelay = null;
    }
    await this.stopAuto();
    if (this.pollTask !== null) await this.pollTask;
    this.pollTask = null;
    const wasRunning = this.state === "ready" || this.state === "connecting";
    this.state = "stopped";
    if (wasRunning && this.config.account !== null)
      await this.api.post("msg/notifystop");
  }

  /** Starts bounded workers after the authenticated poll is alive. */
  private startAuto(): void {
    if (this.autoTasks.size > 0 || this.autoTimer !== null) return;
    const generation = ++this.autoGeneration;
    this.scheduleAuto(generation);
  }

  /** Repeatedly claims at most one message per peer, up to three peers. */
  private scheduleAuto(generation: number): void {
    if (generation !== this.autoGeneration || !this.config.autoReply.enabled || this.autoTimer !== null) return;
    this.autoTimer = setTimeout(() => {
      this.autoTimer = null;
      while (this.autoTasks.size < 3 && generation === this.autoGeneration && this.state === "ready") {
        const event = this.claimAutoEvent();
        if (event === null) break;
        this.launchAuto(event, generation);
      }
      this.scheduleAuto(generation);
    }, 300);
  }

  /** Reserves a peer synchronously before starting asynchronous persistence and model work. */
  private claimAutoEvent(): InboundEvent | null {
    if (this.config.failedDeliveries.length + this.config.activeDeliveries.length >= MAX_EVENTS) return null;
    const blockedUsers = new Set([...this.config.failedDeliveries, ...this.config.activeDeliveries].map((item) => item.event.userId));
    for (const userId of this.autoTasks.keys()) blockedUsers.add(userId);
    const index = this.config.queue.findIndex((item) => !blockedUsers.has(item.userId));
    if (index < 0) return null;
    const event = this.config.queue[index];
    this.config.queue = this.config.queue.filter((_, position) => position !== index);
    this.config.activeDeliveries = [...this.config.activeDeliveries, { event, state: "processing", delivered: 0, error: "" }];
    return event;
  }

  /** Keeps failures in durable records; a persistence error stops the entire worker pool. */
  private launchAuto(event: InboundEvent, generation: number): Promise<void> {
    const task = this.autoTick(event, generation);
    this.autoTasks.set(event.userId, task);
    void task.then(() => {
      this.autoTasks.delete(event.userId);
    }, (error) => {
      this.autoTasks.delete(event.userId);
      this.autoError = this.diagnostic(error);
      void this.stopAuto().catch((stopError) => console.error("[wechat.stop]", this.diagnostic(stopError)));
    });
    return task;
  }

  /** Cancels every active Chat and waits for all records to settle. */
  private async stopAuto(): Promise<void> {
    ++this.autoGeneration;
    if (this.autoTimer !== null) {
      clearTimeout(this.autoTimer);
      this.autoTimer = null;
    }
    await Promise.all([...this.activeChats.values()].map((chatId) => Tools.Chat.cancel(chatId).catch((error) => console.error("[wechat.cancel]", this.diagnostic(error)))));
    await Promise.allSettled([...this.autoTasks.values()]);
  }

  /** Converts one reserved inbound message to an isolated native chat and delivers all segments. */
  private async autoTick(event: InboundEvent, generation: number): Promise<void> {
    const config = this.config.autoReply;
    const binding = `${this.api.requireAccount().botId}:${event.userId}`;
    const current = () => this.config.activeDeliveries.find((item) => item.event === event);
    const update = (delivered: number) => {
      this.config.activeDeliveries = this.config.activeDeliveries.map((item) => item.event === event ? { ...item, delivered } : item);
    };
    let timedOut = false;
    let finishedDelivery: (typeof this.config.activeDeliveries)[number] | undefined;
    try {
      await PluginConfig.flush(this.config);
      await this.conversations();
      const session=await this.openChat(conversationKey("wechat",this.api.requireAccount().botId,"c2c",event.userId));
      const chatId=session.chatId!;
      if (generation !== this.autoGeneration)
        throw new Error("微信自动回复已停止。");
      this.activeChats.set(event.userId, chatId);
      this.activeContexts.set(event.userId, {chatId, accountId: this.api.requireAccount().botId, conversationId: event.userId, authorId: event.userId});
      const attachments = [];
      let failedMedia = 0;
      for (const item of event.items.flatMap(item => {
        const quote = item.ref_msg as JsonObject | undefined;
        return quote?.message_item && typeof quote.message_item === "object" ? [item, quote.message_item as JsonObject] : [item];
      }).slice(0, 20)) {
        if (generation !== this.autoGeneration) return;
        try {
          if (mediaReference(item) !== null) attachments.push(await downloadMedia(this.api, item, event.messageId));
        } catch (error) {
          failedMedia++;
          console.error("[wechat.media]", this.diagnostic(error));
        }
      }
      if (generation !== this.autoGeneration)
        throw new Error("微信自动回复已停止。");
      const timer = setTimeout(() => {
        timedOut = true;
        void Tools.Chat.cancel(chatId!).catch((error) =>
          console.error("[wechat.timeout]", this.diagnostic(error)),
        );
      }, config.aiTimeoutMs);
      try {
        const input = {
          kind: "submit" as const,
          chatId,
          runtime: "main" as const,
          input: {
            text: `${this.messageText(event)}${failedMedia ? `\n\n[${failedMedia} 个附件暂时无法读取，请勿编造内容]` : ""}\n\n${config.assistantInstruction}`,
            attachments,
            replyToMessageTimestamp: null,
          },
          turn: { kind: "execute" as const },
          notifyReply: false,
        };
        for await (const output of replyStream(input, config.waifuProfile)) {
          if (generation !== this.autoGeneration)
            throw new Error("微信自动回复已停止。");
          if (timedOut) throw new Error("微信自动回复已超时。");
          if (output.type !== "segment") continue;
          if (output.segment.kind === "text") {
            const characters = Array.from(output.segment.text);
            for (let start = 0; start < characters.length; start += 4000) {
              if (generation !== this.autoGeneration || timedOut)
                throw new Error("微信自动回复已停止或超时。");
              await this.api.sendText(
                event.userId,
                event.contextToken,
                characters.slice(start, start + 4000).join(""),
              );
              update((current()?.delivered ?? 0) + 1);
              await PluginConfig.flush(this.config);
            }
          } else {
            const item = await uploadMedia(
              this.api,
              event.userId,
              "image",
              output.segment.source,
            );
            if (generation !== this.autoGeneration || timedOut)
              throw new Error("微信自动回复已停止或超时。");
            await this.api.send(event.userId, event.contextToken, item);
            update((current()?.delivered ?? 0) + 1);
            await PluginConfig.flush(this.config);
          }
        }
        if (timedOut) throw new Error("微信自动回复已超时。");
      } finally {
        clearTimeout(timer);
      }
      if ((current()?.delivered ?? 0) === 0)
        throw new Error("模型没有产生可发送的回复。");
      finishedDelivery = current();
      this.config.activeDeliveries = this.config.activeDeliveries.filter((item) => item.event !== event);
      await PluginConfig.flush(this.config);
      this.repliedCount++;
    } catch (error) {
      const message = timedOut
        ? "微信自动回复已超时。"
        : generation !== this.autoGeneration
          ? "微信自动回复已停止，请核对已发送内容后处理。"
          : this.diagnostic(error);
      this.autoError = message;
      const failed = current() ?? finishedDelivery;
      if (failed) {
        this.config.failedDeliveries = [...this.config.failedDeliveries, { ...failed, state: "failed", error: message }];
        this.config.activeDeliveries = this.config.activeDeliveries.filter((item) => item.event !== event);
      }
      await PluginConfig.flush(this.config);
    } finally {
      this.activeChats.delete(event.userId);
      this.activeContexts.delete(event.userId);
    }
  }

  /** Reads selected queue entries without exposing raw payloads unless requested. */
  async receive(params: ReceiveParams): Promise<Response> {
    if (params.consume !== undefined && typeof params.consume !== "boolean") {
      throw new Error("consume must be boolean");
    }
    if (
      params.include_raw !== undefined &&
      typeof params.include_raw !== "boolean"
    ) {
      throw new Error("include_raw must be boolean");
    }
    if (params.user_id !== undefined) nonblank(params.user_id, "user_id");
    const limit =
      params.limit === undefined ? 20 : integer(params.limit, "limit", 1, 100);
    const selected = this.config.queue
      .filter(
        (event) =>
          params.user_id === undefined || event.userId === params.user_id,
      )
      .slice(0, limit);
    if (params.consume !== false && selected.length > 0) {
      const consumed = new Set(selected);
      this.config.queue = this.config.queue.filter((event) => !consumed.has(event));
      await PluginConfig.flush(this.config);
    }
    const events: JsonObject[] = selected.map((event) =>
      params.include_raw === true
        ? { ...event }
        : {
            messageId: event.messageId,
            userId: event.userId,
            text: event.text,
            timestamp: event.timestamp,
            contextToken: event.contextToken,
            items: event.items.map((item) => ({ type: item.type })),
          },
    );
    return { success: true, events, remainingCount: this.config.queue.length };
  }

  /** Persists validated automatic reply settings and starts only when explicitly enabled. */
  async configureAuto(params: AutoReplyParams): Promise<void> {
    const next = { ...this.config.autoReply };
    if (params.enabled !== undefined) {
      if (typeof params.enabled !== "boolean")
        throw new Error("enabled must be boolean");
      next.enabled = params.enabled;
    }
    if (params.ai_timeout_ms !== undefined)
      next.aiTimeoutMs = integer(
        params.ai_timeout_ms,
        "ai_timeout_ms",
        1000,
        600000,
      );
    if (params.chat_group !== undefined)
      next.chatGroup = text(params.chat_group, "chat_group").trim();
    if (params.character_selection !== undefined)
      next.characterSelection = text(
        params.character_selection,
        "character_selection",
      ).trim();
    if (params.assistant_instruction !== undefined)
      next.assistantInstruction = text(
        params.assistant_instruction,
        "assistant_instruction",
      );
    if (params.waifu_profile !== undefined)
      next.waifuProfile = nonblank(params.waifu_profile, "waifu_profile");
    if (next.enabled && next.waifuProfile !== "native") await waifuSettings(next.waifuProfile);
    if (!next.enabled) await this.stopAuto();
    this.config.autoReply = next;
    await PluginConfig.flush(this.config);
    if (next.enabled && this.pollTask !== null) this.startAuto();
  }

  /** Routes one tool or settings operation through the single service owner. */
  async request(request: Request): Promise<Response> {
    switch (request.action) {
      case "status":
        return { success: true, status: this.status() };
      case "login_start":
        await this.loginStart();
        break;
      case "login_cancel":
        await this.cancelLogin();
        break;
      case "login_verify":
        await this.loginVerify(request.code);
        break;
      case "logout":
        await this.logout();
        break;
      case "start":
        await this.start(request.restart === true);
        break;
      case "stop":
        this.config.listenerEnabled = false;
        await PluginConfig.flush(this.config);
        await this.stopRuntime();
        break;
      case "test":
        if (this.config.account === null) throw new Error("请先扫码登录");
        await this.api.post("getconfig", {
          ilink_user_id: this.config.account.userId,
        });
        break;
      case "receive":
        return this.receive(request.params);
      case "clear": {
        const count = this.config.queue.length;
        this.config.queue = [];
        await PluginConfig.flush(this.config);
        return { success: true, clearedCount: count };
      }
      case "send_text":
        return {
          success: true,
          response: await this.api.sendText(
            request.params.user_id,
            this.replyContext(
              request.params.user_id,
              request.params.context_token,
            ),
            nonblank(request.params.text, "text"),
          ),
        };
      case "send_media":
        return {
          success: true,
          response: await this.sendMedia(request.params),
        };
      case "typing":
        await this.api.typing(
          request.user_id,
          this.config.contexts[request.user_id] ?? "",
          request.active,
        );
        break;
      case "auto_configure":
        await this.configureAuto(request.params);
        break;
      case "auto_once": {
        if (this.state !== "ready") throw new Error("请先启动微信监听。");
        if (this.autoTasks.size > 0 || this.autoTimer !== null)
          throw new Error("自动回复已在运行，请先关闭自动回复。");
        const event = this.claimAutoEvent();
        if (event !== null) {
          this.autoError = "";
          await this.launchAuto(event, this.autoGeneration);
        }
        if (this.autoError !== "") throw new Error(this.autoError);
        break;
      }
      case "delivery_resolve": {
        const failed = this.config.failedDeliveries;
        if (request.resolution !== "retry" && request.resolution !== "discard")
          throw new Error("resolution must be retry or discard");
        if ((request.message_id === undefined) !== (request.user_id === undefined))
          throw new Error("message_id and user_id must be provided together");
        if (request.message_id === undefined && failed.length > 1)
          throw new Error("存在多条失败回复，请指定用户和消息。");
        const index = request.message_id === undefined ? 0 : failed.findIndex(
          (item) => item.event.messageId === request.message_id && item.event.userId === request.user_id,
        );
        if (index < 0 || index >= failed.length) throw new Error("没有找到待处理的失败回复");
        if (request.resolution === "retry") {
          if (this.config.queue.length >= MAX_EVENTS)
            throw new Error("消息队列已满，请先消费消息再重试。");
          this.config.queue = [failed[index].event, ...this.config.queue];
        }
        this.config.failedDeliveries = failed.filter((_, position) => position !== index);
        this.autoError = this.config.failedDeliveries[0]?.error ?? "";
        await PluginConfig.flush(this.config);
        if (this.state === "ready" && this.config.autoReply.enabled)
          this.startAuto();
        break;
      }
      case "reset_conversation": {
        await this.conversations();
        if (this.autoTasks.size > 0)
          throw new Error("请先停止正在进行的自动回复。");
        const key = `${this.api.requireAccount().botId}:${nonblank(request.user_id, "user_id")}`;
        const bindings = { ...this.config.bindings };
        delete bindings[key];
        this.config.bindings = bindings;
        await PluginConfig.flush(this.config);
        await this.sessions.bind(conversationKey("wechat",this.api.requireAccount().botId,"c2c",request.user_id),null);
        break;
      }
      case "download_media": {
        const event = [...this.config.queue, ...this.config.history].find(
          (value) => value.messageId === request.message_id,
        );
        if (event === undefined) throw new Error("找不到指定消息");
        const files = [];
        for (const item of event.items) {
          if (mediaReference(item) !== null)
            files.push(await downloadMedia(this.api, item, event.messageId));
        }
        if (files.length === 0) throw new Error("消息不包含媒体");
        return { success: true, response: { files } };
      }
      case "reply_roles": {
        const [cards, groups] = await Promise.all([
          ToolPkg.callDependency<
            Record<string, never>,
            { id: string; name: string }[]
          >("com.operit.character_cards", "character.list", {}),
          ToolPkg.callDependency<
            Record<string, never>,
            { id: string; name: string }[]
          >("com.operit.character_cards", "group.list", {}),
        ]);
        return {
          success: true,
          roles: [
            ...cards.map((card) => ({
              selection: `card:${card.id}`,
              name: card.name,
            })),
            ...groups.map((group) => ({
              selection: `group:${group.id}`,
              name: `${group.name}（角色组）`,
            })),
          ],
        };
      }
      default:
        throw new Error("未知微信 Bot 操作");
    }
    return { success: true, status: this.status() };
  }

  /** Restores explicitly enabled polling after application creation or foreground entry. */
  restore(): Promise<void> {
    return this.config.listenerEnabled ? this.start() : Promise.resolve();
  }

  /** Stops both background tasks and discards all credential-bearing account state. */
  private async logout(): Promise<void> {
    await this.cancelLogin();
    this.config.listenerEnabled = false;
    await this.stopRuntime();
    this.config.account = null;
    this.config.cursor = "";
    this.config.contexts = {};
    this.config.queue = [];
    this.config.history = [];
    this.config.seen = [];
    this.config.delivery = null;
    this.config.activeDeliveries = [];
    this.config.failedDeliveries = [];
    this.api = new WeChatApi(this.http);
    this.login = { state: "idle", qrContent: "", expiresAt: 0, error: "" };
    this.autoError = "";
    await PluginConfig.flush(this.config);
  }

  /** Requires an explicit or previously received conversation context before sending media. */
  private replyContext(userId: string, submitted?: string): string {
    nonblank(userId, "user_id");
    if (submitted !== undefined) return nonblank(submitted, "context_token");
    return nonblank(
      this.config.contexts[userId],
      "当前用户尚无 context_token，请先让对方发送消息",
    );
  }

  /** Uploads validated media and sends optional caption text as a separate message. */
  private async sendMedia(params: MediaParams): Promise<JsonObject> {
    this.api.requireAccount();
    const context = this.replyContext(params.user_id, params.context_token);
    if (
      params.text !== undefined &&
      Array.from(nonblank(params.text, "text")).length > 4000
    )
      throw new Error("媒体说明不能超过 4000 个字符。");
    const item = await uploadMedia(
      this.api,
      params.user_id,
      params.kind,
      params.source,
      params,
    );
    const response = await this.api.send(params.user_id, context, item);
    if (params.text !== undefined) {
      try {
        await this.api.sendText(params.user_id, context, params.text);
      } catch (error) {
        throw new Error(
          `媒体已发送，但说明文字发送失败：${this.diagnostic(error)}`,
        );
      }
    }
    return response;
  }

  /** Releases QR polling as well as account networking during host termination. */
  async shutdown(): Promise<void> {
    await this.cancelLogin();
    await this.stopRuntime();
  }

  /** Resolves explicit quoted message items and locally retained server message IDs. */
  private messageText(event: InboundEvent): string {
    const quotes: string[] = [];
    for (const item of event.items) {
      if (item.ref_msg === undefined) continue;
      const ref = object(item.ref_msg, "ref_msg");
      if (ref.message_item !== undefined) {
        const quoted = object(ref.message_item, "ref_msg.message_item");
        if (quoted.type === 1)
          quotes.push(
            text(
              object(quoted.text_item, "quoted text_item").text,
              "quoted text",
            ),
          );
      } else if (ref.svr_id !== undefined) {
        const id = identifier(ref.svr_id, "ref_msg.svr_id");
        const previous = this.config.history.find(
          (value) => value.messageId === id && value.userId === event.userId,
        );
        if (previous?.text) quotes.push(previous.text);
      }
      if (ref.title !== undefined)
        quotes.push(text(ref.title, "ref_msg.title"));
    }
    return [...quotes.map((value) => `[引用]\n${value}`), event.text].join(
      "\n\n",
    );
  }

  private async conversations():Promise<ChannelConversation[]>{
    const accountId=this.config.account?.botId;if(!accountId)return [];
    for(const [binding,chatId] of Object.entries(this.config.bindings)){
      if(!binding.startsWith(accountId+":"))continue;
      const id=binding.slice(accountId.length+1);
      await this.sessions.remember({key:conversationKey("wechat",accountId,"c2c",id),channel:"wechat",accountId,scene:"c2c",conversationId:id,
        title:`微信私聊 ${id}`,chatId,lastMessageAt:0});
    }
    return this.sessions.conversations("wechat",accountId);
  }
  private openChat(key:string):Promise<ChannelConversation>{
    const existing=this.opening.get(key);if(existing)return existing;
    const task=(async()=>{
      const session=await this.sessions.get(key);
      if(session.accountId!==this.config.account?.botId)throw Error("会话不属于当前微信账号");
      if(session.chatId)return session;
      const retained=this.config.bindings[`${session.accountId}:${session.conversationId}`];
      if(retained)return this.sessions.bind(key,retained);
      const config=this.config.autoReply,input=config.characterSelection===""?null:{"com.operit.character_cards":{version:1,selection:config.characterSelection}};
      const chatId=(await Tools.Chat.createNew({setAsCurrentChat:false,input})).chatId;
      this.config.bindings={...this.config.bindings,[`${session.accountId}:${session.conversationId}`]:chatId};await PluginConfig.flush(this.config);
      const bound=await this.sessions.bind(key,chatId);
      await Tools.Chat.updateTitle(chatId,session.title);if(config.chatGroup)await Tools.Chat.updateGroup([chatId],config.chatGroup);
      return bound;
    })();this.opening.set(key,task);void task.finally(()=>this.opening.delete(key)).catch(()=>{});return task;
  }

  /** Independent public channel contract, separate from the automatic-reply queue. */
  async channel(request: ChannelRequest): Promise<ChannelResponses[ChannelRequest["action"]]> {
    if (request === null || typeof request !== "object") throw new Error("Invalid channel request");
    switch (request.action) {
      case "conversations": return {conversations:await this.conversations()};
      case "open_chat": return this.openChat(request.conversationKey);
      case "history": {
        const session=await this.sessions.get(request.conversationKey);
        if(session.accountId!==this.config.account?.botId)throw Error("会话不属于当前微信账号");
        return this.sessions.history(session.key,request.limit,request.before,undefined,true);
      }
      case "attachment": {
        const requestApi = this.api;
        const session = await this.sessions.get(request.conversationKey);
        if (session.accountId !== requestApi.requireAccount().botId || requestApi !== this.api) throw Error("微信会话不属于当前账户");
        const item = await this.sessions.attachment(session.key,request.messageId,request.index);
        const bytes = await downloadMediaContent(requestApi,item,8 * 1024 * 1024);
        if (requestApi !== this.api) throw Error("微信账户已切换");
        return bytes;
      }
      case "subscribe": return this.sessions.subscribe(request.consumerId,request.routes,"wechat",this.config.account?.botId??"");
      case "configure": {
        if (typeof request.capture !== "boolean") throw new Error("capture must be boolean");
        if (request.conversationIds !== undefined && request.conversationIds !== null &&
            (!Array.isArray(request.conversationIds) || request.conversationIds.some(id => typeof id !== "string" || id.trim() === "")))
          throw new Error("conversationIds must be null or an array of non-empty strings");
        if (request.channelOnlyWhenNoAuto !== undefined && typeof request.channelOnlyWhenNoAuto !== "boolean") throw new Error("channelOnlyWhenNoAuto must be boolean");
        const previous = this.config.channelCapture, previousIds = this.config.channelConversationIds, previousOnly = this.config.channelOnlyWhenNoAuto;
        this.config.channelCapture = request.capture;
        if (request.channelOnlyWhenNoAuto !== undefined) this.config.channelOnlyWhenNoAuto = request.channelOnlyWhenNoAuto;
        if (request.conversationIds !== undefined) this.config.channelConversationIds = request.conversationIds === null ? null : [...new Set(request.conversationIds)];
        try { await PluginConfig.flush(this.config); } catch (error) { this.config.channelCapture = previous; this.config.channelConversationIds = previousIds; this.config.channelOnlyWhenNoAuto = previousOnly; throw error; }
        return { capture: this.config.channelCapture, conversationIds: this.config.channelConversationIds ?? null, channelOnlyWhenNoAuto: this.config.channelOnlyWhenNoAuto === true };
      }
      case "status": return { channel: "wechat", capture: this.config.channelCapture === true, conversationIds: this.config.channelConversationIds ?? null, channelOnlyWhenNoAuto: this.config.channelOnlyWhenNoAuto === true, service: this.state,
        configured: this.config.account !== null, listenerEnabled: this.config.listenerEnabled, autoReplyEnabled: this.config.autoReply.enabled, error: this.lastError || this.autoError };
      case "manage": {
        const command = request.command;
        if (!["start", "stop", "test", "login_start", "login_cancel", "login_status", "login_verify", "logout"].includes(command)) throw Error("Unknown WeChat management command");
        if (command === "start" && "autoReply" in request && request.autoReply !== undefined) await this.configureAuto({ enabled: request.autoReply });
        if (command === "login_start") await this.loginStart(true);
        else if (command === "login_verify") await this.request({ action: command, code: request.code });
        else if (command !== "login_status") await this.request({ action: command });
        const status = await this.channel({ action: "status" });
        return { ...status, login: { ...this.login } } as ChannelResponses["manage"];
      }
      case "active_context": {
        const chatId = nonblank(request.chatId, "chatId");
        const matches = [...this.activeContexts.values()].filter(value => value.chatId === chatId);
        return matches.length === 1 ? {channel: "wechat", ...matches[0]} : null;
      }
      case "peek": return { messages: (request.consumerId===undefined ? await this.channelInbox.peek(request.limit, request.conversationId) : await this.sessions.peek(request.consumerId,request.limit,request.conversationKey)).filter(message=>message.accountId===this.config.account?.botId) };
      case "ack": return { acknowledged: request.consumerId===undefined ? await this.channelInbox.ack(request.key) : await this.sessions.ack(request.consumerId,request.key) };
      case "reply": {
        const message = await this.channelInbox.get(request.key) ?? await this.sessions.replyRoute(request.key);
        if (message === null) throw new Error("Pending message not found");
        if (message.accountId !== this.config.account?.botId) throw new Error("Message belongs to a different WeChat account");
        const { media } = request;
        if (media) return { response: await this.sendMedia({ user_id: message.conversationId, source: media.source,
          kind: media.kind === "audio" ? "voice" : media.kind, file_name: media.fileName,
          duration_ms: media.durationMs, voice_format: media.voiceFormat, sample_rate: media.sampleRate, text: request.text }) };
        return { response: await this.api.sendText(message.conversationId, this.replyContext(message.conversationId), nonblank(request.text, "text")) };
      }
      default: throw new Error("Unknown channel request");
    }
  }

  /** Redacts credentials and conversation tokens before displaying or logging errors. */
  diagnostic(error: unknown, request?: Request): string {
    const secrets = [
      this.config.account?.token ?? "",
      ...Object.values(this.config.contexts),
      this.loginSession?.qrcode ?? "",
      this.loginSession?.verifyCode ?? "",
    ];
    if (request?.action === "send_text" || request?.action === "send_media")
      secrets.push(request.params.context_token ?? "");
    if (request?.action === "login_verify") secrets.push(request.code);
    return errorText(error, secrets);
  }
}

async function* replyStream(input: Parameters<typeof waifuStream>[0], profile: string): ReturnType<typeof waifuStream> {
  if (profile !== "native") { yield* waifuStream(input, profile); return; }
  for await (const item of Tools.Chat.sendMessageStreaming(input)) {
    if (item.type === "completed" && item.result.outcome.type === "committed" && item.result.outcome.status === "completed" && item.result.aiResponse) {
      yield { type: "segment", index: 0, segment: { kind: "text", text: item.result.aiResponse } };
    }
  }
}

let pending: Promise<WeChatService> | null = null;
let commandTail: Promise<void> = Promise.resolve();

/** Initializes one service owner over the durable device configuration. */
async function service(): Promise<WeChatService> {
  if (pending === null)
    pending = PluginConfig.use("wechat", defaultConfig()).then(
      (config) => new WeChatService(config),
    );
  return pending;
}

/** Serializes administrative operations without blocking long polls between commands. */
function command<T>(
  operation: (owner: WeChatService) => Promise<T>,
): Promise<T> {
  const result = commandTail.then(async () => operation(await service()));
  commandTail = result.then(
    () => {},
    () => {},
  );
  return result;
}

/** Receives package IPC requests on the main runtime. */
export function receive(request: Request): Promise<Response> {
  return command(async (owner) => {
    try {
      return await owner.request(request);
    } catch (error) {
      throw new Error(owner.diagnostic(error, request));
    }
  });
}

export function channel(request: ChannelRequest): Promise<ChannelResponses[ChannelRequest["action"]]> {
  return command(async owner => {
    try { return await owner.channel(request); }
    catch (error) { throw new Error(owner.diagnostic(error)); }
  });
}

/** Registers the package service channel. */
export function registerService(): void {
  ToolPkg.ipc.on<Request, Response>(CHANNEL, receive);
}

/** Restores enabled polling during host lifecycle transitions. */
export function restore(): Promise<void> {
  return command((owner) => owner.restore());
}

/** Releases network and model resources at application termination. */
export function terminate(): Promise<void> {
  return command((owner) => owner.shutdown());
}
