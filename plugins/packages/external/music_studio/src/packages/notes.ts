/* METADATA
{
  "name": "music_notes",
  "display_name": {
    "zh": "音符与钢琴卷帘",
    "en": "music_notes"
  },
  "description": {
    "zh": "Operit音乐工作台：音符与钢琴卷帘。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "set",
      "description": "替换轨道所有音符。notes JSON数组，每项 {pitch:0..127,start:拍,duration:拍,velocity:0.01..1,id?:字符串}。拍为四分音符，从0开始；须在工程长度内。鼓轨遵循36底鼓38军鼓42闭镲46开镲等GM映射。",
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
          "name": "notes",
          "type": "string",
          "required": true,
          "description": "音符 JSON 数组"
        }
      ]
    },
    {
      "name": "add",
      "description": "追加音符，不替换已有音符；格式同 set。",
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
          "name": "notes",
          "type": "string",
          "required": true,
          "description": "音符 JSON 数组"
        }
      ]
    },
    {
      "name": "remove",
      "description": "按音符 ID 删除，ids 为 JSON字符串数组。",
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
          "name": "ids",
          "type": "string",
          "required": true,
          "description": "音符ID JSON数组"
        }
      ]
    },
    {
      "name": "transform",
      "description": "批量变换音符：{transpose:半音,shift:拍,quantize:拍网格,velocity:0..1,from:起拍,to:末拍}；范围[from,to)，字段可选，移出边界则整批失败。",
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
          "description": "变换 JSON"
        }
      ]
    }
  ]
}
*/
import {call,batch,json} from "./client";

/** 替换轨道所有音符。notes JSON数组，每项 {pitch:0..127,start:拍,duration:拍,velocity:0.01..1,id?:字符串}。拍为四分音符，从0开始；须在工程长度内。鼓轨遵循36底鼓38军鼓42闭镲46开镲等GM映射。 */
export async function set(p: { projectId: string; revision: number; trackId: string; notes: string }): Promise<unknown> { return batch(p,[{type:"notes.set",trackId:p.trackId,notes:json(p.notes)}]); }

/** 追加音符，不替换已有音符；格式同 set。 */
export async function add(p: { projectId: string; revision: number; trackId: string; notes: string }): Promise<unknown> { return batch(p,[{type:"notes.add",trackId:p.trackId,notes:json(p.notes)}]); }

/** 按音符 ID 删除，ids 为 JSON字符串数组。 */
export async function remove(p: { projectId: string; revision: number; trackId: string; ids: string }): Promise<unknown> { return batch(p,[{type:"notes.remove",trackId:p.trackId,ids:json(p.ids)}]); }

/** 批量变换音符：{transpose:半音,shift:拍,quantize:拍网格,velocity:0..1,from:起拍,to:末拍}；范围[from,to)，字段可选，移出边界则整批失败。 */
export async function transform(p: { projectId: string; revision: number; trackId: string; patch: string }): Promise<unknown> { return batch(p,[{type:"notes.transform",trackId:p.trackId,patch:json(p.patch)}]); }
