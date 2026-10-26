"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.composePrompt = exports.sessionClose = exports.sessionFeed = exports.sessionOpen = exports.settingsSet = exports.settingsGet = void 0;
exports.registerToolPkg = registerToolPkg;
const merge_js_1 = require("./merge.js");
const mode_js_1 = require("./mode.js");
const response_js_1 = require("./response.js");
const settings_ui_js_1 = __importDefault(require("./ui/settings.ui.js"));
const admin_js_1 = require("./admin.js");
const service_js_1 = require("./service.js");
var service_js_2 = require("./service.js");
Object.defineProperty(exports, "settingsGet", { enumerable: true, get: function () { return service_js_2.settingsGet; } });
Object.defineProperty(exports, "settingsSet", { enumerable: true, get: function () { return service_js_2.settingsSet; } });
Object.defineProperty(exports, "sessionOpen", { enumerable: true, get: function () { return service_js_2.sessionOpen; } });
Object.defineProperty(exports, "sessionFeed", { enumerable: true, get: function () { return service_js_2.sessionFeed; } });
Object.defineProperty(exports, "sessionClose", { enumerable: true, get: function () { return service_js_2.sessionClose; } });
Object.defineProperty(exports, "composePrompt", { enumerable: true, get: function () { return service_js_2.composePrompt; } });
ToolPkg.ipc.on("waifu.settings", admin_js_1.admin);
/** Publishes the waifu protocol, prompt hook and native settings surface. */
function registerToolPkg() {
    ToolPkg.registerInputMenuTogglePlugin({ id: "waifu_mode", function: mode_js_1.inputMenu });
    ToolPkg.registerChatInputHook({ id: "waifu_merge_input", function: merge_js_1.mergeInput });
    ToolPkg.registerChatResponseHook({ id: "waifu_reply", function: response_js_1.responseHook });
    ToolPkg.registerApi({ name: "settings.get", function: service_js_1.settingsGet });
    ToolPkg.registerApi({ name: "settings.set", function: service_js_1.settingsSet });
    ToolPkg.registerApi({ name: "session.open", function: service_js_1.sessionOpen });
    ToolPkg.registerApi({ name: "session.feed", function: service_js_1.sessionFeed });
    ToolPkg.registerApi({ name: "session.close", function: service_js_1.sessionClose });
    ToolPkg.registerSystemPromptComposeHook({ id: "waifu_prompt", function: service_js_1.composePrompt });
    ToolPkg.registerToolboxUiModule({ id: "waifu_settings", runtime: "compose_dsl", screen: settings_ui_js_1.default, params: {}, title: { zh: "Waifu 模式", en: "Waifu Mode" } });
    return true;
}
