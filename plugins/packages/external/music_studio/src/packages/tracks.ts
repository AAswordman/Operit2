/* METADATA
{
  "name": "music_tracks",
  "display_name": {
    "zh": "轨道与混音",
    "en": "music_tracks"
  },
  "description": {
    "zh": "Operit音乐工作台：轨道与混音。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "add",
      "description": "添加纯合成轨道。先 catalog 查看 preset；返回创建的轨道 ID。",
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
          "name": "preset",
          "type": "string",
          "required": true,
          "description": "如 analog-kit / sub-bass / aurora-pad / fm-keys"
        },
        {
          "name": "name",
          "type": "string",
          "required": false,
          "description": "轨道名"
        }
      ]
    },
    {
      "name": "set",
      "description": "修改轨道 JSON：name,color(#RRGGBB),gain(0..1.5线性),pan(-1..1),mute,solo。",
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
          "description": "部分字段 JSON"
        }
      ]
    },
    {
      "name": "remove",
      "description": "移除轨道及其所有音符与效果器，可 undo。",
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
        }
      ]
    }
  ]
}
*/
import {call,batch,json} from "./client";

/** 添加纯合成轨道。先 catalog 查看 preset；返回创建的轨道 ID。 */
export async function add(p: { projectId: string; revision: number; preset: string; name?: string }): Promise<unknown> { return batch(p,[{type:"track.add",preset:p.preset,...(p.name?{name:p.name}:{})}]); }

/** 修改轨道 JSON：name,color(#RRGGBB),gain(0..1.5线性),pan(-1..1),mute,solo。 */
export async function set(p: { projectId: string; revision: number; trackId: string; patch: string }): Promise<unknown> { return batch(p,[{type:"track.set",trackId:p.trackId,patch:json(p.patch)}]); }

/** 移除轨道及其所有音符与效果器，可 undo。 */
export async function remove(p: { projectId: string; revision: number; trackId: string }): Promise<unknown> { return batch(p,[{type:"track.remove",trackId:p.trackId}]); }
