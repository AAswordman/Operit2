"use strict";
/* METADATA
{
  "name": "daily_life",
  "display_name": {
    "zh": "日常生活工具包",
    "en": "Daily Life Toolkit"
  },
  "description": {
    "zh": "插件内日常生活工具：日期时间、设备状态、网页天气查询、持久化通知提醒、提醒查询与取消，以及截图。提醒由已有 Host 定时事件触发，不操作系统提醒应用。",
    "en": "Plugin-owned daily utilities: date/time, device status, web-based weather lookup, persistent notification reminders, reminder listing/cancellation, and screenshots. Reminders use existing Host events, not the system reminder app."
  },
  "enabledByDefault": true,
  "category": "Life",
  "tools": [
    {
      "name": "get_current_date",
      "display": {"label": {"zh": "获取日期时间", "en": "Get date and time"}, "icon": "time"},
      "description": {
        "zh": "获取当前日期和时间。",
        "en": "Get the current date and time."
      },
      "parameters": []
    },
    {
      "name": "device_status",
      "display": {"label": {"zh": "查看设备状态", "en": "Read device status"}},
      "description": {
        "zh": "获取设备、电量、内存、存储和网络状态。",
        "en": "Get device, battery, memory, storage, and network status."
      },
      "parameters": []
    },
    {
      "name": "search_weather",
      "display": {"label": {"zh": "查询天气", "en": "Look up weather"}, "icon": "web", "targetParameter": "location"},
      "description": {
        "zh": "查询指定地点的当前天气。",
        "en": "Look up current weather for a location."
      },
      "parameters": [
        {
          "name": "location",
          "description": {
            "zh": "城市或地点名称。",
            "en": "City or place name."
          },
          "type": "string",
          "required": true
        }
      ]
    },
    {
      "name": "set_reminder",
      "display": {"label": {"zh": "创建提醒", "en": "Create reminder"}, "icon": "notification", "targetParameter": "title"},
      "description": {
        "zh": "创建持久化的应用内通知提醒。插件启用且宿主定时事件运行期间，每分钟检查到期提醒；不是系统闹钟或系统提醒应用中的记录。",
        "en": "Create a persistent in-app notification reminder, checked once per minute while the plugin and Host events are active. Does not create a system alarm or a system reminder-app entry."
      },
      "parameters": [
        {
          "name": "title",
          "description": {
            "zh": "提醒标题。",
            "en": "Reminder title."
          },
          "type": "string",
          "required": true
        },
        {
          "name": "description",
          "description": {
            "zh": "提醒详情。",
            "en": "Reminder details."
          },
          "type": "string",
          "required": true
        },
        {
          "name": "due_date",
          "description": {
            "zh": "未来的到期时间，使用带 Z 或明确 UTC 偏移量的 ISO 8601 日期时间。",
            "en": "Future due time in ISO 8601 format with Z or an explicit UTC offset."
          },
          "type": "string",
          "required": true
        }
      ]
    },
    {
      "name": "take_screenshot",
      "display": {"label": {"zh": "截取屏幕", "en": "Take screenshot"}},
      "description": {
        "zh": "截取当前屏幕。",
        "en": "Capture the current screen."
      },
      "parameters": []
    },
    {
      "name": "list_reminders",
      "display": {"label": {"zh": "查看提醒", "en": "List reminders"}, "icon": "notification"},
      "description": {
        "zh": "查询插件内提醒及其待触发、已发送、取消或失败状态。",
        "en": "List plugin reminders and their pending, delivered, cancelled, or failed states."
      },
      "parameters": []
    },
    {
      "name": "cancel_reminder",
      "display": {"label": {"zh": "取消提醒", "en": "Cancel reminder"}, "icon": "notification", "targetParameter": "id"},
      "description": {
        "zh": "按提醒 ID 取消尚未开始投递的通知提醒。",
        "en": "Cancel a notification reminder by ID before delivery starts."
      },
      "parameters": [
        {
          "required": true,
          "description": {
            "zh": "set_reminder 返回的提醒 ID。",
            "en": "Reminder ID returned by set_reminder."
          },
          "type": "string",
          "name": "id"
        }
      ]
    }
  ]
}
*/
Object.defineProperty(exports, "__esModule", { value: true });
exports.get_current_date = get_current_date;
exports.device_status = device_status;
exports.search_weather = search_weather;
exports.set_reminder = set_reminder;
exports.list_reminders = list_reminders;
exports.cancel_reminder = cancel_reminder;
exports.take_screenshot = take_screenshot;
/** Returns the current local date and time in structured form. */
async function get_current_date() {
    const now = new Date();
    return {
        timestamp: now.getTime(),
        iso: now.toISOString(),
        local: now.toLocaleString(),
        date: {
            year: now.getFullYear(),
            month: now.getMonth() + 1,
            day: now.getDate(),
            weekday: now.toLocaleDateString(undefined, { weekday: "long" }),
        },
        time: { hours: now.getHours(), minutes: now.getMinutes(), seconds: now.getSeconds() },
    };
}
/** Returns the current device status supplied by the system Host. */
async function device_status() {
    return Tools.System.getDeviceInfo();
}
/** Looks up readable weather information through the existing web-visit Host. */
async function search_weather(params) {
    const query = encodeURIComponent(`${params.location} weather`);
    return Tools.Net.visit(`https://www.baidu.com/s?wd=${query}`);
}
/** Persists a notification reminder through the plugin's single main-runtime owner. */
async function set_reminder(params) {
    return ToolPkg.ipc.call("daily_life.reminders", {
        action: "create", title: params.title, description: params.description, due_date: params.due_date,
    }, { targetRuntime: "main" });
}
/** Lists reminder records without treating accepted reminders as delivered notifications. */
async function list_reminders() {
    return ToolPkg.ipc.call("daily_life.reminders", { action: "list" }, { targetRuntime: "main" });
}
/** Cancels a pending reminder through the same owner used by scheduled delivery. */
async function cancel_reminder(params) {
    return ToolPkg.ipc.call("daily_life.reminders", { action: "cancel", id: params.id }, { targetRuntime: "main" });
}
/** Captures the current screen through the existing system Host. */
async function take_screenshot() {
    return toolCall("capture_screenshot", {});
}
