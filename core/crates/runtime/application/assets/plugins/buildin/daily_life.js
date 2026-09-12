/* METADATA
{
  "name": "daily_life",
  "display_name": {
    "zh": "日常生活工具包",
    "en": "Daily Life Toolkit"
  },
  "description": {
    "zh": "日常生活工具集合：日期时间、设备状态、天气、提醒、闹钟、短信、电话、社交应用操作、手电筒、音量、Wi-Fi、截图、拍照和深色模式。设备交互通过 Host UI 自动化执行。",
    "en": "Daily utilities for date/time, device status, weather, reminders, alarms, messaging, calls, social-app actions, flashlight, volume, Wi-Fi, screenshots, photos, and dark mode. Device interactions run through Host UI automation."
  },
  "enabledByDefault": true,
  "category": "Life",
  "tools": [
    { "name": "get_current_date", "description": { "zh": "获取当前日期和时间。", "en": "Get the current date and time." }, "parameters": [] },
    { "name": "device_status", "description": { "zh": "获取设备、电量、内存、存储和网络状态。", "en": "Get device, battery, memory, storage, and network status." }, "parameters": [] },
    { "name": "search_weather", "description": { "zh": "查询指定地点的当前天气。", "en": "Look up current weather for a location." }, "parameters": [{ "name": "location", "description": { "zh": "城市或地点名称。", "en": "City or place name." }, "type": "string", "required": true }] },
    { "name": "set_reminder", "description": { "zh": "在设备提醒应用中创建提醒。", "en": "Create a reminder in the device reminder app." }, "parameters": [{ "name": "title", "description": { "zh": "提醒标题。", "en": "Reminder title." }, "type": "string", "required": true }, { "name": "description", "description": { "zh": "提醒详情。", "en": "Reminder details." }, "type": "string", "required": true }, { "name": "due_date", "description": { "zh": "到期时间，使用 ISO 8601 格式。", "en": "Due time in ISO 8601 format." }, "type": "string", "required": true }] },
    { "name": "set_alarm", "description": { "zh": "在设备时钟应用中设置闹钟。", "en": "Set an alarm in the device clock app." }, "parameters": [{ "name": "hour", "description": { "zh": "小时，0-23。", "en": "Hour, from 0 to 23." }, "type": "number", "required": true }, { "name": "minute", "description": { "zh": "分钟，0-59。", "en": "Minute, from 0 to 59." }, "type": "number", "required": true }, { "name": "message", "description": { "zh": "闹钟标签。", "en": "Alarm label." }, "type": "string", "required": true }] },
    { "name": "send_message", "description": { "zh": "在设备短信应用中编写短信。", "en": "Compose an SMS in the device messaging app." }, "parameters": [{ "name": "phone_number", "description": { "zh": "收件人号码。", "en": "Recipient phone number." }, "type": "string", "required": true }, { "name": "message", "description": { "zh": "短信内容。", "en": "SMS body." }, "type": "string", "required": true }] },
    { "name": "wechat_post_moments", "description": { "zh": "在微信朋友圈编辑文本动态。", "en": "Compose a text post for WeChat Moments." }, "parameters": [{ "name": "message", "description": { "zh": "动态内容。", "en": "Post body." }, "type": "string", "required": true }] },
    { "name": "make_phone_call", "description": { "zh": "在电话应用中拨打号码。", "en": "Place a call through the device phone app." }, "parameters": [{ "name": "phone_number", "description": { "zh": "电话号码。", "en": "Phone number." }, "type": "string", "required": true }] },
    { "name": "toggle_flashlight", "description": { "zh": "打开或关闭手电筒。", "en": "Turn the flashlight on or off." }, "parameters": [{ "name": "state", "description": { "zh": "on 或 off。", "en": "on or off." }, "type": "string", "required": true }] },
    { "name": "adjust_volume", "description": { "zh": "调节设备音量。", "en": "Adjust device volume." }, "parameters": [{ "name": "action", "description": { "zh": "up、down 或 mute。", "en": "up, down, or mute." }, "type": "string", "required": true }, { "name": "count", "description": { "zh": "调节次数。", "en": "Number of adjustments." }, "type": "number", "required": true }] },
    { "name": "toggle_wifi", "description": { "zh": "打开或关闭 Wi-Fi。", "en": "Turn Wi-Fi on or off." }, "parameters": [{ "name": "state", "description": { "zh": "on 或 off。", "en": "on or off." }, "type": "string", "required": true }] },
    { "name": "take_screenshot", "description": { "zh": "截取当前屏幕。", "en": "Capture the current screen." }, "parameters": [] },
    { "name": "take_photo", "description": { "zh": "打开相机并拍照。", "en": "Open the camera and take a photo." }, "parameters": [] },
    { "name": "toggle_dark_mode", "description": { "zh": "设置设备深色模式。", "en": "Set device dark mode." }, "parameters": [{ "name": "state", "description": { "zh": "on、off 或 auto。", "en": "on, off, or auto." }, "type": "string", "required": true }] }
  ]
} */
/** Validates a fixed set of accepted values. */
function requireOneOf(value, allowed, name) {
    if (!allowed.includes(value)) {
        throw new Error(`${name} must be one of: ${allowed.join(", ")}`);
    }
    return value;
}
/** Runs one device operation through the platform-neutral UI automation host. */
async function runDeviceAction(action, instruction, targetApp) {
    const agentId = `daily-life-${action}-${Date.now()}`;
    const result = await Tools.UI.runSubAgent(instruction, 24, agentId, targetApp);
    if (!result.executionSuccess) {
        throw new Error(result.executionError ?? result.executionMessage);
    }
    return { action, result };
}
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
        time: {
            hours: now.getHours(),
            minutes: now.getMinutes(),
            seconds: now.getSeconds(),
        },
    };
}
/** Returns the current device status supplied by the system Host. */
async function device_status() {
    return await Tools.System.getDeviceInfo();
}
/** Looks up readable weather information for a supplied location. */
async function search_weather(params) {
    const query = encodeURIComponent(`${params.location} weather`);
    return await Tools.Net.visit(`https://www.baidu.com/s?wd=${query}`);
}
/** Creates a reminder by driving the device reminder application. */
async function set_reminder(params) {
    return await runDeviceAction("set_reminder", `Open the device reminder application. Create a reminder titled "${params.title}" with details "${params.description}" due at "${params.due_date}". Leave the reminder ready for the user to confirm.`);
}
/** Creates an alarm by driving the device clock application. */
async function set_alarm(params) {
    if (!Number.isInteger(params.hour) || params.hour < 0 || params.hour > 23) {
        throw new Error("hour must be an integer from 0 to 23");
    }
    if (!Number.isInteger(params.minute) || params.minute < 0 || params.minute > 59) {
        throw new Error("minute must be an integer from 0 to 59");
    }
    return await runDeviceAction("set_alarm", `Open the device clock application. Create an alarm for ${params.hour}:${params.minute} with the label "${params.message}". Leave the alarm ready for the user to confirm.`);
}
/** Composes an SMS through the device messaging application. */
async function send_message(params) {
    return await runDeviceAction("send_message", `Open the device messaging application. Compose an SMS to "${params.phone_number}" with the text "${params.message}". Leave the message ready for the user to confirm sending.`);
}
/** Composes a WeChat Moments text post through UI automation. */
async function wechat_post_moments(params) {
    return await runDeviceAction("wechat_post_moments", `Open WeChat Moments. Create a text post with the content "${params.message}". Leave it ready for the user to confirm publishing.`, "com.tencent.mm");
}
/** Opens the phone application and prepares a call. */
async function make_phone_call(params) {
    return await runDeviceAction("make_phone_call", `Open the device phone application. Enter the number "${params.phone_number}" and open the call screen. Leave the call ready for the user to confirm.`);
}
/** Changes the flashlight state through the device control surface. */
async function toggle_flashlight(params) {
    const state = requireOneOf(params.state, ["on", "off"], "state");
    return await runDeviceAction("toggle_flashlight", `Open the device control surface and turn the flashlight ${state}.`);
}
/** Adjusts the device volume through the device control surface. */
async function adjust_volume(params) {
    const action = requireOneOf(params.action, ["up", "down", "mute"], "action");
    if (!Number.isInteger(params.count) || params.count < 1 || params.count > 20) {
        throw new Error("count must be an integer from 1 to 20");
    }
    return await runDeviceAction("adjust_volume", `Open the device volume controls. Apply the "${action}" action exactly ${params.count} time(s).`);
}
/** Changes the Wi-Fi state through the device settings surface. */
async function toggle_wifi(params) {
    const state = requireOneOf(params.state, ["on", "off"], "state");
    return await runDeviceAction("toggle_wifi", `Open the device network settings and turn Wi-Fi ${state}.`);
}
/** Captures the current screen through the system Host. */
async function take_screenshot() {
    return await toolCall("capture_screenshot", {});
}
/** Opens the camera application through UI automation. */
async function take_photo() {
    return await runDeviceAction("take_photo", "Open the device camera application and switch to photo capture. Leave the camera ready for the user to take the photo.");
}
/** Changes dark-mode preference through the device settings surface. */
async function toggle_dark_mode(params) {
    const state = requireOneOf(params.state, ["on", "off", "auto"], "state");
    return await runDeviceAction("toggle_dark_mode", `Open the device display settings and set dark mode to "${state}".`);
}
exports.get_current_date = get_current_date;
exports.device_status = device_status;
exports.search_weather = search_weather;
exports.set_reminder = set_reminder;
exports.set_alarm = set_alarm;
exports.send_message = send_message;
exports.wechat_post_moments = wechat_post_moments;
exports.make_phone_call = make_phone_call;
exports.toggle_flashlight = toggle_flashlight;
exports.adjust_volume = adjust_volume;
exports.toggle_wifi = toggle_wifi;
exports.take_screenshot = take_screenshot;
exports.take_photo = take_photo;
exports.toggle_dark_mode = toggle_dark_mode;
