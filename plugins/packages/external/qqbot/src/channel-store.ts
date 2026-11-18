import type { Storage } from "../../../../types/storage.js";
import { consoleAttachment } from "./attachments.js";
import type { ChannelConversation, ChannelMessage, ChannelSubscription, ContextMessage, ConversationHistory } from "./channel-api.js";

export function conversationKey(channel: string, accountId: string, scene: string, conversationId: string): string {
  return JSON.stringify([channel, accountId, scene, conversationId]);
}
const value = (data: unknown): Storage.Value => data as Storage.Value;
export type SendSummary = Pick<ContextMessage, "text" | "media">;
export type RecordSend = (accountId: string, scene: "c2c" | "group", target: string, content: SendSummary,
  delivery: NonNullable<ContextMessage["delivery"]>) => Promise<void>;
const consumerKey = (consumerId: string, key: string): string => JSON.stringify([consumerId, key]);
export function consumer(value: string): string {
  if (typeof value !== "string" || !/^[a-zA-Z0-9_.-]{1,100}$/.test(value)) throw Error("Invalid channel consumer ID");
  return value;
}
/** Bounded readable history and independent durable deliveries; no raw payloads or credentials leave this store. */
export class ChannelStore {
  private database: Promise<Storage.ObjectDatabase> | null = null;
  private queue: Promise<unknown> = Promise.resolve();
  private open(): Promise<Storage.ObjectDatabase> {
    return this.database ??= Tools.Storage.objects.open({path: ToolPkg.getLocalDataDir() + "/channel_sessions.sqlite"}).catch(error => {this.database = null; throw error;});
  }
  private serial<T>(work: () => Promise<T>): Promise<T> {
    const next = this.queue.catch(() => {}).then(work); this.queue = next; return next;
  }
  async get(key: string): Promise<ChannelConversation> {
    const found = await (await this.open()).collection<ChannelConversation>("conversations").get(key);
    if (!found) throw Error("Bot 尚未收到这个会话的消息");
    return found.value;
  }
  async conversations(channel: string, accountId: string): Promise<ChannelConversation[]> {
    const collection = (await this.open()).collection<ChannelConversation>("conversations"), result: ChannelConversation[] = [];
    let after: string | null = null;
    do {
      const rows = await collection.list(after, 100);
      result.push(...rows.map(row => row.value).filter(row => row.channel === channel && row.accountId === accountId));
      if (rows.length < 100) break;
      after = rows[rows.length - 1]!.key;
    } while (true);
    return result.sort((a,b) => b.lastMessageAt - a.lastMessageAt);
  }
  remember(conversation: ChannelConversation): Promise<void> {
    return this.serial(async()=>{
      const db=await this.open(), old=await db.collection<ChannelConversation>("conversations").get(conversation.key);
      if(old?.value.chatId===conversation.chatId)return;
      await db.commit([{op:"put",collection:"conversations",key:conversation.key,value:value(old?{...old.value,chatId:conversation.chatId}:conversation),expectedVersion:old?.version??null}]);
    });
  }
  observe(message: ChannelMessage, scene: "c2c" | "group", title: string, chatId?: string): Promise<void> {
    return this.serial(async () => {
      const db = await this.open(), key = conversationKey(message.channel, message.accountId, scene, message.conversationId);
      const row = await db.collection<ChannelConversation>("conversations").get(key);
      const history = await db.collection<ContextMessage[]>("history").get(key);
      const messages = history?.value ?? [];
      if (await db.collection("reply_routes").get(message.key)) return;
      const at = Math.max(Date.now(), (row?.value.lastMessageAt ?? 0) + 1);
      const readable: ContextMessage = {messageId: message.messageId, authorId: message.authorId,
        authorName: message.channel === "qq" && "author" in message ? String((message as {author?: {name: string}}).author?.name ?? "").slice(0,100) : "",
        text: message.text.slice(0,6000), timestamp: message.timestamp, receivedAt: at};
      const attachments = message.attachments?.slice(0,20).map(consoleAttachment) ?? [];
      if (attachments.length) readable.media = message.attachments!.slice(0,20).map((item,index)=>({kind:item.kind,name:item.fileName.slice(0,150),
        ...(attachments[index] ? {downloadable:true} : {})}));
      for (const field of ["referenceIndex", "referencedMessageIndex"] as const) {
        const reference = (message as unknown as Record<string, unknown>)[field];
        if (typeof reference === "string") readable[field] = reference;
      }
      const conversation: ChannelConversation = {key, channel: message.channel, accountId: message.accountId, scene,
        conversationId: message.conversationId, title: title.slice(0,150), chatId: chatId ?? row?.value.chatId ?? null, lastMessageAt: at};
      const mutations: Storage.ObjectMutation[] = [
        {op:"put",collection:"reply_routes",key:message.key,value:value({key:message.key,channel:message.channel,accountId:message.accountId,messageId:message.messageId,
          conversationId:message.conversationId,authorId:"",text:"",timestamp:"",context:{scene},...("referenceIndex" in readable?{referenceIndex:readable.referenceIndex}:{})})},
        {op:"put", collection:"conversations", key, value:value(conversation), expectedVersion:row?.version ?? null},
        {op:"put", collection:"history", key, value:value([...messages,readable].slice(-80)), expectedVersion:history?.version ?? null},
      ];
      if (attachments.some(Boolean)) mutations.push({op:"put",collection:"attachment_routes",key:JSON.stringify([key,message.messageId]),value:value(attachments)});
      if (messages.length >= 80 && messages[0]?.media?.some(item=>item.downloadable)) mutations.push({op:"delete",collection:"attachment_routes",key:JSON.stringify([key,messages[0].messageId])});
      const subscribers: {key:string;value:{routes:ChannelSubscription[]}}[] = [];
      let after: string | null = null;
      do {
        const page = await db.collection<{routes:ChannelSubscription[]}>("subscribers").list(after,100);
        subscribers.push(...page);
        if(page.length<100)break;
        after=page[page.length-1]!.key;
      } while(true);
      for (const subscriber of subscribers) {
        if (!subscriber.value.routes.some(route => route.conversationKey === key && at >= route.since)) continue;
        mutations.push({op:"put",collection:"deliveries",key:consumerKey(subscriber.key,message.key),value:value({conversationKey:key,receivedAt:at,message}),expectedVersion:null});
      }
      await db.commit(mutations);
    });
  }
  bind(key: string, chatId: string | null): Promise<ChannelConversation> {
    return this.serial(async () => {
      const db = await this.open(), row = await db.collection<ChannelConversation>("conversations").get(key);
      if (!row) throw Error("Bot 会话不存在");
      const next = {...row.value,chatId};
      await db.commit([{op:"put",collection:"conversations",key,value:value(next),expectedVersion:row.version}]);
      return next;
    });
  }
  recordSend(channel: ChannelConversation["channel"]): RecordSend {
    return (accountId, scene, conversationId, content, delivery) => this.serial(async () => {
      const db = await this.open(), key = conversationKey(channel, accountId, scene, conversationId);
      const conversations = db.collection<ChannelConversation>("conversations");
      const [conversation, outgoing] = await Promise.all([
        conversations.get(key), db.collection<ContextMessage[]>("outgoing").get(key),
      ]);
      if (!conversation && delivery !== "accepted") return;
      const timestamp = Date.now();
      const receivedAt = Math.max(timestamp, (conversation?.value.lastMessageAt ?? 0) + 1);
      const item: ContextMessage = { direction: "outgoing", delivery, messageId: `local-${receivedAt}`,
        authorName: "机器人", authorId: "", timestamp: String(timestamp), receivedAt, text: content.text.slice(0,6000),
        ...(content.media ? { media: content.media.slice(0,20).map(({kind,name}) => ({ kind, ...(name ? { name: name.split(/[\\/]/).pop()!.slice(0,150) } : {}) })) } : {}) };
      const next: ChannelConversation = { key, channel, accountId, scene, conversationId, title: scene === "group" ? "群聊" : "私聊",
        chatId: null, ...conversation?.value, lastMessageAt: receivedAt };
      await db.commit([
        {op:"put", collection:"outgoing", key, value:value([...(outgoing?.value ?? []),item].slice(-80)), expectedVersion:outgoing?.version ?? null},
        {op:"put", collection:"conversations", key, value:value(next), expectedVersion:conversation?.version ?? null},
      ]);
    }).catch(() => { console.error("[connector] Could not save outbound history"); });
  }
  async history(key: string, limit = 20, before?: number, authorId?: string, includeOutgoing = false): Promise<ConversationHistory> {
    if (!Number.isSafeInteger(limit) || limit < 1 || limit > 50 || (before !== undefined && (!Number.isSafeInteger(before) || before < 0))) throw Error("Invalid history bounds");
    const conversation = await this.get(key);
    const all = (await (await this.open()).collection<ContextMessage[]>("history").get(key))?.value ?? [];
    const outbound = includeOutgoing ? (await (await this.open()).collection<ContextMessage[]>("outgoing").get(key))?.value ?? [] : [];
    const filtered = [...all, ...outbound].filter(item => (before === undefined || item.receivedAt < before) && (authorId === undefined || item.authorId === authorId))
      .sort((a,b) => a.receivedAt - b.receivedAt);
    const messages = filtered.slice(-limit).map(item=>({...item})); let remaining = 12000;
    for (let index=messages.length-1;index>=0;index--) {const item=messages[index]!;const text=item.text.slice(0,remaining);remaining-=text.length;messages[index]={...item,text};}
    return {conversation, messages:messages.filter(item=>item.direction || item.text.length > 0 || item.media?.length),
      truncated:filtered.length > messages.length || (!includeOutgoing && (all.length === 80 || remaining === 0)),
      ...(includeOutgoing ? { latestInboundId: all[all.length - 1]?.messageId } : {})};
  }
  subscribe(consumerId: string, routes: ChannelSubscription[], channel: string, accountId: string): Promise<{subscribed: number}> {
    consumer(consumerId);
    if (!Array.isArray(routes) || routes.length > 200 || new Set(routes.map(route=>route.conversationKey)).size !== routes.length) throw Error("Invalid channel subscriptions");
    return this.serial(async () => {
      for (const route of routes) {
        if (!Number.isSafeInteger(route.since) || route.since < 0) throw Error("Invalid subscription boundary");
        const conversation = await this.get(route.conversationKey);
        if (conversation.channel !== channel || conversation.accountId !== accountId) throw Error("Subscription belongs to a different Bot account");
      }
      const db = await this.open(), row = await db.collection("subscribers").get(consumerId);
      await db.commit([{op:"put",collection:"subscribers",key:consumerId,value:value({routes}),expectedVersion:row?.version ?? null}]);
      let after:string|null=null;
      const prefix=JSON.stringify([consumerId]).slice(0,-1)+",";
      do {
        const rows=await db.collection<{conversationKey:string;receivedAt:number}>("deliveries").list(after,100);
        for(const entry of rows)if(entry.key.startsWith(prefix)&&!routes.some(route=>route.conversationKey===entry.value.conversationKey&&entry.value.receivedAt>=route.since))
          await db.collection("deliveries").remove(entry.key,{expectedVersion:entry.version});
        if(rows.length<100)break;
        after=rows[rows.length-1]!.key;
      }while(true);
      return {subscribed:routes.length};
    });
  }
  async peek(consumerId: string, limit = 20, key?: string): Promise<ChannelMessage[]> {
    consumer(consumerId);
    if (!Number.isSafeInteger(limit) || limit<1 || limit>100) throw Error("Invalid inbox limit");
    const db=await this.open(), result:{message:ChannelMessage;receivedAt:number}[]=[], prefix=JSON.stringify([consumerId]).slice(0,-1)+",";
    const subscription=(await db.collection<{routes:ChannelSubscription[]}>("subscribers").get(consumerId))?.value;
    let after:string|null=null;
    do {
      const rows=await db.collection<{conversationKey:string;receivedAt:number;message:ChannelMessage}>("deliveries").list(after,100);
      for(const row of rows) if(row.key.startsWith(prefix) && (key===undefined || row.value.conversationKey===key) && subscription?.routes.some(route=>route.conversationKey===row.value.conversationKey && row.value.receivedAt>=route.since)) result.push({message:row.value.message,receivedAt:row.value.receivedAt});
      if(rows.length<100)break;
      after=rows[rows.length-1]!.key;
    }while(true);
    return result.sort((a,b)=>a.receivedAt-b.receivedAt).slice(0,limit).map(item=>item.message);
  }
  async replyRoute(key:string):Promise<ChannelMessage|null>{
    return (await (await this.open()).collection<ChannelMessage>("reply_routes").get(key))?.value??null;
  }
  async attachment(key: string, messageId: string, index: number): Promise<NonNullable<ReturnType<typeof consoleAttachment>>> {
    if (typeof messageId !== "string" || !messageId || !Number.isSafeInteger(index) || index < 0 || index >= 20) throw Error("无效的附件编号");
    const entry = await (await this.open()).collection<ReturnType<typeof consoleAttachment>[]>("attachment_routes").get(JSON.stringify([key,messageId]));
    const item = entry?.value[index];
    if (!item) throw Error("附件不存在或已不在近期记录中");
    return item;
  }
  async ack(consumerId:string,key:string):Promise<boolean> {
    consumer(consumerId);
    const collection=(await this.open()).collection("deliveries"), id=consumerKey(consumerId,key), row=await collection.get(id);
    if(!row)return false;
    await collection.remove(id,{expectedVersion:row.version});return true;
  }
}
