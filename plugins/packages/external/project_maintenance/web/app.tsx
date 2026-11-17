import React, {useEffect, useRef, useState} from 'react';
import {createRoot} from 'react-dom/client';
import type {AvailableChat, ChannelReport, Dashboard, Inspection, Stage, Ticket} from '../src/api';
import {request} from './host';
import {ChannelSessions} from './channel-sessions';
import {Icon} from './icons';
import './style.css';

type Page = 'overview'|'feedback'|'tickets'|'channels'|'projects';
const stageNames:Record<Stage,string> = {new:'待诊断',triage:'诊断中',needs_info:'等待补充',not_actionable:'无需修改',implement:'修复中',review:'验收中',delivery:'交付中',completed:'已完成',interrupted:'执行中断',failed:'处理失败'};
const inProgress = (t:Ticket)=>['triage','implement','review','delivery'].includes(t.stage);
const requiresAttention = (t:Ticket)=>['new','needs_info','interrupted','failed'].includes(t.stage);
const channelName = (value:string)=>({app:'软件内',panel:'面板',qq:'QQ',wechat:'微信'}[value]||value);
const projectName = (value:string)=>value.replace(/[/\\]+$/,'').split(/[/\\]/).pop()||value;
const timestamp = (value:number|null)=>value?new Date(value).toLocaleString('zh-CN',{month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit',hour12:false}):'暂无记录';
function elapsed(value:number) {
  if(!value)return '尚未观测';
  const seconds=Math.max(0,Math.floor((Date.now()-value)/1000));
  return seconds<60?'刚刚':seconds<3600?`${Math.floor(seconds/60)} 分钟前`:seconds<86400?`${Math.floor(seconds/3600)} 小时前`:timestamp(value);
}
const noteNames:Record<string,string> = {triage:'反馈诊断',implement:'修复报告',review:'独立验收',delivery:'交付报告',delivery_verification:'Git 交付核验',user:'补充信息',recovery:'恢复检查'};
function formatNote(text:string) {
  try {
    const value:unknown=JSON.parse(text);
    if(value&&typeof value==='object'&&!Array.isArray(value)) return Object.entries(value).map(([key,value])=>`${({decision:'判断',reason:'依据',requirements:'要求',summary:'结果',limitations:'限制',head:'提交',parent:'原提交',pullRequest:'PR'} as Record<string,string>)[key]||key}：${typeof value==='string'?value:JSON.stringify(value)}`).join('\n\n');
  } catch { /* Plain text notes are rendered as text. */ }
  return text;
}
function statusTone(stage:Stage) {return inProgress({stage} as Ticket)?'working':requiresAttention({stage} as Ticket)?'attention':stage==='completed'?'complete':'neutral'}
function Badge({stage}:{stage:Stage}) {return <span className={`status ${statusTone(stage)}`}><i/>{stageNames[stage]}</span>}
function Empty({title,children}:{title:string;children:React.ReactNode}) {return <div className="empty-state"><Icon name="inbox" size={26}/><h3>{title}</h3><p>{children}</p></div>}
function Section({title,description,extra,children,className=''}:React.PropsWithChildren<{title:string;description?:string;extra?:React.ReactNode;className?:string}>) {
  return <section className={`section ${className}`}><div className="section-heading"><div><h2>{title}</h2>{description&&<p>{description}</p>}</div>{extra}</div>{children}</section>;
}
function ChannelMark({channel}:{channel:'qq'|'wechat'|'app'}) {
  return <span className={`channel-mark ${channel}`}>{channel==='qq'?<svg viewBox="0 0 24 24" width="21" height="21" fill="currentColor" aria-hidden="true"><path d="M12 3c-3.1 0-4.6 2.7-4.6 6.1-1.2 1.3-2.1 3.1-2 5 .1.9.6 1.2 1.3.5-.3 2.4 1.9 4.6 5.3 4.6s5.6-2.2 5.3-4.6c.7.7 1.2.4 1.3-.5.1-1.9-.8-3.7-2-5C16.6 5.7 15.1 3 12 3Z"/><circle cx="10.5" cy="8" r=".8" fill="white"/><circle cx="13.5" cy="8" r=".8" fill="white"/></svg>:<Icon name={channel==='wechat'?'message':'inbox'} size={21}/>}</span>;
}
function observation(report?:ChannelReport) {
  if(!report?.observedAt)return {label:'尚未观测',tone:'neutral'};
  if(Date.now()-report.observedAt>20000)return {label:'状态过期',tone:'attention'};
  if(report.availability!=='available')return {label:'Bot 不可用',tone:'neutral'};
  return {label:({ready:'已连接',connecting:'连接中',stopped:'已停止',error:'连接异常',unknown:'状态未知'} as const)[report.service],tone:report.service==='ready'?'complete':report.service==='error'?'attention':'neutral'};
}
const nav = [
  ['overview','grid','总览'],['feedback','inbox','反馈收件箱'],['tickets','ticket','工单中心'],['channels','link','渠道连接'],['projects','folder','工作区'],
] as const;
const titles:Record<Page,[string,string]> = {
  overview:['维护工作台','看看工作推进到了哪里，还有什么需要你。'],
  feedback:['反馈收件箱','从用户原话，追溯到具体工单。'],
  tickets:['工单中心','跟进诊断、修复、验收与交付。'],
  channels:['渠道连接','沿用 Bot 已有会话，为对应工作区开启维护，无需重复绑定。'],
  projects:['工作区','沿用软件中聊天已绑定的工作区，无需重复添加项目。'],
};
function App() {
  const [page,setPage]=useState<Page>('overview');
  const [data,setData]=useState<Dashboard|null>(null);
  const [loading,setLoading]=useState(true),[loadError,setLoadError]=useState(''),[actionError,setActionError]=useState('');
  const [busy,setBusy]=useState(false),[selected,setSelected]=useState<string|null>(null),[note,setNote]=useState('');
  const [inspection,setInspection]=useState<Inspection|null>(null);
  const [query,setQuery]=useState(''),[filter,setFilter]=useState('all'),[source,setSource]=useState('all');
  const [modal,setModal]=useState<'bind'|'feedback'|null>(null);
  const [chats,setChats]=useState<AvailableChat[]>([]),[form,setForm]=useState({chatId:'',feedback:''});
  const mounted=useRef(true),revision=useRef(0),applied=useRef(0);
  async function refresh() {
    const rev=++revision.current;
    try {const snapshot=await request({action:'dashboard'});if(mounted.current&&rev>=applied.current){applied.current=rev;setData(snapshot);setLoadError('')}}
    catch(error){if(mounted.current&&rev>=applied.current){applied.current=rev;setLoadError(String(error instanceof Error?error.message:error))}}
    finally{if(mounted.current)setLoading(false)}
  }
  useEffect(()=>{mounted.current=true;void refresh();const timer=setInterval(refresh,5000);const ready=()=>void refresh();window.addEventListener('operitComposeDslInterfacesReady',ready);return()=>{mounted.current=false;clearInterval(timer);window.removeEventListener('operitComposeDslInterfacesReady',ready)}},[]);
  useEffect(()=>{const close=(e:KeyboardEvent)=>{if(e.key==='Escape'&&!busy){setSelected(null);setModal(null)}};window.addEventListener('keydown',close);return()=>window.removeEventListener('keydown',close)},[busy]);
  useEffect(()=>{
    if(!selected&&!modal)return;
    const dialog=document.querySelector<HTMLElement>('[role="dialog"]');if(!dialog)return;
    const previous=document.activeElement instanceof HTMLElement?document.activeElement:null;
    const overflow=document.body.style.overflow;document.body.style.overflow='hidden';dialog.tabIndex=-1;
    const items=()=>Array.from(dialog.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]')).filter(el=>el.getClientRects().length);
    (items()[0]??dialog).focus();
    const trap=(e:KeyboardEvent)=>{if(e.key!=='Tab')return;const list=items(),first=list[0],last=list[list.length-1];if(!first){e.preventDefault();dialog.focus()}else if(e.shiftKey&&(document.activeElement===first||document.activeElement===dialog)){e.preventDefault();last.focus()}else if(!e.shiftKey&&(document.activeElement===last||document.activeElement===dialog)){e.preventDefault();first.focus()}};
    dialog.addEventListener('keydown',trap);
    return()=>{dialog.removeEventListener('keydown',trap);document.body.style.overflow=overflow;previous?.focus()};
  },[selected,modal]);
  async function act(fn:()=>Promise<unknown>) {
    if(busy)return;setBusy(true);setActionError('');
    try{await fn();await refresh()}catch(error){setActionError(String(error instanceof Error?error.message:error))}finally{setBusy(false)}
  }
  function navigate(next:Page) {setPage(next);setQuery('');setFilter('all');setSource('all')}
  function openTicket(ticket:Ticket) {setSelected(ticket.id);setNote('');setInspection(null);setActionError('')}
  async function openBind() {setForm({chatId:'',feedback:''});setActionError('');setModal('bind');try{setChats(await request({action:'available_chats'}))}catch(error){setActionError(String(error))}}
  function openFeedback() {setForm({chatId:data?.bindings.length===1?data.bindings[0].chatId:'',feedback:''});setActionError('');setModal('feedback')}
  const all=data?.tickets??[],work=all.filter(inProgress),attention=all.filter(requiresAttention),completed=all.filter(t=>t.stage==='completed');
  const workspaceGroups=Array.from((data?.workspaceChats??[]).reduce((groups,chat)=>{
    if(chat.workspaceId&&chat.workspacePath){const group=groups.get(chat.workspaceId)??{id:chat.workspaceId,path:chat.workspacePath,chats:[] as AvailableChat[]};group.chats.push(chat);groups.set(chat.workspaceId,group)}return groups;
  },new Map<string,{id:string;path:string;chats:AvailableChat[]}>()).values());
  const current=all.find(t=>t.id===selected);
  const channels=data?.channels??[];
  const connected=channels.filter(c=>observation(c).label==='已连接').length;
  const timeline=all.flatMap(ticket=>ticket.notes.map(n=>({...n,ticket}))).filter(n=>n.kind!=='feedback').sort((a,b)=>b.at-a.at).slice(0,5);
  const filtered=all.filter(t=>(filter==='all'||filter==='active'&&inProgress(t)||filter==='attention'&&requiresAttention(t)||filter==='done'&&['completed','not_actionable'].includes(t.stage)||filter==='diagnosis'&&['new','triage'].includes(t.stage)||filter===t.stage)&&(source==='all'||t.source===source)&&`${t.feedback} ${t.projectPath} ${t.id}`.toLowerCase().includes(query.toLowerCase()));
  const feedback=(data?.feedback??[]).filter(f=>(source==='all'||f.channel===source)&&`${f.text} ${f.authorId}`.toLowerCase().includes(query.toLowerCase()));
  function ticketRows(tickets:Ticket[]) {
    return tickets.length?<div className="work-table"><div className="table-labels"><span>工单</span><span>处理状态</span><span>来源</span><span>最近更新</span></div>{tickets.map(ticket=><button key={ticket.id} className="work-row" onClick={()=>openTicket(ticket)}>
      <span className="work-subject"><span className={`work-indicator ${statusTone(ticket.stage)}`}><Icon name={ticket.stage==='completed'?'check':ticket.stage==='review'?'check':inProgress(ticket)?'activity':'ticket'} size={18}/></span><span><strong>{ticket.feedback.replace(/^\[[^\]]+\]\s*/,'')}</strong><small>{projectName(ticket.projectPath)}<span className="issue-id">{ticket.id.length>18?ticket.id.slice(0,8):ticket.id}</span></small></span></span>
      <Badge stage={ticket.stage}/><span className="row-source">{channelName(ticket.source)}</span><time className="row-time">{elapsed(ticket.updatedAt)}</time>
    </button>)}</div>:<Empty title="还没有工单">连接反馈入口，或提交第一条反馈。</Empty>;
  }
  function channelLine(channel:'qq'|'wechat') {
    const report=channels.find(c=>c.channel===channel),state=observation(report);
    return <button className="connection-row" key={channel} onClick={()=>navigate('channels')}><ChannelMark channel={channel}/><span className="connection-name"><strong>{channel==='qq'?'QQ Bot':'微信 Bot'}</strong><small>{report?.enabled?'正在采集反馈':'未启用采集'}</small></span><span className={`connection-state ${state.tone}`}><i/>{state.label}</span><Icon name="chevron" size={14}/></button>;
  }
  function channelCard(channel:'qq'|'wechat') {
    const report=channels.find(c=>c.channel===channel),state=observation(report);
    return <section className="channel-settings" key={channel}>
      <div className="channel-settings-heading"><ChannelMark channel={channel}/><div><h2>{channel==='qq'?'QQ Bot':'微信 Bot'}</h2><p>{channel==='qq'?'接收 QQ 群和私聊中的反馈':'接收微信会话中的反馈'}</p></div><span className={`status ${state.tone}`}><i/>{state.label}</span></div>
      <p className="session-summary">发现 {report?.sessions.length??0} 个会话 · 已开启 {report?.sessions.filter(session=>session.enabled).length??0} 个维护入口</p>
      {report?.lastError&&<p className="channel-error"><Icon name="info" size={15}/>{report.lastError}</p>}
      <ChannelSessions report={report} busy={busy} onAction={payload=>void act(()=>request(payload))}/>

    </section>;
  }
  return <div className="app-shell">
    <aside className="sidebar"><div className="brand"><span className="brand-symbol"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" aria-hidden="true"><path d="M8 4H4v16h4 M16 4h4v16h-4 M10 8l4 4-4 4"/></svg></span><strong>Operit</strong><span className="brand-context">维护</span></div>
      <div className="workspace-label"><span className="workspace-avatar">O</span><div><strong>项目维护</strong><small>本机工作空间</small></div></div>
      <nav aria-label="主导航">{nav.map(([id,icon,label])=><button key={id} className={`nav-item ${page===id?'active':''}`} onClick={()=>navigate(id)} aria-current={page===id?'page':undefined}><Icon name={icon} size={19}/><span>{label}</span>{id==='tickets'&&attention.length>0&&<em>{attention.length}</em>}</button>)}</nav>
      <div className="sidebar-bottom"><button className="sidebar-connection" onClick={()=>navigate('channels')}><Icon name="link" size={16}/><span>渠道连接</span><span>{connected}/2</span></button><div className={`host-connection ${loadError?'disconnected':''}`}><i/><span>{loadError?'数据读取中断':data?'维护服务已连接':'等待维护服务'}</span></div><small>{data?`更新于 ${timestamp(data.observedAt)}`:'从 Operit 侧边栏打开'}</small></div>
    </aside>
    <main className="main"><header className="topbar"><div className="breadcrumb">项目维护<Icon name="chevron" size={12}/><span>{page==='overview'?'总览':titles[page][0]}</span></div><div className="topbar-actions"><span className="experimental">实验版</span><button className="icon-button" aria-label="刷新" onClick={refresh}><Icon name="refresh" size={17}/></button></div></header>
      <div className="page-content"><div className="page-heading"><div><h1>{titles[page][0]}</h1><p>{titles[page][1]}</p></div><button className="button primary" onClick={openFeedback}><Icon name="plus" size={17}/>提交反馈</button></div>
        {actionError&&<div className="error-banner" role="alert"><Icon name="info"/><span>操作未完成：{actionError}</span><button onClick={()=>setActionError('')}>关闭</button></div>}
        {loadError&&<div className="error-banner" role="alert"><Icon name="info"/><span>{loadError}{data?' · 当前显示上一次成功读取的数据。':''}</span><button onClick={refresh}>重试</button></div>}
        {loading&&!data?<div className="loading"><div/><div/><p>正在读取维护服务…</p></div>:<>
          {page==='overview'&&<>
            <div className="overview-summary"><span><Icon name="folder" size={15}/><b>{workspaceGroups.length}</b> 个工作区</span><span><i className="dot working"/><b>{work.length}</b> 正在处理</span><span><i className="dot attention"/><b>{attention.length}</b> 需要关注</span><button onClick={()=>{navigate('tickets');setFilter('done')}}><b>{completed.length}</b> 已交付<Icon name="chevron" size={13}/></button></div>
            <section className="pipeline" aria-label="维护处理阶段"><div className="pipeline-heading"><h2>处理进度</h2><span>工单当前所处阶段</span></div><div className="pipeline-stages">{[
              {label:'诊断',filter:'diagnosis',count:all.filter(t=>['new','triage'].includes(t.stage)).length,icon:'search'},
              {label:'修复',filter:'implement',count:all.filter(t=>t.stage==='implement').length,icon:'branch'},
              {label:'验收',filter:'review',count:all.filter(t=>t.stage==='review').length,icon:'check'},
              {label:'交付',filter:'delivery',count:all.filter(t=>t.stage==='delivery').length,icon:'ticket'},
              {label:'完成',filter:'completed',count:completed.length,icon:'check'},
            ].map((step,index)=><button className={`pipeline-step ${step.count?'has-work':''} ${index===4?'finished':''}`} key={step.filter} onClick={()=>{navigate('tickets');setFilter(step.filter)}}><span className="pipeline-track"><span className="pipeline-node"><Icon name={step.icon} size={17}/></span><span className="pipeline-line"/></span><span className="pipeline-caption"><span>{step.label}</span><strong>{step.count}</strong></span></button>)}</div></section>
            {!data?.bindings.length&&!channels.some(c=>c.enabled)&&<div className="connect-prompt"><Icon name="link" size={21}/><div><h3>先为项目连接一个反馈入口</h3><p>选择已绑定工作区的聊天，后续反馈会自动整理成工单。</p></div><button className="button secondary" onClick={openBind}>连接反馈入口</button></div>}
            <div className="overview-grid"><div className="overview-main"><Section className="queue-section" title="正在推进" description={`${work.length} 个处理中，${attention.length} 个待关注`} extra={<button className="text-button" onClick={()=>navigate('tickets')}>查看全部<Icon name="chevron" size={14}/></button>}>{ticketRows([...work,...attention].slice(0,6))}</Section><Section className="activity-section" title="最近动态" extra={<span className="section-caption">阶段报告</span>}>{timeline.length?<div className="activity-list">{timeline.map((entry,index)=><button className="activity-row" key={`${entry.ticket.id}-${index}`} onClick={()=>openTicket(entry.ticket)}><span className={`activity-point ${entry.kind==='delivery_verification'?'complete':''}`}>{entry.kind==='delivery_verification'?<Icon name="check" size={13}/>:<i/>}</span><span><strong>{noteNames[entry.kind]||entry.kind}</strong><p>{projectName(entry.ticket.projectPath)}<span>{entry.ticket.feedback}</span></p></span><time>{elapsed(entry.at)}</time></button>)}</div>:<Empty title="还没有处理记录">收到反馈后，诊断、修复和验收记录会显示在这里。</Empty>}</Section></div><aside className="overview-side"><Section className="attention-section" title="需要你" extra={<span className="section-count">{attention.length}</span>}>{attention.length?<div className="attention-list">{attention.slice(0,3).map(ticket=><button className="attention-item" key={ticket.id} onClick={()=>openTicket(ticket)}><span className="attention-item-label"><span className="dot attention"/>{stageNames[ticket.stage]}</span><strong>{ticket.feedback}</strong><span className="attention-action">{ticket.stage==='needs_info'?'补充信息':ticket.stage==='new'?'查看诊断任务':ticket.stage==='interrupted'?'检查执行状态':'查看失败原因'}<Icon name="chevron" size={13}/></span></button>)}</div>:<div className="attention-clear"><span><Icon name="check" size={22}/></span><h3>{all.length?'暂时没有待关注的工单':'等待第一条反馈'}</h3><p>{all.length?'执行和验收记录会持续更新。':'连接入口后，在这里跟进需要处理的问题。'}</p></div>}</Section><Section className="connections-section" title="渠道连接" extra={<button className="text-button" onClick={()=>navigate('channels')}>管理</button>}><div className="connection-list">{channelLine('qq')}{channelLine('wechat')}<button className="connection-row" onClick={openBind}><ChannelMark channel="app"/><span className="connection-name"><strong>软件内反馈</strong><small>{data?.bindings.length??0} 个聊天入口</small></span><span className="connection-state neutral">设置</span><Icon name="chevron" size={14}/></button></div><p className="connection-footnote">使用对应 Bot 的现有连接，无需额外桥接插件。</p></Section></aside></div>
          </>}
          {page==='tickets'&&<Section title="全部工单" extra={<span className="section-caption">{all.length} 个工单</span>}><div className="list-toolbar"><div className="search"><Icon name="search" size={17}/><input aria-label="搜索工单" placeholder="搜索反馈、项目或工单 ID" value={query} onChange={e=>setQuery(e.target.value)}/></div><select aria-label="工单来源" value={source} onChange={e=>setSource(e.target.value)}><option value="all">所有来源</option>{['app','panel','qq','wechat'].map(value=><option value={value} key={value}>{channelName(value)}</option>)}</select></div><div className="filter-tabs">{[['all','全部'],['active','处理中'],['attention','待关注'],['done','已结束']].map(([value,label])=><button className={filter===value?'selected':''} key={value} onClick={()=>setFilter(value)}>{label}</button>)}{!['all','active','attention','done'].includes(filter)&&<button className="selected" onClick={()=>setFilter('all')}>{filter==='diagnosis'?'诊断':stageNames[filter as Stage]}<Icon name="close" size={12}/></button>}</div>{ticketRows(filtered)}</Section>}
          {page==='feedback'&&<Section title="原始反馈" description={`共 ${data?.feedbackTotal??0} 条，展示最近 300 条。`}><div className="list-toolbar"><div className="search"><Icon name="search" size={17}/><input aria-label="搜索反馈" placeholder="搜索原话或反馈人" value={query} onChange={e=>setQuery(e.target.value)}/></div><select aria-label="反馈渠道" value={source} onChange={e=>setSource(e.target.value)}><option value="all">全部渠道</option>{['app','qq','wechat'].map(value=><option key={value} value={value}>{channelName(value)}</option>)}</select></div>{feedback.length?<div className="feedback-list">{feedback.map(item=><article className="feedback-item" key={item.key}><div className="feedback-byline"><span className="feedback-avatar">{item.authorId.slice(0,1)||'访'}</span><strong>{item.authorId||'未标明用户'}</strong><span>{channelName(item.channel)}</span><time>{timestamp(item.receivedAt)}</time></div><p>{item.text||'这条反馈没有文本内容。'}</p><div className="feedback-bottom"><span><Icon name="folder" size={13}/>{projectName(item.projectPath)}</span>{item.ticketId?<button className="text-button" onClick={()=>{const ticket=all.find(t=>t.id===item.ticketId);if(ticket)openTicket(ticket)}}>查看工单<Icon name="chevron" size={13}/></button>:<span className={`feedback-processing ${item.ignored?'':'pending'}`}>{item.ignored==='acknowledgement'?'无需建单':item.ignored?'进度问询':'等待整理'}</span>}</div></article>)}</div>:<Empty title={query||source!=='all'?'没有符合条件的反馈':'还没有收到反馈'}>{query||source!=='all'?'调整搜索内容或切换渠道。':'连接专用聊天或 Bot 会话，收集用户的原始反馈。'}</Empty>}</Section>}
          {page==='channels'&&<><div className="channel-grid">{channelCard('qq')}{channelCard('wechat')}</div><p className="page-note"><Icon name="info" size={16}/>每 3 秒读取 Bot 连接状态。账号登录在对应 Bot 插件中完成，会话来自 Bot 的真实绑定，开启维护不会更改 Bot 的自动回复设置。</p><Section title="软件内反馈" description="使用聊天在软件中绑定的工作区，新消息会自动进入维护流程。" extra={<button className="button secondary" onClick={openBind}><Icon name="plus" size={16}/>连接入口</button>}>{data?.bindings.some(binding=>!channels.some(channel=>channel.sessions.some(session=>session.chatId===binding.chatId)))?<div>{data.bindings.filter(binding=>!channels.some(channel=>channel.sessions.some(session=>session.chatId===binding.chatId))).map(binding=><div className="binding-row" key={binding.chatId}><span className="project-mark"><Icon name="folder" size={21}/></span><div><h3>{projectName(binding.projectPath??'')}</h3><p>{binding.projectPath}</p><small>聊天 {binding.chatId}</small></div><button className="text-button danger" disabled={busy} onClick={()=>{if(window.confirm('停止收集这个聊天的新反馈？已有工单会保留。'))void act(()=>request({action:'bind_chat',chatId:binding.chatId,enabled:false}))}}>停止收集</button></div>)}</div>:<Empty title="还没有连接聊天">选择工作区聊天并启用收集，开始接收反馈。</Empty>}</Section></>}
          {page==='projects'&&<div className="projects-list">{workspaceGroups.length?workspaceGroups.map(group=>{
            const items=all.filter(ticket=>group.chats.some(chat=>chat.id===ticket.parentChatId));
            return <section className="project-section" key={group.id}><div className="project-heading"><span className="project-mark"><Icon name="folder" size={22}/></span><div><h2>{projectName(group.path)}</h2><p>{group.path}</p></div></div><div className="project-statistics"><span><b>{items.length}</b>关联工单</span><span><b>{items.filter(inProgress).length}</b>处理中</span><span><b>{items.filter(requiresAttention).length}</b>待关注</span><span><b>{items.filter(t=>t.stage==='completed').length}</b>已完成</span></div><div className="project-footer"><span>{group.chats.map(chat=>chat.title||'未命名聊天').join('、')}</span><span>来自软件工作区绑定</span></div></section>
          }):<Empty title="还没有工作区聊天">先在软件中为聊天绑定工作区，这里会自动显示。</Empty>}</div>}
        </>}
        <footer className="page-footer"><span>Operit 项目维护</span><span><Icon name="device" size={13}/>任务在绑定的执行节点运行</span></footer>
      </div>
    </main>
    {current&&<div className="overlay" onClick={()=>!busy&&setSelected(null)}><section className="drawer" role="dialog" aria-modal="true" aria-label="工单详情" onClick={e=>e.stopPropagation()}><div className="drawer-top"><span>工单 {current.id}</span><button className="icon-button" aria-label="关闭工单" disabled={busy} onClick={()=>setSelected(null)}><Icon name="close"/></button></div><div className="drawer-content"><Badge stage={current.stage}/><h2 className="detail-title">{current.feedback}</h2><div className="detail-context"><span><Icon name="folder" size={14}/>{projectName(current.projectPath)}</span><span>{channelName(current.source)}</span><span>第 {current.attempt} 次尝试</span></div><dl className="detail-facts"><div><dt>项目路径</dt><dd>{current.projectPath}</dd></div><div><dt>关联代理</dt><dd>{current.agentId||'暂无'}</dd></div><div><dt>最近更新</dt><dd>{timestamp(current.updatedAt)}</dd></div></dl>{current.error&&<div className="inline-warning">{current.error}</div>}<h3 className="detail-section-title">处理记录</h3><div className="detail-timeline">{current.notes.filter(entry=>entry.kind!=='feedback').map((entry,index)=><article className="detail-entry" key={index}><span className="entry-dot"/><div><header><h4>{noteNames[entry.kind]||entry.kind}</h4><time>{timestamp(entry.at)}</time></header><pre>{formatNote(entry.text)}</pre></div></article>)}{!current.notes.filter(entry=>entry.kind!=='feedback').length&&<p className="muted">还没有处理记录。</p>}</div><div className="detail-controls"><label>补充信息<textarea placeholder="复现步骤、预期表现，或其他能帮助定位的信息…" value={note} onChange={e=>setNote(e.target.value)}/></label><div className="detail-buttons"><button className="button primary" disabled={busy||!note.trim()} onClick={()=>void act(async()=>{await request({action:'add_note',id:current.id,text:note});setNote('')})}>添加信息</button>{['new','needs_info','not_actionable','failed','interrupted'].includes(current.stage)&&<button className="button secondary" disabled={busy} onClick={()=>void act(()=>request({action:'start',id:current.id}))}>开始 / 重试</button>}<button className="text-button" disabled={busy} onClick={()=>void act(async()=>setInspection(await request({action:'inspect',id:current.id})))}>检查执行状态</button></div>{actionError&&<div className="inline-warning" role="alert">{actionError}</div>}{inspection&&<div className="inspection"><h4>宿主观测</h4><p>项目占用：{inspection.claim}<br/>代理：{inspection.managed}，对话：{inspection.native}</p><small>空闲状态不能确认外部操作已结束，恢复前请核对仓库。</small></div>}</div></div></section></div>}
    {modal&&<div className="overlay modal-overlay" onClick={()=>!busy&&setModal(null)}><section className="modal" role="dialog" aria-modal="true" aria-label={modal==='bind'?'连接反馈入口':'提交反馈'} onClick={e=>e.stopPropagation()}><div className="modal-heading"><h2>{modal==='bind'?'连接反馈入口':'提交反馈'}</h2><button className="icon-button" aria-label="关闭" disabled={busy} onClick={()=>setModal(null)}><Icon name="close"/></button></div><p className="modal-intro">{modal==='bind'?'启用聊天的反馈收集，直接使用它在软件中绑定的工作区。':'描述遇到的问题，维护代理会先诊断是否需要修改代码。'}</p>{actionError&&<div className="inline-warning" role="alert">{actionError}</div>}<form onSubmit={e=>{e.preventDefault();void act(async()=>{if(modal==='bind'){await request({action:'bind_chat',chatId:form.chatId,enabled:true});setModal(null)}else{const ticket=await request({action:'create_from_binding',chatId:form.chatId,feedback:form.feedback});setModal(null);openTicket(ticket)}})}}><fieldset disabled={busy}><label>{modal==='bind'?'反馈聊天':'工作区聊天'}<select required value={form.chatId} onChange={e=>setForm({...form,chatId:e.target.value})}><option value="">请选择{modal==='bind'?'已绑定工作区的聊天':'已启用收集的聊天'}</option>{(modal==='bind'?chats.filter(chat=>chat.workspaceId&&chat.workspacePath&&!channels.some(channel=>channel.sessions.some(session=>session.chatId===chat.id))).map(chat=>({id:chat.id,title:(chat.title||'未命名聊天')+' · '+projectName(chat.workspacePath!)})):data?.bindings.map(binding=>({id:binding.chatId,title:projectName(binding.projectPath??'')+' / '+binding.chatId.slice(0,8)}))??[]).map(chat=><option key={chat.id} value={chat.id}>{chat.title}</option>)}</select></label>{modal==='bind'?<><p className="form-help">工作区：{chats.find(chat=>chat.id===form.chatId)?.workspacePath||'选择聊天后自动读取'}。如需更换目录，请在软件的工作区中修改绑定。</p>{!chats.some(chat=>chat.workspaceId&&chat.workspacePath)&&<p className="inline-warning">还没有已绑定工作区的聊天，请先在软件中为聊天绑定 Git 仓库工作区。</p>}</>:<><label>反馈内容<textarea required maxLength={30000} rows={6} placeholder="发生了什么？如何复现？预期应该怎样？" value={form.feedback} onChange={e=>setForm({...form,feedback:e.target.value})}/></label>{!data?.bindings.length&&<button type="button" className="text-button" onClick={openBind}>先连接一个反馈入口</button>}</>}</fieldset><div className="modal-actions"><button type="button" className="button secondary" disabled={busy} onClick={()=>setModal(null)}>取消</button><button className="button primary" disabled={busy||!form.chatId||(modal==='feedback'&&!form.feedback.trim())}>{busy?'正在保存…':modal==='bind'?'连接并开始收集':'提交并诊断'}</button></div></form></section></div>}
  </div>;
}
createRoot(document.getElementById('root')!).render(<App/>);
