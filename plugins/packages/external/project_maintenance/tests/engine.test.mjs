import test from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { MaintenanceEngine } = require('../dist/engine.js');
const copy = value => structuredClone(value);
const git={async cleanHead(){return "a".repeat(40);},async snapshot(){return {head:"a".repeat(40),tree:"c".repeat(40)};},async verify(_path,reviewed){return {head:"b".repeat(40),parent:reviewed.head,reviewedTree:reviewed.tree,trackedClean:true,untrackedClean:true};}};
class Store {
  records = new Map(); claims = new Map();
  async claimProject(path,id,owner){const old=this.claims.get(path); if(old && (old.id!==id || old.owner!==owner)) return false; this.claims.set(path,{id,owner}); return true;}
  async releaseProject(path,owner){if(this.claims.get(path)?.owner===owner) this.claims.delete(path);}
  async reclaimFailed(id,expected,owner,now){
   const t=await this.get(id), claim=this.claims.get(t.projectPath);
   if(t.stage!=='failed'||t.agentId!==expected||claim?.id!==id) throw new Error('changed');
   const resumeReview=t.notes.some(n=>n.kind==='implement'); t.stage=resumeReview?'review':'triage'; t.agentId=null;t.error=null;t.updatedAt=now;
   this.claims.set(t.projectPath,{id,owner}); await this.save(t); return {ticket:t,resumeReview};
  }
  async list(){ return [...this.records.values()].map(copy); }
  async get(id){ return this.records.has(id) ? copy(this.records.get(id)) : null; }
  async save(ticket, create=false){ if (create && this.records.has(ticket.id)) throw new Error('duplicate'); this.records.set(ticket.id, copy(ticket)); }
}
function worker(responses) {
  const tasks = [];
  return { tasks, async safeToRetry(){return false;}, async spawn(parent, name, task){ const id = String(tasks.length); tasks.push({parent,name,task}); return {id,status:'running',error:null,result:null}; },
    async wait(_parent,id){ const response = responses[Number(id)]; if (response instanceof Error) throw response; return {id,status:'completed',error:null,result:{response: JSON.stringify(response)}}; } };
}
async function finished(store,id){
  for(let i=0;i<200;i++){ const ticket=await store.get(id); if (['completed','failed','interrupted','needs_info','not_actionable'].includes(ticket.stage)) return ticket; await new Promise(resolve=>setTimeout(resolve,1)); }
  throw new Error('workflow did not settle');
}
function make(responses){ const store=new Store(), workers=worker(responses); let n=0; const engine=new MaintenanceEngine(store,workers,{now:()=>++n,id:()=>`ticket-${n}`,report:e=>{throw e;},git}); return {store,workers,engine}; }
test('feedback is diagnosed, failed acceptance returns to implementation, and delivery is recorded', async()=>{
 const x=make([{decision:'fix',reason:'reproduced'}, {summary:'patch 1'}, {decision:'fail',reason:'test still fails'}, {summary:'patch 2'}, {decision:'pass',reason:'test passes'}, {summary:'local commit'}]);
 const t=await x.engine.create('chat','/sandbox','chat','刚玩了一下，重开分数还在……'); await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed'); assert.equal(done.attempt,2); assert.deepEqual(x.workers.tasks.map(t=>t.name),['反馈诊断','实施修复','独立验收','实施修复','独立验收','交付变更']);
 assert.match(x.workers.tasks[3].task,/test still fails/);
});
test('host capacity failure in a failed review stops repeated implementation and retains the project', async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'fail',reason:'could not run tests'}]);
 const wait=x.workers.wait.bind(x.workers); x.workers.wait=async(parent,id)=>{const result=await wait(parent,id); return id==='2' ? {...result,diagnostics:['terminal_capacity_exhausted']} : result;};
 const t=await x.engine.create('chat','/sandbox','chat','score is wrong'); await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'interrupted'); assert.equal(done.attempt,1); assert.match(done.error,/终端容量耗尽/);
 assert.deepEqual(x.workers.tasks.map(t=>t.name),['反馈诊断','实施修复','独立验收']); assert.ok(x.store.claims.has('/sandbox'));
 assert.equal(done.notes.filter(n=>n.kind==='review').length,1);
});

test('a successful review is not rejected merely because an earlier terminal failed', async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, {summary:'commit'}]);
 const wait=x.workers.wait.bind(x.workers); x.workers.wait=async(parent,id)=>{const result=await wait(parent,id); return id==='2' ? {...result,diagnostics:['terminal_capacity_exhausted']} : result;};
 const t=await x.engine.create('chat','/sandbox','chat','bug'); await x.engine.start(t.id); assert.equal((await finished(x.store,t.id)).stage,'completed');
});

test('cleanup runs after each completed agent and retained sessions do not replay work', async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, {summary:'commit'}]); const cleaned=[];
 x.workers.cleanup=async(parent,id)=>{cleaned.push([parent,id]); if(id==='1') throw Error('host unavailable'); return {closed:['session'],retained:id==='2'?['busy']:[]};};
 const t=await x.engine.create('chat','/sandbox','chat','bug'); await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed'); assert.deepEqual(cleaned.map(x=>x[1]),['0','1','2','3']); assert.equal(done.notes.filter(n=>n.kind==='terminal_cleanup').length,2);
 assert.deepEqual(x.workers.tasks.map(t=>t.name),['反馈诊断','实施修复','独立验收','交付变更']);
});

test('insufficient information never triggers implementation', async()=>{
 const x=make([{decision:'needs_info',reason:'which version?'}]); const t=await x.engine.create('chat','/sandbox','chat','它坏了'); await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'needs_info'); assert.equal(x.workers.tasks.length,1);
});
test('simultaneous starts cannot launch duplicate agents', async()=>{
 const x=make([{decision:'not_actionable',reason:'usage'}]); const t=await x.engine.create('chat','/sandbox','chat','怎么操作');
 const results=await Promise.allSettled([x.engine.start(t.id),x.engine.start(t.id)]); assert.equal(results.filter(x=>x.status==='fulfilled').length,1); await finished(x.store,t.id); assert.equal(x.workers.tasks.length,1);
});
test('restart marks unfinished work interrupted without spawning it again',async()=>{
 const x=make([]); const t=await x.engine.create('chat','/sandbox','chat','bug'); t.stage='implement'; t.agentId='existing'; await x.store.save(t);
 await x.engine.recover(); const restored=await x.store.get(t.id); assert.equal(restored.stage,'interrupted'); assert.equal(restored.agentId,'existing'); assert.equal(x.workers.tasks.length,0);
});
test('agent errors are visible rather than reported as success',async()=>{
 const x=make([new Error('network failure')]); const t=await x.engine.create('chat','/sandbox','chat','bug'); await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'interrupted'); assert.equal(done.agentId,'0'); assert.match(done.error,/network failure/);
 await assert.rejects(x.engine.start(t.id), /cannot be started/); assert.equal(x.workers.tasks.length,1);
});
test('retry reviews an existing uncommitted fix before deciding it needs no work', async()=>{
 const x=make([{decision:'pass',reason:'verified'}, {summary:'local commit'}]);
 const t=await x.engine.create('chat','/sandbox','chat','restart bug');
 t.stage='not_actionable'; t.attempt=2; t.notes.push({at:0,kind:'git_baseline',text:'a'.repeat(40)},{at:1,kind:'implement',text:'existing patch'}); await x.store.save(t);
 await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed'); assert.deepEqual(x.workers.tasks.map(t=>t.name),['独立验收','交付变更']);
});
test('an optimistic conflict reloads a ticket before applying an update', async()=>{
 const x=make([]); const ticket=await x.engine.create('chat','/sandbox','chat','bug');
 const save=x.store.save.bind(x.store); let conflicted=false;
 x.store.save=async (value,create)=>{ if(!create && !conflicted){ conflicted=true; const other=await x.store.get(value.id); other.notes.push({at:2,kind:'user',text:'concurrent'}); await save(other); throw new Error('Storage version conflict: tickets/id'); } return save(value,create); };
 await x.engine.addNote(ticket.id,'mine');
 assert.deepEqual((await x.store.get(ticket.id)).notes.map(note=>note.text),['bug','concurrent','mine']);
});

test('a lost spawn response and a failed agent-id save cannot trigger a second agent', async()=>{
 for (const failure of ['spawn','save']) {
  const x=make([]); const t=await x.engine.create('chat','/sandbox','chat','bug');
  if(failure==='spawn') x.workers.spawn=async()=>{ x.workers.tasks.push({name:'unknown'}); throw new Error('response lost'); };
  else { const save=x.store.save.bind(x.store); let rejected=false; x.store.save=async(value,create)=>{ if(value.agentId==='0'&&!rejected){rejected=true; throw new Error('disk error');} return save(value,create); }; }
  await x.engine.start(t.id); const done=await finished(x.store,t.id);
  assert.equal(done.stage,'interrupted'); assert.equal(done.agentId, failure==='save'?'0':null);
  await assert.rejects(x.engine.start(t.id), /cannot be started/); assert.equal(x.workers.tasks.length,1);
 }
});

test('invalid delivery response requires inspection before retrying a possible commit', async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, null]);
 const t=await x.engine.create('chat','/sandbox','chat','bug'); await x.engine.start(t.id);
 const done=await finished(x.store,t.id); assert.equal(done.stage,'interrupted');
 await assert.rejects(x.engine.start(t.id), /cannot be started/); assert.equal(x.workers.tasks.length,4);
});

test('one project is reserved across engine instances while another project can run', async()=>{
 const store=new Store(), tasks=[]; let release; const held=new Promise(r=>release=r); let seq=0;
 const workers={async spawn(parent,name){const id=String(tasks.length);tasks.push({parent,name});return {id,status:'running',error:null,result:null};},async wait(_parent,id){if(id==='0') await held;return {id,status:'completed',error:null,result:{response:'{"decision":"not_actionable"}'}};}};
 const config={now:()=>++seq,id:()=>`unique-${++seq}`,report:e=>{throw e;},git};
 const a=new MaintenanceEngine(store,workers,config), b=new MaintenanceEngine(store,workers,config);
 const first=await a.create('chat','/same','chat','first'), next=await b.create('chat','/same','chat','second'), other=await b.create('chat','/different','chat','third');
 await a.start(first.id); await assert.rejects(b.start(next.id),/项目已有维护工单/);
 await b.start(other.id); await finished(store,other.id); assert.equal(tasks.length,2);
 release(); await finished(store,first.id);
 // The terminal ticket's reservation is released after the durable result.
 for(let i=0;i<50 && store.claims.has('/same');i++) await new Promise(r=>setTimeout(r,1));
 await b.start(next.id); await finished(store,next.id); assert.equal(tasks.length,3);
});

test('unknown execution keeps the project reserved after the ticket stops',async()=>{
 const x=make([new Error('lost connection')]); const first=await x.engine.create('chat','/same','chat','first');
 await x.engine.start(first.id); await finished(x.store,first.id);
 const other=await x.engine.create('chat','/same','chat','second');
 await assert.rejects(x.engine.start(other.id),/项目已有维护工单/); assert.equal(x.workers.tasks.length,1);
});


test('delivery without independent Git evidence is interrupted and keeps its project claim',async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, {summary:'claim of commit'}]);
 const t=await x.engine.create('chat','/sandbox','chat','bug');
 x.engine.config.git = {...git, async verify(){throw new Error('Git HEAD did not change during delivery');}};
 await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'interrupted'); assert.match(done.error,/HEAD did not change/); assert.ok(x.store.claims.has('/sandbox'));
});


test('failed work can reclaim after restart only when the exact agent is verified and the owner matches',async()=>{
 const x=make([{decision:'pass'}, {summary:'commit'}]);
 const t=await x.engine.create('chat','/reclaim','chat','bug'); t.stage='failed'; t.agentId='old-agent'; t.notes.push({at:0,kind:'git_baseline',text:'a'.repeat(40)},{at:1,kind:'implement',text:'patch'}); await x.store.save(t);
 await x.store.claimProject(t.projectPath,t.id,'old-owner');
 await assert.rejects(x.engine.start(t.id),/项目已有维护工单/); assert.equal((await x.store.get(t.id)).stage,'failed');
 x.workers.safeToRetry=async(parent,id)=>parent==='chat'&&id==='old-agent';
 await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed'); assert.deepEqual(x.workers.tasks.map(x=>x.name),['独立验收','交付变更']);
});
test('failed work with a missing agent cannot reacquire an absent claim',async()=>{
 const x=make([]); const t=await x.engine.create('chat','/no-claim','chat','bug'); t.stage='failed';t.agentId=null;await x.store.save(t);
 await assert.rejects(x.engine.start(t.id),/项目已有维护工单/); assert.equal(x.workers.tasks.length,0);
});


test('review mutations stop the workflow before delivery and retain the claim',async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, {summary:'commit'}]);
 let snapshots=0;
 x.engine.config.git={...git,async snapshot(){ snapshots++; return {head:'a'.repeat(40),tree:(snapshots===1?'c':'d').repeat(40)}; }};
 const t=await x.engine.create('chat','/changed-review','chat','bug');
 await x.engine.start(t.id); const done=await finished(x.store,t.id);
 assert.equal(done.stage,'failed'); assert.match(done.error,/验收期间仓库内容发生变化/);
 assert.equal(x.workers.tasks.some(t=>t.name==='交付变更'),false); assert.ok(x.store.claims.has('/changed-review'));
});


test('preflight stops before agents on a dirty repository and allows a clean retry',async()=>{
 const x=make([{decision:'needs_info'}]); let clean=false;
 x.engine.config.git={...git,async cleanHead(){if(!clean)throw new Error('Repository contains existing tracked or untracked changes');return 'a'.repeat(40);}};
 const t=await x.engine.create('chat','/dirty','chat','bug');await x.engine.start(t.id);
 const blocked=await finished(x.store,t.id);assert.equal(blocked.stage,'needs_info');assert.match(blocked.error,/existing tracked or untracked/);
 assert.equal(x.workers.tasks.length,0);assert.equal(x.store.claims.has('/dirty'),false);
 clean=true;await x.engine.start(t.id);await finished(x.store,t.id);assert.equal(x.workers.tasks.length,1);
});

test('triage mutations interrupt the workflow before implementation',async()=>{
 const x=make([{decision:'fix'}]);let checks=0;
 x.engine.config.git={...git,async cleanHead(){if(++checks===2)throw new Error('dirty');return 'a'.repeat(40);}};
 const t=await x.engine.create('chat','/triage-changed','chat','bug');await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'interrupted');assert.match(done.error,/诊断期间仓库发生变化/);assert.equal(x.workers.tasks.length,1);assert.ok(x.store.claims.has('/triage-changed'));
});

test('a commit during implementation cannot become the delivery baseline',async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}]);
 x.engine.config.git={...git,async snapshot(){return {head:'b'.repeat(40),tree:'c'.repeat(40)};}};
 const t=await x.engine.create('chat','/committed','chat','bug');await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'failed');assert.match(done.error,/Git HEAD 已改变/);assert.equal(x.workers.tasks.length,2);assert.ok(x.store.claims.has('/committed'));
});

test('old retry without recorded baseline is interrupted rather than trusting a dirty tree',async()=>{
 const x=make([]);const t=await x.engine.create('chat','/legacy','chat','bug');t.stage='not_actionable';t.notes.push({at:1,kind:'implement',text:'old patch'});await x.store.save(t);
 await x.engine.start(t.id);const done=await finished(x.store,t.id);assert.equal(done.stage,'interrupted');assert.match(done.error,/缺少可信的仓库基线/);assert.equal(x.workers.tasks.length,0);
});

test('a reported PR is independently checked at the verified commit before completion',async()=>{
 const url='https://github.com/example/game/pull/7';const x=make([{decision:'fix'},{summary:'patch'},{decision:'pass'},{summary:'created draft',pullRequestUrl:url}]);
 let observed; x.engine.config.git={...git,async verifyPullRequest(path,pr,head){observed={path,pr,head};return {url:pr,number:7,head,state:'OPEN'};}};
 const t=await x.engine.create('chat','/sandbox','chat','bug');await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed');assert.deepEqual(observed,{path:'/sandbox',pr:url,head:'b'.repeat(40)});
 assert.equal(JSON.parse(done.notes.find(n=>n.kind==='delivery_verification').text).pullRequest.url,url);
});

test('unverifiable reported PR interrupts delivery and retains the project reservation',async()=>{
 const url='https://github.com/example/game/pull/7';const x=make([{decision:'fix'},{summary:'patch'},{decision:'pass'},{summary:`created ${url}`}]);
 x.engine.config.git={...git,async verifyPullRequest(){throw Error('PR head does not match');}};
 const t=await x.engine.create('chat','/sandbox','chat','bug');await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'interrupted');assert.match(done.error,/PR head does not match/);assert.ok(x.store.claims.has('/sandbox'));
});

test('missing trailing optional PR value is repaired visibly while Git still verifies',async()=>{
 const x=make([{decision:'fix'},{summary:'patch'},{decision:'pass'},{}]);const wait=x.workers.wait.bind(x.workers);
 x.workers.wait=async(parent,id)=>id==='3'?{id,status:'completed',error:null,result:{response:'{"summary":"local commit", "pullRequestUrl":}'}}:wait(parent,id);
 const t=await x.engine.create('chat','/sandbox','chat','bug');await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed');assert.ok(done.notes.some(n=>n.kind==='delivery_format_recovery'));
 assert.equal(JSON.parse(done.notes.find(n=>n.kind==='delivery_verification').text).pullRequest,null);
});

test('other malformed delivery JSON still interrupts and cannot trigger another worker',async()=>{
 const x=make([{decision:'fix'},{summary:'patch'},{decision:'pass'},{}]);const wait=x.workers.wait.bind(x.workers);
 x.workers.wait=async(parent,id)=>id==='3'?{id,status:'completed',error:null,result:{response:'{"summary":, "pullRequestUrl":}'}}:wait(parent,id);
 const t=await x.engine.create('chat','/sandbox','chat','bug');await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'interrupted');assert.equal(x.workers.tasks.length,4);assert.ok(x.store.claims.has('/sandbox'));
});

test('new feedback during diagnosis is considered before a no-fix decision',async()=>{
 const x=make([{decision:'not_actionable',reason:'old information'}, {decision:'needs_info',reason:'clarification considered'}]);
 const t=await x.engine.create('chat','/sandbox','chat','initial feedback');const wait=x.workers.wait.bind(x.workers);
 x.workers.wait=async(parent,id)=>{if(id==='0')await x.engine.addNote(t.id,'补充：手机上也能复现');return wait(parent,id);};
 await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'needs_info');assert.equal(x.workers.tasks.length,2);assert.match(x.workers.tasks[1].task,/手机上也能复现/);
});

test('feedback during review triggers a fresh independent review before delivery',async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, {decision:'pass'}, {summary:'commit'}]);
 const t=await x.engine.create('chat','/sandbox','chat','initial feedback');const wait=x.workers.wait.bind(x.workers);
 x.workers.wait=async(parent,id)=>{if(id==='2')await x.engine.addNote(t.id,'补充：重复点击也要清零');return wait(parent,id);};
 await x.engine.start(t.id);const done=await finished(x.store,t.id);
 assert.equal(done.stage,'completed');assert.deepEqual(x.workers.tasks.map(v=>v.name),['反馈诊断','实施修复','独立验收','独立验收','交付变更']);
 assert.match(x.workers.tasks[3].task,/重复点击也要清零/);
});

test('notes arriving once delivery starts are rejected instead of silently marked reviewed',async()=>{
 const x=make([{decision:'fix'}, {summary:'patch'}, {decision:'pass'}, {summary:'commit'}]);
 const t=await x.engine.create('chat','/sandbox','chat','initial feedback');const wait=x.workers.wait.bind(x.workers);let rejected=false;
 x.workers.wait=async(parent,id)=>{if(id==='3'){await assert.rejects(x.engine.addNote(t.id,'late feedback'),/新工单/);rejected=true;}return wait(parent,id);};
 await x.engine.start(t.id);const done=await finished(x.store,t.id);assert.equal(done.stage,'completed');assert.ok(rejected);
});
