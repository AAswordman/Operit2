import { type Operation, type Project, type Note, uid, copy, LIMITS } from "./model";
import { makeTrack, makeEffect, PRESETS } from "./presets";
import { generatePattern, type PatternOptions } from "./composition";
import { parseProject, patchFields, object, array, number, text } from "./validation";
/** Pure transactional reducer: no mutation is published until the whole batch validates. */
export function applyOperations(project: Project, operations: Operation[]): Project {
  let p = copy(project); array(operations, 100, "operations");
  for (const raw of operations) {
    const op = object(raw); const type = text(op.type, "operation.type");
    if (type === "project.set") p = patchFields(p, op.patch, ["name", "bpm", "beatsPerBar", "bars", "swing", "masterGain", "loop"]);
    else if (type === "track.add") { const t = makeTrack(text(op.preset, "preset"), p.tracks.length); if (op.id !== undefined) t.id = text(op.id, "id"); if (op.name !== undefined) t.name = text(op.name, "name"); p.tracks.push(t); }
    else if (type === "sections.set") p.sections = array(op.sections, 64, "sections").map(s => ({ id: uid("section"), ...object(s) })) as Project["sections"];
    else {
      const t = p.tracks.find(t => t.id === op.trackId); if (!t) throw new Error(`Track not found: ${op.trackId}`);
      switch (type) {
        case "track.remove": p.tracks = p.tracks.filter(a => a !== t); break;
        case "track.set": Object.assign(t, patchFields(t, op.patch, ["name", "color", "gain", "pan", "mute", "solo"])); break;
        case "track.preset": { const preset = PRESETS.find(a => a.id === op.preset); if (!preset) throw new Error("Unknown preset"); t.preset = preset.id; t.synth = copy(preset.synth); break; }
        case "automation.set": t.automation = array(op.lanes, 3, "lanes") as import("./model").AutomationLane[]; break;
        case "synth.set": t.synth = patchFields(t.synth, op.patch, Object.keys(t.synth)); break;
        case "notes.set": case "notes.add": { const notes = array(op.notes, LIMITS.notes, "notes").map(n => ({ id: uid("n"), ...object(n) })) as Note[]; t.notes = type === "notes.set" ? notes : [...t.notes, ...notes]; break; }
        case "notes.remove": { const ids = array(op.ids, LIMITS.notes, "ids"); t.notes = t.notes.filter(n => !ids.includes(n.id)); break; }
        case "notes.transform": {
          const patch = object(op.patch); const allowed = ["transpose", "shift", "quantize", "velocity", "from", "to"];
          for (const k of Object.keys(patch)) if (!allowed.includes(k)) throw new Error(`Unknown transform ${k}`);
          const transpose = number(patch.transpose ?? 0, -48, 48, "transpose", true); const shift = number(patch.shift ?? 0, -LIMITS.bars * 7, LIMITS.bars * 7, "shift");
          const quantize = patch.quantize === undefined ? 0 : number(patch.quantize, 0.0625, 4, "quantize");
          const velocity = patch.velocity === undefined ? undefined : number(patch.velocity, 0.01, 1, "velocity");
          const from = number(patch.from ?? 0, 0, LIMITS.bars * 7, "from"); const to = number(patch.to ?? LIMITS.bars * 7, from, LIMITS.bars * 7, "to");
          t.notes = t.notes.map(n => n.start < from || n.start >= to ? n : { ...n, pitch: n.pitch + transpose, start: quantize ? Math.round((n.start + shift) / quantize) * quantize : n.start + shift, velocity: velocity ?? n.velocity }); break;
        }
        case "pattern.generate": {
          const options = { start: 0, bars: p.bars, root: 57, scale: "minor", velocity: 0.7, seed: 1, ...object(op.options) } as unknown as PatternOptions;
          const generated = generatePattern(options, p.beatsPerBar);
          if (op.append !== undefined && typeof op.append !== "boolean") throw new Error("append must be boolean");
          t.notes = op.append === true ? [...t.notes, ...generated] : generated; break;
        }
        case "effect.add": { const e = makeEffect(text(op.effectType, "effectType")); if (op.mix !== undefined) e.mix = op.mix as number; if (op.params !== undefined) e.params = { ...e.params, ...object(op.params) } as Record<string, number>; t.effects.push(e); break; }
        case "effect.set": { const i = t.effects.findIndex(e => e.id === op.effectId); if (i < 0) throw new Error("Effect not found"); const patch = object(op.patch); const e = patchFields(t.effects[i], patch, ["mix", "enabled", "params"]); if (patch.params) e.params = { ...t.effects[i].params, ...object(patch.params) } as Record<string, number>; t.effects[i] = e; break; }
        case "effect.remove": { if (!t.effects.some(e => e.id === op.effectId)) throw new Error("Effect not found"); t.effects = t.effects.filter(e => e.id !== op.effectId); break; }
        default: throw new Error(`Unsupported operation: ${type}`);
      }
    }
  }
  p.revision = project.revision + 1; p.updatedAt = new Date().toISOString(); return parseProject(p);
}
