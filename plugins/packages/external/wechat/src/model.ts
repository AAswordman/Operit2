export const CHANNEL = "wechat.service";
export const API_BASE = "https://ilinkai.weixin.qq.com";
export const CDN_BASE = "https://novac2c.cdn.weixin.qq.com/c2c";
export const MAX_EVENTS = 200;
export const MAX_MEDIA_BYTES = 25 * 1024 * 1024;
export const VERSION = "1.0.0";

export type JsonObject = Record<string, unknown>;
export type MediaKind = "image" | "video" | "file" | "voice";
export type ServiceState = "stopped" | "connecting" | "ready" | "error";
export type LoginState =
  | "idle"
  | "wait"
  | "scaned"
  | "confirmed"
  | "expired"
  | "need_verifycode"
  | "verify_code_blocked"
  | "scaned_but_redirect"
  | "binded_redirect"
  | "error";

export interface Account {
  botId: string;
  userId: string;
  token: string;
  baseUrl: string;
}

export interface AutoReplyConfig {
  enabled: boolean;
  aiTimeoutMs: number;
  chatGroup: string;
  characterSelection: string;
  assistantInstruction: string;
  waifuProfile: string;
}

export interface InboundEvent {
  messageId: string;
  userId: string;
  text: string;
  timestamp: number;
  items: JsonObject[];
  contextToken: string;
  raw: JsonObject;
}

export interface Delivery {
  event: InboundEvent;
  state: "processing" | "failed";
  delivered: number;
  error: string;
}

export interface Config {
  account: Account | null;
  listenerEnabled: boolean;
  autoReply: AutoReplyConfig;
  cursor: string;
  queue: InboundEvent[];
  seen: string[];
  contexts: Record<string, string>;
  bindings: Record<string, string>;
  history: InboundEvent[];
  delivery: Delivery | null;
}

export interface LoginStatus {
  state: LoginState;
  qrContent: string;
  expiresAt: number;
  error: string;
}

export interface Status {
  account: { botId: string; userId: string } | null;
  login: LoginStatus;
  listenerEnabled: boolean;
  service: { state: ServiceState; error: string; receivedCount: number };
  queue: { size: number; capacity: number };
  autoReply: {
    config: AutoReplyConfig;
    running: boolean;
    error: string;
    repliedCount: number;
    bindingCount: number;
    delivery: {
      messageId: string;
      userId: string;
      state: string;
      delivered: number;
      error: string;
    } | null;
  };
}

export interface ReplyRole {
  selection: string;
  name: string;
}
export interface ReceiveParams {
  limit?: number;
  consume?: boolean;
  user_id?: string;
  include_raw?: boolean;
}
export interface SendParams {
  user_id: string;
  text?: string;
  context_token?: string;
}
export interface MediaParams extends SendParams {
  source: string;
  kind: MediaKind;
  file_name?: string;
  voice_format?: number;
  sample_rate?: number;
  duration_ms?: number;
}
export interface AutoReplyParams {
  enabled?: boolean;
  ai_timeout_ms?: number;
  chat_group?: string;
  character_selection?: string;
  assistant_instruction?: string;
  waifu_profile?: string;
}

export type Request =
  | {
      action:
        | "status"
        | "login_start"
        | "login_cancel"
        | "logout"
        | "reply_roles"
        | "test"
        | "stop"
        | "clear"
        | "auto_once";
    }
  | { action: "login_verify"; code: string }
  | { action: "start"; restart?: boolean }
  | { action: "receive"; params: ReceiveParams }
  | { action: "send_text"; params: SendParams }
  | { action: "send_media"; params: MediaParams }
  | { action: "download_media"; message_id: string }
  | { action: "typing"; user_id: string; active: boolean }
  | { action: "auto_configure"; params: AutoReplyParams }
  | { action: "delivery_resolve"; resolution: "retry" | "discard" }
  | { action: "reset_conversation"; user_id: string };

export type Response =
  | { success: true; status: Status }
  | { success: true; roles: ReplyRole[] }
  | { success: true; events: JsonObject[]; remainingCount: number }
  | { success: true; response: JsonObject }
  | { success: true; clearedCount: number };

/** Creates one device-local account and its durable receive queue. */
export function defaultConfig(): Config {
  return {
    account: null,
    listenerEnabled: false,
    autoReply: {
      enabled: true,
      aiTimeoutMs: 120000,
      chatGroup: "微信 Bot",
      characterSelection: "",
      assistantInstruction:
        "请直接回复微信用户，不要输出内部思考或工具执行过程。",
      waifuProfile: "default",
    },
    cursor: "",
    queue: [],
    seen: [],
    contexts: {},
    bindings: {},
    history: [],
    delivery: null,
  };
}

/** Rejects non-object data at protocol boundaries. */
export function object(value: unknown, field: string): JsonObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${field} must be an object`);
  }
  return value as JsonObject;
}

/** Reads a string without silently coercing malformed protocol data. */
export function text(value: unknown, field: string): string {
  if (typeof value !== "string") throw new Error(`${field} must be a string`);
  return value;
}

/** Requires a nonempty identifier, credential or content field. */
export function nonblank(value: unknown, field: string): string {
  const result = text(value, field).trim();
  if (result === "") throw new Error(`${field} must not be blank`);
  return result;
}

/** Validates integer sizes, timer bounds and protocol enum values. */
export function integer(
  value: unknown,
  field: string,
  min: number,
  max: number,
): number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < min ||
    value > max
  ) {
    throw new Error(`${field} must be an integer between ${min} and ${max}`);
  }
  return value;
}

/** Preserves decimal int64 identifiers when parsing JSON responses. */
export function parseJson(source: string): JsonObject {
  const exact = source.replace(
    /"(?:\\.|[^"\\])*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/g,
    (token) => {
      if (/^-?\d+$/.test(token) && !Number.isSafeInteger(Number(token)))
        return JSON.stringify(token);
      return token;
    },
  );
  return object(JSON.parse(exact), "WeChat response");
}

/** Accepts exact remote identifiers without rounding unsafe JSON integers. */
export function identifier(value: unknown, field: string): string {
  if (typeof value === "number")
    return String(integer(value, field, 0, Number.MAX_SAFE_INTEGER));
  return nonblank(value, field);
}

/** Restricts credential-bearing API and media requests to WeChat-owned HTTPS hosts. */
export function wechatUrl(value: unknown, field: string): string {
  const url = nonblank(value, field);
  if (
    !/^https:\/\/(?:[a-z0-9-]+\.)*weixin\.qq\.com(?::443)?(?:[/?][^\s#]*)?$/i.test(
      url,
    )
  ) {
    throw new Error(`${field} must use a WeChat HTTPS host`);
  }
  return url.replace(/\/$/, "");
}

/** Extracts visible text and explicit quoted context from one message item. */
export function itemText(item: JsonObject): string {
  if (item.type === 1)
    return text(object(item.text_item, "text_item").text, "text_item.text");
  if (item.type === 3) {
    const voice = object(item.voice_item, "voice_item");
    return voice.text === undefined ? "" : text(voice.text, "voice_item.text");
  }
  return "";
}

/** Validates a complete incoming user message before committing its receive cursor. */
export function inbound(raw: JsonObject): InboundEvent {
  if (raw.group_id !== undefined && raw.group_id !== "")
    throw new Error("Unsupported group message");
  if (!Array.isArray(raw.item_list) || raw.item_list.length > 32)
    throw new Error("Invalid WeChat item_list");
  const items = raw.item_list.map((item) => object(item, "message item"));
  return {
    messageId: identifier(raw.message_id, "message_id"),
    userId: nonblank(raw.from_user_id, "from_user_id"),
    text: items
      .map(itemText)
      .filter((value) => value !== "")
      .join("\n"),
    timestamp: integer(
      raw.create_time_ms,
      "create_time_ms",
      0,
      Number.MAX_SAFE_INTEGER,
    ),
    items,
    contextToken: nonblank(raw.context_token, "context_token"),
    raw,
  };
}

/** Removes known credentials and request URLs from operational diagnostics. */
export function errorText(error: unknown, secrets: string[] = []): string {
  let result = error instanceof Error ? error.message : String(error);
  result = result.replace(/https:\/\/[^\s"<>]+/g, "[request URL]");
  for (const secret of secrets) {
    if (secret !== "") result = result.split(secret).join("[redacted]");
  }
  return result.slice(0, 1000);
}
