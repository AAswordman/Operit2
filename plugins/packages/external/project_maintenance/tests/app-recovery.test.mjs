import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { objectStorage } from '../../../external/qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url), storage=objectStorage();
globalThis.ToolPkg={getSpaceDataDir:()=>'/recovery-test'};
const rows=Array.from({length:205},(_,i)=>({sender:'ai',timestamp:1000+i,content:'assistant'}));
rows[2]={sender:'user',timestamp:900,content:'before binding'};
rows[3]={sender:'user',timestamp:1001,content:'游戏有点怪'};
rows[150]={sender:'user',timestamp:1002,content:'重开后分数还在'};
// A filtered summary makes a valid page short, but there are later messages.
rows[20]={sender:'summary',timestamp:1020,content:'hidden'};
let fail=true, ranges=[];
globalThis.Tools={Storage:{objects:storage},Chat:{async listChats(){return {chats:[{id:'feedback',messageCount:rows.length}]};},async getMessagesRange(_id,{start,end}){ranges.push(start);if(fail){fail=false;throw Error('temporary outage');}return {messages:rows.slice(start,end+1).filter(x=>x.sender!=='summary')};}}};
const {Repository}=require('../dist/storage.js');
const {CaptureCoordinator}=require('../dist/capture.js');
const {MaintenanceEngine}=require('../dist/engine.js');
const {AppRecovery,isStatusQuestion}=require('../dist/app-recovery.js');
test('missed app notifications are replayed after a transient error, with boundary and deduplication',async()=>{
 const repository=new Repository(); await repository.bindChat('feedback','/game',1000);
 const tasks=[]; let now=2000;
 const engine=new MaintenanceEngine(repository,{async spawn(parent,name,task){tasks.push({parent,name,task});return {id:'a',status:'running',error:null,result:null};},async wait(){return {id:'a',status:'completed',error:null,result:{response:'{"decision":"needs_info"}'}};}},{now:()=>now,id:()=>`ticket-${now}`,report:e=>{throw e;},git:{async cleanHead(){return "a".repeat(40);}}});
 const capture=new CaptureCoordinator(repository,engine,()=>now,e=>{throw e;}); const errors=[];
 const recovery=new AppRecovery(repository,capture,e=>errors.push(String(e)));
 await recovery.run(); assert.match(errors[0],/temporary outage/); assert.equal((await repository.pendingCaptures()).length,0);
 await recovery.run(); await recovery.run(); assert.ok(ranges.includes(100));
 assert.equal((await repository.pendingCaptures()).length,2);
 now+=13000; await capture.tick(); const [ticket]=await engine.list();
 assert.match(ticket.feedback,/游戏有点怪/); assert.match(ticket.feedback,/重开后分数还在/); assert.doesNotMatch(ticket.feedback,/before binding/);
 await new Promise(resolve=>setImmediate(resolve)); assert.equal(tasks.length,1);
 await repository.bindChat('feedback',null,3000); rows.push({sender:'user',timestamp:3001,content:'disabled'});
 await recovery.run(); assert.equal((await repository.pendingCaptures()).length,0);
});

test('plain status questions do not become feedback, while mixed bug reports remain eligible',()=>{
 for(const text of ['现在工单进度怎么样？','请问维护任务状态如何？','修好了吗？','这个工单处理到哪一步了？','请问工单 mwk76p6l-a6f8eec2d122e82f6c4cbd00db779199 进度怎么样？','上次那个问题处理得怎么样了？']) assert.equal(isStatusQuestion(text),true,text);
 for(const text of ['游戏进度丢失了','分数没清零，工单进度怎么样？','刚才按钮坏了','请帮我修复这个问题','工单进度显示错误','工单 mwk76p6l-a6f8eec2d122e82f6c4cbd00db779199 还是会崩溃，处理得怎么样了？']) assert.equal(isStatusQuestion(text),false,text);
});

test('changing a host workspace resets the replay boundary instead of importing old feedback',async()=>{
 const repository=new Repository();await repository.bindChat('rebound','/old-game',1000);
 const errors=[],arrivals=[];
 const recovery=new AppRecovery(repository,{async capture(...args){arrivals.push(args);}},error=>errors.push(error),async()=>'/new-game');
 const before=Date.now();await recovery.run();
 const binding=await repository.chatBinding('rebound');
 assert.equal(binding.projectPath,'/new-game');assert.ok(binding.since>=before);
 assert.equal(arrivals.length,0);assert.deepEqual(errors,[]);
});
