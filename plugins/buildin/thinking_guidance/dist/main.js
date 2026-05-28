const ENV_KEY = "OPERIT_THINKING_GUIDANCE_ENABLED";
const TOGGLE_ID = "thinking_guidance";
const MENU_HOOK_ID = "thinking_guidance_menu";
const PROMPT_HOOK_ID = "thinking_guidance_prompt";
const ENABLED_VALUES = new Set(["1", "true", "yes", "on"]);
const THINKING_GUIDANCE_PROMPT_EN = `THINKING PROCESS GUIDELINES:
- Before providing your final response, you MUST use a <think> block to outline your thought process. This is for your internal monologue.
- In your thoughts, deconstruct the user's request, consider alternatives, anticipate outcomes, and reflect on the best strategy. Formulate a precise action plan. Your plan should be efficient, and you may use tools in parallel or sequentially as appropriate. The tool system will decide and handle execution conflicts automatically.
- The user will see your thoughts but cannot reply to them directly. This block is NOT saved in the chat history, so your final answer must be self-contained.
- The <think> block must be immediately followed by your final answer or tool call without any newlines.
- CRITICAL REMINDER: Even if previous messages in the chat history do not show a <think> block, you MUST include one in your current response. This is a mandatory instruction for this conversation mode.`;
const THINKING_GUIDANCE_PROMPT_ZH = `思考过程指南：
- 在提供最终答案之前，你必须使用 <think> 模块来阐述你的思考过程。这是你的内心独白。
- 在思考中，你需要拆解用户需求，评估备选方案，预判执行结果，并反思最佳策略，最终形成精确的行动计划。你的计划应当是高效的，工具既可以并行调用，也可以串行调用；具体冲突由工具系统自行决定并处理。
- 用户能看到你的思考过程，但无法直接回复。此模块不会保存在聊天记录中，因此你的最终答案必须是完整的。
- <think> 模块必须紧邻你的最终答案或工具调用，中间不要有任何换行。
- 重要提醒：即使聊天记录中之前的消息没有 <think> 模块，你在本次回复中也必须按要求使用它。这是强制指令。`;
function readEnabled() {
    if (typeof getEnv !== "function") {
        return false;
    }
    const raw = getEnv(ENV_KEY);
    return ENABLED_VALUES.has(String(raw || "").trim().toLowerCase());
}
async function writeEnabled(enabled) {
    await Tools.SoftwareSettings.writeEnvironmentVariable(ENV_KEY, enabled ? "true" : "false");
}
function preferredLanguage(event) {
    return event && event.eventPayload && event.eventPayload.useEnglish ? "en" : "zh";
}
function guidancePrompt(event) {
    return preferredLanguage(event) === "en" ? THINKING_GUIDANCE_PROMPT_EN : THINKING_GUIDANCE_PROMPT_ZH;
}
async function onInputMenuToggle(event) {
    const payload = (event && event.eventPayload) || {};
    const action = payload.action;
    const enabled = readEnabled();
    if (action === "create") {
        return {
            toggles: [
                {
                    id: TOGGLE_ID,
                    title: preferredLanguage(event) === "en" ? "Thinking Guidance" : "思考引导",
                    description: preferredLanguage(event) === "en"
                        ? "Injects <think> guidance for non-reasoning models. Not recommended for native reasoning models."
                        : "为非思考模型注入 <think> 思考引导；不建议对原生思考模型开启。",
                    isChecked: enabled,
                    slot: "thinking",
                },
            ],
        };
    }
    if (action === "toggle" && payload.toggleId === TOGGLE_ID) {
        await writeEnabled(!enabled);
        return null;
    }
    return null;
}
function onSystemPromptCompose(event) {
    const stage = (event && (event.eventName || event.event)) || "";
    if (stage !== "after_compose_system_prompt") {
        return null;
    }
    if (!readEnabled()) {
        return null;
    }
    const currentPrompt = (event && event.eventPayload && event.eventPayload.systemPrompt) || "";
    return { systemPrompt: `${currentPrompt}\n\n${guidancePrompt(event)}` };
}
function registerToolPkg() {
    ToolPkg.registerInputMenuTogglePlugin({
        id: MENU_HOOK_ID,
        function: onInputMenuToggle,
    });
    ToolPkg.registerSystemPromptComposeHook({
        id: PROMPT_HOOK_ID,
        function: onSystemPromptCompose,
    });
    return true;
}
if (typeof exports !== "undefined") {
    exports.registerToolPkg = registerToolPkg;
    exports.onInputMenuToggle = onInputMenuToggle;
    exports.onSystemPromptCompose = onSystemPromptCompose;
}
