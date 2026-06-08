"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.onPromptInput = onPromptInput;
exports.onPromptFinalize = onPromptFinalize;
exports.onInputMenuToggle = onInputMenuToggle;
const index_ui_js_1 = __importDefault(require("./ui/index.ui.js"));
const shared_1 = require("./shared");
RuntimeContext.register({
    loadSettings: shared_1.loadSettings,
    saveSettings: shared_1.saveSettings,
});
async function appendExtraInfoWithStatus(processedInput, chatId, activePrompt) {
    return (0, shared_1.appendExtraInfoToMessage)(processedInput, chatId || undefined, activePrompt);
}
function resolveHookActivePrompt(input) {
    return input.eventPayload.metadata?.activePrompt;
}
function registerToolPkg() {
    ToolPkg.registerToolboxUiModule({
        id: "message_insert_settings",
        runtime: "compose_dsl",
        screen: index_ui_js_1.default,
        params: {},
        title: {
            zh: "额外信息注入",
            en: "Extra Info Injection",
        },
    });
    ToolPkg.registerPromptInputHook({
        id: "message_insert_prompt_input",
        function: onPromptInput,
    });
    ToolPkg.registerPromptFinalizeHook({
        id: "message_insert_prompt_finalize",
        function: onPromptFinalize,
    });
    ToolPkg.registerInputMenuTogglePlugin({
        id: "message_insert_input_menu_toggle",
        function: onInputMenuToggle,
    });
    return true;
}
async function onPromptInput(input) {
    const stage = String(input.eventPayload.stage ?? input.eventName ?? "");
    if (stage !== "before_process") {
        return null;
    }
    const settings = await (0, shared_1.loadSettings)();
    if (!settings.persistInjectedContent) {
        return null;
    }
    const processedInput = String(input.eventPayload.processedInput ?? input.eventPayload.rawInput ?? "");
    if (!processedInput.trim()) {
        return null;
    }
    const chatId = String(input.eventPayload.chatId ?? getChatId() ?? "").trim();
    const activePrompt = resolveHookActivePrompt(input);
    return appendExtraInfoWithStatus(processedInput, chatId || undefined, activePrompt);
}
async function onPromptFinalize(input) {
    const stage = String(input.eventPayload.stage ?? input.eventName ?? "");
    if (stage !== "before_send_to_model") {
        return null;
    }
    const settings = await (0, shared_1.loadSettings)();
    if (settings.persistInjectedContent) {
        return null;
    }
    const processedInput = String(input.eventPayload.processedInput ?? input.eventPayload.rawInput ?? "");
    if (!processedInput.trim()) {
        return null;
    }
    const chatId = String(input.eventPayload.chatId ?? getChatId() ?? "").trim();
    const activePrompt = resolveHookActivePrompt(input);
    return appendExtraInfoWithStatus(processedInput, chatId || undefined, activePrompt);
}
async function onInputMenuToggle(input) {
    const action = String(input.eventPayload.action ?? "").toLowerCase();
    if (action === "toggle") {
        await (0, shared_1.setExtraInfoInjectionEnabled)(!(await (0, shared_1.getExtraInfoInjectionEnabled)()));
        return [];
    }
    if (action !== "create") {
        return [];
    }
    const text = (0, shared_1.resolveExtraInfoI18n)();
    return [
        {
            id: "message_extra_info_injection",
            icon: "post_add",
            title: text.menuTitle,
            description: text.menuDescription,
            isChecked: await (0, shared_1.getExtraInfoInjectionEnabled)(),
        },
    ];
}
