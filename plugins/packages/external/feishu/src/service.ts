import type { ChannelRequest, ChannelResponses, ChannelConversation, ChannelMessage, ChannelStatus, ActiveContext } from "./channel-api.js";
import { ChannelStore, conversationKey } from "./channel-store.js";
import { incoming, type Incoming } from "./message.js";
import { FeishuApi, required } from "./openapi.js";
import { Fragments, readFrame, writeFrame, utf8, string } from "./wire.js";

interface Config { appId:string; appSecret:string; enabled:boolean; autoReply:boolean; groupAutoReply?:boolean }
type Result = ChannelResponses[ChannelRequest["action"]];
const defaults:Config = {appId:"",appSecret:"",enabled:false,autoReply:false,groupAutoReply:false};
export const IPC = "feishu.connector";

export class FeishuService {
  private readonly store = new ChannelStore();
  private api:FeishuApi;
  private state:ChannelStatus["service"] = "stopped";
  private error = "";
  private generation = 0;
  private readonly policy = {c2c:0,group:0};
  private botId = "";
  private groupIssue = "";
  private handle:string|null = null;
  private timer:number|null = null;
  private readonly openings = new Map<string,Promise<ChannelConversation>>();
  private readonly pending:Incoming[] = [];
  private readonly active = new Map<string,ActiveContext>();
  private readonly workers = new Map<string,Promise<void>>();
  constructor(private readonly config:Config) { this.api=this.createApi(config.appId,config.appSecret); }
  private createApi(appId: string, secret: string): FeishuApi { return new FeishuApi(appId, secret, this.store.recordSend("feishu")); }
  status():ChannelStatus { return {channel:"feishu",capture:true,conversationIds:null,channelOnlyWhenNoAuto:false,service:this.state,
    configured:!!this.config.appId,listenerEnabled:this.config.enabled,autoReplyEnabled:this.config.autoReply,error:this.error,
    groupAutoReplyEnabled:this.config.groupAutoReply === true, ...(this.groupIssue ? {groupAutoReplyIssue:this.groupIssue} : {})}; }
  private canReply(message: Incoming): boolean {
    return message.scene === "c2c" ? this.config.autoReply : this.config.groupAutoReply === true && !!this.botId && message.mentionIds.includes(this.botId);
  }
  private async disableReply(scene: Incoming["scene"]): Promise<void> {
    this.policy[scene]++;
    for(let index=this.pending.length-1;index>=0;index--) if(this.pending[index]?.scene===scene)this.pending.splice(index,1);
    await Promise.allSettled([...this.active.values()].filter(item=>item.scene===scene).map(item=>Tools.Chat.cancel(item.chatId)));
  }
  private schedule(generation:number, ms:number, work:()=>Promise<void>):void {
    if (generation !== this.generation) return;
    this.timer=setTimeout(()=>{this.timer=null;if(generation===this.generation) void work().catch(error=>this.failed(generation,error));},ms);
  }
  private async close():Promise<void> {
    const handle=this.handle;this.handle=null;
    if(handle!==null) await Tools.Net.webSocket({op:"close",handle});
  }
  private async failed(generation:number,error:unknown):Promise<void> {
    if(generation!==this.generation)return;
    this.state="error";this.error=this.api.error(error);
    await this.close().catch(()=>{});
    this.schedule(generation,10000,()=>this.connect(generation));
  }
  async stop():Promise<void> {
    ++this.generation;
    if(this.timer!==null){clearTimeout(this.timer);this.timer=null;}
    this.pending.splice(0); this.state="stopped"; this.error="";
    await Promise.allSettled([this.close(),...Array.from(this.active.keys(),chatId=>Tools.Chat.cancel(chatId))]);
  }
  start():void {
    if(this.state==="connecting"||this.state==="ready"||this.timer!==null)return;
    this.state="connecting";this.error="";
    const generation=++this.generation;
    this.schedule(generation,0,()=>this.connect(generation));
  }
  private async connect(generation:number):Promise<void> {
    this.state="connecting";
    const endpoint=await this.api.endpoint();
    if(generation!==this.generation)return;
    if(this.config.groupAutoReply && !this.botId) {
      const api=this.api, policy=this.policy.group;
      try {
        const id=await api.botIdentity();
        if(generation===this.generation && policy===this.policy.group){this.botId=id;this.groupIssue="";}
      } catch {
        if(generation===this.generation && policy===this.policy.group)this.groupIssue="无法确认机器人身份，群聊回复已暂停";
      }
      if(generation!==this.generation)return;
    }
    const socket=await Tools.Net.webSocket({op:"open_binary",url:endpoint.url,connect_timeout_seconds:15});
    if(socket.op!=="open")throw Error("飞书没有返回长连接句柄");
    if(generation!==this.generation){await Tools.Net.webSocket({op:"close",handle:socket.handle});return;}
    const handle=socket.handle; this.handle=handle;
    const fragments=new Fragments(); let nextPing=0, interval=endpoint.interval, lastActivity=Date.now(), opened=false;
    const send=(base64:string)=>Tools.Net.webSocket({op:"send_binary",handle,base64});
    const tick=async():Promise<void>=>{
      const records=await Tools.Net.webSocket({op:"read",handle});
      if(generation!==this.generation)return;
      if(records.op!=="read")throw Error("飞书长连接读取失败");
      for(const event of records.events){
        if(event.type==="error"||event.type==="closed") throw Error("飞书长连接已断开，正在重连");
        if(event.type==="opened"){opened=true;lastActivity=Date.now();this.state="ready";this.error="";}
        if(event.type!=="binary")continue;
        lastActivity=Date.now();
        const frame=readFrame(event.base64);
        if(frame.method===0&&frame.headers.type==="pong"&&frame.payload.length){
          const data=JSON.parse(string(frame.payload));
          if(typeof data.PingInterval==="number"&&data.PingInterval>0) interval=Math.min(120,Math.max(5,data.PingInterval))*1000;
        }
        if(frame.method!==1||frame.headers.type!=="event")continue;
        const payload=fragments.join(frame); if(!payload)continue;
        const begin=Date.now(); let code=200;
        try {
          const message=incoming(JSON.parse(string(payload)),this.config.appId);
          if(message)await this.record(message,generation);
        } catch(error) { code=500;this.error=this.api.error(error); }
        if(generation!==this.generation)return;
        await send(writeFrame({...frame,headers:{...frame.headers,biz_rt:String(Date.now()-begin)},payload:utf8(JSON.stringify({code}))}));
      }
      if(Date.now()-lastActivity>(opened?2*interval+5000:15000)) throw Error("飞书长连接心跳超时，正在重连");
      if(opened&&Date.now()>=nextPing){
        await send(writeFrame({service:endpoint.service,seq:0n,log:0n,method:0,headers:{type:"ping"},payload:new Uint8Array()}));
        nextPing=Date.now()+interval;
      }
      this.schedule(generation,250,tick);
    };
    this.schedule(generation,0,tick);
  }
  private async record(input:Incoming,generation:number):Promise<void> {
    const accountId=this.config.appId, key=JSON.stringify(["feishu",accountId,input.chatId,input.id]);
    if(await this.store.replyRoute(key))return;
    if(generation!==this.generation)return;
    const autoReplyEnabled=this.canReply(input)&&this.pending.length<100;
    const policy=this.policy[input.scene];
    const message:ChannelMessage={key,channel:"feishu",accountId,messageId:input.id,conversationId:input.chatId,
      authorId:input.authorId,text:input.text,timestamp:input.timestamp,autoReplyEnabled,
      context:{scene:input.scene,resources:input.resources,messageType:input.messageType,...(input.parentId?{parentId:input.parentId}:{})}};
    const label=input.scene==="group"?"飞书群聊":"飞书私聊";
    await this.store.observe(message,input.scene,`${label} · ${input.chatId.slice(-6)}`);
    if(generation!==this.generation || policy!==this.policy[input.scene])return;
    if(autoReplyEnabled){
      this.pending.push(input);this.drain();
    }else if(this.canReply(input)){
      this.error="飞书自动回复积压过多；消息已保留，可手动处理。";
    }
  }
  private drain():void {
    const generation=this.generation;
    while(this.workers.size<3){
      const index=this.pending.findIndex(message=>!this.workers.has(message.chatId)); if(index<0)return;
      const [message]=this.pending.splice(index,1);
      const policy=this.policy[message.scene];
      const current=()=>generation===this.generation&&policy===this.policy[message.scene]&&this.canReply(message);
      const task=this.respond(message,current).catch(error=>{if(current())this.error=this.api.error(error);}).finally(()=>{
        this.workers.delete(message.chatId);this.drain();
      });
      this.workers.set(message.chatId,task);
    }
  }
  private async respond(message:Incoming,current:()=>boolean):Promise<void> {
    if(!current())return;
    const api=this.api;
    const attachments=[], missing=[];
    for(const resource of message.resources) {
      if(!current())return;
      try{attachments.push(await api.media(resource,message.id));}catch{missing.push(resource.name);}
    }
    if(!current())return;
    const session=await this.openChat(conversationKey("feishu",api.appId,message.scene,message.chatId));
    if(!current()||!session.chatId)return;
    const chatId=session.chatId;
    this.active.set(chatId,{chatId,channel:"feishu",scene:message.scene,accountId:api.appId,conversationId:message.chatId,authorId:message.authorId});
    let timedOut=false;
    const timeout=setTimeout(()=>{timedOut=true;void Tools.Chat.cancel(chatId).catch(()=>{});},120000);
    try{
      let answer="";
      for await(const item of Tools.Chat.sendMessageStreaming({kind:"submit",chatId,runtime:"main",input:{
        text:`${message.text}${missing.length?`\n[${missing.length} 个附件不可用]`:""}\n\n请直接回复飞书用户，不要输出内部信息。`,attachments,replyToMessageTimestamp:null},turn:{kind:"execute"},notifyReply:false})){
        if(!current())return;
        if(timedOut)throw Error("飞书自动回复已超时");
        if(item.type==="completed"&&item.result.outcome.type==="committed"&&item.result.outcome.status==="completed")answer=item.result.aiResponse??"";
      }
      answer=answer.replace(/\[\[(?:reply_?to|reply_?to_?current)(?::[^\]\r\n]*)?\]\]|<\/?reply(?:_?to)?\b[^>]*>/gi,"").trim();
      const chars=Array.from(answer);
      if(!chars.length && current())throw Error("飞书自动回复没有产生可发送的文本");
      for(let i=0;i<chars.length;i+=3000){
        if(!current())return;
        if(timedOut)throw Error("飞书自动回复已超时");
        await api.send({messageId:message.id,thread:message.scene==="group"},{text:chars.slice(i,i+3000).join("")},session,()=>current()&&!timedOut);
      }
    }finally{clearTimeout(timeout);this.active.delete(chatId);}
  }
  private async session(key:string):Promise<ChannelConversation> {
    const result=await this.store.get(key);
    if(result.accountId!==this.config.appId)throw Error("飞书会话不属于当前应用");
    return result;
  }
  private openChat(key:string):Promise<ChannelConversation> {
    const existing=this.openings.get(key);if(existing)return existing;
    const task=(async()=>{
      const session=await this.session(key);if(session.chatId)return session;
      const {chatId}=await Tools.Chat.createNew({setAsCurrentChat:false});
      const bound=await this.store.bind(key,chatId);
      await Tools.Chat.updateTitle(chatId,session.title);
      return bound;
    })();
    this.openings.set(key,task);void task.finally(()=>this.openings.delete(key)).catch(()=>{});return task;
  }
  async request(request:ChannelRequest):Promise<Result> {
    switch(request.action){
      case "status":return this.status();
      case "conversations":return {conversations:await this.store.conversations("feishu",this.config.appId)};
      case "open_chat":return this.openChat(request.conversationKey);
      case "history":await this.session(request.conversationKey);return this.store.history(request.conversationKey,request.limit,request.before,undefined,true);
      case "active_context":return this.active.get(request.chatId)??null;
      case "configure":return {capture:true,conversationIds:null,channelOnlyWhenNoAuto:false};
      case "subscribe":return this.store.subscribe(request.consumerId,request.routes,"feishu",this.config.appId);
      case "peek":return {messages:(await this.store.peek(request.consumerId??"connector",request.limit,request.conversationKey)).filter(message=>message.accountId===this.config.appId)};
      case "ack":return {acknowledged:await this.store.ack(request.consumerId??"connector",request.key)};
      case "attachment": {
        const api = this.api;
        const session = await this.session(request.conversationKey);
        const resource = await this.store.attachment(session.key,request.messageId,request.index);
        if (api !== this.api) throw Error("飞书应用已切换");
        const bytes=await api.download(resource,request.messageId,8 * 1024 * 1024);
        if (api !== this.api) throw Error("飞书应用已切换");
        return bytes;
      }
      case "reply":{
        const route=await this.store.replyRoute(request.key);
        if(!route||route.accountId!==this.config.appId)throw Error("飞书回复目标不存在或不属于当前应用");
        return {response:await this.api.send({messageId:route.messageId},request,{conversationId:route.conversationId,scene:route.context.scene === "group" ? "group" : "c2c"})};
      }
      case "send":{
        const session=await this.session(request.conversationKey);
        return {response:await this.api.send({conversationId:session.conversationId},request,session)};
      }
      case "manage":{
        if(request.command==="connect"){
          const appId=required(request.appId,"App ID"), secret=required(request.appSecret,"App Secret");
          const next=this.createApi(appId,secret);
          try{await next.access();await next.endpoint();}catch(error){throw Error(next.error(error));}
          const sameApp=appId===this.config.appId;
          await this.stop(); this.api=next;this.botId="";this.groupIssue="";
          Object.assign(this.config,{appId,appSecret:secret,enabled:true,...(sameApp ? {} : {autoReply:false,groupAutoReply:false})});
          await PluginConfig.flush(this.config);this.start();return this.status();
        }
        if(!this.config.appId)throw Error("请先连接飞书应用");
        if(request.command==="group_auto_reply"){
          if(typeof request.enabled!=="boolean")throw Error("无效的飞书群聊回复设置");
          if(request.enabled)this.botId=await this.api.botIdentity();
          this.config.groupAutoReply=request.enabled; this.groupIssue="";
          if(!request.enabled)await this.disableReply("group");
          await PluginConfig.flush(this.config);
          return this.status();
        }
        if(request.command==="test"){
          await this.api.access();await this.api.endpoint();
          if(this.config.groupAutoReply){this.botId=await this.api.botIdentity();this.groupIssue="";}
          return this.status();
        }
        if(!["start","stop"].includes(request.command))throw Error("Unknown Feishu management command");
        const privateDisabled=request.autoReply===false&&this.config.autoReply;
        if(request.autoReply!==undefined){if(typeof request.autoReply!=="boolean")throw Error("Invalid auto reply flag");this.config.autoReply=request.autoReply;}
        if(privateDisabled)await this.disableReply("c2c");
        this.config.enabled=request.command==="start";
        await PluginConfig.flush(this.config);
        if(this.config.enabled)this.start();else await this.stop();
        return this.status();
      }
      default:throw Error("Unknown Feishu channel action");
    }
  }
  restore():void { if(this.config.enabled&&this.config.appId)this.start(); }
  diagnostic(error:unknown):string {return this.api.error(error);}
}

let service:Promise<FeishuService>|null=null, tail:Promise<unknown>=Promise.resolve();
function owner():Promise<FeishuService>{return service??=PluginConfig.use("feishu",{...defaults}).then(config=>new FeishuService(config)).catch(error=>{service=null;throw error;});}
export function request(payload:ChannelRequest):Promise<Result>{
  const task=tail.catch(()=>{}).then(async()=>{const service=await owner();try{return await service.request(payload);}catch(error){throw Error(service.diagnostic(error));}});
  tail=task;return task;
}
export async function restore():Promise<void>{(await owner()).restore();}
export async function terminate():Promise<void>{if(service)await (await service).stop();}
