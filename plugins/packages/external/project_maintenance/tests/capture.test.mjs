import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { objectStorage } from '../../../external/qqbot/tests/storage.mjs';
const require = createRequire(import.meta.url);
const storage = objectStorage();
globalThis.ToolPkg = { getSpaceDataDir: () => '/test' };
globalThis.Tools = { Storage: { objects: storage } };
const { Repository } = require('../dist/storage.js');
const { MaintenanceEngine } = require('../dist/engine.js');
const { CaptureCoordinator } = require('../dist/capture.js');
let now = 1000, ids = 0; const tasks = [];
const repository = new Repository();
const engine = new MaintenanceEngine(repository, {
 async spawn(parent,name,task) { const id = String(tasks.length); tasks.push({parent,name,task}); return {id,status:'running',error:null,result:null}; },
 async wait(_parent,id) { return {id,status:'completed',error:null,result:{response:JSON.stringify({decision:'needs_info',reason:'more details'})}}; },
}, { now:()=>now, id:()=>`ticket-${++ids}`, report:()=>{}, git:{async cleanHead(){return "a".repeat(40);}} });
const message = (id, text) => ({ key:`qq:account:group:g:${id}`, channel:'qq',accountId:'account',messageId:id,conversationId:'g',authorId:'alice',text,timestamp:id,context:{raw:{attachments:[]}} });
const coordinator = () => new CaptureCoordinator(repository, engine, ()=>now, ()=>{});

test('durable chat fragments become one automatically started ticket and delivery retries deduplicate', async () => {
 const first = coordinator();
 assert.equal((await first.capture('parent','/sandbox',message('1','游戏好像坏了'))).saved,true);
 assert.equal((await first.capture('parent','/sandbox',message('1','游戏好像坏了'))).saved,false);
 now += 6000; await first.capture('parent','/sandbox',message('2','重开之后分数还在'));
 now += 6000; await first.tick(); assert.equal((await engine.list()).length,0);
 // Constructing a new coordinator simulates a host restart after the captures were committed.
 const resumed = coordinator(); now += 7000; await resumed.tick();
 const tickets = await engine.list(); assert.equal(tickets.length,1);
 assert.match(tickets[0].feedback,/游戏好像坏了/); assert.match(tickets[0].feedback,/重开之后分数还在/);
 assert.equal((await resumed.capture('parent','/sandbox',message('1','游戏好像坏了'))).saved,false);
 assert.equal((await repository.pendingCaptures()).length,0);
 await new Promise(resolve=>setImmediate(resolve)); assert.equal(tasks.length,1); assert.equal(tasks[0].name,'反馈诊断');
});

test('duplicate source message cannot be rebound to a different repository', async () => {
 await assert.rejects(coordinator().capture('parent','/other',message('1','游戏好像坏了')),/different project/);
});

test('continuous conversation is captured after a bounded wait even without a quiet period', async () => {
 const capture=coordinator(), start=now;
 for(let i=0;i<6;i++) { await capture.capture('parent','/continuous',message(`live-${i}`,`消息${i}`)); now+=10000; await capture.tick(); }
 assert.equal(now-start,60000);
 const tickets=(await engine.list()).filter(t=>t.projectPath==='/continuous');
 assert.equal(tickets.length,1);
 assert.match(tickets[0].feedback,/消息0/); assert.match(tickets[0].feedback,/消息5/);
});

test('an error publishing one conversation does not prevent independent feedback or recovery', async () => {
 const reports=[];
 const capture=new CaptureCoordinator(repository,engine,()=>now,e=>reports.push(e));
 await capture.capture('parent','/broken',message('broken-1','未处理的反馈'));
 await capture.capture('parent','/healthy',message('healthy-1','独立会话反馈'));
 const publish=repository.publish.bind(repository);
 repository.publish=async(ticket,rows)=>{if(ticket.projectPath==='/broken')throw new Error('injected failure'); return publish(ticket,rows);};
 try { now+=13000; await capture.tick(); }
 finally { repository.publish=publish; }
 assert.equal(reports.length,1);
 assert.equal((await engine.list()).filter(t=>t.projectPath==='/healthy').length,1);
 assert.equal((await repository.pendingCaptures()).filter(row=>row.value.projectPath==='/broken').length,1);
 await capture.tick();
 assert.equal((await engine.list()).filter(t=>t.projectPath==='/broken').length,1);
});

test('pure acknowledgement is durably suppressed and duplicate delivery remains deduplicated',async()=>{
 const capture=coordinator(), m=message('thanks-1','补充：谢谢。');
 const before=(await engine.list()).length;
 assert.equal((await capture.capture('parent','/thanks',m)).saved,true);
 now+=13000;await capture.tick();
 assert.equal((await engine.list()).length,before);
 assert.equal((await repository.pendingCaptures()).some(row=>row.key===m.key),false);
 assert.equal((await capture.capture('parent','/thanks',m)).saved,false);
 await assert.rejects(capture.capture('parent','/different',m),/different project/);
});

test('mixed meaningful feedback, explicit ticket references and media are never suppressed',async()=>{
 const capture=coordinator();
 await capture.capture('parent','/mixed',message('mixed-1','谢谢'));
 await capture.capture('parent','/mixed',message('mixed-2','重开还是有问题'));
 const media={...message('media-1','谢谢'),context:{raw:{attachments:[{filename:'bug.png'}]}}};
 await capture.capture('parent','/media',media);
 const ref=message('reference-1','工单：ticket-unknown 谢谢');
 await capture.capture('parent','/reference',ref);
 const unknown={...message('unknown-1','谢谢'),context:{}};
 await capture.capture('parent','/unknown',unknown);
 now+=13000;await capture.tick();
 const list=await engine.list();
 assert.match(list.find(t=>t.projectPath==='/mixed').feedback,/重开还是有问题/);
 for(const path of ['/media','/reference','/unknown']) assert.ok(list.some(t=>t.projectPath===path),path);
});

test('WeChat text acknowledgements are suppressed while non-text items remain actionable',async()=>{
 const capture=coordinator();
 const wechat=(id,items)=>({...message(id,'谢谢'),channel:'wechat',key:'wechat-'+id,context:{items}});
 await capture.capture('parent','/wx-thanks',wechat('text',[{type:1,text_item:{text:'谢谢'}}]));
 await capture.capture('parent','/wx-media',wechat('image',[{type:1,text_item:{text:'谢谢'}},{type:2,image_item:{}}]));
 now+=13000;await capture.tick();const list=await engine.list();
 assert.equal(list.some(t=>t.projectPath==='/wx-thanks'),false);
 assert.equal(list.some(t=>t.projectPath==='/wx-media'),true);
});

test('failure recording acknowledgement keeps the message pending for a later retry',async()=>{
 const reports=[];const capture=new CaptureCoordinator(repository,engine,()=>now,e=>reports.push(e));
 const m=message('ack-failure','收到');await capture.capture('parent','/ack-failure',m);
 const commit=storage.db.commit.bind(storage.db);let injected=false;
 storage.db.commit=async(mutations,guards)=>{if(!injected&&mutations.some(item=>item.value?.ignored==='acknowledgement')){injected=true;throw Error('storage unavailable');}return commit(mutations,guards);};
 try {now+=13000;await capture.tick();} finally {storage.db.commit=commit;}
 assert.equal(reports.length,1);
 assert.equal((await repository.pendingCaptures()).some(row=>row.key===m.key),true);
 await capture.tick();
 assert.equal((await repository.pendingCaptures()).some(row=>row.key===m.key),false);
 assert.equal((await engine.list()).some(t=>t.projectPath==='/ack-failure'),false);
});

test('one sender can report two distinct problems in a single burst without mixing their details',async()=>{
 const capture=coordinator(), path='/two-topics';
 for(const [id,text] of [['topic-1','游戏重开后分数没清零'],['topic-2','我点了重新开始以后还是 5 分'],['topic-3','另外一个问题：声音关不掉'],['topic-4','把音量调到零也还有声音'],['topic-5','另一个问题是按钮文字太小'],['topic-6','还有一个细节，是在手机上']]) {
  await capture.capture('parent',path,message(id,text)); now+=1000;
 }
 now+=13000; await capture.tick();
 const list=(await engine.list()).filter(ticket=>ticket.projectPath===path);
 assert.equal(list.length,3);
 const score=list.find(ticket=>ticket.feedback.includes('分数没清零'));
 const audio=list.find(ticket=>ticket.feedback.includes('声音关不掉'));
 assert.ok(score);assert.ok(audio);
 assert.match(score.feedback,/还是 5 分/);assert.doesNotMatch(score.feedback,/声音/);
 assert.match(audio.feedback,/音量调到零/);assert.doesNotMatch(audio.feedback,/分数/);
 const button=list.find(ticket=>ticket.feedback.includes("按钮文字太小"));
 assert.ok(button);assert.match(button.feedback,/还有一个细节/);assert.doesNotMatch(audio.feedback,/按钮文字/);
});
