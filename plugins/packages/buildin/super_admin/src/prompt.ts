/** Adds accurate execution-device and shell semantics to each actual chat request. */
export async function onEnvironmentPrompt(event: ToolPkg.PromptFinalizeHookEvent): Promise<ToolPkg.PromptHookObjectResult | null> {
    const payload = event.eventPayload;
    if (payload.stage !== "before_send_to_model" || payload.promptFunctionType !== "CHAT" || !payload.preparedHistory) return null;
    const terminal = await Tools.System.terminal.info();
    const environment = {
        platform: terminal.platform,
        workspace: payload.metadata?.workspacePath,
        outputDirectory: payload.metadata?.outputPath,
        defaultProfileId: terminal.defaultProfileId, terminal: terminal.terminal, terminalType: terminal.terminalType,
        profiles: terminal.profiles.map(profile => ({ id: profile.id, name: profile.name,
            terminal: profile.terminal, terminalType: profile.terminalType, available: profile.available })),
    };
    const content = "Execution environment (current Core device):\n" + JSON.stringify(environment)
        + "\nUse the selected profile's shell syntax. Terminal run commands use separate processes; use terminal_open for an interactive shell.";
    const history = payload.preparedHistory.filter(turn => turn.metadata?.contextOwner !== "com.operit.super_admin");
    const systemIndex = history.findIndex(turn => turn.kind === "SYSTEM");
    history.splice(systemIndex + 1, 0, { kind: "SYSTEM", content, metadata: { contextOwner: "com.operit.super_admin" } });
    return { preparedHistory: history };
}
