export const CHANNEL = "qqbot.service";
export const MAX_EVENTS = 200;
export type Scene = "c2c" | "group" | "unknown";
export type JsonObject = Record<string, unknown>;

export interface AutoReplyConfig {
  enabled: boolean;
  c2cEnabled: boolean;
  groupEnabled: boolean;
  aiTimeoutMs: number;
  chatGroup: string;
  characterSelection: string;
  assistantInstruction: string;
}
export interface Config {
  waifuProfile: string;
  appId: string;
  appSecret: string;
  useSandbox: boolean;
  listenerEnabled: boolean;
  autoReply: AutoReplyConfig;
  bindings: Record<string, string>;
  processed: string[];
}
export interface InboundEvent {
  scene: Scene;
  eventType: string;
  eventId: string;
  seq: number;
  messageId: string;
  content: string;
  timestamp: string;
  receivedAt: string;
  userOpenId: string;
  groupOpenId: string;
  rawPayload?: JsonObject;
  replyHint: { scene: Scene; msg_id: string; event_id: string; openid: string; group_openid: string };
}
export interface SendParams {
  content?: string;
  msg_type?: number;
  msg_id?: string;
  event_id?: string;
  msg_seq?: number;
  payload?: JsonObject;
}
export interface ConfigureParams { app_id?: string; app_secret?: string; use_sandbox?: boolean; listener_enabled?: boolean; test_connection?: boolean; restart_service?: boolean }
export interface ReceiveParams { limit?: number; consume?: boolean; scene?: Scene; event_type?: string; include_raw?: boolean; auto_start?: boolean }
export interface AutoReplyParams { enabled?: boolean; c2c_enabled?: boolean; group_enabled?: boolean; ai_timeout_ms?: number; chat_group?: string; character_selection?: string; assistant_instruction?: string; waifu_profile?: string; start_now?: boolean }
export type Request =
  | { action: "configure"; params: ConfigureParams }
  | { action: "status" }
  | { action: "reply_roles" }
  | { action: "test" }
  | { action: "start"; restart?: boolean }
  | { action: "stop" }
  | { action: "receive"; params: ReceiveParams }
  | { action: "clear" }
  | { action: "send_c2c"; params: SendParams & { openid: string } }
  | { action: "send_group"; params: SendParams & { group_openid: string } }
  | { action: "upload_image"; scene: "c2c" | "group"; target: string; source: string }
  | { action: "auto_configure"; params: AutoReplyParams }
  | { action: "auto_start" }
  | { action: "auto_stop" }
  | { action: "auto_once" };
export interface ReplyRole { selection: string; name: string }
export interface Status {
  waifuProfile: string;
  configured: boolean;
  appId: string;
  useSandbox: boolean;
  listenerEnabled: boolean;
  service: { status: "stopped" | "connecting" | "ready" | "error"; sessionId: string; lastError: string; receivedCount: number };
  queue: { size: number; capacity: number };
  autoReply: { config: AutoReplyConfig; running: boolean; lastError: string; bindings: Record<string, string>; repliedCount: number };
}
export type Response =
  | { success: true; status: Status }
  | { success: true; roles: ReplyRole[] }
  | { success: true; returnedCount: number; remainingCount: number; events: InboundEvent[] }
  | { success: true; clearedCount: number }
  | { success: true; response: JsonObject };

/** Creates the authored initial persisted configuration. */
export function defaultConfig(): Config {
  return { waifuProfile: "default", appId: "", appSecret: "", useSandbox: false, listenerEnabled: false,
    autoReply: { enabled: false, c2cEnabled: true, groupEnabled: true, aiTimeoutMs: 120000, chatGroup: "QQ Bot", characterSelection: "", assistantInstruction: "请直接回复 QQ 用户，不要输出内部思考或工具执行过程。" },
    bindings: {}, processed: [] };
}

/** Requires an object received at a JSON protocol boundary. */
export function object(value: unknown, field: string): JsonObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${field} must be an object`);
  return value as JsonObject;
}

/** Requires a string without converting invalid protocol data. */
export function text(value: unknown, field: string): string {
  if (typeof value !== "string") throw new Error(`${field} must be a string`);
  return value;
}

/** Requires a nonblank credential, identifier or submitted message. */
export function nonblank(value: unknown, field: string): string {
  const result = text(value, field).trim();
  if (result === "") throw new Error(`${field} must not be blank`);
  return result;
}

/** Checks bounded integers before they reach timers, queues or remote APIs. */
export function integer(value: unknown, field: string, minimum: number, maximum: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum || value > maximum) throw new Error(`${field} must be an integer between ${minimum} and ${maximum}`);
  return value;
}

/** Formats actual thrown values for persisted runtime diagnostics. */
export function errorText(error: unknown): string { return error instanceof Error ? error.message : String(error); }

/** Resolves the exact QQ event schema rather than guessing identifier aliases. */
export function inbound(payload: JsonObject): InboundEvent {
  const eventType = nonblank(payload.t, "dispatch.t");
  const body = object(payload.d, "dispatch.d");
  const seq = integer(payload.s, "dispatch.s", 0, Number.MAX_SAFE_INTEGER);
  let scene: Scene = "unknown", userOpenId = "", groupOpenId = "", messageId = "", content = "", timestamp = "";
  if (eventType === "C2C_MESSAGE_CREATE" || eventType === "GROUP_AT_MESSAGE_CREATE" || eventType === "GROUP_MESSAGE_CREATE") {
    scene = eventType === "C2C_MESSAGE_CREATE" ? "c2c" : "group";
    const author = object(body.author, "message.author");
    userOpenId = nonblank(scene === "c2c" ? author.user_openid : author.member_openid, "message.author openid");
    if (scene === "group") groupOpenId = nonblank(body.group_openid, "message.group_openid");
    messageId = nonblank(body.id, "message.id");
    if (body.content !== undefined) content = text(body.content, "message.content");
    timestamp = text(body.timestamp, "message.timestamp");
  }
  const eventId = payload.id === undefined ? "" : text(payload.id, "dispatch.id");
  return { scene, eventType, eventId, seq, messageId, content, timestamp, receivedAt: new Date().toISOString(), userOpenId, groupOpenId, rawPayload: payload,
    replyHint: { scene, msg_id: messageId, event_id: eventId, openid: userOpenId, group_openid: groupOpenId } };
}
