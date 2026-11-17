import type { OutgoingMedia } from "./channel-api.js";
import { externalText } from "./messages.js";
import type { Net } from "../../../../types/network";
import type { Config, JsonObject, SendParams } from "./model.js";
import { text, integer, nonblank, object } from "./model.js";

export interface HttpPort {
  /** Executes an HTTP request through the cross-platform networking Host. */
  request(options: Net.HostHttpOptions): Promise<{ statusCode: number; content: string }>;
}

/** Implements QQ token acquisition and exact OpenAPI message endpoints. */
export class QQOpenApi {
  private token = "";
  private expiresAt = 0;
  private pendingToken: Promise<string> | null = null;
  private readonly sequences = new Map<string, number>();

  /** Binds credentials to one service generation and one networking port. */
  constructor(private readonly config: Config, private readonly http: HttpPort) {}

  /** Parses one real API response and rejects the original unsuccessful HTTP status. */
  private async json(options: Net.HostHttpOptions): Promise<JsonObject> {
    const response = await this.http.request({ connect_timeout: 20, read_timeout: 20, responseType: "text", follow_redirects: false, ...options });
    if (response.statusCode < 200 || response.statusCode >= 300) throw new Error(`QQ HTTP ${response.statusCode}: ${response.content}`);
    return object(JSON.parse(response.content), "QQ API response");
  }

  /** Shares an in-flight token request without repeating credential submissions. */
  async accessToken(): Promise<string> {
    if (this.token !== "" && Date.now() < this.expiresAt - 60000) return this.token;
    if (this.pendingToken !== null) return this.pendingToken;
    this.pendingToken = this.fetchToken();
    try { return await this.pendingToken; }
    finally { this.pendingToken = null; }
  }

  /** Obtains and validates the official application access token. */
  private async fetchToken(): Promise<string> {
    const response = await this.json({ url: "https://bots.qq.com/app/getAppAccessToken", method: "POST", headers: { "Content-Type": "application/json" }, body: { appId: nonblank(this.config.appId, "AppID"), clientSecret: nonblank(this.config.appSecret, "AppSecret") } });
    const token = nonblank(response.access_token, "access_token");
    const expiry = nonblank(response.expires_in, "expires_in");
    if (!/^[1-9]\d*$/.test(expiry)) throw new Error("expires_in must contain positive decimal seconds");
    const expiresIn = integer(Number(expiry), "expires_in", 1, Math.floor((Number.MAX_SAFE_INTEGER - Date.now()) / 1000));
    this.token = token;
    this.expiresAt = Date.now() + expiresIn * 1000;
    return token;
  }

  /** Sends one authenticated OpenAPI request to the explicitly selected environment. */
  async request(method: "GET" | "POST", path: string, body?: JsonObject): Promise<JsonObject> {
    const token = await this.accessToken();
    const base = this.config.useSandbox ? "https://sandbox.api.sgroup.qq.com" : "https://api.sgroup.qq.com";
    const options: Net.HostHttpOptions = { url: base + path, method, headers: { Authorization: `QQBot ${token}`, "X-Union-Appid": this.config.appId, "Content-Type": "application/json" } };
    if (body !== undefined) options.body = body;
    return this.json(options);
  }

  /** Reads the gateway returned for the configured QQ application. */
  async gateway(): Promise<string> {
    const response = await this.request("GET", "/gateway/bot");
    return nonblank(response.url, "gateway.url");
  }

  /** Validates credentials and reports the real profile and gateway response. */
  async test(): Promise<JsonObject> {
    return { bot: await this.request("GET", "/users/@me"), gateway: await this.request("GET", "/gateway/bot") };
  }

  /** Uploads one of QQ's four media types without consuming a message quota. */
  async uploadMedia(scene: "c2c" | "group", target: string, media: OutgoingMedia): Promise<JsonObject> {
    if (scene !== "c2c" && scene !== "group") throw new Error("Media upload requires a C2C or group scene");
    nonblank(target, "QQ target openid");
    const fileTypes = { image: 1, video: 2, audio: 3, file: 4 } as const;
    if (!Object.prototype.hasOwnProperty.call(fileTypes, media.kind)) throw new Error("Unsupported QQ media kind");
    const source = nonblank(media.source, "media source");
    const body: JsonObject = { file_type: fileTypes[media.kind], srv_send_msg: false };
    if (media.fileName !== undefined) body.file_name = nonblank(media.fileName, "file_name");
    if (/^https?:\/\//i.test(source)) body.url = source;
    else {
      if (/^[a-z][a-z\d+.-]*:\/\//i.test(source) && !/^file:\/\//i.test(source)) throw new Error("Media URL must use HTTP(S)");
      const path = /^file:\/\//i.test(source) ? decodeURIComponent(source.slice(7)) : source;
      const file = await Tools.Files.readBinary(path);
      integer(file.size, "QQ media size (direct upload maximum 5 MiB)", 1, 5 * 1024 * 1024);
      body.file_name = media.fileName === undefined ? nonblank(path.split(/[\\/]/).pop(), "file_name") : body.file_name;
      body.file_data = nonblank(file.contentBase64, "media binary data");
    }
    const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/files` : `/v2/groups/${encodeURIComponent(target)}/files`;
    const uploaded = await this.request("POST", path, body);
    nonblank(uploaded.file_info, "uploaded media file_info");
    return uploaded;
  }

  /** Compatibility wrapper for existing image callers. */
  async uploadImage(scene: "c2c" | "group", target: string, source: string): Promise<JsonObject> {
    return this.uploadMedia(scene, target, { kind: "image", source });
  }

  /** Uploads then sends media using the same sequencing as adjacent text messages. */
  async sendMedia(scene: "c2c" | "group", target: string, media: OutgoingMedia, params: SendParams = {}): Promise<JsonObject> {
    // Validate the complete message before uploading bytes or contacting QQ.
    this.messageBody({ ...params, msg_type: 7, media: { file_info: "pending-upload" } });
    const uploaded = await this.uploadMedia(scene, target, media);
    return this.send(scene, target, { ...params, msg_type: 7, media: { file_info: nonblank(uploaded.file_info, "uploaded media file_info") } });
  }

  /** Compatibility wrapper for Waifu's image segments. */
  async sendImage(scene: "c2c" | "group", target: string, source: string, messageId: string): Promise<JsonObject> {
    return this.sendMedia(scene, target, { kind: "image", source }, { msg_id: messageId });
  }

  /** Validates explicit rich attributes and strips internal envelopes only from visible fields. */
  private messageBody(params: SendParams): JsonObject {
    if (params.msg_seq !== undefined) integer(params.msg_seq, "msg_seq", 1, Number.MAX_SAFE_INTEGER);
    const body: JsonObject = params.payload === undefined ? {} : { ...object(params.payload, "payload") };
    for (const field of ["markdown", "keyboard", "card", "media", "message_reference"] as const) {
      if (params[field] !== undefined) body[field] = object(params[field], field);
      if (body[field] !== undefined) body[field] = object(body[field], field);
    }
    if (params.content !== undefined) body.content = text(params.content, "content");
    if (params.msg_type !== undefined) body.msg_type = params.msg_type;
    if (body.msg_type === undefined) body.msg_type = body.media !== undefined ? 7 : body.markdown !== undefined ? 2 : body.card !== undefined ? 8 : 0;
    integer(body.msg_type, "msg_type", 0, 8);
    for (const field of ["msg_id", "event_id"] as const) {
      if (params[field] !== undefined) body[field] = nonblank(params[field], field);
      if (body[field] !== undefined) nonblank(body[field], field);
    }
    if (body.content !== undefined) body.content = externalText(text(body.content, "content"));
    if (body.msg_type === 0) nonblank(body.content, "text message content");
    for (const [field, type] of [["markdown", 2], ["media", 7], ["card", 8]] as const) {
      if (body[field] !== undefined && body.msg_type !== type) throw new Error(`${field} cannot be used with msg_type=${body.msg_type}`);
    }
    if (body.msg_type === 2) {
      const markdown = object(body.markdown, "markdown");
      if (markdown.content !== undefined) nonblank(externalText(text(markdown.content, "markdown.content")), "markdown.content");
      if (body.content !== undefined && body.content !== "") throw new Error("Markdown messages cannot also contain text content");
      delete body.content;
    }
    if (body.msg_type === 7) nonblank(object(body.media, "media").file_info, "media.file_info");
    if (body.msg_type === 8) object(body.card, "card");
    if (body.message_reference !== undefined) {
      const reference = object(body.message_reference, "message_reference");
      body.message_reference = { message_id: nonblank(reference.message_id, "message_reference.message_id") };
    }
    const visibleKeys = new Set(["content", "label", "visited_label", "title", "description", "desc", "prompt"]);
    const visible = (value: unknown, key = ""): unknown => {
      if (typeof value === "string") return visibleKeys.has(key) ? externalText(value) : value;
      if (Array.isArray(value)) return value.map(item => visible(item, key));
      if (typeof value !== "object" || value === null) return value;
      return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, visible(item, key)]));
    };
    for (const field of ["markdown", "keyboard", "card"] as const) if (body[field] !== undefined) body[field] = visible(body[field]);
    return body;
  }

  /** Builds one authoritative message body and keeps reply sequences monotonic per source message. */
  async send(scene: "c2c" | "group", target: string, params: SendParams): Promise<JsonObject> {
    nonblank(target, "QQ target openid");
    if (scene !== "c2c" && scene !== "group") throw new Error("Sending requires a C2C or group scene");
    const body = this.messageBody(params);
    const source = body.msg_id === undefined ? "" : nonblank(body.msg_id, "msg_id");
    const key = JSON.stringify([scene, target, source || body.event_id || ""]);
    const previous = this.sequences.get(key);
    const seq = params.msg_seq === undefined ? (previous === undefined ? 1 : previous + 1) : integer(params.msg_seq, "msg_seq", 1, Number.MAX_SAFE_INTEGER);
    body.msg_seq = seq;
    this.sequences.set(key, seq);
    if (this.sequences.size > 512) this.sequences.delete(this.sequences.keys().next().value!);
    const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/messages` : `/v2/groups/${encodeURIComponent(target)}/messages`;
    return this.request("POST", path, body);
  }
}
