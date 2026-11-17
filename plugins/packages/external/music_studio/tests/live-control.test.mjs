import test from 'node:test';
import assert from 'node:assert/strict';
import { load } from './helpers.mjs';
const { createLiveControl }=await load('web/live-control.ts');
const { StudioService }=await load('src/service.ts');
const ref=s=>({ projectId:s.projectId,revision:s.revision });
async function fixture() {
  let saved=null;const calls=[];
  const service=new StudioService({ read:async()=>saved,write:async db=>{ saved=structuredClone(db); } });
  const p=(await service.request({ action:'get' })).project;
  let playback={ connected:true,unlocked:false,playing:false,beat:0,peak:0,voices:0,dropped:0,updatedAt:0,projectId:p.id };
  const api=createLiveControl({
    read:async()=> (await service.request({ action:'get' })).project,
    write:async(request,label)=>{ calls.push({ request,label });return (await service.request(request)).project; },
    status:()=>playback,
    transport:async(type,beat)=>{calls.push({type,beat});if(type==='seek')playback.beat=beat;else playback.playing=type==='play';},
    view:(type,trackId,beat)=>calls.push({view:type,trackId,beat}),catalog:()=>service.request({action:'catalog'}),
  });
  return { api,service,calls,unlock:()=>{playback.unlocked=true;},saved:()=>structuredClone(saved) };
}
test('read/notes/catalog/export reflect the same current project without modifying storage',async()=>{
  const f=await fixture(),before=f.saved(),state=await f.api.get();
  assert.equal(state.projectId,before.activeId);assert.equal(state.tracks.length,before.projects[0].tracks.length);
  const notes=await f.api.notes({trackId:state.tracks[0].id,fromBar:17,toBar:20});
  assert.ok(notes.notes.every(n=>n.start>=32&&n.start<40&&n.bar>=17&&n.bar<=20));
  const exported=await f.api.exportProject();assert.deepEqual(JSON.parse(exported.json),await f.api.project());
  assert.ok((await f.api.catalog()).synthRanges);assert.deepEqual(f.saved(),before);assert.equal(f.calls.length,0);
});
test('dry-run predicts changes but returns current revision and never saves or adds undo state',async()=>{
  const f=await fixture(),state=await f.api.get(),before=f.saved();
  const report=await f.api.dryRun({...ref(state),label:'检查速度',operations:[{type:'project.set',patch:{bpm:123}}]});
  assert.equal(report.revision,state.revision);assert.equal(report.proposedRevision,state.revision+1);assert.equal(report.arrangementChanged,true);
  assert.deepEqual(f.saved(),before);await assert.rejects(f.api.undo(ref(state)),/Nothing to undo/);
});
test('scoped explicit replacement preserves other bars and tracks; grouped undo/redo',async()=>{
  const f=await fixture(),state=await f.api.get(),before=await f.api.project(),id=state.tracks[0].id;
  const report=await f.api.replaceNotes({...ref(state),trackId:id,fromBar:17,toBar:18,label:'按指导重写两个小节',notes:[{pitch:36,offset:0,duration:.25,velocity:.8},{pitch:38,offset:1,duration:.25,velocity:.7}]});
  const after=await f.api.project(),outside=p=>p.tracks.find(t=>t.id===id).notes.filter(n=>n.start<32||n.start>=36);
  assert.deepEqual(outside(after),outside(before));assert.deepEqual(after.tracks.filter(t=>t.id!==id),before.tracks.filter(t=>t.id!==id));
  assert.equal(after.revision,before.revision+1);assert.equal(report.tracks[0].notesAdded,2);assert.equal(f.calls.at(-1).label,'协作 · 按指导重写两个小节');
  await f.api.undo(ref(await f.api.get()));assert.deepEqual((await f.api.project()).tracks,before.tracks);
  await f.api.redo(ref(await f.api.get()));assert.deepEqual((await f.api.project()).tracks,after.tracks);
});
test('stale refs, out-of-range notes, invalid batches and generators cannot publish partial edits',async()=>{
  const f=await fixture(),state=await f.api.get(),id=state.tracks[0].id,before=f.saved();
  await assert.rejects(f.api.batch({...ref(state),revision:state.revision+1,label:'旧版本',operations:[{type:'project.set',patch:{bpm:100}}]}),/REVISION_CONFLICT/);
  await assert.rejects(f.api.replaceNotes({...ref(state),trackId:id,fromBar:1,toBar:2,label:'越界',notes:[{pitch:36,offset:7,duration:2,velocity:.8}]}),/完整落在/);
  await assert.rejects(f.api.batch({...ref(state),label:'无效原子事务',operations:[{type:'project.set',patch:{bpm:100}},{type:'synth.set',trackId:id,patch:{unison:100}}]}));
  await assert.rejects(f.api.batch({...ref(state),label:'随机音型',operations:[{type:'pattern.generate',trackId:id}]}),/不自动生成/);
  assert.deepEqual(f.saved(),before);
});
test('audio-locked play rejects honestly without seeking; seek/pause are actual page commands, not queue receipts',async()=>{
  const f=await fixture(),state=await f.api.get();
  await assert.rejects(f.api.transport({...ref(state),type:'play',bar:17}),/AUDIO_LOCKED/);assert.equal(f.calls.length,0);
  const seek=await f.api.transport({...ref(state),type:'seek',bar:17});assert.equal(seek.beat,32);assert.equal(seek.playing,false);
  f.unlock();const play=await f.api.transport({...ref(state),type:'play'});assert.equal(play.playing,true);
  const pause=await f.api.transport({...ref(state),type:'pause'});assert.equal(pause.playing,false);assert.equal(pause.beat,32);
  assert.equal((await f.api.get()).revision,state.revision);
});
test('view changes cannot seek or change notes and no demo promotion method is exposed',async()=>{
  const f=await fixture(),state=await f.api.get(),before=f.saved();
  await f.api.view({type:'piano',trackId:state.tracks[0].id,bar:17});
  assert.deepEqual(f.calls.at(-1),{view:'piano',trackId:state.tracks[0].id,beat:32});
  await f.api.view({type:'arrangement'});assert.deepEqual(f.saved(),before);assert.equal(f.api.promoteDemo,undefined);
});
test('explicit newProject creates an empty active project without deleting or changing the old composition',async()=>{
  const f=await fixture(),before=await f.api.project();
  const created=await f.api.newProject({name:'重写 · 和声与Bass'});
  assert.notEqual(created.projectId,before.id);assert.equal(created.revision,0);assert.deepEqual(created.tracks,[]);
  const saved=f.saved();assert.equal(saved.activeId,created.projectId);assert.equal(saved.projects.length,2);
  assert.deepEqual(saved.projects.find(p=>p.id===before.id),before);
  await assert.rejects(f.api.newProject({name:''}));assert.equal(f.saved().projects.length,2);
});
