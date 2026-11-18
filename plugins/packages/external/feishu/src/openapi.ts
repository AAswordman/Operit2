import type { Chat } from "../../../../types/chat";
import type { RecordSend } from "./channel-store.js";

export type Json = Record<string, unknown>;
export function object(value: unknown): Json {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw Error("飞书返回了无效的数据对象");
  return value as Json;
}
export function required(value: unknown, field: string): string {
  if (typeof value !== "string" || !value.trim()) throw Error(`缺少 ${field}`);
  return value.trim();
}
export type Media = { kind: "image" | "audio" | "video" | "file"; source: string; fileName?: string; durationMs?: number; cover?: string };
export interface Resource { key: string; kind: Media["kind"]; name: string }
type Target = { messageId:string; thread?: boolean } | { conversationId:string };
type SendRoute = { conversationId:string; scene:"c2c" | "group" };
class FeishuSendError extends Error {
  constructor(message: string, readonly rejected: boolean) { super(message); }
}
const ROOT = "https://open.feishu.cn";
const MEDIA_LIMIT = 25 * 1024 * 1024;
const extension = (value: string) => value.split(".").pop()?.toLowerCase() ?? "";
const mimes: Record<string, string> = { jpg:"image/jpeg",jpeg:"image/jpeg",png:"image/png",gif:"image/gif",webp:"image/webp",opus:"audio/opus",ogg:"audio/ogg",wav:"audio/wav",mp3:"audio/mpeg",mp4:"video/mp4",pdf:"application/pdf",txt:"text/plain",csv:"text/csv" };

export class FeishuApi {
  private token: { value: string; expires: number } | null = null;
  private loading: Promise<string> | null = null;
  private identity: Promise<string> | null = null;
  constructor(readonly appId: string, private readonly secret: string, private readonly record?: RecordSend) {}

  private response(response: { statusCode: number; content: string }): Json {
    if (response.statusCode < 200 || response.statusCode >= 300) throw new FeishuSendError(`飞书 HTTP ${response.statusCode}`,
      response.statusCode >= 400 && response.statusCode < 500 && response.statusCode !== 408);
    const data = object(JSON.parse(response.content));
    if (data.code !== 0) throw new FeishuSendError(`飞书接口 ${Number(data.code)}：${typeof data.msg === "string" ? data.msg.slice(0, 300) : "请求失败"}`,
      typeof data.code === "number" && Number.isSafeInteger(data.code));
    return data;
  }
  private async post(path: string, body: Json, token?: string): Promise<Json> {
    const headers: Record<string,string> = { "Content-Type":"application/json", locale:"zh" };
    if (token) headers.Authorization = `Bearer ${token}`;
    return this.response(await Tools.Net.http({ url: ROOT + path, method:"POST", headers, body, follow_redirects:false, connect_timeout:15, read_timeout:20 }));
  }
  async access(): Promise<string> {
    if (this.token && this.token.expires > Date.now()) return this.token.value;
    if (this.loading) return this.loading;
    const pending = this.post("/open-apis/auth/v3/tenant_access_token/internal", { app_id:this.appId, app_secret:this.secret }).then(data => {
      const value = required(data.tenant_access_token, "tenant_access_token"), seconds = Number(data.expire);
      if (!Number.isFinite(seconds) || seconds < 1) throw Error("飞书凭据有效期不正确");
      this.token = { value, expires: Date.now() + Math.max(0, seconds - 60) * 1000 }; return value;
    });
    this.loading = pending;
    try { return await pending; } finally { if (this.loading === pending) this.loading = null; }
  }
  botIdentity(): Promise<string> {
    return this.identity ??= (async () => {
      const token = await this.access();
      const result = this.response(await Tools.Net.http({url:`${ROOT}/open-apis/bot/v3/info`,method:"GET",
        headers:{Authorization:`Bearer ${token}`},follow_redirects:false,connect_timeout:15,read_timeout:20}));
      const id = required(object(result.bot).open_id,"机器人身份");
      if (!/^ou_[a-zA-Z0-9_-]{1,128}$/.test(id)) throw Error("飞书未返回有效的机器人身份");
      return id;
    })().catch(error => { this.identity = null; throw error; });
  }
  async endpoint(): Promise<{ url:string; service:number; interval:number }> {
    const data = object((await this.post("/callback/ws/endpoint", { AppID:this.appId, AppSecret:this.secret })).data);
    const url = required(data.URL,"飞书长连接地址");
    if (!/^wss:\/\/[a-z\d.-]+(?::443)?\//i.test(url)) throw Error("飞书没有返回安全的长连接地址");
    const service = Number(/[?&]service_id=(\d+)(?:&|$)/.exec(url)?.[1]);
    if (!Number.isInteger(service) || service < 0 || service > 2147483647) throw Error("飞书没有返回长连接服务 ID");
    const seconds = Number(object(data.ClientConfig).PingInterval);
    return { url, service, interval:Number.isFinite(seconds) ? Math.min(120, Math.max(5, seconds)) * 1000 : 30000 };
  }
  async media(resource: Resource, messageId: string): Promise<Chat.SendAttachment> {
    const { contentBase64: data, mimeType, fileName } = await this.download(resource, messageId, MEDIA_LIMIT);
    const dir = `${getPluginConfigDir()}/incoming`;
    const made = await Tools.Files.mkdir(dir,true); if (!made.successful) throw Error(made.details);
    const filePath = `${dir}/${CryptoJS.binary.randomHex(16)}`;
    const saved = await Tools.Files.writeBinary(filePath,data); if (!saved.successful) throw Error(saved.details);
    const fileSize = Math.floor(data.length * 3 / 4) - (data.endsWith("==") ? 2 : data.endsWith("=") ? 1 : 0);
    return { filePath, fileName, fileSize, mimeType, content:filePath };
  }
  async download(resource: Resource, messageId: string, limit: number) {
    const response = await Tools.Net.http({
      url:`${ROOT}/open-apis/im/v1/messages/${encodeURIComponent(messageId)}/resources/${encodeURIComponent(resource.key)}?type=${resource.kind === "image" ? "image" : "file"}`,
      method:"GET", headers:{ Authorization:`Bearer ${await this.access()}` }, responseType:"arraybuffer", follow_redirects:false, read_timeout:45,
    });
    const data = required(response.contentBase64, "飞书媒体内容");
    if (response.statusCode !== 200) throw Error(`飞书下载 HTTP ${response.statusCode}`);
    if (data.length * 3 / 4 > limit + 2) throw Error(`飞书附件超过 ${limit / (1024 * 1024)} MiB`);
    const headers = Object.fromEntries(Object.entries(response.headers).map(([k,v])=>[k.toLowerCase(),v]));
    const reported = headers["content-type"]?.split(";")[0].trim();
    if (reported === "application/json" && extension(resource.name) !== "json") throw Error("飞书拒绝下载此附件");
    const mimeType = reported && reported !== "application/octet-stream" ? reported : (mimes[extension(resource.name)] ?? "application/octet-stream");
    return { fileName:resource.name, mimeType, contentBase64:data };
  }
  async upload(media: Media): Promise<Json> {
    const source = required(media.source,"媒体本地路径");
    if (!/^(?:\/|[a-z]:[\\/])/i.test(source)) throw Error("飞书媒体需要设备上的绝对文件路径");
    if (!["image","audio","video","file"].includes(media.kind)) throw Error("未知的飞书媒体类型");
    const name = required(media.fileName ?? source.split(/[\\/]/).pop(),"文件名"), ext = extension(name);
    const info = await Tools.Files.info(source), size = info.size;
    if (typeof size !== "number" || size < 1 || size > (media.kind === "image" ? 10 * 1024 * 1024 : MEDIA_LIMIT)) throw Error("媒体文件不在可上传大小范围内");
    const image = media.kind === "image";
    const fileType = media.kind === "audio" ? "opus" : media.kind === "video" ? "mp4" : ["pdf","doc","xls","ppt"].includes(ext) ? ext : "stream";
    if (media.kind === "audio" && !["opus","ogg"].includes(ext)) throw Error("飞书语音需要 Opus 文件，其他格式可作为文件发送");
    if (media.kind === "video" && ext !== "mp4") throw Error("飞书视频需要 MP4 文件，其他格式可作为文件发送");
    const form: Record<string,string> = image ? { image_type:"message" } : { file_type:fileType, file_name:name };
    if (media.durationMs !== undefined) {
      if (!Number.isSafeInteger(media.durationMs) || media.durationMs < 1) throw Error("媒体时长必须是正整数毫秒数");
      form.duration = String(media.durationMs);
    }
    const cover = media.kind === "video" && media.cover ? await this.upload({kind:"image",source:media.cover}) : null;
    const response = await Tools.Net.uploadFile({ url:`${ROOT}/open-apis/im/v1/${image ? "images" : "files"}`,
      headers:{Authorization:`Bearer ${await this.access()}`},form_data:form,files:[{field_name:image ? "image" : "file",file_path:source,file_name:name}] });
    const data = object(this.response(response).data), key = image ? "image_key" : "file_key";
    return { [key]:required(data[key],key), ...(cover ?? {}) };
  }
  async send(target: Target, params: { text?:string; media?:Media; post?:Json; card?:Json }, route?: SendRoute, canSend?:()=>boolean): Promise<Json> {
    const text = params.text?.trim() ?? "";
    if ([!!text, !!params.media, !!params.post, !!params.card].filter(Boolean).length !== 1) throw Error("请选择一种发送内容：文本、媒体、富文本或卡片");
    const type = params.media ? (params.media.kind === "video" ? "media" : params.media.kind) : params.post ? "post" : params.card ? "interactive" : "text";
    const content = params.media ? await this.upload(params.media) : params.post ?? params.card ?? {text};
    const serialized=JSON.stringify(content);
    if (serialized.length > 16000) throw Error("飞书消息内容过长，请分段发送");
    const body = { msg_type:type, content:serialized, uuid:CryptoJS.binary.randomHex(16) };
    const token = await this.access();
    if(canSend && !canSend())throw Error("飞书自动回复已停止或超时");
    const summary = { text: text || (params.post ? "[富文本]" : params.card ? "[卡片]" : ""),
      ...(params.media ? { media: [{ kind: params.media.kind, name: params.media.fileName }] } : {}) };
    let result: Json;
    try {
      const response = "messageId" in target ? await this.post(`/open-apis/im/v1/messages/${encodeURIComponent(target.messageId)}/reply`,target.thread ? {...body,reply_in_thread:true} : body,token)
        : await this.post("/open-apis/im/v1/messages?receive_id_type=chat_id",{...body,receive_id:target.conversationId},token);
      result = object(response.data);
    } catch (error) {
      if (route) await this.record?.(this.appId, route.scene, route.conversationId, summary, error instanceof FeishuSendError && error.rejected ? "rejected" : "unknown");
      throw error;
    }
    if (route) await this.record?.(this.appId, route.scene, route.conversationId, summary, "accepted");
    return result;
  }
  error(error:unknown): string {
    let value = error instanceof Error ? error.message : String(error);
    for (const secret of [this.secret, this.token?.value]) if (secret) value = value.split(secret).join("[已隐藏]");
    return value.replace(/(?:https|wss):\/\/[^\s"<>]+/g,"[连接地址]").slice(0,400);
  }
}
