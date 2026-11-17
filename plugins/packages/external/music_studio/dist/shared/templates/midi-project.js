"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MIDI_DEMO_DESCRIPTION = void 0;
exports.midiProject = midiProject;
const model_1 = require("../model");
const presets_1 = require("../presets");
const midi_data_1 = require("./midi-data");
const field_mix_1 = require("./field-mix");
exports.MIDI_DEMO_DESCRIPTION = '两首用户提供的完整 MIDI；保留旋律、伴奏和鼓轨，使用工作台合成音色试听。';
const GM_NAMES = { 0: '钢琴', 1: '明亮钢琴', 8: '钢片琴', 9: '钟琴', 10: '八音盒', 12: '马林巴', 21: '手风琴', 33: '指弹贝斯', 40: '小提琴', 41: '中提琴', 42: '大提琴', 44: '拨弦弦乐', 45: '弦乐颤音', 48: '弦乐群', 66: '次中音萨克斯', 72: '短笛', 75: '排箫', 80: '方波主奏', 82: '汽笛主奏', 119: '反向镲片' };
function presetFor(program, drums) {
    if (drums)
        return 'analog-kit';
    if (program <= 7)
        return 'fm-keys';
    if (program <= 15)
        return program === 12 ? 'soft-pluck' : 'starlight-bell';
    if (program <= 23)
        return 'organ';
    if (program >= 32 && program <= 39)
        return 'fm-bass';
    if (program >= 40 && program <= 51)
        return program === 44 ? 'soft-pluck' : 'string-ensemble';
    if (program >= 56 && program <= 71)
        return 'brass-ensemble';
    if (program >= 72 && program <= 79)
        return 'soft-pluck';
    if (program >= 80 && program <= 87)
        return 'neon-lead';
    if (program === 119)
        return 'aether-air';
    return 'fm-keys';
}
function midiProject(template) {
    const source = midi_data_1.MIDI_DEMOS.find(d => d.id === template);
    if (!source)
        throw new Error(`Unknown MIDI demo: ${template}`);
    const p = (0, model_1.emptyProject)(source.title);
    p.bpm = 60000000 / source.tempoMicroseconds;
    p.beatsPerBar = source.beatsPerBar;
    p.bars = Math.ceil(source.endBeat / p.beatsPerBar);
    p.masterGain = .58;
    p.loop = { enabled: false, start: 0, end: p.bars * p.beatsPerBar };
    p.tracks = source.tracks.map((input, i) => {
        const drums = input.channel === 9, t = (0, presets_1.makeTrack)(presetFor(input.program, drums), i);
        t.id = `midi_t${input.index}`;
        t.name = `${String(i + 1).padStart(2, '0')} / ${input.name || (drums ? '鼓组' : GM_NAMES[input.program] ?? `GM ${input.program + 1}`)}`;
        // Static CC7/CC10 controls map to the studio mixer. GM patches remain an approximation.
        t.gain = (drums ? .52 : input.program >= 32 && input.program <= 39 ? .48 : .32) * input.volume / 100;
        t.pan = Math.max(-1, Math.min(1, (input.pan - 64) / 63));
        if (!drums && input.program >= 72 && input.program <= 79) {
            t.synth.wave = 'triangle';
            t.synth.waveB = 'sine';
            t.synth.blend = .2;
            t.synth.sustain = .7;
            t.synth.decay = .1;
            t.synth.release = .08;
        }
        if (input.program >= 40 && input.program <= 51 && !drums) {
            t.synth.attack = .03;
            t.synth.release = .12;
            t.synth.oscBOctave = 0;
        }
        if (t.preset === 'neon-lead') {
            t.synth.wave = 'square';
            t.synth.waveB = 'triangle';
            t.synth.unison = t.synth.unisonB = 1;
            t.synth.release = .08;
        }
        if (!drums) {
            const reverb = (0, presets_1.makeEffect)('reverb');
            reverb.mix = .08;
            t.effects.push(reverb);
        }
        t.notes = input.notes.map(([pitch, tick, duration, velocity], j) => ({
            id: `m${input.index}n${j}`, pitch, start: tick / source.ppq,
            // The second source contains zero-length notes: use the editor's minimum gate.
            duration: Math.max(.01, duration / source.ppq), velocity: Math.max(.01, velocity / 127),
        }));
        return t;
    });
    // Navigation blocks only; no inferred verse/drop labels.
    p.sections = Array.from({ length: Math.ceil(p.bars / 16) }, (_, i) => ({
        id: `midi_section_${i}`, name: `MIDI · ${i * 16 + 1}–${Math.min(p.bars, (i + 1) * 16)}`,
        start: i * 16 * p.beatsPerBar, length: Math.min(16, p.bars - i * 16) * p.beatsPerBar,
        color: p.tracks[i % p.tracks.length].color,
    }));
    if (template === 'field-of-hopes-and-dreams') {
        const notes = new Map(p.tracks.map(track => [track.id, track.notes]));
        p.tracks = field_mix_1.FIELD_MIX.map(settings => ({ ...JSON.parse(JSON.stringify(settings)), notes: notes.get(settings.id) ?? [] }));
    }
    return p;
}
