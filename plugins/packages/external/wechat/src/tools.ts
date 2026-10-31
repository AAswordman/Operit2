/* METADATA
{
  "name": "wechat",
  "display_name": {
    "zh": "微信 Bot",
    "en": "WeChat Bot"
  },
  "description": {
    "zh": "微信扫码登录、原生网络监听、文本与媒体消息及角色卡 / Waifu 自动回复。",
    "en": "WeChat QR login, native polling, encrypted media and character/Waifu replies."
  },
  "category": "Communication",
  "tools": [
    {
      "name": "usage_advice",
      "description": {
        "zh": "查看扫码登录、收发消息和自动回复的使用说明。",
        "en": "Describes QR login, messaging and automatic replies."
      },
      "parameters": []
    },
    {
      "name": "wechat_status",
      "description": {
        "zh": "读取登录、连接、队列和回复状态。",
        "en": "Reads login, listener, queue and delivery state."
      },
      "parameters": []
    },
    {
      "name": "wechat_login",
      "description": {
        "zh": "获取微信登录二维码内容并开始轮询扫码状态。会停止当前机器人。",
        "en": "Starts QR login and stops the current listener."
      },
      "parameters": []
    },
    {
      "name": "wechat_login_cancel",
      "description": {
        "zh": "取消本次微信扫码登录。",
        "en": "Cancels the active QR login."
      },
      "parameters": []
    },
    {
      "name": "wechat_logout",
      "description": {
        "zh": "停止机器人并删除本设备微信登录凭据及待处理消息，保留原生聊天记录。",
        "en": "Stops the bot and removes device credentials and pending messages."
      },
      "parameters": []
    },
    {
      "name": "wechat_service_stop",
      "description": {
        "zh": "停止微信监听和 AI 回复，并取消下次自动启动。",
        "en": "Stops WeChat polling and cancels automatic startup."
      },
      "parameters": []
    },
    {
      "name": "wechat_test_connection",
      "description": {
        "zh": "验证当前微信凭据。",
        "en": "Verifies the saved WeChat credentials."
      },
      "parameters": []
    },
    {
      "name": "wechat_clear_events",
      "description": {
        "zh": "清空待处理微信消息，不删除原生聊天记录。",
        "en": "Clears pending WeChat events without deleting native chats."
      },
      "parameters": []
    },
    {
      "name": "wechat_auto_reply_run_once",
      "description": {
        "zh": "在监听就绪且持续自动回复关闭时，处理一条待处理消息。",
        "en": "Processes one queued event while continuous automatic replies are off."
      },
      "parameters": []
    },
    {
      "name": "wechat_login_verify",
      "description": {
        "zh": "提交微信手机端显示的登录验证码。",
        "en": "Submits the verification code for the current QR session."
      },
      "parameters": [
        {
          "name": "code",
          "type": "string",
          "required": true,
          "description": {
            "zh": "微信手机端显示的验证码。",
            "en": "code"
          }
        }
      ]
    },
    {
      "name": "wechat_service_start",
      "description": {
        "zh": "启动微信消息监听及已启用的自动回复，下次打开应用自动恢复。",
        "en": "Starts WeChat polling and enabled automatic replies."
      },
      "parameters": [
        {
          "name": "restart",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "显式停止现有监听再启动。",
            "en": "restart"
          }
        }
      ]
    },
    {
      "name": "wechat_receive_events",
      "description": {
        "zh": "查看或消费微信消息；默认消费。include_raw 可取得完整媒体字段。",
        "en": "Reads or consumes queued WeChat messages."
      },
      "parameters": [
        {
          "name": "limit",
          "type": "number",
          "required": false,
          "description": {
            "zh": "返回数量，1～100，默认 20。",
            "en": "limit"
          }
        },
        {
          "name": "consume",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否消费返回消息，默认 true；预览请传 false。",
            "en": "consume"
          }
        },
        {
          "name": "user_id",
          "type": "string",
          "required": false,
          "description": {
            "zh": "只读取指定微信用户的消息。",
            "en": "user_id"
          }
        },
        {
          "name": "include_raw",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否包含完整消息和媒体引用。",
            "en": "include_raw"
          }
        }
      ]
    },
    {
      "name": "wechat_send_message",
      "description": {
        "zh": "发送微信文本，最多 4000 个字符；需要该用户的回复上下文。",
        "en": "Sends one WeChat text message with its conversation context."
      },
      "parameters": [
        {
          "name": "user_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "从接收消息得到的微信用户 ID，不是微信号。",
            "en": "user_id"
          }
        },
        {
          "name": "text",
          "type": "string",
          "required": true,
          "description": {
            "zh": "消息文本，最多 4000 个字符。",
            "en": "text"
          }
        },
        {
          "name": "context_token",
          "type": "string",
          "required": false,
          "description": {
            "zh": "入站消息的回复上下文令牌；省略时使用该用户最近收到的令牌。",
            "en": "context_token"
          }
        }
      ]
    },
    {
      "name": "wechat_send_media",
      "description": {
        "zh": "加密上传并发送图片、视频、文件或语音，最大 25 MiB。",
        "en": "Encrypts, uploads and sends an image, video, file or voice message."
      },
      "parameters": [
        {
          "name": "user_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "从接收消息得到的微信用户 ID，不是微信号。",
            "en": "user_id"
          }
        },
        {
          "name": "source",
          "type": "string",
          "required": true,
          "description": {
            "zh": "宿主文件路径或 HTTPS 媒体地址。",
            "en": "source"
          }
        },
        {
          "name": "kind",
          "type": "string",
          "required": true,
          "description": {
            "zh": "image / video / file / voice。",
            "en": "kind"
          }
        },
        {
          "name": "file_name",
          "type": "string",
          "required": false,
          "description": {
            "zh": "文件消息的显示名称。",
            "en": "file_name"
          }
        },
        {
          "name": "text",
          "type": "string",
          "required": false,
          "description": {
            "zh": "可选说明文本，独立消息发送，最多 4000 个字符。",
            "en": "text"
          }
        },
        {
          "name": "context_token",
          "type": "string",
          "required": false,
          "description": {
            "zh": "入站消息的回复上下文令牌；省略时使用该用户最近收到的令牌。",
            "en": "context_token"
          }
        },
        {
          "name": "voice_format",
          "type": "number",
          "required": false,
          "description": {
            "zh": "语音必填：1 PCM、2 ADPCM、3 feature、4 Speex、5 AMR、6 SILK、7 MP3、8 Ogg-Speex。",
            "en": "voice_format"
          }
        },
        {
          "name": "sample_rate",
          "type": "number",
          "required": false,
          "description": {
            "zh": "语音必填：采样率，8000～192000 Hz。",
            "en": "sample_rate"
          }
        },
        {
          "name": "duration_ms",
          "type": "number",
          "required": false,
          "description": {
            "zh": "语音必填：时长，1～600000 毫秒。",
            "en": "duration_ms"
          }
        }
      ]
    },
    {
      "name": "wechat_download_media",
      "description": {
        "zh": "下载并解密指定消息的全部媒体，返回宿主文件路径和附件元数据。",
        "en": "Downloads and decrypts all media items in a received message."
      },
      "parameters": [
        {
          "name": "message_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "接收事件中的 messageId。",
            "en": "message_id"
          }
        }
      ]
    },
    {
      "name": "wechat_set_typing",
      "description": {
        "zh": "设置或取消指定用户的正在输入提示。",
        "en": "Sets or clears the typing indicator for an existing conversation."
      },
      "parameters": [
        {
          "name": "user_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "从接收消息得到的微信用户 ID，不是微信号。",
            "en": "user_id"
          }
        },
        {
          "name": "active",
          "type": "boolean",
          "required": true,
          "description": {
            "zh": "true 表示正在输入，false 表示取消。",
            "en": "active"
          }
        }
      ]
    },
    {
      "name": "wechat_auto_reply_configure",
      "description": {
        "zh": "保存自动回复、角色卡、Waifu 配置和回复等待时间，不会启动已停止的监听。",
        "en": "Persists reply settings without starting a stopped listener."
      },
      "parameters": [
        {
          "name": "enabled",
          "type": "boolean",
          "required": false,
          "description": {
            "zh": "是否自动回复微信消息。",
            "en": "enabled"
          }
        },
        {
          "name": "character_selection",
          "type": "string",
          "required": false,
          "description": {
            "zh": "空字符串跟随当前角色；model:chat / card:<id> / group:<id>。",
            "en": "character_selection"
          }
        },
        {
          "name": "waifu_profile",
          "type": "string",
          "required": false,
          "description": {
            "zh": "Waifu 命名配置，默认 default。",
            "en": "waifu_profile"
          }
        },
        {
          "name": "chat_group",
          "type": "string",
          "required": false,
          "description": {
            "zh": "原生聊天记录分组。",
            "en": "chat_group"
          }
        },
        {
          "name": "assistant_instruction",
          "type": "string",
          "required": false,
          "description": {
            "zh": "额外回复要求。",
            "en": "assistant_instruction"
          }
        },
        {
          "name": "ai_timeout_ms",
          "type": "number",
          "required": false,
          "description": {
            "zh": "AI 回复超时，1000～600000 毫秒。",
            "en": "ai_timeout_ms"
          }
        }
      ]
    },
    {
      "name": "wechat_auto_reply_start",
      "description": {
        "zh": "启用自动回复；监听需已启动。",
        "en": "Updates the automatic reply switch without starting a stopped listener."
      },
      "parameters": []
    },
    {
      "name": "wechat_auto_reply_stop",
      "description": {
        "zh": "关闭自动回复但继续监听消息。",
        "en": "Updates the automatic reply switch without starting a stopped listener."
      },
      "parameters": []
    },
    {
      "name": "wechat_delivery_resolve",
      "description": {
        "zh": "显式重试或跳过失败回复；retry 重试整个请求，可能重复已交付内容。",
        "en": "Explicitly retries or discards a failed delivery; retry can repeat delivered segments."
      },
      "parameters": [
        {
          "name": "resolution",
          "type": "string",
          "required": true,
          "description": {
            "zh": "retry 或 discard。",
            "en": "resolution"
          }
        }
      ]
    },
    {
      "name": "wechat_reset_conversation",
      "description": {
        "zh": "解除指定微信用户的会话绑定，下次消息创建使用当前角色的新会话；原聊天记录保留。",
        "en": "Starts a fresh conversation on the next message without deleting the old chat."
      },
      "parameters": [
        {
          "name": "user_id",
          "type": "string",
          "required": true,
          "description": {
            "zh": "从接收消息得到的微信用户 ID，不是微信号。",
            "en": "user_id"
          }
        }
      ]
    }
  ]
}
*/

import { call } from "./rpc.js";
import type {
  AutoReplyParams,
  MediaParams,
  ReceiveParams,
  Response,
  SendParams,
} from "./model.js";

/** Describes QR login, messaging and automatic replies. */
export function usage_advice(): { advice: string } {
  return {
    advice:
      "先在微信 Bot 设置页扫码登录，选择角色和 Waifu 配置后启动。也可调用 wechat_login 获取二维码内容，再用 wechat_status 查看扫码状态。收到 need_verifycode 时调用 wechat_login_verify。wechat_receive_events 返回 userId、messageId、contextToken；使用 wechat_send_message 或 wechat_send_media 回复。每位用户有独立聊天，凭据只保存在本设备。应用需保持运行。失败或部分交付不会自动重发，请核对微信后使用 wechat_delivery_resolve 显式重试或跳过。",
  };
}

/** Reads login, listener, queue and delivery state. */
export function wechat_status(): Promise<Response> {
  return call({ action: "status" });
}

/** Starts QR login and stops the current listener. */
export function wechat_login(): Promise<Response> {
  return call({ action: "login_start" });
}

/** Cancels the active QR login. */
export function wechat_login_cancel(): Promise<Response> {
  return call({ action: "login_cancel" });
}

/** Stops the bot and removes device credentials and pending messages. */
export function wechat_logout(): Promise<Response> {
  return call({ action: "logout" });
}

/** Stops WeChat polling and cancels automatic startup. */
export function wechat_service_stop(): Promise<Response> {
  return call({ action: "stop" });
}

/** Verifies the saved WeChat credentials. */
export function wechat_test_connection(): Promise<Response> {
  return call({ action: "test" });
}

/** Clears pending WeChat events without deleting native chats. */
export function wechat_clear_events(): Promise<Response> {
  return call({ action: "clear" });
}

/** Processes one queued event while continuous automatic replies are off. */
export function wechat_auto_reply_run_once(): Promise<Response> {
  return call({ action: "auto_once" });
}

/** Submits the verification code for the current QR session. */
export function wechat_login_verify(params: {
  code: string;
}): Promise<Response> {
  return call({ action: "login_verify", code: params.code });
}

/** Starts WeChat polling and enabled automatic replies. */
export function wechat_service_start(
  params: { restart?: boolean } = {},
): Promise<Response> {
  return call({ action: "start", restart: params.restart });
}

/** Reads or consumes queued WeChat messages. */
export function wechat_receive_events(
  params: ReceiveParams = {},
): Promise<Response> {
  return call({ action: "receive", params });
}

/** Sends one WeChat text message with its conversation context. */
export function wechat_send_message(
  params: SendParams & { text: string },
): Promise<Response> {
  return call({ action: "send_text", params });
}

/** Encrypts, uploads and sends an image, video, file or voice message. */
export function wechat_send_media(params: MediaParams): Promise<Response> {
  return call({ action: "send_media", params });
}

/** Downloads and decrypts all media items in a received message. */
export function wechat_download_media(params: {
  message_id: string;
}): Promise<Response> {
  return call({ action: "download_media", message_id: params.message_id });
}

/** Sets or clears the typing indicator for an existing conversation. */
export function wechat_set_typing(params: {
  user_id: string;
  active: boolean;
}): Promise<Response> {
  return call({ action: "typing", ...params });
}

/** Persists reply settings without starting a stopped listener. */
export function wechat_auto_reply_configure(
  params: AutoReplyParams = {},
): Promise<Response> {
  return call({ action: "auto_configure", params });
}

/** Updates the automatic reply switch without starting a stopped listener. */
export function wechat_auto_reply_start(): Promise<Response> {
  return call({ action: "auto_configure", params: { enabled: true } });
}

/** Updates the automatic reply switch without starting a stopped listener. */
export function wechat_auto_reply_stop(): Promise<Response> {
  return call({ action: "auto_configure", params: { enabled: false } });
}

/** Explicitly retries or discards a failed delivery; retry can repeat delivered segments. */
export function wechat_delivery_resolve(params: {
  resolution: "retry" | "discard";
}): Promise<Response> {
  return call({ action: "delivery_resolve", resolution: params.resolution });
}

/** Starts a fresh conversation on the next message without deleting the old chat. */
export function wechat_reset_conversation(params: {
  user_id: string;
}): Promise<Response> {
  return call({ action: "reset_conversation", user_id: params.user_id });
}
