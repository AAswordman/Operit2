import { config, join, memoryPath, type Binding } from "./config";

/** Refreshes project context in the main runtime, including edits made since the last request. */
export async function composeContext(request: { chatId: string; binding: Binding; recordBinding: boolean }): Promise<string> {
    const settings = await config();
    const previous = settings.bindings[request.chatId];
    if (request.recordBinding && (previous?.base !== request.binding.base || previous?.workspace !== request.binding.workspace)) {
        settings.bindings = { ...settings.bindings, [request.chatId]: request.binding };
        await PluginConfig.flush(settings);
    }
    const sections: string[] = [];
    if (settings.prompt.trim() !== "") sections.push(settings.prompt);
    if (request.binding.workspace !== null) {
        const path = join(request.binding.workspace, "AGENTS.md");
        const file = await Tools.Files.exists(path);
        if (file.exists) {
            if (file.isDirectory) throw new Error("Workspace AGENTS.md must be a file");
            sections.push("Project instructions (" + path + "):\n" + (await Tools.Files.read(path)).content);
        }
    }
    if (settings.memoryEnabled) sections.push("Project timeline: " + memoryPath(request.binding)
        + "\nThe background worker appends dated summaries here. The file may not exist before the first summary."
        + " Search it with grep/rg only when historical context is relevant; read matching sections and verify current facts. Do not load the whole file by default or overwrite past entries.");
    return sections.join("\n\n");
}

/** Adds fresh coding instructions once at the final send and token-estimation boundary. */
export async function onCodingPrompt(event: ToolPkg.PromptFinalizeHookEvent): Promise<ToolPkg.PromptHookObjectResult | null> {
    return buildCodingPrompt(event, true);
}

/** Estimates the same context without redirecting pending conversation memory. */
export async function onCodingEstimatePrompt(event: ToolPkg.PromptEstimateFinalizeHookEvent): Promise<ToolPkg.PromptHookObjectResult | null> {
    return buildCodingPrompt(event, false);
}

/** Builds one fresh context contribution without changing any previous history turns. */
async function buildCodingPrompt(event: ToolPkg.PromptFinalizeHookEvent, recordBinding: boolean): Promise<ToolPkg.PromptHookObjectResult | null> {
    const payload = event.eventPayload;
    if (payload.stage !== "before_send_to_model" || payload.promptFunctionType !== "CHAT" || !payload.preparedHistory || !payload.chatId) return null;
    const workspace = payload.metadata?.workspacePath;
    const output = payload.metadata?.outputPath;
    if (workspace !== null && workspace !== undefined && (typeof workspace !== "string" || workspace === "")) throw new Error("Invalid workspace VFS path");
    const binding: Binding = typeof workspace === "string" ? { base: workspace, workspace }
        : { base: requireOutputPath(output), workspace: null };
    const content = await ToolPkg.ipc.call<{ chatId: string; binding: Binding; recordBinding: boolean }, string>("coding.context", { chatId: payload.chatId, binding, recordBinding }, { targetRuntime: "main" });
    const history = payload.preparedHistory.filter(turn => turn.metadata?.contextOwner !== "com.operit.coding_suite");
    if (content !== "") {
        const systemIndex = history.findIndex(turn => turn.kind === "SYSTEM");
        history.splice(systemIndex + 1, 0, { kind: "SYSTEM", content, metadata: { contextOwner: "com.operit.coding_suite" } });
    }
    return { preparedHistory: history };
}

/** Requires the host's actual conversation output binding when no workspace is selected. */
function requireOutputPath(value: unknown): string {
    if (typeof value !== "string" || value === "") throw new Error("Coding Suite requires the host conversation output path");
    return value;
}
