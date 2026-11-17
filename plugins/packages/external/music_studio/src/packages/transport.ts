/* METADATA
{
  "name": "music_transport",
  "display_name": {
    "zh": "播放与渲染",
    "en": "music_transport"
  },
  "description": {
    "zh": "Operit音乐工作台：播放与渲染。先 music_project.get/catalog 获取当前模型与限制。修改必须带 projectId 和 revision，默认仅作用于当前工程；播放命令不在后台创建音频设备。",
    "en": "AI-first synthesis studio. Query project and catalog before editing. Writes require projectId and revision."
  },
  "tools": [
    {
      "name": "status",
      "description": "读取工作台连接、音频解锁、播放位置、声部预算、命令队列与回执。",
      "parameters": []
    },
    {
      "name": "control",
      "description": "play/pause/stop/seek。需要工作台已打开；play需用户先点击解锁音频。返回queued不代表完成，用status查回执。seek还需beat。",
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
          "name": "type",
          "type": "string",
          "required": true,
          "description": "play|pause|stop|seek"
        },
        {
          "name": "beat",
          "type": "number",
          "required": false,
          "description": "定位到第几拍，从0开始"
        }
      ]
    },
    {
      "name": "render_wav",
      "description": "请求WebView离线渲染立体声PCM16 WAV，44.1kHz，含效果器与尾音，不含节拍器。音乐长度上限240秒；需工作台打开。status回执返回保存路径或失败原因，不会伪造成功。",
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
    }
  ]
}
*/
import {call,batch,json} from "./client";

/** 读取工作台连接、音频解锁、播放位置、声部预算、命令队列与回执。 */
export async function status(): Promise<unknown> { return call({action:"get"}); }

/** play/pause/stop/seek。需要工作台已打开；play需用户先点击解锁音频。返回queued不代表完成，用status查回执。seek还需beat。 */
export async function control(p: { projectId: string; revision: number; type: string; beat?: number }): Promise<unknown> { if (!["play","pause","stop","seek"].includes(p.type)) throw new Error("Invalid transport command"); return call({action:"command",...p}); }

/** 请求WebView离线渲染立体声PCM16 WAV，44.1kHz，含效果器与尾音，不含节拍器。音乐长度上限240秒；需工作台打开。status回执返回保存路径或失败原因，不会伪造成功。 */
export async function render_wav(p: { projectId: string; revision: number }): Promise<unknown> { return call({action:"command",...p,type:"render"}); }
