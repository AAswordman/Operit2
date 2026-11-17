/** Beat positions are quarter-note units, MIDI pitch is 0..127, all gains linear. */
export type Engine = "spectral" | "fm" | "ensemble" | "drums" | "atmosphere";
export type Wave = "sine" | "triangle" | "sawtooth" | "square" | "glass" | "hollow";
export interface Synth {
  engine: Engine; wave: Wave; waveB: Wave; blend: number; detune: number; unison: number;
  attack: number; decay: number; sustain: number; release: number; cutoff: number; resonance: number;
  fmRatio: number; fmDepth: number; brightness: number;
  /** A uses unison/detune/width; B has independent voices, tuning and stereo spread. */
  unisonB: number; detuneB: number; widthB: number;
  oscBOctave: number; oscBSemitone: number; oscBFine: number;
  subLevel: number; subOctave: number; noiseLevel: number; phase: number; phaseRandom: number;
  /** Signed ms: positive delays R, negative delays L. Zero bypasses Haas. */
  haasMs: number; haasMix: number; bassMono: number;
  width: number; filterEnv: number; lfoRate: number; lfoDepth: number; pitchSweep: number;
}
export type EffectType = "eq" | "filter" | "drive" | "chorus" | "delay" | "reverb" | "compressor";
export interface Effect { id: string; type: EffectType; enabled: boolean; mix: number; params: Record<string, number> }
export interface Note { id: string; pitch: number; start: number; duration: number; velocity: number }
export type AutomationTarget = "level" | "pan" | "cutoff";
export interface AutomationPoint { beat: number; value: number }
export interface AutomationLane { target: AutomationTarget; points: AutomationPoint[] }
export interface Track {
  id: string; name: string; color: string; preset: string; synth: Synth; notes: Note[]; effects: Effect[];
  gain: number; pan: number; mute: boolean; solo: boolean; automation: AutomationLane[];
}
export interface Section { id: string; name: string; start: number; length: number; color: string }
export interface Project {
  format: "operit-music"; version: 1; id: string; revision: number; name: string; bpm: number;
  beatsPerBar: number; bars: number; swing: number; masterGain: number;
  loop: { enabled: boolean; start: number; end: number }; tracks: Track[]; sections: Section[];
  createdAt: string; updatedAt: string;
}
export interface Preset { id: string; name: string; category: string; description: string; synth: Synth }
export interface Operation { type: string; [key: string]: unknown }
export interface Request { action: string; [key: string]: unknown }
export interface Command { id: string; type: "play" | "pause" | "stop" | "seek" | "render"; beat?: number; projectId: string; revision: number; createdAt: number }
export interface PlaybackStatus { connected: boolean; unlocked: boolean; playing: boolean; beat: number; peak: number; voices: number; dropped: number; updatedAt: number; projectId: string }
export interface Receipt { id: string; state: "queued" | "done" | "failed"; message: string; path?: string }
export interface Snapshot { project: Project; projects: { id: string; name: string; revision: number }[]; status: PlaybackStatus; commands: Command[]; receipts: Receipt[] }
export const LIMITS = { tracks: 24, notes: 6000, totalNotes: 16000, bars: 256, voices: 64, projects: 30, renderSeconds: 240, effects: 6, jsonBytes: 8000000 };
export const COLORS = ["#b8f36b", "#b6a0ff", "#ffb56b", "#65dbe5", "#ed8ac5", "#879eff", "#dfd27d", "#88d5ae"];
export function uid(prefix = "id"): string { return `${prefix}_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 9)}`; }
export function copy<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T; }
export function emptyProject(name = "Untitled session"): Project {
  const now = new Date().toISOString();
  return { format: "operit-music", version: 1, id: uid("project"), revision: 0, name, bpm: 112, beatsPerBar: 4, bars: 8, swing: 0, masterGain: 0.72, loop: { enabled: true, start: 0, end: 32 }, tracks: [], sections: [], createdAt: now, updatedAt: now };
}
export function noteName(pitch: number): string { return ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"][pitch % 12] + (Math.floor(pitch / 12) - 1); }
export function audible(track: Track, project: Project): boolean { return !track.mute && (!project.tracks.some(t => t.solo) || track.solo); }
export function eventBeat(note: Note, swing: number): number { return note.start + (Math.abs(note.start % 1 - 0.5) < 0.001 ? swing * 0.5 : 0); }
