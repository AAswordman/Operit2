/* METADATA
{
    "api_version": "2.0.0",
  "name": "music_project",
  "display_name": {
    "zh": "工程管理",
    "en": "music_project"
  },
  "description": {
    "zh": "Operit音乐工作台：工程管理。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "get",
      "description": "获取当前完整工程、轨道/音符/效果器 ID、revision、工程列表、播放状态和执行回执。所有修改前先调用。",
      "parameters": []
    },
    {
      "name": "catalog",
      "description": "列出全部乐器预置、效果器参数范围、工程模板与资源限制。",
      "parameters": []
    },
    {
      "name": "create",
      "description": "创建并打开空白工程或内置模板：the-world-revolving / field-of-hopes-and-dreams（用户 MIDI 示范）。",
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
          "name": "name",
          "type": "string",
          "required": false,
          "description": "工程名"
        },
        {
          "name": "template",
          "type": "string",
          "required": false,
          "description": "模板 ID，可不传"
        }
      ]
    },
    {
      "name": "open",
      "description": "切换工程；停止当前演奏，未保存改动不会静默覆盖。",
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
          "name": "id",
          "type": "string",
          "required": true,
          "description": "目标工程 ID"
        }
      ]
    },
    {
      "name": "duplicate",
      "description": "复制当前工程并切换至副本。",
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
        }
      ]
    },
    {
      "name": "import_project",
      "description": "导入 operit-music v1 JSON，生成新工程 ID，完整校验，拒绝未知格式。",
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
          "name": "json",
          "type": "string",
          "required": true,
          "description": "工程 JSON 字符串"
        }
      ]
    },
    {
      "name": "export_project",
      "description": "返回可保存的完整 .operitmusic.json，含文件名与 JSON。不需要打开 UI。",
      "parameters": []
    },
    {
      "name": "undo",
      "description": "撤销最近修改（当前运行时最多20步），保存为新 revision。",
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
        }
      ]
    },
    {
      "name": "redo",
      "description": "重做上次撤销。",
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
        }
      ]
    },
    {
      "name": "delete_project",
      "description": "删除当前工程，必须 confirm=当前工程 ID；至少保留一个。",
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
          "name": "confirm",
          "type": "string",
          "required": true,
          "description": "明确确认的工程 ID"
        }
      ]
    }
  ]
}
*/
import {call,batch,json} from "./client";

/** 获取当前完整工程、轨道/音符/效果器 ID、revision、工程列表、播放状态和执行回执。所有修改前先调用。 */
export async function get(): Promise<unknown> { return call({action:"get"}); }

/** 列出全部乐器预置、效果器参数范围、工程模板与资源限制。 */
export async function catalog(): Promise<unknown> { return call({action:"catalog"}); }

/** 创建并打开空白工程或内置模板：the-world-revolving / field-of-hopes-and-dreams（用户 MIDI 示范）。 */
export async function create(p: { projectId: string; revision: number; name?: string; template?: string }): Promise<unknown> { return call({action:"create",...p}); }

/** 切换工程；停止当前演奏，未保存改动不会静默覆盖。 */
export async function open(p: { projectId: string; revision: number; id: string }): Promise<unknown> { return call({action:"open",...p}); }

/** 复制当前工程并切换至副本。 */
export async function duplicate(p: { projectId: string; revision: number }): Promise<unknown> { return call({action:"duplicate",...p}); }

/** 导入 operit-music v1 JSON，生成新工程 ID，完整校验，拒绝未知格式。 */
export async function import_project(p: { projectId: string; revision: number; json: string }): Promise<unknown> { return call({action:"import",...p}); }

/** 返回可保存的完整 .operitmusic.json，含文件名与 JSON。不需要打开 UI。 */
export async function export_project(): Promise<unknown> { return call({action:"export"}); }

/** 撤销最近修改（当前运行时最多20步），保存为新 revision。 */
export async function undo(p: { projectId: string; revision: number }): Promise<unknown> { return call({action:"undo",...p}); }

/** 重做上次撤销。 */
export async function redo(p: { projectId: string; revision: number }): Promise<unknown> { return call({action:"redo",...p}); }

/** 删除当前工程，必须 confirm=当前工程 ID；至少保留一个。 */
export async function delete_project(p: { projectId: string; revision: number; confirm: string }): Promise<unknown> { return call({action:"delete",...p}); }
