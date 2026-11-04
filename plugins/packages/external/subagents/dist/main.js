"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.test_connection = test_connection;
exports.test_tool_call = test_tool_call;
const panel_ui_js_1 = __importDefault(require("./ui/panel.ui.js"));
const conversation_ui_js_1 = __importDefault(require("./ui/conversation.ui.js"));
const shared_js_1 = require("./ui/shared.js");
const service_js_1 = require("./service.js");
(0, service_js_1.registerService)();
/** Registers the same platform-independent composer contribution for every supported chat input style. */
function registerToolPkg() {
    ToolPkg.registerUiRoute({ id: "conversation", route: shared_js_1.CONVERSATION_ROUTE, runtime: "compose_dsl", screen: conversation_ui_js_1.default,
        keepAlive: true, title: { zh: "子代理会话", en: "Subagent conversation" } });
    ToolPkg.registerNavigationEntry({ id: "conversation", route: shared_js_1.CONVERSATION_ROUTE, surface: "workspace",
        title: { zh: "子代理会话", en: "Subagent conversation" }, icon: "account_tree" });
    ToolPkg.registerChatComposerSlot({ id: "subagents", slot: "above_input", screen: panel_ui_js_1.default, order: 20, keepAlive: true });
    return true;
}
/** Proves that the package main module can execute without creating conversations or tasks. */
function test_connection() {
    return { passed: true };
}
/** Checks the actual Chat SDK through a read-only host operation without starting model generation. */
async function test_tool_call() {
    await Tools.Chat.listChats({ limit: 1 });
    return { passed: true, message: "Chat SDK read succeeded. Model execution requires an explicit spawn_agent task." };
}
