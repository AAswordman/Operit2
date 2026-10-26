"use strict";
/* METADATA
{
    "name": "plan_mode_tools",
    "display_name": {
        "zh": "计划模式工具",
        "en": "Plan Mode Tools"
    },
    "description": {
        "zh": "读取和完成当前计划内容。",
        "en": "Read and complete the current plan content."
    },
    "enabledByDefault": true,
    "tools": [
        {
            "name": "get_plan",
            "description": {
                "zh": "读取当前计划内容。",
                "en": "Read the current plan content."
            },
            "parameters": []
        },
        {
            "name": "complete_plan",
            "description": {
                "zh": "在所有计划项都真正完成后，完成当前计划。",
                "en": "Complete the current plan after every planned item is truly done."
            },
            "parameters": []
        }
    ]
}
*/
Object.defineProperty(exports, "__esModule", { value: true });
exports.get_plan = get_plan;
exports.complete_plan = complete_plan;
const plan_mode_i18n_js_1 = require("../shared/plan_mode_i18n.js");
const plan_mode_plan_file_js_1 = require("../shared/plan_mode_plan_file.js");
/** Converts a supported tool error value to user-visible text. */
function toErrorText(error) {
    if (error instanceof Error) {
        return error.message || "error";
    }
    return error || "error";
}
/** Completes the tool call with the current plugin-owned chat plan content. */
async function get_plan() {
    const text = (0, plan_mode_i18n_js_1.resolvePlanModeI18n)();
    const chatId = getChatId();
    if (chatId === undefined) {
        complete({
            success: false,
            error: "chatId is unavailable",
        });
        return;
    }
    try {
        const plan = await (0, plan_mode_plan_file_js_1.readPlanFile)(chatId);
        if (!plan) {
            complete({
                success: false,
                error: text.toastPlanAlreadyCompleted,
            });
            return;
        }
        complete({
            success: true,
            content: plan.content,
        });
    }
    catch (error) {
        const errorText = error instanceof Error || typeof error === "string" || error == null
            ? toErrorText(error)
            : "error";
        complete({
            success: false,
            error: errorText,
        });
    }
}
/** Deletes the current plugin-owned chat plan after implementation completes. */
async function complete_plan() {
    const chatId = getChatId();
    if (chatId === undefined) {
        complete({
            success: false,
            error: "chatId is unavailable",
        });
        return;
    }
    try {
        await (0, plan_mode_plan_file_js_1.deletePlanFile)(chatId);
        complete({
            success: true,
        });
    }
    catch (error) {
        const errorText = error instanceof Error || typeof error === "string" || error == null
            ? toErrorText(error)
            : "error";
        complete({
            success: false,
            error: errorText,
        });
    }
}
