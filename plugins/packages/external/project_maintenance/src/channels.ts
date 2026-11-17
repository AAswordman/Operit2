import type { Storage } from "../../../../types/storage.js";
import { NotificationOutbox, type Notice } from "./channel-notifications.js";
import type { CapturedMessage } from "./model.js";
import type { AvailableChat, ChatBinding, ChannelReport, ChannelSession } from "./api.js";
import { request as qqRequest } from "./dependencies/qqbot/api.js";
import { request as wechatRequest, type ChannelRequest, type ChannelResponse } from "./dependencies/wechat/api.js";
type Channel = "qq" | "wechat";
const CONSUMER="com.operit.project_maintenance";
export interface ChannelConfig {enabled:boolean;projectPath:string;parentChatId:string;conversationId:string;notifySince:number}
interface Port {
  chats(): Promise<AvailableChat[]>;
  binding(chatId:string):Promise<Omit<ChatBinding,"chatId">|null>;
  workspace(chatId:string):Promise<string>;
  resetBinding(chatId:string,path:string):Promise<void>;
  capture(parentChatId:string,projectPath:string,message:CapturedMessage):Promise<unknown>;
  notices(channel:Channel,config:ChannelConfig):Promise<Notice[]>;
  status(scope:{channel:Channel;parentChatId:string;projectPath:string;accountId:string;conversationId:string;authorId:string}):Promise<unknown>;
}
const states=new Set(["ready","connecting","stopped","error"]);
/** Bot routing is authoritative. Maintenance only subscribes enabled software chats. */
export class ChannelCoordinator {
  private timer:number|null=null;
  private readonly busy=new Map<Channel,Promise<void>>();
  private readonly reports=new Map<Channel,ChannelReport>();
  private readonly epochs={qq:0,wechat:0};
  private readonly outbox=new NotificationOutbox();
  private readonly migrated=new Set<Channel>();
  private readonly knownBotChats=new Set<string>();
  constructor(private readonly port:Port){}
  private bot<R extends ChannelRequest>(channel:Channel,payload:R):Promise<ChannelResponse<R>>{
    return channel==="qq"?qqRequest(payload):wechatRequest(payload);
  }
  private async observe(channel:Channel):Promise<ChannelReport>{
    try{
      const [state,result,chats]=await Promise.all([this.bot(channel,{action:"status"}),this.bot(channel,{action:"conversations"}),this.port.chats()]);
      if(!state||!states.has(state.service)||!Array.isArray(result.conversations))throw Error("Bot 会话接口不可用，请更新对应 Bot 插件");
      const sessions:ChannelSession[]=[];
      for(const session of result.conversations){
        if(session.channel!==channel || !session.key || !session.accountId || !session.conversationId || (session.scene!=="c2c"&&session.scene!=="group"))throw Error("Bot 返回了无效的会话绑定");
        const chat=chats.find(chat=>chat.id===session.chatId), binding=session.chatId?await this.port.binding(session.chatId):null;
        for(const id of [session.chatId,...(session.contextChatIds??[])])if(id)this.knownBotChats.add(id);
        sessions.push({...session,title:chat?.title||session.title,workspacePath:chat?.workspacePath??null,enabled:binding?.projectPath!==null&&binding!==null,
          error:session.chatId&&!chat?"软件聊天已不存在，请在 Bot 中检查会话":session.chatId&&!chat?.workspaceId?"请先在软件中为这个聊天绑定工作区":""});
      }
      const report:ChannelReport={channel,availability:"available",service:state.service,enabled:sessions.some(s=>s.enabled),sessions,lastError:this.reports.get(channel)?.lastError??"",observedAt:Date.now()};
      this.reports.set(channel,report);return report;
    }catch(error){
      const previous=this.reports.get(channel);
      const report:ChannelReport={channel,availability:"unavailable",service:"unknown",enabled:previous?.enabled??false,sessions:previous?.sessions??[],lastError:String(error instanceof Error?error.message:error),observedAt:Date.now()};
      this.reports.set(channel,report);return report;
    }
  }
  async status():Promise<ChannelReport[]>{return Promise.all((["qq","wechat"]as Channel[]).map(channel=>this.observe(channel)));}
  async prepare(channel:Channel,key:string):Promise<{chatId:string}>{
    if(channel!=="qq"&&channel!=="wechat")throw Error("Invalid channel");
    const session=await this.bot(channel,{action:"open_chat",conversationKey:key});
    if(!session.chatId||session.channel!==channel)throw Error("Bot 没有返回有效的聊天绑定");
    return {chatId:session.chatId};
  }
  async isBotChat(chatId:string):Promise<boolean>{
    if(this.knownBotChats.has(chatId))return true;
    const stale=([...this.reports.values()].length<2)||[...this.reports.values()].some(report=>Date.now()-report.observedAt>3000);
    if(stale)await this.status();
    if(this.knownBotChats.has(chatId))return true;
    return false;
  }
  /** Removes the old one-channel selector without silently redirecting its work into another chat. */
  private async migrate(channel:Channel):Promise<void>{
    if(this.migrated.has(channel))return;
    const db=await Tools.Storage.objects.open({path:ToolPkg.getLocalDataDir()+"/channel-connections.sqlite"});
    const old=await db.collection<ChannelConfig>("connections").get(channel);
    if(old?.value.enabled){
      const state=await this.bot(channel,{action:"status"});
      if(state.channelOnlyWhenNoAuto&&state.conversationIds?.length===1&&state.conversationIds[0]===old.value.conversationId)
        await this.bot(channel,{action:"configure",capture:false,conversationIds:[],channelOnlyWhenNoAuto:false});
      await db.commit([{op:"put",collection:"connections",key:channel,value:{...old.value,enabled:false} as unknown as Storage.Value,expectedVersion:old.version}]);
    }
    this.migrated.add(channel);
  }
  async sync():Promise<void>{await Promise.all((["qq","wechat"]as Channel[]).map(channel=>this.tick(channel)));}
  tick(channel:Channel):Promise<void>{
    const pending=this.busy.get(channel);if(pending)return pending;
    const task=this.collect(channel);this.busy.set(channel,task);void task.finally(()=>{if(this.busy.get(channel)===task)this.busy.delete(channel)}).catch(()=>{});return task;
  }
  private async collect(channel:Channel):Promise<void>{
    const epoch=this.epochs[channel];
    try{
      const report=await this.observe(channel);if(report.availability!=="available")return;
      await this.migrate(channel);
      const routes:{session:ChannelSession;config:ChannelConfig;since:number}[]=[];
      for(const session of report.sessions){
        if(!session.chatId||!session.enabled)continue;
        try{
          const path=await this.port.workspace(session.chatId);let binding=await this.port.binding(session.chatId);
          if(binding?.projectPath===null||!binding)continue;
          if(binding.projectPath!==path){await this.port.resetBinding(session.chatId,path);binding=await this.port.binding(session.chatId);}
          if(!binding?.since)continue;
          routes.push({session,since:binding.since,config:{enabled:true,projectPath:path,parentChatId:session.chatId,conversationId:session.conversationId,notifySince:binding.since}});
        }catch(error){session.error=String(error instanceof Error?error.message:error);}
      }
      if(epoch!==this.epochs[channel])return;
      await this.bot(channel,{action:"subscribe",consumerId:CONSUMER,routes:routes.map(({session,since})=>({conversationKey:session.key,since}))});
      for(const {session,config,since}of routes){
        const inbox=await this.bot(channel,{action:"peek",consumerId:CONSUMER,conversationKey:session.key,limit:100});
        if(!Array.isArray(inbox.messages))throw Error("Invalid Bot inbox");
        for(const message of inbox.messages){
          if(epoch!==this.epochs[channel])return;
          const current=await this.port.binding(config.parentChatId);
          if(current?.projectPath!==config.projectPath||current.since!==since)break;
          if(await this.port.workspace(config.parentChatId)!==config.projectPath)break;
          if(message.channel!==channel||message.accountId!==session.accountId||message.conversationId!==session.conversationId||message.context.scene!==session.scene&&channel==="qq")throw Error("消息与 Bot 会话来源不一致");
          await this.port.capture(config.parentChatId,config.projectPath,message);
          await this.bot(channel,{action:"ack",consumerId:CONSUMER,key:message.key});
        }
        const current=await this.port.binding(config.parentChatId);
        if(current?.projectPath!==config.projectPath||current.since!==since)continue;
        for(const notice of await this.port.notices(channel,config))
          await this.outbox.send(notice,async()=>{
            const binding=await this.port.binding(config.parentChatId);
            return epoch===this.epochs[channel]&&binding?.projectPath===config.projectPath&&binding.since===since&&await this.port.workspace(config.parentChatId)===config.projectPath;
          },()=>this.bot(channel,{action:"reply",key:notice.messageKey,text:notice.text}));
      }
      report.lastError="";
    }catch(error){const report=this.reports.get(channel);if(report)report.lastError=String(error instanceof Error?error.message:error);}
  }
  async conversationStatus(chatId:string):Promise<unknown|null>{
    for(const channel of ["qq","wechat"]as Channel[]){
      let context;try{context=await this.bot(channel,{action:"active_context",chatId});}catch{continue;}
      if(!context||context.channel!==channel||context.chatId!==chatId)continue;
      const report=await this.observe(channel),session=report.sessions.find(s=>s.enabled&&s.accountId===context.accountId&&s.conversationId===context.conversationId&&(!context.scene||s.scene===context.scene));
      if(!session?.chatId)continue;
      const projectPath=await this.port.workspace(session.chatId);
      return this.port.status({channel,parentChatId:session.chatId,projectPath,accountId:context.accountId,conversationId:context.conversationId,authorId:context.authorId});
    }return null;
  }
  start():void{if(this.timer!==null)return;this.timer=setInterval(()=>void this.sync(),3000);void this.sync();}
  stop():void{this.epochs.qq++;this.epochs.wechat++;if(this.timer!==null)clearInterval(this.timer);this.timer=null;}
}
