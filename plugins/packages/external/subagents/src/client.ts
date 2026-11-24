import { CHANNEL, type Agent, type AgentSettings, type AgentSummary, type Capacity, type ParentSnapshot, type Request, type Response, type WaitResult } from "./model.js";
import { watchAgent, watchParent } from "./storage.js";

/** Calls the retained owner instead of storing task control inside a tool or mounted screen. */
function call(request: Request): Promise<Response> {
  return ToolPkg.ipc.call<Request, Response>(CHANNEL, request, { targetRuntime: "main" });
}

/** Provides typed operations for tools and UI over one explicit IPC protocol. */
export const Agents = {
  /** Reads shared execution slots from the owner. */
  async capacity(): Promise<Capacity> { return ToolPkg.ipc.call("subagents.capacity", { action: "get" }, { targetRuntime: "main" }); },
  /** Persists the shared execution limit through the owner. */
  async setLimit(limit: number): Promise<Capacity> { return ToolPkg.ipc.call("subagents.capacity", { action: "set", limit }, { targetRuntime: "main" }); },
  /** Reads behavior preferences shared by the current space. */
  async settings(): Promise<AgentSettings> { return ToolPkg.ipc.call("subagents.capacity", { action: "get_settings" }, { targetRuntime: "main" }); },
  /** Applies a complete, validated preference snapshot. */
  async setSettings(settings: AgentSettings): Promise<AgentSettings> { return ToolPkg.ipc.call("subagents.capacity", { action: "set_settings", settings }, { targetRuntime: "main" }); },
  /** Creates a child using an explicit source chat and execution participant. */
  async spawn(parentChatId: string, name: string, task: string, participantId: string | null): Promise<AgentSummary> {
    return await call({ action: "spawn", parentChatId, name, task, participantId }) as AgentSummary;
  },
  /** Returns the currently retained children of exactly one parent chat. */
  async list(parentChatId: string): Promise<ParentSnapshot> {
    return await call({ action: "list", parentChatId }) as ParentSnapshot;
  },
  /** Waits for any selected turn to finish without cancelling work on timeout. */
  async wait(parentChatId: string, agentIds: string[], timeoutMs: number): Promise<WaitResult> {
    return await call({ action: "wait", parentChatId, agentIds, timeoutMs }) as WaitResult;
  },
  /** Sends another turn and explicitly controls interruption of an active one. */
  async send(parentChatId: string, agentId: string, message: string, interrupt: boolean): Promise<AgentSummary> {
    return await call({ action: "send", parentChatId, agentId, message, interrupt }) as AgentSummary;
  },
  /** Closes a task and its descendants after cancelling their actual executions. */
  async close(parentChatId: string, agentId: string): Promise<AgentSummary> {
    return await call({ action: "close", parentChatId, agentId }) as AgentSummary;
  },
  /** Initializes the owner once and subscribes to synchronized composer summaries. */
  async *watch(parentChatId: string, signal: AbortSignal): AsyncGenerator<ParentSnapshot> {
    const initial = await Agents.list(parentChatId);
    if (signal.aborted) return;
    yield initial;
    yield* watchParent(parentChatId, signal);
  },
  /** Subscribes only to the selected agent's latest semantic execution snapshots. */
  async *watchDetail(parentChatId: string, agentId: string, signal: AbortSignal): AsyncGenerator<Agent> {
    await Agents.list(parentChatId);
    yield* watchAgent(parentChatId, agentId, signal);
  },
};
