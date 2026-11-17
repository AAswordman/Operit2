import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { load } from './helpers.mjs';
const { DspRuntime } = await load('web/audio/dsp-runtime.ts');
const { emptyProject } = await load('src/shared/model.ts');
const { makeTrack } = await load('src/shared/presets.ts');
const module = await WebAssembly.compile(await readFile('resources/dsp/studio.wasm'));
function render(patch={}, { pitch=69,rate=48000,length=16384,chunk=128,gate=12000,gain=.6,master=.5 }={}) {
  const p=emptyProject();p.masterGain=master;
  const t=makeTrack('sub-bass',0);t.gain=gain;t.synth={...t.synth,wave:'sine',waveB:'sine',blend:0,unison:1,unisonB:1,width:0,widthB:0,detune:0,detuneB:0,oscBFine:0,subLevel:0,phaseRandom:0,attack:.001,decay:.1,sustain:1,release:.04,cutoff:18000,resonance:.707,filterEnv:0,lfoDepth:0,pitchSweep:0,...patch};
  p.tracks=[t];const r=new DspRuntime(module,rate);r.load(p,false);r.api.note(0,pitch,.8,gate);
  const left=new Float32Array(length),right=new Float32Array(length);
  for(let i=0;i<length;i+=chunk){const n=Math.min(chunk,length-i);r.api.render(n);left.set(r.left.subarray(0,n),i);right.set(r.right.subarray(0,n),i);}
  assert.ok([...left,...right].every(Number.isFinite));return {left,right};
}
function rms(x){return Math.sqrt(x.reduce((n,v)=>n+v*v,0)/x.length);}
function band(x,hz,rate){let re=0,im=0,sum=0;for(let i=2048;i<x.length;i++){const w=.5-.5*Math.cos(2*Math.PI*(i-2048)/(x.length-2048-1)),p=2*Math.PI*hz*i/rate;re+=x[i]*w*Math.cos(p);im+=x[i]*w*Math.sin(p);sum+=w;}return Math.hypot(re,im)/sum;}
test('filter envelope, LFO and pitch sweep are sample-clock invariant across caller block sizes',()=>{
  const patch={ wave:'sawtooth',unison:3,detune:11,phaseRandom:.7,cutoff:1300,filterEnv:4,lfoRate:9,lfoDepth:2,pitchSweep:12 };
  const reference=render(patch,{ chunk:128 });
  for(const chunk of [1,7,17,64,127]) {
    const actual=render(patch,{chunk});
    assert.deepEqual(actual.left,reference.left,`left: block=${chunk}`);assert.deepEqual(actual.right,reference.right,`right: block=${chunk}`);
  }
});
test('coherent A/B zero-detune layers keep unity gain instead of growing with oscillator count',()=>{
  for(const blend of [0,1,.4]) {
    const one=render({ blend }),eight=render({ blend,unison:8,unisonB:8 });
    const error=one.left.map((v,i)=>v-eight.left[i]);assert.ok(rms(error)<1e-7);
  }
});
test('fixed harmonic waves suppress the folded 7th/11th partials at high pitch',()=>{
  const pitch=108,rate=48000,frequency=440*2**((pitch-69)/12);
  for(const wave of ['glass','hollow','triangle']) {
    const sound=render({ wave },{ pitch,rate,length:32768,gate:48000,gain:.1,master:.25 });
    const fundamental=band(sound.left,frequency,rate);
    for(const h of [7,11]) {
      const f=frequency*h,folded=Math.abs(((f+rate/2)%rate)-rate/2);
      const alias=band(sound.left,folded,rate);
      assert.ok(alias<fundamental*.002,`${wave} h=${h}: alias/fundamental=${alias/fundamental}`);
    }
  }
});
test('curved release reaches silence with a small residual slope at the voice boundary',()=>{
  const gate=4800,release=1920,total=gate+release;
  const sound=render({release:.04},{gate,length:total+128});
  const peak=Math.max(...sound.left.map(Math.abs));
  assert.ok(rms(sound.left.slice(total-16,total))<peak*.0001);
  assert.ok(sound.left.slice(total).every(v=>Math.abs(v)<1e-7));
});
test('voice modulation remains finite and bounded at high Q and several sample rates',()=>{
  for(const rate of [22050,44100,48000,96000]) {
    const sound=render({wave:'square',unison:8,unisonB:8,blend:.5,detune:30,detuneB:20,phaseRandom:1,resonance:12,cutoff:3000,filterEnv:5,lfoRate:16,lfoDepth:3,pitchSweep:24},{rate,pitch:100,length:8192});
    assert.ok([...sound.left,...sound.right].every(v=>Math.abs(v)<=.93));
  }
});
