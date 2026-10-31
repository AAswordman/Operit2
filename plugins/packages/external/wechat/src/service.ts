import {
  getSettings as waifuSettings,
  stream as waifuStream,
} from "./dependencies/waifu/api.js";
import { WeChatApi, hostHttp, WeChatApiError } from "./api.js";
import { downloadMedia, uploadMedia, mediaReference } from "./media.js";
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
  } | null = null;
  private loginDelay: { timer: number; resolve: () => void } | null = null;
  private pollDelay: { timer: number; resolve: () => void } | null = null;
  private login: LoginStatus = {
    state: "idle",
    qrContent: "",
    expiresAt: 0,
    error: "",
  };
  private autoTask: Promise<void> | null = null;
  private autoTimer: number | null = null;
  private autoGeneration = 0;
  private activeChat: string | null = null;
  private autoError = "";
  private repliedCount = 0;

  /** Binds a stored account to the ordinary cross-platform host network port. */
  constructor(private readonly config: Config) {
    this.api = new WeChatApi(this.http, config.account);
    if (config.delivery?.state === "processing") {
      config.delivery.state = "failed";
      config.delivery.error =
        "上次运行在回复完成前结束，请确认已发送内容后选择重试或跳过。";
      this.autoError = config.delivery.error;
    }
  }

  /** Exposes operational state while never returning the bearer token. */
  status(): Status {
    const account = this.config.account;
    const delivery = this.config.delivery;
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
        running: this.autoTask !== null || this.autoTimer !== null,
        error: this.autoError,
        repliedCount: this.repliedCount,
        bindingCount: Object.keys(this.config.bindings).length,
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
  async loginStart(): Promise<void> {
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
        verifyCode: "",
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
          }
          this.config.account = account;
          this.api = new WeChatApi(this.http, account);
          this.login.qrContent = "";
          this.login.error = "扫码登录成功。";
          this.loginSession = null;
          await PluginConfig.flush(this.config);
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
    if (this.config.autoReply.enabled)
      await waifuSettings(this.config.autoReply.waifuProfile);
    this.state = "connecting";
    this.lastError = "";
    try {
      await this.api.post("msg/notifystart");
    } catch (error) {
      this.state = "error";
      this.lastError = this.diagnostic(error);
      throw new Error(this.lastError);
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
      let timeoutSeconds = 45;
      while (
        generation === this.pollGeneration &&
        this.config.listenerEnabled
      ) {
        const response = await this.api.updates(
          this.config.cursor,
          timeoutSeconds,
        );
        if (generation !== this.pollGeneration) return;
        const messages = response.msgs === undefined ? [] : response.msgs;
        if (!Array.isArray(messages))
          throw new Error("微信 getUpdates 的 msgs 必须是数组。");
        const cursor = text(response.get_updates_buf, "get_updates_buf");
        const seen = new Set(this.config.seen);
        const incoming: InboundEvent[] = [];
        for (const value of messages) {
          const raw = object(value, "WeChat message");
          if (
            raw.message_type !== 1 ||
            (raw.message_state !== undefined && raw.message_state !== 2)
          )
            continue;
          const event = inbound(raw);
          if (seen.has(event.messageId)) continue;
          seen.add(event.messageId);
          incoming.push(event);
        }
        if (this.config.queue.length + incoming.length > MAX_EVENTS)
          throw new Error(
            "微信事件队列已满，请先消费或清空事件，再启动机器人。",
          );
        this.config.queue.push(...incoming);
        this.config.seen = [...seen].slice(-512);
        this.config.history = [...this.config.history, ...incoming].slice(-512);
        for (const event of incoming)
          this.config.contexts[event.userId] = event.contextToken;
        if (cursor !== "") this.config.cursor = cursor;
        this.receivedCount += incoming.length;
        await PluginConfig.flush(this.config);
        this.state = "ready";
        if (response.longpolling_timeout_ms !== undefined) {
          const duration = integer(
            response.longpolling_timeout_ms,
            "longpolling_timeout_ms",
            1000,
            120000,
          );
          timeoutSeconds = Math.ceil(duration / 1000) + 10;
        }
        await new Promise<void>((resolve) => {
          const timer = setTimeout(() => {
            this.pollDelay = null;
            resolve();
          }, 100);
          this.pollDelay = { timer, resolve };
        });
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

  /** Starts automatic replies only after the authenticated poll is alive. */
  private startAuto(): void {
    if (this.autoTask !== null || this.autoTimer !== null) return;
    const generation = ++this.autoGeneration;
    this.scheduleAuto(generation);
  }

  /** Schedules one serialized event consumption pass. */
  private scheduleAuto(generation: number): void {
    if (
      generation !== this.autoGeneration ||
      !this.config.autoReply.enabled ||
      this.config.delivery !== null
    )
      return;
    this.autoTimer = setTimeout(() => {
      this.autoTimer = null;
      this.autoTask = this.autoTick(generation);
      void this.autoTask.then(
        () => {
          this.autoTask = null;
          this.scheduleAuto(generation);
        },
        (error) => {
          this.autoTask = null;
          this.autoError = this.diagnostic(error);
        },
      );
    }, 300);
  }

  /** Cancels active Chat work and prevents another event from being claimed. */
  private async stopAuto(): Promise<void> {
    ++this.autoGeneration;
    if (this.autoTimer !== null) {
      clearTimeout(this.autoTimer);
      this.autoTimer = null;
    }
    if (this.activeChat !== null) await Tools.Chat.cancel(this.activeChat);
    if (this.autoTask !== null) await this.autoTask;
    this.autoTask = null;
  }

  /** Converts one inbound message into an isolated native chat and delivers all AI segments. */
  private async autoTick(generation: number): Promise<void> {
    if (
      generation !== this.autoGeneration ||
      this.state !== "ready" ||
      this.config.delivery !== null
    )
      return;
    const event = this.config.queue.shift();
    if (event === undefined || generation !== this.autoGeneration) return;
    const config = this.config.autoReply;
    this.autoError = "";
    const binding = `${this.api.requireAccount().botId}:${event.userId}`;
    this.config.delivery = {
      event,
      state: "processing",
      delivered: 0,
      error: "",
    };
    let timedOut = false;
    try {
      await PluginConfig.flush(this.config);
      let chatId = this.config.bindings[binding];
      if (chatId === undefined) {
        const input =
          config.characterSelection === ""
            ? null
            : {
                "com.operit.character_cards": {
                  version: 1,
                  selection: config.characterSelection,
                },
              };
        chatId = (
          await Tools.Chat.createNew({ setAsCurrentChat: false, input })
        ).chatId;
        await Tools.Chat.updateTitle(chatId, `微信 ${event.userId}`);
        if (config.chatGroup !== "")
          await Tools.Chat.updateGroup([chatId], config.chatGroup);
        this.config.bindings = { ...this.config.bindings, [binding]: chatId };
        await PluginConfig.flush(this.config);
      }
      if (generation !== this.autoGeneration)
        throw new Error("微信自动回复已停止。");
      this.activeChat = chatId;
      const attachments = [];
      for (const item of event.items) {
        if (mediaReference(item) === null) continue;
        attachments.push(await downloadMedia(this.api, item, event.messageId));
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
            text: `${this.messageText(event)}\n\n[微信用户=${event.userId}; message=${event.messageId}]\n${config.assistantInstruction}`,
            attachments,
            replyToMessageTimestamp: null,
          },
          turn: { kind: "execute" as const },
          notifyReply: false,
        };
        for await (const output of waifuStream(input, config.waifuProfile)) {
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
              this.config.delivery.delivered++;
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
            this.config.delivery.delivered++;
            await PluginConfig.flush(this.config);
          }
        }
        if (timedOut) throw new Error("微信自动回复已超时。");
      } finally {
        clearTimeout(timer);
      }
      if (this.config.delivery.delivered === 0)
        throw new Error("Waifu 没有产生可发送的回复。");
      this.repliedCount++;
      this.config.delivery = null;
      await PluginConfig.flush(this.config);
    } catch (error) {
      const message = timedOut
        ? "微信自动回复已超时。"
        : generation !== this.autoGeneration
          ? "微信自动回复已停止，请核对已发送内容后处理。"
          : this.diagnostic(error);
      this.autoError = message;
      if (this.config.delivery !== null) {
        this.config.delivery.state = "failed";
        this.config.delivery.error = message;
      }
      await PluginConfig.flush(this.config);
    } finally {
      this.activeChat = null;
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
    if (params.consume !== false)
      for (const event of selected)
        this.config.queue.splice(this.config.queue.indexOf(event), 1);
    if (params.consume !== false) await PluginConfig.flush(this.config);
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
    if (next.enabled) await waifuSettings(next.waifuProfile);
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
      case "auto_once":
        if (this.state !== "ready") throw new Error("请先启动微信监听。");
        if (this.autoTask !== null || this.autoTimer !== null)
          throw new Error("自动回复已在运行，请先关闭自动回复。");
        this.autoTask = this.autoTick(this.autoGeneration);
        try {
          await this.autoTask;
        } finally {
          this.autoTask = null;
        }
        if (this.autoError !== "") throw new Error(this.autoError);
        break;
      case "delivery_resolve": {
        if (
          this.config.delivery === null ||
          this.config.delivery.state !== "failed"
        )
          throw new Error("没有待处理的失败回复");
        if (request.resolution !== "retry" && request.resolution !== "discard")
          throw new Error("resolution must be retry or discard");
        if (request.resolution === "retry") {
          if (this.config.queue.length >= MAX_EVENTS)
            throw new Error("消息队列已满，请先消费消息再重试。");
          this.config.queue.unshift(this.config.delivery.event);
        }
        this.config.delivery = null;
        this.autoError = "";
        await PluginConfig.flush(this.config);
        if (this.state === "ready" && this.config.autoReply.enabled)
          this.startAuto();
        break;
      }
      case "reset_conversation":
        if (this.activeChat !== null)
          throw new Error("请先停止正在进行的自动回复。");
        delete this.config.bindings[
          `${this.api.requireAccount().botId}:${nonblank(request.user_id, "user_id")}`
        ];
        await PluginConfig.flush(this.config);
        break;
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
        quotes.push(
          previous === undefined
            ? `[引用消息 ${id}，本地未保存内容]`
            : previous.text,
        );
      }
      if (ref.title !== undefined)
        quotes.push(text(ref.title, "ref_msg.title"));
    }
    return [...quotes.map((value) => `[引用]\n${value}`), event.text].join(
      "\n\n",
    );
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
