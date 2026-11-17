"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.TEMPLATE_LABELS = exports.TEMPLATES = exports.PATTERNS = exports.SCALES = void 0;
exports.rng = rng;
exports.generatePattern = generatePattern;
exports.demoProject = demoProject;
const model_1 = require("./model");
const midi_project_1 = require("./templates/midi-project");
const validation_1 = require("./validation");
exports.SCALES = { minor: [0, 2, 3, 5, 7, 8, 10], major: [0, 2, 4, 5, 7, 9, 11], dorian: [0, 2, 3, 5, 7, 9, 10], pentatonic: [0, 3, 5, 7, 10] };
exports.PATTERNS = ["four-floor", "half-time", "breakbeat", "bass-pulse", "arpeggio", "chords", "texture"];
function rng(seed) { let s = seed | 0; return () => { s = (s * 1664525 + 1013904223) | 0; return (s >>> 0) / 4294967296; }; }
function generatePattern(options, beatsPerBar = 4) {
    const kind = (0, validation_1.choice)(options.kind, exports.PATTERNS, "pattern");
    const start = (0, validation_1.number)(options.start, 0, model_1.LIMITS.bars * 7, "start");
    const bars = (0, validation_1.number)(options.bars, 1, model_1.LIMITS.bars, "bars", true);
    const root = (0, validation_1.number)(options.root, 12, 96, "root", true);
    const velocity = (0, validation_1.number)(options.velocity, 0.01, 1, "velocity");
    const scaleName = (0, validation_1.choice)(options.scale, Object.keys(exports.SCALES), "scale");
    (0, validation_1.number)(options.seed, 0, 2147483647, "seed", true);
    const random = rng(options.seed);
    const notes = [];
    const scale = exports.SCALES[scaleName];
    const add = (pitch, beat, duration, v = velocity) => { notes.push({ id: (0, model_1.uid)("n"), pitch, start: start + beat, duration, velocity: Math.min(1, Math.max(0.01, v)) }); };
    for (let bar = 0; bar < bars; bar++) {
        const offset = bar * beatsPerBar;
        const degree = [0, 5, 3, 4][Math.floor(bar / 2) % 4] % scale.length;
        const chord = [0, 2, 4].map(d => root + scale[(degree + d) % scale.length] + Math.floor((degree + d) / scale.length) * 12);
        if (["four-floor", "half-time", "breakbeat"].includes(kind)) {
            for (let step = 0; step < beatsPerBar * 4; step++) {
                const beat = offset + step / 4;
                if ((kind === "four-floor" && step % 4 === 0) || (kind === "half-time" && (step === 0 || step === 6)) || (kind === "breakbeat" && [0, 6, 10].includes(step)))
                    add(36, beat, 0.15);
                if ((kind === "half-time" && step === 8) || (kind !== "half-time" && step % 8 === 4))
                    add(38, beat, 0.12, velocity * 0.85);
                if (step % 2 === 0)
                    add(step % 8 === 6 ? 46 : 42, beat, 0.1, velocity * (step % 4 === 0 ? 0.42 : 0.3));
                if (kind === "breakbeat" && step % 8 === 7)
                    add(38, beat, 0.08, velocity * 0.24);
            }
        }
        else if (kind === "bass-pulse") {
            for (let i = 0; i < beatsPerBar * 2; i++)
                add(chord[0] - 12 + (i % 7 === 6 ? 12 : 0), offset + i / 2, 0.35, velocity * (i % 2 ? 0.72 : 1));
        }
        else if (kind === "arpeggio") {
            for (let i = 0; i < beatsPerBar * 2; i++)
                add(chord[i % chord.length] + (i % 4 === 3 ? 12 : 0), offset + i / 2, 0.28, velocity * (0.65 + random() * 0.35));
        }
        else {
            for (const pitch of chord)
                add(pitch, offset, beatsPerBar - 0.1, velocity * (kind === "texture" ? 0.7 : 1));
        }
    }
    return notes;
}
exports.TEMPLATES = ["the-world-revolving", "field-of-hopes-and-dreams"];
exports.TEMPLATE_LABELS = { "the-world-revolving": "THE WORLD REVOLVING · MIDI", "field-of-hopes-and-dreams": "The Field of Hopes and Dreams · MIDI" };
function demoProject(template = exports.TEMPLATES[0]) {
    (0, validation_1.choice)(template, exports.TEMPLATES, "template");
    return (0, midi_project_1.midiProject)(template);
}
