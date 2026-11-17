import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { load } from './helpers.mjs';
const { DspRuntime } = await load('web/audio/dsp-runtime.ts');
const { emptyProject } = await load('src/shared/model.ts');
const { PRESETS } = await load('src/shared/presets.ts');
const { parseSynth } = await load('src/shared/validation.ts');
const module = await WebAssembly.compile(await readFile('resources/dsp/studio.wasm'));
const base = { ...PRESETS.find(p => p.id === 'sub-bass').synth, engine: 'spectral', wave: 'sine', waveB: 'sine', blend: 0, unison: 1, unisonB: 1, detune: 0, detuneB: 0, oscBFine: 0, width: 0, widthB: 0, cutoff: 18000, resonance: .707, attack: .001, decay: .1, sustain: 1, release: .02, phaseRandom: 0 };
function project(patch = {}, pitch = 45) {
  const p = emptyProject(); p.bpm = 120; p.bars = 1; p.loop.enabled = false; p.masterGain = .5;
  p.tracks = [{ id: 'test', name: 'test', color: '#b8f36b', preset: 'sub-bass', synth: parseSynth({ ...base, ...patch }), gain: .8, pan: 0, mute: false, solo: false, automation: [], effects: [], notes: [{ id: 'note', start: 0, duration: 1, pitch, velocity: .8 }] }];
  return p;
}
function render(patch = {}, { pitch = 45, rate = 48000, frames = 24000 } = {}) {
  const runtime = new DspRuntime(module, rate); runtime.load(project(patch, pitch), false); runtime.playing = true;
  const left = new Float32Array(frames), right = new Float32Array(frames);
  for (let at = 0; at < frames; at += 128) runtime.process(left.subarray(at, at + 128), right.subarray(at, at + 128));
  assert.ok([...left, ...right].every(Number.isFinite), 'all samples must stay finite');
  return { left, right, runtime };
}
function rms(x, start = 2048) { let sum = 0; for (let i = start; i < x.length; i++) sum += x[i] ** 2; return Math.sqrt(sum / (x.length - start)); }
function distance(a, b, start = 2048) { return rms(a.map((x, i) => x - b[i]), start); }
function energyAt(x, hz, rate = 48000) { let a = 0, b = 0; for (let i = 2048; i < x.length; i++) { const phase = 2 * Math.PI * hz * i / rate; a += x[i] * Math.cos(phase); b += x[i] * Math.sin(phase); } return Math.hypot(a, b) / (x.length - 2048); }

test('six selectable waves and independent A/B tuning change actual WASM samples', () => {
  const signals = ['sine', 'triangle', 'sawtooth', 'square', 'glass', 'hollow'].map(wave => render({ wave }).left);
  for (let i = 0; i < signals.length; i++) for (let j = i + 1; j < signals.length; j++) assert.ok(distance(signals[i], signals[j]) > .0005);
  const a = render().left;
  assert.ok(distance(a, render({ blend: 1, waveB: 'square' }).left) > .005);
  const octave = render({ blend: 1, oscBOctave: 1 }).left;
  assert.ok(energyAt(octave, 220) > energyAt(octave, 110) * 10);
  const semitone = render({ blend: 1, oscBSemitone: 12 }).left;
  assert.ok(distance(octave, semitone) < 1e-6);
  assert.ok(distance(a, render({ blend: 1, oscBFine: 50 }).left) > .005);
  assert.ok(distance(a, render({ oscBOctave: 2, oscBSemitone: 7, oscBFine: 50 }).left) < 1e-6, 'inaudible B must not change A');
});

test('A and B independently support 8 voices, detune and stereo spread', () => {
  for (const bank of ['A', 'B']) {
    const blend = bank === 'A' ? 0 : 1;
    const keys = bank === 'A' ? ['unison', 'detune', 'width'] : ['unisonB', 'detuneB', 'widthB'];
    const one = render({ blend, phaseRandom: 1 });
    const many = render({ blend, phaseRandom: 1, [keys[0]]: 8, [keys[1]]: 30, [keys[2]]: 1 });
    assert.ok(distance(one.left, many.left) > .005);
    assert.ok(distance(many.left, many.right) > .005);
    const centered = render({ blend, phaseRandom: 1, [keys[0]]: 8, [keys[1]]: 30, [keys[2]]: 0 });
    assert.ok(distance(centered.left, centered.right) < 1e-7);
    assert.equal(many.runtime.stats().maxVoices, 1, 'unison oscillators are not MIDI note voices');
  }
});

test('Sub adds a centered lower octave and noise/phase are audible', () => {
  const dry = render();
  const sub = render({ subLevel: 1, subOctave: -1 });
  assert.ok(energyAt(sub.left, 55) > energyAt(dry.left, 55) * 10);
  assert.ok(distance(sub.left, sub.right) < 1e-7);
  assert.ok(energyAt(render({ subLevel: 1, subOctave: -2 }).left, 27.5) > energyAt(dry.left, 27.5) * 10);
  assert.ok(distance(dry.left, render({ noiseLevel: 1 }).left) > .005);
  assert.ok(distance(dry.left, render({ phase: .25 }).left) > .005);
  assert.ok(distance(render({ unison: 8, phaseRandom: 0 }).left, render({ unison: 8, phaseRandom: 1 }).left) > .005);
});

test('resonance, octave LFO and cutoff are functional and stable at high Q', () => {
  const low = render({ wave: 'sawtooth', cutoff: 800, resonance: .707 });
  const resonant = render({ wave: 'sawtooth', cutoff: 800, resonance: 12 });
  assert.ok(distance(low.left, resonant.left) > .005);
  assert.ok(distance(low.left, render({ wave: 'sawtooth', cutoff: 800, lfoRate: 8, lfoDepth: 3 }).left) > .005);
  for (const rate of [22050, 44100, 48000, 96000]) for (const cutoff of [40, 18000]) {
    const sound = render({ wave: 'square', unison: 8, unisonB: 8, blend: .5, cutoff, resonance: 12 }, { rate, pitch: 100, frames: 4096 });
    assert.ok(sound.left.every(x => Math.abs(x) <= .93));
  }
});

test('Haas delays exactly the chosen side, including fractional samples and bypass', () => {
  const dry = render();
  for (const sign of [-1, 1]) {
    const sound = render({ haasMs: sign * 12 });
    const delayed = sign > 0 ? sound.right : sound.left, direct = sign > 0 ? sound.left : sound.right;
    assert.ok(distance(direct, dry.left) < 1e-7);
    assert.ok(delayed.slice(0, 576).every(x => x === 0));
    for (let i = 576; i < delayed.length; i++) assert.ok(Math.abs(delayed[i] - direct[i - 576]) < 1e-6);
  }
  for (const patch of [{ haasMs: 0 }, { haasMs: 35, haasMix: 0 }]) assert.ok(distance(dry.right, render(patch).right) < 1e-7);
  const wet = render({ haasMs: 12.1, haasMix: .5 });
  const saturate = x => .92 * x * (27 + x*x) / (27 + 9*x*x);
  const inverse = value => { let lo = -3, hi = 3; for (let i = 0; i < 32; i++) { const mid = (lo+hi)/2; if (saturate(mid) < value) lo = mid; else hi = mid; } return (lo+hi)/2; };
  for (let i = 2048; i < 4096; i++) {
    // 12.1ms at 48kHz = 580.8 frames; interpolation and wet mix precede master saturation.
    const expected = saturate(.5 * inverse(dry.left[i]) + .5 * (.2 * inverse(dry.left[i-580]) + .8 * inverse(dry.left[i-581])));
    assert.ok(Math.abs(wet.right[i] - expected) < 1e-6);
  }
  assert.ok(distance(dry.right, wet.right) > .005);
  for (const rate of [44100, 96000, 192000]) {
    const sound = render({ haasMs: 35 }, { rate, frames: 8192 });
    const lag = Math.floor(rate * .035);
    assert.ok(sound.right.slice(0, lag).every(x => x === 0));
    assert.ok(rms(sound.right, lag) > 0);
  }
});

test('post-Haas bass narrowing attenuates low side but preserves the mid', () => {
  const dry = render({ haasMs: 12 }, { pitch: 33 });
  const narrow = render({ haasMs: 12, bassMono: 200 }, { pitch: 33 });
  const side = sound => sound.left.map((x, i) => (x - sound.right[i]) * .5);
  const mid = sound => sound.left.map((x, i) => (x + sound.right[i]) * .5);
  assert.ok(rms(side(narrow)) < rms(side(dry)) * .12);
  assert.ok(distance(mid(dry), mid(narrow)) < .0005);
  const highDry = render({ haasMs: 12 }, { pitch: 93 });
  const highNarrow = render({ haasMs: 12, bassMono: 200 }, { pitch: 93 });
  assert.ok(rms(side(highNarrow)) > rms(side(highDry)) * .8, 'upper stereo image should remain');
});

test('seek/reset clears Haas tails and fixed-phase notes are reproducible', () => {
  const first = render({ haasMs: -35, bassMono: 200, unison: 8, unisonB: 8, phaseRandom: 0 });
  first.runtime.seek(0); first.runtime.playing = true;
  const left = new Float32Array(1024), right = new Float32Array(1024); first.runtime.process(left, right);
  assert.deepEqual(left, first.left.slice(0, 1024)); assert.deepEqual(right, first.right.slice(0, 1024));
  first.runtime.api.reset(); first.runtime.api.render(128);
  assert.ok(first.runtime.left.every(x => x === 0) && first.runtime.right.every(x => x === 0));
});


test('Haas drains delayed audio after note release rather than cutting it off', () => {
  const sound = render({ haasMs: 35 }, { frames: 28000 });
  assert.equal(sound.runtime.stats().voices, 0);
  assert.ok(sound.left.slice(25000).every(x => Math.abs(x) < 1e-8));
  assert.ok(rms(sound.right.slice(25000, 26600), 0) > .001);
  assert.ok(sound.right.slice(26700).every(x => Math.abs(x) < 1e-8));
});

test('random phase varies between triggers; fixed phase retriggers identically', () => {
  function triggers(phaseRandom) {
    const runtime = new DspRuntime(module, 48000); runtime.load(project({ unison: 5, detune: 0, phaseRandom }), false);
    function note() {
      runtime.api.note(0, 45, .8, 1024);
      const result = new Float32Array(2048);
      for (let at = 0; at < result.length; at += 128) { runtime.api.render(128); result.set(runtime.left, at); }
      assert.equal(runtime.api.active(), 0);
      return result;
    }
    return [note(), note()];
  }
  const fixed = triggers(0), random = triggers(1);
  assert.ok(distance(fixed[0], fixed[1], 128) < 1e-7);
  assert.ok(distance(random[0], random[1], 128) > .005);
  const again = triggers(1);
  assert.deepEqual(random, again, 'seeded project reset must reproduce offline rendering');
});
