import React from 'react';
import type {ChannelReport,Request} from '../src/api';
import {Icon} from './icons';
export function ChannelSessions({report,busy,onAction}:{report:ChannelReport|undefined;busy:boolean;onAction:(request:Request)=>void}){
  if(!report?.sessions.length)return <div className="channel-empty"><strong>还没有发现会话</strong><p>先在对应 Bot 中连接账号，再从群或私聊发一条消息。这里会自动显示 Bot 会话，无需填写群号或会话 ID。</p></div>;
  return <div className="channel-sessions">{report.sessions.map(session=><article className="channel-session" key={session.key}>
    <div className="session-heading"><span className="session-scene">{session.scene==='group'?'群聊':'私聊'}</span><strong>{session.title}</strong><span className={`status ${session.enabled?'complete':'neutral'}`}>{session.enabled?'维护已开启':'维护未开启'}</span></div>
    <p className="session-workspace"><Icon name="folder" size={14}/>{session.workspacePath||'尚未绑定工作区'}</p>
    <p className="form-help">{!session.chatId?'创建此 Bot 会话对应的软件聊天，然后在软件中绑定待维护的工作区。':!session.workspacePath?'在软件的聊天列表找到此会话，绑定工作区后即可开启维护。':'复用这个 Bot 聊天的模型、工具权限和工作区。工作区须为 Git 仓库根目录，且已完成首次提交；后台任务交给子代理。'}</p>
    {session.error&&<p className="channel-error">{session.error}</p>}
    <div className="channel-actions">{!session.chatId?<button className="button secondary" disabled={busy} onClick={()=>onAction({action:'prepare_channel_chat',channel:session.channel,conversationKey:session.key})}>创建会话聊天</button>:<button className={`button ${session.enabled?'secondary':'primary'}`} disabled={busy||(!session.enabled&&!session.workspacePath)} onClick={()=>onAction({action:'bind_chat',chatId:session.chatId!,enabled:!session.enabled})}>{session.enabled?'停止维护':'开启维护'}</button>}</div>
  </article>)}</div>;
}
