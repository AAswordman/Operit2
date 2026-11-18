import type { Net } from "../../../../types/network";
import type { HttpResponseData } from "../../../../types/results";
import type { RecordSend, SendSummary } from "./channel-store.js";
import {
  API_BASE,
  VERSION,
  integer,
  nonblank,
  parseJson,
  text,
  wechatUrl,
  type Account,
  type JsonObject,
} from "./model.js";

const [majorVersion, minorVersion, patchVersion] =
  VERSION.split(".").map(Number);
const CLIENT_VERSION = String(
  (majorVersion << 16) | (minorVersion << 8) | patchVersion,
);

export interface HttpPort {
  request(options: Net.HostHttpOptions): Promise<HttpResponseData>;
  upload(url: string, base64: string): Promise<HttpResponseData>;
}

/** Carries exact WeChat business failures without conflating them with network errors. */
export class WeChatApiError extends Error {
  /** Retains the server code for explicit expired-session handling. */
  constructor(
    readonly code: number,
    message: string,
  ) {
    super(`微信接口 ${code}: ${message}`);
    this.name = "WeChatApiError";
  }
}

export class WeChatTransportError extends Error {}
class WeChatHttpError extends Error {
  constructor(readonly status: number) { super(`微信 HTTP ${status}`); }
}

/** Implements the official QR, long-poll, message and typing HTTP endpoints. */
export class WeChatApi {
  /** Binds a host HTTP port and an optional authenticated account. */
  constructor(
    readonly http: HttpPort,
    private readonly account: Account | null = null,
    private readonly record?: RecordSend,
  ) {}

  /** Builds official application headers without reusing tokens on QR requests. */
  private headers(
    authenticated: boolean,
    post: boolean,
  ): Record<string, string> {
    const headers: Record<string, string> = {
      "iLink-App-Id": "bot",
      "iLink-App-ClientVersion": CLIENT_VERSION,
    };
    if (post) {
      headers["Content-Type"] = "application/json";
      headers.AuthorizationType = "ilink_bot_token";
      const random = parseInt(CryptoJS.binary.randomHex(4), 16);
      headers["X-WECHAT-UIN"] = encodeAsciiBase64(String(random));
    }
    if (authenticated)
      headers.Authorization = `Bearer ${this.requireAccount().token}`;
    return headers;
  }

  /** Requires a fully authenticated account before issuing a Bot request. */
  requireAccount(): Account {
    if (this.account === null) throw new Error("请先扫码登录微信机器人。");
    return this.account;
  }

  /** Checks HTTP status and documented business error fields separately. */
  private async json(options: Net.HostHttpOptions): Promise<JsonObject> {
    const response = await this.http.request({
      connect_timeout: 15,
      read_timeout: 45,
      follow_redirects: false,
      responseType: "text",
      ...options,
    }).catch(error => { throw new WeChatTransportError(String(error)); });
    if (response.statusCode === 429 || response.statusCode >= 500)
      throw new WeChatTransportError(`微信 HTTP ${response.statusCode}`);
    if (response.statusCode < 200 || response.statusCode >= 300)
      throw new WeChatHttpError(response.statusCode);
    const data = parseJson(response.content);
    const codes = ["ret", "errcode"].filter(field => data[field] !== undefined)
      .map(field => integer(data[field], field, -2147483648, 2147483647));
    const code = codes.includes(-14) ? -14 : codes.find(value => value !== 0);
    if (code !== undefined) throw new WeChatApiError(code, data.errmsg === undefined ? "请求失败" : text(data.errmsg, "errmsg"));
    return data;
  }

  /** Requests a new QR code without sending existing credentials. */
  loginStart(): Promise<JsonObject> {
    return this.json({
      url: `${API_BASE}/ilink/bot/get_bot_qrcode?bot_type=3`,
      method: "POST",
      headers: this.headers(false, true),
      body: { local_token_list: [] },
    });
  }

  /** Polls one QR session on its explicitly selected WeChat routing host. */
  loginPoll(base: string, qrcode: string, code: string): Promise<JsonObject> {
    let url = `${wechatUrl(base, "QR base URL")}/ilink/bot/get_qrcode_status?qrcode=${encodeURIComponent(qrcode)}`;
    if (code !== "") url += `&verify_code=${encodeURIComponent(code)}`;
    return this.json({
      url,
      method: "GET",
      headers: this.headers(false, false),
    });
  }

  /** Sends an authenticated JSON operation with Operit attribution metadata. */
  post(
    path: string,
    body: JsonObject = {},
    timeoutSeconds = 20,
  ): Promise<JsonObject> {
    const account = this.requireAccount();
    return this.json({
      url: `${wechatUrl(account.baseUrl, "API base URL")}/ilink/bot/${path}`,
      method: "POST",
      headers: this.headers(true, true),
      read_timeout: timeoutSeconds,
      body: {
        ...body,
        base_info: { channel_version: VERSION, bot_agent: `Operit/${VERSION}` },
      },
    });
  }

  /** Fetches one server batch with an exact persistent get_updates_buf cursor. */
  updates(cursor: string, timeoutSeconds: number): Promise<JsonObject> {
    return this.post("getupdates", { get_updates_buf: cursor }, timeoutSeconds);
  }

  /** Delivers exactly one text or media item with a caller-selected conversation context. */
  send(
    userId: string,
    contextToken: string,
    item: JsonObject,
  ): Promise<JsonObject> {
    const account = this.requireAccount().botId;
    const type = integer(item.type, "message item type", 1, 5);
    const kinds = ["image", "audio", "file", "video"] as const;
    const summary = type === 1 ? { text: text((item.text_item as JsonObject)?.text, "text") } : {
      text: "", media: [{ kind: kinds[type - 2]!, name: type === 4 && typeof (item.file_item as JsonObject)?.file_name === "string" ? String((item.file_item as JsonObject).file_name) : undefined }],
    };
    const payload = {
      msg: {
        from_user_id: "",
        to_user_id: nonblank(userId, "user_id"),
        client_id: `operit-${CryptoJS.binary.randomHex(16)}`,
        message_type: 2,
        message_state: 2,
        context_token: nonblank(contextToken, "context_token"),
        item_list: [item],
      },
    };
    return this.sendTracked(account, userId, summary, payload);
  }

  private async sendTracked(account: string, userId: string, summary: SendSummary, payload: JsonObject): Promise<JsonObject> {
    let response: JsonObject;
    try { response = await this.post("sendmessage", payload); }
    catch (error) {
      const rejected = error instanceof WeChatApiError || (error instanceof WeChatHttpError && error.status >= 400 && error.status < 500 && error.status !== 408);
      await this.record?.(account, "c2c", userId, summary, rejected ? "rejected" : "unknown");
      throw error;
    }
    await this.record?.(account, "c2c", userId, summary, "accepted");
    return response;
  }

  /** Sends a complete plain text message within the channel text limit. */
  sendText(
    userId: string,
    contextToken: string,
    content: string,
  ): Promise<JsonObject> {
    content = content.replace(/\[\[(?:reply_?to|reply_?to_?current)(?::[^\]\r\n]*)?\]\]|\[CQ:reply,[^\]\r\n]*\]|<\/?reply(?:_?to)?\b[^>]*>|\[微信用户=[^\]\r\n]*\]/gi, "").trim();
    nonblank(content, "text");
    if (Array.from(content).length > 4000)
      throw new Error("微信单条文本不能超过 4000 个字符。");
    return this.send(userId, contextToken, {
      type: 1,
      text_item: { text: content },
    });
  }

  /** Obtains a typing ticket for the exact peer and reply context. */
  async typing(
    userId: string,
    contextToken: string,
    active: boolean,
  ): Promise<void> {
    const config = await this.post("getconfig", {
      ilink_user_id: nonblank(userId, "user_id"),
      context_token: nonblank(contextToken, "context_token"),
    });
    const ticket = nonblank(config.typing_ticket, "typing_ticket");
    await this.post("sendtyping", {
      ilink_user_id: userId,
      typing_ticket: ticket,
      status: active ? 1 : 2,
    });
  }
}

/** Encodes protocol ASCII fields without requiring browser or Node globals. */
export function encodeAsciiBase64(value: string): string {
  const alphabet =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let output = "";
  for (let index = 0; index < value.length; index += 3) {
    const a = value.charCodeAt(index),
      b = value.charCodeAt(index + 1),
      c = value.charCodeAt(index + 2);
    if (a > 127 || b > 127 || c > 127) throw new Error("Expected ASCII input");
    const bits = (a << 16) | ((b || 0) << 8) | (c || 0);
    output += alphabet[(bits >>> 18) & 63] + alphabet[(bits >>> 12) & 63];
    output += index + 1 < value.length ? alphabet[(bits >>> 6) & 63] : "=";
    output += index + 2 < value.length ? alphabet[bits & 63] : "=";
  }
  return output;
}

/** Creates the ordinary cross-platform host networking port, including binary uploads. */
export function hostHttp(): HttpPort {
  return {
    request: (options) => Tools.Net.http(options),
    async upload(url, base64) {
      const client = OkHttp.newBuilder()
        .connectTimeout(15000)
        .readTimeout(60000)
        .writeTimeout(60000)
        .followRedirects(false)
        .retryOnConnectionFailure(false)
        .build();
      const response = await client
        .newRequest()
        .url(url)
        .method("POST")
        .header("Content-Type", "application/octet-stream")
        .body(base64, "base64")
        .build()
        .execute();
      return response.raw;
    },
  };
}
