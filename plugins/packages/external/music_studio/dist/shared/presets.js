"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.EFFECTS = exports.PRESETS = void 0;
exports.makeEffect = makeEffect;
exports.makeTrack = makeTrack;
const model_1 = require("./model");
const base = { engine: "spectral", wave: "sawtooth", waveB: "triangle", blend: 0.25, detune: 12, unison: 2, attack: 0.008, decay: 0.22, sustain: 0.6, release: 0.3, cutoff: 6500, resonance: 0.7, fmRatio: 2, fmDepth: 2, brightness: 0.65, width: 0.65, filterEnv: 0, lfoRate: 0, lfoDepth: 0, pitchSweep: 0, unisonB: 2, detuneB: 10.44, widthB: 0.65, oscBOctave: 0, oscBSemitone: 0, oscBFine: 3, subLevel: 0, subOctave: -1, noiseLevel: 0, phase: 0, phaseRandom: 1, haasMs: 0, haasMix: 1, bassMono: 0 };
function preset(id, name, category, description, options) {
    const a = { ...base, ...options };
    return { id, name, category, description, synth: { ...a,
            unisonB: options.unisonB ?? a.unison, detuneB: options.detuneB ?? a.detune * .87,
            widthB: options.widthB ?? a.width, oscBOctave: options.oscBOctave ?? (a.engine === "ensemble" ? 1 : 0),
            oscBFine: options.oscBFine ?? (a.detune > 0 ? 3 : 0) } };
}
/** Original oscillator patches. No bundled recordings, SoundFonts, or third-party presets. */
exports.PRESETS = [
    preset("thick-reese", "Reese / 厚低音叠层", "Bass", "独立双锯齿叠层 + 居中 Sub + 低频收窄", { wave: "sawtooth", waveB: "sawtooth", blend: .45, unison: 5, unisonB: 4, detune: 22, detuneB: 15, width: .8, widthB: .65, oscBFine: -7, subLevel: .45, phaseRandom: .6, bassMono: 150, cutoff: 1600, attack: .005, decay: .25, sustain: .75, release: .14 }),
    preset("haas-bass", "Haas / 宽中频低音", "Bass", "右侧 12ms 延迟、单声道 Sub；折叠单声道时请检查相位", { wave: "sawtooth", waveB: "square", blend: .28, unison: 3, unisonB: 2, detune: 14, detuneB: 8, subLevel: .4, bassMono: 180, haasMs: 12, haasMix: 1, cutoff: 2400, attack: .004, sustain: .65, release: .12 }),
    preset("neon-lead", "Neon / 霓虹主奏", "Lead", "双振荡器失谐主奏", { wave: "sawtooth", waveB: "square", cutoff: 4200, sustain: 0.4 }),
    preset("glass-pluck", "Glass / 玻璃拨弦", "Keys", "加法谐波 + 短包络", { wave: "glass", waveB: "sine", attack: 0.002, decay: 0.38, sustain: 0.05, release: 0.22, unison: 1 }),
    preset("acid-bass", "Acid / 酸性贝斯", "Bass", "共振低通锯齿贝斯", { wave: "sawtooth", waveB: "square", cutoff: 750, resonance: 5, attack: 0.003, decay: 0.18, sustain: 0.2, release: 0.09, unison: 1 }),
    preset("sub-bass", "Sub / 深潜", "Bass", "正弦 + 三角波低频", { wave: "sine", waveB: "triangle", blend: 0.16, cutoff: 900, attack: 0.005, decay: 0.15, sustain: 0.85, release: 0.1, unison: 1 }),
    preset("reese-bass", "Reese / 双生", "Bass", "失谐锯齿贝斯", { cutoff: 1100, detune: 20, unison: 3, release: 0.14 }),
    preset("aurora-pad", "Aurora / 极光织体", "Texture", "慢起音宽阔合成织体", { wave: "hollow", waveB: "sawtooth", blend: 0.22, attack: 0.8, decay: 0.7, sustain: 0.65, release: 1.7, cutoff: 2600, detune: 18, unison: 3 }),
    preset("cloud-pad", "Cloud / 云层", "Texture", "柔和正弦叠层", { engine: "ensemble", wave: "sine", waveB: "triangle", attack: 1.1, release: 2.2, cutoff: 1800, unison: 3 }),
    preset("string-ensemble", "Strings / 弦乐群", "Ensemble", "纯振荡器模拟弦乐，非采样真实乐器", { engine: "ensemble", wave: "sawtooth", waveB: "triangle", attack: 0.3, release: 0.8, cutoff: 3300, detune: 9, unison: 3 }),
    preset("brass-ensemble", "Brass / 铜管群", "Ensemble", "减法合成铜管色彩", { engine: "ensemble", wave: "square", waveB: "sawtooth", attack: 0.08, decay: 0.25, release: 0.24, cutoff: 2000, unison: 2 }),
    preset("organ", "Organ / 风琴", "Keys", "空心谐波风琴", { wave: "hollow", waveB: "sine", unison: 1, attack: 0.008, decay: 0.05, sustain: 0.9, release: 0.06, cutoff: 8500 }),
    preset("fm-keys", "FM / 电钢琴", "Keys", "双算子调频电钢音色", { engine: "fm", wave: "sine", unison: 1, attack: 0.002, decay: 0.9, sustain: 0.12, release: 0.5, fmRatio: 1, fmDepth: 2.4, cutoff: 9000 }),
    preset("fm-bell", "Bell / 星铃", "Keys", "非整数频比金属钟声", { engine: "fm", unison: 1, attack: 0.002, decay: 1.1, sustain: 0.02, release: 1.1, fmRatio: 3.5, fmDepth: 4, cutoff: 12000 }),
    preset("fm-bass", "FM / 弹性低音", "Bass", "调频短促贝斯", { engine: "fm", unison: 1, attack: 0.003, decay: 0.22, sustain: 0.12, release: 0.08, fmRatio: 2, fmDepth: 3, cutoff: 2000 }),
    preset("soft-pluck", "Pluck / 木质拨弦", "Keys", "三角波短包络拨弦", { wave: "triangle", waveB: "sine", unison: 1, attack: 0.002, decay: 0.28, sustain: 0.02, release: 0.18, cutoff: 3600 }),
    preset("drift-texture", "Drift / 漂移", "Texture", "宽失谐空心织体", { engine: "ensemble", wave: "hollow", waveB: "glass", unison: 4, detune: 30, attack: 1.2, sustain: 0.5, release: 2, cutoff: 4000 }),
    preset("eclipse-chords", "Eclipse / 超锯齿和弦", "Chords", "宽立体声多锯齿，melodic dubstep 和弦主体", { wave: "sawtooth", waveB: "sawtooth", blend: 0.38, unison: 3, detune: 18, width: 0.95, attack: 0.015, decay: 0.28, sustain: 0.68, release: 0.26, cutoff: 6200, filterEnv: 0.7, resonance: 0.65 }),
    preset("halo-chords", "Halo / 空灵和声层", "Chords", "高八度泛音层，适合加九与挂留和弦", { wave: "hollow", waveB: "triangle", blend: 0.4, unison: 2, detune: 9, width: 0.8, attack: 0.045, decay: 0.4, sustain: 0.55, release: 0.5, cutoff: 4800 }),
    preset("comet-lead", "Comet / 彗星主旋律", "Lead", "明亮宽主奏与细微滤波颤动", { wave: "sawtooth", waveB: "square", blend: 0.16, unison: 2, detune: 9, width: 0.55, attack: 0.012, decay: 0.3, sustain: 0.66, release: 0.19, cutoff: 5700, lfoRate: 5, lfoDepth: 0.05 }),
    preset("prism-arp", "Prism / 棱镜琶音", "Keys", "短促明亮 trance 琶音", { wave: "sawtooth", waveB: "glass", blend: 0.4, unison: 1, attack: 0.003, decay: 0.16, sustain: 0.08, release: 0.13, cutoff: 2200, filterEnv: 2.2, resonance: 1.2 }),
    preset("titan-sub", "Titan / 单声道低频", "Bass", "无失谐居中正弦与弱三角谐波，保证低频相容", { wave: "sine", waveB: "triangle", blend: 0.13, unison: 1, detune: 0, width: 0, attack: 0.008, decay: 0.07, sustain: 0.94, release: 0.08, cutoff: 220 }),
    preset("fault-bass", "Fault / 断层咆哮", "Bass", "FM + 低通调制，点缀式 dubstep 重音", { engine: "fm", unison: 1, attack: 0.007, decay: 0.34, sustain: 0.45, release: 0.09, cutoff: 700, resonance: 2.2, fmRatio: 1, fmDepth: 4.5, filterEnv: 2, lfoRate: 6.25, lfoDepth: 1.4, width: 0 }),
    preset("horizon-reese", "Horizon / 宽频低音层", "Bass", "上层中低频失谐，需与居中 sub 分频搭配", { wave: "sawtooth", waveB: "square", blend: 0.24, unison: 2, detune: 14, width: 0.45, attack: 0.009, decay: 0.12, sustain: 0.6, release: 0.1, cutoff: 1200, filterEnv: 0.8 }),
    preset("aether-air", "Aether / 空气与风", "Texture", "三段共振气流、缓慢漂移与同调谐波，无录音素材", { engine: "atmosphere", noiseLevel: .7, blend: .12, unison: 1, attack: 1.8, decay: 1.2, sustain: 0.5, release: 2.3, cutoff: 2800, resonance: 0.6, brightness: 0.5, width: 0.9, lfoRate: 0.17, lfoDepth: 0.45 }),
    preset("ascension-riser", "Ascension / 升空", "Texture", "分频气流渐开与上行谐波，适用于 build-up", { engine: "atmosphere", noiseLevel: .9, blend: .15, unison: 1, attack: 3, decay: 0.2, sustain: 0.9, release: 0.2, cutoff: 11000, filterEnv: -6, pitchSweep: 24, brightness: 0.9, width: 1 }),
    preset("afterglow-pad", "Afterglow / 余晖弦幕", "Texture", "慢包络暖锯齿弦乐与缓慢滤波漂移", { engine: "ensemble", wave: "sawtooth", waveB: "hollow", blend: 0.35, unison: 2, detune: 7, width: 0.9, attack: 0.9, decay: 0.9, sustain: 0.65, release: 1.8, cutoff: 1900, lfoRate: 0.12, lfoDepth: 0.18 }),
    preset("starlight-bell", "Starlight / 星尘", "Keys", "清脆 FM 星铃，适合呼应旋律", { engine: "fm", unison: 1, attack: 0.002, decay: 0.65, sustain: 0.035, release: 0.8, cutoff: 9000, fmRatio: 2, fmDepth: 1.8 }),
    preset("cinematic-kit", "Impact / 电影打击", "Drums", "鼓、通鼓、噪声冲击与金属镲", { engine: "drums", unison: 1, decay: 0.27, brightness: 0.68, release: 0.12 }),
    preset("analog-kit", "Circuit / 模拟鼓组", "Drums", "GM: 底鼓、军鼓、拍手、踩镲、六档通鼓、49/57等镲片、56牛铃、60/61邦戈、62–64康加", { engine: "drums", unison: 1, attack: 0.001, decay: 0.18, sustain: 0, release: 0.05 }),
    preset("tight-kit", "Tight / 紧致鼓组", "Drums", "短促电子鼓", { engine: "drums", brightness: 0.85, unison: 1, decay: 0.1, release: 0.04 }),
    preset("deep-kit", "Deep / 深鼓", "Drums", "长尾低频电子鼓", { engine: "drums", brightness: 0.4, unison: 1, decay: 0.4, release: 0.1 }),
];
exports.EFFECTS = {
    eq: { name: "三段 EQ", defaults: { low: 0, mid: 0, high: 0 }, ranges: { low: [-18, 18], mid: [-18, 18], high: [-18, 18] } },
    filter: { name: "共振滤波", defaults: { cutoff: 4000, resonance: 1 }, ranges: { cutoff: [40, 18000], resonance: [0.1, 12] } },
    drive: { name: "软饱和", defaults: { amount: 3 }, ranges: { amount: [1, 30] } },
    chorus: { name: "立体声合唱", defaults: { rate: 0.8, depth: 0.003 }, ranges: { rate: [0.1, 5], depth: [0.001, 0.01] } },
    delay: { name: "立体声节拍延迟", defaults: { beats: 0.75, feedback: 0.3, pingPong: 0 }, ranges: { beats: [0.125, 2], feedback: [0, 0.75], pingPong: [0, 1] } },
    reverb: { name: "算法空间", defaults: { seconds: 1.5, decay: 2.5 }, ranges: { seconds: [0.1, 3], decay: [1, 6] } },
    compressor: { name: "动态压缩", defaults: { threshold: -20, ratio: 4, attack: 0.015, release: 0.2 }, ranges: { threshold: [-60, 0], ratio: [1, 20], attack: [0.001, 0.2], release: [0.02, 1] } },
};
function makeEffect(type) {
    if (!exports.EFFECTS[type])
        throw new Error(`Unknown effect: ${type}`);
    return { id: (0, model_1.uid)("fx"), type: type, enabled: true, mix: ["reverb", "delay", "chorus"].includes(type) ? 0.2 : 1, params: { ...exports.EFFECTS[type].defaults } };
}
function makeTrack(presetId, index = 0) {
    const p = exports.PRESETS.find(p => p.id === presetId);
    if (!p)
        throw new Error(`Unknown preset: ${presetId}`);
    return { id: (0, model_1.uid)("track"), name: p.name, color: model_1.COLORS[index % model_1.COLORS.length], preset: p.id, synth: (0, model_1.copy)(p.synth), notes: [], effects: [], gain: 0.65, pan: 0, mute: false, solo: false, automation: [] };
}
