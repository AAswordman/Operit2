import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {objectStorage} from '../../../external/qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url),storage=objectStorage();
globalThis.ToolPkg={getSpaceDataDir:()=>'/test'};globalThis.Tools={Storage:{objects:storage}};
const {Repository}=require('../dist/storage.js'),{MaintenanceEngine}=require('../dist/engine.js'),{CaptureCoordinator}=require('../dist/capture.js');
let now=10000,seq=0;const reports=[],tasks=[];const repo=new Repository();
const engine=new MaintenanceEngine(repo,{
 async spawn(parent,name,task){const id=String(tasks.length);tasks.push({id,task});return {id,status:'running',error:null,result:null};},
 async wait(_parent,id){return {id,status:'completed',error:null,result:{response:JSON.stringify({decision:'needs_info',reason:'which steps?'})}};},
 async safeToRetry(){return false;}
},{now:()=>now,id:()=>`implicit-${++seq}`,report:e=>reports.push(e),git:{async cleanHead(){return 'a'.repeat(40);}}});
const capture=new CaptureCoordinator(repo,engine,()=>now,e=>reports.push(e));
const msg=(id,text,author='alice')=>({key:`qq:${author}:${id}`,channel:'qq',accountId:'account',messageId:id,conversationId:'group',authorId:author,text,timestamp:id,context:{raw:{}}});
async function settled(id){for(let i=0;i<100;i++){const t=await engine.get(id);if(t.stage==='needs_info' && await repo.projectClaim(t.projectPath)===null)return t;await new Promise(r=>setImmediate(r));}throw Error('ticket did not settle');}
async function send(id,text,path='/implicit',author='alice'){await capture.capture('parent',path,msg(id,text,author));now+=13000;await capture.tick();}

test('same author can explicitly supplement the one recent ticket without a number',async()=>{
 await send('a','游戏出了问题');const original=(await engine.list())[0];await settled(original.id);
 await send('b','补充一下：手机上重开后分数没清零');const result=await settled(original.id);
 assert.equal((await engine.list()).length,1);assert.match(result.notes.find(n=>n.kind==='user').text,/手机上/);
 assert.equal(result.notes.filter(n=>n.kind==='triage').length,2);
});

test('different authors and ordinary new feedback do not attach',async()=>{
 const original=await settled('implicit-1'),before=original.notes.length;
 await send('other','补充：我的设备也有问题','/implicit','bob');
 await send('new','现在还有闪退','/independent','alice');
 assert.equal((await engine.get(original.id)).notes.length,before);
 assert.ok((await engine.list()).some(t=>t.projectPath==='/independent'));
 assert.ok((await engine.list()).some(t=>t.feedback.includes('我的设备')));
});

test('ambiguous open tickets from the same author are never inferred',async()=>{
 await send('first','第一个问题','/ambiguous');let list=(await engine.list()).filter(t=>t.projectPath==='/ambiguous');await settled(list[0].id);
 await send('second','另外一个问题','/ambiguous');list=(await engine.list()).filter(t=>t.projectPath==='/ambiguous');await settled(list.find(t=>t.id!==list[0].id)?.id ?? list[0].id);
 // Wait until both projects claims have cleared before another automatic attempt.
 for(const t of list) await settled(t.id);
 await send('ambiguous','补充一下：手机上出现这个问题','/ambiguous');
 assert.equal((await engine.list()).filter(t=>t.projectPath==='/ambiguous').length,3);
});

test('old clarification requests are not automatically reopened',async()=>{
 await send('old','问题还没描述清楚','/old');const old=(await engine.list()).find(t=>t.projectPath==='/old');await settled(old.id);
 now+=31*60*1000;await send('old-follow','补充：现在才发现','/old');
 assert.equal((await engine.list()).filter(t=>t.projectPath==='/old').length,2);
});

test('a concurrently published competing ticket invalidates inference and retains the capture',async()=>{
 await send('race-origin','问题未说明','/race');const old=(await engine.list()).find(t=>t.projectPath==='/race');await settled(old.id);
 const incoming=msg('race-follow','补充：我用的是手机');await capture.capture('parent','/race',incoming);now+=13000;
 const realCommit=storage.db.commit.bind(storage.db);let injected=false;
 storage.db.commit=async(mutations,guards)=>{
  if(!injected&&guards?.some(g=>g.collection==='capture_epoch')){
   injected=true;
   const competing=engine.prepare('parent','/race','qq','新问题');
   const competingMessage=msg('racing-new','新问题');
   await capture.capture('parent','/race',competingMessage);
   const row=(await repo.pendingCaptures()).find(x=>x.key===competingMessage.key);
   await repo.publish(competing,[row]);
  }
  return realCommit(mutations,guards);
 };
 try {await capture.tick();} finally {storage.db.commit=realCommit;}
 assert.equal(injected,true);assert.equal(reports.length,1);
 assert.equal((await repo.pendingCaptures()).some(x=>x.key===incoming.key),true);
 assert.equal((await engine.get(old.id)).notes.some(n=>n.kind==='user'),false);
 await capture.tick();
 assert.equal((await engine.list()).filter(t=>t.projectPath==='/race').length,3);
});

test('a supplement joins the open issue while an explicit new topic starts another ticket',async()=>{
 await send('new-topic-origin','问题需要补充信息','/new-topic');const first=(await engine.list()).find(t=>t.projectPath==='/new-topic');await settled(first.id);
 await capture.capture('parent','/new-topic',msg('topic-part-1','补充一下：这是手机'));
 await capture.capture('parent','/new-topic',msg('topic-part-2','另外一个新问题是背景变黑了'));
 now+=13000;await capture.tick();
 assert.equal((await engine.list()).filter(t=>t.projectPath==='/new-topic').length,2);
 const updated=await engine.get(first.id);
 assert.match(updated.notes.find(n=>n.kind==='user').text,/这是手机/);
 assert.doesNotMatch(updated.notes.find(n=>n.kind==='user').text,/背景变黑/);
 const next=(await engine.list()).find(t=>t.projectPath==='/new-topic' && t.id!==first.id);
 assert.match(next.feedback,/背景变黑/);
});

test('a clear nearby supplement joins the one matching active ticket',async()=>{
 const path='/active-implicit',original=msg('active-start','游戏有问题');await capture.capture('parent',path,original);
 const rows=(await repo.pendingCaptures()).filter(row=>row.key===original.key);
 const ticket=engine.prepare('parent',path,'qq','游戏有问题');ticket.stage='implement';await repo.publish(ticket,rows);await repo.claimProject(path,ticket.id,'active-owner');
 await send('active-extra','补充一下：横屏也会出现',path);
 const current=await repo.get(ticket.id);assert.equal(current.stage,'implement');assert.match(current.notes.at(-1).text,/横屏也会出现/);
 assert.equal((await engine.list()).filter(t=>t.projectPath===path).length,1);await repo.releaseProject(path,'active-owner');
});
