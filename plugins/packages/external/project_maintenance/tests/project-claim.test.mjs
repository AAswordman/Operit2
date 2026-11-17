import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {objectStorage} from '../../../external/qqbot/tests/storage.mjs';
const require=createRequire(import.meta.url), storage=objectStorage();
globalThis.ToolPkg={getSpaceDataDir:()=>'/claim-test'}; globalThis.Tools={Storage:{objects:storage}};
const {Repository}=require('../dist/storage.js');
test('project claims use atomic storage, normalize aliases, and only the owner may release',async()=>{
 const a=new Repository(), b=new Repository();
 const results=await Promise.all([a.claimProject('/repo/./game/','one','owner-a'),b.claimProject('/repo/other/../game','two','owner-b')]);
 assert.equal(results.filter(Boolean).length,1);
 const owner=results[0]?'owner-a':'owner-b', loser=results[0]?'owner-b':'owner-a';
 await assert.rejects(a.releaseProject('/repo/game',loser),/owner changed/);
 assert.equal(await b.claimProject('/repo/game','new','third'),false);
 await a.releaseProject('/repo/game',owner);
 assert.equal(await b.claimProject('/repo/game','new','third'),true);
 await assert.rejects(a.claimProject('relative','bad','owner'),/absolute/);
});
test('failed-ticket reclamation atomically transfers the claim and transitions one executor',async()=>{
 const a=new Repository(), b=new Repository();
 const t={schemaVersion:1,id:'resume-test',parentChatId:'p',projectPath:'/repo/resume-test',source:'chat',feedback:'bug',stage:'failed',createdAt:1,updatedAt:2,agentId:'agent',attempt:1,error:'failed',notes:[{at:1,kind:'implement',text:'patch'}]};
 await a.save(t,true); await a.claimProject(t.projectPath,t.id,'old-owner');
 const results=await Promise.allSettled([a.reclaimFailed(t.id,'agent','new-a',3),b.reclaimFailed(t.id,'agent','new-b',4)]);
 assert.equal(results.filter(x=>x.status==='fulfilled').length,1);
 const winner=results[0].status==='fulfilled'?'new-a':'new-b';
 const after=await a.get(t.id); assert.equal(after.stage,'review'); assert.equal(after.agentId,null); assert.equal(after.error,null);
 assert.equal(results.find(x=>x.status==='fulfilled').value.resumeReview,true);
 await assert.rejects(a.releaseProject(t.projectPath,'old-owner'),/owner changed/); await a.releaseProject(t.projectPath,winner);
});
test('failed-ticket reclamation rejects changed agent and other ticket ownership',async()=>{
 const a=new Repository(); const t={schemaVersion:1,id:'reclaim-reject',projectPath:'/repo/reclaim-reject',stage:'failed',agentId:'expected',notes:[]};
 await a.save(t,true); await a.claimProject(t.projectPath,'another-ticket','old');
 await assert.rejects(a.reclaimFailed(t.id,'wrong','new',1),/Ticket changed/);
 await assert.rejects(a.reclaimFailed(t.id,'expected','new',1),/reservation changed/);
 assert.equal((await a.get(t.id)).stage,'failed');
});
