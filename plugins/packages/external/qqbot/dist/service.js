"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.receive = receive;
exports.channel = channel;
exports.registerService = registerService;
exports.restore = restore;
exports.terminate = terminate;
const channel_store_js_1 = require("./channel-store.js");
const channel_inbox_js_1 = require("./channel-inbox.js");
const api_js_1 = require("./dependencies/waifu/api.js");
const messages_js_1 = require("./messages.js");
const attachments_js_1 = require("./attachments.js");
const gateway_js_1 = require("./gateway.js");
const openapi_js_1 = require("./openapi.js");
const model_js_1 = require("./model.js");
/** Owns configuration, Gateway events and AI work in the package main runtime. */
class QQBotService {
    /** Creates one native network service around a durable plugin configuration. */
    constructor(config) {
        this.config = config;
        this.sessions = new channel_store_js_1.ChannelStore();
        this.opening = new Map();
        this.channelInbox = new channel_inbox_js_1.ChannelInbox();
        this.events = [];
        this.seen = new Set();
        this.autoTimer = null;
        this.autoTasks = new Map();
        this.autoGeneration = 0;
        this.activeChats = new Map();
        this.activeContexts = new Map();
        this.autoRunning = false;
        this.autoError = "";
        this.repliedCount = 0;
        config.autoReply = { ...config.autoReply, groupContext: config.autoReply.groupContext ?? "shared", groupReplyMode: config.autoReply.groupReplyMode ?? "mentioned", listenGroupMessages: config.autoReply.listenGroupMessages ?? true };
        config.memberBindings ?? (config.memberBindings = {});
        config.deliveries ?? (config.deliveries = []);
        config.bindingsAccountId ?? (config.bindingsAccountId = config.appId);
        config.deliveries = config.deliveries.map(delivery => delivery.state === "processing" ? { ...delivery, state: "failed", error: "上次回复中断，请确认已发送内容后重试或跳过。" } : delivery);
        this.api = this.createApi();
        this.gateway = this.createGateway();
    }
    /** Binds all HTTP calls to the ordinary cross-platform networking SDK. */
    createApi(config = this.config) { return new openapi_js_1.QQOpenApi(config, { request: options => Tools.Net.http(options) }); }
    /** Binds socket handles and ordered dispatches to the current main engine. */
    createGateway() {
        return new gateway_js_1.QQGateway(this.api, { request: request => Tools.Net.webSocket(request) }, event => this.enqueue(event));
    }
    /** Exposes operational state without returning stored application credentials. */
    status() {
        return { waifuProfile: this.config.waifuProfile, configured: this.config.appId !== "" && this.config.appSecret !== "", appId: this.config.appId, useSandbox: this.config.useSandbox, listenerEnabled: this.config.listenerEnabled,
            service: { status: this.gateway.status, sessionId: this.gateway.sessionId, lastError: this.gateway.lastError, receivedCount: this.gateway.receivedCount },
            queue: { size: this.events.length, capacity: model_js_1.MAX_EVENTS },
            autoReply: { config: { ...this.config.autoReply }, running: this.autoRunning, lastError: this.autoError, bindings: { ...this.config.bindings }, repliedCount: this.repliedCount, deliveries: (this.config.deliveries ?? []).filter(item => item.state !== "pending").map(({ key, state, delivered, error }) => ({ key, state, delivered, error })) } };
    }
    /** Restores explicitly enabled services after application creation or foreground entry. */
    async restore() {
        if (!this.config.listenerEnabled)
            return;
        await this.start(false);
    }
    /** Starts the listener and awaits its authenticated READY event. */
    async start(restart) {
        (0, model_js_1.nonblank)(this.config.appId, "AppID");
        (0, model_js_1.nonblank)(this.config.appSecret, "AppSecret");
        if (restart)
            await this.stopRuntime();
        if (this.gateway.status === "ready") {
            if (this.config.autoReply.enabled && !this.autoRunning)
                this.startAuto();
            return;
        }
        if (this.gateway.status !== "connecting") {
            this.api = this.createApi();
            this.gateway = this.createGateway();
            await this.gateway.start();
        }
        await this.gateway.waitReady();
        if (this.config.autoReply.enabled)
            this.startAuto();
    }
    /** Stops pending AI work and the socket before releasing the listener runtime. */
    async stopRuntime() { await this.stopAuto(); await this.gateway.stop(); }
    /** Validates a configuration patch before durably committing it and restarting its affected runtime. */
    async configure(params) {
        const next = { appId: this.config.appId, appSecret: this.config.appSecret, useSandbox: this.config.useSandbox, listenerEnabled: this.config.listenerEnabled };
        if (params.app_id !== undefined)
            next.appId = (0, model_js_1.text)(params.app_id, "app_id").trim();
        if (params.app_secret !== undefined)
            next.appSecret = (0, model_js_1.text)(params.app_secret, "app_secret").trim();
        if (params.use_sandbox !== undefined) {
            if (typeof params.use_sandbox !== "boolean")
                throw new Error("use_sandbox must be boolean");
            next.useSandbox = params.use_sandbox;
        }
        if (params.listener_enabled !== undefined) {
            if (typeof params.listener_enabled !== "boolean")
                throw new Error("listener_enabled must be boolean");
            next.listenerEnabled = params.listener_enabled;
        }
        if (next.listenerEnabled) {
            (0, model_js_1.nonblank)(next.appId, "AppID");
            (0, model_js_1.nonblank)(next.appSecret, "AppSecret");
        }
        if (params.test_connection === true)
            await this.createApi({ ...this.config, ...next }).test();
        const changed = next.appId !== this.config.appId || next.appSecret !== this.config.appSecret || next.useSandbox !== this.config.useSandbox;
        if (changed || !next.listenerEnabled || params.restart_service === true)
            await this.stopRuntime();
        if (next.appId !== this.config.appId) {
            this.events.length = 0;
            this.seen.clear();
            this.config.bindings = {};
            this.config.memberBindings = {};
            this.config.processed = [];
            this.config.deliveries = [];
            this.config.bindingsAccountId = next.appId;
        }
        Object.assign(this.config, next);
        if (changed) {
            this.api = this.createApi();
            this.gateway = this.createGateway();
        }
        await PluginConfig.flush(this.config);
        if (next.listenerEnabled)
            await this.start(false);
    }
    normalized(event) {
        const scene = event.scene;
        if (scene !== "group" && scene !== "c2c")
            throw Error("Unsupported QQ conversation scene");
        const accountId = (0, model_js_1.nonblank)(this.config.appId, "AppID"), conversationId = scene === "group" ? event.groupOpenId : event.userOpenId;
        return { key: (0, channel_inbox_js_1.messageKey)(["qq", accountId, scene, conversationId, event.messageId]), channel: "qq", accountId,
            messageId: event.messageId, conversationId, authorId: event.userOpenId, text: (0, messages_js_1.chatText)(event), timestamp: event.timestamp,
            autoReplyEnabled: this.shouldReply(event), author: event.author, messageType: event.messageType, attachments: event.attachments, elements: event.elements,
            card: event.card, referenceIndex: event.referenceIndex, referencedMessageIndex: event.referencedMessageIndex,
            context: JSON.parse(JSON.stringify({ scene, replyHint: event.replyHint, raw: event.rawPayload ?? null })) };
    }
    shouldReply(event) {
        const config = this.config.autoReply;
        return config.enabled && (event.scene === "c2c" ? config.c2cEnabled : event.scene === "group" && config.groupEnabled &&
            (config.groupReplyMode === "all" || event.eventType === "GROUP_AT_MESSAGE_CREATE"));
    }
    /** Stores readable group history separately from model turns and maintenance deliveries. */
    async enqueue(event) {
        const key = event.messageId === "" ? `${event.eventType}:${event.eventId}:${event.seq}` : JSON.stringify([this.config.appId, event.scene, event.scene === "group" ? event.groupOpenId : event.userOpenId, event.messageId]);
        if (this.seen.has(key))
            return;
        const valid = event.messageId !== "" && (event.scene === "c2c" || event.scene === "group");
        let channelOnly = false;
        if (valid) {
            const message = this.normalized(event), binding = `${event.scene}:${message.conversationId}`;
            if (event.scene !== "group" || this.config.autoReply.listenGroupMessages || this.shouldReply(event))
                await this.sessions.observe(message, event.scene, `QQ ${event.scene === "group" ? "群聊" : "私聊"} ${message.conversationId}`, this.config.bindings[binding]);
            const captured = this.config.channelCapture && (!Array.isArray(this.config.channelConversationIds) || this.config.channelConversationIds.includes(message.conversationId));
            if (captured)
                await this.channelInbox.record(message);
            channelOnly = captured && this.config.channelOnlyWhenNoAuto && !this.shouldReply(event);
            if (this.shouldReply(event) && !this.config.processed.includes(key) && !this.config.processed.includes(`${event.scene}:${event.messageId}`) && !this.config.deliveries.some(item => item.key === key)) {
                if (this.config.deliveries.length >= 200)
                    this.autoError = "自动回复积压已达 200 条；新消息仍保存在近期上下文和已订阅反馈入口中，请处理失败回复。";
                else {
                    this.config.deliveries = [...this.config.deliveries, { key, event, state: "pending", delivered: 0, error: "" }];
                    await PluginConfig.flush(this.config);
                }
            }
        }
        this.seen.add(key);
        if (this.seen.size > 512)
            this.seen.delete(this.seen.values().next().value);
        if (!channelOnly) {
            if (this.events.length >= model_js_1.MAX_EVENTS)
                this.events.shift();
            this.events.push(event);
        }
    }
    async conversations() {
        // Existing Bot-created chats remain the authoritative routing after upgrade.
        for (const [binding, chatId] of Object.entries(this.config.bindings)) {
            const separator = binding.indexOf(":"), scene = binding.slice(0, separator), id = binding.slice(separator + 1);
            if ((scene !== "c2c" && scene !== "group") || !id)
                continue;
            await this.sessions.remember({ key: (0, channel_store_js_1.conversationKey)("qq", this.config.appId, scene, id), channel: "qq", accountId: this.config.appId, scene,
                conversationId: id, title: `QQ ${scene === "group" ? "群聊" : "私聊"} ${id}`, chatId, lastMessageAt: 0 });
        }
        return (await this.sessions.conversations("qq", this.config.appId)).map(session => ({ ...session,
            contextChatIds: Object.entries(this.config.memberBindings ?? {}).filter(([key]) => JSON.parse(key)[0] === session.key).map(([, id]) => id) }));
    }
    openChat(key) {
        const existing = this.opening.get(key);
        if (existing)
            return existing;
        const task = (async () => {
            const session = await this.sessions.get(key);
            if (session.accountId !== this.config.appId)
                throw Error("会话不属于当前 QQ 账号");
            if (session.chatId)
                return session;
            const retained = this.config.bindings[`${session.scene}:${session.conversationId}`];
            if (retained)
                return this.sessions.bind(key, retained);
            const config = this.config.autoReply, input = config.characterSelection === "" ? null : { "com.operit.character_cards": { version: 1, selection: config.characterSelection } };
            const chatId = (await Tools.Chat.createNew({ setAsCurrentChat: false, input })).chatId;
            // Publish the identity before presentation updates, so a failed title update cannot create duplicates.
            this.config.bindings = { ...this.config.bindings, [`${session.scene}:${session.conversationId}`]: chatId };
            await PluginConfig.flush(this.config);
            const bound = await this.sessions.bind(key, chatId);
            await Tools.Chat.updateTitle(chatId, session.title);
            if (config.chatGroup !== "")
                await Tools.Chat.updateGroup([chatId], config.chatGroup);
            return bound;
        })();
        this.opening.set(key, task);
        void task.finally(() => this.opening.delete(key)).catch(() => { });
        return task;
    }
    async chatContext(chatId, limit, before) {
        const active = [...this.activeContexts.values()].find(context => context.chatId === chatId);
        const sessions = await this.conversations();
        let session = sessions.find(item => item.chatId === chatId);
        let authorId;
        if (!session) {
            const member = Object.entries(this.config.memberBindings ?? {}).find(([, id]) => id === chatId);
            if (member) {
                const [key, author] = JSON.parse(member[0]);
                session = sessions.find(item => item.key === key);
                authorId = author;
            }
        }
        if (!session)
            throw Error("当前聊天不是此 QQ Bot 的会话");
        if (this.config.autoReply.groupContext === "member" && session.scene === "group")
            authorId = authorId ?? active?.authorId;
        if (this.config.autoReply.groupContext === "member" && session.scene === "group" && !authorId)
            throw Error("成员隔离模式需要明确的当前发送者");
        return this.sessions.history(session.key, limit, before, authorId);
    }
    /** Returns and optionally consumes only events matching explicit filters. */
    receive(params) {
        const limit = params.limit === undefined ? 20 : (0, model_js_1.integer)(params.limit, "limit", 1, 100);
        if (params.scene !== undefined && params.scene !== "c2c" && params.scene !== "group" && params.scene !== "unknown")
            throw new Error("Invalid scene");
        const selected = this.events.filter(event => (params.scene === undefined || params.scene === event.scene) && (params.event_type === undefined || params.event_type === event.eventType)).slice(0, limit);
        if (params.consume !== false)
            for (const event of selected)
                this.events.splice(this.events.indexOf(event), 1);
        const events = selected.map(event => {
            if (params.include_raw === true)
                return { ...event };
            const { rawPayload: _raw, ...summary } = event;
            return summary;
        });
        return { success: true, returnedCount: events.length, remainingCount: this.events.length, events };
    }
    /** Commits automatic reply settings using the v2 conversation creation contract. */
    async configureAuto(params) {
        const next = { ...this.config.autoReply };
        for (const [field, value] of [["enabled", params.enabled], ["c2cEnabled", params.c2c_enabled], ["groupEnabled", params.group_enabled]]) {
            if (value !== undefined) {
                if (typeof value !== "boolean")
                    throw new Error(`${field} must be boolean`);
                next[field] = value;
            }
        }
        if (params.group_context !== undefined) {
            if (params.group_context !== "shared" && params.group_context !== "member")
                throw Error("Invalid group context mode");
            next.groupContext = params.group_context;
        }
        if (params.group_reply_mode !== undefined) {
            if (params.group_reply_mode !== "mentioned" && params.group_reply_mode !== "all")
                throw Error("Invalid group reply mode");
            next.groupReplyMode = params.group_reply_mode;
        }
        if (params.listen_group_messages !== undefined) {
            if (typeof params.listen_group_messages !== "boolean")
                throw Error("Invalid group listening setting");
            next.listenGroupMessages = params.listen_group_messages;
        }
        if (params.ai_timeout_ms !== undefined)
            next.aiTimeoutMs = (0, model_js_1.integer)(params.ai_timeout_ms, "ai_timeout_ms", 1000, 600000);
        if (params.chat_group !== undefined)
            next.chatGroup = (0, model_js_1.text)(params.chat_group, "chat_group").trim();
        if (params.character_selection !== undefined)
            next.characterSelection = (0, model_js_1.text)(params.character_selection, "character_selection").trim();
        if (params.assistant_instruction !== undefined)
            next.assistantInstruction = (0, model_js_1.text)(params.assistant_instruction, "assistant_instruction");
        if (next.enabled && params.start_now !== false && !this.config.listenerEnabled)
            throw new Error("Enable the QQ listener before starting automatic replies");
        const profile = params.waifu_profile === undefined ? this.config.waifuProfile : (0, model_js_1.nonblank)(params.waifu_profile, "waifu_profile");
        if (next.enabled)
            await (0, api_js_1.getSettings)(profile);
        await this.stopAuto();
        this.config.waifuProfile = profile;
        this.config.autoReply = next;
        await PluginConfig.flush(this.config);
        if (next.enabled && params.start_now !== false) {
            await this.start(false);
        }
    }
    /** Processes separate QQ conversations concurrently while preserving each conversation's order. */
    startAuto() {
        if (this.autoRunning)
            return;
        this.autoRunning = true;
        this.autoError = "";
        this.scheduleAuto(this.autoGeneration);
    }
    binding(event) {
        return JSON.stringify([event.scene, event.scene === "group" ? event.groupOpenId : event.userOpenId]);
    }
    /** Reserves the next eligible conversation before starting any asynchronous work. */
    takeEvent() {
        const delivery = this.config.deliveries.find((item, index) => item.state === "pending" && this.shouldReply(item.event) && !this.autoTasks.has(this.binding(item.event)) && !this.config.deliveries.slice(0, index).some(prior => this.binding(prior.event) === this.binding(item.event)));
        if (!delivery)
            return null;
        delivery.state = "processing";
        this.config.deliveries = [...this.config.deliveries];
        const index = this.events.findIndex(item => item === delivery.event || (item.messageId === delivery.event.messageId && this.binding(item) === this.binding(delivery.event)));
        if (index >= 0)
            this.events.splice(index, 1);
        return { event: delivery.event, binding: this.binding(delivery.event) };
    }
    launchAuto(generation, item) {
        const task = this.autoTick(generation, item.event, item.binding);
        this.autoTasks.set(item.binding, task);
        void task.then(() => { if (this.autoTasks.get(item.binding) === task)
            this.autoTasks.delete(item.binding); }, error => { if (this.autoTasks.get(item.binding) === task)
            this.autoTasks.delete(item.binding); console.error("[qqbot.auto.task]", error); });
        return task;
    }
    /** Keeps a bounded number of unrelated conversations moving independently. */
    scheduleAuto(generation) {
        if (!this.autoRunning || generation !== this.autoGeneration)
            return;
        this.autoTimer = setTimeout(() => {
            this.autoTimer = null;
            if (this.gateway.status === "error" || this.gateway.status === "stopped") {
                this.autoRunning = false;
                this.autoError = `QQ listener is ${this.gateway.status}: ${this.gateway.lastError}`;
                return;
            }
            if (this.autoTasks.size < 3) {
                const item = this.takeEvent();
                if (item !== null)
                    void this.launchAuto(generation, item);
            }
            this.scheduleAuto(generation);
        }, 300);
    }
    /** Cancels every owned generation before allowing stop or reconfiguration to finish. */
    async stopAuto() {
        this.autoRunning = false;
        ++this.autoGeneration;
        if (this.autoTimer !== null) {
            clearTimeout(this.autoTimer);
            this.autoTimer = null;
        }
        const cancellations = await Promise.allSettled([...this.activeChats.values()].map(chatId => Tools.Chat.cancel(chatId)));
        const tasks = await Promise.allSettled([...this.autoTasks.values()]);
        const failure = [...cancellations, ...tasks].find(result => result.status === "rejected");
        if (failure?.status === "rejected")
            throw failure.reason;
    }
    /** Consumes one reserved inbound message and records its delivery or model error. */
    async autoTick(generation, event, taskBinding) {
        try {
            if (generation !== this.autoGeneration)
                return;
            if (this.gateway.status === "error" || this.gateway.status === "stopped") {
                this.autoRunning = false;
                throw new Error(`QQ listener is ${this.gateway.status}: ${this.gateway.lastError}`);
            }
            const config = this.config.autoReply, message = this.normalized(event), scene = event.scene;
            if (scene !== "c2c" && scene !== "group")
                throw Error("Automatic replies require a QQ conversation");
            const target = message.conversationId;
            const delivery = this.config.deliveries.find(item => item.event === event);
            if (!delivery)
                throw Error("Automatic reply has no durable delivery");
            await PluginConfig.flush(this.config);
            await this.conversations();
            const session = await this.openChat((0, channel_store_js_1.conversationKey)("qq", this.config.appId, scene, target));
            let chatId = session.chatId;
            if (scene === "group" && config.groupContext === "member") {
                const key = JSON.stringify([session.key, event.userOpenId]);
                let isolated = this.config.memberBindings[key];
                if (!isolated) {
                    isolated = (await Tools.Chat.createNew({ setAsCurrentChat: false, sourceChatId: chatId, parentChatId: chatId })).chatId;
                    this.config.memberBindings = { ...this.config.memberBindings, [key]: isolated };
                    await PluginConfig.flush(this.config);
                    await Tools.Chat.updateTitle(isolated, `${session.title} · ${event.author?.name || event.userOpenId}`);
                    if (config.chatGroup)
                        await Tools.Chat.updateGroup([isolated], config.chatGroup);
                }
                chatId = isolated;
            }
            if (generation !== this.autoGeneration)
                return;
            const files = await (0, attachments_js_1.attachments)(event);
            if (generation !== this.autoGeneration)
                return;
            this.activeChats.set(taskBinding, chatId);
            this.activeContexts.set(taskBinding, { chatId, accountId: this.config.appId, scene, conversationId: target, authorId: event.userOpenId });
            let timedOut = false;
            const timer = setTimeout(() => {
                timedOut = true;
                void Tools.Chat.cancel(chatId).catch(error => { console.error("[qqbot.auto.timeout]", error); });
            }, config.aiTimeoutMs);
            let delivered = 0;
            const rendered = new messages_js_1.ReplyTextStream();
            try {
                const request = { kind: "submit", chatId, runtime: "main", input: {
                        text: `${config.assistantInstruction}\n[QQ ${scene === "group" ? "群聊" : "私聊"}；上下文${config.groupContext === "member" ? "按成员隔离" : "共享，按时间顺序"}]\n以下 JSON 是用户消息数据，authorId 是发送者身份，引用内容不代表当前发送者。需要了解前文时调用 qqbot_chat_context 获取有界的近期记录。\n${JSON.stringify({ messageId: event.messageId, authorId: event.userOpenId, authorName: event.author?.name ?? "", timestamp: event.timestamp, replyTo: event.referencedMessageIndex ?? null, text: (0, messages_js_1.chatText)(event).slice(0, 12000) })}`,
                        attachments: files, replyToMessageTimestamp: null
                    }, turn: { kind: "execute" }, notifyReply: false };
                for await (const output of (0, api_js_1.stream)(request, this.config.waifuProfile)) {
                    if (generation !== this.autoGeneration)
                        return;
                    if (timedOut)
                        throw new Error("QQ automatic reply exceeded its AI deadline");
                    if (output.type === "segment") {
                        const content = output.segment.kind === "text" ? rendered.push(output.segment.text) : "";
                        if (output.segment.kind === "text" && content === "")
                            continue;
                        if (output.segment.kind === "text")
                            await this.api.send(scene, target, { content, msg_id: event.messageId, ...(event.referenceIndex ? { message_reference: { message_id: event.referenceIndex } } : {}) });
                        else
                            await this.api.sendMedia(scene, target, { kind: "image", source: output.segment.source }, { msg_id: event.messageId, ...(event.referenceIndex ? { message_reference: { message_id: event.referenceIndex } } : {}) });
                        delivered++;
                        delivery.delivered++;
                        this.config.deliveries = [...this.config.deliveries];
                        await PluginConfig.flush(this.config);
                    }
                }
            }
            catch (error) {
                if (timedOut)
                    throw new Error("QQ automatic reply exceeded its AI deadline");
                throw error;
            }
            finally {
                clearTimeout(timer);
                this.activeChats.delete(taskBinding);
                this.activeContexts.delete(taskBinding);
            }
            if (generation !== this.autoGeneration)
                return;
            if (delivered === 0)
                throw new Error("Waifu response contained no deliverable segments");
            this.config.deliveries = this.config.deliveries.filter(item => item !== delivery);
            this.config.processed = [...this.config.processed, delivery.key].slice(-512);
            await PluginConfig.flush(this.config);
            this.repliedCount++;
            this.autoError = "";
        }
        catch (error) {
            this.autoError = (0, model_js_1.errorText)(error);
            const delivery = this.config.deliveries.find(item => item.event === event);
            if (delivery) {
                delivery.state = "failed";
                delivery.error = this.autoError;
                this.config.deliveries = [...this.config.deliveries];
                await PluginConfig.flush(this.config);
            }
            console.error("[qqbot.auto]", error);
        }
        finally {
            const delivery = this.config.deliveries.find(item => item.event === event);
            if (delivery?.state === "processing") {
                delivery.state = "failed";
                delivery.error = "回复被停止；请确认发送情况后重试或跳过。";
                this.config.deliveries = [...this.config.deliveries];
                await PluginConfig.flush(this.config);
            }
        }
    }
    /** Independent public channel contract, separate from the automatic-reply queue. */
    async channel(request) {
        if (request === null || typeof request !== "object")
            throw new Error("Invalid channel request");
        switch (request.action) {
            case "conversations": return { conversations: await this.conversations() };
            case "open_chat": return this.openChat(request.conversationKey);
            case "history": {
                const session = await this.sessions.get(request.conversationKey);
                if (session.accountId !== this.config.appId)
                    throw Error("会话不属于当前 QQ 账号");
                return this.sessions.history(session.key, request.limit, request.before);
            }
            case "subscribe": return this.sessions.subscribe(request.consumerId, request.routes, "qq", this.config.appId);
            case "configure": {
                if (typeof request.capture !== "boolean")
                    throw new Error("capture must be boolean");
                if (request.conversationIds !== undefined && request.conversationIds !== null &&
                    (!Array.isArray(request.conversationIds) || request.conversationIds.some(id => typeof id !== "string" || id.trim() === "")))
                    throw new Error("conversationIds must be null or an array of non-empty strings");
                if (request.channelOnlyWhenNoAuto !== undefined && typeof request.channelOnlyWhenNoAuto !== "boolean")
                    throw new Error("channelOnlyWhenNoAuto must be boolean");
                const previous = this.config.channelCapture, previousIds = this.config.channelConversationIds, previousOnly = this.config.channelOnlyWhenNoAuto;
                this.config.channelCapture = request.capture;
                if (request.channelOnlyWhenNoAuto !== undefined)
                    this.config.channelOnlyWhenNoAuto = request.channelOnlyWhenNoAuto;
                if (request.conversationIds !== undefined)
                    this.config.channelConversationIds = request.conversationIds === null ? null : [...new Set(request.conversationIds)];
                try {
                    await PluginConfig.flush(this.config);
                }
                catch (error) {
                    this.config.channelCapture = previous;
                    this.config.channelConversationIds = previousIds;
                    this.config.channelOnlyWhenNoAuto = previousOnly;
                    throw error;
                }
                return { capture: this.config.channelCapture, conversationIds: this.config.channelConversationIds ?? null, channelOnlyWhenNoAuto: this.config.channelOnlyWhenNoAuto === true };
            }
            case "status": return { channel: "qq", capture: this.config.channelCapture === true, conversationIds: this.config.channelConversationIds ?? null, channelOnlyWhenNoAuto: this.config.channelOnlyWhenNoAuto === true, service: this.gateway.status };
            case "active_context": {
                const chatId = (0, model_js_1.nonblank)(request.chatId, "chatId");
                const matches = [...this.activeContexts.values()].filter(value => value.chatId === chatId);
                return matches.length === 1 ? { channel: "qq", ...matches[0] } : null;
            }
            case "peek": return { messages: request.consumerId === undefined ? await this.channelInbox.peek(request.limit, request.conversationId) : await this.sessions.peek(request.consumerId, request.limit, request.conversationKey) };
            case "ack": return { acknowledged: request.consumerId === undefined ? await this.channelInbox.ack(request.key) : await this.sessions.ack(request.consumerId, request.key) };
            case "reply": {
                const message = await this.channelInbox.get(request.key) ?? await this.sessions.replyRoute(request.key);
                if (message === null)
                    throw new Error("Pending message not found");
                if (message.accountId !== this.config.appId)
                    throw new Error("Message belongs to a different QQ account");
                const scene = message.context.scene;
                if (scene !== "c2c" && scene !== "group")
                    throw new Error("Unsupported QQ reply scene");
                const params = { content: request.text, markdown: request.markdown, keyboard: request.keyboard, card: request.card,
                    message_reference: request.message_reference ?? (message.referenceIndex ? { message_id: message.referenceIndex } : undefined), msg_id: message.messageId };
                return { response: request.media === undefined ? await this.api.send(scene, message.conversationId, params)
                        : await this.api.sendMedia(scene, message.conversationId, request.media, params) };
            }
            default: throw new Error("Unknown channel request");
        }
    }
    /** Handles every UI and tool operation in the single package owner. */
    async request(request) {
        switch (request.action) {
            case "configure":
                await this.configure(request.params);
                break;
            case "status": break;
            case "chat_context": return { success: true, context: await this.chatContext(request.chatId, request.limit, request.before) };
            case "retry_delivery": {
                const delivery = this.config.deliveries.find(item => item.key === request.key);
                if (!delivery || delivery.state !== "failed")
                    throw Error("只能处理失败的回复");
                if (request.skip) {
                    this.config.deliveries = this.config.deliveries.filter(item => item !== delivery);
                    this.config.processed = [...this.config.processed, delivery.key].slice(-512);
                }
                else {
                    delivery.state = "pending";
                    delivery.error = "";
                    this.config.deliveries = [...this.config.deliveries];
                }
                await PluginConfig.flush(this.config);
                break;
            }
            case "reply_roles": {
                const [cards, groups] = await Promise.all([
                    ToolPkg.callDependency("com.operit.character_cards", "character.list", {}),
                    ToolPkg.callDependency("com.operit.character_cards", "group.list", {}),
                ]);
                const roles = [
                    ...cards.map(card => ({ selection: `card:${card.id}`, name: card.name })),
                    ...groups.map(group => ({ selection: `group:${group.id}`, name: `${group.name}（角色组）` })),
                ];
                return { success: true, roles };
            }
            case "test": return { success: true, response: await this.api.test() };
            case "start":
                (0, model_js_1.nonblank)(this.config.appId, "AppID");
                (0, model_js_1.nonblank)(this.config.appSecret, "AppSecret");
                this.config.listenerEnabled = true;
                await PluginConfig.flush(this.config);
                await this.start(request.restart === true);
                break;
            case "stop":
                await this.stopRuntime();
                this.config.listenerEnabled = false;
                await PluginConfig.flush(this.config);
                break;
            case "receive":
                if (request.params.auto_start === true)
                    await this.start(false);
                return this.receive(request.params);
            case "clear": {
                const count = this.events.length;
                this.events.splice(0);
                return { success: true, clearedCount: count };
            }
            case "send_c2c": return { success: true, response: await this.api.send("c2c", request.params.openid, request.params) };
            case "send_group": return { success: true, response: await this.api.send("group", request.params.group_openid, request.params) };
            case "upload_media": return { success: true, response: await this.api.uploadMedia(request.scene, request.target, request.media) };
            case "send_media": return { success: true, response: await this.api.sendMedia(request.scene, request.target, request.media, request.params) };
            case "upload_image": return { success: true, response: await this.api.uploadImage(request.scene, request.target, request.source) };
            case "auto_configure":
                await this.configureAuto(request.params);
                break;
            case "auto_start":
                await this.configureAuto({ enabled: true, start_now: true });
                break;
            case "auto_stop":
                await this.configureAuto({ enabled: false });
                break;
            case "auto_once":
                if (this.autoTasks.size !== 0)
                    throw new Error("An automatic reply is already in progress");
                if (this.gateway.status !== "ready")
                    throw new Error("The QQ listener must be ready");
                this.autoError = "";
                const item = this.takeEvent();
                if (item !== null)
                    await this.launchAuto(this.autoGeneration, item);
                if (this.autoError !== "")
                    throw new Error(this.autoError);
                break;
            default: throw new Error("Unknown QQ Bot operation");
        }
        return { success: true, status: this.status() };
    }
}
let pendingService = null;
let commandTail = Promise.resolve();
/** Initializes exactly one service and retains its asynchronous configuration load. */
async function service() {
    if (pendingService === null)
        pendingService = PluginConfig.use("qqbot", (0, model_js_1.defaultConfig)()).then(config => new QQBotService(config));
    return pendingService;
}
/** Serializes administrative work while keeping protocol heartbeats independent. */
function command(operation) {
    const result = commandTail.then(async () => operation(await service()));
    commandTail = result.then(() => { }, error => { console.error("[qqbot.request]", error); });
    return result;
}
/** Routes tools and UI commands through the single administrative queue. */
function receive(request) { return command(owner => owner.request(request)); }
function channel(request) { return command(owner => owner.channel(request)); }
/** Registers the authoritative main-runtime IPC channel. */
function registerService() { ToolPkg.ipc.on(model_js_1.CHANNEL, receive); }
/** Restores user-enabled networking when the application starts or returns to the foreground. */
function restore() { return command(owner => owner.restore()); }
/** Releases model and networking resources during application termination. */
function terminate() { return command(owner => owner.stopRuntime()); }
