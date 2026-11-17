import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { load } from './helpers.mjs';
const { auditionProject }=await load('web/audio/audition.ts');
const { DspRuntime }=await load('web/audio/dsp-runtime.ts');
const { WasmAudioEngine }=await load('web/audio/wasm-engine.ts');
const { emptyProject }=await load('src/shared/model.ts');
const { makeTrack }=await load('src/shared/presets.ts');
const module=await WebAssembly.compile(await readFile('resources/dsp/studio.wasm'));
function project() { const p=emptyProject();p.tracks=[makeTrack('sub-bass',0)];p.tracks[0].synth.release=.02;p.tracks[0].synth.haasMs=0;p.tracks[0].notes=[{id:'n',start:0,duration:1,pitch:45,velocity:.8}];return p; }
test('audition isolates actual selected instrument/effects, ignores mute and automation, never mutates source',()=>{
  const p=project();p.tracks[0].mute=true;p.tracks[0].automation=[{target:'level',points:[{beat:0,value:0}]}];
  const before=structuredClone(p),preview=auditionProject(p,p.tracks[0].id);
  assert.equal(preview.tracks.length,1);assert.equal(preview.loop.enabled,false);assert.equal(preview.tracks[0].mute,false);assert.deepEqual(preview.tracks[0].notes,[]);assert.deepEqual(preview.tracks[0].automation,[]);
  assert.deepEqual(preview.tracks[0].synth,p.tracks[0].synth);preview.tracks[0].synth.cutoff=40;assert.deepEqual(p,before);
});
test('held WASM audition produces sound while transport stopped, supports chords and note-off/release',()=>{
  const p=project(),r=new DspRuntime(module,48000);r.load(auditionProject(p,p.tracks[0].id),false);
  const l=new Float32Array(128),right=new Float32Array(128);
  r.processPreview(l,right);assert.ok(l.every(v=>v===0));
  r.previewOn(45);r.previewOn(52);let peak=0;
  for(let i=0;i<50;i++){r.processPreview(l,right);for(const v of l)peak=Math.max(peak,Math.abs(v));}
  assert.ok(peak>.005);assert.equal(r.api.active(),2);assert.equal(r.playing,false);assert.equal(r.frame,0);
  r.previewOff(45);for(let i=0;i<20;i++)r.processPreview(l,right);assert.equal(r.api.active(),1);
  r.previewOff();for(let i=0;i<500;i++)r.processPreview(l,right);assert.equal(r.api.active(),0);assert.ok(l.every(v=>Number.isFinite(v)&&Math.abs(v)<1e-7));
});
test('releasing a preview key before async audio unlock cannot emit a late stuck note',async()=>{
  const p=project(),engine=new WasmAudioEngine(p),messages=[];let resolve;
  engine.unlock=()=>new Promise(r=>{resolve=r;});engine.node={port:{postMessage:m=>messages.push(m)}};engine.message=async(type,extras)=>{messages.push({type,...extras});};
  const starting=engine.previewNoteOn(p.tracks[0].id,45);engine.previewNoteOff(45);resolve();await starting;
  assert.equal(messages.some(m=>m.type==='preview-on'),false);assert.deepEqual(engine.auditionPitches,[]);
});
test('preview keys share a solo routing load and never trigger sequence play/seek',async()=>{
  const p=project(),engine=new WasmAudioEngine(p),messages=[];
  engine.unlock=async()=>{};engine.node={port:{postMessage:m=>messages.push(m)}};engine.message=async(type,extras)=>{messages.push({type,...extras});};
  await Promise.all([engine.previewNoteOn(p.tracks[0].id,45),engine.previewNoteOn(p.tracks[0].id,52)]);
  assert.equal(messages.filter(m=>m.type==='preview-load').length,1);assert.equal(messages.filter(m=>m.type==='preview-on').length,2);
  assert.equal(messages.some(m=>['play','seek'].includes(m.type)),false);assert.equal(engine.playing,false);assert.equal(engine.beat,0);
  engine.previewClear();assert.deepEqual(engine.auditionPitches,[]);
});
