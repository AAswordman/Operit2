import { type Note, type Project, uid, LIMITS } from "./model";
import { midiProject } from "./templates/midi-project";
import { number, choice } from "./validation";
export const SCALES: Record<string, number[]> = { minor: [0, 2, 3, 5, 7, 8, 10], major: [0, 2, 4, 5, 7, 9, 11], dorian: [0, 2, 3, 5, 7, 9, 10], pentatonic: [0, 3, 5, 7, 10] };
export const PATTERNS = ["four-floor", "half-time", "breakbeat", "bass-pulse", "arpeggio", "chords", "texture"] as const;
export interface PatternOptions { kind: string; start: number; bars: number; root: number; scale: string; velocity: number; seed: number }
export function rng(seed: number): () => number { let s = seed | 0; return () => { s = (s * 1664525 + 1013904223) | 0; return (s >>> 0) / 4294967296; }; }
export function generatePattern(options: PatternOptions, beatsPerBar = 4): Note[] {
  const kind = choice(options.kind, PATTERNS, "pattern"); const start = number(options.start, 0, LIMITS.bars * 7, "start"); const bars = number(options.bars, 1, LIMITS.bars, "bars", true);
  const root = number(options.root, 12, 96, "root", true); const velocity = number(options.velocity, 0.01, 1, "velocity"); const scaleName = choice(options.scale, Object.keys(SCALES), "scale"); number(options.seed, 0, 2147483647, "seed", true);
  const random = rng(options.seed); const notes: Note[] = []; const scale = SCALES[scaleName];
  const add = (pitch: number, beat: number, duration: number, v = velocity): void => { notes.push({ id: uid("n"), pitch, start: start + beat, duration, velocity: Math.min(1, Math.max(0.01, v)) }); };
  for (let bar = 0; bar < bars; bar++) {
    const offset = bar * beatsPerBar; const degree = [0, 5, 3, 4][Math.floor(bar / 2) % 4] % scale.length;
    const chord = [0, 2, 4].map(d => root + scale[(degree + d) % scale.length] + Math.floor((degree + d) / scale.length) * 12);
    if (["four-floor", "half-time", "breakbeat"].includes(kind)) {
      for (let step = 0; step < beatsPerBar * 4; step++) {
        const beat = offset + step / 4;
        if ((kind === "four-floor" && step % 4 === 0) || (kind === "half-time" && (step === 0 || step === 6)) || (kind === "breakbeat" && [0, 6, 10].includes(step))) add(36, beat, 0.15);
        if ((kind === "half-time" && step === 8) || (kind !== "half-time" && step % 8 === 4)) add(38, beat, 0.12, velocity * 0.85);
        if (step % 2 === 0) add(step % 8 === 6 ? 46 : 42, beat, 0.1, velocity * (step % 4 === 0 ? 0.42 : 0.3));
        if (kind === "breakbeat" && step % 8 === 7) add(38, beat, 0.08, velocity * 0.24);
      }
    } else if (kind === "bass-pulse") {
      for (let i = 0; i < beatsPerBar * 2; i++) add(chord[0] - 12 + (i % 7 === 6 ? 12 : 0), offset + i / 2, 0.35, velocity * (i % 2 ? 0.72 : 1));
    } else if (kind === "arpeggio") {
      for (let i = 0; i < beatsPerBar * 2; i++) add(chord[i % chord.length] + (i % 4 === 3 ? 12 : 0), offset + i / 2, 0.28, velocity * (0.65 + random() * 0.35));
    } else {
      for (const pitch of chord) add(pitch, offset, beatsPerBar - 0.1, velocity * (kind === "texture" ? 0.7 : 1));
    }
  }
  return notes;
}
export const TEMPLATES = ["the-world-revolving", "field-of-hopes-and-dreams"] as const;
export const TEMPLATE_LABELS: Record<string, string> = { "the-world-revolving": "THE WORLD REVOLVING · MIDI", "field-of-hopes-and-dreams": "The Field of Hopes and Dreams · MIDI" };
export function demoProject(template: string = TEMPLATES[0]): Project {
  choice(template, TEMPLATES, "template");
  return midiProject(template);
}
