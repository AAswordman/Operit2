/** Public contract for dependent packages. A caller must declare com.operit.subagents. */
export interface AgentSummary {
  id: string; parentChatId: string; chatId: string; name: string; status: string;
  diagnostics?: Array<"terminal_capacity_exhausted">;
  error: string | null; result: { response: string | null; outcome: unknown } | null;
}
export type Request =
  | { action: "spawn"; parentChatId: string; name: string; task: string; participantId: string | null }
  | { action: "list"; parentChatId: string }
  | { action: "wait"; parentChatId: string; agentIds: string[]; timeoutMs: number }
  | { action: "send"; parentChatId: string; agentId: string; message: string; interrupt: boolean }
  | { action: "close" | "close_if_completed" | "release_idle_terminals"; parentChatId: string; agentId: string };
export interface TerminalCleanupResult { closed: string[]; retained: string[] }
export interface WaitResult { timed_out: boolean; agents: AgentSummary[] }
export function request<T>(payload: Request): Promise<T> {
  return ToolPkg.callDependency<Request, T>("com.operit.subagents", "agents.request", payload);
}
