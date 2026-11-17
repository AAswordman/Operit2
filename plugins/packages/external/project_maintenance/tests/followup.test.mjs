import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {objectStorage} from '../../../external/qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url), storage=objectStorage();
globalThis.ToolPkg={getSpaceDataDir:()=>'/test'};globalThis.Tools={Storage:{objects:storage}};
const {Repository}=require('../dist/storage.js');
const {MaintenanceEngine}=require('../dist/engine.js');
const {CaptureCoordinator}=require('../dist/capture.js');
const repo=new Repository();let now=1000,ids=0;const tasks=[];
const engine=new MaintenanceEngine(repo,{
 async spawn(parent,name,task){const id=String(tasks.length);tasks.push({id,parent,name,task});return {id,status:'running',error:null,result:null};},
 async wait(_parent,id){return {id,status:'completed',error:null,result:{response:JSON.stringify({decision:'needs_info',reason:'which device?'})}};},
 async safeToRetry(){return false;},
},{now:()=>now,id:()=>`ticket-${++ids}`,report:e=>{throw e;},git:{async cleanHead(){return 'a'.repeat(40);}}});
const coordinator=new CaptureCoordinator(repo,engine,()=>now,e=>{throw e;});
const msg=(id,text,author='alice',channel='qq',conversation='group')=>({key:`${channel}:acct:${conversation}:${author}:${id}`,channel,accountId:'acct',messageId:id,conversationId:conversation,authorId:author,text,timestamp:id,context:{}});
async function finish(id){for(let i=0;i<100;i++){const t=await engine.get(id);if(t.stage==='needs_info' && await repo.projectClaim(t.projectPath)===null)return t;await new Promise(r=>setImmediate(r));}throw Error('ticket did not settle');}
async function submit(message,path='/game'){await coordinator.capture('parent',path,message);now+=13000;await coordinator.tick();}
test('explicit clarification and nearby fragments resume the original ticket once',async()=>{
 await submit(msg('1','游戏出问题了'));const [original]=await engine.list();await finish(original.id);
 await coordinator.capture('parent','/game',msg('2',`补充 工单：${original.id} 我用手机`));
 await coordinator.capture('parent','/game',msg('3','点击重开后分数仍然在'));
 now+=13000;await coordinator.tick();const resumed=await finish(original.id);
 assert.equal((await engine.list()).length,1);assert.equal(resumed.notes.filter(n=>n.kind==='user').length,1);
 assert.match(resumed.notes.find(n=>n.kind==='user').text,/我用手机/);assert.match(resumed.notes.find(n=>n.kind==='user').text,/点击重开/);
 assert.equal(tasks.length,2);assert.match(tasks[1].task,/我用手机/);
 await coordinator.capture('parent','/game',msg('2',`补充 工单：${original.id} 我用手机`));now+=13000;await coordinator.tick();
 assert.equal((await engine.get(original.id)).notes.filter(n=>n.kind==='user').length,1);assert.equal(tasks.length,2);
});
test('a different sender cannot attach to an existing ticket',async()=>{
 const original=(await engine.list())[0],before=original.notes.length;
 await submit(msg('4',`工单：${original.id} 另一个人的情况`,'bob'));
 const list=await engine.list();assert.equal(list.length,2);
 assert.equal((await engine.get(original.id)).notes.length,before);
 assert.match(list.find(t=>t.id!==original.id).feedback,/另一个人/);
});
test('a claim still held by a finishing run defers linking without losing the batch',async()=>{
 const original=(await engine.list()).find(t=>t.id==='ticket-1');await finish(original.id);
 await repo.claimProject('/game',original.id,'other-owner');
 await coordinator.capture('parent','/game',msg('5',`工单：${original.id} 更多信息`));now+=13000;await coordinator.tick();
 assert.equal((await repo.pendingCaptures()).length,1);assert.equal((await engine.get(original.id)).stage,'needs_info');
 await repo.releaseProject('/game','other-owner');await coordinator.tick();const done=await finish(original.id);
 assert.match(done.notes.filter(n=>n.kind==='user').at(-1).text,/更多信息/);assert.equal((await repo.pendingCaptures()).length,0);
});


test('messages from different authors in the same group become separate tickets',async()=>{
 const before=(await engine.list()).length;
 await coordinator.capture('parent','/another',msg('6','我看到闪退','alice'));
 await coordinator.capture('parent','/another',msg('7','我的画面变黑','bob'));
 now+=13000;await coordinator.tick();const tickets=(await engine.list()).filter(t=>t.projectPath==='/another');
 assert.equal((await engine.list()).length,before+2);assert.equal(tickets.length,2);
 assert.ok(tickets.some(t=>t.feedback.includes('闪退')&&!t.feedback.includes('画面变黑')));
 assert.ok(tickets.some(t=>t.feedback.includes('画面变黑')&&!t.feedback.includes('闪退')));
});

test('claim guard makes a racing executor leave both ticket and capture unchanged',async()=>{
 const ticket=await finish('ticket-1');const incoming=msg('8',`工单：${ticket.id} 再补充`);
 await coordinator.capture('parent','/game',incoming);const batch=(await repo.pendingCaptures()).filter(row=>row.key===incoming.key);
 const originalCommit=storage.db.commit.bind(storage.db);let inserted=false;
 storage.db.commit=async(mutations,guards)=>{
  if(guards?.length && !inserted){inserted=true;await repo.claimProject('/game','other-ticket','racing-owner');}
  return originalCommit(mutations,guards);
 };
 await assert.rejects(repo.publishFollowup(ticket.id,batch,'clarification',now),/version conflict/);
 storage.db.commit=originalCommit;
 assert.equal((await engine.get(ticket.id)).stage,'needs_info');assert.equal((await repo.pendingCaptures()).some(row=>row.key===incoming.key),true);
 await repo.releaseProject('/game','racing-owner');
});

test('a restart after atomic linking resumes the ticket without duplicating the note',async()=>{
 const incoming=msg('8','');const batch=(await repo.pendingCaptures()).filter(row=>row.key===incoming.key);
 const prior=(await engine.get('ticket-1')).notes.filter(note=>note.kind==='user').length;
 assert.equal(await repo.publishFollowup('ticket-1',batch,'[alice] 再补充',now),'linked');
 assert.equal((await engine.get('ticket-1')).stage,'new');
 const restarted=new CaptureCoordinator(repo,engine,()=>now,e=>{throw e;});await restarted.tick();const done=await finish('ticket-1');
 assert.equal(done.notes.filter(note=>note.kind==='user').length,prior+1);
 assert.equal((await repo.pendingCaptures()).some(row=>row.key===incoming.key),false);
});

test('clarification triages again even when the ticket has an earlier implementation',async()=>{
 const ticket=await repo.get('ticket-1');
 ticket.notes.push({at:now,kind:'implement',text:'Earlier work before requesting clarification'});
 await repo.save(ticket);
 const previous=tasks.length;
 await submit(msg('9',`工单：${ticket.id} 新的补充`));
 const settled=await finish(ticket.id);
 assert.equal(tasks.length,previous+1);
 assert.match(tasks[previous].task,/诊断并判断/);
 assert.equal(settled.forceTriage,false);
});

test('explicit same-sender feedback joins a running ticket while a foreign sender cannot attach',async()=>{
 const path='/active-clarification', first=msg('active-original','游戏中分数不对');
 await coordinator.capture('parent',path,first); const rows=(await repo.pendingCaptures()).filter(r=>r.key===first.key);
 const ticket=engine.prepare('parent',path,'qq','initial');ticket.stage='review';await repo.publish(ticket,rows);await repo.claimProject(path,ticket.id,'owner');
 const incoming=msg('active-note',`工单：${ticket.id} 横屏时也会发生`);await coordinator.capture('parent',path,incoming);
 const batch=(await repo.pendingCaptures()).filter(r=>r.key===incoming.key);
 assert.equal(await repo.publishFollowup(ticket.id,batch,'横屏时也会发生',now),'linked');
 const updated=await repo.get(ticket.id);assert.equal(updated.stage,'review');assert.equal(updated.notes.at(-1).text,'横屏时也会发生');
 const foreign=msg('active-foreign',`工单：${ticket.id} 别人的反馈`,'bob');await coordinator.capture('parent',path,foreign);
 const foreignRows=(await repo.pendingCaptures()).filter(r=>r.key===foreign.key);
 assert.equal(await repo.publishFollowup(ticket.id,foreignRows,'foreign',now),'not_found');
 await repo.releaseProject(path,'owner');
});

test('same-sender feedback during delivery remains durable until a new ticket can be made',async()=>{
 const path='/delivering-clarification', first=msg('delivery-original','游戏出错');
 await coordinator.capture('parent',path,first);const rows=(await repo.pendingCaptures()).filter(r=>r.key===first.key);
 const ticket=engine.prepare('parent',path,'qq','initial');ticket.stage='delivery';await repo.publish(ticket,rows);
 const incoming=msg('delivery-note',`工单：${ticket.id} 还有个问题`);await coordinator.capture('parent',path,incoming);
 const batch=(await repo.pendingCaptures()).filter(r=>r.key===incoming.key);
 assert.equal(await repo.publishFollowup(ticket.id,batch,'还有个问题',now),'defer');
 assert.equal((await repo.pendingCaptures()).some(r=>r.key===incoming.key),true);
 const completed=await repo.get(ticket.id);completed.stage='completed';await repo.save(completed);now+=13000;await coordinator.tick();
 const next=(await engine.list()).find(t=>t.id!==ticket.id && t.projectPath===path);
 assert.ok(next);assert.match(next.feedback,/还有个问题/);
});
