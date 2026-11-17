"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SYNTH_INTEGERS = exports.SYNTH_RANGES = void 0;
exports.object = object;
exports.number = number;
exports.text = text;
exports.bool = bool;
exports.id = id;
exports.array = array;
exports.choice = choice;
exports.parseSynth = parseSynth;
exports.parseNote = parseNote;
exports.parseEffect = parseEffect;
exports.parseAutomation = parseAutomation;
exports.parseProject = parseProject;
exports.patchFields = patchFields;
const model_1 = require("./model");
const presets_1 = require("./presets");
function object(value, label = "object") { if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(`${label} must be an object`); return value; }
function number(value, min, max, label, integer = false) { if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max || (integer && !Number.isInteger(value)))
    throw new Error(`${label}: expected ${integer ? "integer" : "number"} ${min}..${max}`); return value; }
function text(value, label, max = 120) { if (typeof value !== "string" || !value.trim() || value.length > max)
    throw new Error(`${label}: expected non-empty text (≤${max})`); return value; }
function bool(value, label) { if (typeof value !== "boolean")
    throw new Error(`${label}: expected boolean`); return value; }
function id(value) { const s = text(value, "id", 100); if (!/^[a-zA-Z0-9_-]+$/.test(s))
    throw new Error("id: unsafe identifier"); return s; }
function array(value, max, label) { if (!Array.isArray(value) || value.length > max)
    throw new Error(`${label}: expected array ≤${max}`); return value; }
function choice(value, options, label) { if (!options.includes(value))
    throw new Error(`${label}: expected ${options.join("|")}`); return value; }
function unique(items, label) { if (new Set(items.map(i => i.id)).size !== items.length)
    throw new Error(`${label}: duplicate id`); return items; }
exports.SYNTH_RANGES = { blend: [0, 1], detune: [0, 50], unison: [1, 8], attack: [0.001, 3], decay: [0.01, 3], sustain: [0, 1], release: [0.02, 3], cutoff: [40, 18000], resonance: [0.1, 12], fmRatio: [0.25, 12], fmDepth: [0, 10], brightness: [0, 1], width: [0, 1], filterEnv: [-6, 6], lfoRate: [0, 16], lfoDepth: [0, 3], pitchSweep: [-48, 48], unisonB: [1, 8], detuneB: [0, 50], widthB: [0, 1], oscBOctave: [-3, 3], oscBSemitone: [-12, 12], oscBFine: [-100, 100], subLevel: [0, 1], subOctave: [-2, 0], noiseLevel: [0, 1], phase: [0, 1], phaseRandom: [0, 1], haasMs: [-35, 35], haasMix: [0, 1], bassMono: [0, 500] };
exports.SYNTH_INTEGERS = new Set(["unison", "unisonB", "oscBOctave", "oscBSemitone", "subOctave"]);
function parseSynth(raw) {
    const o = object(raw, "synth");
    const s = {
        engine: choice(o.engine, ["spectral", "fm", "ensemble", "drums", "atmosphere"], "engine"),
        wave: choice(o.wave, ["sine", "triangle", "sawtooth", "square", "glass", "hollow"], "wave"),
        waveB: choice(o.waveB, ["sine", "triangle", "sawtooth", "square", "glass", "hollow"], "waveB"),
    };
    // Preserve v1 oscillator settings: B previously inherited A's spread/count,
    // used 87% of its detune, +3 cents, and an octave up in ensemble mode.
    const defaults = { width: 0.65, filterEnv: 0, lfoRate: 0, lfoDepth: 0, pitchSweep: 0,
        unisonB: o.unison, detuneB: typeof o.detune === "number" ? o.detune * 0.87 : o.detune,
        widthB: o.width ?? 0.65, oscBOctave: o.engine === "ensemble" ? 1 : 0, oscBSemitone: 0,
        oscBFine: typeof o.detune === "number" && o.detune > 0 ? 3 : 0,
        subLevel: 0, subOctave: -1, noiseLevel: 0, phase: 0, phaseRandom: 1, haasMs: 0, haasMix: 1, bassMono: 0 };
    for (const [key, [min, max]] of Object.entries(exports.SYNTH_RANGES))
        s[key] = number(o[key] ?? defaults[key], min, max, key, exports.SYNTH_INTEGERS.has(key));
    return s;
}
function parseNote(raw, end) {
    const n = object(raw, "note");
    const start = number(n.start, 0, end - 0.001, "note.start");
    return { id: id(n.id), pitch: number(n.pitch, 0, 127, "pitch", true), start, duration: number(n.duration, 0.01, end - start, "duration"), velocity: number(n.velocity, 0.01, 1, "velocity") };
}
function parseEffect(raw) {
    const e = object(raw, "effect");
    const type = choice(e.type, Object.keys(presets_1.EFFECTS), "effect.type");
    const definition = presets_1.EFFECTS[type];
    const params = object(e.params, "effect.params");
    for (const k of Object.keys(params))
        if (!(k in definition.ranges))
            throw new Error(`Unknown ${type} parameter: ${k}`);
    const parsed = {};
    for (const [k, [min, max]] of Object.entries(definition.ranges))
        parsed[k] = number(params[k] ?? definition.defaults[k], min, max, `${type}.${k}`);
    return { id: id(e.id), type: type, enabled: bool(e.enabled, "effect.enabled"), mix: number(e.mix, 0, 1, "effect.mix"), params: parsed };
}
function parseAutomation(raw, end) {
    const lanes = array(raw, 3, "automation").map((entry) => {
        const lane = object(entry);
        const target = choice(lane.target, ["level", "pan", "cutoff"], "automation.target");
        const range = target === "level" ? [0, 1.5] : target === "pan" ? [-1, 1] : [40, 18000];
        const points = array(lane.points, 1024, "automation.points").map(raw => { const p = object(raw); return { beat: number(p.beat, 0, end, "automation.beat"), value: number(p.value, range[0], range[1], "automation.value") }; });
        if (!points.length || points.some((p, i) => i > 0 && p.beat <= points[i - 1].beat))
            throw new Error("Automation points must be nonempty and strictly ordered");
        return { target, points };
    });
    if (new Set(lanes.map(l => l.target)).size !== lanes.length)
        throw new Error("Duplicate automation target");
    return lanes;
}
function parseProject(raw) {
    const p = object(raw, "project");
    if (p.format !== "operit-music" || p.version !== 1)
        throw new Error("Unsupported project format/version");
    const bars = number(p.bars, 1, model_1.LIMITS.bars, "bars", true);
    const beatsPerBar = number(p.beatsPerBar, 1, 7, "beatsPerBar", true);
    const end = bars * beatsPerBar;
    const tracks = unique(array(p.tracks, model_1.LIMITS.tracks, "tracks").map((item) => {
        const t = object(item, "track");
        const color = text(t.color, "color", 7);
        if (!/^#[a-fA-F0-9]{6}$/.test(color))
            throw new Error("Expected #RRGGBB color");
        return { id: id(t.id), name: text(t.name, "track.name"), color, preset: text(t.preset, "preset"), synth: parseSynth(t.synth), notes: unique(array(t.notes, model_1.LIMITS.notes, "notes").map(n => parseNote(n, end)), "notes"), effects: unique(array(t.effects, model_1.LIMITS.effects, "effects").map(parseEffect), "effects"), gain: number(t.gain, 0, 1.5, "gain"), pan: number(t.pan, -1, 1, "pan"), mute: bool(t.mute, "mute"), solo: bool(t.solo, "solo"), automation: parseAutomation(t.automation ?? [], end) };
    }), "tracks");
    if (tracks.reduce((n, t) => n + t.notes.length, 0) > model_1.LIMITS.totalNotes)
        throw new Error(`Project has more than ${model_1.LIMITS.totalNotes} notes`);
    const loop = object(p.loop, "loop");
    const start = number(loop.start, 0, end - 0.01, "loop.start");
    const sections = unique(array(p.sections, 64, "sections").map((raw) => {
        const s = object(raw);
        const start = number(s.start, 0, end - 0.01, "section.start");
        const color = text(s.color, "section.color", 7);
        if (!/^#[a-fA-F0-9]{6}$/.test(color))
            throw new Error("Invalid section color");
        return { id: id(s.id), name: text(s.name, "section.name"), start, length: number(s.length, 0.01, end - start, "section.length"), color };
    }), "sections");
    const createdAt = text(p.createdAt, "createdAt", 40);
    const updatedAt = text(p.updatedAt, "updatedAt", 40);
    if (!Number.isFinite(Date.parse(createdAt)) || !Number.isFinite(Date.parse(updatedAt)))
        throw new Error("Invalid project timestamp");
    return { format: "operit-music", version: 1, id: id(p.id), revision: number(p.revision, 0, Number.MAX_SAFE_INTEGER, "revision", true), name: text(p.name, "name"), bpm: number(p.bpm, 40, 240, "bpm"), bars, beatsPerBar, swing: number(p.swing, 0, 0.65, "swing"), masterGain: number(p.masterGain, 0, 1, "masterGain"), loop: { enabled: bool(loop.enabled, "loop.enabled"), start, end: number(loop.end, start + 0.01, end, "loop.end") }, tracks, sections, createdAt, updatedAt };
}
/** Reject misspellings rather than silently accepting a no-op AI patch. */
function patchFields(target, raw, allowed) {
    const patch = object(raw, "patch");
    for (const k of Object.keys(patch))
        if (!allowed.includes(k))
            throw new Error(`Unsupported patch field: ${k}`);
    return Object.assign((0, model_1.copy)(target), patch);
}
