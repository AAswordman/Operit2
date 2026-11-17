import type { Agent } from "./model.js";

export interface OwnedTerminal { sessionId: string; sessionName: string }

/** Only foreground super_admin receipts identify the agent-scoped default sessions.
 * The host must independently compare both id and name and check idleness. */
export function ownedForegroundTerminals(agent: Agent): OwnedTerminal[] {
  const found = new Map<string, OwnedTerminal>();
  for (const message of agent.messages) for (const part of message.parts) {
    if (part.kind !== "tool_result" || part.attributes.status !== "success") continue;
    const match = /^super_admin:(bash|zsh|shell|powershell)$/.exec(part.toolName ?? "");
    if (match === null && part.toolName !== "super_admin:close") continue;
    try {
      const value: unknown = JSON.parse(part.content);
      if (value === null || typeof value !== "object" || Array.isArray(value)) continue;
      const result = value as Record<string, unknown>;
      if (match === null) {
        if (result.__type === "TerminalSessionCloseResultData" && result.success === true && typeof result.sessionId === "string") found.delete(result.sessionId);
        continue;
      }
      if (result.background !== false || result.terminalType !== match[1] ||
          typeof result.sessionId !== "string" || !result.sessionId || result.sessionId.trim() !== result.sessionId) continue;
      const sessionName = `super_admin_default_session_${match[1]}_${agent.chatId}`;
      found.set(result.sessionId, {sessionId: result.sessionId, sessionName});
    } catch { /* Invalid or incomplete snapshots are never authority to close. */ }
  }
  return [...found.values()];
}
