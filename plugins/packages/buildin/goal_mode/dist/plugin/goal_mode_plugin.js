"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.onChatMessagePersisted = onChatMessagePersisted;
exports.onGoalCommand = onGoalCommand;
exports.onChatViewEvent = onChatViewEvent;
exports.onInputMenuToggle = onInputMenuToggle;
exports.onChatInput = onChatInput;
exports.onPromptFinalize = onPromptFinalize;
exports.registerToolPkg = registerToolPkg;
const index_ui_js_1 = __importDefault(require("../ui/goal_panel/index.ui.js"));
const goal_mode_ipc_js_1 = require("../shared/goal_mode_ipc.js");
const goal_mode_state_js_1 = require("../shared/goal_mode_state.js");
const GOAL_COMMAND_ID = "goal_mode_command";
const GOAL_COMMAND_NAME = "goal";
const GOAL_SLOT_ID = "goal_mode_above_input";
const GOAL_INPUT_MENU_HOOK_ID = "goal_mode_input_menu";
const GOAL_INPUT_SLOT_TOGGLE_ID = "goal_mode_input_slot";
const GOAL_CHAT_INPUT_HOOK_ID = "goal_mode_chat_input";
const MAX_GOAL_OBJECTIVE_LENGTH = 4000;
const reviewedMessageTimestamps = new Set();
/** Adds a non-empty supervision instruction to a composed prompt. */
function appendPrompt(base, extra) {
    const normalizedBase = base.trim();
    const normalizedExtra = extra.trim();
    return normalizedBase ? `${normalizedBase}\n\n${normalizedExtra}` : normalizedExtra;
}
/** Builds the model-visible supervision instruction for an active task goal. */
function buildGoalPrompt(objective, useEnglish) {
    if (useEnglish) {
        return [
            "An active task goal is attached to this chat.",
            `Goal: ${objective}`,
            "Keep working toward this goal across turns. Verify the requested result before declaring it complete.",
            "Return the actual work result in your response. Goal completion is checked separately after your response is persisted.",
        ].join("\n");
    }
    return [
        "当前聊天附带一个进行中的任务目标。",
        `目标：${objective}`,
        "你必须跨回合持续推进该目标，在确认结果真正完成前不得宣称完成。",
        "请直接在回复中给出实际工作结果。目标是否完成会在回复落库后由独立检查流程判断。",
    ].join("\n");
}
/** Builds the strict JSON request used to review one persisted assistant response. */
function buildGoalReviewPrompt(objective, response) {
    return [
        "Review whether the assistant response completed the task goal.",
        "Return only one JSON object, with exactly these fields:",
        '{"decision":"completed|incomplete|insufficient","reason":"...","missing":"..."}',
        "Use completed only when the goal is actually fulfilled.",
        "Use incomplete when the response is understandable but still misses required work; explain the gap in reason and missing.",
        "Use insufficient when the response is too vague or incomplete to determine whether the goal is fulfilled.",
        `Goal: ${objective}`,
        `Assistant response: ${response}`,
    ].join("\n");
}
/** Parses and validates the three-state goal review response. */
function parseGoalReviewResult(text) {
    const parsed = JSON.parse(text.trim());
    if (typeof parsed !== "object" || parsed === null) {
        throw new Error("Goal review response must be a JSON object.");
    }
    const value = parsed;
    const decision = value.decision;
    if (decision !== "completed" && decision !== "incomplete" && decision !== "insufficient") {
        throw new Error("Goal review response has an invalid decision.");
    }
    if (typeof value.reason !== "string" || typeof value.missing !== "string") {
        throw new Error("Goal review response must include reason and missing strings.");
    }
    return { decision, reason: value.reason, missing: value.missing };
}
/** Sends one follow-up message without blocking the persisted-message hook. */
function sendGoalFollowup(chatId, message) {
    void Tools.Chat.sendMessage({ kind: "submit", chatId, runtime: "main", input: { text: message, attachments: [], replyToMessageTimestamp: null }, turn: { kind: "execute" }, notifyReply: false }).catch((error) => {
        const detail = error instanceof Error ? error.message : String(error);
        void Tools.System.toast(`Goal follow-up failed: ${detail}`);
    });
}
/** Sends the next instruction that follows one goal review decision. */
async function applyGoalReviewDecision(chatId, review) {
    if (review.decision === "completed") {
        await (0, goal_mode_ipc_js_1.setGoalStatusAsync)(chatId, "completed");
        return;
    }
    if (review.decision === "incomplete") {
        sendGoalFollowup(chatId, `请继续完成当前 Goal。复核结果：${review.reason}\n仍缺少：${review.missing}`);
        return;
    }
    sendGoalFollowup(chatId, "请重新对照当前 Goal，逐项检查你刚才的结果是否真正完成目标。只补充实际缺失内容，并在最后明确说明仍未完成的部分。");
}
/** Reviews one persisted assistant response and applies the resulting goal transition. */
async function onChatMessagePersisted(event) {
    if (event.eventName !== "message_persisted") {
        return;
    }
    const payload = event.eventPayload;
    if (payload.sender !== "ai" ||
        typeof payload.chatId !== "string" ||
        typeof payload.timestamp !== "number" ||
        typeof payload.completedAt !== "number" ||
        payload.completedAt <= 0) {
        return;
    }
    const response = payload.content?.trim();
    if (!response) {
        return;
    }
    const goal = await (0, goal_mode_ipc_js_1.readGoalAsync)(payload.chatId);
    if (!goal || goal.status !== "active") {
        return;
    }
    const reviewKey = `${payload.chatId}:${payload.timestamp}`;
    if ((goal.lastReviewedMessageTimestamp !== undefined && payload.timestamp <= goal.lastReviewedMessageTimestamp) ||
        reviewedMessageTimestamps.has(reviewKey)) {
        return;
    }
    reviewedMessageTimestamps.add(reviewKey);
    try {
        const result = await Tools.Chat.call({
            functionType: "CHAT",
            turns: [
                {
                    kind: "SYSTEM",
                    content: "You are a strict task-completion reviewer. Do not call tools.",
                },
                { kind: "USER", content: buildGoalReviewPrompt(goal.objective, response) },
            ],
            recordTokenUsage: false,
            enableThinking: false,
        });
        const review = parseGoalReviewResult(result.text);
        await applyGoalReviewDecision(payload.chatId, review);
        await (0, goal_mode_ipc_js_1.markGoalMessageReviewedAsync)(payload.chatId, payload.timestamp);
    }
    finally {
        reviewedMessageTimestamps.delete(reviewKey);
    }
}
/** Resolves the single chat currently targeted by a slash command. */
async function requireActiveChatView() {
    const view = await (0, goal_mode_ipc_js_1.readSingleActiveChatViewAsync)();
    return view ?? {
        stderr: "No active chat view is available for /goal.",
        json: { ok: false, error: "chat_view_missing" },
    };
}
/** Distinguishes a command error result from a tracked chat view. */
function isCommandResult(value) {
    return "stdout" in value || "stderr" in value;
}
/** Builds the rejection result for an invalid one-message goal input. */
function buildGoalInputValidationResult(objective) {
    if (!objective) {
        return { action: "Block", message: "A task goal requires message text." };
    }
    if (objective.length > MAX_GOAL_OBJECTIVE_LENGTH) {
        return {
            action: "Block",
            message: `Task goal exceeds ${MAX_GOAL_OBJECTIVE_LENGTH} characters.`,
        };
    }
    return null;
}
/** Executes Codex-compatible goal state commands for the active chat. */
async function onGoalCommand(event) {
    const args = event.eventPayload.args.map((arg) => arg.trim()).filter((arg) => arg !== "");
    const viewOrResult = await requireActiveChatView();
    if (isCommandResult(viewOrResult)) {
        return viewOrResult;
    }
    const chatId = viewOrResult.chatId;
    if (!viewOrResult.workspacePath) {
        return { stderr: "A workspace is required for Goal Mode.", json: { ok: false, error: "workspace_required" } };
    }
    if (args.length === 0) {
        const goal = await (0, goal_mode_ipc_js_1.readGoalAsync)(chatId);
        return {
            stdout: goal ? `Goal (${goal.status}): ${goal.objective}` : "No task goal is set.",
            json: { ok: true, chatId, goal },
        };
    }
    const command = args[0].toLowerCase();
    if (command === "clear" && args.length === 1) {
        await (0, goal_mode_ipc_js_1.clearGoalAsync)(chatId);
        return { stdout: "Task goal cleared.", json: { ok: true, action: "clear", chatId } };
    }
    if (command === "pause" && args.length === 1) {
        const goal = await (0, goal_mode_ipc_js_1.setGoalStatusAsync)(chatId, "paused");
        return {
            stdout: goal ? "Task goal paused." : "No task goal is set.",
            json: { ok: goal !== null, action: "pause", chatId, goal },
        };
    }
    if (command === "resume" && args.length === 1) {
        const goal = await (0, goal_mode_ipc_js_1.setGoalStatusAsync)(chatId, "active");
        return {
            stdout: goal ? "Task goal resumed." : "No task goal is set.",
            json: { ok: goal !== null, action: "resume", chatId, goal },
        };
    }
    const objective = command === "edit" ? args.slice(1).join(" ") : args.join(" ");
    if (!objective) {
        return { stderr: "Goal objective is required.", json: { ok: false, error: "objective_required" } };
    }
    if (objective.length > MAX_GOAL_OBJECTIVE_LENGTH) {
        return {
            stderr: `Goal objective exceeds ${MAX_GOAL_OBJECTIVE_LENGTH} characters.`,
            json: { ok: false, error: "objective_too_long" },
        };
    }
    const goal = await (0, goal_mode_ipc_js_1.setGoalAsync)(chatId, objective);
    return {
        stdout: command === "edit" ? "Task goal updated." : "Task goal set.",
        json: { ok: true, action: command === "edit" ? "edit" : "set", chatId, goal },
    };
}
/** Tracks the chat view targeted by goal commands and Compose DSL panels. */
async function onChatViewEvent(event) {
    const payload = event.eventPayload;
    const viewId = payload.viewId;
    const chatId = payload.chatId;
    const runtime = payload.runtime;
    if (typeof viewId !== "string" || typeof chatId !== "string" || !(0, goal_mode_state_js_1.isGoalRuntime)(runtime)) {
        return;
    }
    if (event.eventName === "view_closed") {
        await (0, goal_mode_ipc_js_1.removeTrackedChatViewAsync)(runtime, viewId);
        return;
    }
    const workspacePath = payload.workspacePath;
    const title = payload.title;
    if (typeof workspacePath !== "string" || workspacePath.trim() === "") {
        await (0, goal_mode_ipc_js_1.removeTrackedChatViewAsync)(runtime, viewId);
        await (0, goal_mode_ipc_js_1.setGoalInputSlotEnabledAsync)(chatId, false);
        return;
    }
    if (typeof title !== "string") {
        return;
    }
    await (0, goal_mode_ipc_js_1.upsertTrackedChatViewAsync)({
        viewId,
        chatId,
        runtime,
        workspacePath,
        workspaceEnv: typeof payload.workspaceEnv === "string" ? payload.workspaceEnv : undefined,
        title,
        updatedAt: Date.now(),
    });
}
/** Creates and toggles the one-message goal input control in the chat input menu. */
async function onInputMenuToggle(event) {
    const payload = event.eventPayload;
    const action = payload.action;
    const chatId = payload.chatId;
    const runtime = payload.runtime === "main" || payload.runtime === "floating" ? payload.runtime : undefined;
    if ((action !== "create" && action !== "toggle") || chatId === undefined) {
        return null;
    }
    const enabled = await (0, goal_mode_ipc_js_1.isGoalInputSlotEnabledAsync)(chatId);
    if (action === "create") {
        const workspace = await (0, goal_mode_ipc_js_1.resolveGoalWorkspaceAsync)(chatId, runtime);
        return {
            toggles: [
                {
                    id: GOAL_INPUT_SLOT_TOGGLE_ID,
                    title: "Task goal",
                    description: workspace
                        ? "Use the next message as this chat's task goal."
                        : "A workspace is required before Goal Mode can be enabled.",
                    icon: Icons.Assignment,
                    isChecked: enabled,
                    slot: "general",
                },
            ],
        };
    }
    if (payload.toggleId === GOAL_INPUT_SLOT_TOGGLE_ID) {
        if (!enabled && !(await (0, goal_mode_ipc_js_1.resolveGoalWorkspaceAsync)(chatId, runtime))) {
            await Tools.System.toast("当前聊天未绑定工作区，不能开启目标模式。");
            return null;
        }
        await (0, goal_mode_ipc_js_1.setGoalInputSlotEnabledAsync)(chatId, !enabled);
    }
    return null;
}
/** Establishes a goal from the armed chat input and closes its one-message slot. */
async function onChatInput(event) {
    const payload = event.eventPayload;
    const chatId = payload.chatId;
    if ((event.eventName !== "submit_requested" && event.eventName !== "submitted") ||
        chatId === undefined ||
        !(await (0, goal_mode_ipc_js_1.isGoalInputSlotEnabledAsync)(chatId))) {
        return null;
    }
    const objective = (payload.text ?? "").trim();
    if (event.eventName === "submit_requested") {
        const validation = buildGoalInputValidationResult(objective);
        if (validation) {
            return validation;
        }
        if (!(await (0, goal_mode_ipc_js_1.resolveGoalWorkspaceAsync)(chatId))) {
            return { action: "Block", message: "当前聊天未绑定工作区，不能设置目标。" };
        }
    }
    if (event.eventName === "submitted") {
        await (0, goal_mode_ipc_js_1.setGoalAsync)(chatId, objective);
        await (0, goal_mode_ipc_js_1.setGoalInputSlotEnabledAsync)(chatId, false);
    }
    return null;
}
/** Adds active goal supervision immediately before the model request. */
async function onPromptFinalize(event) {
    const payload = event.eventPayload;
    const stage = payload.stage === undefined ? event.eventName : payload.stage;
    if (stage !== "before_send_to_model" || payload.chatId === undefined || payload.preparedHistory === undefined) {
        return null;
    }
    const goal = await (0, goal_mode_ipc_js_1.readGoalAsync)(payload.chatId);
    if (!goal || goal.status !== "active") {
        return null;
    }
    const instruction = buildGoalPrompt(goal.objective, payload.useEnglish === true);
    const history = payload.preparedHistory.slice();
    const systemIndex = history.findIndex((turn) => turn.kind === "SYSTEM");
    if (systemIndex < 0) {
        history.unshift({ kind: "SYSTEM", content: instruction });
    }
    else {
        history[systemIndex] = {
            ...history[systemIndex],
            content: appendPrompt(history[systemIndex].content, instruction),
        };
    }
    return { preparedHistory: history };
}
(0, goal_mode_ipc_js_1.registerGoalModeIpc)();
/** Registers goal commands, supervision hooks, and the composer panel. */
function registerToolPkg() {
    ToolPkg.registerCoreCommand({
        id: GOAL_COMMAND_ID,
        name: GOAL_COMMAND_NAME,
        title: { zh: "任务目标", en: "Task Goal" },
        description: { zh: "设置、查看、暂停、恢复、编辑或清除当前聊天的任务目标。", en: "Set, view, pause, resume, edit, or clear the current chat task goal." },
        usage: "/goal [objective|edit <objective>|pause|resume|clear]",
        function: onGoalCommand,
    });
    ToolPkg.registerInputMenuTogglePlugin({
        id: GOAL_INPUT_MENU_HOOK_ID,
        function: onInputMenuToggle,
    });
    ToolPkg.registerChatInputHook({ id: GOAL_CHAT_INPUT_HOOK_ID, function: onChatInput });
    ToolPkg.registerChatViewHook({ id: "goal_mode_chat_view", function: onChatViewEvent });
    ToolPkg.registerChatMessageHook({ id: "goal_mode_message_persisted", function: onChatMessagePersisted });
    ToolPkg.registerPromptFinalizeHook({ id: "goal_mode_prompt_finalize", function: onPromptFinalize });
    ToolPkg.registerPromptEstimateFinalizeHook({ id: "goal_mode_prompt_estimate", function: onPromptFinalize });
    ToolPkg.registerChatComposerSlot({
        id: GOAL_SLOT_ID,
        slot: "above_input",
        screen: index_ui_js_1.default,
        order: -100,
        keepAlive: true,
    });
    return true;
}
