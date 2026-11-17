/* METADATA
{
  "name": "music_instruments",
  "display_name": {
    "zh": "合成器与音色",
    "en": "music_instruments"
  },
  "description": {
    "zh": "Operit音乐工作台：合成器与音色。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "catalog",
      "description": "返回原生合成器预置。spectral双振荡器/谐波波形、fm双算子、ensemble合成乐器、drums电子鼓、atmosphere立体声环境；无采样文件。",
      "parameters": []
    },
    {
      "name": "preset",
      "description": "切换乐器预置并重置合成参数，保留音符和效果器。",
      "parameters": [
        {
          "name": "projectId",
          "type": "string",
          "required": true,
          "description": "get 返回的当前工程 ID"
        },
        {
          "name": "revision",
          "type": "number",
          "required": true,
          "description": "get 返回的最新 revision，冲突时重新 get 并重新应用意图"
        },
        {
          "name": "trackId",
          "type": "string",
          "required": true,
          "description": "轨道 ID"
        },
        {
          "name": "preset",
          "type": "string",
          "required": true,
          "description": "预置ID"
        }
      ]
    },
    {
      "name": "design",
      "description": "修改 synth JSON：engine(spectral/fm/ensemble/drums/atmosphere),wave/waveB(sine/triangle/sawtooth/square/glass/hollow),blend(0..1),unison/unisonB(整数1..8),detune/detuneB(0..50音分),attack(.001..3秒),decay(.01..3),sustain(0..1),release(.02..3),cutoff(40..18000Hz),resonance(.1..12),fmRatio(.25..12),fmDepth(0..10),brightness(0..1),width/widthB(0..1),oscBOctave(整数-3..3),oscBSemitone(整数-12..12),oscBFine(-100..100音分),subLevel(0..1),subOctave(整数-2..0),noiseLevel(0..1),phase/phaseRandom(0..1),haasMs(-35..35ms，正延迟R/负延迟L/0关闭),haasMix(0..1),bassMono(0..500Hz，0关闭，效果器后低频收窄),filterEnv(-6..6八度，负值渐开),lfoRate(0..16Hz),lfoDepth(0..3八度),pitchSweep(-48..48半音)。",
      "parameters": [
        {
          "name": "projectId",
          "type": "string",
          "required": true,
          "description": "get 返回的当前工程 ID"
        },
        {
          "name": "revision",
          "type": "number",
          "required": true,
          "description": "get 返回的最新 revision，冲突时重新 get 并重新应用意图"
        },
        {
          "name": "trackId",
          "type": "string",
          "required": true,
          "description": "轨道 ID"
        },
        {
          "name": "patch",
          "type": "string",
          "required": true,
          "description": "合成器部分参数JSON"
        }
      ]
    }
  ]
}
*/
import {call,batch,json} from "./client";

/** 返回原生合成器预置。spectral双振荡器/谐波波形、fm双算子、ensemble合成乐器、drums电子鼓、atmosphere立体声环境；无采样文件。 */
export async function catalog(): Promise<unknown> { return call({action:"catalog"}); }

/** 切换乐器预置并重置合成参数，保留音符和效果器。 */
export async function preset(p: { projectId: string; revision: number; trackId: string; preset: string }): Promise<unknown> { return batch(p,[{type:"track.preset",trackId:p.trackId,preset:p.preset}]); }

/** 修改 synth JSON：engine(spectral/fm/ensemble/drums/atmosphere),wave/waveB(sine/triangle/sawtooth/square/glass/hollow),blend(0..1),unison/unisonB(整数1..8),detune/detuneB(0..50音分),attack(.001..3秒),decay(.01..3),sustain(0..1),release(.02..3),cutoff(40..18000Hz),resonance(.1..12),fmRatio(.25..12),fmDepth(0..10),brightness(0..1),width/widthB(0..1),oscBOctave(整数-3..3),oscBSemitone(整数-12..12),oscBFine(-100..100音分),subLevel(0..1),subOctave(整数-2..0),noiseLevel(0..1),phase/phaseRandom(0..1),haasMs(-35..35ms，正延迟R/负延迟L/0关闭),haasMix(0..1),bassMono(0..500Hz，0关闭，效果器后低频收窄),filterEnv(-6..6八度，负值渐开),lfoRate(0..16Hz),lfoDepth(0..3八度),pitchSweep(-48..48半音)。 */
export async function design(p: { projectId: string; revision: number; trackId: string; patch: string }): Promise<unknown> { return batch(p,[{type:"synth.set",trackId:p.trackId,patch:json(p.patch)}]); }
