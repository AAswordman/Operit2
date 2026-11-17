import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { load } from './helpers.mjs';
const { DspRuntime } = await load('web/audio/dsp-runtime.ts');
const { emptyProject } = await load('src/shared/model.ts');
const { makeTrack } = await load('src/shared/presets.ts');
const module = await WebAssembly.compile(await readFile('resources/dsp/studio.wasm'));
function setup(patch={},rate=48000){
  const p=emptyProject(),t=makeTrack('aether-air');
  p.masterGain=.5;t.gain=.6;t.effects=[];
  t.synth={...t.synth,attack:.001,decay:.1,sustain:1,release:.05,cutoff:18000,resonance:.707,lfoDepth:0,blend:0,noiseLevel:.7,brightness:.5,width:.8,...patch};
  p.tracks=[t];const r=new DspRuntime(module,rate);r.load(p,false);return r;
}
function capture(r,length=32768,chunk=128){
  const left=new Float32Array(length),right=new Float32Array(length);
  for(let i=0;i<length;i+=chunk){const n=Math.min(chunk,length-i);r.api.render(n);left.set(r.left.subarray(0,n),i);right.set(r.right.subarray(0,n),i);}
  assert.ok(left.every(Number.isFinite)&&right.every(Number.isFinite));return {left,right};
}
function render(patch={},options={}){const r=setup(patch,options.rate);for(const pitch of options.pitches??[74])r.api.note(0,pitch,.65,200000);return capture(r,options.length,options.chunk);}
const rms=x=>Math.sqrt(x.reduce((s,v)=>s+v*v,0)/x.length);
const roughness=x=>rms(x.slice(2049).map((v,i)=>v-x[i+2048]))/rms(x.slice(2048));
test('air width=0 is mono, widening creates independent side energy without a Haas delay',()=>{
  const mono=render({width:0}),wide=render({width:1});assert.deepEqual(mono.left,mono.right);
  assert.ok(rms(wide.left.map((v,i)=>v-wide.right[i]))>rms(wide.left)*.7);
  assert.ok(Math.abs(rms(wide.left)/rms(mono.left)-1)<.2,'width should not act like a volume boost');
});
test('air brightness controls audible spectral balance, not a cosmetic parameter',()=>{
  const dark=render({brightness:0}),bright=render({brightness:1});
  assert.ok(roughness(bright.left)>roughness(dark.left)*1.35);
});
test('air and harmonic amounts have useful independent endpoints',()=>{
  const silence=render({noiseLevel:0,blend:0}),tone=render({noiseLevel:0,blend:1}),air=render({noiseLevel:1,blend:0});
  assert.equal(rms(silence.left),0);assert.ok(rms(tone.left)>.005&&rms(air.left)>.005);
  assert.deepEqual(tone.left,tone.right,'harmonic core is coherent, not detuned against the chord');
});
test('two-voice air drift and random texture are invariant to render block size',()=>{
  const reference=render({}, {pitches:[74,77],length:8192});
  for(const chunk of [1,7,64,127]){const actual=render({}, {pitches:[74,77],length:8192,chunk});assert.deepEqual(actual.left,reference.left);assert.deepEqual(actual.right,reference.right);}
});
test('live air brightness/width updates preserve held voices and finite output',()=>{
  const r=setup();r.api.note(0,74,.65,200000);capture(r,8192);
  assert.ok(r.liveParameter(r.project.tracks[0].id,'synth','brightness',1));
  assert.ok(r.liveParameter(r.project.tracks[0].id,'synth','width',0));
  assert.equal(r.api.active(),1);assert.equal(r.api.has_note(0,74),1);
  const sound=capture(r);assert.ok(rms(sound.left)>.005);assert.ok(sound.left.every(v=>Math.abs(v)<=.93));
});
test('resonant air bands remain bounded and finite across supported sample rates',()=>{
  for(const rate of [22050,44100,48000,96000]){
    const sound=render({noiseLevel:1,brightness:1,blend:.4,resonance:12,lfoRate:16,lfoDepth:3},{rate,length:16384,pitches:[65,74,82]});
    assert.ok(sound.left.every(v=>Math.abs(v)<=.93)&&sound.right.every(v=>Math.abs(v)<=.93));
  }
});
