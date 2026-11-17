import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { load } from './helpers.mjs';
const { DspRuntime } = await load('web/audio/dsp-runtime.ts');
const { emptyProject } = await load('src/shared/model.ts');
const { makeTrack } = await load('src/shared/presets.ts');
const module = await WebAssembly.compile(await readFile('resources/dsp/studio.wasm'));

function render(pitch, { rate=48000, chunk=128, seconds=1.25 }={}) {
  const project=emptyProject();project.masterGain=.5;
  const track=makeTrack('analog-kit',0);track.effects=[];track.gain=.6;
  project.tracks=[track];
  const runtime=new DspRuntime(module,rate);runtime.load(project,false);
  runtime.api.note(0,pitch,.8,Math.round(rate*.1));
  const audio=new Float32Array(Math.round(rate*seconds));
  for(let i=0;i<audio.length;i+=chunk) {
    const count=Math.min(chunk,audio.length-i);runtime.api.render(count);
    audio.set(runtime.left.subarray(0,count),i);
    for(const value of runtime.right.subarray(0,count))assert.ok(Number.isFinite(value)&&Math.abs(value)<=.93);
  }
  assert.equal(runtime.api.dropped(),0);
  assert.equal(runtime.api.active(),0);
  return audio;
}
function rms(audio) { return Math.sqrt(audio.reduce((sum,value)=>sum+value*value,0)/audio.length); }
function amplitude(audio,hz,rate,start,end) {
  let re=0,im=0,weight=0;
  for(let i=Math.round(start*rate);i<Math.round(end*rate);i++) {
    const w=.5-.5*Math.cos(2*Math.PI*(i-start*rate)/((end-start)*rate-1));
    const phase=2*Math.PI*hz*i/rate;re+=audio[i]*w*Math.cos(phase);im+=audio[i]*w*Math.sin(phase);weight+=w;
  }
  return Math.hypot(re,im)/weight;
}
function dominant(audio,rate,start,end,min=150,max=800) {
  let peak=0,frequency=0;
  for(let hz=min;hz<=max;hz+=2) {
    const value=amplitude(audio,hz,rate,start,end);
    if(value>peak){peak=value;frequency=hz;}
  }
  return frequency;
}

test('Crash 2 and other GM cymbals use a noise tail instead of the descending tonal fallback',()=>{
  const crash=render(49);
  for(const pitch of [51,52,53,55,57,59]) {
    const sound=render(pitch);
    assert.ok(sound.every((value,i)=>value===crash[i]),`GM cymbal ${pitch} must share the cymbal voice`);
  }
  const window=crash.subarray(960,9600),energy=rms(window);
  let strongest=0;
  for(let hz=350;hz<=800;hz+=5)strongest=Math.max(strongest,amplitude(crash,hz,48000,.02,.2));
  assert.ok(strongest<energy*.12,'cymbal must not contain a prominent low tonal sweep');
  assert.ok(rms(crash.subarray(24000,28800))>1e-5,'crash keeps an audible noise tail');
});

test('GM bongos and congas have distinct, stable resonances and short natural tails',()=>{
  const pitches=[];
  for(const pitch of [60,61,62,63,64]) {
    const sound=render(pitch);
    const early=dominant(sound,48000,.012,.062),late=dominant(sound,48000,.07,.12);
    assert.ok(Math.abs(early-late)<=8,`${pitch}: early ${early}Hz, late ${late}Hz must not dive`);
    assert.ok(rms(sound.subarray(0,4800))>.005,`${pitch}: audible attack`);
    assert.equal(rms(sound.subarray(19200)),0,`${pitch}: no lingering melodic tone`);
    pitches.push(early);
  }
  assert.ok(pitches[0]>pitches[1]&&pitches[1]>pitches[2]&&pitches[2]>pitches[3]&&pitches[3]>pitches[4],`distinct high/low hand drums: ${pitches}`);
});

test('kick and all six GM toms retain their tuned drum transients',()=>{
  const kick=render(36),early=dominant(kick,48000,.005,.025,40,200),late=dominant(kick,48000,.08,.14,40,200);
  assert.ok(early>late+15,'kick retains its short pitch transient');
  const toms=[41,43,45,47,48,50].map(pitch=>dominant(render(pitch),48000,.08,.16,50,250));
  for(let i=1;i<toms.length;i++)assert.ok(toms[i]>toms[i-1],`tom tuning rises with GM mapping: ${toms}`);
});

test('unmapped percussion stays unpitched instead of inventing a descending note',()=>{
  const sound=render(80),energy=rms(sound.subarray(480,4800));
  let strongest=0;
  for(let hz=350;hz<=800;hz+=5)strongest=Math.max(strongest,amplitude(sound,hz,48000,.01,.1));
  assert.ok(energy>.001,'fallback remains audible');
  assert.ok(strongest<energy*.2,'unsupported percussion must not produce a narrow tonal sweep');
  assert.equal(rms(sound.subarray(7200)),0);
});

test('drum rendering is finite and independent of caller block size at common sample rates',()=>{
  for(const rate of [22050,44100,48000,96000])for(const pitch of [36,38,42,47,50,56,57,60,61,62,63,64,80]) {
    const reference=render(pitch,{rate});
    assert.ok(rms(reference)>.0001,`audible ${pitch} at ${rate}Hz`);
    const actual=render(pitch,{rate,chunk:17});assert.ok(actual.every((value,i)=>value===reference[i]),`block-size invariant ${pitch} at ${rate}Hz`);
  }
});
