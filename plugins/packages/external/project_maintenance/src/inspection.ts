import type { Ticket, Inspection } from "./api.js";
export type { Inspection } from "./api.js";

interface Ports {
  claim(path: string): Promise<{ticketId: string} | null>;
  list(parentChatId: string): Promise<{schemaVersion: number; agents: {id: string; parentChatId: string; status: string}[]}>;
  status(chatId: string): Promise<{chatId: string; isIdle: boolean; isProcessing: boolean}>;
  now(): number;
}
const statuses = ["starting", "running", "completed", "cancelled", "failed", "closing", "closed", "interrupted"];
/** Read-only evidence. An idle native chat is not proof that external effects ended. */
export async function inspect(ticket: Ticket, ports: Ports): Promise<Inspection> {
  let claim: Inspection["claim"] = "unknown", managed = ticket.agentId === null ? (ticket.stage === "interrupted" || ["triage", "implement", "review", "delivery"].includes(ticket.stage) ? "unknown_spawn" : "none") : "unavailable";
  let native: Inspection["native"] = "unknown";
  try {
    const value = await ports.claim(ticket.projectPath);
    claim = value === null ? "none" : value.ticketId === ticket.id ? "this_ticket" : "other_ticket";
  } catch { /* An unavailable store is not evidence that a reservation is absent. */ }
  if (ticket.agentId !== null) {
    try {
      const list = await ports.list(ticket.parentChatId);
      if (list.schemaVersion !== 1 || !Array.isArray(list.agents)) throw new Error("Invalid agent list");
      const matching = list.agents.filter(agent => agent.id === ticket.agentId && agent.parentChatId === ticket.parentChatId);
      if (matching.length > 1) throw new Error("Duplicate agent");
      if (matching.length === 0) managed = "not_found";
      else if (statuses.includes(matching[0].status)) managed = matching[0].status;
      else throw new Error("Invalid agent state");
    } catch { managed = "unavailable"; }
    try {
      const state = await ports.status(ticket.agentId);
      if (state.chatId !== ticket.agentId || typeof state.isProcessing !== "boolean" || typeof state.isIdle !== "boolean" || state.isIdle === state.isProcessing) throw new Error("Invalid native status");
      native = state.isProcessing ? "processing" : "idle";
    } catch { /* Fail closed. */ }
  }
  return {ticketId: ticket.id, observedAt: ports.now(), claim, agentId: ticket.agentId, managed, native};
}
