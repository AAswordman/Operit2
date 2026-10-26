/* METADATA
{
  "name": "qqbot",
  "display_name": {
    "zh": "QQ Bot",
    "en": "QQ Bot"
  },
  "description": {
    "zh": "QQ Bot 原生网络监听、C2C/群消息、事件队列和 AI 自动回复。openid 由接收事件获得，不是 QQ 号。",
    "en": "Native QQ Bot Gateway, C2C/group messaging, events and AI replies. Use openids from inbound events."
  },
  "category": "Communication",
  "tools": [
    {
      "name": "usage_advice",
      "description": {
        "zh": "查看 QQ Bot 配置、接收和回复的使用方法。",
        "en": "Read QQ Bot setup and messaging instructions."
      },
      "parameters": []
    },
    {
      "name": "qqbot_configure",
      "description": {
        "zh": "保存 QQ Bot 配置。listener_enabled 控制应用启动时的原生监听。",
        "en": "Save QQ Bot credentials and listener configuration."
      },
      "parameters": [
        {
          "name": "app_id",
          "type": "string",
          "required": false,
          "description": {
            "zh": "QQ Bot AppID",
            "en": "app_id"
          }
        },
        {
          "name": "app_secret",
          "type": "string",
          "required": false,
          "description": {
            "zh": "QQ Bot AppSecret",
            "en": "app_secret"
          }
        },
        {
          "name": "use_sandbox",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否使用 QQ 沙箱环境",
            "en": "use_sandbox"
          }
        },
        {
          "name": "listener_enabled",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否启用应用生命周期监听",
            "en": "listener_enabled"
          }
        },
        {
          "name": "test_connection",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "保存后验证远程连接",
            "en": "test_connection"
          }
        },
        {
          "name": "restart_service",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "保存后重启监听",
            "en": "restart_service"
          }
        }
      ]
    },
    {
      "name": "qqbot_status",
      "description": {
        "zh": "读取连接、事件队列和自动回复状态，不返回密钥。",
        "en": "Read connection, event queue and AI reply status without secrets."
      },
      "parameters": []
    },
    {
      "name": "qqbot_dashboard_status",
      "description": {
        "zh": "读取 QQ Bot 仪表盘状态。",
        "en": "Read QQ Bot dashboard status."
      },
      "parameters": []
    },
    {
      "name": "qqbot_test_connection",
      "description": {
        "zh": "验证鉴权并获取机器人资料和 Gateway。",
        "en": "Verify credentials and retrieve the bot profile and Gateway."
      },
      "parameters": []
    },
    {
      "name": "qqbot_service_start",
      "description": {
        "zh": "启动原生 Gateway 并等待 READY。restart=true 可显式重启。",
        "en": "Start the native Gateway and await READY; restart explicitly when requested."
      },
      "parameters": [
        {
          "name": "restart",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "显式重启现有连接",
            "en": "restart"
          }
        }
      ]
    },
    {
      "name": "qqbot_service_stop",
      "description": {
        "zh": "关闭 Gateway、停止自动回复并禁用开机监听。",
        "en": "Close the Gateway, stop AI work and disable listener startup."
      },
      "parameters": []
    },
    {
      "name": "qqbot_receive_events",
      "description": {
        "zh": "读取有界事件队列。consume 默认 true；scene 支持 c2c/group/unknown；include_raw 返回原始数据。",
        "en": "Read queued events with consumption and scene/type filters."
      },
      "parameters": [
        {
          "name": "limit",
          "type": "number",
          "required": false,
          "description": {
            "zh": "返回事件数，1..100，默认 20",
            "en": "limit"
          }
        },
        {
          "name": "consume",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "读取后消费事件，默认 true",
            "en": "consume"
          }
        },
        {
          "name": "scene",
          "type": "string",
          "required": false,
          "description": {
            "zh": "事件场景：c2c、group 或 unknown",
            "en": "scene"
          }
        },
        {
          "name": "event_type",
          "type": "string",
          "required": false,
          "description": {
            "zh": "QQ 官方事件名称的精确匹配",
            "en": "event_type"
          }
        },
        {
          "name": "include_raw",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "返回原始 Gateway payload，默认 false",
            "en": "include_raw"
          }
        },
        {
          "name": "auto_start",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "显式请求启动监听，默认 false",
            "en": "auto_start"
          }
        }
      ]
    },
    {
      "name": "qqbot_clear_events",
      "description": {
        "zh": "清空待消费事件。",
        "en": "Clear queued inbound events."
      },
      "parameters": []
    },
    {
      "name": "qqbot_send_c2c_message",
      "description": {
        "zh": "向 user_openid 发送消息。回复时填写 msg_id；payload 为 JSON 对象文本，可指定 QQ 富媒体字段。",
        "en": "Send to a user openid; include msg_id for a reply and optional JSON payload for rich messages."
      },
      "parameters": [
        {
          "name": "openid",
          "type": "string",
          "required": true,
          "description": {
            "zh": "接收事件中的 user_openid，不是 QQ 号",
            "en": "openid"
          }
        },
        {
          "name": "content",
          "type": "string",
          "required": false,
          "description": {
            "zh": "文本消息内容",
            "en": "content"
          }
        },
        {
          "name": "msg_type",
          "type": "number",
          "required": false,
          "description": {
            "zh": "QQ 消息类型，默认 0",
            "en": "msg_type"
          }
        },
        {
          "name": "msg_id",
          "type": "string",
          "required": false,
          "description": {
            "zh": "被回复的入站消息 ID",
            "en": "msg_id"
          }
        },
        {
          "name": "event_id",
          "type": "string",
          "required": false,
          "description": {
            "zh": "关联的 QQ 事件 ID",
            "en": "event_id"
          }
        },
        {
          "name": "msg_seq",
          "type": "number",
          "required": false,
          "description": {
            "zh": "当前回复源的正整数发送序号",
            "en": "msg_seq"
          }
        },
        {
          "name": "payload",
          "type": "string",
          "required": false,
          "description": {
            "zh": "额外 QQ 消息字段组成的 JSON 对象文本",
            "en": "payload"
          }
        }
      ]
    },
    {
      "name": "qqbot_send_group_message",
      "description": {
        "zh": "向 group_openid 发送消息。回复时填写 msg_id；payload 为 JSON 对象文本。",
        "en": "Send to a group openid with an optional source message and JSON payload."
      },
      "parameters": [
        {
          "name": "group_openid",
          "type": "string",
          "required": true,
          "description": {
            "zh": "接收事件中的 group_openid",
            "en": "group_openid"
          }
        },
        {
          "name": "content",
          "type": "string",
          "required": false,
          "description": {
            "zh": "文本消息内容",
            "en": "content"
          }
        },
        {
          "name": "msg_type",
          "type": "number",
          "required": false,
          "description": {
            "zh": "QQ 消息类型，默认 0",
            "en": "msg_type"
          }
        },
        {
          "name": "msg_id",
          "type": "string",
          "required": false,
          "description": {
            "zh": "被回复的入站消息 ID",
            "en": "msg_id"
          }
        },
        {
          "name": "event_id",
          "type": "string",
          "required": false,
          "description": {
            "zh": "关联的 QQ 事件 ID",
            "en": "event_id"
          }
        },
        {
          "name": "msg_seq",
          "type": "number",
          "required": false,
          "description": {
            "zh": "当前回复源的正整数发送序号",
            "en": "msg_seq"
          }
        },
        {
          "name": "payload",
          "type": "string",
          "required": false,
          "description": {
            "zh": "额外 QQ 消息字段组成的 JSON 对象文本",
            "en": "payload"
          }
        }
      ]
    },
    {
      "name": "qqbot_auto_reply_configure",
      "description": {
        "zh": "配置 C2C/群 AI 回复、超时、会话分组、角色选择标识和额外指令。",
        "en": "Configure AI replies, deadlines, chat grouping, character selection and instructions."
      },
      "parameters": [
        {
          "name": "enabled",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否启用 AI 自动回复",
            "en": "enabled"
          }
        },
        {
          "name": "c2c_enabled",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否处理私聊消息",
            "en": "c2c_enabled"
          }
        },
        {
          "name": "group_enabled",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否处理群消息",
            "en": "group_enabled"
          }
        },
        {
          "name": "ai_timeout_ms",
          "type": "number",
          "required": false,
          "description": {
            "zh": "AI 请求超时毫秒数，1000..600000",
            "en": "ai_timeout_ms"
          }
        },
        {
          "name": "chat_group",
          "type": "string",
          "required": false,
          "description": {
            "zh": "自动回复会话所在的分组名称",
            "en": "chat_group"
          }
        },
        {
          "name": "character_selection",
          "type": "string",
          "required": false,
          "description": {
            "zh": "角色卡插件的选择标识，新建会话时生效",
            "en": "character_selection"
          }
        },
        {
          "name": "assistant_instruction",
          "type": "string",
          "required": false,
          "description": {
            "zh": "随入站消息提交的 QQ 回复指令",
            "en": "assistant_instruction"
          }
        },
        {
          "name": "start_now",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "保存后启动自动回复，默认 true",
            "en": "start_now"
          }
        },
        {
          "name": "waifu_profile",
          "type": "string",
          "required": false,
          "description": {
            "zh": "Waifu 插件配置名称",
            "en": "Named waifu dependency profile"
          }
        }
      ]
    },
    {
      "name": "qqbot_auto_reply_status",
      "description": {
        "zh": "读取自动回复和会话绑定。",
        "en": "Read AI reply status and conversation bindings."
      },
      "parameters": []
    },
    {
      "name": "qqbot_auto_reply_start",
      "description": {
        "zh": "启用自动回复并启动已配置的监听。",
        "en": "Enable automatic replies on the configured listener."
      },
      "parameters": []
    },
    {
      "name": "qqbot_auto_reply_stop",
      "description": {
        "zh": "停止自动回复并取消正在执行的 AI 请求。",
        "en": "Disable AI replies and cancel active generation."
      },
      "parameters": []
    },
    {
      "name": "qqbot_auto_reply_run_once",
      "description": {
        "zh": "处理一条符合自动回复场景配置的待处理消息。",
        "en": "Process one eligible queued message."
      },
      "parameters": []
    },
    {
      "name": "qqbot_waifu_preview",
      "description": {
        "zh": "在指定会话中预览依赖插件的真实 AI 流式分句及图片，返回各段出现时间，不向 QQ 发消息。",
        "en": "Preview real waifu dependency segments and arrival times in an explicit chat."
      },
      "parameters": [
        {
          "name": "chat_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "会话 ID",
            "en": "Conversation ID"
          }
        },
        {
          "name": "message",
          "type": "string",
          "required": true,
          "description": {
            "zh": "预览输入",
            "en": "Input text"
          }
        },
        {
          "name": "profile",
          "type": "string",
          "required": false,
          "description": {
            "zh": "Waifu 配置名称，默认 default",
            "en": "Waifu profile name, default: default"
          }
        }
      ]
    },
    {
      "name": "qqbot_upload_image",
      "description": {
        "zh": "通过 QQ 原生接口上传图片并返回 file_info，不发送消息。",
        "en": "Upload an image through QQ and return file_info without sending a message."
      },
      "parameters": [
        {
          "name": "scene",
          "type": "string",
          "required": true,
          "description": {
            "zh": "c2c 或 group",
            "en": "scene"
          }
        },
        {
          "name": "target",
          "type": "string",
          "required": true,
          "description": {
            "zh": "目标 user_openid 或 group_openid",
            "en": "target"
          }
        },
        {
          "name": "source",
          "type": "string",
          "required": true,
          "description": {
            "zh": "图片 HTTP(S) 地址或 Host 文件路径",
            "en": "source"
          }
        }
      ]
    }
  ],
  "enabled_by_default": true
}
*/
import { stream as waifuStream, type Segment } from "./dependencies/waifu/api.js";
import { call } from "./rpc.js";
import { object, type AutoReplyParams, type ConfigureParams, type ReceiveParams, type Response, type SendParams } from "./model.js";

type ToolSendParams = Omit<SendParams, "payload"> & { payload?: string };

/** Describes the v2 setup and event-based reply workflow. */
export function usage_advice(): { advice: string } {
  return { advice: "使用 qqbot_configure 保存 app_id、app_secret，再使用 qqbot_service_start。收到事件后从 replyHint 取得 openid/group_openid 和 msg_id，调用对应发送工具。开启自动回复前配置 AI 模型和执行角色。QQ 号不能代替 openid。应用需保持运行，连接错误可在状态页查看。" };
}
/** Saves application credentials and native listener controls. */
export function qqbot_configure(params: ConfigureParams = {}): Promise<Response> { return call({ action: "configure", params }); }
/** Reads the authoritative main-runtime status. */
export function qqbot_status(): Promise<Response> { return call({ action: "status" }); }
/** Reads the dashboard projection from the same owner. */
export function qqbot_dashboard_status(): Promise<Response> { return call({ action: "status" }); }
/** Verifies the configured QQ credentials against the remote API. */
export function qqbot_test_connection(): Promise<Response> { return call({ action: "test" }); }
/** Starts and optionally restarts the native Gateway. */
export function qqbot_service_start(params: { restart?: boolean } = {}): Promise<Response> { return call({ action: "start", ...params }); }
/** Stops the native listener and its AI consumer. */
export function qqbot_service_stop(): Promise<Response> { return call({ action: "stop" }); }
/** Reads only the requested events and consumption policy. */
export function qqbot_receive_events(params: ReceiveParams = {}): Promise<Response> { return call({ action: "receive", params }); }
/** Clears pending inbound events in the main runtime. */
export function qqbot_clear_events(): Promise<Response> { return call({ action: "clear" }); }
/** Parses the explicitly declared JSON payload tool argument. */
function sendParams(params: ToolSendParams): SendParams {
  const { payload, ...fields } = params;
  return payload === undefined ? fields : { ...fields, payload: object(JSON.parse(payload), "payload") };
}
/** Sends one C2C message through the configured QQ application. */
export function qqbot_send_c2c_message(params: ToolSendParams & { openid: string }): Promise<Response> { return call({ action: "send_c2c", params: { ...sendParams(params), openid: params.openid } }); }
/** Sends one group message through the configured QQ application. */
export function qqbot_send_group_message(params: ToolSendParams & { group_openid: string }): Promise<Response> { return call({ action: "send_group", params: { ...sendParams(params), group_openid: params.group_openid } }); }
/** Persists explicit automatic reply settings. */
export function qqbot_auto_reply_configure(params: AutoReplyParams = {}): Promise<Response> { return call({ action: "auto_configure", params }); }
/** Reads automatic reply state and durable conversation identities. */
export function qqbot_auto_reply_status(): Promise<Response> { return call({ action: "status" }); }
/** Enables the sequential automatic reply consumer. */
export function qqbot_auto_reply_start(): Promise<Response> { return call({ action: "auto_start" }); }
/** Stops and cancels the automatic reply consumer. */
export function qqbot_auto_reply_stop(): Promise<Response> { return call({ action: "auto_stop" }); }
/** Processes exactly one eligible message without starting an additional loop. */
export function qqbot_auto_reply_run_once(): Promise<Response> { return call({ action: "auto_once" }); }

/** Previews actual dependency streaming and records delivery timing in an explicitly selected chat. */
export async function qqbot_waifu_preview(params: { chat_id: string; message: string; profile?: string }): Promise<{ segments: { elapsedMs: number; segment: Segment }[]; completedAfterMs: number }> {
  const { profile = "default" } = params, started = Date.now();
  const segments: { elapsedMs: number; segment: Segment }[] = [];
  for await (const event of waifuStream({ kind: "submit", chatId: params.chat_id, runtime: "main", input: { text: params.message, attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute" }, notifyReply: false }, profile)) {
    if (event.type === "segment") segments.push({ elapsedMs: Date.now() - started, segment: event.segment });
  }
  return { segments, completedAfterMs: Date.now() - started };
}

/** Uploads media through the actual QQ owner runtime without submitting a message. */
export function qqbot_upload_image(params: { scene: "c2c" | "group"; target: string; source: string }): Promise<Response> {
  return call({ action: "upload_image", ...params });
}
