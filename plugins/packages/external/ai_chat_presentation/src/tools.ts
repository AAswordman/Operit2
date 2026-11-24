/* METADATA
{
  "api_version": "2.0.0",
  "name": "ai_chat_presentation",
  "display_name": {
    "zh": "AI 分段对话",
    "en": "AI Segmented Chat"
  },
  "description": {
    "zh": "在已配置的展示对话中发布 AI 选择的分段消息，完整执行记录保留在原始子对话。",
    "en": "Publish AI-selected message segments while retaining the original conversation in a child chat."
  },
  "category": "Chat",
  "enabled_by_default": true,
  "tools": [
    {
      "name": "show_messages",
      "description": {
        "zh": "向用户展示自主选择的消息分段。仅在当前分段对话会话内可用。消息按数组顺序逐段展示；重试使用相同 message_id 和相同内容。",
        "en": "Display selected message segments in order in the current autonomous reply session. Retry with the same message_id and content."
      },
      "parameters": [
        {
          "name": "session_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "系统提示词提供的当前展示会话 ID。",
            "en": "Current display session ID from the system prompt."
          }
        },
        {
          "name": "message_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "本次展示调用的唯一标识。",
            "en": "Unique ID for this display batch."
          }
        },
        {
          "name": "segments_json",
          "type": "string",
          "required": true,
          "description": {
            "zh": "JSON 数组，每项包含 text（Markdown 正文）和可选 delayMs（展示前等待 0～3000 毫秒），每次 1～32 段。",
            "en": "JSON array of 1–32 items with text (Markdown) and optional delayMs (0–3000 ms before display)."
          }
        }
      ]
    }
  ]
}
*/
import { validateSegments, type PublishRequest } from "./model";

export function show_messages(params: { session_id: string; message_id: string; segments_json: string }): Promise<{ displayed: number }> {
  return ToolPkg.ipc.call<PublishRequest, { displayed: number }>("ai_chat_presentation.publish", {
    sessionId: params.session_id, messageId: params.message_id, segments: validateSegments(JSON.parse(params.segments_json)),
  }, { targetRuntime: "main" });
}
