/* METADATA
{
  "name": "waifu",
  "display_name": {
    "zh": "Waifu 模式",
    "en": "Waifu Mode"
  },
  "description": {
    "zh": "管理普通聊天的角色绑定模式及外部插件分句配置。",
    "en": "Manage waifu sentence and sticker profiles."
  },
  "category": "Chat",
  "tools": [
    {
      "name": "waifu_settings_get",
      "description": {
        "zh": "读取 Waifu 命名配置。",
        "en": "Read a named waifu profile."
      },
      "parameters": [
        {
          "name": "profile",
          "type": "string",
          "required": false,
          "description": {
            "zh": "配置名称，默认 default",
            "en": "Profile name, default: default"
          }
        }
      ]
    },
    {
      "name": "waifu_settings_set",
      "description": {
        "zh": "保存完整 Waifu 配置。settings_json 为包含打字延迟、表情、自拍及提示词的完整 JSON。",
        "en": "Save a complete waifu profile."
      },
      "parameters": [
        {
          "name": "profile",
          "type": "string",
          "required": true,
          "description": {
            "zh": "配置名称",
            "en": "Profile name"
          }
        },
        {
          "name": "settings_json",
          "type": "string",
          "required": true,
          "description": {
            "zh": "完整 Settings JSON",
            "en": "Complete Settings JSON"
          }
        }
      ]
    },
    {
      "name": "waifu_mode_get",
      "description": {
        "zh": "读取当前角色卡或角色组的普通聊天 Waifu 设置。",
        "en": "Read the active role-bound native chat mode."
      },
      "parameters": []
    },
    {
      "name": "waifu_mode_set",
      "description": {
        "zh": "保存指定角色绑定的完整普通聊天 Waifu 设置。",
        "en": "Save a complete role-bound native chat mode."
      },
      "parameters": [
        {
          "name": "selection",
          "type": "string",
          "required": true,
          "description": {
            "zh": "角色插件返回的绑定标识",
            "en": "Selection returned by the character plugin"
          }
        },
        {
          "name": "mode_json",
          "type": "string",
          "required": true,
          "description": {
            "zh": "完整 Mode JSON，包含 enabled、mergeSend、mergeDelayMs 和 settings",
            "en": "Complete Mode JSON"
          }
        }
      ]
    }
  ],
  "enabled_by_default": true
}
*/
import { callAdmin } from "./admin.js";
import type { Mode, ModeContext } from "./mode.js";
import type { Settings } from "./api.js";

/** Reads a named profile from the provider's authoritative runtime. */
export function waifu_settings_get(params: { profile?: string } = {}): Promise<Settings> {
  const { profile = "default" } = params;
  return callAdmin({ action: "get", profile });
}

/** Parses and saves one explicitly supplied complete profile. */
export function waifu_settings_set(params: { profile: string; settings_json: string }): Promise<Settings> {
  return callAdmin({ action: "set", profile: params.profile, settings: JSON.parse(params.settings_json) });
}

/** Reads the active character's normal-chat mode through the same channel as the settings page. */
export function waifu_mode_get(): Promise<ModeContext> {
  return callAdmin({ action: "mode.get" });
}

/** Saves a complete ordinary-chat mode for an explicitly selected role binding. */
export function waifu_mode_set(params: { selection: string; mode_json: string }): Promise<Mode> {
  return callAdmin({ action: "mode.set", selection: params.selection, mode: JSON.parse(params.mode_json) });
}
