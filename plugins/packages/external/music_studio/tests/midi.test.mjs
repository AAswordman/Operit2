import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { parseMidi, DEMO_SOURCES } from '../scripts/import-demo-midi.mjs';
import { load } from './helpers.mjs';
const { MIDI_DEMOS } = await load('src/shared/templates/midi-data.ts');
const { demoProject } = await load('src/shared/composition.ts');
const { LIMITS } = await load('src/shared/model.ts');
const smf = events => Buffer.from([...'MThd'].map(c=>c.charCodeAt(0)).concat([0,0,0,6,0,0,0,1,1,224], [...'MTrk'].map(c=>c.charCodeAt(0)), [0,0,events.length>>8,events.length&255], events));
test('SMF running status, explicit note-off, velocity-zero release and same-pitch FIFO retain timing', () => {
  const midi = parseMidi(smf([0,0x90,60,100, 120,60,80, 120,60,0, 120,0x80,60,0, 0,0xff,0x2f,0]));
  assert.deepEqual(midi.tracks[0].notes.map(n => [n.pitch,n.tick,n.end,n.velocity]), [[60,0,240,100],[60,120,360,80]]);
});
test('SMF tempo, 2/4 meter, programs, pan, volume and sustain release survive conversion', () => {
  const midi = parseMidi(smf([0,0xff,0x51,3,7,0xa1,0x20, 0,0xff,0x58,4,2,2,24,8,
    0,0xc0,72, 0,0xb0,7,88, 0,10,32, 0,64,127, 0,0x90,60,100, 120,60,0,
    120,0xb0,64,0, 0,0xff,0x2f,0]));
  assert.equal(midi.tempos[0].microseconds,500000); assert.equal(midi.signatures[0].numerator,2);
  const n=midi.tracks[0].notes[0]; assert.equal(n.program,72); assert.equal(n.volume,88); assert.equal(n.pan,32); assert.equal(n.end,240);
});
test('SMF rejects truncated tracks, malformed running status, unsupported timing and unclosed notes', () => {
  assert.throws(()=>parseMidi(smf([0,0x90,60,100])) , /Unclosed/);
  assert.throws(()=>parseMidi(smf([0,60,100])), /running/);
  assert.throws(()=>parseMidi(smf([0,0xff,0x2f,0]).subarray(0,-1)), /Truncated/);
  const b=smf([0,0xff,0x2f,0]); b[12]=0x80; assert.throws(()=>parseMidi(b), /PPQ/);
});
for (const [i, source] of DEMO_SOURCES.entries()) test(`${source.title}: source hash and every note match the published template`, async () => {
  const bytes=await readFile(`resources/midi/${source.filename}`), midi=parseMidi(bytes), data=MIDI_DEMOS[i], p=demoProject(source.id);
  assert.equal(data.sha256,createHash('sha256').update(bytes).digest('hex'));
  assert.equal(p.tracks.length,midi.tracks.filter(t=>t.notes.length).length);
  for(const input of midi.tracks.filter(t=>t.notes.length)) {
    const track=p.tracks.find(t=>t.id===`midi_t${input.index}`);
    assert.deepEqual(track.notes.map(n=>[n.pitch,n.start,n.duration,n.velocity]),input.notes.map(n=>[n.pitch,n.tick/midi.division,Math.max(.01,(n.end-n.tick)/midi.division),Math.max(.01,n.velocity/127)]));
    assert.equal(track.synth.engine==='drums',input.notes[0].channel===9);
  }
  const json=JSON.stringify(p,null,2); assert.ok(Buffer.byteLength(json)<LIMITS.jsonBytes,'export must fit the import limit');
});
