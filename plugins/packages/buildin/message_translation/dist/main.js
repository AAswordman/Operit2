"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.openTranslateDialog = openTranslateDialog;
exports.test_connection = test_connection;
exports.test_tool_call = test_tool_call;
const TRANSLATE_DIALOG_SCREEN = "dist/ui/translate_dialog.ui.js";
/** Registers translation for both user and assistant message menus. */
function registerToolPkg() {
    ToolPkg.registerChatMessageMenuItem({
        id: "translate_message",
        title: {
            zh: "翻译",
            en: "Translate",
        },
        icon: "translate",
        order: 20,
        senders: ["user", "ai"],
        dialog: {
            screen: TRANSLATE_DIALOG_SCREEN,
            title: {
                zh: "翻译消息",
                en: "Translate Message",
            },
        },
        function: openTranslateDialog,
    });
    return true;
}
/** Supplies the selected message to the registered translation dialog. */
function openTranslateDialog(event) {
    const payload = event.eventPayload;
    return {
        dialog: {
            state: {
                chatId: payload.chatId,
                messageIndex: payload.messageIndex,
                message: payload.message,
            },
            moduleSpec: {
                id: "translate_message_dialog",
                source: "builtin_message_translation",
            },
        },
    };
}
/** Connectivity proves the existing plugin main runtime actually executed this function. */
function test_connection() {
    return { passed: true };
}
/** Never invoke state-changing hooks or fabricate a tool success for a UI-only plugin. */
function test_tool_call() {
    return { passed: false, message: "此插件仅提供界面或聊天钩子，没有业务工具" };
}
