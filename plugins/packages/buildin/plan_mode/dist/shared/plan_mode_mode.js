"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.isPlanModeEnabledForChat = isPlanModeEnabledForChat;
exports.enablePlanModeForChat = enablePlanModeForChat;
exports.disablePlanMode = disablePlanMode;
const plan_mode_plan_file_js_1 = require("./plan_mode_plan_file.js");
/** Reads the planning flag from synchronized plugin state. */
async function isPlanModeEnabledForChat(chatId) {
    const entry = await (await (0, plan_mode_plan_file_js_1.openPlanDatabase)()).collection("modes").get(chatId);
    if (entry === null)
        return false;
    if (entry.value !== true)
        throw new Error("Persisted plan mode flag is invalid");
    return true;
}
/** Enables planning for a chat across the device space. */
async function enablePlanModeForChat(chatId) {
    await (await (0, plan_mode_plan_file_js_1.openPlanDatabase)()).collection("modes").put(chatId, true);
}
/** Removes the synchronized planning flag for the chat. */
async function disablePlanMode(chatId) {
    await (await (0, plan_mode_plan_file_js_1.openPlanDatabase)()).collection("modes").remove(chatId);
}
