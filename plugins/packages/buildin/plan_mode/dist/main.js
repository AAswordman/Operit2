"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.onPlantodoXmlRender = exports.onPlanaskXmlRender = exports.registerToolPkg = exports.onToolPromptCompose = exports.onSystemPromptCompose = exports.onPromptFinalize = exports.onPromptEstimateFinalize = exports.onInputMenuToggle = exports.onChatViewEvent = void 0;
exports.test_connection = test_connection;
exports.test_tool_call = test_tool_call;
var plan_mode_plugin_js_1 = require("./plugin/plan_mode_plugin.js");
Object.defineProperty(exports, "onChatViewEvent", { enumerable: true, get: function () { return plan_mode_plugin_js_1.onChatViewEvent; } });
Object.defineProperty(exports, "onInputMenuToggle", { enumerable: true, get: function () { return plan_mode_plugin_js_1.onInputMenuToggle; } });
Object.defineProperty(exports, "onPromptEstimateFinalize", { enumerable: true, get: function () { return plan_mode_plugin_js_1.onPromptEstimateFinalize; } });
Object.defineProperty(exports, "onPromptFinalize", { enumerable: true, get: function () { return plan_mode_plugin_js_1.onPromptFinalize; } });
Object.defineProperty(exports, "onSystemPromptCompose", { enumerable: true, get: function () { return plan_mode_plugin_js_1.onSystemPromptCompose; } });
Object.defineProperty(exports, "onToolPromptCompose", { enumerable: true, get: function () { return plan_mode_plugin_js_1.onToolPromptCompose; } });
Object.defineProperty(exports, "registerToolPkg", { enumerable: true, get: function () { return plan_mode_plugin_js_1.registerToolPkg; } });
var planask_xml_render_plugin_js_1 = require("./plugin/planask-xml-render-plugin.js");
Object.defineProperty(exports, "onPlanaskXmlRender", { enumerable: true, get: function () { return planask_xml_render_plugin_js_1.onPlanaskXmlRender; } });
var plantodo_xml_render_plugin_js_1 = require("./plugin/plantodo-xml-render-plugin.js");
Object.defineProperty(exports, "onPlantodoXmlRender", { enumerable: true, get: function () { return plantodo_xml_render_plugin_js_1.onPlantodoXmlRender; } });
/** Connectivity proves the existing plugin main runtime actually executed this function. */
function test_connection() {
    return { passed: true };
}
/** Never invoke state-changing hooks or fabricate a tool success for a UI-only plugin. */
function test_tool_call() {
    return { passed: false, message: "此插件仅提供界面或聊天钩子，没有业务工具" };
}
