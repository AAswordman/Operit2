import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdtemp,writeFile,rm,readFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {exec,execFileSync} from 'node:child_process';
import {promisify} from 'node:util';
const require=createRequire(import.meta.url);
const {MaintenanceEngine}=require('../dist/engine.js');
const {GitVerifier}=require('../dist/git-verifier.js');
const shell=promisify(exec);
const run=async command=>{try {const r=await shell(command,{timeout:10000});return {output:r.stdout,exitCode:0,timedOut:false};} catch(e){return {output:e.stdout??'',exitCode:e.code??1,timedOut:!!e.killed};}};
const git=(dir,...args)=>execFileSync('git',['-C',dir,...args],{encoding:'utf8'}).trim();
class Store {
 records=new Map(); claims=new Map();
 async list(){return [...this.records.values()].map(structuredClone);}
 async get(id){return this.records.has(id)?structuredClone(this.records.get(id)):null;}
 async save(ticket){this.records.set(ticket.id,structuredClone(ticket));}
 async claimProject(path,id,owner){if(this.claims.has(path))return false;this.claims.set(path,{id,owner});return true;}
 async releaseProject(path,owner){if(this.claims.get(path)?.owner===owner)this.claims.delete(path);}
 async reclaimFailed(){throw new Error('not used');}
}
async function waitDone(store,id){for(let i=0;i<1000;i++){const t=await store.get(id);if(['completed','failed','interrupted'].includes(t.stage))return t;await new Promise(r=>setTimeout(r,5));}throw new Error('timed out');}
for(const altered of [false,true]) test(`full maintenance pipeline ${altered?'rejects altered delivery':'accepts the reviewed commit'}`,async t=>{
 const dir=await mkdtemp(join(tmpdir(),'maintenance-flow-'));t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q');git(dir,'config','user.name','Tester');git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'bug');git(dir,'add','game.txt');git(dir,'commit','-qm','initial');
 const store=new Store(), tasks=[];
 const workers={async spawn(_parent,name){tasks.push(name);return {id:String(tasks.length-1)};},async wait(_parent,id){
  const name=tasks[Number(id)]; let response;
  if(name==='反馈诊断')response={decision:'fix'};
  else if(name==='实施修复'){await writeFile(join(dir,'game.txt'),'fixed');await writeFile(join(dir,'new-test.txt'),'test');response={summary:'fixed'};}
  else if(name==='独立验收')response={decision:'pass'};
  else {if(altered)await writeFile(join(dir,'game.txt'),'unreviewed');git(dir,'add','-A');git(dir,'commit','-qm','fix');response={summary:'committed'};}
  return {id,status:'completed',error:null,result:{response:JSON.stringify(response)}};
 },async safeToRetry(){return false;}};
 const engine=new MaintenanceEngine(store,workers,{id:()=> 'ticket',now:()=>Date.now(),report:()=>{},git:new GitVerifier(run)});
 const ticket=await engine.create('chat',dir,'simulated','游戏计分有问题');await engine.start(ticket.id);
 const result=await waitDone(store,ticket.id);
 assert.deepEqual(tasks,['反馈诊断','实施修复','独立验收','交付变更']);
 if(altered){assert.equal(result.stage,'interrupted');assert.match(result.error,/does not match/);assert.ok(store.claims.has(dir));}
 else {assert.equal(result.stage,'completed');assert.ok(result.notes.some(note=>note.kind==='delivery_verification'));assert.equal(store.claims.has(dir),false);}
});

test('a real dirty repository becomes actionable without touching its existing changes',async t=>{
 const dir=await mkdtemp(join(tmpdir(),'maintenance-dirty-'));t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q');git(dir,'config','user.name','Tester');git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'original');git(dir,'add','game.txt');git(dir,'commit','-qm','initial');
 await writeFile(join(dir,'personal.txt'),'user data');
 const store=new Store(),tasks=[];
 const workers={async spawn(){tasks.push('spawn');return {id:'agent'};},async wait(){return {id:'agent',status:'completed',error:null,result:{response:'{"decision":"needs_info"}'}};},async safeToRetry(){return false;}};
 const engine=new MaintenanceEngine(store,workers,{id:()=> 'ticket',now:()=>Date.now(),report:()=>{},git:new GitVerifier(run)});
 const ticket=await engine.create('chat',dir,'simulated','问题');await engine.start(ticket.id);
 for(let i=0;i<300;i++){const current=await store.get(ticket.id);if(current.stage==='needs_info')break;await new Promise(r=>setTimeout(r,5));}
 const blocked=await store.get(ticket.id);assert.equal(blocked.stage,'needs_info');assert.match(blocked.error,/未提交改动/);
 assert.equal(tasks.length,0);assert.equal(store.claims.has(dir),false);
 assert.equal(await readFile(join(dir,'personal.txt'),'utf8'),'user data');
 assert.match(git(dir,'status','--porcelain'),/\?\? personal\.txt/);
});

for(const remoteHeadMatches of [true,false]) test(`pipeline ${remoteHeadMatches?'accepts verified PR':'interrupts when PR head differs'}`,async t=>{
 const dir=await mkdtemp(join(tmpdir(),'maintenance-pr-flow-'));t.after(()=>rm(dir,{recursive:true,force:true}));
 git(dir,'init','-q');git(dir,'config','user.name','Tester');git(dir,'config','user.email','test@example.invalid');
 await writeFile(join(dir,'game.txt'),'bug');git(dir,'add','game.txt');git(dir,'commit','-qm','initial');
 git(dir,'remote','add','origin','https://github.com/example/game.git');
 const store=new Store(),tasks=[],url='https://github.com/example/game/pull/7';
 const runFake=async command=>{
  const head=git(dir,'rev-parse','HEAD'); const payload=JSON.stringify({url,number:7,state:'OPEN',headRefOid:remoteHeadMatches?head:'f'.repeat(40)});
  // Use the real Git commands but a deterministic local gh stand-in for the network response.
  const bin=join(tmpdir(),'maintenance-pr-bin-'+process.pid+'-'+(remoteHeadMatches?'good':'bad'));
  await import('node:fs/promises').then(fs=>fs.mkdir(bin,{recursive:true}));
  await writeFile(join(bin,'gh'),'#!/bin/sh\nprintf "%s\\n" "$FAKE_PR_RESPONSE"\n',{mode:0o755});
  t.after(()=>rm(bin,{recursive:true,force:true}));
  try{const result=await shell(command,{timeout:10000,env:{...process.env,PATH:bin+':'+process.env.PATH,FAKE_PR_RESPONSE:payload}});return {output:result.stdout,exitCode:0,timedOut:false};}
  catch(e){return {output:e.stdout??'',exitCode:e.code??1,timedOut:!!e.killed};}
 };
 const workers={async spawn(_parent,name){tasks.push(name);return {id:String(tasks.length-1)};},async wait(_parent,id){
  const name=tasks[Number(id)];let response;
  if(name==='反馈诊断')response={decision:'fix'};
  else if(name==='实施修复'){await writeFile(join(dir,'game.txt'),'fixed');response={summary:'fixed'};}
  else if(name==='独立验收')response={decision:'pass'};
  else {git(dir,'add','-A');git(dir,'commit','-qm','fix');response={summary:'draft PR',pullRequestUrl:url};}
  return {id,status:'completed',error:null,result:{response:JSON.stringify(response)}};
 },async safeToRetry(){return false;}};
 const engine=new MaintenanceEngine(store,workers,{id:()=> 'ticket',now:()=>Date.now(),report:()=>{},git:new GitVerifier(runFake)});
 const ticket=await engine.create('chat',dir,'simulated','game bug');await engine.start(ticket.id);const result=await waitDone(store,ticket.id);
 if(remoteHeadMatches){assert.equal(result.stage,'completed');assert.equal(JSON.parse(result.notes.find(n=>n.kind==='delivery_verification').text).pullRequest.url,url);}
 else {assert.equal(result.stage,'interrupted');assert.match(result.error,/verified commit/);assert.equal(store.claims.has(dir),true);}
});
