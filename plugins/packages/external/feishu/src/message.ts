import { object, required, type Json, type Resource } from "./openapi.js";

export interface Incoming { id:string; chatId:string; authorId:string; scene:"c2c"|"group"; timestamp:string; text:string; messageType:string; parentId?:string; resources:Resource[]; mentionIds:string[] }

export function incoming(value:unknown, appId:string): Incoming | null {
  const data = object(value), header = object(data.header);
  if (header.event_type !== "im.message.receive_v1" || (header.app_id !== undefined && header.app_id !== appId)) return null;
  const event = object(data.event), sender = object(event.sender);
  if (sender.sender_type !== "user") return null;
  const msg = object(event.message), type = required(msg.message_type,"消息类型");
  if (msg.chat_type !== "p2p" && msg.chat_type !== "group") return null;
  const content = object(JSON.parse(required(msg.content,"消息内容"))), resources:Resource[] = [];
  const put = (key:unknown, kind:Resource["kind"], name:unknown, fallback:string) => {
    if (typeof key === "string" && key !== "" && resources.length < 10 && !resources.some(item=>item.key===key)) resources.push({key,kind,name:typeof name === "string" && name ? name : fallback});
  };
  const lines: string[] = [];
  if (typeof content.text === "string") lines.push(content.text);
  if (type === "image") put(content.image_key,"image",null,"image.jpg");
  if (type === "audio") put(content.file_key,"audio",null,"voice.opus");
  if (type === "media") put(content.file_key,"video",content.file_name,"video.mp4");
  if (type === "file") put(content.file_key,"file",content.file_name,"attachment");
  if (type === "sticker") put(content.file_key,"file",null,"sticker");
  if (type === "post") {
    const post = Array.isArray(content.content) ? content : (Object.values(content).find(x=>x && typeof x === "object" && Array.isArray((x as Json).content)) as Json | undefined);
    if (typeof post?.title === "string") lines.push(post.title);
    if (Array.isArray(post?.content)) for (const row of post.content.slice(0,100)) {
      if (!Array.isArray(row)) continue;
      const line: string[] = [];
      for (const value of row.slice(0,100)) {
        if (!value || typeof value !== "object") continue;
        const element=value as Json;
        if (typeof element.text === "string") line.push(element.text);
        if (element.tag === "img") put(element.image_key,"image",null,"image.jpg");
        if (element.tag === "media") put(element.file_key,"video",element.file_name,"video.mp4");
      }
      if (line.length) lines.push(line.join(""));
    }
  }
  const mentions = Array.isArray(msg.mentions) ? msg.mentions.slice(0,100).filter(x=>x && typeof x === "object") as Json[] : [];
  const mentionIds = new Set<string>();
  let text=lines.join("\n");
  for (const mention of mentions) {
    const id = (mention.id as Json | undefined)?.open_id;
    if ((mention.mentioned_type === "bot" || mention.mentioned_type === undefined) && typeof id === "string" && /^ou_[a-zA-Z0-9_-]{1,128}$/.test(id)) mentionIds.add(id);
  }
  const keys = new Set(mentions.map(mention=>mention.key));
  text=text.replace(/@_user_\d+/g, key=>keys.has(key) ? "" : key);
  const timestamp=Number(msg.create_time);
  return { id:required(msg.message_id,"消息 ID"), chatId:required(msg.chat_id,"会话 ID"), authorId:required(object(sender.sender_id).open_id,"发送者 ID"),
    scene:msg.chat_type === "p2p" ? "c2c" : "group", timestamp:Number.isFinite(timestamp) ? String(timestamp) : String(Date.now()),
    text:text.trim().slice(0,12000) || (resources.length ? `[${resources.map(x=>x.name).join("、")}]` : `[飞书 ${type} 消息]`),
    ...(typeof msg.parent_id === "string" && msg.parent_id ? {parentId:msg.parent_id}:{}), resources, messageType:type, mentionIds:[...mentionIds] };
}
