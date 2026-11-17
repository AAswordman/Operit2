type Agent = {id: string; parentChatId: string; status: string};
interface Snapshot {schemaVersion: number; agents: Agent[]}
interface Ports {
  list(parent: string): Promise<Snapshot>;
  close(parent: string, id: string): Promise<{id: string; status: string}>;
  status(id: string): Promise<{chatId: string; isIdle: boolean; isProcessing: boolean}>;
}
/** Fence a completed agent before transferring a failed ticket's reservation. */
export async function prepareRetry(parentChatId: string, agentId: string, ports: Ports): Promise<boolean> {
  try {
    const snapshot = await ports.list(parentChatId);
    if (snapshot.schemaVersion !== 1 || !Array.isArray(snapshot.agents)) return false;
    const found = snapshot.agents.filter(agent => agent.id === agentId && agent.parentChatId === parentChatId);
    if (found.length !== 1 || !["completed", "closed"].includes(found[0].status)) return false;
    const children = await ports.list(agentId);
    if (children.schemaVersion !== 1 || !Array.isArray(children.agents) || children.agents.length !== 0) return false;
    // Also fence a previously closed agent: ordinary closed conversations may
    // still be used as parents, whereas a recovery-fenced agent may not.
    const closed = await ports.close(parentChatId, agentId);
    if (closed.id !== agentId || closed.status !== "closed") return false;
    const status = await ports.status(agentId);
    return status.chatId === agentId && status.isIdle === true && status.isProcessing === false;
  } catch { return false; }
}
