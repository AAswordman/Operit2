import { CHANNELS, PACKAGE, type Channel, type Connector, type ChannelStatus, type Dashboard, type History,
  type HistoryMessage, type Media, type Message, type Request, type Responses, type RouteRequest, type SendFields } from "./api.js";
import type { Storage } from "../../../../types/storage.js";

export const IPC = "connector-hub.console";
const providers: Record<Channel, string> = { qq: "com.operit.qqbot_bundle", wechat: "com.operit.wechat_bot", feishu: "com.operit.feishu_bot" };
const labels = { qq: "QQ Bot", wechat: "微信 Bot", feishu: "飞书 Bot" };
const maintenance = "com.operit.project_maintenance";
const routeActions = new Set(["status", "conversations", "open_chat", "history", "active_context", "subscribe", "peek", "ack", "configure", "reply", "send"]);
export type Manage = { action: "manage"; channel: Channel } & (
  | { command: "start" | "stop" | "test"; autoReply?: boolean }
  | { command: "group_auto_reply"; enabled: boolean }
  | { command: "connect"; appId: string; appSecret: string }
  | { command: "login_start" | "login_cancel" | "login_status" | "logout" }
  | { command: "login_verify"; code: string }
);
export type ManagementStatus = ChannelStatus & { login?: { state: string; qrContent: string; expiresAt: number; error: string } };
export type ConsoleAttachment = { fileName: string; mimeType: string; contentBase64: string };
export type ConsoleRequest = Request | Manage | ({ action: "console_reply"; channel: Channel; conversationKey: string; key: string } & SendFields)
  | { action: "attachment"; channel: Channel; conversationKey: string; messageId: string; index: number };

async function bot<T>(channel: Channel, payload: object): Promise<T> {
  if (!CHANNELS.includes(channel)) throw Error("未知的外部渠道");
  try { return await ToolPkg.callDependency<object, T>(providers[channel], "channel.v1", payload); }
  catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (/ToolPkg container not found|(?:package|extension owner|public API owner) is disabled|Dependency version mismatch|Public API not found/i.test(message)) {
      throw Error(`${labels[channel]} 未安装、未启用或版本过旧`);
    }
    throw Error(message.replace(/(?:https?|wss):\/\/[^\s"<>]+/g, "[连接地址]").slice(0, 400));
  }
}

async function dashboard(): Promise<Dashboard> {
  const rows = await Promise.all(CHANNELS.map(async channel => {
    try {
      const status = await bot<ChannelStatus>(channel, { action: "status" });
      const connector: Connector = { channel, available: true, status };
      try { return { connector, conversations: (await bot<Responses["conversations"]>(channel, { action: "conversations" })).conversations }; }
      catch (error) { connector.issue = String(error instanceof Error ? error.message : error); return { connector, conversations: [] }; }
    } catch (error) {
      return { connector: { channel, available: false, issue: String(error instanceof Error ? error.message : error) }, conversations: [] };
    }
  }));
  return { connectors: rows.map(row => row.connector), conversations: rows.flatMap(row => row.conversations).sort((a, b) => b.lastMessageAt - a.lastMessageAt) };
}

function sanitizedMessage(message: Message): Message {
  const context = message.context ?? {};
  const scene = message.channel === "wechat" ? "c2c" : context.scene;
  let plainText = false;
  const media: { kind: string; name?: string }[] = [];
  if (message.channel === "wechat" && Array.isArray(context.items)) {
    const kinds: Record<number, string> = { 2: "image", 3: "audio", 4: "file", 5: "video" };
    plainText = context.items.every(item => item?.type === 1 && !item.ref_msg);
    for (const item of context.items.slice(0, 20)) {
      const kind = kinds[item?.type]; if (kind) media.push({ kind });
      const quoted = kinds[item?.ref_msg?.message_item?.type]; if (quoted) media.push({ kind: quoted });
    }
  } else if (message.channel === "qq") {
    const raw = context.raw as { attachments?: unknown } | undefined;
    const attachments = (message as Message & { attachments?: { kind?: string; filename?: string }[] }).attachments;
    const keys = new Set(["id", "content", "timestamp", "author", "group_openid", "channel_id", "guild_id", "attachments"]);
    plainText = !!raw && Object.keys(raw).every(key => keys.has(key)) && (!raw.attachments || Array.isArray(raw.attachments) && !raw.attachments.length);
    for (const item of attachments?.slice(0, 20) ?? []) media.push({ kind: item.kind || "file" });
  } else if (message.channel === "feishu" && Array.isArray(context.resources)) {
    for (const item of context.resources.slice(0, 20)) if (typeof item?.kind === "string") media.push({ kind: item.kind, name: typeof item.name === "string" ? item.name.slice(0, 150) : undefined });
    plainText = context.messageType === "text" && !media.length && !context.parentId;
  }
  const { key, channel, accountId, messageId, conversationId, authorId, text, timestamp, autoReplyEnabled } = message;
  return { key, channel, accountId, messageId, conversationId, authorId, text, timestamp, autoReplyEnabled, context: { scene, media, plainText } };
}

function validateSend(channel: Channel, fields: SendFields): void {
  const parts = [fields.text, fields.media, fields.markdown, fields.post, fields.card].filter(part => part !== undefined);
  if (parts.length !== 1) throw Error("每次发送一种内容");
  if (fields.markdown && channel !== "qq" || fields.post && channel !== "feishu" || fields.card && channel === "wechat") throw Error("该渠道不支持这种消息类型");
}

function replyKey(history: History, messageId: string): string {
  const { channel, accountId, scene, conversationId } = history.conversation;
  const base = [channel, accountId, conversationId, messageId];
  if (channel === "feishu") return JSON.stringify(base);
  if (channel === "qq") base.splice(2, 0, scene);
  return base.map(encodeURIComponent).join(":");
}

let legacyDb: Promise<Storage.ObjectDatabase> | undefined;
async function legacyOutgoing(conversationKey: string): Promise<HistoryMessage[]> {
  try {
    const db = await (legacyDb ??= Tools.Storage.objects.open({ path: ToolPkg.getLocalDataDir() + "/outgoing.sqlite" }).catch(error => { legacyDb = undefined; throw error; }));
    return (await db.collection<HistoryMessage[]>("history").get(conversationKey))?.value ?? [];
  } catch { return []; }
}

async function history(channel: Channel, payload: Extract<RouteRequest, { action: "history" }>): Promise<History> {
  const value = await bot<History & { latestInboundId?: string }>(channel, payload);
  if (value.conversation.channel !== channel || value.conversation.key !== payload.conversationKey) throw Error("会话来源不一致");
  const out = await legacyOutgoing(payload.conversationKey);
  const rows = [...value.messages.map(({ messageId, authorName, text, timestamp, receivedAt, media, direction, delivery }) => (
    { messageId, authorName, text, timestamp, receivedAt, media, ...(direction === "outgoing" ? { direction, delivery } : { replyKey: replyKey(value, messageId) }) }
  )), ...out.filter(item => payload.before === undefined || item.receivedAt < payload.before)].sort((a, b) => a.receivedAt - b.receivedAt);
  const limit = payload.limit ?? 20;
  return { conversation: value.conversation, messages: rows.slice(-limit), truncated: value.truncated || rows.length > limit,
    ...(value.latestInboundId ? { latestReplyKey: replyKey(value, value.latestInboundId) } : {}) };
}

export async function dispatch(payload: Request, caller: string): Promise<unknown> {
  if (!payload || typeof payload !== "object" || !caller) throw Error("无效的连接器请求");
  if (payload.action === "dashboard") return dashboard();
  if (!routeActions.has(payload.action)) throw Error("未知的连接器操作");
  const { channel, ...body } = payload;
  switch (payload.action) {
    case "configure":
      if (caller !== maintenance) throw Error("没有迁移旧版采集设置的权限");
      return bot(channel, { action: "configure", capture: payload.capture, conversationIds: payload.conversationIds, channelOnlyWhenNoAuto: payload.channelOnlyWhenNoAuto });
    case "history": return history(channel, payload);
    case "subscribe": return bot(channel, { action: "subscribe", routes: payload.routes, consumerId: caller });
    case "peek": {
      if (typeof payload.conversationKey !== "string" || !payload.conversationKey) throw Error("缺少会话键");
      const result = await bot<Responses["peek"]>(channel, { action: "peek", conversationKey: payload.conversationKey, limit: payload.limit, consumerId: caller });
      return { messages: result.messages.map(sanitizedMessage) };
    }
    case "ack": return bot(channel, { action: "ack", key: payload.key, consumerId: caller });
    case "reply":
    case "send": {
      validateSend(channel, payload);
      if (payload.action === "send" && channel !== "feishu") throw Error("此渠道需要回复一条已收到的消息");
      const { text, media, markdown, post, card } = payload;
      await bot(channel, { action: payload.action, ...(payload.action === "reply" ? { key: payload.key } : { conversationKey: payload.conversationKey }), text, media, markdown, post, card });
      return { response: null };
    }
    default: return bot(channel, body);
  }
}

export async function consoleRequest(payload: ConsoleRequest): Promise<unknown> {
  if (payload.action === "attachment") {
    const { channel, ...request } = payload;
    const result = await bot<ConsoleAttachment>(channel, request).catch(() => { throw Error("无法读取附件，可能已过期、权限不足或超过 8 MiB"); });
    const { fileName, mimeType, contentBase64 } = result;
    if (typeof contentBase64 !== "string" || !contentBase64 || contentBase64.length > Math.ceil((8 * 1024 * 1024) / 3) * 4) throw Error("附件内容无效或超过 8 MiB");
    return {fileName: typeof fileName === "string" ? fileName.split(/[\\/]/).pop()!.replace(/[\u0000-\u001f\u007f:]/g, "_").slice(0,150) : "附件",
      mimeType: typeof mimeType === "string" && /^[\w.+-]+\/[\w.+-]+$/.test(mimeType) ? mimeType : "application/octet-stream", contentBase64};
  }
  if (payload.action === "manage") {
    const { channel, ...command } = payload;
    if (command.command === "group_auto_reply" && channel !== "feishu") throw Error("此设置仅适用于飞书");
    return bot<ManagementStatus>(channel, command);
  }
  if (payload.action === "console_reply") {
    const { conversationKey, key, channel, text, media } = payload;
    const existing = await history(channel, { action: "history", channel, conversationKey, limit: 50 });
    if (!key || existing.latestReplyKey !== key && !existing.messages.some(item => item.replyKey === key)) throw Error("该消息不在近期会话记录内，请刷新后再试");
    return dispatch({ action: "reply", channel, key, text, media }, PACKAGE);
  }
  return dispatch(payload, PACKAGE);
}
