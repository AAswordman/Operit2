import { AgentManager, type ChatPort, type Clock } from "./manager.js";
import { CHANNEL, requireText, type Request, type Response } from "./model.js";
import { Repository } from "./storage.js";

let manager: AgentManager | null = null;

/** Binds platform-independent execution exclusively through the current host Chat SDK. */
const chat: ChatPort = {
  /** Creates a persisted child without changing the visible parent chat. */
  async create(parentChatId) {
    return (await Tools.Chat.createNew({ setAsCurrentChat: false, sourceChatId: parentChatId, parentChatId })).chatId;
  },
  /** Observes the one original send through semantic message snapshots. */
  send(request) { return Tools.Chat.sendMessageStreaming(request); },
  /** Uses authenticated package ownership for native cancellation. */
  cancel(chatId) { return Tools.Chat.cancel(chatId); },
  /** Reads the structured processing flag without using the visible chat selection. */
  async isProcessing(chatId) {
    const result = await Tools.Chat.agentStatus(chatId);
    if (result.chatId !== chatId || typeof result.isProcessing !== "boolean") throw new Error("Invalid native chat processing result");
    return result.isProcessing;
  },
};

const clock: Clock = {
  /** Reads the host-backed JavaScript date clock. */
  now() { return Date.now(); },
  /** Returns the exact timer cleanup used by the manager's wait subscription. */
  schedule(callback, milliseconds) {
    const timer = setTimeout(callback, milliseconds);
    return () => { clearTimeout(timer); };
  },
  /** Allows native admission and cancellation callbacks to advance independently. */
  delay(milliseconds) { return new Promise<void>(resolve => { setTimeout(resolve, milliseconds); }); },
};

/** Constructs the retained owner lazily; registration does not open storage or start agents. */
function owner(): AgentManager {
  if (manager === null) manager = new AgentManager(new Repository(), chat, clock, error => { console.error("[subagents]", error); });
  return manager;
}

/** Validates the exact discriminated IPC request before entering the shared manager. */
export function decodeRequest(value: unknown): Request {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("Subagent request must be an object");
  const input = value as Record<string, unknown>;
  const parentChatId = requireText(input.parentChatId, "parentChatId");
  switch (input.action) {
    case "list": return { action: "list", parentChatId };
    case "spawn": {
      const participantId = input.participantId === null ? null : requireText(input.participantId, "participantId");
      return { action: "spawn", parentChatId, task: requireText(input.task, "task"), name: requireText(input.name, "name"), participantId };
    }
    case "wait": {
      if (!Array.isArray(input.agentIds) || !input.agentIds.every(id => typeof id === "string")) throw new Error("agentIds must be a string array");
      if (typeof input.timeoutMs !== "number") throw new Error("timeoutMs must be a number");
      return { action: "wait", parentChatId, agentIds: input.agentIds, timeoutMs: input.timeoutMs };
    }
    case "send": {
      if (typeof input.interrupt !== "boolean") throw new Error("interrupt must be boolean");
      return { action: "send", parentChatId, agentId: requireText(input.agentId, "agentId"), message: requireText(input.message, "message"), interrupt: input.interrupt };
    }
    case "close": return { action: "close", parentChatId, agentId: requireText(input.agentId, "agentId") };
    default: throw new Error("Unknown subagent operation");
  }
}

/** Receives all tools and UI actions in the same package main runtime. */
export async function receive(value: unknown): Promise<Response> {
  return owner().request(decodeRequest(value));
}

/** Declares handlers during module evaluation so targeted main-runtime IPC can initialize the service. */
export function registerService(): void {
  ToolPkg.ipc.on<unknown, Response>(CHANNEL, receive);
}
