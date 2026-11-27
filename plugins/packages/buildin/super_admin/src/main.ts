import { receiveSessionRequest, sweepIdleSessions } from "./sessions";
import { onEnvironmentPrompt } from "./prompt";

export { onEnvironmentPrompt } from "./prompt";

ToolPkg.ipc.on("super_admin.sessions", receiveSessionRequest);

/** Registers a Host-driven clock owned by the plugin rather than individual tool calls. */
export function registerToolPkg(): boolean {
    ToolPkg.registerPromptFinalizeHook({ id: "terminal-environment", function: onEnvironmentPrompt });
    ToolPkg.registerPromptEstimateFinalizeHook({ id: "terminal-environment-estimate", function: onEnvironmentPrompt });
    ToolPkg.registerHostEventHook({
        id: "super_admin_idle_sessions", source: "interval",
        trigger: { kind: "interval", intervalMs: 60000 }, function: onClock,
    });
    return true;
}

/** Reclaims idle tool sessions even when the AI makes no further tool calls. */
export async function onClock(): Promise<void> {
    await sweepIdleSessions();
}
