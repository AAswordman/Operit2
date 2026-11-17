"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.configure = configure;
exports.generate_pattern = generate_pattern;
exports.sections = sections;
exports.batch = batch;
exports.automation = automation;
/* METADATA
{
  "name": "music_arrangement",
  "display_name": {
    "zh": "编曲与生成",
    "en": "music_arrangement"
  },
  "description": {
    "zh": "Operit音乐工作台：编曲与生成。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "configure",
      "description": "设置工程全局参数 JSON：name,bpm(40..240),bars(1..256),beatsPerBar(1..7),swing(0..0.65),masterGain(0..1),loop:{enabled,start,end}。缩短工程需同一batch处理越界音符/段落。",
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
          "name": "patch",
          "type": "string",
          "required": true,
          "description": "工程设置JSON"
        }
      ]
    },
    {
      "name": "generate_pattern",
      "description": "生成可编辑音符，不是生成音频模型。options:{kind:four-floor/half-time/breakbeat/bass-pulse/arpeggio/chords/texture,start:0,bars:8,root:57,scale:minor/major/dorian/pentatonic,velocity:.7,seed:1}；默认替换，append=true追加。",
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
          "description": "轨道 ID，鼓节奏请选鼓轨"
        },
        {
          "name": "options",
          "type": "string",
          "required": true,
          "description": "生成参数JSON"
        },
        {
          "name": "append",
          "type": "boolean",
          "required": false,
          "description": "是否追加"
        }
      ]
    },
    {
      "name": "sections",
      "description": "设置展示段落标记 JSON数组 [{name,start,length,color,id?}]；单位拍。",
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
          "name": "sections",
          "type": "string",
          "required": true,
          "description": "段落JSON数组"
        }
      ]
    },
    {
      "name": "automation",
      "description": "替换轨道自动化 lanes JSON 数组。最多3条，target:level(0..1.5,轨道增益后倍率)/pan(-1..1,绝对声像)/cutoff(40..18000Hz)。points:[{beat,value}]按拍严格递增，每条1..1024点；线性插值。[]清空。支持侧链式避让、渐强、滤波推进、声像移动。",
      "parameters": [
        {
          "name": "projectId",
          "type": "string",
          "required": true,
          "description": "当前工程ID"
        },
        {
          "name": "revision",
          "type": "number",
          "required": true,
          "description": "最新revision"
        },
        {
          "name": "trackId",
          "type": "string",
          "required": true,
          "description": "轨道ID"
        },
        {
          "name": "lanes",
          "type": "string",
          "required": true,
          "description": "AutomationLane[] JSON字符串"
        }
      ]
    },
    {
      "name": "batch",
      "description": "一次原子编曲事务(最多100操作)：project.set; track.add{preset,id?,name?}; track.set/remove/preset; synth.set; notes.set/add/remove/transform; pattern.generate; effect.add/set/remove; sections.set; automation.set{trackId,lanes}。所有轨道操作需trackId；set操作需patch(但notes.set用notes)。track.add可指定ID供本批后续引用。所有值按JSON真实类型。错误整批回滚；只增加一次revision。",
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
          "name": "operations",
          "type": "string",
          "required": true,
          "description": "Operation[] JSON字符串"
        }
      ]
    }
  ]
}
*/
const client_1 = require("./client");
/** 设置工程全局参数 JSON：name,bpm(40..240),bars(1..256),beatsPerBar(1..7),swing(0..0.65),masterGain(0..1),loop:{enabled,start,end}。缩短工程需同一batch处理越界音符/段落。 */
async function configure(p) { return (0, client_1.batch)(p, [{ type: "project.set", patch: (0, client_1.json)(p.patch) }]); }
/** 生成可编辑音符，不是生成音频模型。options:{kind:four-floor/half-time/breakbeat/bass-pulse/arpeggio/chords/texture,start:0,bars:8,root:57,scale:minor/major/dorian/pentatonic,velocity:.7,seed:1}；默认替换，append=true追加。 */
async function generate_pattern(p) { return (0, client_1.batch)(p, [{ type: "pattern.generate", trackId: p.trackId, options: (0, client_1.json)(p.options), append: p.append ?? false }]); }
/** 设置展示段落标记 JSON数组 [{name,start,length,color,id?}]；单位拍。 */
async function sections(p) { return (0, client_1.batch)(p, [{ type: "sections.set", sections: (0, client_1.json)(p.sections) }]); }
/** 一次原子编曲事务(最多100操作)：project.set; track.add{preset,id?,name?}; track.set/remove/preset; synth.set; notes.set/add/remove/transform; pattern.generate; effect.add/set/remove; sections.set。所有轨道操作需trackId；set操作需patch(但notes.set用notes)。track.add可指定ID供本批后续引用。所有值按JSON真实类型。错误整批回滚；只增加一次revision。 */
async function batch(p) { return (0, client_1.batch)(p, (0, client_1.json)(p.operations)); }
/** Replace validated beat-based automation; empty lanes clears automation. */
async function automation(p) { return (0, client_1.batch)(p, [{ type: "automation.set", trackId: p.trackId, lanes: (0, client_1.json)(p.lanes) }]); }
