import type { Net } from "../../../../types/network";
import type { Config, JsonObject, SendParams } from "./model.js";
import { integer, nonblank, object } from "./model.js";

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

  /** Uploads an actual image through the official scene-specific QQ media endpoint. */
  async uploadImage(scene: "c2c" | "group", target: string, source: string): Promise<JsonObject> {
    if (scene !== "c2c" && scene !== "group") throw new Error("Image upload requires a C2C or group scene");
    nonblank(target, "QQ target openid");
    nonblank(source, "image source");
    const body: JsonObject = { file_type: 1, srv_send_msg: false };
    if (/^https?:\/\//i.test(source)) body.url = source;
    else {
      const path = /^file:\/\//i.test(source) ? decodeURI(source.slice(7)) : source;
      const image = await Tools.Files.readBinary(path);
      integer(image.size, "QQ image size", 1, 5 * 1024 * 1024);
      body.file_data = nonblank(image.contentBase64, "image binary data");
    }
    const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/files` : `/v2/groups/${encodeURIComponent(target)}/files`;
    const uploaded = await this.request("POST", path, body);
    nonblank(uploaded.file_info, "uploaded image file_info");
    return uploaded;
  }

  /** Sends one uploaded image using the same passive reply sequence as adjacent text segments. */
  async sendImage(scene: "c2c" | "group", target: string, source: string, messageId: string): Promise<JsonObject> {
    const uploaded = await this.uploadImage(scene, target, source);
    const fileInfo = nonblank(uploaded.file_info, "uploaded image file_info");
    return this.send(scene, target, { msg_type: 7, msg_id: messageId, payload: { media: { file_info: fileInfo } } });
  }

  /** Builds one authoritative message body and keeps reply sequences monotonic per source message. */
  async send(scene: "c2c" | "group", target: string, params: SendParams): Promise<JsonObject> {
    nonblank(target, "QQ target openid");
    const body: JsonObject = params.payload === undefined ? {} : { ...object(params.payload, "payload") };
    if (params.content !== undefined) body.content = nonblank(params.content, "content");
    if (params.msg_type !== undefined) body.msg_type = integer(params.msg_type, "msg_type", 0, 7);
    if (body.msg_type === undefined) body.msg_type = 0;
    if (params.msg_id !== undefined) body.msg_id = nonblank(params.msg_id, "msg_id");
    if (params.event_id !== undefined) body.event_id = nonblank(params.event_id, "event_id");
    if (body.msg_type === 0) nonblank(body.content, "text message content");
    const source = body.msg_id === undefined ? "" : nonblank(body.msg_id, "msg_id");
    const key = `${scene}:${target}:${source}`;
    const previous = this.sequences.get(key);
    const seq = params.msg_seq === undefined ? (previous === undefined ? 1 : previous + 1) : integer(params.msg_seq, "msg_seq", 1, Number.MAX_SAFE_INTEGER);
    body.msg_seq = seq;
    this.sequences.set(key, seq);
    if (this.sequences.size > 512) this.sequences.delete(this.sequences.keys().next().value!);
    const path = scene === "c2c" ? `/v2/users/${encodeURIComponent(target)}/messages` : `/v2/groups/${encodeURIComponent(target)}/messages`;
    return this.request("POST", path, body);
  }
}
