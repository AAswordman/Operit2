"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.catalog = catalog;
exports.add = add;
exports.set = set;
exports.remove = remove;
/* METADATA
{
  "name": "music_effects",
  "display_name": {
    "zh": "效果器机架",
    "en": "music_effects"
  },
  "description": {
    "zh": "Operit音乐工作台：效果器机架。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "catalog",
      "description": "获取 eq/filter/drive/chorus/delay/reverb/compressor 的默认值和参数范围。每轨最多6个串联插槽。",
      "parameters": []
    },
    {
      "name": "add",
      "description": "添加效果器。mix 0..1；params为部分参数JSON。延迟beats单位为拍；gain EQ单位dB；合成混响无需IR资源。",
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
          "name": "effectType",
          "type": "string",
          "required": true,
          "description": "效果类型"
        },
        {
          "name": "mix",
          "type": "number",
          "required": false,
          "description": "干湿比例"
        },
        {
          "name": "params",
          "type": "string",
          "required": false,
          "description": "参数JSON"
        }
      ]
    },
    {
      "name": "set",
      "description": "修改效果器：{enabled:boolean,mix:0..1,params:{...}}，不传字段保持原值。",
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
          "name": "effectId",
          "type": "string",
          "required": true,
          "description": "效果器 ID"
        },
        {
          "name": "patch",
          "type": "string",
          "required": true,
          "description": "部分字段JSON"
        }
      ]
    },
    {
      "name": "remove",
      "description": "移除指定效果器。",
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
          "name": "effectId",
          "type": "string",
          "required": true,
          "description": "效果器 ID"
        }
      ]
    }
  ]
}
*/
const client_1 = require("./client");
/** 获取 eq/filter/drive/chorus/delay/reverb/compressor 的默认值和参数范围。每轨最多6个串联插槽。 */
async function catalog() { return (0, client_1.call)({ action: "catalog" }); }
/** 添加效果器。mix 0..1；params为部分参数JSON。延迟beats单位为拍；gain EQ单位dB；合成混响无需IR资源。 */
async function add(p) { return (0, client_1.batch)(p, [{ type: "effect.add", trackId: p.trackId, effectType: p.effectType, ...(p.mix !== undefined ? { mix: p.mix } : {}), ...(p.params ? { params: (0, client_1.json)(p.params) } : {}) }]); }
/** 修改效果器：{enabled:boolean,mix:0..1,params:{...}}，不传字段保持原值。 */
async function set(p) { return (0, client_1.batch)(p, [{ type: "effect.set", trackId: p.trackId, effectId: p.effectId, patch: (0, client_1.json)(p.patch) }]); }
/** 移除指定效果器。 */
async function remove(p) { return (0, client_1.batch)(p, [{ type: "effect.remove", trackId: p.trackId, effectId: p.effectId }]); }
