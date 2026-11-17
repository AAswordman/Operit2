"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.maintenanceRequest = maintenanceRequest;
exports.onFeedbackMessage = onFeedbackMessage;
exports.registerToolPkg = registerToolPkg;
const channel_conversation_js_1 = require("./channel-conversation.js");
const screen_js_1 = require("./ui/screen.js");
const service_js_1 = require("./service.js");
const conversation_js_1 = require("./conversation.js");
const client_js_1 = require("./client.js");
const ROUTE = "toolpkg:com.operit.project_maintenance:ui:dashboard";
(0, service_js_1.registerService)();
async function maintenanceRequest(event) { return (0, service_js_1.receive)(event.payload); }
/** Only persisted user messages in an explicitly bound chat become feedback. */
async function onFeedbackMessage(event) {
    const p = event.eventPayload;
    if (event.eventName !== "message_persisted" || p.sender !== "user" || typeof p.chatId !== "string" || typeof p.timestamp !== "number" || typeof p.content !== "string")
        return;
    try {
        await (0, client_js_1.call)({ action: "app_message", chatId: p.chatId, timestamp: p.timestamp, text: p.content });
    }
    catch (error) {
        console.error("[project_maintenance.app_capture]", error instanceof Error ? error.message : String(error));
    }
}
function registerToolPkg() {
    ToolPkg.registerPromptFinalizeHook({ id: "maintenance_channel_conversation", function: channel_conversation_js_1.channelConversation });
    ToolPkg.registerChatMessageHook({ id: "maintenance_app_feedback", function: onFeedbackMessage });
    ToolPkg.registerPromptFinalizeHook({ id: "maintenance_conversation", function: conversation_js_1.maintenanceConversation });
    ToolPkg.registerApi({ name: "maintenance.request", function: maintenanceRequest });
    ToolPkg.registerUiRoute({ id: "dashboard", route: ROUTE, runtime: "compose_dsl", screen: screen_js_1.default, keepAlive: true, title: { zh: "维护中控", en: "Maintenance control" } });
    ToolPkg.registerNavigationEntry({ id: "maintenance_sidebar", route: ROUTE, surface: "main_sidebar_plugins", title: { zh: "维护中控", en: "Maintenance control" }, icon: "Build", order: 145 });
    return true;
}
