import assert from 'node:assert/strict';
import {mkdir,readFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {fileURLToPath} from 'node:url';
import {chromium,webkit} from 'playwright';
// Exercise the actual Compose bridge generator, including its wire envelope and ready event.
const bridgeSource=await readFile(new URL('../../../../../apps/flutter/app/lib/ui/features/packages/screens/ToolPkgComposeDslWebViewBridgeRuntime.dart',import.meta.url),'utf8');
const bridgeTemplate=bridgeSource.match(/return '''([\s\S]*?)''';/)[1];
const bridgeRuntime=bridgeTemplate
 .replaceAll('$hiddenBridgeNameJson',JSON.stringify('__ComposeDslWebViewHostBridge__'))
 .replaceAll('$channelNameJson',JSON.stringify('__ComposeDslWebViewHostBridgeChannel__'))
 .replaceAll('$interfacesJson',"window.__fixture.delayInterfaces ? {} : {MaintenanceHost:['request']}");
const html=await readFile(new URL('../resources/control.html',import.meta.url));
const server=createServer((_req,res)=>{res.setHeader('content-type','text/html; charset=utf-8');res.end(html)});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const url=`http://127.0.0.1:${server.address().port}/`;
const output=process.env.OPERIT_MAINTENANCE_SCREENSHOTS||'/tmp/operit-maintenance-web';await mkdir(output,{recursive:true});
const now=Date.now();
const makeTicket=(id,stage,source,feedback,offset=0)=>({schemaVersion:1,id,parentChatId:'maintenance-chat',projectPath:'/Users/demo/projects/pixel-garden',source,feedback,stage,createdAt:now-3600000,updatedAt:now-offset,agentId:stage==='new'?null:`agent-${id}`,attempt:1,error:null,notes:[{at:now-600000,kind:'triage',text:JSON.stringify({decision:'actionable',reason:'复现后确认分数状态没有重置。'})},{at:now-300000,kind:stage==='completed'?'delivery_verification':'implement',text:JSON.stringify({summary:'已修复计分状态，等待独立验收。'})}]});
const tickets=[makeTicket('T-103','review','qq','重新开始后，上一局的分数没有清零',12000),makeTicket('T-102','implement','app','希望在暂停时停止计时，恢复后继续',60000),makeTicket('T-101','needs_info','wechat','手机上点开始，有时没有反应',180000),makeTicket('T-100','completed','app','修复结算界面按钮被遮挡的问题',720000)];
const workspaceChats=[{id:'maintenance-chat',title:'像素花园 · 维护执行',workspaceId:'ws-main',workspacePath:'/Users/demo/projects/pixel-garden'},{id:'new-chat',title:'新工作区反馈',workspaceId:'ws-new',workspacePath:'/Users/demo/projects/new-game'},{id:'unbound',title:'没有工作区',workspaceId:null,workspacePath:null}];
const snapshot={workspaceChats,observedAt:now,tickets,bindings:[{chatId:'maintenance-chat',projectPath:'/Users/demo/projects/pixel-garden',since:now-7200000}],channels:[{availability:'available',channel:'qq',enabled:true,service:'ready',sessions:[{key:'qq-group',channel:'qq',accountId:'qq-account',scene:'group',conversationId:'group-demo-42',title:'游戏体验群',chatId:'maintenance-chat',workspacePath:'/Users/demo/projects/pixel-garden',lastMessageAt:now,enabled:true,error:''},{key:'qq-private',channel:'qq',accountId:'qq-account',scene:'c2c',conversationId:'private-user',title:'QQ 私聊用户',chatId:null,workspacePath:null,lastMessageAt:now,enabled:false,error:''}],lastError:'',observedAt:now},{availability:'unavailable',channel:'wechat',enabled:false,service:'unknown',sessions:[],lastError:'对应 Bot 未安装或未启用',observedAt:now}],feedbackTotal:5,feedback:[{key:'f-pending',channel:'qq',authorId:'体验用户',conversationId:'group-demo-42',projectPath:'/Users/demo/projects/pixel-garden',text:'再补充一下：第二次点重新开始也能复现。',receivedAt:now-4000,ticketId:null,ignored:null},...tickets.map((t,i)=>({key:`f-${i}`,channel:t.source,authorId:`玩家 ${i+1}`,conversationId:'feedback-demo',projectPath:t.projectPath,text:t.feedback,receivedAt:t.createdAt,ticketId:t.id,ignored:null}))],projects:[{path:'/Users/demo/projects/pixel-garden',tickets:4,active:2,attention:1,completed:1,updatedAt:now-12000,chats:['maintenance-chat'],channels:['qq']}]};
async function fixture(page,{delayInterfaces=false}={}){
  const initializeFixture=({snapshot,delayInterfaces})=>{
    window.__fixture={snapshot,delayInterfaces,calls:[],failAction:false,failDashboard:false};
    async function dispatch(payload){
      const f=window.__fixture;f.calls.push(payload);
      if(payload.action==='dashboard'){if(f.failDashboard)throw Error('服务暂时不可用');return structuredClone(f.snapshot)}
      if(payload.action==='available_chats')return structuredClone(f.snapshot.workspaceChats);
      if(f.failAction)throw Error('保存失败，请稍后重试');
      if(payload.action==='prepare_channel_chat'){
        const session=f.snapshot.channels.flatMap(c=>c.sessions).find(c=>c.key===payload.conversationKey);session.chatId='prepared-chat';return {chatId:session.chatId};
      }
      if(payload.action==='bind_chat'){
        f.snapshot.bindings=f.snapshot.bindings.filter(b=>b.chatId!==payload.chatId);const path=f.snapshot.workspaceChats.find(c=>c.id===payload.chatId)?.workspacePath;
        if(payload.enabled)f.snapshot.bindings.push({chatId:payload.chatId,projectPath:path,since:Date.now()});
        for(const channel of f.snapshot.channels){for(const session of channel.sessions)if(session.chatId===payload.chatId)session.enabled=payload.enabled;channel.enabled=channel.sessions.some(s=>s.enabled)}
        return {chatId:payload.chatId,projectPath:payload.enabled?path:null};
      }
      if(payload.action==='create_from_binding'){
        const ticket={...f.snapshot.tickets[0],id:'T-new',stage:'new',feedback:payload.feedback};f.snapshot.tickets.unshift(ticket);return structuredClone(ticket);
      }
      if(payload.action==='add_note'){const t=f.snapshot.tickets.find(t=>t.id===payload.id);t.notes.push({kind:'user',at:Date.now(),text:payload.text});return structuredClone(t)}
      if(payload.action==='inspect')return {ticketId:payload.id,observedAt:Date.now(),claim:'this_ticket',agentId:'agent',managed:'running',native:'processing'};
      if(payload.action==='start')return f.snapshot.tickets.find(t=>t.id===payload.id);
      if(payload.action==='get')return f.snapshot.tickets.find(t=>t.id===payload.id);
      throw Error('Unknown action '+payload.action);
    }
    window.__ComposeDslWebViewHostBridgeChannel__={postMessage(raw){
      const {id,type,payload}=JSON.parse(raw);
      Promise.resolve().then(()=>{
        if(type!=='invoke'||payload.interfaceName!=='MaintenanceHost'||payload.methodName!=='request')throw Error('Unexpected Compose bridge envelope');
        const args=JSON.parse(payload.args);
        if(!Array.isArray(args)||args.length!==1)throw Error('Invalid Compose bridge arguments');
        return dispatch(args[0]).then(data=>({ok:true,data}),error=>({ok:false,error:error.message}));
      }).then(data=>window.__operitComposeDslWebViewHostReceive({id,success:true,data}),error=>window.__operitComposeDslWebViewHostReceive({id,success:false,message:error.message}));
    }};
  };
  await page.addInitScript({content:`(${initializeFixture.toString()})(${JSON.stringify({snapshot,delayInterfaces})});\n${bridgeRuntime}`});
  await page.goto(url);await page.getByRole('heading',{name:'维护工作台',exact:true}).waitFor();
  if(!delayInterfaces)await page.locator('.host-connection').getByText('维护服务已连接',{exact:true}).waitFor({state:'attached'});
}
async function navigate(page,name){await page.getByRole('navigation').getByRole('button',{name:new RegExp(`^${name}`)}).click()}
async function noOverflow(page){assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,'Page must fit the viewport')}
const browser=await chromium.launch({headless:true});
try{
 const page=await browser.newPage({viewport:{width:1440,height:1000},deviceScaleFactor:1});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 page.setDefaultTimeout(7000);await fixture(page);await noOverflow(page);
 await page.screenshot({path:`${output}/desktop-overview.png`,fullPage:true});
 await navigate(page,'渠道连接');await page.getByText('Bot 不可用',{exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'配置采集',exact:true}).count(),0);
 assert.equal(await page.getByLabel('精确会话 ID').count(),0);
 await page.getByText('游戏体验群',{exact:true}).waitFor();
 await page.getByRole('button',{name:'停止维护',exact:true}).click();
 await page.getByRole('button',{name:'开启维护',exact:true}).first().waitFor();
 await page.getByRole('button',{name:'开启维护',exact:true}).first().click();
 await page.getByRole('button',{name:'停止维护',exact:true}).waitFor();
 await page.getByRole('button',{name:'创建会话聊天',exact:true}).click();
 await page.getByText(/在软件的聊天列表找到此会话/).waitFor();
 assert.equal(await page.evaluate(()=>window.__fixture.calls.some(c=>c.action==='prepare_channel_chat'&&c.conversationKey==='qq-private')),true);
 await page.screenshot({path:`${output}/desktop-channel-sessions.png`,fullPage:true});
 await navigate(page,'反馈收件箱');await page.getByLabel('搜索反馈').fill('第二次');assert.equal(await page.locator('.feedback-item').count(),1);
 await navigate(page,'工单中心');await page.getByLabel('搜索工单').fill('分数');assert.equal(await page.locator('.work-row').count(),1);
 await page.locator('.work-row').click();
 const detail=page.getByRole('dialog');await detail.getByLabel('补充信息').fill('复现浏览器：Safari，重开第二局。');
 await detail.getByRole('button',{name:'添加信息'}).click();await detail.getByText('复现浏览器：Safari，重开第二局。',{exact:true}).waitFor();
 await detail.getByRole('button',{name:'检查执行状态'}).click();await detail.getByRole('heading',{name:'宿主观测'}).waitFor();
 await page.screenshot({path:`${output}/desktop-ticket.png`,fullPage:true});
 // Keyboard focus remains inside the drawer and Escape restores the page.
 await page.keyboard.press('Shift+Tab');assert.equal(await page.evaluate(()=>!!document.activeElement.closest('[role="dialog"]')),true);
 await page.keyboard.press('Escape');await detail.waitFor({state:'hidden'});
 await navigate(page,'工作区');await page.getByRole('heading',{name:'pixel-garden'}).waitFor();
 await navigate(page,'渠道连接');await page.getByRole('button',{name:'连接入口',exact:true}).click();
 let modal=page.getByRole('dialog',{name:'连接反馈入口'});
 await modal.getByLabel('反馈聊天').selectOption('new-chat');assert.equal(await modal.getByRole('option',{name:'没有工作区'}).count(),0);assert.equal(await modal.locator('input').count(),0);await modal.getByText(/工作区：.*new-game/).waitFor();
 await modal.getByRole('button',{name:'连接并开始收集'}).click();await modal.waitFor({state:'hidden'});
 await page.getByRole('button',{name:'提交反馈',exact:true}).click();modal=page.getByRole('dialog',{name:'提交反馈'});
 await modal.getByLabel('工作区聊天').selectOption('maintenance-chat');await modal.getByLabel('反馈内容').fill('提交入口的交互验证反馈');
 await modal.getByRole('button',{name:'提交并诊断'}).click();await modal.waitFor({state:'hidden'});await page.getByRole('dialog').getByText('提交入口的交互验证反馈',{exact:true}).waitFor();await page.keyboard.press('Escape');
 // A successful background refresh cannot erase an action failure.
 await navigate(page,'渠道连接');await page.evaluate(()=>window.__fixture.failAction=true);
 page.once('dialog',dialog=>dialog.accept());
 await page.getByRole('button',{name:'停止收集',exact:true}).first().click();await page.getByRole('alert').getByText(/保存失败/).waitFor();
 await page.getByRole('button',{name:'刷新',exact:true}).click();await page.waitForFunction(()=>window.__fixture.calls.filter(c=>c.action==='dashboard').length>5);
 assert.equal(await page.getByRole('alert').getByText(/保存失败/).isVisible(),true);
 await page.evaluate(()=>{window.__fixture.failAction=false;window.__fixture.failDashboard=true});await page.getByRole('button',{name:'刷新',exact:true}).click();
 await page.getByRole('alert').getByText(/当前显示上一次/).waitFor();assert.equal(await page.getByRole('heading',{name:'QQ Bot',exact:true}).isVisible(),true);
 assert.deepEqual(errors,[]);
 // A late interface installation must recover from the startup error immediately via the host event.
 const delayed=await browser.newPage({viewport:{width:1000,height:800}});
 await fixture(delayed,{delayInterfaces:true});
 await delayed.getByRole('alert').getByText(/维护接口尚未就绪/).waitFor();
 await delayed.evaluate(()=>window.__operitInstallComposeDslJavascriptInterfaces({MaintenanceHost:['request']}));
 await delayed.locator('.host-connection').getByText('维护服务已连接',{exact:true}).waitFor({timeout:1500});
 assert.equal(await delayed.getByRole('alert').count(),0);
 assert.equal(await delayed.evaluate(()=>typeof window.flutter_inappwebview),'undefined');
 await delayed.close();
 // Narrow screens exercise every page rather than only the overview.
 const mobile=await browser.newPage({viewport:{width:360,height:850},deviceScaleFactor:1,isMobile:true,hasTouch:true});await fixture(mobile);
 for(const name of ['总览','反馈收件箱','工单中心','渠道连接','工作区']){await navigate(mobile,name);await noOverflow(mobile);await mobile.screenshot({path:`${output}/mobile-${{'总览':'overview','反馈收件箱':'feedback','工单中心':'tickets','渠道连接':'channels','工作区':'projects'}[name]}.png`,fullPage:true})}
 await navigate(mobile,'渠道连接');await mobile.getByText('游戏体验群',{exact:true}).waitFor();await noOverflow(mobile);await mobile.screenshot({path:`${output}/mobile-channel-settings.png`,fullPage:true});
 console.log('Chromium: real Compose bridge, delayed interface readiness, five pages, Bot-owned conversation routing, ticket actions, feedback entry, refresh errors, focus and 360px layout passed.');
}finally{await browser.close()}
try{
 const safari=await webkit.launch({headless:true});
 try{const page=await safari.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});page.setDefaultTimeout(7000);await fixture(page);await noOverflow(page);await navigate(page,'渠道连接');await page.getByText('游戏体验群',{exact:true}).waitFor();await noOverflow(page);console.log('WebKit: mobile overview, navigation and conversation sessions passed.')}finally{await safari.close()}
}finally{server.close()}
console.log(`Screenshots: ${output}`);
