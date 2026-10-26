"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.chatTarget = chatTarget;
exports.activeTarget = activeTarget;
exports.ensureMode = ensureMode;
exports.modeForChat = modeForChat;
exports.modeContext = modeContext;
exports.saveMode = saveMode;
exports.inputMenu = inputMenu;
const settings_js_1 = require("./settings.js");
let store = null;
/** Loads only the independent ordinary-chat mode authoring state. */
function loadModes() {
    if (store === null)
        store = PluginConfig.use("mode", { bindings: {} });
    return store;
}
/** Calls the declared character dependency without interpreting its private storage. */
function characters(method, input) {
    return ToolPkg.callDependency("com.operit.character_cards", method, input);
}
/** Resolves the exact selection already persisted on an actual conversation. */
async function chatTarget(chatId) {
    const binding = await characters("chat.configuration.binding.read", { chatId: (0, settings_js_1.identifier)(chatId, "chatId") });
    return (0, settings_js_1.identifier)(binding.selection, "character selection");
}
/** Reads the same currently active card or group shown by the original Waifu settings page. */
async function activeTarget() {
    const active = await characters("activePrompt.get", {});
    if ("CharacterCard" in active) {
        const card = await characters("character.get", active.CharacterCard);
        return { selection: `card:${active.CharacterCard.id}`, name: card.name };
    }
    if ("CharacterGroup" in active) {
        const group = await characters("group.get", active.CharacterGroup);
        return { selection: `group:${active.CharacterGroup.id}`, name: group.name };
    }
    if (active.Model.id !== "chat")
        throw new Error("Invalid active model selection");
    return { selection: "model:chat", name: "模型对话" };
}
/** Authors a new explicitly selected role binding with the original disabled defaults. */
async function ensureMode(selection) {
    (0, settings_js_1.identifier)(selection, "selection");
    const config = await loadModes();
    if (config.bindings[selection] === undefined) {
        config.bindings[selection] = { enabled: false, mergeSend: false, mergeDelayMs: 5000, settings: (0, settings_js_1.defaultSettings)() };
        await PluginConfig.flush(config);
    }
    return JSON.parse(JSON.stringify(config.bindings[selection]));
}
/** Reads an enabled role mode without creating a binding during ordinary model execution. */
async function modeForChat(chatId) {
    const selection = await chatTarget(chatId), config = await loadModes();
    const mode = config.bindings[selection];
    if (mode === undefined || !mode.enabled)
        return null;
    return JSON.parse(JSON.stringify(mode));
}
/** Loads the authoring context used by native settings and CLI controls. */
async function modeContext() {
    const target = await activeTarget();
    return { target, mode: await ensureMode(target.selection) };
}
/** Saves one complete role-bound mode and validates timing before persistence. */
async function saveMode(selection, mode) {
    (0, settings_js_1.identifier)(selection, "selection");
    if (typeof mode.enabled !== "boolean" || typeof mode.mergeSend !== "boolean")
        throw new Error("Mode switches must be boolean");
    if (!Number.isSafeInteger(mode.mergeDelayMs) || mode.mergeDelayMs < 500 || mode.mergeDelayMs > 10000)
        throw new Error("Merge delay must be between 500 and 10000 ms");
    const checked = { enabled: mode.enabled, mergeSend: mode.mergeSend, mergeDelayMs: mode.mergeDelayMs, settings: (0, settings_js_1.validateSettings)(mode.settings) };
    const config = await loadModes();
    config.bindings = { ...config.bindings, [selection]: checked };
    await PluginConfig.flush(config);
    return JSON.parse(JSON.stringify(checked));
}
/** Adds a native input-menu switch for the current conversation's bound role. */
async function inputMenu(event) {
    const { action, chatId, toggleId } = event.eventPayload;
    if (chatId === undefined || (action !== "create" && action !== "toggle"))
        return;
    if (action === "toggle" && toggleId !== "waifu.mode")
        return;
    const selection = await chatTarget(chatId), mode = await ensureMode(selection);
    if (action === "toggle")
        await saveMode(selection, { ...mode, enabled: !mode.enabled });
    else
        return { toggles: [{ id: "waifu.mode", title: "Waifu 模式", description: "按句子发送气泡，模拟自然聊天节奏", icon: "Face", isChecked: mode.enabled }] };
}
