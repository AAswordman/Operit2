"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
const settings_ui_js_1 = __importDefault(require("./ui/settings.ui.js"));
const service_js_1 = require("./service.js");
(0, service_js_1.registerService)();
/** Registers QQ Bot controls and the listener's application lifecycle. */
function registerToolPkg() {
    ToolPkg.registerToolboxUiModule({ id: "qqbot_settings", runtime: "compose_dsl", screen: settings_ui_js_1.default, params: {}, title: { zh: "QQ Bot 设置", en: "QQ Bot Settings" } });
    ToolPkg.registerAppLifecycleHook({ id: "qqbot_startup", event: "application_on_create", function: service_js_1.restore });
    ToolPkg.registerAppLifecycleHook({ id: "qqbot_foreground", event: "application_on_foreground", function: service_js_1.restore });
    ToolPkg.registerAppLifecycleHook({ id: "qqbot_shutdown", event: "application_on_terminate", function: service_js_1.terminate });
    return true;
}
