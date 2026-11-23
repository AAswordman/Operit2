import { ChannelStore, conversationKey } from "./channel-store.js";
import { ChannelInbox, messageKey } from "./channel-inbox.js";
import type { ChannelRequest, ChannelResponses, ChannelMessage, ChannelConversation, ConversationHistory, ReplyFields } from "./channel-api.js";
import { getSettings as waifuSettings, stream as waifuStream } from "./dependencies/waifu/api.js";
import { chatText, ReplyTextStream } from "./messages.js";
import { attachments, downloadConsoleAttachment } from "./attachments.js";
import { QQGateway } from "./gateway.js";
import { QQOpenApi } from "./openapi.js";
import { CHANNEL, MAX_EVENTS, defaultConfig, errorText, integer, nonblank, text, type AutoReplyParams, type Config, type ConfigureParams, type InboundEvent, type ReceiveParams, type Request, type Response, type ReplyRole, type Status } from "./model.js";

const MAX_DELIVERIES = 200;
type Delivery = NonNullable<Config["deliveries"]>[number];
interface ReplyTask {
  superseded: boolean;
  stopDelivery?: () => void;
  done?: Promise<void>;
  cancellation?: Promise<unknown>;
}

/** Owns configuration, Gateway events and AI work in the package main runtime. */
class QQBotService {
  private readonly sessions = new ChannelStore();
  private readonly opening = new Map<string, Promise<ChannelConversation>>();
  private readonly channelInbox = new ChannelInbox();
  private api: QQOpenApi;
  private gateway: QQGateway;
  private readonly events: InboundEvent[] = [];
  private readonly seen = new Set<string>();
  private autoTimer: number | null = null;
  private readonly autoTasks = new Map<string, ReplyTask>();
  private autoGeneration = 0;
  private readonly activeChats = new Map<string, string>();
  private readonly activeContexts = new Map<string, {chatId: string; accountId: string; conversationId: string; authorId: string; scene: "c2c" | "group"}>();
  private autoRunning = false;
  private autoError = "";
  private repliedCount = 0;

  /** Creates one native network service around a durable plugin configuration. */
  constructor(private readonly config: Config) {
    config.autoReply = {...config.autoReply,groupContext:config.autoReply.groupContext??"shared",groupReplyMode:config.autoReply.groupReplyMode??"mentioned",listenGroupMessages:config.autoReply.listenGroupMessages??true};
    config.memberBindings ??= {};
    config.deliveries ??= [];
    config.bindingsAccountId ??= config.appId;
    config.deliveries=config.deliveries.map(delivery=>delivery.state==="processing"?{...delivery,state:"failed" as const,error:"上次回复中断，请确认已发送内容后重试或跳过。"}:delivery);
    this.coalescePending();
    this.api = this.createApi();
    this.gateway = this.createGateway();
  }

  /** Binds all HTTP calls to the ordinary cross-platform networking SDK. */
  private createApi(config: Config = this.config): QQOpenApi {
    return new QQOpenApi({ ...config }, { request: options => Tools.Net.http(options) }, this.sessions.recordSend("qq"));
  }

  /** Binds socket handles and ordered dispatches to the current main engine. */
  private createGateway(): QQGateway {
    return new QQGateway(this.api, { request: request => Tools.Net.webSocket(request) }, event => this.enqueue(event));
  }

  /** Exposes operational state without returning stored application credentials. */
  status(): Status {
    return { waifuProfile: this.config.waifuProfile, configured: this.config.appId !== "" && this.config.appSecret !== "", appId: this.config.appId, useSandbox: this.config.useSandbox, listenerEnabled: this.config.listenerEnabled,
      service: { status: this.gateway.status, sessionId: this.gateway.sessionId, lastError: this.gateway.lastError, receivedCount: this.gateway.receivedCount },
      queue: { size: this.events.length, capacity: MAX_EVENTS },
      autoReply: { config: { ...this.config.autoReply }, running: this.autoRunning, lastError: this.autoError, bindings: { ...this.config.bindings }, repliedCount: this.repliedCount, deliveries: (this.config.deliveries ?? []).filter(item=>item.state!=="pending").map(({key,state,delivered,error})=>({key,state,delivered,error})) } };
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
    if (next.appId !== this.config.appId) {
      this.events.length=0;this.seen.clear();this.config.bindings={};this.config.memberBindings={};this.config.processed=[];this.config.deliveries=[];
      this.config.bindingsAccountId=next.appId;
    }
    Object.assign(this.config, next);
    if (changed) { this.api = this.createApi(); this.gateway = this.createGateway(); }
    await PluginConfig.flush(this.config);
    if (next.listenerEnabled) await this.start(false);
  }

  private normalized(event: InboundEvent): ChannelMessage {
    const scene = event.scene;
    if (scene !== "group" && scene !== "c2c") throw Error("Unsupported QQ conversation scene");
    const accountId=nonblank(this.config.appId,"AppID"), conversationId=scene==="group"?event.groupOpenId:event.userOpenId;
    return {key:messageKey(["qq",accountId,scene,conversationId,event.messageId]),channel:"qq",accountId,
      messageId:event.messageId,conversationId,authorId:event.userOpenId,text:chatText(event),timestamp:event.timestamp,
      autoReplyEnabled:this.shouldReply(event),
      ...(event.author === undefined ? {} : {author:event.author}),
      ...(event.messageType === undefined ? {} : {messageType:event.messageType}),
      ...(event.attachments === undefined ? {} : {attachments:event.attachments}),
      ...(event.elements === undefined ? {} : {elements:event.elements}),
      ...(event.card === undefined ? {} : {card:event.card}),
      ...(event.referenceIndex === undefined ? {} : {referenceIndex:event.referenceIndex}),
      ...(event.referencedMessageIndex === undefined ? {} : {referencedMessageIndex:event.referencedMessageIndex}),
      context:JSON.parse(JSON.stringify({scene,replyHint:event.replyHint,raw:event.rawPayload??null}))};
  }
  private shouldReply(event: InboundEvent): boolean {
    const config=this.config.autoReply;
    return config.enabled && (event.scene==="c2c"?config.c2cEnabled:event.scene==="group" && config.groupEnabled &&
      (config.groupReplyMode==="all" || event.eventType==="GROUP_AT_MESSAGE_CREATE"));
  }
  /** Stores readable group history separately from model turns and maintenance deliveries. */
  private async enqueue(event: InboundEvent): Promise<void> {
    const key=event.messageId===""?`${event.eventType}:${event.eventId}:${event.seq}`:JSON.stringify([this.config.appId,event.scene,event.scene==="group"?event.groupOpenId:event.userOpenId,event.messageId]);
    if(this.seen.has(key))return;
    const valid=event.messageId!=="" && (event.scene==="c2c" || event.scene==="group");
    let channelOnly=false;
    if(valid){
      const message=this.normalized(event), binding=`${event.scene}:${message.conversationId}`;
      if(event.scene!=="group" || this.config.autoReply.listenGroupMessages || this.shouldReply(event))
        await this.sessions.observe(message,event.scene as "c2c"|"group",`QQ ${event.scene==="group"?"群聊":"私聊"} ${message.conversationId}`,this.config.bindings[binding]);
      const captured=this.config.channelCapture && (!Array.isArray(this.config.channelConversationIds) || this.config.channelConversationIds.includes(message.conversationId));
      if(captured)await this.channelInbox.record(message);
      channelOnly=captured && this.config.channelOnlyWhenNoAuto && !this.shouldReply(event);
      if(this.shouldReply(event) && !this.config.processed.includes(key) && !this.config.processed.includes(`${event.scene}:${event.messageId}`) && !this.config.deliveries!.some(item=>item.key===key)){
        this.interrupt(this.binding(event));
        this.coalescePending(event);
        const deliveries = this.config.deliveries!;
        if (deliveries.length >= MAX_DELIVERIES) {
          const oldestFailure = deliveries.findIndex(item => item.state === "failed");
          if (oldestFailure >= 0) {
            const [expired] = deliveries.splice(oldestFailure, 1);
            this.config.processed = [...this.config.processed, expired.key].slice(-512);
          }
        }
        if (deliveries.length >= MAX_DELIVERIES) {
          this.autoError = "自动回复积压已达 200 条，新消息仍保存在近期会话中。";
        } else {
          this.config.deliveries = [...deliveries, { key, event, state: "pending", delivered: 0, error: "" }];
          await PluginConfig.flush(this.config);
        }
      }
    }
    this.seen.add(key);if(this.seen.size>512)this.seen.delete(this.seen.values().next().value!);
    if(!channelOnly){if(this.events.length>=MAX_EVENTS)this.events.shift();this.events.push(event);}
  }

  private async conversations(): Promise<ChannelConversation[]> {
    // Existing Bot-created chats remain the authoritative routing after upgrade.
    for(const [binding,chatId] of Object.entries(this.config.bindings)){
      const separator=binding.indexOf(":"), scene=binding.slice(0,separator), id=binding.slice(separator+1);
      if((scene!=="c2c"&&scene!=="group")||!id)continue;
      await this.sessions.remember({key:conversationKey("qq",this.config.appId,scene,id),channel:"qq",accountId:this.config.appId,scene,
        conversationId:id,title:`QQ ${scene==="group"?"群聊":"私聊"} ${id}`,chatId,lastMessageAt:0});
    }
    return (await this.sessions.conversations("qq",this.config.appId)).map(session=>({...session,
      contextChatIds:Object.entries(this.config.memberBindings??{}).filter(([key])=>JSON.parse(key)[0]===session.key).map(([,id])=>id)}));
  }
  private openChat(key:string):Promise<ChannelConversation>{
    const existing=this.opening.get(key);if(existing)return existing;
    const task=(async()=>{
      const session=await this.sessions.get(key);
      if(session.accountId!==this.config.appId)throw Error("会话不属于当前 QQ 账号");
      if(session.chatId)return session;
      const retained=this.config.bindings[`${session.scene}:${session.conversationId}`];
      if(retained)return this.sessions.bind(key,retained);
      const config=this.config.autoReply, input=config.characterSelection===""?null:{"com.operit.character_cards":{version:1,selection:config.characterSelection}};
      const chatId=(await Tools.Chat.createNew({setAsCurrentChat:false,input})).chatId;
      // Publish the identity before presentation updates, so a failed title update cannot create duplicates.
      this.config.bindings={...this.config.bindings,[`${session.scene}:${session.conversationId}`]:chatId};
      await PluginConfig.flush(this.config);
      const bound=await this.sessions.bind(key,chatId);
      await Tools.Chat.updateTitle(chatId,session.title);
      if(config.chatGroup!=="")await Tools.Chat.updateGroup([chatId],config.chatGroup);
      return bound;
    })();
    this.opening.set(key,task);void task.finally(()=>this.opening.delete(key)).catch(()=>{});return task;
  }
  private async chatContext(chatId:string,limit?:number,before?:number):Promise<ConversationHistory>{
    const active=[...this.activeContexts.values()].find(context=>context.chatId===chatId);
    const sessions=await this.conversations();
    let session=sessions.find(item=>item.chatId===chatId);
    let authorId:string|undefined;
    if(!session){
      const member=Object.entries(this.config.memberBindings??{}).find(([,id])=>id===chatId);
      if(member){const [key,author]=JSON.parse(member[0]) as [string,string];session=sessions.find(item=>item.key===key);authorId=author;}
    }
    if(!session)throw Error("当前聊天不是此 QQ Bot 的会话");
    if(this.config.autoReply.groupContext==="member"&&session.scene==="group")authorId=authorId??active?.authorId;
    if(this.config.autoReply.groupContext==="member"&&session.scene==="group"&&!authorId)throw Error("成员隔离模式需要明确的当前发送者");
    return this.sessions.history(session.key,limit,before,authorId);
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
    if(params.group_context!==undefined){if(params.group_context!=="shared"&&params.group_context!=="member")throw Error("Invalid group context mode");next.groupContext=params.group_context;}
    if(params.group_reply_mode!==undefined){if(params.group_reply_mode!=="mentioned"&&params.group_reply_mode!=="all")throw Error("Invalid group reply mode");next.groupReplyMode=params.group_reply_mode;}
    if(params.listen_group_messages!==undefined){if(typeof params.listen_group_messages!=="boolean")throw Error("Invalid group listening setting");next.listenGroupMessages=params.listen_group_messages;}
    if (params.ai_timeout_ms !== undefined) next.aiTimeoutMs = integer(params.ai_timeout_ms, "ai_timeout_ms", 1000, 600000);
    if (params.chat_group !== undefined) next.chatGroup = text(params.chat_group, "chat_group").trim();
    if (params.character_selection !== undefined) next.characterSelection = text(params.character_selection, "character_selection").trim();
    if (params.assistant_instruction !== undefined) next.assistantInstruction = text(params.assistant_instruction, "assistant_instruction");
    if (next.enabled && params.start_now !== false && !this.config.listenerEnabled) throw new Error("Enable the QQ listener before starting automatic replies");
    const profile = params.waifu_profile === undefined ? this.config.waifuProfile : nonblank(params.waifu_profile, "waifu_profile");
    if (next.enabled && profile !== "native") await preferWaifu(profile);
    await this.stopAuto();
    this.config.waifuProfile = profile;
    this.config.autoReply = next;
    this.coalescePending();
    await PluginConfig.flush(this.config);
    if (next.enabled && params.start_now !== false) {
      await this.start(false);
    }
  }

  /** Processes unrelated conversations concurrently. */
  private startAuto(): void {
    if (this.autoRunning) return;
    this.autoRunning = true; this.autoError = "";
    this.scheduleAuto(this.autoGeneration);
  }

  private binding(event: InboundEvent): string {
    if (event.scene !== "group") return JSON.stringify([event.scene, event.userOpenId]);
    return JSON.stringify(this.config.autoReply.groupContext === "member"
      ? [event.scene, event.groupOpenId, event.userOpenId] : [event.scene, event.groupOpenId]);
  }

  /** Keeps the last waiting event per reply context, including after upgrading. */
  private coalescePending(replacement?: InboundEvent): void {
    const seen = new Set(replacement ? [this.binding(replacement)] : []);
    const dropped: string[] = [];
    const deliveries = [...this.config.deliveries!].reverse().filter(item => {
      if (item.state !== "pending") return true;
      const binding = this.binding(item.event);
      if (seen.has(binding)) { dropped.push(item.key); return false; }
      seen.add(binding);
      return true;
    }).reverse();
    if (dropped.length === 0) return;
    this.config.deliveries = deliveries;
    this.config.processed = [...this.config.processed, ...dropped].slice(-512);
  }

  private interrupt(binding: string): void {
    const task = this.autoTasks.get(binding);
    if (!task || task.superseded) return;
    task.superseded = true;
    void this.cancelReply(binding, task);
  }

  private cancelReply(binding: string, task: ReplyTask): Promise<unknown> | undefined {
    task.stopDelivery?.();
    const chatId = this.activeChats.get(binding);
    if (chatId) task.cancellation ??= Tools.Chat.cancel(chatId).catch(error => console.error("[qqbot.cancel]", error));
    return task.cancellation;
  }

  private takeEvent(): Delivery | undefined {
    const delivery = this.config.deliveries!.find(item => item.state === "pending" && this.shouldReply(item.event) &&
      !this.autoTasks.has(this.binding(item.event)));
    if (!delivery) return;
    delivery.state = "processing";
    this.config.deliveries = [...this.config.deliveries!];
    const index = this.events.indexOf(delivery.event);
    if (index >= 0) this.events.splice(index, 1);
    return delivery;
  }

  private launchAuto(generation: number, delivery: Delivery): Promise<void> {
    const binding = this.binding(delivery.event);
    const task: ReplyTask = { superseded: false };
    this.autoTasks.set(binding, task);
    const done = this.autoTick(generation, delivery, binding, task).finally(async () => {
      // Cancel by chat ID must settle before another turn reuses that chat.
      await task.cancellation;
      this.autoTasks.delete(binding);
    });
    task.done = done;
    void done.catch(error => console.error("[qqbot.auto.task]", error));
    return done;
  }

  /** Keeps a bounded number of unrelated conversations moving independently. */
  private scheduleAuto(generation: number): void {
    if (!this.autoRunning || generation !== this.autoGeneration) return;
    this.autoTimer = setTimeout(() => {
      this.autoTimer = null;
      if (this.gateway.status === "error" || this.gateway.status === "stopped") {
        this.autoRunning = false; this.autoError = `QQ listener is ${this.gateway.status}: ${this.gateway.lastError}`; return;
      }
      if (this.gateway.status === "ready" && this.autoTasks.size < 3) {
        const item = this.takeEvent();
        if (item) void this.launchAuto(generation, item);
      }
      this.scheduleAuto(generation);
    }, 300);
  }

  /** Cancels every owned generation before allowing stop or reconfiguration to finish. */
  private async stopAuto(): Promise<void> {
    this.autoRunning = false; ++this.autoGeneration;
    if (this.autoTimer !== null) { clearTimeout(this.autoTimer); this.autoTimer = null; }
    const cancellations = await Promise.allSettled([...this.autoTasks].map(([binding, task]) => this.cancelReply(binding, task)));
    const tasks = await Promise.allSettled([...this.autoTasks.values()].map(task => task.done));
    const failure = [...cancellations, ...tasks].find(result => result.status === "rejected");
    if (failure?.status === "rejected") throw failure.reason;
  }

  /** Consumes one reserved inbound message and records its delivery or model error. */
  private async autoTick(generation: number, delivery: Delivery, taskBinding: string, task: ReplyTask): Promise<void> {
    const event = delivery.event;
    const current = () => generation === this.autoGeneration && !task.superseded;
    try {
      if (!current()) return;
      if (this.gateway.status === "error" || this.gateway.status === "stopped") {
        this.autoRunning = false;
        throw new Error(`QQ listener is ${this.gateway.status}: ${this.gateway.lastError}`);
      }
      const config=this.config.autoReply, message=this.normalized(event), scene=event.scene;
      if(scene!=="c2c"&&scene!=="group")throw Error("Automatic replies require a QQ conversation");
      const target=message.conversationId;
      await PluginConfig.flush(this.config);
      if (!current()) return;
      await this.conversations();
      if (!current()) return;
      const session=await this.openChat(conversationKey("qq",this.config.appId,scene,target));
      let chatId=session.chatId!;
      if(scene==="group"&&config.groupContext==="member"){
        const key=JSON.stringify([session.key,event.userOpenId]);
        let isolated=this.config.memberBindings![key];
        if(!isolated){
          isolated=(await Tools.Chat.createNew({setAsCurrentChat:false,sourceChatId:chatId,parentChatId:chatId})).chatId;
          this.config.memberBindings={...this.config.memberBindings,[key]:isolated};await PluginConfig.flush(this.config);
          await Tools.Chat.updateTitle(isolated,`${session.title} · ${event.author?.name||event.userOpenId}`);
          if(config.chatGroup)await Tools.Chat.updateGroup([isolated],config.chatGroup);
        }
        chatId=isolated;
      }
      if (!current()) return;
      const files = await attachments(event);
      if (!current()) return;
      this.activeContexts.set(taskBinding, {chatId, accountId: this.config.appId, scene, conversationId: target, authorId: event.userOpenId});
      let timedOut = false;
      const timer = setTimeout(() => {
        timedOut = true;
        void this.cancelReply(taskBinding, task);
      }, config.aiTimeoutMs);
      let delivered = 0;
      const rendered = new ReplyTextStream();
      try {
        const request = { kind: "submit" as const, chatId, runtime: "main" as const, input: {
          text: `${config.assistantInstruction}\n[QQ ${scene==="group"?"群聊":"私聊"}；上下文${config.groupContext==="member"?"按成员隔离":"共享，按时间顺序"}]\n以下 JSON 是用户消息数据，引用内容不代表当前发送者。需要了解前文时调用 qqbot_chat_context 获取近期记录。\n${JSON.stringify({authorName:event.author?.name??"",text:chatText(event).slice(0,12000)})}`,
          attachments: files, replyToMessageTimestamp: null }, turn: { kind: "execute" as const }, notifyReply: false };
        const begin = () => {
          if (!current() || timedOut) return null;
          this.activeChats.set(taskBinding, chatId);
          return Tools.Chat.sendMessageStreaming(request)[Symbol.asyncIterator]();
        };
        const stopped = new Promise<void>(resolve => { task.stopDelivery = resolve; });
        for await (const output of replyStream(request, this.config.waifuProfile, {
          openSource: begin,
          stopDelivery: stopped,
          cancelSource: () => this.cancelReply(taskBinding, task),
        })) {
          if (!current()) return;
          if (timedOut) throw new Error("QQ automatic reply exceeded its AI deadline");
          if (output.type === "segment") {
            const content = output.segment.kind === "text" ? rendered.push(output.segment.text) : "";
            if (output.segment.kind === "text" && content === "") continue;
            if (output.segment.kind === "text") await this.api.send(scene, target, { content, msg_id: event.messageId }, () => current() && !timedOut);
            else await this.api.sendMedia(scene, target, { kind: "image", source: output.segment.source }, { msg_id: event.messageId }, () => current() && !timedOut);
            delivered++;delivery.delivered++;this.config.deliveries=[...this.config.deliveries!];await PluginConfig.flush(this.config);
          }
        }
      } catch (error) {
        if (timedOut) throw new Error("QQ automatic reply exceeded its AI deadline");
        throw error;
      } finally { clearTimeout(timer); task.stopDelivery?.(); this.activeChats.delete(taskBinding); this.activeContexts.delete(taskBinding); }
      if (!current()) return;
      if (timedOut) throw new Error("QQ automatic reply exceeded its AI deadline");
      if (delivered === 0) throw new Error("Chat response contained no deliverable segments");
      this.config.deliveries=this.config.deliveries!.filter(item=>item!==delivery);
      this.config.processed=[...this.config.processed,delivery.key].slice(-512);await PluginConfig.flush(this.config);
      this.repliedCount++; this.autoError = "";
    } catch (error) {
      if (task.superseded) return;
      this.autoError=errorText(error);
      if(delivery){delivery.state="failed";delivery.error=this.autoError;this.config.deliveries=[...this.config.deliveries!];await PluginConfig.flush(this.config);}
      console.error("[qqbot.auto]",error);
    } finally {
      if (task.superseded && this.config.deliveries!.includes(delivery)) {
        this.config.deliveries = this.config.deliveries!.filter(item => item !== delivery);
        this.config.processed = [...this.config.processed, delivery.key].slice(-512);
        await PluginConfig.flush(this.config);
      } else if(delivery.state==="processing" && this.config.deliveries!.includes(delivery)){delivery.state="failed";delivery.error="回复被停止；请确认发送情况后重试或跳过。";this.config.deliveries=[...this.config.deliveries!];await PluginConfig.flush(this.config);}
    }
  }

  private async sendChannelMessage(scene: ChannelConversation["scene"], target: string, request: ReplyFields,
    reply?: { messageId: string; referenceIndex?: string }): Promise<ChannelResponses["send"]> {
    const api = this.api;
    const params = {
      content: request.text, markdown: request.markdown, keyboard: request.keyboard, card: request.card,
      msg_id: reply?.messageId,
      message_reference: request.message_reference ?? (reply?.referenceIndex ? { message_id: reply.referenceIndex } : undefined),
    };
    const accountIsCurrent = () => api === this.api;
    return { response: request.media === undefined ? await api.send(scene, target, params, accountIsCurrent)
      : await api.sendMedia(scene, target, request.media, params, accountIsCurrent) };
  }

  /** Independent public channel contract, separate from the automatic-reply queue. */
  async channel(request: ChannelRequest): Promise<ChannelResponses[ChannelRequest["action"]]> {
    if (request === null || typeof request !== "object") throw new Error("Invalid channel request");
    switch (request.action) {
      case "conversations": return {conversations:await this.conversations()};
      case "open_chat": return this.openChat(request.conversationKey);
      case "history": {
        const session=await this.sessions.get(request.conversationKey);
        if(session.accountId!==this.config.appId)throw Error("会话不属于当前 QQ 账号");
        return this.sessions.history(session.key,request.limit,request.before,undefined,true);
      }
      case "subscribe": return this.sessions.subscribe(request.consumerId,request.routes,"qq",this.config.appId);
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
      case "status": return { channel: "qq", capture: this.config.channelCapture === true, conversationIds: this.config.channelConversationIds ?? null, channelOnlyWhenNoAuto: this.config.channelOnlyWhenNoAuto === true, service: this.gateway.status,
        configured: !!(this.config.appId && this.config.appSecret), listenerEnabled: this.config.listenerEnabled,
        autoReplyEnabled: this.config.autoReply.enabled, error: this.gateway.lastError || this.autoError,
        replyActivity: {
          received: this.gateway.receivedCount,
          replied: this.repliedCount,
          failed: this.config.deliveries!.filter(item => item.state === "failed").length,
          running: this.autoRunning && this.gateway.status === "ready",
          directEnabled: this.config.autoReply.c2cEnabled,
          groupMode: this.config.autoReply.groupEnabled ? this.config.autoReply.groupReplyMode : "off",
        } };
      case "manage": {
        const command = request.command;
        if (command === "connect") await this.configure({ app_id: nonblank(request.appId, "App ID"), app_secret: nonblank(request.appSecret, "App Secret"), listener_enabled: true, test_connection: true });
        else {
          if (!["start", "stop", "test"].includes(command)) throw Error("Unknown QQ management command");
          if (command === "start" && request.autoReply !== undefined) await this.configureAuto({ enabled: request.autoReply, start_now: false });
          await this.request({ action: command });
        }
        return this.channel({ action: "status" });
      }
      case "active_context": {
        const chatId = nonblank(request.chatId, "chatId");
        const matches = [...this.activeContexts.values()].filter(value => value.chatId === chatId);
        return matches.length === 1 ? {channel: "qq", ...matches[0]} : null;
      }
      case "peek": return { messages: (request.consumerId===undefined ? await this.channelInbox.peek(request.limit, request.conversationId) : await this.sessions.peek(request.consumerId,request.limit,request.conversationKey)).filter(message=>message.accountId===this.config.appId) };
      case "ack": return { acknowledged: request.consumerId===undefined ? await this.channelInbox.ack(request.key) : await this.sessions.ack(request.consumerId,request.key) };
      case "attachment": {
        const api = this.api;
        const session = await this.sessions.get(request.conversationKey);
        if (session.accountId !== this.config.appId || api !== this.api) throw Error("QQ 会话不属于当前账户");
        const bytes = await downloadConsoleAttachment(await this.sessions.attachment(session.key, request.messageId, request.index));
        if (api !== this.api) throw Error("QQ 账户已切换");
        return bytes;
      }
      case "reply": {
        const api = this.api;
        const message = await this.channelInbox.get(request.key) ?? await this.sessions.replyRoute(request.key);
        if (message === null) throw new Error("Pending message not found");
        if (message.accountId !== this.config.appId || api !== this.api) throw new Error("Message belongs to a different QQ account");
        const scene = message.context.scene;
        if (scene !== "c2c" && scene !== "group") throw new Error("Unsupported QQ reply scene");
        return this.sendChannelMessage(scene, message.conversationId, request, {
          messageId: message.messageId, referenceIndex: request.quote === true ? message.referenceIndex : undefined,
        });
      }
      case "send": {
        const api = this.api;
        const session = await this.sessions.get(request.conversationKey);
        if (session.accountId !== this.config.appId || api !== this.api) throw Error("QQ 会话不属于当前账户");
        return this.sendChannelMessage(session.scene, session.conversationId, request);
      }
      default: throw new Error("Unknown channel request");
    }
  }

  /** Handles every UI and tool operation in the single package owner. */
  async request(request: Request): Promise<Response> {
    switch (request.action) {
      case "configure": await this.configure(request.params); break;
      case "status": break;
      case "chat_context": return {success:true,context:await this.chatContext(request.chatId,request.limit,request.before)};
      case "retry_delivery": {
        const delivery=this.config.deliveries!.find(item=>item.key===request.key);
        if(!delivery||delivery.state!=="failed")throw Error("只能处理失败的回复");
        if(request.skip){this.config.deliveries=this.config.deliveries!.filter(item=>item!==delivery);this.config.processed=[...this.config.processed,delivery.key].slice(-512);}
        else {delivery.state="pending";delivery.error="";this.config.deliveries=[...this.config.deliveries!];}
        await PluginConfig.flush(this.config);break;
      }
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
      case "upload_media": return { success: true, response: await this.api.uploadMedia(request.scene, request.target, request.media) };
      case "send_media": return { success: true, response: await this.api.sendMedia(request.scene, request.target, request.media, request.params) };
      case "upload_image": return { success: true, response: await this.api.uploadImage(request.scene, request.target, request.source) };
      case "auto_configure": await this.configureAuto(request.params); break;
      case "auto_start": await this.configureAuto({ enabled: true, start_now: true }); break;
      case "auto_stop": await this.configureAuto({ enabled: false }); break;
      case "auto_once":
        if (this.autoTasks.size !== 0) throw new Error("An automatic reply is already in progress");
        if (this.gateway.status !== "ready") throw new Error("The QQ listener must be ready");
        this.autoError = "";
        const item = this.takeEvent();
        if (item) await this.launchAuto(this.autoGeneration, item);
        if (this.autoError !== "") throw new Error(this.autoError);
        break;
      default: throw new Error("Unknown QQ Bot operation");
    }
    return { success: true, status: this.status() };
  }
}

async function preferWaifu(profile: string): Promise<boolean> {
  if (profile === "native") return false;
  try { await waifuSettings(profile); return true; }
  catch (error) {
    if (/(?:ToolPkg (?:container not found|(?:package|extension owner|public API owner) is disabled)|Dependency version mismatch): com\.operit\.waifu\b/i.test(errorText(error))) return false;
    throw error;
  }
}

async function* replyStream(request: Parameters<typeof waifuStream>[0], profile: string, controls: Required<NonNullable<Parameters<typeof waifuStream>[2]>>): ReturnType<typeof waifuStream> {
  // Select the backend before submitting the turn; never replay a failed or partial Waifu turn.
  if (await preferWaifu(profile)) { yield* waifuStream(request, profile, controls); return; }
  const source = controls.openSource();
  if (!source) return;
  for await (const event of { [Symbol.asyncIterator]: () => source }) {
    if (event.type !== "completed") continue;
    const { outcome, aiResponse } = event.result;
    if (outcome.type !== "committed" || outcome.status !== "completed") throw Error(`Chat did not complete: ${JSON.stringify(outcome)}`);
    if (aiResponse) yield { type: "segment", index: 0, segment: { kind: "text", text: aiResponse } };
    return;
  }
  throw Error("Chat stream ended without a completed receipt");
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

export function channel(request: ChannelRequest): Promise<ChannelResponses[ChannelRequest["action"]]> { return command(owner => owner.channel(request)); }

/** Registers the authoritative main-runtime IPC channel. */
export function registerService(): void { ToolPkg.ipc.on<Request, Response>(CHANNEL, receive); }

/** Restores user-enabled networking when the application starts or returns to the foreground. */
export function restore(): Promise<void> { return command(owner => owner.restore()); }

/** Releases model and networking resources during application termination. */
export function terminate(): Promise<void> { return command(owner => owner.stopRuntime()); }
