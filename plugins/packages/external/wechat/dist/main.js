"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
const settings_ui_js_1 = __importDefault(require("./ui/settings.ui.js"));
const service_js_1 = require("./service.js");
(0, service_js_1.registerService)();
/** Registers WeChat settings and the application-owned Bot lifecycle. */
function registerToolPkg() {
    ToolPkg.registerToolboxUiModule({
        id: "wechat_settings",
        runtime: "compose_dsl",
        screen: settings_ui_js_1.default,
        params: {},
        title: { zh: "微信 Bot 设置", en: "WeChat Bot Settings" },
    });
    ToolPkg.registerAppLifecycleHook({
        id: "wechat_startup",
        event: "application_on_create",
        function: service_js_1.restore,
    });
    ToolPkg.registerAppLifecycleHook({
        id: "wechat_foreground",
        event: "application_on_foreground",
        function: service_js_1.restore,
    });
    ToolPkg.registerAppLifecycleHook({
        id: "wechat_shutdown",
        event: "application_on_terminate",
        function: service_js_1.terminate,
    });
    return true;
}
