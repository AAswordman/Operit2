import test from "node:test";
import assert from "node:assert/strict";
import { load } from "./helpers.mjs";
const { parseProject, parseSynth } = await load("src/shared/validation.ts");
const { demoProject, generatePattern, TEMPLATES } = await load("src/shared/composition.ts");
const { applyOperations } = await load("src/shared/operations.ts");
const { PRESETS, makeEffect } = await load("src/shared/presets.ts");
const { encodeWav } = await load("web/audio/wav.ts");
const clone = x => JSON.parse(JSON.stringify(x));
test("every factory template round trips with original presets", () => {
  for (const id of TEMPLATES) { const p = demoProject(id); assert.deepEqual(parseProject(JSON.parse(JSON.stringify(p))), p); assert.ok(p.tracks.length >= 4); }
  for (const preset of PRESETS) assert.deepEqual(parseSynth(preset.synth), preset.synth);
});
test("seeded patterns deterministic apart from generated IDs", () => {
  const options = { kind: "arpeggio", start: 0, bars: 8, root: 57, scale: "minor", velocity: 0.7, seed: 17 };
  const clean = notes => notes.map(({ id, ...n }) => n);
  assert.deepEqual(clean(generatePattern(options)), clean(generatePattern(options)));
  assert.notDeepEqual(clean(generatePattern(options)), clean(generatePattern({ ...options, seed: 18 })));
});
test("atomic batch rollback and one revision for multiple changes", () => {
  const p = demoProject(); const before = clone(p); const t = p.tracks[0].id;
  assert.throws(() => applyOperations(p, [{ type: "track.set", trackId: t, patch: { gain: 1 } }, { type: "notes.add", trackId: t, notes: [{ pitch: 128, start: 0, duration: 1, velocity: 0.5 }] }]));
  assert.deepEqual(p, before);
  const next = applyOperations(p, [{ type: "track.set", trackId: t, patch: { gain: 1 } }, { type: "project.set", patch: { bpm: 100 } }]);
  assert.equal(next.revision, p.revision + 1); assert.equal(next.bpm, 100); assert.equal(next.tracks[0].gain, 1); assert.deepEqual(p, before);
});
test("cross-reference a new AI track inside one transaction", () => {
  const p = applyOperations(demoProject(), [{ type: "track.add", preset: "fm-keys", id: "keys_1" }, { type: "pattern.generate", trackId: "keys_1", options: { kind: "chords", bars: 2 } }]);
  assert.equal(p.tracks.find(t => t.id === "keys_1").notes.length, 6);
});
test("malformed projects, unsafe IDs, NaN, limits and duplicate IDs fail", () => {
  const p = demoProject();
  for (const modify of [p => p.version = 2, p => p.bpm = NaN, p => p.tracks[0].id = "../x", p => p.tracks[1].id = p.tracks[0].id, p => p.tracks[0].synth.unison = 9, p => p.loop.end = p.loop.start, p => p.tracks[0].notes[0].duration = -1, p => p.tracks[0].notes[0].start = 900, p => p.tracks[0].color = "url(x)", p => p.tracks[0].mute = "false", p => p.bars = 257]) { const next = clone(p); modify(next); assert.throws(() => parseProject(next)); }
  const next = clone(p); next.tracks[0].notes = Array.from({ length: 6001 }, (_, i) => ({ id: "n" + i, pitch: 60, start: 0, duration: 1, velocity: 0.5 })); assert.throws(() => parseProject(next));
});
test("preset switches preserve notes; partial effects patches preserve parameters", () => {
  const p = demoProject(); const t = p.tracks[0].id;
  const next = applyOperations(p, [{ type: "track.preset", trackId: t, preset: "fm-keys" }, { type: "effect.add", trackId: t, effectType: "delay", params: { beats: 1 } }]);
  assert.equal(next.tracks[0].notes.length, p.tracks[0].notes.length);
  const effect = next.tracks[0].effects.at(-1); const edited = applyOperations(next, [{ type: "effect.set", trackId: t, effectId: effect.id, patch: { params: { feedback: 0.5 } } }]);
  assert.equal(edited.tracks[0].effects.at(-1).params.beats, 1); assert.equal(edited.tracks[0].effects.at(-1).params.feedback, 0.5);
  assert.throws(() => applyOperations(p, [{ type: "effect.add", trackId: t, effectType: "delay", params: { feedback: 1.5 } }]));
  assert.throws(() => applyOperations(p, [{ type: "effect.add", trackId: t, effectType: "delay", params: { feedbak: 0.3 } }]));
});
test("note transforms are bounded and precise", () => {
  const p = demoProject(); const t = p.tracks[1].id;
  const next = applyOperations(p, [{ type: "notes.set", trackId: t, notes: [{ pitch: 48, start: 0.26, duration: 0.25, velocity: 0.5 }] }, { type: "notes.transform", trackId: t, patch: { transpose: 12, quantize: 0.25, velocity: 0.8 } }]);
  const n = next.tracks[1].notes[0]; assert.equal(n.pitch, 60); assert.equal(n.start, 0.25); assert.equal(n.velocity, 0.8);
  assert.throws(() => applyOperations(next, [{ type: "notes.transform", trackId: t, patch: { shift: -5 } }]));
});
test("all seven effects have valid factory defaults", () => {
  for (const type of ["eq", "filter", "drive", "chorus", "delay", "reverb", "compressor"]) { const p = demoProject(); p.tracks[0].effects = [makeEffect(type)]; assert.doesNotThrow(() => parseProject(p)); }
});
test("PCM16 WAV header, samples, finite clipping, stereo interleaving", () => {
  const wav = encodeWav([new Float32Array([-1, 0, 1, NaN]), new Float32Array([0.5, -0.5, 2, -2])], 44100); const v = new DataView(wav.buffer);
  assert.equal(new TextDecoder().decode(wav.slice(0, 4)), "RIFF"); assert.equal(new TextDecoder().decode(wav.slice(8, 12)), "WAVE");
  assert.equal(v.getUint16(22, true), 2); assert.equal(v.getUint32(24, true), 44100); assert.equal(v.getUint32(40, true), 16); assert.equal(wav.length, 60);
  assert.equal(v.getInt16(44, true), -32768); assert.equal(v.getInt16(46, true), 16384); assert.equal(v.getInt16(52, true), 32767); assert.equal(v.getInt16(56, true), 0);
});

test("only the two supplied MIDIs are built in, without truncating note tracks", () => {
  assert.deepEqual(TEMPLATES, ['the-world-revolving', 'field-of-hopes-and-dreams']);
  for (const [template, title, tracks, notes, bars, tempo] of [
    ['the-world-revolving', 'THE WORLD REVOLVING', 19, 11410, 160, 631580],
    ['field-of-hopes-and-dreams', 'The Field of Hopes and Dreams', 16, 5275, 168, 480001],
  ]) {
    const p = parseProject(demoProject(template));
    assert.equal(p.name, title); assert.equal(p.beatsPerBar, 2); assert.equal(p.bars, bars);
    assert.equal(p.bpm, 60000000 / tempo); assert.equal(p.tracks.length, tracks);
    assert.equal(p.tracks.reduce((n, t) => n + t.notes.length, 0), notes);
    assert.equal(p.loop.enabled, false); assert.equal(p.sections.at(-1).start + p.sections.at(-1).length, p.bars * 2);
    assert.ok(p.tracks.some(t => t.synth.engine === 'drums'));
    assert.ok(p.tracks.some(t => t.synth.engine !== 'drums'));
    const other = demoProject(template); p.tracks[0].notes[0].pitch = 0;
    assert.notEqual(other.tracks[0].notes[0].pitch, 0, 'templates must return independent projects');
  }
  for (const removed of ['eclipse', 'neon-drive', 'ambient-orbit', 'midnight-keys']) assert.throws(() => demoProject(removed));
});
test("automation validates transactions, interpolates seek boundaries and imports legacy projects", async () => {
  const { automationValue, automationSegment } = await load('src/shared/automation.ts');
  const points = [{beat:0,value:0.2},{beat:4,value:1},{beat:8,value:0}];
  assert.ok(Math.abs(automationValue(points,2)-0.6) < 1e-12);
  assert.deepEqual(automationSegment(points,4,6),[{beat:4,value:1},{beat:6,value:0.5}]);
  const p = demoProject(); const trackId = p.tracks[0].id;
  const next = applyOperations(p,[{type:'automation.set',trackId,lanes:[{target:'level',points}]}]);
  assert.equal(next.tracks[0].automation[0].points.length,3);
  for (const lanes of [[{target:'pan',points:[{beat:0,value:2}]}],[{target:'level',points:[]}],[{target:'level',points:[{beat:2,value:1},{beat:1,value:0}]}],[{target:'cutoff',points:[{beat:p.bars*p.beatsPerBar+1,value:1000}]}]]) assert.throws(()=>applyOperations(p,[{type:'automation.set',trackId,lanes}]));
  const old = clone(p); for (const t of old.tracks) { delete t.automation; for (const k of ['width','filterEnv','lfoRate','lfoDepth','pitchSweep']) delete t.synth[k]; for (const fx of t.effects) delete fx.params.pingPong; }
  const restored = parseProject(old); assert.deepEqual(restored.tracks[0].automation,[]); assert.equal(restored.tracks[0].synth.width,0.65);
});

test('legacy synths inherit B controls and disable new layers without losing original tuning', () => {
  const keys = ['unisonB', 'detuneB', 'widthB', 'oscBOctave', 'oscBSemitone', 'oscBFine', 'subLevel', 'subOctave', 'noiseLevel', 'phase', 'phaseRandom', 'haasMs', 'haasMix', 'bassMono'];
  for (const engine of ['spectral', 'ensemble']) {
    const legacy = { ...PRESETS[0].synth, engine, unison: 4, detune: 20, width: .8 };
    for (const key of keys) delete legacy[key];
    const s = parseSynth(legacy);
    assert.equal(s.unisonB, 4); assert.equal(s.detuneB, 17.4); assert.equal(s.widthB, .8);
    assert.equal(s.oscBOctave, engine === 'ensemble' ? 1 : 0); assert.equal(s.oscBFine, 3);
    for (const key of ['subLevel', 'noiseLevel', 'haasMs', 'bassMono']) assert.equal(s[key], 0);
    assert.deepEqual(parseSynth(JSON.parse(JSON.stringify(s))), s);
  }
});
test('new synthesis parameters validate bounds and integer fields in atomic patches', async () => {
  const { SYNTH_RANGES, SYNTH_INTEGERS } = await load('src/shared/validation.ts');
  const p = demoProject(), trackId = p.tracks[0].id;
  const patch = { unison: 8, unisonB: 7, detuneB: 25, widthB: .9, oscBOctave: -1, oscBSemitone: 7, oscBFine: -12, subLevel: .6, subOctave: -2, noiseLevel: .05, phase: .3, phaseRandom: 0, haasMs: -12.1, haasMix: .8, bassMono: 150 };
  const next = applyOperations(p, [{ type: 'synth.set', trackId, patch }]);
  for (const [key, value] of Object.entries(patch)) assert.equal(next.tracks[0].synth[key], value);
  for (const key of Object.keys(patch)) {
    const [min, max] = SYNTH_RANGES[key];
    for (const value of [min - 1, max + 1, NaN, Infinity, '2', ...(SYNTH_INTEGERS.has(key) ? [min + .5] : [])]) {
      assert.throws(() => applyOperations(p, [{ type: 'synth.set', trackId, patch: { [key]: value } }]));
    }
    for (const value of [min, max]) assert.doesNotThrow(() => applyOperations(p, [{ type: 'synth.set', trackId, patch: { [key]: value } }]));
  }
  assert.equal(p.revision, 0);
});
