import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { objectStorage } from '../../../external/qqbot/tests/storage.mjs';
const require = createRequire(import.meta.url);
const storage = objectStorage();
let handler, hook;
const workspacePaths = new Map([['feedback','/game'],['panel-chat','/game-panel'],['resume-chat','/game-resume'],['status-summary','/status-game']]);
globalThis.Tools = {Storage:{objects:storage}, SoftwareSettings:{async exec(args){
 if(args[1]==='plugin'&&args[2]==='list')return '[]';
 if(args[1]==='chat'&&args[2]==='list')return JSON.stringify([...workspacePaths].map(([id,path])=>({id,title:id,workspaceId:'ws-'+id,workspacePrimaryPath:path})));
 if(args[1]==='workspace'&&args[2]==='exec')return JSON.stringify({exitCode:0,stdout:args[3]+'\n\n'});
 throw Error('Unexpected command '+JSON.stringify(args));
}}};
let nextRandom = 0;
globalThis.CryptoJS = {binary:{randomHex:()=>String(++nextRandom).padStart(32,"0")}};
globalThis.ToolPkg = {
 getSpaceDataDir:()=>'/app-test', ipc:{on(_channel,fn){handler=fn;},async call(_channel,request){return handler(request);}},
 registerChatMessageHook(def){hook=def.function;}, registerPromptFinalizeHook(){}, registerApi(){},registerUiRoute(){},registerNavigationEntry(){},
 registerAppLifecycleHook(){},registerHostEventHook(){},
};
const main=require('../dist/main.js'); main.registerToolPkg();
const {Repository}=require('../dist/storage.js');
const {CaptureCoordinator}=require('../dist/capture.js');
const {MaintenanceEngine}=require('../dist/engine.js');
const event=(chatId,timestamp,content,sender='user')=>({eventName:'message_persisted',eventPayload:{chatId,timestamp,content,sender}});
test('bound in-app conversation collects and deduplicates only user messages into one ticket',async()=>{
 const repository=new Repository(); let now=Date.now()+20000; const tasks=[];
 const engine=new MaintenanceEngine(repository,{async spawn(parent,name,task){tasks.push({parent,name,task});return {id:'a',status:'running',error:null,result:null};},async wait(){return {id:'a',status:'completed',error:null,result:{response:'{"decision":"needs_info"}'}};}},{now:()=>now,id:()=>`app-ticket-${now}`,report:e=>{throw e;},git:{async cleanHead(){return "a".repeat(40);}}});
 const capture=new CaptureCoordinator(repository,engine,()=>now,e=>{throw e;});
 await hook(event('unbound',1,'private'));
 await assert.rejects(handler({action:'bind_chat',chatId:'unbound',enabled:true}),/尚未绑定工作区/);
 await handler({action:'bind_chat',chatId:'feedback',enabled:true});
 await hook(event('feedback',1,'刚玩游戏有点奇怪'));
 await hook(event('feedback',1,'刚玩游戏有点奇怪'));
 await hook(event('feedback',2,'重开以后分数没清空'));
 await hook(event('feedback',3,'assistant output','ai'));
 await capture.tick(); const tickets=await engine.list();
 assert.equal(tickets.length,1); assert.equal(tickets[0].source,'app');
 assert.match(tickets[0].feedback,/刚玩游戏/); assert.match(tickets[0].feedback,/分数没清空/);
 assert.doesNotMatch(tickets[0].feedback,/private|assistant output/); await new Promise(resolve=>setImmediate(resolve)); assert.equal(tasks.length,1);
 await handler({action:'bind_chat',chatId:'feedback',enabled:false});
 await hook(event('feedback',4,'should not be collected'));
 assert.equal((await repository.pendingCaptures()).length,0);
});

test('panel feedback uses the current binding, starts automatically, and refuses a disabled binding',async()=>{
 await handler({action:'bind_chat',chatId:'panel-chat',enabled:true});
 const ticket=await handler({action:'create_from_binding',chatId:'panel-chat',feedback:'暂停后音乐还在放'});
 assert.equal(ticket.parentChatId,'panel-chat'); assert.equal(ticket.projectPath,'/game-panel');
 assert.equal(ticket.source,'panel'); assert.equal(ticket.autoStart,true); assert.equal(ticket.stage,'new');
 const repository=new Repository(), tasks=[];
 const engine=new MaintenanceEngine(repository,{async spawn(parent,name,task){tasks.push({parent,name,task});return {id:'panel-agent',status:'running',error:null,result:null};},async wait(){return {id:'panel-agent',status:'completed',error:null,result:{response:'{"decision":"needs_info","reason":"请说明设备"}'}};}},{now:()=>Date.now()+20000,id:()=>`unused`,report:e=>{throw e;},git:{async cleanHead(){return 'a'.repeat(40);}}});
 const capture=new CaptureCoordinator(repository,engine,()=>Date.now()+20000,e=>{throw e;});
 await capture.tick(); await new Promise(resolve=>setImmediate(resolve));
 assert.ok(tasks.some(task=>task.parent==='panel-chat' && task.task.includes('暂停后音乐还在放')));
 await handler({action:'bind_chat',chatId:'panel-chat',enabled:false});
 await assert.rejects(handler({action:'create_from_binding',chatId:'panel-chat',feedback:'过期提交'}),/已停用/);
});

test('a panel clarification waits for the prior project claim and then automatically re-diagnoses',async()=>{
 await handler({action:'bind_chat',chatId:'resume-chat',enabled:true});
 const created=await handler({action:'create_from_binding',chatId:'resume-chat',feedback:'游戏不对劲'});
 const repo=new Repository(), ticket=await repo.get(created.id);
 ticket.stage='needs_info'; ticket.autoStart=false;
 ticket.notes.push({at:Date.now(),kind:'triage',text:'{"decision":"needs_info","reason":"怎么复现？"}'});
 await repo.save(ticket);
 assert.equal(await repo.claimProject('/game-resume',created.id,'prior-owner'),true);
 const added=await handler({action:'add_note',id:created.id,text:'原来是我的浏览器扩展禁止了脚本，关闭后正常'});
 assert.equal(added.autoResumeAfterInfo,true); assert.equal(added.stage,'needs_info');
 const tasks=[];
 const engine=new MaintenanceEngine(repo,{async spawn(parent,name,task){tasks.push({parent,name,task});return {id:'resume-agent',status:'running',error:null,result:null};},async wait(){return {id:'resume-agent',status:'completed',error:null,result:{response:'{"decision":"not_actionable","reason":"本地设置"}'}};}},{now:()=>Date.now()+100,id:()=>`unused`,report:e=>{throw e;},git:{async cleanHead(){return 'a'.repeat(40);}}});
 const capture=new CaptureCoordinator(repo,engine,()=>Date.now()+100,e=>{throw e;});
 await capture.tick(); assert.equal((await repo.get(created.id)).stage,'needs_info'); assert.equal(tasks.length,0);
 await repo.releaseProject('/game-resume','prior-owner');
 await capture.tick(); await new Promise(resolve=>setImmediate(resolve));
 assert.equal(tasks.length,1); assert.equal(tasks[0].name,'反馈诊断');
 assert.match(tasks[0].task,/浏览器扩展/);
 for(let i=0;i<20 && (await repo.get(created.id)).stage!=='not_actionable';i++) await new Promise(resolve=>setImmediate(resolve));
 const done=await repo.get(created.id); assert.equal(done.stage,'not_actionable'); assert.equal(done.autoResumeAfterInfo,false);
});

test('a competing project claim prevents a pending clarification from being requeued',async()=>{
 const repo=new Repository();
 const ticket={schemaVersion:1,id:'pending-claim-race',parentChatId:'race-chat',projectPath:'/race-project',source:'panel',feedback:'问题',stage:'needs_info',createdAt:1,updatedAt:1,agentId:null,attempt:0,error:null,autoResumeAfterInfo:true,notes:[{at:1,kind:'user',text:'补充'}]};
 await repo.save(ticket,true);
 const original=storage.db.commit;let injected=false;
 storage.db.commit=async(mutations,guards=[])=>{
  if(!injected&&guards.some(guard=>guard.collection==='project_claims')){
   injected=true; assert.equal(await repo.claimProject('/race-project','another-ticket','other-owner'),true);
  }
  return original.call(storage.db,mutations,guards);
 };
 try {
  await assert.rejects(repo.queuePendingInfo(ticket.id,2),/version conflict/);
  assert.equal(injected,true);const retained=await repo.get(ticket.id);
  assert.equal(retained.stage,'needs_info');assert.equal(retained.autoResumeAfterInfo,true);
 } finally {storage.db.commit=original;await repo.releaseProject('/race-project','other-owner');}
});

test('status questions are not captured and the conversation snapshot counts all its own tickets',async()=>{
 const chatId='status-summary'; await handler({action:'bind_chat',chatId,enabled:true});
 const id='mwk76p6l-a6f8eec2d122e82f6c4cbd00db779199';
 await hook(event(chatId,101,`请问工单 ${id} 进度怎么样？`));
 await hook(event(chatId,102,'上次那个问题处理得怎么样了？'));
 await hook(event(chatId,103,`工单 ${id} 还是会崩溃，处理得怎么样了？`));
 const repo=new Repository(); const pending=(await repo.pendingCaptures()).filter(x=>x.value.parentChatId===chatId);
 assert.equal(pending.length,1); assert.match(pending[0].value.message.text,/崩溃/);
 for(let i=0;i<7;i++) await repo.save({schemaVersion:1,id:`summary-${i}`,parentChatId:chatId,projectPath:'/status-game',source:'panel',feedback:'private feedback',stage:i<3?'review':'completed',createdAt:i,updatedAt:i,agentId:null,attempt:0,error:null,notes:[]},true);
 await repo.save({schemaVersion:1,id:'other-summary',parentChatId:'other',projectPath:'/other',source:'panel',feedback:'other',stage:'failed',createdAt:1,updatedAt:1,agentId:null,attempt:0,error:null,notes:[]},true);
 const state=await handler({action:'conversation_status',chatId});
 assert.equal(state.total,7); assert.deepEqual(state.counts,{review:3,completed:4}); assert.equal(state.tickets.length,5);
 assert.ok(state.tickets.every(ticket=>!('feedback' in ticket))); assert.equal(state.counts.failed,undefined);
});

test('channel status is scoped to the exact sender, account, conversation and project',async()=>{
 const repo=new Repository(), parent='channel-status-parent', path='/channel-status-game';
 const source=(key,author,account='account',conversation='group',project=path)=>({parentChatId:parent,projectPath:project,receivedAt:100,ticketId:null,message:{key,channel:'qq',accountId:account,messageId:key,conversationId:conversation,authorId:author,text:'private report',timestamp:'100',context:{}}});
 for(const item of [source('alice-ticket','alice'),source('alice-pending','alice'),source('bob-ticket','bob'),source('other-account','alice','other'),source('other-project','alice','account','group','/other')]) await repo.capture(item);
 const rows=await repo.pendingCaptures();
 for(const [id,key] of [['alice-work','alice-ticket'],['bob-work','bob-ticket']]) {
  const ticket={schemaVersion:1,id,parentChatId:parent,projectPath:path,source:'qq',feedback:'private',stage:'triage',createdAt:100,updatedAt:101,agentId:null,attempt:0,error:null,notes:[]};
  await repo.publish(ticket,rows.filter(row=>row.key===key));
 }
 const scope={action:'channel_status',channel:'qq',parentChatId:parent,projectPath:path,accountId:'account',conversationId:'group',authorId:'alice'};
 const result=await handler(scope);
 assert.deepEqual(result,{pending:1,total:1,counts:{triage:1},tickets:[{id:'alice-work',stage:'triage',updatedAt:101}]});
 const other=await handler({...scope,authorId:'nobody'});
 assert.deepEqual(other,{pending:0,total:0,counts:{},tickets:[]});
 assert.doesNotMatch(JSON.stringify(result),/private|bob-work/);
});
