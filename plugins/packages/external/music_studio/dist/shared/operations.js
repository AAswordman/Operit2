"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.applyOperations = applyOperations;
const model_1 = require("./model");
const presets_1 = require("./presets");
const composition_1 = require("./composition");
const validation_1 = require("./validation");
/** Pure transactional reducer: no mutation is published until the whole batch validates. */
function applyOperations(project, operations) {
    let p = (0, model_1.copy)(project);
    (0, validation_1.array)(operations, 100, "operations");
    for (const raw of operations) {
        const op = (0, validation_1.object)(raw);
        const type = (0, validation_1.text)(op.type, "operation.type");
        if (type === "project.set")
            p = (0, validation_1.patchFields)(p, op.patch, ["name", "bpm", "beatsPerBar", "bars", "swing", "masterGain", "loop"]);
        else if (type === "track.add") {
            const t = (0, presets_1.makeTrack)((0, validation_1.text)(op.preset, "preset"), p.tracks.length);
            if (op.id !== undefined)
                t.id = (0, validation_1.text)(op.id, "id");
            if (op.name !== undefined)
                t.name = (0, validation_1.text)(op.name, "name");
            p.tracks.push(t);
        }
        else if (type === "sections.set")
            p.sections = (0, validation_1.array)(op.sections, 64, "sections").map(s => ({ id: (0, model_1.uid)("section"), ...(0, validation_1.object)(s) }));
        else {
            const t = p.tracks.find(t => t.id === op.trackId);
            if (!t)
                throw new Error(`Track not found: ${op.trackId}`);
            switch (type) {
                case "track.remove":
                    p.tracks = p.tracks.filter(a => a !== t);
                    break;
                case "track.set":
                    Object.assign(t, (0, validation_1.patchFields)(t, op.patch, ["name", "color", "gain", "pan", "mute", "solo"]));
                    break;
                case "track.preset": {
                    const preset = presets_1.PRESETS.find(a => a.id === op.preset);
                    if (!preset)
                        throw new Error("Unknown preset");
                    t.preset = preset.id;
                    t.synth = (0, model_1.copy)(preset.synth);
                    break;
                }
                case "automation.set":
                    t.automation = (0, validation_1.array)(op.lanes, 3, "lanes");
                    break;
                case "synth.set":
                    t.synth = (0, validation_1.patchFields)(t.synth, op.patch, Object.keys(t.synth));
                    break;
                case "notes.set":
                case "notes.add": {
                    const notes = (0, validation_1.array)(op.notes, model_1.LIMITS.notes, "notes").map(n => ({ id: (0, model_1.uid)("n"), ...(0, validation_1.object)(n) }));
                    t.notes = type === "notes.set" ? notes : [...t.notes, ...notes];
                    break;
                }
                case "notes.remove": {
                    const ids = (0, validation_1.array)(op.ids, model_1.LIMITS.notes, "ids");
                    t.notes = t.notes.filter(n => !ids.includes(n.id));
                    break;
                }
                case "notes.transform": {
                    const patch = (0, validation_1.object)(op.patch);
                    const allowed = ["transpose", "shift", "quantize", "velocity", "from", "to"];
                    for (const k of Object.keys(patch))
                        if (!allowed.includes(k))
                            throw new Error(`Unknown transform ${k}`);
                    const transpose = (0, validation_1.number)(patch.transpose ?? 0, -48, 48, "transpose", true);
                    const shift = (0, validation_1.number)(patch.shift ?? 0, -model_1.LIMITS.bars * 7, model_1.LIMITS.bars * 7, "shift");
                    const quantize = patch.quantize === undefined ? 0 : (0, validation_1.number)(patch.quantize, 0.0625, 4, "quantize");
                    const velocity = patch.velocity === undefined ? undefined : (0, validation_1.number)(patch.velocity, 0.01, 1, "velocity");
                    const from = (0, validation_1.number)(patch.from ?? 0, 0, model_1.LIMITS.bars * 7, "from");
                    const to = (0, validation_1.number)(patch.to ?? model_1.LIMITS.bars * 7, from, model_1.LIMITS.bars * 7, "to");
                    t.notes = t.notes.map(n => n.start < from || n.start >= to ? n : { ...n, pitch: n.pitch + transpose, start: quantize ? Math.round((n.start + shift) / quantize) * quantize : n.start + shift, velocity: velocity ?? n.velocity });
                    break;
                }
                case "pattern.generate": {
                    const options = { start: 0, bars: p.bars, root: 57, scale: "minor", velocity: 0.7, seed: 1, ...(0, validation_1.object)(op.options) };
                    const generated = (0, composition_1.generatePattern)(options, p.beatsPerBar);
                    if (op.append !== undefined && typeof op.append !== "boolean")
                        throw new Error("append must be boolean");
                    t.notes = op.append === true ? [...t.notes, ...generated] : generated;
                    break;
                }
                case "effect.add": {
                    const e = (0, presets_1.makeEffect)((0, validation_1.text)(op.effectType, "effectType"));
                    if (op.mix !== undefined)
                        e.mix = op.mix;
                    if (op.params !== undefined)
                        e.params = { ...e.params, ...(0, validation_1.object)(op.params) };
                    t.effects.push(e);
                    break;
                }
                case "effect.set": {
                    const i = t.effects.findIndex(e => e.id === op.effectId);
                    if (i < 0)
                        throw new Error("Effect not found");
                    const patch = (0, validation_1.object)(op.patch);
                    const e = (0, validation_1.patchFields)(t.effects[i], patch, ["mix", "enabled", "params"]);
                    if (patch.params)
                        e.params = { ...t.effects[i].params, ...(0, validation_1.object)(patch.params) };
                    t.effects[i] = e;
                    break;
                }
                case "effect.remove": {
                    if (!t.effects.some(e => e.id === op.effectId))
                        throw new Error("Effect not found");
                    t.effects = t.effects.filter(e => e.id !== op.effectId);
                    break;
                }
                default: throw new Error(`Unsupported operation: ${type}`);
            }
        }
    }
    p.revision = project.revision + 1;
    p.updatedAt = new Date().toISOString();
    return (0, validation_1.parseProject)(p);
}
