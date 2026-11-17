import test from 'node:test';
import assert from 'node:assert/strict';
import { load } from './helpers.mjs';
const { trackRegions, regionNotes, regionWindow } = await load('web/ui/regions.ts');
const { emptyProject } = await load('src/shared/model.ts');
const { makeTrack } = await load('src/shared/presets.ts');
const { demoProject } = await load('src/shared/composition.ts');
function fixture(events, sections = []) {
  const project = emptyProject(); project.bars = 12; project.sections = sections;
  const track = makeTrack('neon-lead'); track.id = 'track';
  track.notes = events.map(([start, duration = .5], index) => ({ id: `n${index}`, start, duration, pitch: 60, velocity: .7 }));
  return { project, track };
}
test('phrases group occupied bars, split rests and never mutate source notes', () => {
  const { project, track } = fixture([[17], [0], [4], [1], [20]]);
  const original = structuredClone(track);
  const regions = trackRegions(track, project);
  assert.deepEqual(regions.map(r => [r.start, r.end]), [[0, 8], [16, 24]]);
  assert.deepEqual(track, original);
  assert.deepEqual(regionNotes(track, regions[0]).map(n => n.start), [0, 4, 1]);
  assert.deepEqual(trackRegions({ ...track, notes: [] }, project), []);
  assert.deepEqual(trackRegions(track, project), regions, 'keys are stable across redraws');
});
test('authored sections create independently editable phrases, including non-bar boundaries', () => {
  const { project, track } = fixture([[0, 8], [4], [8], [9], [11], [16]], [
    { id: 'a', name: 'INTRO', start: 0, length: 8, color: '#b8f36b' },
    { id: 'b', name: 'DROP', start: 8, length: 2.5, color: '#b8f36b' },
  ]);
  const regions = trackRegions(track, project);
  assert.deepEqual(regions.map(r => [r.start, r.end]), [[0, 8], [8, 10.5], [10.5, 12], [16, 20]]);
  assert.equal(regions[0].name, 'INTRO'); assert.equal(regions[1].name, 'DROP');
  assert.ok(regions.every((r, i) => r.start < r.end && (!i || regions[i-1].end <= r.start)));
  const owned = regions.flatMap(r => regionNotes(track, r).map(n => n.id));
  assert.equal(new Set(owned).size, track.notes.length); assert.equal(owned.length, track.notes.length);
});
test('sustaining notes extend a phrase but are not owned by the next section', () => {
  const { project, track } = fixture([[0, 12], [16]], [{ id: 'a', name: 'A', start: 0, length: 8, color: '#b8f36b' }]);
  const regions = trackRegions(track, project);
  assert.deepEqual(regions.map(r => [r.start, r.end]), [[0, 8], [16, 20]]);
  assert.equal(regionNotes(track, regions[1]).length, 1);
});
test('piano viewport is clamped to the clicked phrase, not the whole song', () => {
  const region = { key: 'r', trackId: 't', name: 'DROP', start: 64, end: 74 };
  assert.deepEqual(regionWindow(region, 0, 4, 4), { start: 64, end: 74 });
  assert.deepEqual(regionWindow(region, 16, 2, 4), { start: 64, end: 72 });
  assert.deepEqual(regionWindow(region, 18, 8, 4), { start: 72, end: 74 });
  assert.deepEqual(regionWindow({ ...region, start: 64.5, end: 66.5 }, 100, 4, 4), { start: 64.5, end: 66.5 });
});
test('all demo notes belong to exactly one derived region with bounded coordinates', () => {
  const project = demoProject();
  for (const track of project.tracks) {
    const regions = trackRegions(track, project);
    assert.ok(regions.every(r => r.start >= 0 && r.end <= project.bars*project.beatsPerBar && r.start < r.end));
    const ids = regions.flatMap(r => regionNotes(track, r).map(n => n.id));
    assert.equal(ids.length, track.notes.length); assert.equal(new Set(ids).size, track.notes.length);
  }
});
