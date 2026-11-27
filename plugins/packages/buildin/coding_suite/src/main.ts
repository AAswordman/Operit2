import screen from "./settings.ui.js";
import { config } from "./config";
import { composeContext, onCodingPrompt, onCodingEstimatePrompt } from "./prompt";
import { enqueueMemory, runMemoryJob, retryMemoryJobs } from "./memory";

export { onCodingPrompt, onCodingEstimatePrompt } from "./prompt";

ToolPkg.ipc.on("coding.context", composeContext);
ToolPkg.ipc.on("coding.memory.enqueue", enqueueMemory);
ToolPkg.ipc.on("coding.memory.tick", runMemoryJob);
ToolPkg.ipc.on("coding.settings", settingsRequest);

/** Reads and edits the plain prompt independently from character-card configuration. */
export async function settingsRequest(request: { action: "read" | "save" | "retry"; prompt?: string; memoryEnabled?: boolean }) {
    const settings = await config();
    if (request.action === "save") {
        if (typeof request.prompt !== "string" || typeof request.memoryEnabled !== "boolean") throw new Error("Invalid coding settings");
        settings.prompt = request.prompt;
        settings.memoryEnabled = request.memoryEnabled;
        await PluginConfig.flush(settings);
    } else if (request.action === "retry") await retryMemoryJobs();
    else if (request.action !== "read") throw new Error("Unknown coding settings action");
    return { prompt: settings.prompt, memoryEnabled: settings.memoryEnabled,
        pending: settings.jobs.length, errors: settings.jobs.filter(job => job.error !== null).map(job => job.error) };
}

/** Enqueues persisted assistant replies through the single main-runtime storage owner. */
export async function onCodingMessage(event: ToolPkg.ChatMessageHookEvent): Promise<void> {
    if (event.eventName !== "message_persisted") return;
    await ToolPkg.ipc.call("coding.memory.enqueue", event.eventPayload, { targetRuntime: "main" });
}

/** Runs the background worker through the same single owner used by prompt and UI requests. */
export async function onCodingClock(): Promise<void> {
    await ToolPkg.ipc.call("coding.memory.tick", {}, { targetRuntime: "main" });
}

/** Registers a coding environment whose activation suspends the character dependency graph. */
export function registerToolPkg(): boolean {
    ToolPkg.registerPromptFinalizeHook({ id: "coding-prompt", function: onCodingPrompt });
    ToolPkg.registerPromptEstimateFinalizeHook({ id: "coding-prompt-estimate", function: onCodingEstimatePrompt });
    ToolPkg.registerChatMessageHook({ id: "coding-memory-enqueue", function: onCodingMessage });
    ToolPkg.registerHostEventHook({ id: "coding-memory-worker", source: "interval", trigger: { kind: "interval", intervalMs: 60000 }, function: onCodingClock });
    const route = "toolpkg:com.operit.coding_suite:ui:settings";
    const title = { zh: "编程套装", en: "Coding Suite" };
    ToolPkg.registerUiRoute({ id: "settings", route, title, runtime: "compose_dsl", screen });
    ToolPkg.registerNavigationEntry({ id: "coding-settings", route, title, surface: "toolbox", icon: "Code", order: 160 });
    ToolPkg.registerNavigationEntry({ id: "coding-settings-sidebar", route, title, surface: "main_sidebar_plugins", icon: "Code", order: 160 });
    return true;
}
