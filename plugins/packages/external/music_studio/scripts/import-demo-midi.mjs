import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

/** Build-time SMF reader; runtime uses generated, offline project data. */
export function parseMidi(bytes) {
  const b = Buffer.from(bytes); let pos = 0;
  const need = (n, end = b.length) => { if (pos + n > end) throw Error('Truncated MIDI'); };
  const u16 = () => { need(2); const n = b.readUInt16BE(pos); pos += 2; return n; };
  const u32 = () => { need(4); const n = b.readUInt32BE(pos); pos += 4; return n; };
  const tag = () => { need(4); const s = b.toString('ascii', pos, pos + 4); pos += 4; return s; };
  const vlq = end => { let n = 0; for (let i = 0; i < 4; i++) { need(1, end); const c = b[pos++]; n = n * 128 + (c & 127); if (!(c & 128)) return n; } throw Error('Invalid MIDI VLQ'); };
  if (tag() !== 'MThd') throw Error('Missing MIDI header');
  const headerSize = u32(); if (headerSize < 6) throw Error('Invalid MIDI header');
  const format = u16(), trackCount = u16(), division = u16();
  if (format > 1 || !division || division & 0x8000) throw Error('Demo requires SMF format 0/1 and PPQ timing');
  need(headerSize - 6); pos += headerSize - 6;
  const tracks = [], tempos = [], signatures = [], inherited = new Map(); let lastTick = 0;
  for (let index = 0; index < trackCount; index++) {
    if (tag() !== 'MTrk') throw Error('Missing MIDI track');
    const size = u32(); need(size); const end = pos + size;
    let tick = 0, running = 0, port = 0, name = '';
    const active = new Map(), notes = [], configs = new Map(), pedal = new Map(), held = new Map();
    const config = channel => {
      if (!configs.has(channel)) configs.set(channel, { ...(inherited.get(`${port}:${channel}`) ?? { program: 0, volume: 100, pan: 64 }) });
      return configs.get(channel);
    };
    const releasePedal = channel => { for (const n of held.get(channel) ?? []) n.end = tick; held.delete(channel); };
    while (pos < end) {
      tick += vlq(end); need(1, end);
      let status = b[pos]; if (status & 128) pos++; else { if (!running) throw Error('Missing running status'); status = running; }
      if (status === 0xff) {
        need(1, end); const type = b[pos++], length = vlq(end); need(length, end);
        const data = b.subarray(pos, pos + length); pos += length;
        if (type === 3) name = data.toString('utf8').replaceAll('\0', '');
        if (type === 0x21 && length === 1) port = data[0];
        if (type === 0x51 && length === 3) tempos.push({ tick, microseconds: data.readUIntBE(0, 3) });
        if (type === 0x58 && length >= 2) signatures.push({ tick, numerator: data[0], denominator: 2 ** data[1] });
      } else if (status === 0xf0 || status === 0xf7) {
        running = 0; const length = vlq(end); need(length, end); pos += length;
      } else {
        if (status < 0x80 || status > 0xef) throw Error('Unsupported MIDI status');
        running = status; const op = status >> 4, channel = status & 15, count = op === 12 || op === 13 ? 1 : 2;
        need(count, end); const a = b[pos++], c = count === 2 ? b[pos++] : 0;
        if (a > 127 || c > 127) throw Error('Invalid MIDI data byte');
        const key = `${port}:${channel}:${a}`;
        if (op === 12) config(channel).program = a;
        if (op === 11) {
          if (a === 7) config(channel).volume = c;
          if (a === 10) config(channel).pan = c;
          if (a === 64) { pedal.set(channel, c >= 64); if (c < 64) releasePedal(channel); }
        }
        if (op === 9 && c) {
          const queue = active.get(key) ?? [];
          queue.push({ pitch: a, tick, velocity: c, channel, port, ...config(channel) }); active.set(key, queue);
        } else if (op === 8 || op === 9) {
          const note = active.get(key)?.shift();
          if (note) {
            note.end = tick; notes.push(note);
            if (pedal.get(channel)) { const queue = held.get(channel) ?? []; queue.push(note); held.set(channel, queue); }
          }
        }
      }
    }
    for (const channel of held.keys()) releasePedal(channel);
    if ([...active.values()].some(queue => queue.length)) throw Error(`Unclosed notes in track ${index}`);
    for (const [channel, value] of configs) inherited.set(`${port}:${channel}`, value);
    notes.sort((a, c) => a.tick - c.tick || a.pitch - c.pitch);
    lastTick = Math.max(lastTick, tick); tracks.push({ index, name, notes });
  }
  return { format, division, tracks, tempos: tempos.sort((a, c) => a.tick - c.tick), signatures, lastTick };
}

export const DEMO_SOURCES = [
  { id: 'the-world-revolving', title: 'THE WORLD REVOLVING', filename: 'the-world-revolving.mid', original: "THE_WORLD_REVOLVING_[Jevil's_Battle_Theme]_-_DELTARUNE.mid" },
  { id: 'field-of-hopes-and-dreams', title: 'The Field of Hopes and Dreams', filename: 'field-of-hopes-and-dreams.mid', original: 'The_Field_of_Hopes_and_Dreams_(Deltarune).mid' },
];
export async function importDemos() {
  const demos = [];
  for (const source of DEMO_SOURCES) {
    const bytes = await readFile(new URL(`../resources/midi/${source.filename}`, import.meta.url)), midi = parseMidi(bytes);
    const tempos = [...new Set(midi.tempos.map(t => t.microseconds))];
    if (tempos.length !== 1 || midi.tempos[0].tick !== 0) throw Error('Demo needs a constant tempo');
    const signature = midi.signatures[0] ?? { numerator: 4, denominator: 4 };
    if (midi.signatures.some(s => s.numerator !== signature.numerator || s.denominator !== signature.denominator)) throw Error('Demo needs a constant time signature');
    const beatsPerBar = signature.numerator * 4 / signature.denominator;
    if (!Number.isInteger(beatsPerBar) || beatsPerBar < 1 || beatsPerBar > 7) throw Error('Unsupported demo meter');
    const data = {
      ...source, sha256: createHash('sha256').update(bytes).digest('hex'), sourceTracks: midi.tracks.length,
      ppq: midi.division, tempoMicroseconds: tempos[0], beatsPerBar, endBeat: midi.lastTick / midi.division,
      tracks: midi.tracks.filter(t => t.notes.length).map(t => ({
        index: t.index, name: t.name, channel: t.notes[0].channel, program: t.notes[0].program,
        volume: t.notes[0].volume, pan: t.notes[0].pan,
        // Note-on tick, pedal-aware duration and velocity retained as integer MIDI values.
        notes: t.notes.map(n => [n.pitch, n.tick, n.end - n.tick, n.velocity]),
      })),
    };
    demos.push(data);
    console.log(`Imported ${source.title}: ${data.tracks.length} note tracks, ${data.tracks.reduce((n, t) => n + t.notes.length, 0)} notes, ${(data.endBeat * tempos[0] / 1e6).toFixed(3)} seconds`);
  }
  await writeFile(new URL('../src/shared/templates/midi-data.ts', import.meta.url),
    '// Generated by scripts/import-demo-midi.mjs from resources/midi/.\nexport const MIDI_DEMOS = ' + JSON.stringify(demos) + ';\n');
  return demos;
}
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await importDemos();
