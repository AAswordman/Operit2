import { LIMITS, type Project, type Synth, type Note, type Effect, type Track, type Section, type AutomationLane, copy } from "./model";
import { EFFECTS } from "./presets";
export function object(value: unknown, label = "object"): Record<string, unknown> { if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} must be an object`); return value as Record<string, unknown>; }
export function number(value: unknown, min: number, max: number, label: string, integer = false): number { if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max || (integer && !Number.isInteger(value))) throw new Error(`${label}: expected ${integer ? "integer" : "number"} ${min}..${max}`); return value; }
export function text(value: unknown, label: string, max = 120): string { if (typeof value !== "string" || !value.trim() || value.length > max) throw new Error(`${label}: expected non-empty text (≤${max})`); return value; }
export function bool(value: unknown, label: string): boolean { if (typeof value !== "boolean") throw new Error(`${label}: expected boolean`); return value; }
export function id(value: unknown): string { const s = text(value, "id", 100); if (!/^[a-zA-Z0-9_-]+$/.test(s)) throw new Error("id: unsafe identifier"); return s; }
export function array(value: unknown, max: number, label: string): unknown[] { if (!Array.isArray(value) || value.length > max) throw new Error(`${label}: expected array ≤${max}`); return value; }
export function choice<T extends string>(value: unknown, options: readonly T[], label: string): T { if (!options.includes(value as T)) throw new Error(`${label}: expected ${options.join("|")}`); return value as T; }
function unique<T extends { id: string }>(items: T[], label: string): T[] { if (new Set(items.map(i => i.id)).size !== items.length) throw new Error(`${label}: duplicate id`); return items; }
export const SYNTH_RANGES: Record<string, [number, number]> = { blend: [0, 1], detune: [0, 50], unison: [1, 8], attack: [0.001, 3], decay: [0.01, 3], sustain: [0, 1], release: [0.02, 3], cutoff: [40, 18000], resonance: [0.1, 12], fmRatio: [0.25, 12], fmDepth: [0, 10], brightness: [0, 1], width: [0, 1], filterEnv: [-6, 6], lfoRate: [0, 16], lfoDepth: [0, 3], pitchSweep: [-48, 48], unisonB: [1, 8], detuneB: [0, 50], widthB: [0, 1], oscBOctave: [-3, 3], oscBSemitone: [-12, 12], oscBFine: [-100, 100], subLevel: [0, 1], subOctave: [-2, 0], noiseLevel: [0, 1], phase: [0, 1], phaseRandom: [0, 1], haasMs: [-35, 35], haasMix: [0, 1], bassMono: [0, 500] };
export const SYNTH_INTEGERS = new Set(["unison", "unisonB", "oscBOctave", "oscBSemitone", "subOctave"]);
export function parseSynth(raw: unknown): Synth {
  const o = object(raw, "synth"); const s: Record<string, unknown> = {
    engine: choice(o.engine, ["spectral", "fm", "ensemble", "drums", "atmosphere"], "engine"),
    wave: choice(o.wave, ["sine", "triangle", "sawtooth", "square", "glass", "hollow"], "wave"),
    waveB: choice(o.waveB, ["sine", "triangle", "sawtooth", "square", "glass", "hollow"], "waveB"),
  };
  // Preserve v1 oscillator settings: B previously inherited A's spread/count,
  // used 87% of its detune, +3 cents, and an octave up in ensemble mode.
  const defaults: Record<string, unknown> = { width: 0.65, filterEnv: 0, lfoRate: 0, lfoDepth: 0, pitchSweep: 0,
    unisonB: o.unison, detuneB: typeof o.detune === "number" ? o.detune * 0.87 : o.detune,
    widthB: o.width ?? 0.65, oscBOctave: o.engine === "ensemble" ? 1 : 0, oscBSemitone: 0,
    oscBFine: typeof o.detune === "number" && o.detune > 0 ? 3 : 0,
    subLevel: 0, subOctave: -1, noiseLevel: 0, phase: 0, phaseRandom: 1, haasMs: 0, haasMix: 1, bassMono: 0 };
  for (const [key, [min, max]] of Object.entries(SYNTH_RANGES)) s[key] = number(o[key] ?? defaults[key], min, max, key, SYNTH_INTEGERS.has(key));
  return s as unknown as Synth;
}
export function parseNote(raw: unknown, end: number): Note {
  const n = object(raw, "note"); const start = number(n.start, 0, end - 0.001, "note.start");
  return { id: id(n.id), pitch: number(n.pitch, 0, 127, "pitch", true), start, duration: number(n.duration, 0.01, end - start, "duration"), velocity: number(n.velocity, 0.01, 1, "velocity") };
}
export function parseEffect(raw: unknown): Effect {
  const e = object(raw, "effect"); const type = choice(e.type, Object.keys(EFFECTS), "effect.type");
  const definition = EFFECTS[type]; const params = object(e.params, "effect.params");
  for (const k of Object.keys(params)) if (!(k in definition.ranges)) throw new Error(`Unknown ${type} parameter: ${k}`);
  const parsed: Record<string, number> = {};
  for (const [k, [min, max]] of Object.entries(definition.ranges)) parsed[k] = number(params[k] ?? definition.defaults[k], min, max, `${type}.${k}`);
  return { id: id(e.id), type: type as Effect["type"], enabled: bool(e.enabled, "effect.enabled"), mix: number(e.mix, 0, 1, "effect.mix"), params: parsed };
}
export function parseAutomation(raw: unknown, end: number): AutomationLane[] {
  const lanes = array(raw, 3, "automation").map((entry): AutomationLane => {
    const lane = object(entry); const target = choice(lane.target, ["level", "pan", "cutoff"], "automation.target");
    const range = target === "level" ? [0, 1.5] : target === "pan" ? [-1, 1] : [40, 18000];
    const points = array(lane.points, 1024, "automation.points").map(raw => { const p = object(raw); return { beat: number(p.beat, 0, end, "automation.beat"), value: number(p.value, range[0], range[1], "automation.value") }; });
    if (!points.length || points.some((p, i) => i > 0 && p.beat <= points[i - 1].beat)) throw new Error("Automation points must be nonempty and strictly ordered");
    return { target, points };
  });
  if (new Set(lanes.map(l => l.target)).size !== lanes.length) throw new Error("Duplicate automation target");
  return lanes;
}
export function parseProject(raw: unknown): Project {
  const p = object(raw, "project"); if (p.format !== "operit-music" || p.version !== 1) throw new Error("Unsupported project format/version");
  const bars = number(p.bars, 1, LIMITS.bars, "bars", true); const beatsPerBar = number(p.beatsPerBar, 1, 7, "beatsPerBar", true); const end = bars * beatsPerBar;
  const tracks = unique(array(p.tracks, LIMITS.tracks, "tracks").map((item): Track => {
    const t = object(item, "track"); const color = text(t.color, "color", 7); if (!/^#[a-fA-F0-9]{6}$/.test(color)) throw new Error("Expected #RRGGBB color");
    return { id: id(t.id), name: text(t.name, "track.name"), color, preset: text(t.preset, "preset"), synth: parseSynth(t.synth), notes: unique(array(t.notes, LIMITS.notes, "notes").map(n => parseNote(n, end)), "notes"), effects: unique(array(t.effects, LIMITS.effects, "effects").map(parseEffect), "effects"), gain: number(t.gain, 0, 1.5, "gain"), pan: number(t.pan, -1, 1, "pan"), mute: bool(t.mute, "mute"), solo: bool(t.solo, "solo"), automation: parseAutomation(t.automation ?? [], end) };
  }), "tracks");
  if (tracks.reduce((n, t) => n + t.notes.length, 0) > LIMITS.totalNotes) throw new Error(`Project has more than ${LIMITS.totalNotes} notes`);
  const loop = object(p.loop, "loop"); const start = number(loop.start, 0, end - 0.01, "loop.start");
  const sections = unique(array(p.sections, 64, "sections").map((raw): Section => {
    const s = object(raw); const start = number(s.start, 0, end - 0.01, "section.start"); const color = text(s.color, "section.color", 7); if (!/^#[a-fA-F0-9]{6}$/.test(color)) throw new Error("Invalid section color");
    return { id: id(s.id), name: text(s.name, "section.name"), start, length: number(s.length, 0.01, end - start, "section.length"), color };
  }), "sections");
  const createdAt = text(p.createdAt, "createdAt", 40); const updatedAt = text(p.updatedAt, "updatedAt", 40);
  if (!Number.isFinite(Date.parse(createdAt)) || !Number.isFinite(Date.parse(updatedAt))) throw new Error("Invalid project timestamp");
  return { format: "operit-music", version: 1, id: id(p.id), revision: number(p.revision, 0, Number.MAX_SAFE_INTEGER, "revision", true), name: text(p.name, "name"), bpm: number(p.bpm, 40, 240, "bpm"), bars, beatsPerBar, swing: number(p.swing, 0, 0.65, "swing"), masterGain: number(p.masterGain, 0, 1, "masterGain"), loop: { enabled: bool(loop.enabled, "loop.enabled"), start, end: number(loop.end, start + 0.01, end, "loop.end") }, tracks, sections, createdAt, updatedAt };
}
/** Reject misspellings rather than silently accepting a no-op AI patch. */
export function patchFields<T extends object>(target: T, raw: unknown, allowed: string[]): T {
  const patch = object(raw, "patch"); for (const k of Object.keys(patch)) if (!allowed.includes(k)) throw new Error(`Unsupported patch field: ${k}`);
  return Object.assign(copy(target), patch);
}
